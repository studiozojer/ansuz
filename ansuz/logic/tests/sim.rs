use ansuz_logic::{
    camera_pose, records::Dataset, Ansuz, Archive, Journey, Options, SurfaceRecords, ORBIT_HEIGHT,
    START_HEIGHT,
};
use exact_game::{Clock, InputEvent, PointerPhase, Sim, Transform, Vec3, Visible};
fn game() -> Sim<Ansuz> {
    let mut g = exact_game::sim_with_art!(Options::default()).unwrap();
    g.viewport(800., 600.);
    g
}
fn surface_game() -> Sim<Ansuz> {
    let mut g = game();
    g.tap("KeyF");
    g.run(1000.);
    g
}
fn pointer(g: &mut Sim<Ansuz>, phase: PointerPhase, x: f32, dx: f32, buttons: u32, at_ms: f64) {
    let at_ms = g.world().seconds() * 1000. + at_ms;
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
fn fresh_world_is_empty_and_starts_in_atmospheric_view() {
    let g = game();
    let j = *g.world().resource::<Journey>();
    assert_eq!(g.world().resource::<Dataset>().objects.len(), 0);
    assert!(g.world().resource::<SurfaceRecords>().placements.is_empty());
    assert!(g.get::<Transform>("place-0").is_none());
    assert!(g.get::<Transform>("reading-0-0").is_none());
    let camera = camera_pose(&j);
    assert_eq!(
        camera.position,
        ansuz_logic::normal(0., 1.05) * (120. + ORBIT_HEIGHT)
    );
    assert!(!j.surface);
    assert!((camera.rotation * -Vec3::Z).y < -0.2);
}
#[test]
fn quick_click_selects_a_real_top_without_creating_an_object() {
    let mut g = surface_game();
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
    let mut g = surface_game();
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
    let mut g = surface_game();
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

#[test]
fn descent_is_a_subview_and_ascent_restores_the_rotated_view() {
    let mut g = game();
    g.hold("KeyD", 300.);
    g.run(2500.);
    let before = *g.get::<Transform>("camera").unwrap();
    let records = g.world().resource::<Dataset>().clone();
    g.tap("KeyF");
    for _ in 0..10 {
        g.run(100.);
        assert!(g.get::<Transform>("camera").unwrap().position.length() > RADIUS_FOR_TEST);
    }
    let j = *g.world().resource::<Journey>();
    assert!(j.surface);
    assert_eq!(j.blend, 1.);
    let pose = camera_pose(&j);
    assert_eq!(pose.position, Vec3::new(0., 120. + START_HEIGHT, 0.));
    assert!((pose.rotation * -Vec3::Z + Vec3::Y).length() < 0.0001);
    g.tap("KeyR");
    g.run(1000.);
    assert_eq!(*g.get::<Transform>("camera").unwrap(), before);
    assert_eq!(*g.world().resource::<Dataset>(), records);
}
const RADIUS_FOR_TEST: f32 = 120.;
#[test]
fn atmospheric_drag_coasts_and_a_new_contact_catches_it() {
    let mut g = game();
    pointer(&mut g, PointerPhase::Down, 400., 0., 1, 1.);
    pointer(&mut g, PointerPhase::Move, 460., 60., 1, 21.);
    pointer(&mut g, PointerPhase::Up, 460., 0., 0, 21.);
    let lon = g.world().resource::<Journey>().longitude;
    assert!(lon < 0.);
    assert!(g.world().resource::<Journey>().selected.is_none());
    g.run(100.);
    assert!(g.world().resource::<Journey>().longitude < lon);
    pointer(&mut g, PointerPhase::Down, 460., 0., 1, 1.);
    let caught = g.world().resource::<Journey>().longitude;
    g.run(100.);
    assert_eq!(g.world().resource::<Journey>().longitude, caught);
}

#[test]
fn selected_surface_opens_one_turnable_tablet_without_creating_content() {
    use ansuz_logic::{
        pick,
        tablet::{hit, TabletView},
    };
    use exact_game::{PointLight, Vec2};
    let mut g = surface_game();
    assert!(!g.get::<Visible>("tablet").unwrap().0);
    let before = *g.get::<Transform>("camera").unwrap();
    g.tap("Enter");
    g.run(600.);
    let j = *g.world().resource::<Journey>();
    let cell = j.selected.unwrap();
    assert!(g.get::<Visible>("tablet").unwrap().0);
    assert_eq!(g.world().resource::<TabletView>().cell, Some(cell));
    let tablet = *g.get::<Transform>("tablet").unwrap();
    assert!(tablet.position.y > cell.center().y + 3.);
    assert_eq!(*g.get::<Transform>("camera").unwrap(), before);
    assert!(g.world().resource::<Dataset>().objects.is_empty());
    // A click on the risen object keeps its owning surface, even though the ray
    // would land on a neighbouring terrain cell behind the tilted tablet.
    let ray = (tablet.position - before.position).normalize();
    assert!(hit(g.world(), before.position, ray).is_some());
    let local = before.rotation.inverse() * (tablet.position - before.position);
    let clip = exact_game::Camera::default()
        .matrix(Vec2::new(800., 600.))
        .project_point3(local);
    let screen = Vec2::new((clip.x + 1.) * 400., (1. - clip.y) * 300.);
    assert_eq!(
        pick(g.world(), screen, Vec2::new(800., 600.), &j),
        Some(cell)
    );
    g.tap("KeyE");
    g.run(40.);
    assert_ne!(
        g.get::<Transform>("tablet").unwrap().rotation,
        tablet.rotation
    );
    assert_eq!(g.world().resource::<TabletView>().cell, Some(cell));
    g.tap("Escape");
    g.run(40.);
    assert!(!g.get::<Visible>("tablet").unwrap().0);
    assert_eq!(g.get::<PointLight>("tablet-light").unwrap().intensity, 0.);
    assert!(g.world().resource::<Dataset>().objects.is_empty());
}

#[test]
fn atmospheric_start_exposes_selectable_tops() {
    let mut g = game();
    g.tap("Enter");
    g.run(600.);
    assert!(g.world().resource::<Journey>().selected.is_some());
    assert!(g.get::<Visible>("tablet").unwrap().0);
    assert!(g.world().resource::<Dataset>().objects.is_empty());
}
