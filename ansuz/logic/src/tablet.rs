//! A temporary rendering of the selected surface, never a new content record.
use crate::{camera_pose, terrain_pick::Cell, Journey};
use exact_game::*;

#[derive(Default, Resource)]
pub struct TabletView {
    pub cell: Option<Cell>,
    pub reveal: f32,
    pub heading: f32,
    pub turn: f32,
}

pub fn setup(w: &mut World) {
    w.insert_resource(TabletView::default());
    w.spawn_named(
        "tablet",
        (
            Transform::default(),
            Mesh::asset("tablet.model"),
            Visible(false),
        ),
    );
    w.spawn_named(
        "tablet-light",
        (
            Transform::default(),
            PointLight {
                color: [1., 0.97, 0.92],
                intensity: 0.,
                range: 16.,
            },
        ),
    );
}

pub fn draw(w: &mut World, j: &Journey) {
    let dt = w.dt();
    let camera = camera_pose(j);
    let mut view = w.resource_mut::<TabletView>();
    if view.cell != j.selected {
        view.cell = j.selected;
        view.reveal = 0.;
        view.turn = 0.;
        view.heading = j.selected.map_or(0., |cell| {
            let d = camera.position - cell.center();
            if j.surface {
                0.
            } else {
                math::atan2(d.x, d.z)
            }
        });
    }
    let Some(cell) = view.cell else {
        w.require_mut::<Visible>("tablet").0 = false;
        w.require_mut::<PointLight>("tablet-light").intensity = 0.;
        return;
    };
    view.reveal = (view.reveal + dt / 0.42).min(1.);
    let t = view.reveal * view.reveal * (3. - 2. * view.reveal);
    let rotation = Quat::from_rotation_y(view.heading)
        * Quat::from_rotation_x(0.85 * t)
        * Quat::from_rotation_z((view.turn - 0.18) * t);
    let pose = Transform {
        position: cell.center() + Vec3::Y * (0.5 + 4.8 * t),
        rotation,
        scale: Vec3::splat(0.48 + 1.02 * t),
    };
    let moving = view.reveal < 1.;
    *w.require_mut::<Transform>("tablet") = pose;
    w.require_mut::<Visible>("tablet").0 = true;
    *w.require_mut::<Transform>("tablet-light") = Transform {
        position: pose.position + Quat::from_rotation_y(view.heading) * Vec3::new(-3., 7., 4.),
        ..Transform::default()
    };
    w.require_mut::<PointLight>("tablet-light").intensity = 160000. * t;
    if moving {
        w.busy("tablet rising");
    }
}

/// Test actual clay triangles so its rounded corners don't steal neighbouring faces.
pub fn hit(w: &World, origin: Vec3, direction: Vec3) -> Option<f32> {
    if !w.get::<Visible>("tablet")?.0 {
        return None;
    }
    let pose = w.get::<Transform>("tablet")?;
    let inverse =
        Mat4::from_scale_rotation_translation(pose.scale, pose.rotation, pose.position).inverse();
    let origin = inverse.transform_point3(origin);
    let ray = inverse.transform_vector3(direction);
    let model = w.model("tablet.model")?;
    let mut nearest = f32::INFINITY;
    for mesh in &model.meshes {
        for tri in mesh.indices.chunks_exact(3) {
            let p = |index: u32| Vec3::from_slice(&mesh.positions[index as usize * 3..]);
            let a = p(tri[0]);
            let ab = p(tri[1]) - a;
            let ac = p(tri[2]) - a;
            let cross = ray.cross(ac);
            let det = ab.dot(cross);
            if det.abs() < 1e-8 {
                continue;
            }
            let delta = origin - a;
            let u = delta.dot(cross) / det;
            if !(0. ..=1.).contains(&u) {
                continue;
            }
            let q = delta.cross(ab);
            let v = ray.dot(q) / det;
            if v < 0. || u + v > 1. {
                continue;
            }
            let distance = ac.dot(q) / det;
            if distance > 0. {
                nearest = nearest.min(distance);
            }
        }
    }
    nearest.is_finite().then_some(nearest)
}
