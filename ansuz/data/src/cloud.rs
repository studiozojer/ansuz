//! Explicit archival sync: the local take file is the durable upload queue.
//! Snapback stores immutable files/history; this is not a device replica.
use super::*;
use exact_runner::Request;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
pub const ORIGIN: &str = "https://ansuz.jera.ink";
pub const SECRET: &str = "ansuz-recordings-session-v1";
// Public identity of the studio reader; no credential is compiled into the app.
pub const AGENT: &str = "user:01M6ATM1EP4D9YXZQD5Q394J8Y";
#[derive(Default)]
pub struct Cloud {
    session: Option<Json>,
    initialized: bool,
    pending: Option<Step>,
    queue: VecDeque<Json>,
    items: Vec<Json>,
    next: Json,
    error: String,
    status: String,
    shared: bool,
}
enum Step {
    Auth,
    Refresh,
    Find(Json),
    Upload(Json),
    Archive,
    History(bool),
    Access,
    Share,
    Download(String),
}
pub fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
fn s(j: &Json, key: &str) -> String {
    j[key].as_str().unwrap_or("").to_string()
}
fn loaded(text: &str, error: &str) -> Value {
    let note = serde_json::from_str::<Json>(text)
        .ok()
        .map(|j| s(&j, "expected"))
        .unwrap_or_default();
    Value::record(vec![Value::str(text), Value::str(error), Value::str(&note)])
}
impl Cloud {
    pub fn owns(source: &str) -> bool {
        matches!(
            source,
            "recordingCloud"
                | "connectRecordings"
                | "syncRecordings"
                | "moreRecordings"
                | "loadCloudTake"
                | "disconnectRecordings"
                | "shareRecordings"
        )
    }
    pub fn view(&self) -> Value {
        Value::record(vec![
            Value::list(
                self.items
                    .iter()
                    .map(|j| {
                        Value::record(vec![
                            Value::str(&s(j, "id")),
                            Value::str(&s(j, "label")),
                            Value::str(&s(j, "expected")),
                            Value::str(&s(j, "digest")),
                            Value::str(&s(&j["trace"], "id")),
                            Value::Number(j["captured"].as_f64().unwrap_or(0.)),
                        ])
                    })
                    .collect(),
            ),
            Value::Bool(self.session.is_some()),
            Value::str(&self.status),
            Value::str(&self.error),
            Value::Bool(!self.next.is_null()),
            Value::Bool(self.shared),
            Value::str(
                &self
                    .session
                    .as_ref()
                    .map(|j| s(j, "principal"))
                    .unwrap_or_default(),
            ),
        ])
    }
    pub fn query(&self, source: &str) -> Value {
        if source == "loadCloudTake" {
            loaded("", "")
        } else {
            self.view()
        }
    }
    fn request(&mut self, path: &str, body: Json, step: Step) -> Answer {
        self.pending = Some(step);
        let mut r = Request::post_json(&format!("{ORIGIN}{path}"), &body.to_string())
            .independent_http(3 * 1024 * 1024)
            .timeout(30000);
        if let Some(session) = &self.session {
            r = r.header("authorization", &format!("Bearer {}", s(session, "token")));
        }
        Answer::Later(r)
    }
    fn fail(&mut self, error: &str) -> Answer {
        self.error = error.into();
        self.status = "Cloud operation paused.".into();
        self.queue.clear();
        self.pending = None;
        Answer::Now(self.view())
    }
    fn owner(&self) -> String {
        self.session
            .as_ref()
            .map(|j| s(j, "principal"))
            .unwrap_or_default()
    }
    fn history(&mut self, more: bool) -> Answer {
        self.request("/q/history",json!({"args":{"owner":self.owner(),"c":if more {self.next.clone()} else {Json::Null}}}),Step::History(more))
    }
    fn advance(&mut self) -> Answer {
        if let Some(take) = self.queue.pop_front() {
            let key = digest(&take["trace"].to_string());
            self.request(
                "/q/findTake",
                json!({"args":{"owner":self.owner(),"digest":key}}),
                Step::Find(take),
            )
        } else {
            self.history(false)
        }
    }
    pub fn answer(
        &mut self,
        store: &mut Store,
        source: &str,
        args: &[Value],
        takes: Option<Vec<Json>>,
    ) -> Answer {
        if !self.initialized {
            self.initialized = true;
            self.session = store
                .get(SECRET)
                .and_then(|v| serde_json::from_str::<Json>(v).ok())
                .filter(|v| v["token"].is_string() && v["principal"].is_string());
        }
        if self.pending.is_some() {
            return if source == "loadCloudTake" {
                Answer::Now(loaded("", "Wait for the current cloud operation."))
            } else {
                Answer::Now(self.view())
            };
        }
        if source != "recordingCloud" {
            self.error.clear();
        }
        match source {
            "recordingCloud" => Answer::Now(self.view()),
            "disconnectRecordings" => {
                if store.forget(SECRET).is_err() {
                    return self.fail("Could not remove the saved session. Try again.");
                }
                self.session = None;
                self.items.clear();
                self.next = Json::Null;
                self.shared = false;
                self.status = "Disconnected. Device takes remain here.".into();
                Answer::Now(self.view())
            }
            "connectRecordings" => {
                let email = args.first().and_then(Value::as_str).unwrap_or("").trim();
                let password = args.get(1).and_then(Value::as_str).unwrap_or("");
                let signup = matches!(args.get(2), Some(Value::Bool(true)));
                if email.is_empty() || password.len() < 8 {
                    return self.fail("Enter an email and a password of at least 8 characters.");
                }
                self.items.clear();
                self.next = Json::Null;
                self.shared = false;
                self.request(
                    if signup {
                        "/auth/signup"
                    } else {
                        "/auth/login"
                    },
                    json!({"email":email,"password":password}),
                    Step::Auth,
                )
            }
            _ if self.session.is_none() => self.fail("Connect your recording account first."),
            "syncRecordings" => {
                let Some(takes) = takes else {
                    return self.fail("Wait for device recordings to load before syncing.");
                };
                self.queue = takes.into();
                self.status = "Syncing device takes…".into();
                self.request("/auth/refresh", json!({}), Step::Refresh)
            }
            "moreRecordings" => self.history(true),
            "shareRecordings" => {
                if AGENT.is_empty() {
                    return self.fail("The studio agent is not configured yet.");
                }
                let enable = matches!(args.first(), Some(Value::Bool(true)));
                let path = if enable {
                    "/m/allowReader"
                } else {
                    "/m/removeReader"
                };
                // Counter is scoped to the persisted session and survives app restarts.
                let session = self.session.as_mut().unwrap();
                let counter = session["ansuz_intent"].as_u64().unwrap_or(0) + 1;
                session["ansuz_intent"] = json!(counter);
                if store.set(SECRET, &session.to_string()).is_err() {
                    return self.fail("Could not save sharing intent.");
                }
                let id = format!("share-{}-{counter}", &digest(&s(session, "token"))[..24]);
                self.request(path, json!({"id":id,"args":{"reader":AGENT}}), Step::Share)
            }
            "loadCloudTake" => {
                let key = args.first().and_then(Value::as_str).unwrap_or("");
                let Some(row) = self.items.iter().find(|j| j["digest"] == key) else {
                    return Answer::Now(loaded("", "Take is no longer in this history page."));
                };
                let asset = s(&row["trace"], "id");
                if !asset
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                {
                    return Answer::Now(loaded("", "Invalid asset identity."));
                }
                self.pending = Some(Step::Download(key.into()));
                Answer::Later(
                    Request::get(&format!("{ORIGIN}/assets/{asset}"))
                        .header(
                            "authorization",
                            &format!("Bearer {}", s(self.session.as_ref().unwrap(), "token")),
                        )
                        .independent_http(2097152)
                        .timeout(30000),
                )
            }
            _ => self.fail("Unknown recording operation."),
        }
    }
    pub fn parse(&mut self, store: &mut Store, outcome: Outcome) -> Answer {
        let Some(step) = self.pending.take() else {
            return self.fail("No cloud operation pending.");
        };
        let result = (|| -> Result<Vec<u8>, String> {
            match outcome {
            Outcome::Response(r) if (200..300).contains(&r.status)=>Ok(r.body),
            Outcome::Response(r)=>Err(if r.status==401 {"Session expired. Sign in again; device takes are safe.".into()}else{format!("Cloud request failed (HTTP {}). Device takes remain saved.",r.status)}),
            _=>Err("Cannot reach recording storage. Device takes remain saved; Sync retries safely.".into()),
        }
        })();
        if let Step::Download(key) = step {
            return Answer::Now(match result {
                Ok(bytes) => match String::from_utf8(bytes) {
                    Ok(text) if digest(&text) == key => loaded(&text, ""),
                    _ => loaded("", "Recording checksum failed. Replay was not loaded."),
                },
                Err(e) => loaded("", &e),
            });
        }
        let bytes = match result {
            Ok(b) => b,
            Err(e) => return self.fail(&e),
        };
        let j: Json = match serde_json::from_slice(&bytes) {
            Ok(j) => j,
            Err(_) => return self.fail("Invalid response from recording storage."),
        };
        if !j["denied"].is_null() || j["state"] == "failed" {
            return self.fail("Recording storage refused the operation. Sign in again or retry Sync; local takes are preserved.");
        }
        match step {
            Step::Auth | Step::Refresh => {
                let session = &j["session"];
                if !session["token"].is_string() || !session["principal"].is_string() {
                    return self.fail("Recording storage did not return a session.");
                }
                if store.set(SECRET, &session.to_string()).is_err() {
                    return self.fail("Could not save the recording session.");
                }
                self.session = Some(session.clone());
                self.status = "Connected. Sync uploads saved device takes.".into();
                if matches!(step, Step::Refresh) {
                    self.advance()
                } else {
                    self.history(false)
                }
            }
            Step::Find(take) => {
                if j["complete"] != true {
                    return self.fail("Cloud lookup was incomplete. Retry Sync.");
                }
                if !j["data"].is_null() {
                    return self.advance();
                }
                let text = take["trace"].to_string();
                self.pending = Some(Step::Upload(take));
                let mut r = Request::post_json(
                    &format!("{ORIGIN}/assets?as=file&name=gesture.json"),
                    &text,
                )
                .independent_http(65536)
                .timeout(30000);
                r = r.header(
                    "authorization",
                    &format!("Bearer {}", s(self.session.as_ref().unwrap(), "token")),
                );
                Answer::Later(r)
            }
            Step::Upload(take) => {
                let asset = &j["asset"]["id"];
                if !asset.is_string() {
                    return self.fail("Recording upload was not confirmed.");
                }
                let trace = &take["trace"];
                let key = digest(&trace.to_string());
                let args = json!({"digest":key,"label":take["label"],"expected":take["expected"],"element":trace["element"].as_u64().unwrap_or(0),"captured":trace["captured_at_ms"].as_f64().unwrap_or(0.) as i64,"frames":trace["frames"].as_array().map(Vec::len).unwrap_or(0),"model":trace["element_model"].as_str().unwrap_or("legacy"),"trace":{"id":asset}});
                // Each new upload gets a new intent. Content identity is the server unique key;
                // after an unknown outcome, findTake checks before uploading or sending again.
                let id = format!("{}-{}", &key[..16], asset.as_str().unwrap());
                self.request("/m/archive", json!({"id":id,"args":args}), Step::Archive)
            }
            Step::Archive => {
                if j["state"] != "sent" {
                    self.fail("Upload pending; Sync checks before retrying.")
                } else {
                    self.advance()
                }
            }
            Step::History(more) => {
                if j["complete"] != true {
                    return self.fail("History was incomplete. Retry Sync.");
                }
                let Some(rows) = j["data"].as_array() else {
                    return self.fail("Invalid history response.");
                };
                if !more {
                    self.items.clear()
                }
                for row in rows {
                    if !self.items.iter().any(|i| i["id"] == row["id"]) {
                        self.items.push(row.clone())
                    }
                }
                self.next = j["next"].clone();
                self.status = "Cloud history refreshed. Original takes are preserved.".into();
                self.request("/q/access", json!({"args":{}}), Step::Access)
            }
            Step::Access => {
                self.shared = j["data"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|r| r["reader"] == AGENT));
                Answer::Now(self.view())
            }
            Step::Share => self.request("/q/access", json!({"args":{}}), Step::Access),
            Step::Download(_) => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exact_runner::{FailureKind, Response};
    fn store() -> Store {
        Store::new(
            &format!("secret.keep {SECRET}"),
            [(
                SECRET.into(),
                json!({"token":"token","principal":"owner"})
                    .to_string()
                    .into_bytes(),
            )],
        )
    }
    fn reply(c: &mut Cloud, s: &mut Store, j: Json) -> Answer {
        c.parse(
            s,
            Outcome::Response(Response {
                status: 200,
                headers: vec![],
                body: j.to_string().into_bytes(),
            }),
        )
    }
    fn request(a: Answer) -> Request {
        if let Answer::Later(r) = a {
            r
        } else {
            panic!("expected request")
        }
    }
    fn take() -> Json {
        json!({"label":"Take 1","expected":"Earth rises","trace":{"schema":"ansuz.gesture-take","version":3,"hz":120,"frames":[[0]],"expected":"Earth rises","captured_at_ms":1000}})
    }
    #[test]
    fn unknown_archive_outcome_checks_content_before_retry_even_after_restart() {
        let mut c = Cloud::default();
        let mut s = store();
        let takes = vec![take()];
        assert!(
            request(c.answer(&mut s, "syncRecordings", &[], Some(takes.clone())))
                .url
                .ends_with("/auth/refresh")
        );
        reply(
            &mut c,
            &mut s,
            json!({"session":{"token":"new","principal":"owner"}}),
        );
        let find_key = if let Some(Step::Find(t)) = &c.pending {
            digest(&t["trace"].to_string())
        } else {
            panic!()
        };
        let upload = request(reply(&mut c, &mut s, json!({"data":null,"complete":true})));
        assert_eq!(digest(std::str::from_utf8(&upload.body).unwrap()), find_key);
        let archive = request(reply(&mut c, &mut s, json!({"asset":{"id":"asset1"}})));
        assert_eq!(
            serde_json::from_slice::<Json>(&archive.body).unwrap()["args"]["digest"],
            find_key
        );
        c.parse(
            &mut s,
            Outcome::Failed {
                kind: FailureKind::Network,
                message: "lost acknowledgement".into(),
            },
        );
        assert!(c.error.contains("Sync retries safely"));
        let mut restarted = Cloud::default();
        restarted.answer(&mut s, "syncRecordings", &[], Some(takes));
        reply(
            &mut restarted,
            &mut s,
            json!({"session":{"token":"newer","principal":"owner"}}),
        );
        let next = request(reply(
            &mut restarted,
            &mut s,
            json!({"complete":true,"data":{"id":"archived"}}),
        ));
        assert!(
            next.url.ends_with("/q/history"),
            "no duplicate upload after a successful but unacknowledged write"
        );
    }
    #[test]
    fn refused_or_partial_lookup_never_uploads_and_disconnect_clears_account_rows() {
        let mut c = Cloud::default();
        let mut s = store();
        c.answer(&mut s, "syncRecordings", &[], Some(vec![take()]));
        reply(
            &mut c,
            &mut s,
            json!({"session":{"token":"new","principal":"owner"}}),
        );
        assert!(matches!(
            reply(&mut c, &mut s, json!({"complete":false,"data":null})),
            Answer::Now(_)
        ));
        assert!(c.error.contains("incomplete"));
        assert!(c.pending.is_none());
        c.answer(&mut s, "recordingCloud", &[], None);
        assert!(
            c.error.contains("incomplete"),
            "resource refresh must retain the visible failure"
        );
        c.items = vec![json!({"id":"private"})];
        c.answer(&mut s, "disconnectRecordings", &[], None);
        assert!(s.get(SECRET).is_none());
        assert!(c.items.is_empty());
        assert!(c.session.is_none());
    }
    #[test]
    fn download_verifies_original_bytes_and_never_passes_tampered_json_to_replay() {
        let mut c = Cloud::default();
        let mut s = store();
        let original = "{\"expected\":\"Earth rises\"}";
        c.pending = Some(Step::Download(digest(original)));
        let got = c.parse(
            &mut s,
            Outcome::Response(Response {
                status: 200,
                headers: vec![],
                body: original.as_bytes().to_vec(),
            }),
        );
        assert!(matches!(got,Answer::Now(v) if v==loaded(original,"")));
        c.pending = Some(Step::Download(digest(original)));
        let got = c.parse(
            &mut s,
            Outcome::Response(Response {
                status: 200,
                headers: vec![],
                body: b"{}".to_vec(),
            }),
        );
        assert!(
            matches!(got,Answer::Now(v) if v==loaded("","Recording checksum failed. Replay was not loaded."))
        );
    }
}
