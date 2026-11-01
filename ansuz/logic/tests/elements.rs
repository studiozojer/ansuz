use ansuz_logic::{elements::Elements, lab::Contact};
use exact_game::*;
const VIEW: Vec2 = Vec2::new(400., 500.);
fn step(e: &mut Elements, p: Vec2, began: bool, down: bool, ticks: usize) {
    for i in 0..ticks {
        e.step(
            &Contact {
                present: true,
                id: 1,
                position: VIEW * 0.5 + p * 250.,
                origin: VIEW * 0.5,
                has_origin: down,
                down,
                began: began && i == 0,
                ended: !down && i == 0,
                ..Default::default()
            },
            VIEW,
            1. / 120.,
            false,
        );
    }
}
fn new(kind: u32) -> Elements {
    let mut e = Elements::default();
    e.reset(kind);
    e
}
#[test]
fn fire_needs_stillness_preparation_and_a_fast_finish_then_cools() {
    let mut e = new(1);
    step(&mut e, Vec2::ZERO, true, true, 1);
    step(&mut e, Vec2::X * 0.25, false, true, 1);
    assert_eq!(e.triggers, 0, "an unprepared flick is not fire");
    e.reset(1);
    step(&mut e, Vec2::ZERO, true, true, 70);
    assert_eq!(e.phase, 2);
    step(&mut e, Vec2::X * 0.03, false, true, 10);
    step(&mut e, Vec2::X * 0.23, false, true, 1);
    assert_eq!(e.triggers, 1);
    assert!(e.energy > 0.95);
    step(&mut e, Vec2::X * 0.23, false, false, 700);
    assert_eq!(e.energy, 0.);
    assert_eq!(e.phase, 0);
}
#[test]
fn earth_requires_both_holds_and_rejects_an_off_axis_lift() {
    let mut e = new(3);
    step(&mut e, Vec2::ZERO, true, true, 70);
    assert_eq!(e.phase, 2);
    step(&mut e, Vec2::new(0.2, -0.32), false, true, 45);
    assert_eq!(e.triggers, 0);
    e.reset(3);
    step(&mut e, Vec2::ZERO, true, true, 70);
    step(&mut e, Vec2::new(0., -0.32), false, true, 10);
    assert_eq!(e.triggers, 0, "the finishing pose must be held");
    step(&mut e, Vec2::new(0., -0.32), false, true, 30);
    assert_eq!(e.triggers, 1);
    assert_eq!(e.phase, 4);
}
#[test]
fn water_needs_winding_and_keeps_a_bounded_body_at_its_original_center() {
    let mut e = new(2);
    step(&mut e, Vec2::ZERO, true, true, 1);
    for i in 0..50 {
        step(&mut e, Vec2::X * (i as f32 * 0.004), false, true, 1);
    }
    assert_eq!(e.triggers, 0, "a straight drag is not a vortex");
    for i in 0..180 {
        let t = i as f32 * core::f32::consts::TAU / 90.;
        step(
            &mut e,
            Vec2::new(math::cos(t), math::sin(t)) * 0.2,
            false,
            true,
            2,
        );
    }
    assert_eq!(e.triggers, 1);
    assert_eq!(e.object, Vec2::ZERO);
    assert!(e.energy > 0.3);
    assert!(e.particles[..144]
        .iter()
        .any(|p| p.velocity.length() > 0.01));
    step(&mut e, Vec2::ZERO, false, false, 900);
    for p in &e.particles[..144] {
        assert!(p.position.is_finite());
        assert!(p.position.length() < 0.3);
    }
}
#[test]
fn air_is_continuous_and_only_a_grab_stirs_it() {
    let mut e = new(4);
    step(&mut e, Vec2::new(0.6, 0.6), true, true, 1);
    step(&mut e, Vec2::ZERO, false, true, 1);
    assert_eq!(e.energy, 0.);
    step(&mut e, Vec2::ZERO, true, true, 1);
    for i in 0..60 {
        step(
            &mut e,
            Vec2::new(math::sin(i as f32 * 0.15) * 0.35, 0.),
            false,
            true,
            1,
        );
    }
    assert!(e.energy > 0.1);
    assert!(e.particles.iter().any(|p| p.velocity.length() > 0.05));
    let before = e.particles.iter().map(|p| p.velocity.length()).sum::<f32>();
    step(&mut e, Vec2::ZERO, false, false, 480);
    assert!(e.particles.iter().map(|p| p.velocity.length()).sum::<f32>() < before * 0.1);
}
#[test]
fn cancelled_contact_cannot_finish_a_charged_strike() {
    let mut e = new(1);
    step(&mut e, Vec2::ZERO, true, true, 70);
    step(&mut e, Vec2::X * 0.03, false, true, 10);
    step(&mut e, Vec2::X * 0.3, false, false, 1);
    assert_eq!(e.triggers, 0);
    assert!(!e.grabbed);
}
