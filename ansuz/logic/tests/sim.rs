use ansuz_logic::{Ansuz, Journey, Options, HIGH, LOW};
use exact_game::{Sim, Transform};

#[test]
fn selection_descent_and_return_preserve_geography() {
    let mut game: Sim<Ansuz> = exact_game::sim_with_art!(Options::default()).unwrap();
    game.viewport(800., 600.);
    game.tap("Enter");
    game.run(100.);
    assert_eq!(game.world().resource::<Journey>().altitude, HIGH);
    game.hold("KeyD", 500.);
    game.run(2500.);
    assert!(game.settle());
    let before = *game.world().resource::<Journey>();
    let marker = *game.get::<Transform>("reading-0-0").unwrap();
    game.tap("KeyN");
    game.run(100.);
    assert_eq!(game.world().resource::<Journey>().selected, 0);
    assert_eq!(game.world().resource::<Journey>().altitude, HIGH);
    game.tap("Enter");
    game.run(3000.);
    assert_eq!(game.world().resource::<Journey>().altitude, LOW);
    assert!(game.world().resource::<Journey>().entered);
    game.tap("Escape");
    game.run(3000.);
    let after = game.world().resource::<Journey>();
    assert!(!after.entered);
    assert_eq!(after.altitude, before.altitude);
    assert!((after.longitude - before.longitude).abs() < 0.00001);
    assert_eq!(after.latitude, before.latitude);
    assert_eq!(
        game.get::<Transform>("reading-0-0").unwrap().position,
        marker.position
    );
}

#[test]
fn reduced_motion_arrives_without_a_long_transition() {
    let mut game: Sim<Ansuz> = exact_game::sim_with_art!(Options {
        reduced: true,
        ..Options::default()
    })
    .unwrap();
    game.tap("KeyN");
    game.tap("Enter");
    game.run(30.);
    let journey = game.world().resource::<Journey>();
    assert_eq!(journey.altitude, LOW);
    assert!(!journey.travelling);
}

#[test]
fn dragging_coasts_and_a_new_touch_catches_it() {
    use exact_game::{Clock, InputEvent, PointerPhase};
    let mut game: Sim<Ansuz> = exact_game::sim_with_art!(Options::default()).unwrap();
    game.viewport(800., 600.);
    game.advance(0., Clock::Seekable);
    let mut pointer = |phase, x, dx, buttons, at_ms| {
        game.input(InputEvent::Pointer {
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
        game.advance(at_ms + 10., Clock::Seekable);
    };
    pointer(PointerPhase::Down, 400., 0., 1, 1.);
    pointer(PointerPhase::Move, 460., 60., 1, 21.);
    pointer(PointerPhase::Up, 460., 0., 0, 41.);
    let released = *game.world().resource::<Journey>();
    assert!(released.longitude < 0.);
    assert_eq!(released.selected, -1, "a drag must not select a clearing");
    game.advance(141., Clock::Seekable);
    assert!(
        game.world().resource::<Journey>().longitude < released.longitude,
        "release coasts"
    );
    game.input(InputEvent::Pointer {
        id: 1,
        phase: PointerPhase::Down,
        x: 460.,
        y: 300.,
        dx: 0.,
        dy: 0.,
        buttons: 1,
        at_ms: 142.,
        player: 0,
    });
    game.advance(155., Clock::Seekable);
    let caught = *game.world().resource::<Journey>();
    assert_eq!(caught.vx, 0.);
    game.advance(355., Clock::Seekable);
    assert_eq!(
        game.world().resource::<Journey>().longitude,
        caught.longitude
    );
}

#[test]
fn touching_a_visible_clearing_selects_without_descending() {
    use exact_game::{Clock, InputEvent, PointerPhase};
    let mut game: Sim<Ansuz> = exact_game::sim_with_art!(Options::default()).unwrap();
    game.viewport(800., 600.);
    game.advance(0., Clock::Seekable);
    let center = game.layout("place-0").unwrap().screen.center();
    for (phase, buttons, at_ms) in [(PointerPhase::Down, 1, 1.), (PointerPhase::Up, 0, 21.)] {
        game.input(InputEvent::Pointer {
            id: 1,
            phase,
            x: center.x,
            y: center.y,
            dx: 0.,
            dy: 0.,
            buttons,
            at_ms,
            player: 0,
        });
        game.advance(at_ms + 10., Clock::Seekable);
    }
    let journey = game.world().resource::<Journey>();
    assert_eq!(journey.selected, 0);
    assert_eq!(journey.altitude, HIGH);
    assert!(!journey.entered);
}
