use ansuz_logic::{
    ascend, camera_pose, focus_surface,
    records::Dataset,
    terrain_pick::{self, Cell},
    Ansuz, Journey, Options, SurfaceRecords,
};
use exact_game::{Camera, Clock, InputEvent, PointerPhase, Sim, Transform, Vec2, Vec3, Visible};
fn game() -> Sim<Ansuz> {
    let mut g = exact_game::sim_with_art!(Options::default()).unwrap();
    g.viewport(800., 600.);
    g.advance(0., Clock::Seekable);
    g
}
fn focus_game() -> Sim<Ansuz> {
    let mut g = game();
    g.tap("Enter");
    g.run(1100.);
    g
}
fn pointer(g: &mut Sim<Ansuz>, phase: PointerPhase, x: f32, dx: f32, buttons: u32) {
    let at_ms = g.world().seconds() * 1000. + 20.;
    g.input(InputEvent::Pointer {
        id: 1,
        phase,
        x,
        y: 300.,
        dx,
        dy: 0.,
        buttons,
        at_ms,
        player: 0,
    });
    g.advance(at_ms + 10., Clock::Seekable);
}
#[test]
fn entry_is_atmospheric_empty_and_has_no_tablet() {
    let g = game();
    let j = g.world().resource::<Journey>();
    assert!(!j.surface);
    assert_eq!(j.travel, 1.);
    assert!(g.world().resource::<Dataset>().objects.is_empty());
    assert!(g.world().resource::<SurfaceRecords>().placements.is_empty());
    assert!(g.get::<Transform>("tablet").is_none());
    assert!(g.get::<Transform>("tablet-writing").is_none());
}
#[test]
fn click_moves_to_the_face_without_creating_content() {
    let mut g = game();
    let before = *g.get::<Transform>("camera").unwrap();
    pointer(&mut g, PointerPhase::Down, 410., 0., 1);
    pointer(&mut g, PointerPhase::Up, 410., 0., 0);
    assert!(g.world().resource::<Journey>().surface);
    g.run(1100.);
    let j = g.world().resource::<Journey>();
    let cell = j.selected.unwrap();
    let pose = camera_pose(&j);
    assert_ne!(pose, before);
    assert!((pose.rotation * -Vec3::Z + cell.normal()).length() < 0.0001);
    assert!((pose.position - cell.center() - cell.normal() * j.altitude).length() < 0.0001);
    assert!(g.get::<Visible>("selected-face").unwrap().0);
    assert!(g.world().resource::<Dataset>().objects.is_empty());
}
#[test]
fn all_six_exposed_directions_focus_and_return_to_the_same_view() {
    let g = game();
    for face in 0..6 {
        let normal = Cell {
            face,
            ..Cell::default()
        }
        .normal();
        // Offset avoids shared-edge ambiguity at the poles.
        let origin = normal * 170. + Vec3::splat(0.7);
        let cell = terrain_pick::pick(g.world(), origin, -normal).unwrap();
        assert_eq!(cell.face, face);
        let mut j = *g.world().resource::<Journey>();
        let before = camera_pose(&j);
        focus_surface(g.world(), &mut j, cell);
        assert_eq!(camera_pose(&j), before);
        for step in 0..=100 {
            j.travel = step as f32 / 100.;
            let pose = camera_pose(&j);
            // An outward ray exiting a back face would put the camera inside terrain.
            if let Some((_, hit)) =
                terrain_pick::hit(g.world(), pose.position, pose.position.normalize())
            {
                assert!(
                    hit.is_some(),
                    "camera entered terrain: face {face}, step {step}"
                );
            }
        }
        let pose = camera_pose(&j);
        assert!((pose.rotation * -Vec3::Z + normal).length() < 0.0001);
        assert!((pose.position - cell.center() - normal * j.altitude).length() < 0.0001);
        ascend(&mut j);
        j.travel = 1.;
        assert_eq!(camera_pose(&j), before);
    }
}
#[test]
fn changing_faces_starts_at_current_camera_and_changes_its_local_basis() {
    let g = game();
    let mut j = *g.world().resource::<Journey>();
    let a = terrain_pick::pick(g.world(), Vec3::new(0.7, 0.7, 170.), -Vec3::Z).unwrap();
    let b = terrain_pick::pick(g.world(), Vec3::new(170., 0.7, 0.7), -Vec3::X).unwrap();
    focus_surface(g.world(), &mut j, a);
    j.travel = 1.;
    let before = camera_pose(&j);
    focus_surface(g.world(), &mut j, b);
    assert_eq!(camera_pose(&j), before);
    j.travel = 1.;
    assert!((camera_pose(&j).rotation * -Vec3::Z + Vec3::X).length() < 0.0001);
    assert_eq!(j.selected, Some(b));
}
#[test]
fn drag_orbits_without_focusing_and_new_contact_catches_the_coast() {
    let mut g = game();
    pointer(&mut g, PointerPhase::Down, 400., 0., 1);
    pointer(&mut g, PointerPhase::Move, 460., 60., 1);
    pointer(&mut g, PointerPhase::Up, 460., 0., 0);
    let lon = g.world().resource::<Journey>().longitude;
    assert!(lon < 0.);
    assert!(!g.world().resource::<Journey>().surface);
    g.run(100.);
    assert!(g.world().resource::<Journey>().longitude < lon);
    pointer(&mut g, PointerPhase::Down, 460., 0., 1);
    let caught = g.world().resource::<Journey>().longitude;
    g.run(100.);
    assert_eq!(g.world().resource::<Journey>().longitude, caught);
}
#[test]
fn focused_pan_is_local_and_does_not_change_selection() {
    let mut g = focus_game();
    let before = *g.world().resource::<Journey>();
    pointer(&mut g, PointerPhase::Down, 400., 0., 1);
    pointer(&mut g, PointerPhase::Move, 430., 30., 1);
    pointer(&mut g, PointerPhase::Up, 430., 0., 0);
    let after = *g.world().resource::<Journey>();
    assert!(after.x < 0.);
    assert_eq!(after.selected, before.selected);
    g.run(300.);
    assert_eq!(g.world().resource::<Journey>().x, after.x);
}
#[test]
fn selected_and_occupied_markers_align_with_side_faces() {
    let json = r#"{"version":1,"next_id":2,"objects":[{"id":"note-1","title":"Side","body":"Remember","content_kind":"note","authored_weight":1}],"relations":[],"placements":[{"object_id":"note-1","x":47,"y":0,"z":0,"face":2}]}"#;
    let mut g: Sim<Ansuz> = exact_game::sim_with_art!(Options {
        records: json.into(),
        editing: false
    })
    .unwrap();
    let cell = Cell {
        x: 47,
        y: 0,
        z: 0,
        face: 2,
    };
    let mut j = *g.world().resource::<Journey>();
    focus_surface(g.world(), &mut j, cell);
    j.travel = 1.;
    *g.world().resource_mut::<Journey>() = j;
    g.run(30.);
    assert!(
        (g.get::<Transform>("selected-face").unwrap().rotation * Vec3::Y - Vec3::X).length()
            < 0.0001
    );
    assert_eq!(g.world().resource::<Dataset>().objects[0].id, "note-1");
    assert_eq!(
        g.world().resource::<SurfaceRecords>().placements[0].cell(),
        cell
    );
    let camera = camera_pose(&j);
    let ndc = Camera::default()
        .matrix(Vec2::new(800., 600.))
        .project_point3(camera.rotation.inverse() * (cell.center() - camera.position));
    assert!(ndc.x.abs() < 0.0001 && ndc.y.abs() < 0.0001);
}

#[test]
fn visible_face_transitions_stay_outside_the_terrain() {
    let g = game();
    let start = *g.world().resource::<Journey>();
    let mut faces = Vec::new();
    for y in [180., 260., 340., 420., 500.] {
        for x in [120., 260., 400., 540., 680.] {
            if let Some(cell) =
                ansuz_logic::pick(g.world(), Vec2::new(x, y), Vec2::new(800., 600.), &start)
            {
                faces.push(cell);
            }
        }
    }
    assert!(faces.len() > 10);
    assert!(faces.iter().any(|c| c.face != 0));
    for cell in faces {
        let mut j = start;
        focus_surface(g.world(), &mut j, cell);
        for step in 0..=100 {
            j.travel = step as f32 / 100.;
            let pose = camera_pose(&j);
            if let Some((_, hit)) =
                terrain_pick::hit(g.world(), pose.position, pose.position.normalize())
            {
                assert!(
                    hit.is_some(),
                    "camera entered terrain at {cell:?}, step {step}"
                );
            }
        }
    }
}

#[test]
fn focused_pan_and_zoom_do_not_enter_neighbouring_terrain() {
    let mut g = game();
    for face in 0..6 {
        let normal = Cell {
            face,
            ..Cell::default()
        }
        .normal();
        let cell =
            terrain_pick::pick(g.world(), normal * 170. + Vec3::splat(0.7), -normal).unwrap();
        for key in ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"] {
            let mut j = Journey::default();
            focus_surface(g.world(), &mut j, cell);
            j.travel = 1.;
            *g.world().resource_mut::<Journey>() = j;
            for step in 0..100 {
                g.hold(key, 30.);
                if step % 10 == 0 {
                    g.tap("Equal");
                }
                let eye = camera_pose(&g.world().resource::<Journey>()).position;
                if let Some((_, hit)) = terrain_pick::hit(g.world(), eye, eye.normalize()) {
                    assert!(
                        hit.is_some(),
                        "pan entered terrain at face {face}, key {key}"
                    );
                }
            }
        }
    }
}
