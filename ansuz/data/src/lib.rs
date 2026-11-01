//! Device-local authored records. Geometry only refers to stable object IDs.
//! This is a single-writer prototype store, not a synchronization protocol.
use exact_data::{envelope::unbase64, storage};
use exact_runner::{Answer, DataError, DataSource, Outcome, Store, Value};
use serde_json::{json, Value as Json};
use std::collections::BTreeSet;

mod cloud;
mod gestures;

const DIRECTORY: &str = "app:/data";
const FILE: &str = "surface-notes.json";
const PATH: &str = "app:/data/surface-notes.json";
const MAX_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;

fn empty() -> Json {
    json!({"version":1,"next_id":1,"objects":[],"relations":[],"placements":[]})
}

enum Pending {
    Directory,
    Listing,
    Read,
    Write(Json),
}

#[derive(Default)]
pub struct AppData {
    gestures: gestures::Gestures,
    cloud: cloud::Cloud,
    document: Option<Json>,
    pending: Option<Pending>,
    load_error: String,
}

// Missing orientation is the original +Y placement. Normalize the in-memory
// view; the existing file is untouched until a successful authored save.
fn with_faces(document: &Json) -> Json {
    let mut result = document.clone();
    if let Some(placements) = result["placements"].as_array_mut() {
        for placement in placements {
            if placement.get("face").is_none() {
                placement["face"] = json!(0);
            }
        }
    }
    result
}
fn face(placement: &Json) -> Option<u32> {
    match placement.get("face") {
        None => Some(0),
        Some(value) => value.as_u64().filter(|n| *n < 6).map(|n| n as u32),
    }
}

fn library(document: &Json, error: &str) -> Value {
    library_value(document, error, true)
}

fn library_value(document: &Json, error: &str, ready: bool) -> Value {
    // Field order is the Contract SurfaceNotes shape: json, error, ready.
    // The canvas mounts only after storage has answered, so its first live
    // binding contains the library rather than the bake-time placeholder.
    Value::record(vec![
        Value::str(&with_faces(document).to_string()),
        Value::str(error),
        Value::Bool(ready),
    ])
}

fn saved(ok: bool, error: &str) -> Value {
    // Field order is the Contract SaveResult shape: ok, error.
    Value::record(vec![Value::Bool(ok), Value::str(error)])
}

fn request(op: &str, args: Json) -> Answer {
    Answer::Later(storage::request(op, args))
}

fn coordinate(value: &Json) -> Option<i32> {
    value.as_i64().and_then(|v| i32::try_from(v).ok())
}

/// Reject incomplete/unknown records instead of replacing unreadable data.
fn validate(document: &Json) -> Result<(), String> {
    if document["version"] != 1 || document["next_id"].as_u64().filter(|n| *n > 0).is_none() {
        return Err("Unsupported or incomplete note library.".into());
    }
    let objects = document["objects"].as_array().ok_or("Missing objects.")?;
    let relations = document["relations"]
        .as_array()
        .ok_or("Missing relations.")?;
    let placements = document["placements"]
        .as_array()
        .ok_or("Missing placements.")?;
    let mut ids = BTreeSet::new();
    for object in objects {
        let id = object["id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("Invalid object ID.")?;
        if !ids.insert(id)
            || ["title", "body", "content_kind"]
                .iter()
                .any(|k| !object[k].is_string())
            || object["authored_weight"]
                .as_f64()
                .filter(|n| n.is_finite() && *n >= 0.)
                .is_none()
        {
            return Err("Invalid or duplicate object.".into());
        }
    }
    let mut cells = BTreeSet::new();
    for placement in placements {
        let id = placement["object_id"]
            .as_str()
            .ok_or("Invalid placement.")?;
        let cell = (
            coordinate(&placement["x"]).ok_or("Invalid cell x.")?,
            coordinate(&placement["y"]).ok_or("Invalid cell y.")?,
            coordinate(&placement["z"]).ok_or("Invalid cell z.")?,
            face(placement).ok_or("Invalid face orientation.")?,
        );
        if !ids.contains(id) || !cells.insert(cell) {
            return Err("Missing object or duplicate cell placement.".into());
        }
    }
    for relation in relations {
        if !relation.is_object()
            || !relation["from"].as_str().is_some_and(|id| ids.contains(id))
            || !relation["to"].as_str().is_some_and(|id| ids.contains(id))
            || !relation["source_anchor"].is_string()
        {
            return Err("Invalid relation.".into());
        }
        // Relation encoding belongs to game Data; preserve it verbatim.
        if relation.get("kind").is_none() {
            return Err("Missing relation kind.".into());
        }
    }
    Ok(())
}

fn edit(document: &Json, args: &[Value]) -> Result<Json, String> {
    let [x, y, z, orientation, title, body] = args else {
        return Err("Expected cell x, y, z, face, title and body.".into());
    };
    let axis = |v: &Value| -> Result<i32, String> {
        let n = v.as_number().ok_or("Invalid cell coordinate.")?;
        if !n.is_finite() || n.fract() != 0. || n < i32::MIN as f64 || n > i32::MAX as f64 {
            return Err("Invalid cell coordinate.".into());
        }
        Ok(n as i32)
    };
    let (x, y, z) = (axis(x)?, axis(y)?, axis(z)?);
    let orientation = axis(orientation)?;
    if !(0..6).contains(&orientation) {
        return Err("Invalid face orientation.".into());
    }
    let title = title.as_str().ok_or("Invalid title.")?.trim();
    let body = body.as_str().ok_or("Invalid body.")?;
    if title.is_empty() && body.trim().is_empty() {
        return Err("Add a title or a note.".into());
    }
    let title = if title.is_empty() {
        "Untitled note"
    } else {
        title
    };
    validate(document)?;
    let mut next = with_faces(document);
    let existing = next["placements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["x"] == x && p["y"] == y && p["z"] == z && p["face"] == orientation)
        .map(|p| p["object_id"].as_str().unwrap().to_owned());
    if let Some(id) = existing {
        let object = next["objects"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|o| o["id"] == id)
            .unwrap();
        object["title"] = json!(title);
        object["body"] = json!(body);
    } else {
        // Monotonic IDs are local to this library; coordinates never enter them.
        // A future multi-device/import protocol must introduce a namespace.
        let mut serial = next["next_id"].as_u64().unwrap();
        let id = loop {
            let id = format!("note-{serial}");
            serial = serial.checked_add(1).ok_or("Object ID space exhausted.")?;
            if !next["objects"]
                .as_array()
                .unwrap()
                .iter()
                .any(|o| o["id"] == id)
            {
                break id;
            }
        };
        next["next_id"] = json!(serial);
        next["objects"].as_array_mut().unwrap().push(json!({
            "id":id,"title":title,"body":body,"content_kind":"note","authored_weight":1.0
        }));
        next["placements"]
            .as_array_mut()
            .unwrap()
            .push(json!({"object_id":id,"x":x,"y":y,"z":z,"face":orientation}));
    }
    if next.to_string().len() > MAX_DOCUMENT_BYTES {
        return Err("This prototype's local library is full (4 MiB).".into());
    }
    Ok(next)
}

impl DataSource for AppData {
    fn app_id(&self) -> &str {
        "studio.zojer.ansuz"
    }

    fn grants(&self) -> &str {
        cloud::GRANTS
    }

    fn query(&mut self, source: &str, args: &[Value]) -> Result<Value, DataError> {
        if cloud::Cloud::owns(source) {
            return Ok(self.cloud.query(source));
        }
        if gestures::Gestures::owns(source) {
            if source == "gestureTakes" && args.first().and_then(Value::as_str).unwrap_or("") != self.cloud.owner() {
                return Ok(gestures::Gestures::hidden());
            }
            return Ok(self.gestures.query(source, &self.cloud.owner()));
        }
        match source {
            "loadSurfaceNotes" => Ok(library_value(&empty(), "", false)),
            "saveSurfaceNote" => Ok(saved(false, "")),
            other => Err(DataError::UnknownSource(other.into())),
        }
    }

    fn answer(
        &mut self,
        store: &mut Store,
        source: &str,
        args: &[Value],
    ) -> Result<Answer, DataError> {
        if cloud::Cloud::owns(source) {
            return Ok(self.cloud.answer(
                store,
                source,
                args,
                if source == "syncRecordings" {
                    self.gestures.takes(&self.cloud.owner())
                } else {
                    None
                },
            ));
        }
        if gestures::Gestures::owns(source) {
            // Never publish one account's rows under a different resource key,
            // including the empty key while authorization is still restoring.
            if source == "gestureTakes" && args.first().and_then(Value::as_str).unwrap_or("") != self.cloud.owner() {
                return Ok(Answer::Now(gestures::Gestures::hidden()));
            }
            return Ok(self.gestures.answer(source, args, &self.cloud.owner()));
        }
        match source {
            "loadSurfaceNotes" => {
                if let Some(document) = &self.document {
                    return Ok(Answer::Now(library(document, "")));
                }
                if self.pending.is_some() {
                    return Ok(Answer::Now(library(&empty(), "Notes are still loading.")));
                }
                self.pending = Some(Pending::Directory);
                Ok(request("fs.mkdir", json!({"path":DIRECTORY})))
            }
            "saveSurfaceNote" => {
                if self.pending.is_some() {
                    return Ok(Answer::Now(saved(
                        false,
                        "Wait for the current storage operation.",
                    )));
                }
                let Some(document) = &self.document else {
                    let error = if self.load_error.is_empty() {
                        "Notes have not loaded yet."
                    } else {
                        &self.load_error
                    };
                    return Ok(Answer::Now(saved(false, error)));
                };
                let next = match edit(document, args) {
                    Ok(next) => next,
                    Err(error) => return Ok(Answer::Now(saved(false, &error))),
                };
                let text = next.to_string();
                self.pending = Some(Pending::Write(next));
                Ok(request(
                    "fs.atomicWriteFile",
                    json!({"path":PATH,"text":text}),
                ))
            }
            other => Err(DataError::UnknownSource(other.into())),
        }
    }

    fn parse(
        &mut self,
        store: &mut Store,
        source: &str,
        args: &[Value],
        outcome: Outcome,
    ) -> Result<Answer, DataError> {
        if cloud::Cloud::owns(source) {
            return Ok(self
                .cloud
                .parse(store, &cloud::request_key(source, args), source, outcome));
        }
        if gestures::Gestures::owns(source) {
            let answer = self.gestures.parse(source, outcome, &self.cloud.owner());
            if source == "gestureTakes" && matches!(answer, Answer::Now(_)) && args.first().and_then(Value::as_str).unwrap_or("") != self.cloud.owner() {
                return Ok(Answer::Now(gestures::Gestures::hidden()));
            }
            return Ok(answer);
        }
        let Some(pending) = self.pending.take() else {
            return Err(DataError::Unavailable(
                "No pending note storage operation.".into(),
            ));
        };
        let result = storage::response(outcome);
        if let Pending::Write(next) = pending {
            return Ok(Answer::Now(match result {
                Ok(_) => {
                    self.document = Some(next);
                    saved(true, "")
                }
                Err(error) => saved(false, &error),
            }));
        }
        if source != "loadSurfaceNotes" {
            return Err(DataError::UnknownSource(source.into()));
        }
        let loaded: Result<Option<Json>, String> = (|| {
            let reply = result?;
            match pending {
                Pending::Directory => {
                    self.pending = Some(Pending::Listing);
                    Ok(None)
                }
                Pending::Listing => {
                    let names = reply.as_array().ok_or("Invalid directory response.")?;
                    if names.iter().any(|name| name == FILE) {
                        self.pending = Some(Pending::Read);
                        Ok(None)
                    } else {
                        Ok(Some(empty()))
                    }
                }
                Pending::Read => {
                    let text = reply["base64"]
                        .as_str()
                        .and_then(unbase64)
                        .ok_or("Invalid file response.")?;
                    if text.len() > MAX_DOCUMENT_BYTES {
                        return Err("Note library exceeds the prototype's 4 MiB limit.".into());
                    }
                    let document: Json = serde_json::from_slice(&text)
                        .map_err(|error| format!("Cannot read note library: {error}"))?;
                    validate(&document)?;
                    Ok(Some(document))
                }
                Pending::Write(_) => unreachable!(),
            }
        })();
        match loaded {
            Ok(Some(document)) => {
                self.load_error.clear();
                let value = library(&document, "");
                self.document = Some(document);
                Ok(Answer::Now(value))
            }
            Ok(None) => Ok(match self.pending {
                Some(Pending::Listing) => request("fs.readdir", json!({"path":DIRECTORY})),
                _ => request("fs.readFile", json!({"path":PATH})),
            }),
            Err(error) => {
                self.load_error = error;
                Ok(Answer::Now(library(&empty(), &self.load_error)))
            }
        }
    }
}

#[cfg(test)]
mod tests;
