use ansuz_logic::hologram::{self, Holograms};
use ansuz_logic::records::{Dataset, ROOT_IDS};
use ansuz_logic::{camera_pose, Ansuz, Journey, Options, RootObject};
use exact_game::{Camera, Clock, InputEvent, PointerPhase, Sim, Transform, Vec2, Vec3, Visible};

fn descended(reduced: bool) -> Sim<Ansuz> {
    let mut game = exact_game::sim_with_art!(Options {
        reduced,
        ..Options::default()
    })
    .unwrap();
    game.viewport(800., 600.);
    game.tap("KeyN");
    game.tap("Enter");
    game.run(3000.);
    game
}
fn top_point(game: &Sim<Ansuz>) -> Vec2 {
    let root = game.get::<Transform>("place-0").unwrap();
    let point = root.position + root.rotation * Vec3::Y * 0.4;
    let camera = camera_pose(&game.world().resource::<Journey>());
    let ndc = Camera::default()
        .matrix(Vec2::new(800., 600.))
        .project_point3(camera.rotation.inverse() * (point - camera.position));
    Vec2::new((ndc.x + 1.) * 400., (1. - ndc.y) * 300.)
}
fn pointer(game: &mut Sim<Ansuz>, phase: PointerPhase, point: Vec2, delta: Vec2, buttons: u32) {
    let at_ms = game.world().seconds() * 1000. + 1.;
    game.input(InputEvent::Pointer {
        id: 1,
        phase,
        x: point.x,
        y: point.y,
        dx: delta.x,
        dy: delta.y,
        buttons,
        at_ms,
        player: 0,
    });
    game.advance(at_ms + 20., Clock::Seekable);
}
fn wheel(game: &mut Sim<Ansuz>, dy: f32) {
    let at_ms = game.world().seconds() * 1000. + 1.;
    game.input(InputEvent::Wheel {
        dx: 0.,
        dy,
        at_ms,
        player: 0,
    });
    game.advance(at_ms + 20., Clock::Seekable);
}

#[test]
fn five_regular_solids_have_correct_vertex_edge_counts_and_equal_edges() {
    for (index, (vertices, edges, degree)) in
        [(4, 6, 3), (8, 12, 3), (6, 12, 4), (20, 30, 3), (12, 30, 5)]
            .into_iter()
            .enumerate()
    {
        let (v, e) = hologram::solid(index);
        assert_eq!((v.len(), e.len()), (vertices, edges));
        let edge_length = v[e[0].0].distance(v[e[0].1]);
        for vertex in &v {
            assert!((vertex.length() - 1.).abs() < 0.00001);
        }
        for &(a, b) in &e {
            assert!((v[a].distance(v[b]) - edge_length).abs() < 0.00001);
        }
        for i in 0..v.len() {
            assert_eq!(e.iter().filter(|&&(a, b)| a == i || b == i).count(), degree);
        }
    }
}

#[test]
fn box_picking_distinguishes_top_side_bottom_and_misses() {
    let half = Vec3::ONE;
    assert_eq!(
        hologram::box_hit(Vec3::Y * 3., -Vec3::Y, half),
        Some((2., true))
    );
    assert_eq!(
        hologram::box_hit(Vec3::X * 3., -Vec3::X, half),
        Some((2., false))
    );
    assert_eq!(
        hologram::box_hit(-Vec3::Y * 3., Vec3::Y, half),
        Some((2., false))
    );
    assert_eq!(
        hologram::box_hit(Vec3::new(2., 3., 0.), -Vec3::Y, half),
        None
    );
    assert_eq!(hologram::box_hit(Vec3::ZERO, Vec3::Y, half), None);
}

#[test]
fn scene_top_pick_rejects_terrain_and_sample_marker_occlusion() {
    let mut game = descended(true);
    let point = top_point(&game);
    let size = Vec2::new(800., 600.);
    let journey = *game.world().resource::<Journey>();
    assert_eq!(hologram::pick_top(game.world(), point, size, &journey), 0);
    // Keep the camera outside the enlarged shell, with the root behind its front face.
    game.world_mut().require_mut::<Transform>("terrain").scale = Vec3::splat(1.04);
    assert_eq!(
        hologram::pick_top(game.world(), point, size, &journey),
        -1,
        "terrain hides the target, even if its screen bounds overlap"
    );
    game.world_mut().require_mut::<Transform>("terrain").scale = Vec3::ONE;
    let target = game.get::<Transform>("place-0").unwrap().position;
    let eye = camera_pose(&journey).position;
    game.world_mut()
        .require_mut::<Transform>("reading-0-0")
        .position = (target + eye) * 0.5;
    assert_eq!(
        hologram::pick_top(game.world(), point, size, &journey),
        -1,
        "an intervening reading is opaque"
    );
}

#[test]
fn hovering_and_scroll_change_representation_without_moving_camera_or_records() {
    let mut game = descended(true);
    let point = top_point(&game);
    pointer(&mut game, PointerPhase::Move, point, Vec2::ONE, 0);
    assert_eq!(game.world().resource::<Holograms>().hovered, 0);
    assert!(game.get::<Visible>("active-top-0").unwrap().0);
    assert!(game.get::<Visible>("hologram-edge-0").unwrap().0);
    let before = game.world().resource::<Dataset>().clone();
    let journey = *game.world().resource::<Journey>();
    wheel(&mut game, 100.);
    assert_eq!(game.world().resource::<Holograms>().shapes[0], 1);
    wheel(&mut game, 100.);
    assert_eq!(
        game.world().resource::<Holograms>().shapes[0],
        1,
        "wheel burst is debounced"
    );
    let after = *game.world().resource::<Journey>();
    assert_eq!(
        (after.longitude, after.latitude),
        (journey.longitude, journey.latitude)
    );
    assert_eq!(*game.world().resource::<Dataset>(), before);
    assert_eq!(game.get::<RootObject>("place-0").unwrap().id, ROOT_IDS[0]);
    pointer(
        &mut game,
        PointerPhase::Move,
        Vec2::new(5., 5.),
        Vec2::ONE,
        0,
    );
    assert_eq!(game.world().resource::<Holograms>().hovered, -1);
    assert!(!game.get::<Visible>("active-top-0").unwrap().0);
    wheel(&mut game, 100.);
    assert_ne!(
        game.world().resource::<Journey>().latitude,
        journey.latitude,
        "wheel away from a root still navigates"
    );
}

#[test]
fn drag_suspends_hologram_and_preserves_root_shape() {
    let mut game = descended(true);
    let point = top_point(&game);
    pointer(&mut game, PointerPhase::Move, point, Vec2::ONE, 0);
    pointer(&mut game, PointerPhase::Down, point, Vec2::ZERO, 1);
    pointer(
        &mut game,
        PointerPhase::Move,
        point + Vec2::new(70., 0.),
        Vec2::new(70., 0.),
        1,
    );
    assert_eq!(game.world().resource::<Holograms>().hovered, -1);
    assert!(!game.get::<Visible>("active-top-0").unwrap().0);
    assert!(!game.get::<Visible>("hologram-edge-0").unwrap().0);
    assert_eq!(game.world().resource::<Holograms>().shapes, [0; 5]);
}

#[test]
fn resizing_rechecks_the_ray_under_a_stationary_pointer() {
    let mut game = descended(true);
    let point = top_point(&game);
    pointer(&mut game, PointerPhase::Move, point, Vec2::ONE, 0);
    assert_eq!(game.world().resource::<Holograms>().hovered, 0);
    game.viewport(1600., 1200.);
    game.run(30.);
    assert_eq!(game.world().resource::<Holograms>().hovered, -1);
}

#[test]
fn a_quick_scene_click_clears_keyboard_focus_even_between_ticks() {
    let mut game = descended(true);
    assert_eq!(game.world().resource::<Holograms>().focused, 0);
    let at_ms = game.world().seconds() * 1000. + 1.;
    for (phase, buttons, time) in [
        (PointerPhase::Down, 1, at_ms),
        (PointerPhase::Up, 0, at_ms + 0.1),
    ] {
        game.input(InputEvent::Pointer {
            id: 1,
            phase,
            x: 5.,
            y: 5.,
            dx: 0.,
            dy: 0.,
            buttons,
            at_ms: time,
            player: 0,
        });
    }
    game.advance(at_ms + 20., Clock::Seekable);
    assert_eq!(game.world().resource::<Holograms>().focused, -1);
    assert_eq!(game.world().resource::<Holograms>().hovered, -1);
    assert!(!game.get::<Visible>("hologram-edge-0").unwrap().0);
}

#[test]
fn keyboard_cycles_both_directions_and_reduced_motion_is_stationary() {
    let mut game = descended(true);
    pointer(
        &mut game,
        PointerPhase::Move,
        Vec2::new(5., 5.),
        Vec2::ONE,
        0,
    );
    assert_eq!(
        game.world().resource::<Holograms>().focused,
        0,
        "moving toward controls keeps keyboard focus"
    );
    let records = game.world().resource::<Dataset>().clone();
    game.tap("KeyQ");
    game.run(30.);
    assert_eq!(game.world().resource::<Holograms>().shapes[0], 4);
    assert!(game.get::<Visible>("hologram-edge-29").unwrap().0);
    let still = *game.get::<Transform>("hologram-edge-0").unwrap();
    game.run(1200.);
    assert_eq!(*game.get::<Transform>("hologram-edge-0").unwrap(), still);
    game.tap("KeyE");
    game.run(30.);
    assert_eq!(game.world().resource::<Holograms>().shapes[0], 0);
    assert_eq!(*game.world().resource::<Dataset>(), records);
    assert!(hologram::frequency(1.) > hologram::frequency(20.));
    assert_eq!(hologram::bob(4., 10., true), 0.);
    assert_ne!(
        hologram::bob(0., 10., false),
        hologram::bob(0.5, 10., false)
    );
}
