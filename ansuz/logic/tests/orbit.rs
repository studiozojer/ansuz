use ansuz_logic::{orbit::*, Ansuz, Navigation, Options};
use exact_game::*;
fn sim(mode: u32) -> Sim<Ansuz> {
    let mut sim = Sim::new(Options {
        mode,
        ..Default::default()
    })
    .unwrap();
    sim.viewport(390., 500.);
    sim.run(20.);
    sim
}
fn pointer(s: &mut Sim<Ansuz>, phase: PointerPhase, dx: f32, dy: f32) {
    s.input(InputEvent::Pointer {
        id: 7,
        phase,
        x: 195. + dx,
        y: 250. + dy,
        dx,
        dy,
        buttons: 0,
        at_ms: 0.,
        player: 0,
    });
    s.run(16.);
}
fn drag(s: &mut Sim<Ansuz>) {
    pointer(s, PointerPhase::Down, 0., 0.);
    for _ in 0..8 {
        pointer(s, PointerPhase::Move, 12., 8.);
    }
}
fn orientation(s: &Sim<Ansuz>) -> Quat {
    s.world().resource::<Navigation>().orientation
}
#[test]
fn every_cell_roundtrips_including_polar_faces() {
    for face in 0..6 {
        for u in 0..CELLS {
            for v in 0..CELLS {
                let cell = Cell { face, u, v };
                let center = cell_point(cell, 0.5, 0.5, RADIUS);
                assert_eq!(cell_at(center), cell);
                assert!((center.length() - RADIUS).abs() < 0.00001);
            }
        }
    }
}
#[test]
fn orbital_modes_preserve_relative_view_and_fixed_altitude_through_many_turns() {
    let mut q = Quat::IDENTITY;
    for _ in 0..2000 {
        q = turn(q, Vec2::new(0.013, 0.017));
        let (fly, sphere) = poses(q, true);
        let (fixed, spin) = poses(q, false);
        assert_eq!(sphere, Transform::default());
        assert_eq!(fixed, base_camera());
        assert!((fly.position.length() - RADIUS - ALTITUDE).abs() < 0.00002);
        assert!(
            fly.position
                .distance(spin.rotation.inverse() * fixed.position)
                < 0.00002
        );
        assert_eq!(target(q, true), target(q, false));
        assert!(target(q, true).is_some());
    }
    assert!(ray_sphere(Vec3::Z * 12., Vec3::Z).is_none());
}
#[test]
fn touch_without_mouse_buttons_moves_the_correct_object() {
    for mode in [0, 1] {
        let mut s = sim(mode);
        let before_camera = *s.world().require::<Transform>("camera");
        let before_sphere = *s.world().require::<Transform>("sphere");
        let before_cell = target(orientation(&s), mode == 1);
        drag(&mut s);
        assert_ne!(target(orientation(&s), mode == 1), before_cell);
        if mode == 0 {
            assert_eq!(*s.world().require::<Transform>("camera"), before_camera);
            assert_ne!(*s.world().require::<Transform>("sphere"), before_sphere);
        } else {
            assert_ne!(*s.world().require::<Transform>("camera"), before_camera);
            assert_eq!(*s.world().require::<Transform>("sphere"), before_sphere);
        }
    }
}
#[test]
fn stationary_contact_catches_drift_in_both_modes() {
    for mode in [0, 1] {
        let mut s = sim(mode);
        drag(&mut s);
        pointer(&mut s, PointerPhase::Up, 0., 0.);
        let released = orientation(&s);
        s.run(80.);
        assert_ne!(orientation(&s), released);
        pointer(&mut s, PointerPhase::Down, 0., 0.);
        let caught = orientation(&s);
        s.run(300.);
        assert_eq!(orientation(&s), caught);
        pointer(&mut s, PointerPhase::Move, 15., 0.);
        assert_ne!(orientation(&s), caught);
        pointer(&mut s, PointerPhase::Cancel, 0., 0.);
        let cancelled = orientation(&s);
        s.run(200.);
        assert_eq!(orientation(&s), cancelled);
    }
}
#[test]
fn reduced_motion_keeps_direct_input_without_release_drift() {
    let mut s = Sim::<Ansuz>::new(Options {
        reduced_motion: true,
        ..Default::default()
    })
    .unwrap();
    s.viewport(390., 500.);
    let start = orientation(&s);
    drag(&mut s);
    assert_ne!(orientation(&s), start);
    pointer(&mut s, PointerPhase::Up, 0., 0.);
    let released = orientation(&s);
    s.run(500.);
    assert_eq!(orientation(&s), released);
}
#[test]
fn switching_modes_keeps_cell_and_reset_restores_start() {
    let mut s = sim(0);
    drag(&mut s);
    pointer(&mut s, PointerPhase::Down, 0., 0.);
    s.run(150.);
    let q = orientation(&s);
    let cell = target(q, false);
    s.bind(
        &Options {
            mode: 1,
            ..Default::default()
        }
        .values(),
        None,
    )
    .unwrap();
    s.run(20.);
    assert_eq!(orientation(&s), q);
    assert_eq!(target(orientation(&s), true), cell);
    s.bind(
        &Options {
            mode: 1,
            home: 1,
            ..Default::default()
        }
        .values(),
        None,
    )
    .unwrap();
    s.run(20.);
    assert_eq!(orientation(&s), Navigation::default().orientation);
}
