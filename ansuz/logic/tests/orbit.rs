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

fn queue_pointer(s: &mut Sim<Ansuz>, phase: PointerPhase, x: f32, dx: f32, mouse: bool) {
    s.input(InputEvent::Pointer {
        id: 9,
        phase,
        x,
        y: 250.,
        dx,
        dy: 0.,
        buttons: if mouse && matches!(phase, PointerPhase::Down | PointerPhase::Move) {
            1
        } else {
            0
        },
        at_ms: 0.,
        player: 0,
    });
}
#[test]
fn quick_flick_between_ticks_moves_and_launches_drift() {
    for mode in [0, 1] {
        for mouse in [false, true] {
            for terminal_only in [false, true] {
                let mut s = sim(mode);
                let before = orientation(&s);
                // Hover earlier in the same batch must not become contact travel.
                queue_pointer(&mut s, PointerPhase::Move, 140., 300., false);
                queue_pointer(&mut s, PointerPhase::Down, 140., 0., mouse);
                if !terminal_only {
                    queue_pointer(&mut s, PointerPhase::Move, 200., 60., mouse);
                }
                queue_pointer(
                    &mut s,
                    PointerPhase::Up,
                    220.,
                    if terminal_only { 0. } else { 20. },
                    mouse,
                );
                s.run(10.);
                let released = orientation(&s);
                let travel = released.angle_between(before);
                assert!(
                    travel > 0.06 && travel < 0.09,
                    "Contact's 80-point travel was lost or mixed with hover: {travel}"
                );
                assert!(s.world().resource::<Navigation>().velocity.x > 0.);
                s.run(80.);
                assert_ne!(
                    orientation(&s),
                    released,
                    "Quick flick did not launch drift"
                );
            }
        }
    }
}
#[test]
fn release_tick_keeps_its_final_movement() {
    let mut s = sim(0);
    queue_pointer(&mut s, PointerPhase::Down, 140., 0., false);
    s.run(10.);
    let before = orientation(&s);
    queue_pointer(&mut s, PointerPhase::Move, 200., 60., false);
    queue_pointer(&mut s, PointerPhase::Up, 220., 20., false);
    s.run(10.);
    assert!(
        orientation(&s).angle_between(before) > 0.06,
        "Movement immediately before Up was lost"
    );
}
#[test]
fn tap_and_cancel_in_one_tick_catch_without_spurious_motion() {
    for cancelled in [false, true] {
        let mut s = sim(0);
        drag(&mut s);
        pointer(&mut s, PointerPhase::Up, 0., 0.);
        let before = orientation(&s);
        queue_pointer(&mut s, PointerPhase::Down, 140., 0., false);
        if cancelled {
            queue_pointer(&mut s, PointerPhase::Move, 220., 80., false);
        }
        queue_pointer(
            &mut s,
            if cancelled {
                PointerPhase::Cancel
            } else {
                PointerPhase::Up
            },
            140.,
            0.,
            false,
        );
        s.run(10.);
        assert_eq!(
            orientation(&s),
            before,
            "Tap/cancel must catch existing drift"
        );
        s.run(100.);
        assert_eq!(orientation(&s), before);
    }
}
#[test]
fn reduced_motion_keeps_quick_flick_displacement_but_not_drift_or_hover() {
    let mut s = Sim::<Ansuz>::new(Options {
        reduced_motion: true,
        ..Default::default()
    })
    .unwrap();
    s.viewport(390., 500.);
    let start = orientation(&s);
    queue_pointer(&mut s, PointerPhase::Down, 140., 0., false);
    queue_pointer(&mut s, PointerPhase::Move, 220., 80., false);
    queue_pointer(&mut s, PointerPhase::Up, 220., 0., false);
    s.run(10.);
    let released = orientation(&s);
    assert!(released.angle_between(start) > 0.06);
    s.run(150.);
    assert_eq!(orientation(&s), released);
    queue_pointer(&mut s, PointerPhase::Move, 300., 80., false);
    s.run(10.);
    assert_eq!(
        orientation(&s),
        released,
        "Hover after release rotated the sphere"
    );
}
