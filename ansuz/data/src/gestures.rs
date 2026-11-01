//! Gesture recordings use their own file and never load the reading library.
use super::*;
#[path = "../../shared/gesture_trace.rs"]
mod trace_contract;
const PATH: &str = "app:/data/gesture-takes-v2.json";
const FILE: &str = "gesture-takes-v2.json";
const LEGACY: &str = "gesture-takes.json";
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
    Read(bool),
    Write(Json),
}
fn empty() -> Json {
    json!({"version":2,"next":1,"takes":[]})
}
fn result(ok: bool, error: &str) -> Value {
    Value::record(vec![Value::Bool(ok), Value::str(error)])
}
fn take_value(t: &Json) -> Value {
    Value::record(vec![
        Value::str(t["id"].as_str().unwrap()),
        Value::str(t["label"].as_str().unwrap()),
        Value::str(t["expected"].as_str().unwrap_or("")),
        Value::str(&t["trace"].to_string()),
    ])
}
fn owner(t: &Json) -> &str {
    t["owner"].as_str().unwrap_or("")
}
fn view(doc: &Json, identity: &str, error: &str, ready: bool) -> Value {
    let all = doc["takes"].as_array().unwrap();
    Value::record(vec![
        Value::list(
            all.iter()
                .rev()
                .filter(|t| owner(t) == identity)
                .map(take_value)
                .collect(),
        ),
        Value::str(error),
        Value::Bool(ready),
        Value::list(
            all.iter()
                .rev()
                .filter(|t| owner(t).is_empty())
                .map(take_value)
                .collect(),
        ),
    ])
}
fn valid_trace(j: &Json) -> bool {
    trace_contract::validate(j).is_ok()
}
fn validate(j: &Json) -> bool {
    matches!(j["version"].as_u64(), Some(1 | 2))
        && j["next"].as_u64().is_some_and(|n| n > 0)
        && j["takes"].as_array().is_some_and(|a| {
            a.len() <= 256
                && a.iter().all(|t| {
                    (j["version"] == 1
                        || t["owner"].is_null()
                        || t["owner"].as_str().is_some_and(cloud::valid_did))
                        && t["id"].is_string()
                        && t["label"].is_string()
                        && t["expected"].is_string()
                        && valid_trace(&t["trace"])
                })
        })
}
impl Gestures {
    pub fn takes(&self, identity: &str) -> Option<Vec<Json>> {
        self.document
            .as_ref()
            .and_then(|d| d["takes"].as_array())
            .map(|a| a.iter().filter(|t| owner(t) == identity).cloned().collect())
    }
    pub fn owns(source: &str) -> bool {
        matches!(
            source,
            "gestureTakes" | "saveGestureTake" | "deleteGestureTake" | "claimGestureTake"
        )
    }
    pub fn hidden() -> Value {
        view(&empty(), "", "", false)
    }
    pub fn query(&self, source: &str, identity: &str) -> Value {
        if source == "gestureTakes" {
            view(
                self.document.as_ref().unwrap_or(&empty()),
                identity,
                &self.error,
                self.document.is_some() || !self.error.is_empty(),
            )
        } else {
            result(false, "")
        }
    }
    pub fn answer(&mut self, source: &str, args: &[Value], identity: &str) -> Answer {
        // There is one gestureTakes resource. A new ask replaces its former
        // continuation (for example when account restoration changes its args).
        // Restart a superseded read; retaining it would strand the new view.
        // Writes keep their original owner/document until acknowledgement.
        if source == "gestureTakes" && !matches!(self.pending, Some(PendingTrace::Write(_))) {
            self.pending = None;
        }
        if self.pending.is_some() {
            return Answer::Now(if source == "gestureTakes" {
                self.query(source, identity)
            } else {
                result(false, "A recording is still being saved. Try again.")
            });
        }
        if source == "gestureTakes" {
            if self.document.is_some() || !self.error.is_empty() {
                return Answer::Now(self.query(source, identity));
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
        let expected = args
            .get(if source == "saveGestureTake" { 3 } else { 1 })
            .and_then(Value::as_str)
            .unwrap_or("");
        if expected != identity {
            return Answer::Now(result(
                false,
                "The account changed. This take was not reassigned.",
            ));
        }
        let save_key = cloud::digest(
            &json!([
                args.first().and_then(Value::as_str).unwrap_or(""),
                args.get(1).and_then(Value::as_str).unwrap_or(""),
                args.get(2).and_then(|v| if let Value::Number(n) = v {
                    Some(*n)
                } else {
                    None
                })
            ])
            .to_string(),
        );
        if source == "saveGestureTake"
            && document["takes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| owner(t) == identity && t["save_key"] == save_key)
        {
            return Answer::Now(result(true, ""));
        }
        let mut next = document.clone();
        if source == "saveGestureTake" {
            if next["takes"].as_array().unwrap().len() >= 256 {
                return Answer::Now(result(false, "Device recording storage is full."));
            }
            if next["takes"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|t| owner(t) == identity)
                .count()
                >= 12
            {
                return Answer::Now(result(
                    false,
                    "Twelve takes saved. Export and delete one before saving another.",
                ));
            }
            let text = args.first().and_then(|v| v.as_str()).unwrap_or("");
            if text.len() > trace_contract::MAX_BYTES {
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
            if trace.to_string().len() > trace_contract::MAX_BYTES {
                return Answer::Now(result(false, "Take with its note exceeds 2 MiB."));
            }
            let seconds = trace["frames"].as_array().unwrap().len() as f32 / 120.;
            next["takes"].as_array_mut().unwrap().push(json!({"id":format!("take-{id}"),"label":format!("Take {id} · {seconds:.1}s"),"expected":note,"trace":trace,"save_key":save_key,"owner":if identity.is_empty(){Json::Null}else{json!(identity)}}));
            next["next"] = json!(id + 1);
        } else if source == "claimGestureTake" {
            if !cloud::valid_did(identity) {
                return Answer::Now(result(false, "Sign in before adding a device-only take."));
            }
            if next["takes"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|t| owner(t) == identity)
                .count()
                >= 12
            {
                return Answer::Now(result(false, "Twelve takes saved for this account. Export and delete one before adding another."));
            }
            let id = args.first().and_then(Value::as_str).unwrap_or("");
            let Some(take) = next["takes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|t| t["id"] == id && owner(t).is_empty())
            else {
                return Answer::Now(result(
                    false,
                    "This take is unavailable or already belongs to an account.",
                ));
            };
            // Explicit local attribution. The source trace bytes stay unchanged.
            take["owner"] = json!(identity);
        } else {
            let id = args.first().and_then(|v| v.as_str()).unwrap_or("");
            next["takes"]
                .as_array_mut()
                .unwrap()
                .retain(|t| t["id"] != id || owner(t) != identity);
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
    pub fn parse(&mut self, source: &str, outcome: Outcome, identity: &str) -> Answer {
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
                        self.pending = Some(PendingTrace::Read(false));
                        Ok(None)
                    } else if a.iter().any(|v| v == LEGACY) {
                        self.pending = Some(PendingTrace::Read(true));
                        Ok(None)
                    } else {
                        Ok(Some(empty()))
                    }
                }
                PendingTrace::Read(_) => {
                    let bytes = r["base64"]
                        .as_str()
                        .and_then(unbase64)
                        .ok_or("Cannot read recording file.")?;
                    if bytes.len() > LIMIT {
                        return Err("Recording file exceeds 8 MiB.".into());
                    }
                    let mut doc: Json = serde_json::from_slice(&bytes)
                        .map_err(|_| "Unreadable recording file; it has been preserved.")?;
                    if !validate(&doc) {
                        return Err("Unsupported recording file; it has been preserved.".into());
                    }
                    if doc["version"] == 1 {
                        doc["version"] = json!(2);
                        for take in doc["takes"].as_array_mut().unwrap() {
                            take["owner"] = Json::Null
                        }
                    }
                    Ok(Some(doc))
                }
                PendingTrace::Write(_) => unreachable!(),
            }
        })();
        match loaded {
            Ok(Some(doc)) => {
                self.document = Some(doc);
                Answer::Now(self.query(source, identity))
            }
            Ok(None) => match self.pending {
                Some(PendingTrace::Listing) => request("fs.readdir", json!({"path":"app:/data"})),
                Some(PendingTrace::Read(true)) => request(
                    "fs.readFile",
                    json!({"path":"app:/data/gesture-takes.json"}),
                ),
                _ => request("fs.readFile", json!({"path":PATH})),
            },
            Err(e) => {
                self.error = e;
                Answer::Now(self.query(source, identity))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn response(g: &mut Gestures, source: &str, j: Json) -> Answer {
        g.parse(source, Outcome::Storage(j.to_string().into_bytes()), "")
    }
    fn fresh() -> Gestures {
        let mut g = Gestures::default();
        assert!(matches!(
            g.answer("gestureTakes", &[], ""),
            Answer::Later(_)
        ));
        response(&mut g, "gestureTakes", Json::Null);
        response(&mut g, "gestureTakes", json!([]));
        g
    }
    fn fixture() -> Json {
        json!({"schema":"ansuz.gesture-take","version":1,"hz":120,"viewport":[390,500],"mode":0,"reduced_motion":false,
        "initial":{"orientation":[0,0,0,1],"velocity":[0,0],"position":[0,0],"down":false,"pointer":0,"quiet":0},
        "frames":[[0,0,false,0,0,0,0,0,0,0,0,0,1,0,0]],"observed":[]})
    }
    fn args() -> Vec<Value> {
        vec![
            Value::str(&fixture().to_string()),
            Value::str("Catch on contact, then hold still"),
            Value::Number(1234.),
        ]
    }
    #[test]
    fn retry_after_ack_or_reload_does_not_duplicate_a_take() {
        let mut g = fresh();
        assert!(matches!(
            g.answer("saveGestureTake", &args(), ""),
            Answer::Later(_)
        ));
        response(&mut g, "saveGestureTake", Json::Null);
        let document = g.document.clone();
        assert!(matches!(
            g.answer("saveGestureTake", &args(), ""),
            Answer::Now(_)
        ));
        assert!(g.pending.is_none());
        assert_eq!(g.takes("").unwrap().len(), 1);
        let mut reloaded = Gestures {
            document,
            ..Default::default()
        };
        assert!(matches!(
            reloaded.answer("saveGestureTake", &args(), ""),
            Answer::Now(_)
        ));
        assert!(reloaded.pending.is_none());
        assert_eq!(reloaded.takes("").unwrap().len(), 1);
    }
    #[test]
    fn gesture_take_commits_only_after_write_and_preserves_expected_behavior() {
        let mut g = fresh();
        assert!(matches!(
            g.answer("saveGestureTake", &args(), ""),
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
        reloaded.answer("gestureTakes", &[], "");
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
    fn account_restoration_restarts_superseded_file_read() {
        let a = "did:plc:aaaaaaaaaaaaaaaaaaaaaaaa";
        let mut g = fresh();
        let mut values = args();
        values.push(Value::str(a));
        g.answer("saveGestureTake", &values, a);
        response(&mut g, "saveGestureTake", Json::Null);
        let bytes = g.document.as_ref().unwrap().to_string();
        let mut restored = Gestures::default();
        restored.answer("gestureTakes", &[Value::str("")], "");
        response(&mut restored, "gestureTakes", Json::Null);
        // Exact discards that old resource continuation on identity change.
        assert!(matches!(
            restored.answer("gestureTakes", &[Value::str(a)], a),
            Answer::Later(_)
        ));
        response(&mut restored, "gestureTakes", Json::Null);
        response(&mut restored, "gestureTakes", json!([FILE]));
        restored.parse(
            "gestureTakes",
            Outcome::Storage(
                json!({"base64":exact_data::envelope::base64(bytes.as_bytes())})
                    .to_string()
                    .into_bytes(),
            ),
            a,
        );
        assert_eq!(restored.takes(a).unwrap().len(), 1);
        assert!(restored.takes("").unwrap().is_empty());
    }
    #[test]
    fn unreadable_gesture_file_cannot_be_overwritten() {
        let mut g = Gestures::default();
        g.answer("gestureTakes", &[], "");
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
            g.answer("saveGestureTake", &args(), ""),
            Answer::Now(_)
        ));
        assert!(g.pending.is_none());
    }
    #[test]
    fn failed_recording_write_keeps_prior_takes() {
        let mut g = fresh();
        g.answer("saveGestureTake", &args(), "");
        response(&mut g, "saveGestureTake", Json::Null);
        let before = g.document.clone();
        g.answer("saveGestureTake", &args(), "");
        response(&mut g, "saveGestureTake", json!({"error":"disk full"}));
        assert_eq!(g.document, before);
    }
    #[test]
    fn account_takes_are_isolated_and_guest_adoption_is_explicit_and_durable() {
        let a = "did:plc:aaaaaaaaaaaaaaaaaaaaaaaa";
        let b = "did:plc:bbbbbbbbbbbbbbbbbbbbbbbb";
        let mut g = fresh();
        let mut values = args();
        values.push(Value::str(a));
        g.answer("saveGestureTake", &values, a);
        g.parse("saveGestureTake", Outcome::Storage(b"null".to_vec()), b);
        assert_eq!(g.takes(a).unwrap().len(), 1);
        assert!(g.takes(b).unwrap().is_empty());
        assert!(g.takes("").unwrap().is_empty());
        assert!(matches!(
            g.answer("saveGestureTake", &values, b),
            Answer::Now(_)
        ));
        assert!(g.pending.is_none());
        g.answer(
            "deleteGestureTake",
            &[Value::str("take-1"), Value::str(b)],
            b,
        );
        g.parse("deleteGestureTake", Outcome::Storage(b"null".to_vec()), b);
        assert_eq!(g.takes(a).unwrap().len(), 1);
        g.answer("saveGestureTake", &args(), "");
        response(&mut g, "saveGestureTake", Json::Null);
        let original = g.takes("").unwrap()[0]["trace"].clone();
        g.answer(
            "claimGestureTake",
            &[Value::str("take-2"), Value::str(a)],
            a,
        );
        assert_eq!(
            g.takes("").unwrap().len(),
            1,
            "assignment is not committed before file acknowledgement"
        );
        g.parse("claimGestureTake", Outcome::Storage(b"null".to_vec()), a);
        assert!(g.takes("").unwrap().is_empty());
        assert_eq!(g.takes(a).unwrap()[1]["trace"], original);
        assert!(matches!(
            g.answer(
                "claimGestureTake",
                &[Value::str("take-2"), Value::str(b)],
                b
            ),
            Answer::Now(_)
        ));
        assert!(g.takes(b).unwrap().is_empty());
    }
    #[test]
    fn legacy_file_is_read_as_unassigned_and_new_writes_use_a_new_file() {
        let legacy = json!({"version":1,"next":2,"takes":[{"id":"take-1","label":"Old take","expected":"Earth","trace":fixture()}]});
        let mut g = Gestures::default();
        g.answer("gestureTakes", &[], "");
        response(&mut g, "gestureTakes", Json::Null);
        let read = response(&mut g, "gestureTakes", json!([LEGACY]));
        if let Answer::Later(r) = read {
            assert!(String::from_utf8(r.storage.unwrap())
                .unwrap()
                .contains(LEGACY))
        } else {
            panic!()
        }
        response(
            &mut g,
            "gestureTakes",
            json!({"base64":exact_data::envelope::base64(legacy.to_string().as_bytes())}),
        );
        assert_eq!(g.takes("").unwrap().len(), 1);
        let write = g.answer("saveGestureTake", &args(), "");
        if let Answer::Later(r) = write {
            assert!(String::from_utf8(r.storage.unwrap())
                .unwrap()
                .contains(FILE))
        } else {
            panic!()
        }
    }
}

#[cfg(test)]
mod reliability_tests {
    use super::*;
    #[test]
    fn malformed_trace_is_refused_before_writing() {
        let mut g = Gestures {
            document: Some(empty()),
            ..Default::default()
        };
        let args = [
            Value::str(r#"{"schema":"ansuz.gesture-take","version":4,"hz":120,"frames":[[0]]}"#),
            Value::str(""),
            Value::Number(1.),
            Value::str(""),
        ];
        assert!(matches!(
            g.answer("saveGestureTake", &args, ""),
            Answer::Now(_)
        ));
        assert!(g.pending.is_none());
        assert!(g.takes("").unwrap().is_empty());
    }
}
