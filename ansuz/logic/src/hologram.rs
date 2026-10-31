//! Presentation only: neither changing a solid nor moving it edits authored records.
use crate::{
    camera_pose,
    records::{Dataset, ROOT_IDS},
    surface, Journey, PLACES,
};
use exact_game::*;

pub const SOLID_NAMES: [&str; 5] = [
    "Tetrahedron",
    "Cube",
    "Octahedron",
    "Dodecahedron",
    "Icosahedron",
];
pub const ROOT_SIZE: Vec3 = Vec3::new(2.5, 0.8, 2.5);
pub const BOB_AMPLITUDE: f32 = 0.12;
pub const WHEEL_THRESHOLD: f32 = 45.;
pub const WHEEL_DEBOUNCE_SECONDS: f32 = 0.18;

/// Provisional design tuning, not a P-tree equation or a physical law.
/// Heavier containment aggregates bob more slowly, approaching 0.28 Hz.
pub fn frequency(weight: f32) -> f32 {
    0.28 + 0.5 / (1. + math::sqrt(weight.max(0.)))
}
pub fn bob(seconds: f32, weight: f32, reduced: bool) -> f32 {
    if reduced {
        0.
    } else {
        BOB_AMPLITUDE * math::sin(seconds * std::f32::consts::TAU * frequency(weight))
    }
}

#[derive(Clone, Copy, Resource)]
pub struct Holograms {
    pub hovered: i32,
    pub focused: i32,
    pub shapes: [u32; 5],
    pub wheel_sum: f32,
    pub cooldown: f32,
    cached_point: Vec2,
    cached_viewport: Vec2,
    cached_camera: Vec3,
    cache_valid: bool,
}
impl Default for Holograms {
    fn default() -> Self {
        Self {
            hovered: -1,
            focused: -1,
            shapes: [0; 5],
            wheel_sum: 0.,
            cooldown: 0.,
            cached_point: Vec2::ZERO,
            cached_viewport: Vec2::ZERO,
            cached_camera: Vec3::ZERO,
            cache_valid: false,
        }
    }
}
impl Holograms {
    pub fn active(&self) -> i32 {
        if self.hovered >= 0 {
            self.hovered
        } else {
            self.focused
        }
    }
    pub fn cycle(&mut self, direction: i32) {
        let active = self.active();
        if active >= 0 {
            self.shapes[active as usize] =
                (self.shapes[active as usize] as i32 + direction).rem_euclid(5) as u32;
        }
    }
    /// One discrete step at most per tick, and an interval between wheel steps.
    /// All wheel input over an active top is consumed, even below the threshold.
    pub fn wheel(&mut self, delta: f32, dt: f32) -> bool {
        self.cooldown = (self.cooldown - dt).max(0.);
        if self.hovered < 0 {
            self.wheel_sum = 0.;
            return false;
        }
        if self.cooldown > 0. {
            return true;
        }
        if self.wheel_sum.signum() != delta.signum() && delta != 0. {
            self.wheel_sum = 0.;
        }
        self.wheel_sum += delta;
        if self.wheel_sum.abs() >= WHEEL_THRESHOLD {
            self.cycle(if self.wheel_sum > 0. { 1 } else { -1 });
            self.wheel_sum = 0.;
            self.cooldown = WHEEL_DEBOUNCE_SECONDS;
        }
        true
    }
}

/// Unit circumradius vertices; nearest-neighbour edges give the five regular solids.
pub fn solid(index: usize) -> (Vec<Vec3>, Vec<(usize, usize)>) {
    let phi = (1. + math::sqrt(5.)) / 2.;
    let mut vertices = Vec::new();
    match index % 5 {
        0 => vertices.extend([
            Vec3::new(1., 1., 1.),
            Vec3::new(1., -1., -1.),
            Vec3::new(-1., 1., -1.),
            Vec3::new(-1., -1., 1.),
        ]),
        1 | 3 => {
            for x in [-1., 1.] {
                for y in [-1., 1.] {
                    for z in [-1., 1.] {
                        vertices.push(Vec3::new(x, y, z));
                    }
                }
            }
            if index % 5 == 3 {
                for a in [-1., 1.] {
                    for b in [-1., 1.] {
                        vertices.extend([
                            Vec3::new(0., a / phi, b * phi),
                            Vec3::new(a / phi, b * phi, 0.),
                            Vec3::new(b * phi, 0., a / phi),
                        ]);
                    }
                }
            }
        }
        2 => vertices.extend([Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z]),
        _ => {
            for a in [-1., 1.] {
                for b in [-1., 1.] {
                    vertices.extend([
                        Vec3::new(0., a, b * phi),
                        Vec3::new(a, b * phi, 0.),
                        Vec3::new(b * phi, 0., a),
                    ]);
                }
            }
        }
    }
    for vertex in &mut vertices {
        *vertex = vertex.normalize();
    }
    let mut nearest = f32::INFINITY;
    for a in 0..vertices.len() {
        for b in a + 1..vertices.len() {
            nearest = nearest.min(vertices[a].distance_squared(vertices[b]));
        }
    }
    let mut edges = Vec::new();
    for a in 0..vertices.len() {
        for b in a + 1..vertices.len() {
            if (vertices[a].distance_squared(vertices[b]) - nearest).abs() < 0.0001 {
                edges.push((a, b));
            }
        }
    }
    (vertices, edges)
}

/// Ray in box-local coordinates. Return first hit and whether it enters +Y.
/// An origin inside the box is deliberately ineligible for top-face hover.
pub fn box_hit(origin: Vec3, direction: Vec3, half: Vec3) -> Option<(f32, bool)> {
    let mut enter = 0.;
    let mut exit = f32::INFINITY;
    let mut top = false;
    for axis in 0..3 {
        if direction[axis].abs() < 0.000001 {
            if origin[axis].abs() > half[axis] {
                return None;
            }
            continue;
        }
        let a = (-half[axis] - origin[axis]) / direction[axis];
        let b = (half[axis] - origin[axis]) / direction[axis];
        let near = a.min(b);
        if near > enter {
            enter = near;
            top = axis == 1 && direction[axis] < 0.;
        }
        exit = exit.min(a.max(b));
        if enter > exit {
            return None;
        }
    }
    if enter <= 0. || exit < 0. {
        None
    } else {
        Some((enter, top))
    }
}
fn transformed_box(
    origin: Vec3,
    direction: Vec3,
    pose: &Transform,
    size: Vec3,
) -> Option<(f32, bool)> {
    let rotation = pose.rotation.inverse();
    box_hit(
        rotation * (origin - pose.position) / pose.scale,
        rotation * direction / pose.scale,
        size * 0.5,
    )
}
pub fn triangle_hit(origin: Vec3, direction: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let ab = b - a;
    let ac = c - a;
    let p = direction.cross(ac);
    let determinant = ab.dot(p);
    if determinant.abs() < 0.000001 {
        return None;
    }
    let t = origin - a;
    let u = t.dot(p) / determinant;
    if !(0. ..=1.).contains(&u) {
        return None;
    }
    let q = t.cross(ab);
    let v = direction.dot(q) / determinant;
    if v < 0. || u + v > 1. {
        return None;
    }
    let distance = ac.dot(q) / determinant;
    if distance > 0. {
        Some(distance)
    } else {
        None
    }
}

/// Exact triangle occlusion for the static terrain, including baked node transforms.
fn terrain_occludes(w: &World, origin: Vec3, direction: Vec3, limit: f32) -> bool {
    let Some(model) = w.model("terrain.model") else {
        return false;
    };
    let pose = w.get::<Transform>("terrain").unwrap();
    let terrain_matrix =
        Mat4::from_scale_rotation_translation(pose.scale, pose.rotation, pose.position);
    for (node_index, node) in model.nodes.iter().enumerate() {
        let Some(mesh_index) = node.mesh else {
            continue;
        };
        let mut matrix = Mat4::from_cols_array(&node.transform);
        let mut parent = model.nodes[node_index].parent;
        let mut depth = 0;
        while let Some(index) = parent {
            if depth >= model.nodes.len() {
                break;
            }
            let ancestor = &model.nodes[index as usize];
            matrix = Mat4::from_cols_array(&ancestor.transform) * matrix;
            parent = ancestor.parent;
            depth += 1;
        }
        let inverse = (terrain_matrix * matrix).inverse();
        let local_origin = inverse.transform_point3(origin);
        let local_direction = inverse.transform_vector3(direction);
        let mesh = &model.meshes[mesh_index as usize];
        let point = |i: u32| Vec3::from_slice(&mesh.positions[i as usize * 3..i as usize * 3 + 3]);
        for triangle in mesh.indices.chunks_exact(3) {
            if triangle_hit(
                local_origin,
                local_direction,
                point(triangle[0]),
                point(triangle[1]),
                point(triangle[2]),
            )
            .is_some_and(|d| d < limit - 0.002)
            {
                return true;
            }
        }
    }
    false
}

/// Only five authored platforms carry roots. Decorative terrain cells have no IDs.
pub fn pick_top(w: &World, point: Vec2, viewport: Vec2, journey: &Journey) -> i32 {
    if viewport.min_element() <= 0.
        || point.x < 0.
        || point.y < 0.
        || point.x > viewport.x
        || point.y > viewport.y
    {
        return -1;
    }
    let camera = camera_pose(journey);
    let ndc = Vec3::new(
        point.x / viewport.x * 2. - 1.,
        1. - point.y / viewport.y * 2.,
        1.,
    );
    let direction = camera.rotation
        * Camera::default()
            .matrix(viewport)
            .inverse()
            .project_point3(ndc)
            .normalize();
    let mut nearest = f32::INFINITY;
    let mut picked = -1;
    // Root sides and all sample reading boxes occlude roots behind them.
    for i in 0..PLACES.len() {
        let pose = w.get::<Transform>(format!("place-{i}")).unwrap();
        if let Some((distance, top)) = transformed_box(camera.position, direction, &pose, ROOT_SIZE)
        {
            if distance < nearest {
                nearest = distance;
                picked = if top { i as i32 } else { -1 };
            }
        }
        for k in 0..9 {
            let pose = w.get::<Transform>(format!("reading-{i}-{k}")).unwrap();
            if let Some((distance, _)) =
                transformed_box(camera.position, direction, &pose, Vec3::new(0.8, 0.5, 0.8))
            {
                if distance < nearest {
                    nearest = distance;
                    picked = -1;
                }
            }
        }
        let halo = w.get::<Transform>(format!("halo-{i}")).unwrap();
        if let Some((distance, _)) =
            transformed_box(camera.position, direction, &halo, Vec3::new(2.4, 0.06, 2.4))
        {
            if distance < nearest {
                nearest = distance;
                picked = -1;
            }
        }
    }
    if picked >= 0 && terrain_occludes(w, camera.position, direction, nearest) {
        -1
    } else {
        picked
    }
}

pub fn update_hover(w: &World, input: &Input, journey: &Journey, state: &mut Holograms) {
    let pointer = input.pointer();
    // A complete click can arrive between fixed ticks, leaving `down` false.
    // The press edge still hands keyboard focus back to the scene interaction.
    if input.pressed("contact") || pointer.is_some_and(|p| p.down) || journey.was_down {
        state.hovered = -1;
        state.focused = -1;
        state.cache_valid = false;
        return;
    }
    if journey.travelling {
        state.hovered = -1;
        state.cache_valid = false;
        return;
    }
    let Some(pointer) = pointer else {
        state.hovered = -1;
        state.cache_valid = false;
        return;
    };
    let camera = Vec3::new(journey.longitude, journey.latitude, journey.altitude);
    if !state.cache_valid
        || state.cached_point != pointer.position
        || state.cached_camera != camera
        || state.cached_viewport != input.viewport()
    {
        let picked = pick_top(w, pointer.position, input.viewport(), journey);
        // An explicit keyboard/button focus survives moving toward the HUD.
        // Entering a real top surface hands control back to pointer hover.
        if picked >= 0 && pointer.delta.length_squared() > 0. {
            state.focused = -1;
        }
        if state.hovered != picked {
            state.wheel_sum = 0.;
        }
        state.hovered = picked;
        state.cached_point = pointer.position;
        state.cached_viewport = input.viewport();
        state.cached_camera = camera;
        state.cache_valid = true;
    }
}

pub fn setup(w: &mut World) {
    w.insert_resource(Holograms::default());
    for i in 0..PLACES.len() {
        let (_, lon, lat, _) = PLACES[i];
        w.spawn_named(
            format!("active-top-{i}"),
            (
                surface(lon, lat, 6.407),
                Mesh::cuboid(Vec3::new(2.5, 0.012, 2.5)),
                Material::hex("#b7d8de").emissive(0.4, 0.52, 0.55),
                Visible(false),
            ),
        );
    }
    for edge in 0..30 {
        w.spawn_named(
            format!("hologram-edge-{edge}"),
            (
                Transform::default(),
                Mesh::cube(1.),
                Material::hex("#d7f3f5").emissive(0.65, 0.85, 0.9),
                Visible(false),
                Ambient,
            ),
        );
    }
}
pub fn draw(w: &mut World, state: &Holograms, journey: &Journey, reduced: bool) {
    let active = if journey.travelling || journey.was_down {
        -1
    } else {
        state.active()
    };
    for i in 0..PLACES.len() {
        w.require_mut::<Visible>(format!("active-top-{i}")).0 = active == i as i32;
    }
    let (vertices, edges) = if active >= 0 {
        solid(state.shapes[active as usize] as usize)
    } else {
        (Vec::new(), Vec::new())
    };
    if active >= 0 {
        let (_, lon, lat, _) = PLACES[active as usize];
        let weight = w
            .resource::<Dataset>()
            .aggregate_weight(ROOT_IDS[active as usize]);
        let pose = surface(lon, lat, 8.05 + bob(w.seconds() as f32, weight, reduced));
        let orientation = pose.rotation * Quat::from_rotation_y(0.3);
        for (edge, &(a, b)) in edges.iter().enumerate() {
            let start = pose.position + orientation * vertices[a] * 0.95;
            let end = pose.position + orientation * vertices[b] * 0.95;
            let delta = end - start;
            *w.require_mut::<Transform>(format!("hologram-edge-{edge}")) = Transform {
                position: (start + end) * 0.5,
                rotation: Quat::from_rotation_arc(Vec3::Y, delta.normalize()),
                scale: Vec3::new(0.025, delta.length(), 0.025),
            };
        }
    }
    for edge in 0..30 {
        w.require_mut::<Visible>(format!("hologram-edge-{edge}")).0 = edge < edges.len();
    }
}
