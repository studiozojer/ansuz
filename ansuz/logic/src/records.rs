//! Authored records are independent of voxel cells, mesh faces and hologram shapes.
//! These are sample records, not a storage schema or a P-tree implementation.
use exact_game::{Data, Resource};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Fixture identities stay fixed when a title, placement or representation changes.
pub const ROOT_IDS: [&str; 5] = [
    "root-returning",
    "root-ember",
    "root-stillwater",
    "root-unwritten",
    "root-questions",
];
pub const WEIGHT_RULE: &str = "unique-containment-sum-v1";

#[derive(Clone, Debug, Default, PartialEq, Data)]
pub struct Object {
    pub id: String,
    pub title: String,
    /// Opaque authored content for this experiment; a kind does not select a shape.
    pub body: String,
    pub content_kind: String,
    pub authored_weight: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Data)]
pub enum RelationKind {
    /// `from` contains `to`; independent of where either object is drawn.
    #[default]
    Contains,
    /// `from` was derived from `to` (child -> source).
    DerivedFrom,
    /// An authored association; not containment or provenance.
    AssociatedWith,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Data)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub kind: RelationKind,
    /// Optional stable block/fragment anchor in the source; blank means whole object.
    pub source_anchor: String,
}

#[derive(Clone, Debug, Default, PartialEq, Resource)]
pub struct Dataset {
    pub objects: Vec<Object>,
    pub relations: Vec<Relation>,
}

impl Dataset {
    pub fn demo() -> Self {
        let mut data = Self::default();
        let names = [
            "The Returning",
            "Ember Grove",
            "Stillwater",
            "The Unwritten",
            "Open Questions",
        ];
        for (root_index, id) in ROOT_IDS.iter().enumerate() {
            data.objects.push(Object {
                id: (*id).into(),
                title: names[root_index].into(),
                body: "A sample topic. Its identity survives changes of shape and place.".into(),
                content_kind: "topic".into(),
                authored_weight: 1. + root_index as f32 * 0.25,
            });
            for child_index in 0..9 {
                let child_id = format!("{id}-sample-{}", child_index + 1);
                let (kind, title) = match child_index % 3 {
                    0 => ("iching", "A sample cast"),
                    1 => ("rune", "A sample rune"),
                    _ => ("note", "An open note"),
                };
                data.objects.push(Object {
                    id: child_id.clone(),
                    title: format!("{title} {}", child_index + 1),
                    body: "Placeholder content; this is not a recorded divination.".into(),
                    content_kind: kind.into(),
                    // Authored fixture values demonstrate unequal weight. They do not
                    // assert that one divination or content type is inherently heavier.
                    authored_weight: 0.5 + ((root_index + child_index) % 4) as f32 * 0.5,
                });
                data.relations.push(Relation {
                    from: (*id).into(),
                    to: child_id.clone(),
                    kind: RelationKind::Contains,
                    source_anchor: String::new(),
                });
                data.relations.push(Relation {
                    from: child_id,
                    to: (*id).into(),
                    kind: RelationKind::DerivedFrom,
                    source_anchor: "sample-response".into(),
                });
            }
        }
        data.relations.push(Relation {
            from: ROOT_IDS[0].into(),
            to: ROOT_IDS[2].into(),
            kind: RelationKind::AssociatedWith,
            source_anchor: String::new(),
        });
        data
    }

    pub fn root_id(index: usize) -> Option<&'static str> {
        ROOT_IDS.get(index).copied()
    }

    pub fn object(&self, id: &str) -> Option<&Object> {
        self.objects.iter().find(|object| object.id == id)
    }

    pub fn object_mut(&mut self, id: &str) -> Option<&mut Object> {
        self.objects.iter_mut().find(|object| object.id == id)
    }

    /// Prototype rule: count each reachable contained object once, including root.
    /// Shared descendants, duplicate links and cycles do not multiply mass. Missing
    /// objects and invalid/negative weights contribute nothing. Other link types do
    /// not contribute. The rule is provisional and explicitly versioned above.
    pub fn aggregate_weight(&self, id: &str) -> f32 {
        let mut seen = BTreeSet::new();
        let mut pending = vec![id];
        let mut total = 0.;
        while let Some(next) = pending.pop() {
            if !seen.insert(next) {
                continue;
            }
            let Some(object) = self.object(next) else {
                continue;
            };
            if object.authored_weight.is_finite() && object.authored_weight >= 0. {
                total += object.authored_weight;
            }
            pending.extend(
                self.relations
                    .iter()
                    .filter(|edge| edge.kind == RelationKind::Contains && edge.from == next)
                    .map(|edge| edge.to.as_str()),
            );
        }
        total
    }

    /// A pure reference projection: source objects sit above their derivations.
    /// Every object appears once. Multi-parent edges remain explicit; the layout
    /// never rewrites them to manufacture a tree. Cyclic components start a new
    /// row-zero traversal. This is an inspectable fallback, not the final pyramid UX.
    pub fn provenance_projection(&self) -> Projection {
        let known: BTreeSet<&str> = self
            .objects
            .iter()
            .map(|object| object.id.as_str())
            .collect();
        let mut descendants: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        let mut has_parent = BTreeSet::new();
        for edge in &self.relations {
            if edge.kind == RelationKind::DerivedFrom
                && known.contains(edge.from.as_str())
                && known.contains(edge.to.as_str())
            {
                descendants.entry(&edge.to).or_default().insert(&edge.from);
                has_parent.insert(edge.from.as_str());
            }
        }
        let mut depths = BTreeMap::new();
        let mut pending: VecDeque<(&str, u32)> = known
            .iter()
            .filter(|id| !has_parent.contains(**id))
            .map(|id| (*id, 0))
            .collect();
        loop {
            while let Some((id, depth)) = pending.pop_front() {
                if depths.contains_key(id) {
                    continue;
                }
                depths.insert(id, depth);
                if let Some(children) = descendants.get(id) {
                    pending.extend(children.iter().map(|child| (*child, depth + 1)));
                }
            }
            let Some(id) = known.iter().find(|id| !depths.contains_key(**id)) else {
                break;
            };
            pending.push_back((*id, 0));
        }
        let mut rows: BTreeMap<u32, Vec<&str>> = BTreeMap::new();
        for (id, depth) in depths {
            rows.entry(depth).or_default().push(id);
        }
        let mut placements = Vec::new();
        for (depth, ids) in rows {
            let center = (ids.len() as f32 - 1.) * 0.5;
            for (column, id) in ids.into_iter().enumerate() {
                placements.push(Placement {
                    object_id: id.into(),
                    position: [(column as f32 - center) * 3., -(depth as f32) * 4., 0.],
                    depth,
                });
            }
        }
        Projection {
            rule: "provenance-breadth-first-v1".into(),
            placements,
        }
    }
}

/// A renderer/view owns placement. The object itself contains no geometry.
#[derive(Clone, Debug, Default, PartialEq, Data)]
pub struct Placement {
    pub object_id: String,
    pub position: [f32; 3],
    pub depth: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Data)]
pub struct Projection {
    pub rule: String,
    pub placements: Vec<Placement>,
}

impl Projection {
    /// Accept a surface's placements without copying its records. Unknown IDs
    /// are rejected rather than silently creating objects from generated geometry.
    pub fn surface(data: &Dataset, placements: Vec<Placement>) -> Result<Self, String> {
        for placement in &placements {
            if data.object(&placement.object_id).is_none() {
                return Err(format!("unknown object {}", placement.object_id));
            }
            if placement.position.iter().any(|value| !value.is_finite()) {
                return Err(format!("non-finite placement for {}", placement.object_id));
            }
        }
        Ok(Self {
            rule: "authored-surface-placements-v1".into(),
            placements,
        })
    }

    /// Resolve against current records: edits through another view are visible.
    pub fn object_at<'a>(&self, data: &'a Dataset, index: usize) -> Option<&'a Object> {
        self.placements
            .get(index)
            .and_then(|placement| data.object(&placement.object_id))
    }
}
