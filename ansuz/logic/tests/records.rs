use ansuz_logic::records::{
    Dataset, Object, Placement, Projection, Relation, RelationKind, ROOT_IDS,
};
use std::collections::BTreeSet;

fn object(id: &str, weight: f32) -> Object {
    Object {
        id: id.into(),
        title: id.into(),
        body: "original".into(),
        content_kind: "note".into(),
        authored_weight: weight,
    }
}

fn edge(from: &str, to: &str, kind: RelationKind) -> Relation {
    Relation {
        from: from.into(),
        to: to.into(),
        kind,
        source_anchor: "a-line-within-the-source".into(),
    }
}

#[test]
fn weight_counts_unique_containment_and_not_provenance_or_association() {
    use RelationKind::*;
    let data = Dataset {
        objects: vec![
            object("root", 1.),
            object("a", 2.),
            object("b", 3.),
            object("shared", 0.5),
            object("related", 100.),
        ],
        relations: vec![
            edge("root", "a", Contains),
            edge("root", "b", Contains),
            edge("a", "shared", Contains),
            edge("b", "shared", Contains),
            edge("root", "a", Contains),
            edge("shared", "root", Contains),
            edge("root", "related", DerivedFrom),
            edge("root", "related", AssociatedWith),
            edge("root", "missing", Contains),
        ],
    };
    assert_eq!(data.aggregate_weight("root"), 6.5);
    assert_eq!(data.aggregate_weight("missing"), 0.);
    assert_eq!(data.aggregate_weight("shared"), 6.5, "cycles terminate");
}

#[test]
fn invalid_authored_weights_do_not_poison_the_motion_input() {
    let data = Dataset {
        objects: vec![
            object("root", 1.),
            object("negative", -3.),
            object("nan", f32::NAN),
        ],
        relations: vec![
            edge("root", "negative", RelationKind::Contains),
            edge("root", "nan", RelationKind::Contains),
        ],
    };
    assert_eq!(data.aggregate_weight("root"), 1.);
}

#[test]
fn surface_and_provenance_views_resolve_the_same_edited_record_without_reparenting() {
    let mut data = Dataset::demo();
    let before = data.clone();
    // Surface geometry is an input owned by the view, not a field on each object.
    let surface = Projection::surface(
        &data,
        data.objects
            .iter()
            .enumerate()
            .map(|(index, object)| Placement {
                object_id: object.id.clone(),
                position: [(index % 10) as f32, 20., (index / 10) as f32],
                depth: 0,
            })
            .collect(),
    )
    .unwrap();
    let hierarchy = data.provenance_projection();
    assert_eq!(data, before, "layout must not modify source records");
    let ids = |projection: &Projection| -> BTreeSet<String> {
        projection
            .placements
            .iter()
            .map(|p| p.object_id.clone())
            .collect()
    };
    assert_eq!(ids(&surface), ids(&hierarchy));
    let root = ROOT_IDS[0];
    let surface_index = surface
        .placements
        .iter()
        .position(|p| p.object_id == root)
        .unwrap();
    let hierarchy_index = hierarchy
        .placements
        .iter()
        .position(|p| p.object_id == root)
        .unwrap();
    assert_ne!(
        surface.placements[surface_index].position,
        hierarchy.placements[hierarchy_index].position
    );
    data.object_mut(root).unwrap().body = "edited through a view".into();
    data.object_mut(root).unwrap().title = "A new title, same identity".into();
    assert_eq!(
        surface.object_at(&data, surface_index).unwrap().body,
        "edited through a view"
    );
    assert_eq!(
        surface.object_at(&data, surface_index),
        hierarchy.object_at(&data, hierarchy_index)
    );
    assert_eq!(data.relations, before.relations);
    assert_eq!(data.object(root).unwrap().id, root);
}

#[test]
fn cyclic_and_multi_parent_provenance_is_not_rewritten_into_a_tree() {
    use RelationKind::*;
    let data = Dataset {
        objects: vec![
            object("a", 1.),
            object("b", 1.),
            object("c", 1.),
            object("other", 1.),
        ],
        relations: vec![
            edge("a", "b", DerivedFrom),
            edge("b", "a", DerivedFrom),
            edge("c", "a", DerivedFrom),
            edge("c", "other", DerivedFrom),
        ],
    };
    let before = data.clone();
    let view = data.provenance_projection();
    let ids: BTreeSet<_> = view
        .placements
        .iter()
        .map(|p| p.object_id.as_str())
        .collect();
    assert_eq!(ids, BTreeSet::from(["a", "b", "c", "other"]));
    assert_eq!(view.placements.len(), 4);
    assert_eq!(data, before);
    assert_eq!(data.relations.iter().filter(|r| r.from == "c").count(), 2);
}

#[test]
fn generated_geometry_cannot_create_records_by_accident() {
    let data = Dataset::demo();
    assert!(Projection::surface(
        &data,
        vec![Placement {
            object_id: "voxel-120-4-3".into(),
            position: [1., 2., 3.],
            depth: 0,
        }]
    )
    .is_err());
    assert!(Projection::surface(
        &data,
        vec![Placement {
            object_id: ROOT_IDS[0].into(),
            position: [f32::INFINITY, 2., 3.],
            depth: 0,
        }]
    )
    .is_err());
}

#[test]
fn sample_dataset_roundtrips_with_typed_edges_and_block_anchors() {
    let data = Dataset::demo();
    let encoded = exact_game::json::to_string(&data).unwrap();
    let decoded: Dataset = exact_game::json::from_str(&encoded).unwrap();
    assert_eq!(decoded, data);
    assert!(decoded
        .relations
        .iter()
        .any(|r| r.kind == RelationKind::DerivedFrom && r.source_anchor == "sample-response"));
    assert_ne!(
        decoded.aggregate_weight(ROOT_IDS[0]),
        decoded.aggregate_weight(ROOT_IDS[4])
    );
}
