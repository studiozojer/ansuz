use ansuz_logic::{lab::Lab, Ansuz, Options};
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
fn pointer(s: &mut Sim<Ansuz>, id: u64, phase: PointerPhase, x: f32, y: f32) {
    s.input(InputEvent::Pointer {
        id,
        phase,
        x,
        y,
        dx: 0.,
        dy: 0.,
        buttons: 0,
        at_ms: 0.,
        player: 0,
    });
}
fn begin(s: &mut Sim<Ansuz>) {
    s.post("lab:record|0");
    s.run(16.);
}
fn finish(s: &mut Sim<Ansuz>) -> String {
    s.post("lab:stop");
    s.run(16.);
    s.take_messages().pop().unwrap()
}
#[test]
fn simultaneous_contacts_release_independently_and_roundtrip_for_replay() {
    let mut s = sim();
    begin(&mut s);
    pointer(&mut s, 17, PointerPhase::Down, 100., 200.);
    pointer(&mut s, 32, PointerPhase::Down, 280., 200.);
    s.run(16.);
    assert_eq!(s.world().resource::<Lab>().pointers.active.len(), 2);
    pointer(&mut s, 17, PointerPhase::Move, 80., 170.);
    pointer(&mut s, 32, PointerPhase::Move, 300., 230.);
    s.run(16.);
    pointer(&mut s, 17, PointerPhase::Up, 80., 170.);
    s.run(16.);
    assert_eq!(s.world().resource::<Lab>().pointers.active[0].id, 32);
    pointer(&mut s, 32, PointerPhase::Move, 320., 250.);
    s.run(16.);
    pointer(&mut s, 32, PointerPhase::Cancel, 320., 250.);
    s.run(16.);
    assert!(s.world().resource::<Lab>().pointers.active.is_empty());
    let text = finish(&mut s);
    let j: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(j["version"], 4);
    let mut l = Lab::default();
    l.load(&text).unwrap();
    let first = l.pointers.events[0].tick;
    assert_eq!(l.pointers.events.len(), 7);
    assert_eq!(l.pointers.events[6].phase, 3);
    l.pointers.reset_replay();
    l.pointers.replay(first);
    assert_eq!(l.pointers.active.len(), 2);
    l.pointers.replay(l.frames.len() as u32);
    assert!(l.pointers.active.is_empty());
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&l.export()).unwrap(),
        j
    );
}
#[test]
fn quick_contact_edges_survive_even_when_down_and_up_share_a_tick() {
    let mut s = sim();
    begin(&mut s);
    pointer(&mut s, 88, PointerPhase::Down, 30., 40.);
    pointer(&mut s, 88, PointerPhase::Up, 30., 40.);
    s.run(16.);
    let text = finish(&mut s);
    let mut l = Lab::default();
    l.load(&text).unwrap();
    assert_eq!(l.pointers.events.len(), 2);
    assert_eq!(l.pointers.events[0].tick, l.pointers.events[1].tick);
}
#[test]
fn recording_mid_contact_seeds_the_existing_fingers() {
    let mut s = sim();
    pointer(&mut s, 8, PointerPhase::Down, 30., 40.);
    s.run(16.);
    begin(&mut s);
    pointer(&mut s, 8, PointerPhase::Move, 100., 50.);
    s.run(16.);
    pointer(&mut s, 8, PointerPhase::Up, 100., 50.);
    s.run(16.);
    let mut l = Lab::default();
    l.load(&finish(&mut s)).unwrap();
    assert_eq!(l.pointers.initial.len(), 1);
    l.pointers.reset_replay();
    assert_eq!(l.pointers.active[0].id, 8);
    l.pointers.replay(l.frames.len() as u32);
    assert!(l.pointers.active.is_empty());
}
#[test]
fn background_blur_stops_and_exports_an_explicitly_interrupted_take() {
    let mut s = sim();
    begin(&mut s);
    pointer(&mut s, 5, PointerPhase::Down, 90., 80.);
    s.run(16.);
    s.input(InputEvent::Blur {
        at_ms: 0.,
        player: 0,
    });
    s.run(300.);
    let l = s.world().resource::<Lab>();
    assert!(!l.recording);
    assert!(l.pointers.active.is_empty());
    drop(l);
    let text = s.take_messages().pop().unwrap();
    let j: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(j["capture"]["stop_reason"], "interrupted");
    assert_eq!(j["delivered"].as_array().unwrap().last().unwrap()[2], 4);
    Lab::default().load(&text).unwrap();
}
#[test]
fn too_many_contacts_stop_capture_instead_of_silently_losing_fingers() {
    let mut s = sim();
    begin(&mut s);
    for id in 0..11 {
        pointer(&mut s, id, PointerPhase::Down, 50., 50.);
    }
    s.run(16.);
    assert!(!s.world().resource::<Lab>().recording);
    let text = s.take_messages().pop().unwrap();
    let j: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(j["capture"]["stop_reason"], "input_limit");
    assert_eq!(j["capture"]["dropped_events"], 1);
    Lab::default().load(&text).unwrap();
}
#[test]
fn malformed_multitouch_does_not_replace_a_loaded_take() {
    let mut s = sim();
    begin(&mut s);
    pointer(&mut s, 50_000_001, PointerPhase::Down, 50., 50.);
    s.run(16.);
    let text = finish(&mut s);
    let mut l = Lab::default();
    l.load(&text).unwrap();
    let before = l.export();
    let mut j: serde_json::Value = serde_json::from_str(&text).unwrap();
    j["delivered"][0][2] = serde_json::json!(1);
    assert!(l.load(&j.to_string()).is_err());
    assert_eq!(l.export(), before);
    assert_eq!(l.pointers.events[0].id, 50_000_001);
    j = serde_json::from_str(&text).unwrap();
    j["delivered"][0][1] = serde_json::json!(1234567890.12345);
    l.load(&j.to_string()).unwrap();
    assert_eq!(l.pointers.events[0].ms, 1234567890.12345);
}

#[test]
fn host_timestamp_preserves_every_bit_through_repeated_json_roundtrips() {
    let mut s = sim();
    begin(&mut s);
    pointer(&mut s, 1, PointerPhase::Down, 10., 20.);
    s.run(16.);
    let mut j: serde_json::Value = serde_json::from_str(&finish(&mut s)).unwrap();
    let original = 939.8999996185303_f64;
    j["delivered"][0][1] = serde_json::json!(original);
    let mut text = j.to_string();
    for _ in 0..10 {
        let mut l = Lab::default();
        l.load(&text).unwrap();
        assert_eq!(l.pointers.events[0].ms.to_bits(), original.to_bits());
        text = l.export();
    }
}

#[test]
fn touching_during_replay_interrupts_it_and_keeps_the_new_live_finger() {
    let mut s = sim();
    begin(&mut s);
    pointer(&mut s, 1, PointerPhase::Down, 10., 20.);
    s.run(100.);
    pointer(&mut s, 1, PointerPhase::Up, 10., 20.);
    s.run(100.);
    finish(&mut s);
    s.post("lab:replay");
    s.run(20.);
    assert!(s.world().resource::<Lab>().replaying);
    pointer(&mut s, 99, PointerPhase::Down, 100., 200.);
    s.run(16.);
    let l = s.world().resource::<Lab>();
    assert!(!l.replaying);
    assert_eq!(l.pointers.active.len(), 1);
    assert_eq!(l.pointers.active[0].id, 99);
}

#[test]
fn explicit_stop_wins_the_canvas_focus_handoff_without_duplicate_save() {
    let mut s = sim();
    begin(&mut s);
    pointer(&mut s, 1, PointerPhase::Down, 10., 20.);
    s.run(30.);
    pointer(&mut s, 1, PointerPhase::Up, 10., 20.);
    s.input(InputEvent::Blur {
        at_ms: 0.,
        player: 0,
    });
    s.run(16.);
    assert!(
        s.world().resource::<Lab>().recording,
        "Stop must remain available during its focus handoff"
    );
    let text = finish(&mut s);
    let j: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(j["capture"]["stop_reason"], "manual");
    s.run(500.);
    assert!(s.take_messages().is_empty());
}
