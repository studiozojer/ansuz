use ansuz_logic::{
    lab::{Lab, MAX_FRAMES},
    Ansuz, Navigation, Options,
};
use exact_game::*;
fn sim() -> Sim<Ansuz> {
    let mut s = Sim::new(Options {
        lab: true,
        flat: true,
        ..Default::default()
    })
    .unwrap();
    s.viewport(390., 500.);
    s.run(20.);
    s
}
fn contact(s: &mut Sim<Ansuz>, id: u64, phase: PointerPhase, x: f32, dx: f32) {
    s.input(InputEvent::Pointer {
        id,
        phase,
        x,
        y: 250.,
        dx,
        dy: 0.,
        buttons: if matches!(phase, PointerPhase::Down | PointerPhase::Move) {
            1
        } else {
            0
        },
        at_ms: 0.,
        player: 0,
    });
    s.run(16.);
}
fn take(s: &mut Sim<Ansuz>) -> String {
    s.post("lab:record|100");
    s.run(16.);
    s.post("finger|down|41|140|250|1|110");
    contact(s, 1, PointerPhase::Down, 140., 0.);
    contact(s, 1, PointerPhase::Move, 210., 70.);
    contact(s, 1, PointerPhase::Up, 210., 0.);
    s.run(80.);
    contact(s, 2, PointerPhase::Down, 220., 0.);
    contact(s, 2, PointerPhase::Move, 222., 2.);
    s.run(200.);
    s.viewport(390., 300.);
    contact(s, 2, PointerPhase::Move, 225., 3.);
    contact(s, 2, PointerPhase::Up, 225., 0.);
    s.post("lab:stop");
    s.run(16.);
    s.take_messages().pop().unwrap()
}
#[test]
fn captured_input_and_observer_tracks_replay_the_same_motion() {
    let mut s = sim();
    let text = take(&mut s);
    let saved = serde_json::from_str::<serde_json::Value>(&text).unwrap();
    assert_eq!(saved["observed"].as_array().unwrap().len(), 1);
    assert_eq!(saved["observed"][0][3], 41); // Observer ID is intentionally not engine ID.
    let expected = s.world().resource::<Lab>().frames.clone();
    s.viewport(320., 700.);
    s.post("lab:reset");
    s.run(16.);
    s.post("lab:replay");
    let mut checked = 0;
    let now = s.world().tick() as f64 * 1000. / 120.;
    s.advance_with(
        now + expected.len() as f64 * 1000. / 120. + 50.,
        Clock::Seekable,
        |w, _| {
            let lab = w.resource::<Lab>();
            if lab.cursor as usize > checked {
                let i = lab.cursor as usize - 1;
                let n = w.resource::<Navigation>();
                assert!(n.orientation.abs_diff_eq(expected[i].orientation, 0.000001));
                assert!(lab.position.abs_diff_eq(expected[i].position, 0.000001));
                checked = lab.cursor as usize;
            }
        },
    );
    assert_eq!(checked, expected.len());
    assert!(!s.world().resource::<Lab>().replaying);
}
#[test]
fn take_roundtrips_and_invalid_import_keeps_previous_recording() {
    let mut s = sim();
    let text = take(&mut s);
    let mut loaded = Lab::default();
    loaded.load(&text).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&loaded.export()).unwrap(),
        serde_json::from_str::<serde_json::Value>(&text).unwrap()
    );
    let before = loaded.export();
    let mut broken: serde_json::Value = serde_json::from_str(&text).unwrap();
    broken["initial"]["pointer"] = serde_json::json!(-1);
    assert!(loaded.load(&broken.to_string()).is_err());
    assert_eq!(loaded.export(), before);
    assert!(loaded.load("{}").is_err());
    assert_eq!(loaded.export(), before);
}
#[test]
fn recording_is_bounded_and_a_live_contact_interrupts_replay() {
    let mut s = sim();
    s.post("lab:record|0");
    s.run(21000.);
    assert!(!s.world().resource::<Lab>().recording);
    assert_eq!(s.world().resource::<Lab>().frames.len(), MAX_FRAMES);
    assert_eq!(s.take_messages().len(), 1);
    s.post("lab:replay");
    s.run(80.);
    assert!(s.world().resource::<Lab>().replaying);
    contact(&mut s, 91, PointerPhase::Down, 200., 0.);
    assert!(!s.world().resource::<Lab>().replaying);
    assert!(s.world().resource::<Navigation>().down);
}

#[test]
fn legacy_takes_use_their_recorded_viewport_and_motion_preference() {
    let mut s = sim();
    let mut old: serde_json::Value = serde_json::from_str(&take(&mut s)).unwrap();
    old["version"] = serde_json::json!(1);
    for frame in old["frames"].as_array_mut().unwrap() {
        frame.as_array_mut().unwrap().truncate(15);
    }
    let mut loaded = Lab::default();
    loaded.load(&old.to_string()).unwrap();
    assert!(loaded
        .frames
        .iter()
        .all(|f| f.viewport == loaded.viewport && f.reduced_motion == loaded.reduced_motion));
}

#[test]
fn elemental_take_replays_the_recognizer_and_response_instead_of_playing_back_results() {
    use ansuz_logic::elements::Elements;
    let mut s = Sim::<Ansuz>::new(Options {
        lab: true,
        flat: true,
        element: 1,
        ..Default::default()
    })
    .unwrap();
    s.viewport(390., 500.);
    s.run(20.);
    s.post("lab:record|0");
    s.run(16.);
    contact(&mut s, 1, PointerPhase::Down, 195., 0.);
    s.run(600.);
    contact(&mut s, 1, PointerPhase::Move, 202.5, 7.5);
    s.run(80.);
    contact(&mut s, 1, PointerPhase::Move, 255., 52.5);
    assert_eq!(s.world().resource::<Elements>().triggers, 1);
    s.post("lab:stop");
    s.run(16.);
    let text = s.take_messages().pop().unwrap();
    let expected = s.world().resource::<Lab>().frames.clone();
    let mut loaded = Lab::default();
    loaded.load(&text).unwrap();
    assert_eq!(loaded.source_element, 1);
    assert!(expected.iter().any(|f| f.effect_energy > 0.95));
    s.post("lab:replay");
    let now = s.world().tick() as f64 * 1000. / 120.;
    let mut checked = 0;
    s.advance_with(
        now + expected.len() as f64 * 1000. / 120. + 25.,
        Clock::Seekable,
        |w, _| {
            let lab = w.resource::<Lab>();
            if lab.cursor as usize > checked {
                let f = expected[lab.cursor as usize - 1];
                let e = w.resource::<Elements>();
                assert!((e.energy - f.effect_energy).abs() < 0.000001);
                assert_eq!(e.phase, f.effect_phase);
                assert!(e.object.abs_diff_eq(f.effect_position, 0.000001));
                checked = lab.cursor as usize;
            }
        },
    );
    assert_eq!(checked, expected.len());
}

#[test]
fn version_two_motion_takes_remain_readable() {
    let mut s = sim();
    let mut j: serde_json::Value = serde_json::from_str(&take(&mut s)).unwrap();
    j["version"] = serde_json::json!(2);
    for f in j["frames"].as_array_mut().unwrap() {
        f.as_array_mut().unwrap().truncate(18);
    }
    j.as_object_mut().unwrap().remove("element");
    j.as_object_mut().unwrap().remove("element_model");
    let mut loaded = Lab::default();
    loaded.load(&j.to_string()).unwrap();
    assert_eq!(loaded.source_element, 0);
}
