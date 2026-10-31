//! Picking the real, greedily merged voxel terrain. Cells are placement coordinates,
//! not object IDs: the content model decides whether a picked surface has content.
use exact_game::{asset::Model, Data, Mat4, Resource, Transform, Vec3, World};

pub const CELL: f32 = 2.5;
const EPSILON: f32 = 0.0001;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Data)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Cell {
    /// Center of the voxel's +Y face in the terrain's local coordinate system.
    pub fn center(self) -> Vec3 {
        Vec3::new(self.x as f32 + 0.5, self.y as f32 + 1., self.z as f32 + 0.5) * CELL
    }
}

#[derive(Clone, Default, Data)]
struct Triangle {
    a: Vec3,
    b: Vec3,
    c: Vec3,
    min: Vec3,
    max: Vec3,
    top: bool,
}

#[derive(Clone, Default, Data)]
struct Branch {
    min: Vec3,
    max: Vec3,
    left: u32,
    right: u32,
    start: u32,
    count: u32,
}

/// Derived acceleration data only. Rebuilt when the model changes; never an
/// authority for content, identities, or provenance.
#[derive(Default, Resource)]
struct TerrainPickCache {
    revision: u64,
    ready: bool,
    triangles: Vec<Triangle>,
    branches: Vec<Branch>,
}

/// Install a lazy cache. Model loading can finish after setup. `pick` also works
/// without this call, but then rebuilds its acceleration data for each query.
pub fn setup(w: &mut World) {
    let cache = w
        .model("terrain.model")
        .map_or_else(TerrainPickCache::default, |model| {
            TerrainPickCache::build(model, w.model_revision())
        });
    w.insert_resource(cache);
}

/// Closest actual terrain triangle wins. A side or underside blocks any top
/// farther along the ray. Only upward faces approached from above are eligible.
pub fn pick(w: &World, origin: Vec3, direction: Vec3) -> Option<Cell> {
    hit(w, origin, direction).and_then(|(_, cell)| cell)
}

/// Distance includes blocking side/underside triangles, whose cell is None.
pub fn hit(w: &World, origin: Vec3, direction: Vec3) -> Option<(f32, Option<Cell>)> {
    if !origin.is_finite() || !direction.is_finite() || direction.length_squared() < 1e-12 {
        return None;
    }
    let model = w.model("terrain.model")?;
    let pose = w.get::<Transform>("terrain")?;
    let matrix = Mat4::from_scale_rotation_translation(pose.scale, pose.rotation, pose.position);
    if matrix.determinant().abs() < 1e-12 {
        return None;
    }
    let inverse = matrix.inverse();
    let origin = inverse.transform_point3(origin);
    let direction = inverse.transform_vector3(direction.normalize());
    let revision = w.model_revision();
    let needs_rebuild = w
        .try_resource::<TerrainPickCache>()
        .map(|cache| !cache.ready || cache.revision != revision);
    if let Some(needs_rebuild) = needs_rebuild {
        if needs_rebuild {
            *w.resource_mut::<TerrainPickCache>() = TerrainPickCache::build(model, revision);
        }
        w.resource::<TerrainPickCache>().hit(origin, direction)
    } else {
        TerrainPickCache::build(model, revision).hit(origin, direction)
    }
}

impl TerrainPickCache {
    fn build(model: &Model, revision: u64) -> Self {
        let mut cache = Self {
            revision,
            ready: true,
            ..Self::default()
        };
        if model.nodes.is_empty() {
            for mesh in &model.meshes {
                cache.mesh(mesh, Mat4::IDENTITY);
            }
        } else {
            for node in &model.nodes {
                let Some(mesh_index) = node.mesh else {
                    continue;
                };
                let Some(mesh) = model.meshes.get(mesh_index as usize) else {
                    continue;
                };
                let mut matrix = Mat4::from_cols_array(&node.transform);
                let mut parent = node.parent;
                let mut depth = 0;
                let mut valid = true;
                while let Some(index) = parent {
                    let Some(ancestor) = model.nodes.get(index as usize) else {
                        valid = false;
                        break;
                    };
                    if depth >= model.nodes.len() {
                        valid = false;
                        break;
                    }
                    matrix = Mat4::from_cols_array(&ancestor.transform) * matrix;
                    parent = ancestor.parent;
                    depth += 1;
                }
                if valid {
                    cache.mesh(mesh, matrix);
                }
            }
        }
        if !cache.triangles.is_empty() {
            let count = cache.triangles.len();
            cache.branch(0, count);
        }
        cache
    }

    fn mesh(&mut self, mesh: &exact_game::asset::MeshData, matrix: Mat4) {
        for indices in mesh.indices.chunks_exact(3) {
            let point = |index: u32| {
                let start = index as usize * 3;
                mesh.positions
                    .get(start..start + 3)
                    .map(|v| matrix.transform_point3(Vec3::from_slice(v)))
            };
            let (Some(a), Some(b), Some(c)) =
                (point(indices[0]), point(indices[1]), point(indices[2]))
            else {
                continue;
            };
            if !a.is_finite() || !b.is_finite() || !c.is_finite() {
                continue;
            }
            let cross = (b - a).cross(c - a);
            if cross.length_squared() < 1e-12 {
                continue;
            }
            self.triangles.push(Triangle {
                a,
                b,
                c,
                min: a.min(b).min(c),
                max: a.max(b).max(c),
                top: cross.normalize().dot(Vec3::Y) > 0.9999,
            });
        }
    }

    fn branch(&mut self, start: usize, end: usize) -> u32 {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for triangle in &self.triangles[start..end] {
            min = min.min(triangle.min);
            max = max.max(triangle.max);
        }
        let index = self.branches.len() as u32;
        self.branches.push(Branch {
            min,
            max,
            start: start as u32,
            count: (end - start) as u32,
            ..Branch::default()
        });
        if end - start > 8 {
            let extent = max - min;
            let axis = if extent.x >= extent.y && extent.x >= extent.z {
                0
            } else if extent.y >= extent.z {
                1
            } else {
                2
            };
            self.triangles[start..end].sort_by(|a, b| {
                (a.min[axis] + a.max[axis]).total_cmp(&(b.min[axis] + b.max[axis]))
            });
            let middle = start + (end - start) / 2;
            let left = self.branch(start, middle);
            let right = self.branch(middle, end);
            let branch = &mut self.branches[index as usize];
            branch.left = left;
            branch.right = right;
            branch.count = 0;
        }
        index
    }

    #[cfg(test)]
    fn cast(&self, origin: Vec3, direction: Vec3) -> Option<Cell> {
        self.hit(origin, direction).and_then(|(_, cell)| cell)
    }

    fn hit(&self, origin: Vec3, direction: Vec3) -> Option<(f32, Option<Cell>)> {
        if self.branches.is_empty() {
            return None;
        }
        let mut nearest = f32::INFINITY;
        let mut hit: Option<usize> = None;
        let mut stack = vec![0u32];
        while let Some(index) = stack.pop() {
            let branch = &self.branches[index as usize];
            if bounds_hit(origin, direction, branch.min, branch.max, nearest + EPSILON).is_none() {
                continue;
            }
            if branch.count > 0 {
                for i in branch.start as usize..(branch.start + branch.count) as usize {
                    let triangle = &self.triangles[i];
                    if let Some(distance) = triangle_hit(origin, direction, triangle) {
                        // At an exact shared edge, prefer rejection over reaching through a side.
                        if distance < nearest - EPSILON
                            || ((distance - nearest).abs() <= EPSILON && !triangle.top)
                        {
                            nearest = distance;
                            hit = Some(i);
                        }
                    }
                }
            } else {
                let left = &self.branches[branch.left as usize];
                let right = &self.branches[branch.right as usize];
                let l = bounds_hit(origin, direction, left.min, left.max, nearest + EPSILON);
                let r = bounds_hit(origin, direction, right.min, right.max, nearest + EPSILON);
                match (l, r) {
                    (Some(l), Some(r)) => {
                        if l <= r {
                            stack.push(branch.right);
                            stack.push(branch.left);
                        } else {
                            stack.push(branch.left);
                            stack.push(branch.right);
                        }
                    }
                    (Some(_), None) => stack.push(branch.left),
                    (None, Some(_)) => stack.push(branch.right),
                    _ => (),
                }
            }
        }
        let triangle = &self.triangles[hit?];
        if !triangle.top || direction.y >= 0. {
            return Some((nearest, None));
        }
        let point = origin + direction * nearest;
        // A greedy quad may span dozens of cells. Use its hit position, then
        // clamp boundary ties to an actually represented cell of that quad.
        let coordinate = |value: f32, min: f32, max: f32| {
            let lower = (min / CELL).round() as i32;
            let upper = (max / CELL).round() as i32 - 1;
            ((value / CELL + 0.00001).floor() as i32).clamp(lower, upper.max(lower))
        };
        Some((
            nearest,
            Some(Cell {
                x: coordinate(point.x, triangle.min.x, triangle.max.x),
                y: (triangle.a.y / CELL).round() as i32 - 1,
                z: coordinate(point.z, triangle.min.z, triangle.max.z),
            }),
        ))
    }
}

fn bounds_hit(origin: Vec3, direction: Vec3, min: Vec3, max: Vec3, limit: f32) -> Option<f32> {
    let mut near: f32 = 0.;
    let mut far = limit;
    for axis in 0..3 {
        if direction[axis].abs() < 1e-9 {
            if origin[axis] < min[axis] - EPSILON || origin[axis] > max[axis] + EPSILON {
                return None;
            }
        } else {
            let a = (min[axis] - origin[axis]) / direction[axis];
            let b = (max[axis] - origin[axis]) / direction[axis];
            near = near.max(a.min(b));
            far = far.min(a.max(b));
            if near > far + EPSILON {
                return None;
            }
        }
    }
    if far < 0. {
        None
    } else {
        Some(near)
    }
}

fn triangle_hit(origin: Vec3, direction: Vec3, triangle: &Triangle) -> Option<f32> {
    let ab = triangle.b - triangle.a;
    let ac = triangle.c - triangle.a;
    let p = direction.cross(ac);
    let determinant = ab.dot(p);
    if determinant.abs() < 1e-8 {
        return None;
    }
    let t = origin - triangle.a;
    let u = t.dot(p) / determinant;
    if !(-0.00001..=1.00001).contains(&u) {
        return None;
    }
    let q = t.cross(ab);
    let v = direction.dot(q) / determinant;
    if v < -0.00001 || u + v > 1.00001 {
        return None;
    }
    let distance = ac.dot(q) / determinant;
    if distance > EPSILON {
        Some(distance)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exact_game::asset::{MeshData, Node};

    fn quad_model() -> Model {
        // A single merged 4 x 2-cell +Y surface, spanning negative coordinates.
        Model {
            meshes: vec![MeshData {
                positions: vec![-5., 5., -2.5, -5., 5., 2.5, 5., 5., 2.5, 5., 5., -2.5],
                indices: vec![0, 1, 2, 0, 2, 3],
                ..MeshData::default()
            }],
            ..Model::default()
        }
    }

    #[test]
    fn merged_quad_resolves_individual_cells_and_correct_top_centers() {
        let cache = TerrainPickCache::build(&quad_model(), 1);
        for x in -2..2 {
            for z in -1..1 {
                let cell = Cell { x, y: 1, z };
                assert_eq!(
                    cache.cast(cell.center() + Vec3::Y * 20., -Vec3::Y),
                    Some(cell)
                );
                assert_eq!(cell.center().y, 5.);
            }
        }
        assert_eq!(
            cache.cast(Vec3::new(5., 20., 1.), -Vec3::Y),
            Some(Cell { x: 1, y: 1, z: 0 })
        );
        assert_eq!(
            cache.cast(Vec3::new(0., 0., 0.), Vec3::Y),
            None,
            "underside approach is ineligible"
        );
    }

    #[test]
    fn nearest_side_blocks_top_behind_it() {
        let mut model = quad_model();
        // +Z wall between the ray and the top; ray reaches the top at z=0.
        model.meshes.push(MeshData {
            positions: vec![-5., 5., 2.5, 5., 5., 2.5, 5., 15., 2.5, -5., 15., 2.5],
            indices: vec![0, 1, 2, 0, 2, 3],
            ..MeshData::default()
        });
        let origin = Vec3::new(1., 15., 10.);
        let direction = Vec3::new(0., -1., -1.);
        assert!(TerrainPickCache::build(&quad_model(), 1)
            .cast(origin, direction)
            .is_some());
        assert_eq!(
            TerrainPickCache::build(&model, 1).cast(origin, direction),
            None
        );
    }

    #[test]
    fn baked_node_hierarchy_is_applied_before_subdividing() {
        let mut model = quad_model();
        model.nodes = vec![
            Node {
                transform: Mat4::from_translation(Vec3::new(5., 2.5, 0.)).to_cols_array(),
                ..Node::default()
            },
            Node {
                parent: Some(0),
                mesh: Some(0),
                ..Node::default()
            },
        ];
        let cache = TerrainPickCache::build(&model, 2);
        assert_eq!(
            cache.cast(Vec3::new(6.25, 20., 1.25), -Vec3::Y),
            Some(Cell { x: 2, y: 2, z: 0 })
        );
    }

    #[test]
    fn bvh_keeps_nearest_hits_across_many_merged_surfaces() {
        let mut model = Model::default();
        for column in 0..24 {
            let mut mesh = quad_model().meshes.remove(0);
            for point in mesh.positions.chunks_exact_mut(3) {
                point[0] += column as f32 * 10.;
                point[1] += (column % 3) as f32 * CELL;
            }
            model.meshes.push(mesh);
        }
        let cache = TerrainPickCache::build(&model, 1);
        assert!(cache.branches.len() > 1);
        for column in 0..24 {
            assert_eq!(
                cache.cast(Vec3::new(column as f32 * 10. + 1.25, 40., 1.25), -Vec3::Y),
                Some(Cell {
                    x: column * 4,
                    y: 1 + column % 3,
                    z: 0
                })
            );
        }
        assert_eq!(cache.cast(Vec3::new(-50., 40., 1.25), -Vec3::Y), None);
    }
}
