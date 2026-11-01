//! A cubed-sphere is a projection, not a content hierarchy. Cell IDs stay in
//! sphere-local coordinates regardless of which object the experiment moves.
use exact_game::*;
pub const RADIUS: f32 = 10.;
pub const ALTITUDE: f32 = 1.4;
pub const CELLS: u32 = 40;
pub const EDGE_STEPS: usize = 4;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Data)]
pub struct Cell {
    pub face: u32,
    pub u: u32,
    pub v: u32,
}
pub fn basis(face: u32) -> (Vec3, Vec3, Vec3) {
    match face {
        0 => (Vec3::Z, Vec3::X, Vec3::Y),
        1 => (Vec3::X, -Vec3::Z, Vec3::Y),
        2 => (-Vec3::Z, -Vec3::X, Vec3::Y),
        3 => (-Vec3::X, Vec3::Z, Vec3::Y),
        4 => (Vec3::Y, Vec3::X, -Vec3::Z),
        _ => (-Vec3::Y, Vec3::X, Vec3::Z),
    }
}
pub fn point(face: u32, u: f32, v: f32, radius: f32) -> Vec3 {
    let (n, right, up) = basis(face);
    (n + right * u + up * v).normalize() * radius
}
pub fn cell_at(p: Vec3) -> Cell {
    let face = (0..6)
        .max_by(|a, b| p.dot(basis(*a).0).total_cmp(&p.dot(basis(*b).0)))
        .unwrap();
    let (n, right, up) = basis(face);
    let axis =
        |a: Vec3| (((p.dot(a) / p.dot(n) + 1.) * 0.5 * CELLS as f32).floor() as u32).min(CELLS - 1);
    Cell {
        face,
        u: axis(right),
        v: axis(up),
    }
}
pub fn cell_point(cell: Cell, u: f32, v: f32, radius: f32) -> Vec3 {
    point(
        cell.face,
        -1. + 2. * (cell.u as f32 + u) / CELLS as f32,
        -1. + 2. * (cell.v as f32 + v) / CELLS as f32,
        radius,
    )
}
pub fn base_camera() -> Transform {
    let eye = Vec3::Z * (RADIUS + ALTITUDE);
    Transform::at(eye.x, eye.y, eye.z).looking_at(
        eye + Vec3::Y * math::cos(0.78) - Vec3::Z * math::sin(0.78),
        Vec3::Z,
    )
}
/// The same relative view in two different world-space experiments.
pub fn poses(orientation: Quat, flying: bool) -> (Transform, Transform) {
    let mut camera = base_camera();
    let mut globe = Transform::default();
    if flying {
        camera.position = orientation * camera.position;
        camera.rotation = orientation * camera.rotation;
    } else {
        globe.rotation = orientation.inverse();
    }
    (camera, globe)
}
pub fn ray_sphere(origin: Vec3, direction: Vec3) -> Option<Vec3> {
    let b = origin.dot(direction);
    let d = b * b - (origin.length_squared() - RADIUS * RADIUS);
    if d < 0. {
        return None;
    }
    let t = -b - math::sqrt(d);
    (t >= 0.).then_some(origin + direction * t)
}
pub fn target(orientation: Quat, flying: bool) -> Option<Cell> {
    let (camera, globe) = poses(orientation, flying);
    let inv = globe.rotation.inverse();
    ray_sphere(inv * camera.position, inv * camera.rotation * -Vec3::Z).map(cell_at)
}
pub fn turn(orientation: Quat, delta: Vec2) -> Quat {
    // Tangent axes avoid longitude singularities and arbitrary pole clamps.
    (orientation * Quat::from_rotation_y(-delta.x) * Quat::from_rotation_x(-delta.y)).normalize()
}
pub fn edge(cell: Cell, index: usize) -> (Vec3, Vec3) {
    let side = index / EDGE_STEPS;
    let t = (index % EDGE_STEPS) as f32 / EDGE_STEPS as f32;
    let next = t + 1. / EDGE_STEPS as f32;
    let uv = |t| match side {
        0 => (t, 0.),
        1 => (1., t),
        2 => (1. - t, 1.),
        _ => (0., 1. - t),
    };
    let (u, v) = uv(t);
    let (nu, nv) = uv(next);
    (
        cell_point(cell, u, v, RADIUS + 0.018),
        cell_point(cell, nu, nv, RADIUS + 0.018),
    )
}
fn quad(mesh: &mut asset::MeshData, points: [Vec3; 4]) {
    let offset = (mesh.positions.len() / 3) as u32;
    for p in points {
        mesh.positions.extend(p.to_array());
        mesh.normals.extend(p.normalize().to_array());
        mesh.uvs.extend([0., 0.]);
    }
    let front = (points[1] - points[0])
        .cross(points[2] - points[0])
        .dot(points[0])
        > 0.;
    for i in if front {
        [0, 1, 2, 0, 2, 3]
    } else {
        [0, 2, 1, 0, 3, 2]
    } {
        mesh.indices.push(offset + i);
    }
}
pub fn geometry() -> (asset::MeshData, asset::MeshData) {
    let bounds = [-10.1, -10.1, -10.1, 10.1, 10.1, 10.1];
    let mut shell = asset::MeshData {
        bounds,
        ..Default::default()
    };
    let mut grid = asset::MeshData {
        bounds,
        ..Default::default()
    };
    let steps = CELLS * 2;
    for face in 0..6 {
        for y in 0..steps {
            for x in 0..steps {
                let a = -1. + 2. * x as f32 / steps as f32;
                let b = -1. + 2. * y as f32 / steps as f32;
                let d = 2. / steps as f32;
                quad(
                    &mut shell,
                    [
                        point(face, a, b, RADIUS),
                        point(face, a + d, b, RADIUS),
                        point(face, a + d, b + d, RADIUS),
                        point(face, a, b + d, RADIUS),
                    ],
                );
            }
        }
        for line in 0..=CELLS {
            for segment in 0..CELLS * EDGE_STEPS as u32 {
                let fixed = -1. + 2. * line as f32 / CELLS as f32;
                let a = -1. + 2. * segment as f32 / (CELLS as usize * EDGE_STEPS) as f32;
                let b = a + 2. / (CELLS as usize * EDGE_STEPS) as f32;
                for swap in [false, true] {
                    let sample = |t| {
                        if swap {
                            point(face, t, fixed, RADIUS + 0.008)
                        } else {
                            point(face, fixed, t, RADIUS + 0.008)
                        }
                    };
                    let p = sample(a);
                    let q = sample(b);
                    let side = (q - p).cross((p + q).normalize()).normalize() * 0.0025;
                    quad(&mut grid, [p - side, q - side, q + side, p + side]);
                }
            }
        }
    }
    (shell, grid)
}
