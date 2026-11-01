//! Gesture recordings use their own file and never load the reading library.
use super::*;
const PATH: &str = "app:/data/gesture-takes.json";
const FILE: &str = "gesture-takes.json";
const LIMIT: usize = 8 * 1024 * 1024;
#[derive(Default)]
pub struct Gestures {
    document: Option<Json>,
    pending: Option<PendingTrace>,
    error: String,
}
enum PendingTrace {
    Directory,
    Listing,
    Read,
    Write(Json),
}
fn empty() -> Json {
    json!({"version":1,"next":1,"takes":[]})
}
fn result(ok: bool, error: &str) -> Value {
    Value::record(vec![Value::Bool(ok), Value::str(error)])
}
fn view(doc: &Json, error: &str, ready: bool) -> Value {
    let items = doc["takes"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .map(|t| {
            Value::record(vec![
                Value::str(t["id"].as_str().unwrap()),
                Value::str(t["label"].as_str().unwrap()),
                Value::str(t["expected"].as_str().unwrap_or("")),
                Value::str(&t["trace"].to_string()),
            ])
        })
        .collect();
    Value::record(vec![
        Value::list(items),
        Value::str(error),
        Value::Bool(ready),
    ])
}
fn valid_trace(j: &Json) -> bool {
    j["schema"] == "ansuz.gesture-take"
        && matches!(j["version"].as_u64(), Some(1 | 2 | 3))
        && j["hz"] == 120
        && j["frames"]
            .as_array()
            .is_some_and(|a| !a.is_empty() && a.len() <= 2400)
}
fn validate(j: &Json) -> bool {
    j["version"] == 1
        && j["next"].as_u64().is_some_and(|n| n > 0)
        && j["takes"].as_array().is_some_and(|a| {
            a.len() <= 12
                && a.iter().all(|t| {
                    t["id"].is_string()
                        && t["label"].is_string()
                        && t["expected"].is_string()
                        && valid_trace(&t["trace"])
                })
        })
}
impl Gestures {
    pub fn takes(&self) -> Option<Vec<Json>> {
        self.document
            .as_ref()
            .and_then(|d| d["takes"].as_array())
            .cloned()
    }
    pub fn owns(source: &str) -> bool {
        matches!(
            source,
            "gestureTakes" | "saveGestureTake" | "deleteGestureTake"
        )
    }
    pub fn query(&self, source: &str) -> Value {
        if source == "gestureTakes" {
            view(
                self.document.as_ref().unwrap_or(&empty()),
                &self.error,
                self.document.is_some() || !self.error.is_empty(),
            )
        } else {
            result(false, "")
        }
    }
    pub fn answer(&mut self, source: &str, args: &[Value]) -> Answer {
        if self.pending.is_some() {
            return Answer::Now(if source == "gestureTakes" {
                self.query(source)
            } else {
                result(false, "A recording is still being saved. Try again.")
            });
        }
        if source == "gestureTakes" {
            if self.document.is_some() || !self.error.is_empty() {
                return Answer::Now(self.query(source));
            }
            self.pending = Some(PendingTrace::Directory);
            return request("fs.mkdir", json!({"path":"app:/data"}));
        }
        let Some(document) = &self.document else {
            return Answer::Now(result(
                false,
                "Recordings have not loaded, or their file could not be read.",
            ));
        };
        let mut next = document.clone();
        if source == "saveGestureTake" {
            if next["takes"].as_array().unwrap().len() >= 12 {
                return Answer::Now(result(
                    false,
                    "Twelve takes saved. Export and delete one before saving another.",
                ));
            }
            let text = args.first().and_then(|v| v.as_str()).unwrap_or("");
            if text.len() > 2 * 1024 * 1024 {
                return Answer::Now(result(false, "Take exceeds 2 MiB."));
            }
            let Ok(mut trace) = serde_json::from_str::<Json>(text) else {
                return Answer::Now(result(false, "Invalid take JSON."));
            };
            if !valid_trace(&trace) {
                return Answer::Now(result(false, "Unsupported or empty take."));
            }
            let note: String = args
                .get(1)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .chars()
                .take(1000)
                .collect();
            let id = next["next"].as_u64().unwrap();
            trace["expected"] = json!(note);
            trace["take_id"] = json!(format!("take-{id}"));
            if let Some(Value::Number(at)) = args.get(2) {
                if at.is_finite() {
                    trace["captured_at_ms"] = json!(at);
                }
            }
            let seconds = trace["frames"].as_array().unwrap().len() as f32 / 120.;
            next["takes"].as_array_mut().unwrap().push(json!({"id":format!("take-{id}"),"label":format!("Take {id} · {seconds:.1}s"),"expected":note,"trace":trace}));
            next["next"] = json!(id + 1);
        } else {
            let id = args.first().and_then(|v| v.as_str()).unwrap_or("");
            next["takes"]
                .as_array_mut()
                .unwrap()
                .retain(|t| t["id"] != id);
        }
        let text = next.to_string();
        if text.len() > LIMIT {
            return Answer::Now(result(
                false,
                "Recording storage is full. Export and delete a take first.",
            ));
        }
        self.pending = Some(PendingTrace::Write(next));
        request("fs.atomicWriteFile", json!({"path":PATH,"text":text}))
    }
    pub fn parse(&mut self, source: &str, outcome: Outcome) -> Answer {
        let Some(pending) = self.pending.take() else {
            return Answer::Now(result(false, "No recording storage operation pending."));
        };
        let reply = storage::response(outcome);
        if let PendingTrace::Write(next) = pending {
            return Answer::Now(match reply {
                Ok(_) => {
                    self.document = Some(next);
                    result(true, "")
                }
                Err(e) => result(false, &e),
            });
        }
        let loaded = (|| -> Result<Option<Json>, String> {
            let r = reply?;
            match pending {
                PendingTrace::Directory => {
                    self.pending = Some(PendingTrace::Listing);
                    Ok(None)
                }
                PendingTrace::Listing => {
                    let a = r.as_array().ok_or("Invalid recording directory.")?;
                    if a.iter().any(|v| v == FILE) {
                        self.pending = Some(PendingTrace::Read);
                        Ok(None)
                    } else {
                        Ok(Some(empty()))
                    }
                }
                PendingTrace::Read => {
                    let bytes = r["base64"]
                        .as_str()
                        .and_then(unbase64)
                        .ok_or("Cannot read recording file.")?;
                    if bytes.len() > LIMIT {
                        return Err("Recording file exceeds 8 MiB.".into());
                    }
                    let doc: Json = serde_json::from_slice(&bytes)
                        .map_err(|_| "Unreadable recording file; it has been preserved.")?;
                    if !validate(&doc) {
                        return Err("Unsupported recording file; it has been preserved.".into());
                    }
                    Ok(Some(doc))
                }
                PendingTrace::Write(_) => unreachable!(),
            }
        })();
        match loaded {
            Ok(Some(doc)) => {
                self.document = Some(doc);
                Answer::Now(self.query(source))
            }
            Ok(None) => match self.pending {
                Some(PendingTrace::Listing) => request("fs.readdir", json!({"path":"app:/data"})),
                _ => request("fs.readFile", json!({"path":PATH})),
            },
            Err(e) => {
                self.error = e;
                Answer::Now(self.query(source))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn response(g: &mut Gestures, source: &str, j: Json) -> Answer {
        g.parse(source, Outcome::Storage(j.to_string().into_bytes()))
    }
    fn fresh() -> Gestures {
        let mut g = Gestures::default();
        assert!(matches!(g.answer("gestureTakes", &[]), Answer::Later(_)));
        response(&mut g, "gestureTakes", Json::Null);
        response(&mut g, "gestureTakes", json!([]));
        g
    }
    fn args() -> Vec<Value> {
        vec![
            Value::str(
                &json!({"schema":"ansuz.gesture-take","version":1,"hz":120,"frames":[[0]]})
                    .to_string(),
            ),
            Value::str("Catch on contact, then hold still"),
            Value::Number(1234.),
        ]
    }
    #[test]
    fn gesture_take_commits_only_after_write_and_preserves_expected_behavior() {
        let mut g = fresh();
        assert!(matches!(
            g.answer("saveGestureTake", &args()),
            Answer::Later(_)
        ));
        assert!(g.document.as_ref().unwrap()["takes"]
            .as_array()
            .unwrap()
            .is_empty());
        response(&mut g, "saveGestureTake", Json::Null);
        let doc = g.document.as_ref().unwrap();
        assert_eq!(
            doc["takes"][0]["trace"]["expected"],
            "Catch on contact, then hold still"
        );
        assert_eq!(doc["takes"][0]["trace"]["captured_at_ms"], 1234.);
        let serialized = doc.to_string();
        let mut reloaded = Gestures::default();
        reloaded.answer("gestureTakes", &[]);
        response(&mut reloaded, "gestureTakes", Json::Null);
        response(&mut reloaded, "gestureTakes", json!([FILE]));
        response(
            &mut reloaded,
            "gestureTakes",
            json!({"base64":exact_data::envelope::base64(serialized.as_bytes())}),
        );
        assert_eq!(reloaded.document, g.document);
    }
    #[test]
    fn unreadable_gesture_file_cannot_be_overwritten() {
        let mut g = Gestures::default();
        g.answer("gestureTakes", &[]);
        response(&mut g, "gestureTakes", Json::Null);
        response(&mut g, "gestureTakes", json!([FILE]));
        response(
            &mut g,
            "gestureTakes",
            json!({"base64":exact_data::envelope::base64(b"not json")}),
        );
        assert!(g.document.is_none());
        assert!(!g.error.is_empty());
        assert!(matches!(
            g.answer("saveGestureTake", &args()),
            Answer::Now(_)
        ));
        assert!(g.pending.is_none());
    }
    #[test]
    fn failed_recording_write_keeps_prior_takes() {
        let mut g = fresh();
        g.answer("saveGestureTake", &args());
        response(&mut g, "saveGestureTake", Json::Null);
        let before = g.document.clone();
        g.answer("saveGestureTake", &args());
        response(&mut g, "saveGestureTake", json!({"error":"disk full"}));
        assert_eq!(g.document, before);
    }
}
