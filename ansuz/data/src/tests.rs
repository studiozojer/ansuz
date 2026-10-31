use super::*;
use exact_data::envelope::base64;

fn args(x: f64, title: &str, body: &str) -> Vec<Value> {
    vec![
        Value::Number(x),
        Value::Number(0.),
        Value::Number(-8.),
        Value::Number(0.),
        Value::str(title),
        Value::str(body),
    ]
}

fn reply(data: &mut AppData, source: &str, result: Json) -> Answer {
    data.parse(
        &mut Store::default(),
        source,
        &[],
        Outcome::Storage(result.to_string().into_bytes()),
    )
    .unwrap()
}

fn load(data: &mut AppData, document: Option<&Json>) -> Answer {
    assert!(matches!(
        data.answer(&mut Store::default(), "loadSurfaceNotes", &[])
            .unwrap(),
        Answer::Later(_)
    ));
    assert!(matches!(
        reply(data, "loadSurfaceNotes", Json::Null),
        Answer::Later(_)
    ));
    let listing = reply(
        data,
        "loadSurfaceNotes",
        if document.is_some() {
            json!([FILE])
        } else {
            json!([])
        },
    );
    if let Some(document) = document {
        assert!(matches!(listing, Answer::Later(_)));
        reply(
            data,
            "loadSurfaceNotes",
            json!({"base64":base64(document.to_string().as_bytes())}),
        )
    } else {
        listing
    }
}

#[test]
fn fresh_library_is_empty_and_create_edit_roundtrip_keep_identity() {
    let mut data = AppData::default();
    assert_eq!(
        data.query("loadSurfaceNotes", &[]).unwrap(),
        library_value(&empty(), "", false)
    );
    assert_eq!(load(&mut data, None), Answer::Now(library(&empty(), "")));
    assert!(matches!(
        data.answer(
            &mut Store::default(),
            "saveSurfaceNote",
            &args(7., "First thought", "Body")
        )
        .unwrap(),
        Answer::Later(_)
    ));
    // Pending writes are invisible until the host acknowledges durable success.
    assert!(data.document.as_ref().unwrap()["objects"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        reply(&mut data, "saveSurfaceNote", Json::Null),
        Answer::Now(saved(true, ""))
    );
    let created = data.document.clone().unwrap();
    let id = created["objects"][0]["id"].clone();
    assert_eq!(created["placements"][0]["object_id"], id);
    assert_eq!(created["objects"][0]["body"], "Body");
    assert!(created["objects"][0].get("x").is_none());
    assert!(created["relations"].as_array().unwrap().is_empty());
    let mut reloaded = AppData::default();
    assert_eq!(
        load(&mut reloaded, Some(&created)),
        Answer::Now(library(&created, ""))
    );
    reloaded
        .answer(
            &mut Store::default(),
            "saveSurfaceNote",
            &args(7., "Changed", "New body"),
        )
        .unwrap();
    reply(&mut reloaded, "saveSurfaceNote", Json::Null);
    let edited = reloaded.document.as_ref().unwrap();
    assert_eq!(edited["objects"].as_array().unwrap().len(), 1);
    assert_eq!(edited["objects"][0]["id"], id);
    assert_eq!(edited["objects"][0]["title"], "Changed");
    assert_eq!(edited["placements"], created["placements"]);
    assert_eq!(edited["relations"], created["relations"]);
}

#[test]
fn failed_write_preserves_committed_data_and_can_be_retried() {
    let mut data = AppData::default();
    load(&mut data, None);
    data.answer(
        &mut Store::default(),
        "saveSurfaceNote",
        &args(3., "Note", ""),
    )
    .unwrap();
    assert_eq!(
        data.answer(
            &mut Store::default(),
            "saveSurfaceNote",
            &args(4., "Other", "")
        )
        .unwrap(),
        Answer::Now(saved(false, "Wait for the current storage operation."))
    );
    assert_eq!(
        reply(&mut data, "saveSurfaceNote", json!({"error":"Disk full"})),
        Answer::Now(saved(false, "Disk full"))
    );
    assert_eq!(data.document, Some(empty()));
    assert!(matches!(
        data.answer(
            &mut Store::default(),
            "saveSurfaceNote",
            &args(3., "Note", "")
        )
        .unwrap(),
        Answer::Later(_)
    ));
    reply(&mut data, "saveSurfaceNote", Json::Null);
    assert_eq!(
        data.document.as_ref().unwrap()["objects"][0]["id"],
        "note-1"
    );
}

#[test]
fn unreadable_existing_library_never_becomes_a_writable_empty_library() {
    let mut data = AppData::default();
    let corrupted = json!({"version":99});
    let result = load(&mut data, Some(&corrupted));
    assert!(matches!(result, Answer::Now(_)));
    assert!(data.document.is_none());
    assert!(!data.load_error.is_empty());
    assert_eq!(
        data.answer(
            &mut Store::default(),
            "saveSurfaceNote",
            &args(0., "Overwrite?", "")
        )
        .unwrap(),
        Answer::Now(saved(false, &data.load_error))
    );
    assert!(data.pending.is_none());
}

#[test]
fn edit_preserves_provenance_and_layout_does_not_define_identity() {
    let first = edit(&empty(), &args(1., "Root", "one")).unwrap();
    let mut second = edit(&first, &args(2., "Child", "two")).unwrap();
    second["relations"] =
        json!([{"from":"note-2","to":"note-1","kind":"DerivedFrom","source_anchor":"image"}]);
    second["objects"][0]["authored_weight"] = json!(2.5);
    second["objects"][0]["content_kind"] = json!("iching");
    // Moving a projection placement leaves records and provenance alone.
    second["placements"][0]["x"] = json!(100);
    let edited = edit(&second, &args(100., "Root revised", "three")).unwrap();
    assert_eq!(edited["objects"][0]["id"], "note-1");
    assert_eq!(edited["objects"][0]["authored_weight"], 2.5);
    assert_eq!(edited["objects"][0]["content_kind"], "iching");
    assert_eq!(edited["relations"], second["relations"]);
    assert_eq!(edited["next_id"], second["next_id"]);
}

#[test]
fn invalid_edits_and_broken_placements_are_refused_without_mutation() {
    let body_only = edit(&empty(), &args(0., "  ", "A thought")).unwrap();
    assert_eq!(body_only["objects"][0]["title"], "Untitled note");
    assert_eq!(body_only["objects"][0]["body"], "A thought");
    assert!(edit(&empty(), &args(0.5, "Fractional cell", "")).is_err());
    assert!(edit(&empty(), &args(f64::NAN, "NaN cell", "")).is_err());
    assert!(edit(&empty(), &args(0., "  ", "")).is_err());
    let mut broken = empty();
    broken["placements"] = json!([{"object_id":"missing","x":1,"y":0,"z":0}]);
    assert!(validate(&broken).is_err());
    let mut duplicate = edit(&empty(), &args(1., "One", "")).unwrap();
    let placement = duplicate["placements"][0].clone();
    duplicate["placements"]
        .as_array_mut()
        .unwrap()
        .push(placement);
    assert!(validate(&duplicate).is_err());
}

#[test]
fn legacy_top_and_new_sides_keep_separate_ids_and_survive_reload() {
    let mut legacy = edit(&empty(), &args(2., "Original top", "Keep this")).unwrap();
    legacy["placements"][0]
        .as_object_mut()
        .unwrap()
        .remove("face");
    validate(&legacy).unwrap();
    let mut data = AppData::default();
    load(&mut data, Some(&legacy));
    assert_eq!(
        data.document.as_ref().unwrap(),
        &legacy,
        "read never rewrites the old file"
    );
    let top_id = legacy["objects"][0]["id"].clone();
    let mut next = legacy;
    for direction in 1..6 {
        let mut side = args(2., &format!("Face {direction}"), "Different note");
        side[3] = Value::Number(direction as f64);
        next = edit(&next, &side).unwrap();
    }
    assert_eq!(next["objects"].as_array().unwrap().len(), 6);
    assert_eq!(next["placements"][0]["face"], 0);
    assert_eq!(next["objects"][0]["id"], top_id);
    assert_eq!(next["objects"][0]["body"], "Keep this");
    let edited = edit(&next, &args(2., "Top edited", "Still top")).unwrap();
    assert_eq!(edited["objects"][0]["id"], top_id);
    assert_eq!(edited["objects"][1], next["objects"][1]);
    let mut reload = AppData::default();
    load(&mut reload, Some(&edited));
    assert_eq!(reload.document, Some(edited));
    let mut invalid = args(2., "Invalid", "");
    invalid[3] = Value::Number(6.);
    assert!(edit(&next, &invalid).is_err());
    next["placements"][1]["face"] = json!(-1);
    assert!(validate(&next).is_err());
}
