use ansuz_logic::{
    camera_pose, records::Dataset, Ansuz, Archive, Journey, Options, SurfaceRecords, START_HEIGHT,
};
use exact_game::{Clock, InputEvent, PointerPhase, Sim, Transform, Vec3, Visible};
fn game() -> Sim<Ansuz> {
    let mut g = exact_game::sim_with_art!(Options::default()).unwrap();
    g.viewport(800., 600.);
    g
}
fn pointer(g: &mut Sim<Ansuz>, phase: PointerPhase, x: f32, dx: f32, buttons: u32, at_ms: f64) {
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
fn fresh_world_is_empty_and_camera_looks_down_from_north_pole() {
    let g = game();
    let j = *g.world().resource::<Journey>();
    assert_eq!(g.world().resource::<Dataset>().objects.len(), 0);
    assert!(g.world().resource::<SurfaceRecords>().placements.is_empty());
    assert!(g.get::<Transform>("place-0").is_none());
    assert!(g.get::<Transform>("reading-0-0").is_none());
    let camera = camera_pose(&j);
    assert_eq!(camera.position, Vec3::new(0., 120. + START_HEIGHT, 0.));
    assert!((camera.rotation * -Vec3::Z + Vec3::Y).length() < 0.0001);
}
#[test]
fn quick_click_selects_a_real_top_without_creating_an_object() {
    let mut g = game();
    pointer(&mut g, PointerPhase::Down, 420., 0., 1, 1.);
    pointer(&mut g, PointerPhase::Up, 420., 0., 0, 21.);
    let cell = g
        .world()
        .resource::<Journey>()
        .selected
        .expect("real surface");
    assert!(cell.center().y > 115.);
    assert!(g.get::<Visible>("selected-face").unwrap().0);
    assert!(g.world().resource::<Dataset>().objects.is_empty());
}
#[test]
fn drag_moves_grid_without_selecting_and_release_is_still() {
    let mut g = game();
    pointer(&mut g, PointerPhase::Down, 400., 0., 1, 1.);
    pointer(&mut g, PointerPhase::Move, 460., 60., 1, 21.);
    pointer(&mut g, PointerPhase::Up, 460., 0., 0, 41.);
    let before = *g.world().resource::<Journey>();
    assert!(before.x < 0.);
    assert!(before.selected.is_none());
    g.run(500.);
    assert_eq!(g.world().resource::<Journey>().x, before.x);
    g.tap("Home");
    g.run(30.);
    assert_eq!(g.world().resource::<Journey>().x, 0.);
}
#[test]
fn keyboard_can_select_and_clear_a_surface() {
    let mut g = game();
    g.tap("Enter");
    g.run(30.);
    assert!(g.world().resource::<Journey>().selected.is_some());
    g.tap("Escape");
    g.run(30.);
    assert!(g.world().resource::<Journey>().selected.is_none());
}
#[test]
fn stored_object_is_referenced_by_placement_and_survives_camera_navigation() {
    let json = r#"{"version":1,"next_id":2,"objects":[{"id":"note-1","title":"Question","body":"A thought","content_kind":"note","authored_weight":1}],"relations":[],"placements":[{"object_id":"note-1","x":0,"y":47,"z":0}]}"#;
    let archive: Archive = exact_game::json::from_str(json).unwrap();
    assert_eq!(archive.objects[0].id, archive.placements[0].object_id);
    let mut g: Sim<Ansuz> = exact_game::sim_with_art!(Options {
        records: json.into(),
        editing: false
    })
    .unwrap();
    g.viewport(800., 600.);
    g.hold("KeyD", 400.);
    g.tap("Home");
    g.run(30.);
    assert_eq!(g.world().resource::<Dataset>().objects[0].id, "note-1");
    assert_eq!(g.world().resource::<Dataset>().objects[0].body, "A thought");
}
