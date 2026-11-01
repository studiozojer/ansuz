//! Explicit archival sync: the local take file is the durable upload queue.
//! Snapback stores immutable files/history; this is not a device replica.
use super::*;
use exact_runner::Request;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
pub const ORIGIN: &str = "https://ansuz.jera.ink";
pub const SECRET: &str = "ansuz-https-ansuz-jera-ink-atproto-v1";
pub const LOGIN: &str = "ansuz-https-ansuz-jera-ink-atproto-v1-login";
pub const GRANTS: &str = "fs.read app:/data\nfs.write app:/data\nnet.fetch https://ansuz.jera.ink\nauth.session https://ansuz.jera.ink\nauth.callback studio.zojer.ansuz:/oauth\nauth.callback http://127.0.0.1:8768/.exact/auth/callback\nsecret.keep ansuz-https-ansuz-jera-ink-atproto-v1\nsecret.keep ansuz-https-ansuz-jera-ink-atproto-v1-login";
pub fn valid_did(did: &str) -> bool {
    if let Some(id) = did.strip_prefix("did:plc:") {
        return id.len() == 24
            && id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || (b'2'..=b'7').contains(&b));
    }
    did.strip_prefix("did:web:").is_some_and(|id| {
        !id.is_empty()
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:%-".contains(&b))
    })
}
pub fn request_key(source: &str, args: &[Value]) -> String {
    format!("{source}:{args:?}")
}
fn base64url(bytes: &[u8]) -> String {
    exact_data::envelope::base64(bytes)
        .replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .into()
}
fn prepare(store: &mut Store) -> Result<(), String> {
    if store.get(LOGIN).is_some() {
        return Ok(());
    }
    let mut bytes = [0u8; 64];
    exact_data::crypto::random_bytes(store, &mut bytes)
        .map_err(|_| "Secure randomness unavailable.")?;
    let verifier = base64url(&bytes[..32]);
    let state = base64url(&bytes[32..]);
    let challenge = base64url(&Sha256::digest(verifier.as_bytes()));
    let callback = if cfg!(target_arch = "wasm32") {
        "http://127.0.0.1:8768/.exact/auth/callback"
    } else {
        "studio.zojer.ansuz:/oauth"
    };
    store
        .set(
            LOGIN,
            &json!({"verifier":verifier,"state":state,"challenge":challenge,"callback":callback})
                .to_string(),
        )
        .map_err(|_| "Could not save sign-in preparation.".into())
}
// Public identity of the studio reader; no credential is compiled into the app.
pub const AGENT: &str = "operator:ansuz-recorder";
#[derive(Default)]
pub struct Cloud {
    session: Option<Json>,
    initialized: bool,
    ready: bool,
    now_ms: f64,
    pending_key: String,
    pending: Option<Step>,
    queue: VecDeque<Json>,
    items: Vec<Json>,
    next: Json,
    error: String,
    status: String,
    shared: bool,
    confirmed: Vec<String>,
    current_digest: String,
}
enum Step {
    Browser(Json),
    Exchange,
    Check,
    Logout,
    Refresh(Next),
    Find(Json),
    Upload(Json),
    Archive,
    Confirm,
    History(bool),
    Access,
    Share,
    Download(String),
}
enum Next {
    Sync,
    More,
    Share(bool),
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
            Value::Bool(self.ready),
            Value::str(
                &self
                    .session
                    .as_ref()
                    .map(|j| s(j, "principal"))
                    .unwrap_or_default(),
            ),
            Value::str(&json!(self.confirmed).to_string()),
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
    pub fn owner(&self) -> String {
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
            self.current_digest = key.clone();
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
        if let Some(Value::Number(now)) = args.last() {
            if *now > 1e12 {
                self.now_ms = *now
            }
        }
        if source == "disconnectRecordings" {
            let old = self.session.as_ref().map(|j| s(j, "token"));
            if store.forget(SECRET).is_err() || store.forget(LOGIN).is_err() {
                return self.fail("Could not remove the saved account. Try again.");
            }
            self.clear();
            self.ready = true;
            self.initialized = true;
            self.pending_key = request_key(source, args);
            self.status = "Signed out. Account recordings remain preserved.".into();
            let _ = prepare(store);
            if let Some(token) = old {
                let answer = self.request("/auth/logout", json!({}), Step::Logout);
                if let Answer::Later(r) = answer {
                    return Answer::Later(r.header("authorization", &format!("Bearer {token}")));
                }
            }
            return Answer::Now(self.view());
        }
        if self.pending.is_some() {
            return Answer::Now(self.query(source));
        }
        self.pending_key = request_key(source, args);
        if !self.initialized && source != "recordingCloud" {
            return self.fail_for(source, "Wait for the account to finish loading.");
        }
        if !self.initialized {
            self.initialized = true;
            if let Err(e) = prepare(store) {
                return self.fail(&e);
            }
            self.session = store
                .get(SECRET)
                .and_then(|v| serde_json::from_str::<Json>(v).ok())
                .filter(|j| valid_did(&s(j, "principal")) && j["token"].is_string());
            if self.session.is_some() {
                return self.request("/oauth/check", json!({}), Step::Check);
            }
            self.ready = true;
        }
        if source != "recordingCloud" {
            self.error.clear()
        }
        match source {
            "recordingCloud" => Answer::Now(self.view()),
            "connectRecordings" => {
                if self.session.is_some() {
                    return self.fail("Sign out before changing accounts.");
                }
                if let Err(e) = prepare(store) {
                    return self.fail(&e);
                }
                let flow: Json = match store.get(LOGIN).and_then(|j| serde_json::from_str(j).ok()) {
                    Some(j) => j,
                    None => return self.fail("Prepare sign-in again."),
                };
                let handle = args
                    .first()
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .trim_start_matches('@');
                if handle.is_empty() {
                    return self.fail("Enter your everything-account handle or DID.");
                }
                let query = url::form_urlencoded::Serializer::new(String::new())
                    .append_pair("handle", handle)
                    .append_pair("state", &s(&flow, "state"))
                    .append_pair("challenge", &s(&flow, "challenge"))
                    .append_pair("callback", &s(&flow, "callback"))
                    .finish();
                let request = exact_data::auth::open_auth_session(
                    &format!("{ORIGIN}/oauth/start?{query}"),
                    &s(&flow, "callback"),
                    &s(&flow, "state"),
                    false,
                );
                self.pending = Some(Step::Browser(flow));
                Answer::Later(request)
            }
            _ if self.session.is_none() => self.fail("Sign in with your everything account first."),
            "syncRecordings" => {
                let Some(takes) = takes else {
                    return self.fail("Wait for this account's device takes to load.");
                };
                if takes
                    .iter()
                    .any(|t| t["owner"].as_str() != Some(self.owner().as_str()))
                {
                    return self.fail("A take belongs to another account. Nothing was uploaded.");
                }
                self.queue = takes
                    .into_iter()
                    .filter(|t| t["upload_state"] == "queued")
                    .collect();
                self.status = "Syncing this account's takes…".into();
                self.request("/auth/refresh", json!({}), Step::Refresh(Next::Sync))
            }
            "moreRecordings" => self.request("/auth/refresh", json!({}), Step::Refresh(Next::More)),
            "shareRecordings" => self.request(
                "/auth/refresh",
                json!({}),
                Step::Refresh(Next::Share(matches!(args.first(), Some(Value::Bool(true))))),
            ),
            "loadCloudTake" => self.request(
                "/auth/refresh",
                json!({}),
                Step::Refresh(Next::Download(
                    args.first().and_then(Value::as_str).unwrap_or("").into(),
                )),
            ),
            _ => self.fail("Unknown recording operation."),
        }
    }
    fn clear(&mut self) {
        self.confirmed.clear();
        self.current_digest.clear();
        self.session = None;
        self.pending = None;
        self.queue.clear();
        self.items.clear();
        self.next = Json::Null;
        self.shared = false;
        self.error.clear()
    }
    fn after_check(&mut self, store: &mut Store, next: Next) -> Answer {
        match next {
            Next::Sync => self.advance(),
            Next::More => self.history(true),
            Next::Share(enable) => {
                let id = match exact_data::crypto::random_uuid(store) {
                    Ok(id) => id,
                    Err(_) => return self.fail("Could not create sharing intent."),
                };
                self.request(
                    if enable {
                        "/m/allowReader"
                    } else {
                        "/m/removeReader"
                    },
                    json!({"id":id,"args":{"reader":AGENT}}),
                    Step::Share,
                )
            }
            Next::Download(key) => {
                let Some(row) = self.items.iter().find(|j| j["digest"] == key) else {
                    return Answer::Now(loaded("", "Take is no longer in this account's history."));
                };
                let asset = s(&row["trace"], "id");
                if asset.is_empty()
                    || !asset
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                {
                    return Answer::Now(loaded("", "Invalid asset identity."));
                }
                self.pending = Some(Step::Download(key));
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
        }
    }
    fn fail_for(&mut self, source: &str, error: &str) -> Answer {
        let result = self.fail(error);
        if source == "loadCloudTake" {
            Answer::Now(loaded("", error))
        } else {
            result
        }
    }
    pub fn parse(
        &mut self,
        store: &mut Store,
        key: &str,
        source: &str,
        outcome: Outcome,
    ) -> Answer {
        if key != self.pending_key {
            return Answer::Now(self.query(source));
        }

        let Some(step) = self.pending.take() else {
            return self.fail_for(source, "No cloud operation pending.");
        };
        if let Step::Browser(flow) = step {
            let active = store
                .get(LOGIN)
                .and_then(|j| serde_json::from_str::<Json>(j).ok());
            if active.as_ref() != Some(&flow) || self.session.is_some() {
                return self.fail_for(source, "Sign-in attempt changed. Start again.");
            }
            let _ = store.forget(LOGIN);
            let back = match exact_data::auth::reply(&outcome) {
                Ok(s) => s,
                Err(_) => {
                    let _ = prepare(store);
                    return self.fail_for(source, "Sign-in cancelled or unavailable. Try again.");
                }
            };
            let url = match url::Url::parse(&back) {
                Ok(u) => u,
                Err(_) => return self.fail_for(source, "Invalid sign-in callback."),
            };
            let mut base = url.clone();
            base.set_query(None);
            base.set_fragment(None);
            let values: Vec<_> = url.query_pairs().collect();
            let state: Vec<_> = values.iter().filter(|(k, _)| k == "state").collect();
            let codes: Vec<_> = values.iter().filter(|(k, _)| k == "code").collect();
            if base.as_str() != s(&flow, "callback")
                || url.fragment().is_some()
                || state.len() != 1
                || codes.len() != 1
                || state[0].1 != s(&flow, "state")
                || codes[0].1.len() != 43
            {
                return self.fail_for(source, "Sign-in callback could not be verified.");
            }
            return self.request(
                "/oauth/exchange",
                json!({"state":flow["state"],"code":codes[0].1,"verifier":flow["verifier"]}),
                Step::Exchange,
            );
        }
        if matches!(&outcome,Outcome::Response(r) if r.status==401)
            && !matches!(step, Step::Exchange | Step::Logout)
        {
            let _ = store.forget(SECRET);
            self.clear();
            self.ready = true;
            let _ = prepare(store);
            let error = "Account authorization ended. Sign in again; recordings are preserved.";
            return if source == "loadCloudTake" {
                Answer::Now(loaded("", error))
            } else {
                self.fail_for(source, error)
            };
        }
        if matches!(step, Step::Logout) {
            self.status = if matches!(outcome,Outcome::Response(r) if r.status==200) {
                "Signed out."
            } else {
                "Signed out on this device; remote session revocation was not confirmed."
            }
            .into();
            return Answer::Now(self.view());
        }
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
            Err(e) => {
                self.ready = true;
                return if source == "loadCloudTake" {
                    Answer::Now(loaded("", &e))
                } else {
                    self.fail_for(source, &e)
                };
            }
        };
        let j: Json = match serde_json::from_slice(&bytes) {
            Ok(j) => j,
            Err(_) => return self.fail_for(source, "Invalid response from recording storage."),
        };
        if !j["denied"].is_null() || j["state"] == "failed" {
            return if source == "loadCloudTake" {
                Answer::Now(loaded("", "Recording access was refused."))
            } else {
                self.fail_for(source, "Recording storage refused the operation. Sign in again or retry Sync; local takes are preserved.")
            };
        }
        match step {
            Step::Check => {
                self.ready = true;
                if s(&j, "did") != self.owner() {
                    let _ = store.forget(SECRET);
                    self.clear();
                    return self.fail_for(source, "Account binding changed. Sign in again.");
                }
                self.status = "Account restored.".into();
                Answer::Now(self.view())
            }
            Step::Exchange | Step::Refresh(_) => {
                let session = &j["session"];
                let did = s(session, "principal");
                let refreshing = matches!(step, Step::Refresh(_));
                if !valid_did(&did)
                    || !session["token"].is_string()
                    || !session["expiresAt"]
                        .as_f64()
                        .is_some_and(|t| t.is_finite() && t > self.now_ms)
                    || (refreshing && did != self.owner())
                {
                    let _ = store.forget(SECRET);
                    self.clear();
                    self.ready = true;
                    return self
                        .fail_for(source, "Invalid or changed account binding. Sign in again.");
                }
                if store.set(SECRET, &session.to_string()).is_err() {
                    return self.fail_for(source, "Could not save the verified account.");
                }
                self.session = Some(session.clone());
                self.ready = true;
                match step {
                    Step::Refresh(next) => self.after_check(store, next),
                    _ => {
                        self.status = "Signed in. Only this account's takes will sync.".into();
                        self.history(false)
                    }
                }
            }
            Step::Find(take) => {
                if j["complete"] != true {
                    return self.fail_for(source, "Cloud lookup was incomplete. Retry Sync.");
                }
                if !j["data"].is_null() {
                    if j["data"]["owner"] != self.owner()
                        || j["data"]["digest"] != self.current_digest
                    {
                        return self.fail_for(source, "Invalid upload confirmation.");
                    }
                    if !self.confirmed.contains(&self.current_digest) {
                        self.confirmed.push(self.current_digest.clone());
                    }
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
                    return self.fail_for(source, "Recording upload was not confirmed.");
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
                    self.fail_for(source, "Upload pending; Sync checks before retrying.")
                } else {
                    // A successful archive acknowledgement is verified by lookup,
                    // including the server's final owner/digest binding.
                    self.request(
                        "/q/findTake",
                        json!({"args":{"owner":self.owner(),"digest":self.current_digest}}),
                        Step::Confirm,
                    )
                }
            }
            Step::Confirm => {
                if j["complete"] != true
                    || j["data"]["owner"] != self.owner()
                    || j["data"]["digest"] != self.current_digest
                {
                    return self
                        .fail_for(source, "Archive confirmation was incomplete. Retry safely.");
                }
                if !self.confirmed.contains(&self.current_digest) {
                    self.confirmed.push(self.current_digest.clone());
                }
                self.advance()
            }
            Step::History(more) => {
                if j["complete"] != true {
                    return self.fail_for(source, "History was incomplete. Retry Sync.");
                }
                let Some(rows) = j["data"].as_array() else {
                    return self.fail_for(source, "Invalid history response.");
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
            Step::Download(_) | Step::Browser(_) | Step::Logout => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exact_runner::{FailureKind, Response};
    const A: &str = "did:plc:aaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "did:plc:bbbbbbbbbbbbbbbbbbbbbbbb";
    fn session(did: &str) -> Json {
        json!({"principal":did,"token":format!("token-{did}"),"expiresAt":9000000000000u64})
    }
    fn store() -> Store {
        Store::new(GRANTS, [])
    }
    fn reply(c: &mut Cloud, s: &mut Store, j: Json) -> Answer {
        let key = c.pending_key.clone();
        c.parse(
            s,
            &key,
            "test",
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
    fn signed(did: &str) -> (Cloud, Store) {
        let mut s = store();
        s.set(SECRET, &session(did).to_string()).unwrap();
        let mut c = Cloud::default();
        c.answer(&mut s, "recordingCloud", &[], None);
        reply(&mut c, &mut s, json!({"did":did}));
        (c, s)
    }
    #[test]
    fn cached_identity_is_checked_and_old_password_secret_is_never_loaded() {
        let mut c = Cloud::default();
        let mut s = Store::new(
            &format!("{GRANTS}\nsecret.keep ansuz-recordings-session-v1"),
            [(
                "ansuz-recordings-session-v1".into(),
                session(A).to_string().into_bytes(),
            )],
        );
        assert!(matches!(
            c.answer(&mut s, "recordingCloud", &[], None),
            Answer::Now(_)
        ));
        assert!(c.owner().is_empty());
        assert!(s.get(LOGIN).is_some());
        let (c, _) = signed(A);
        assert_eq!(c.owner(), A);
        assert!(c.ready);
    }
    #[test]
    fn signed_out_or_wrong_account_takes_never_upload() {
        let (mut c, mut s) = signed(A);
        let result = c.answer(
            &mut s,
            "syncRecordings",
            &[Value::Number(1.)],
            Some(vec![json!({"owner":B,"trace":{}})]),
        );
        assert!(matches!(result, Answer::Now(_)));
        assert!(c.error.contains("another account"));
        assert!(c.pending.is_none());
        let result = c.answer(
            &mut s,
            "syncRecordings",
            &[Value::Number(2.)],
            Some(vec![json!({"owner":null,"trace":{}})]),
        );
        assert!(matches!(result, Answer::Now(_)));
    }
    #[test]
    fn logout_fences_late_authorization_and_another_accounts_download() {
        let (mut c, mut s) = signed(A);
        c.items = vec![json!({"id":"a","digest":"x","trace":{"id":"asset"}})];
        c.answer(
            &mut s,
            "loadCloudTake",
            &[Value::str("x"), Value::Number(1.)],
            None,
        );
        let old = c.pending_key.clone();
        c.answer(&mut s, "disconnectRecordings", &[Value::Number(2.)], None);
        assert!(c.owner().is_empty());
        assert!(c.items.is_empty());
        assert!(s.get(SECRET).is_none());
        c.parse(
            &mut s,
            &old,
            "loadCloudTake",
            Outcome::Response(Response {
                status: 200,
                headers: vec![],
                body: json!({"session":session(A)}).to_string().into_bytes(),
            }),
        );
        assert!(c.owner().is_empty());
        assert!(matches!(c.pending, Some(Step::Logout)));
        reply(&mut c, &mut s, json!({}));
        c.answer(
            &mut s,
            "connectRecordings",
            &[Value::str("alice.test"), Value::Number(3.)],
            None,
        );
        let old = c.pending_key.clone();
        c.answer(&mut s, "disconnectRecordings", &[Value::Number(4.)], None);
        c.parse(
            &mut s,
            &old,
            "connectRecordings",
            Outcome::Response(Response {
                status: 200,
                headers: vec![],
                body: json!({"session":session(A)}).to_string().into_bytes(),
            }),
        );
        assert!(s.get(SECRET).is_none());
        assert!(c.owner().is_empty());
    }
    #[test]
    fn refresh_cannot_substitute_another_did_and_revocation_clears_local_access() {
        let (mut c, mut s) = signed(A);
        c.answer(&mut s, "syncRecordings", &[Value::Number(1.)], Some(vec![]));
        reply(&mut c, &mut s, json!({"session":session(B)}));
        assert!(c.owner().is_empty());
        assert!(s.get(SECRET).is_none());
        let (mut c, mut s) = signed(A);
        c.answer(&mut s, "syncRecordings", &[Value::Number(2.)], Some(vec![]));
        let key = c.pending_key.clone();
        c.parse(
            &mut s,
            &key,
            "syncRecordings",
            Outcome::Response(Response {
                status: 401,
                headers: vec![],
                body: vec![],
            }),
        );
        assert!(c.owner().is_empty());
        assert!(s.get(SECRET).is_none());
    }
    #[test]
    fn offline_check_keeps_previously_verified_account_and_its_error() {
        let mut s = store();
        s.set(SECRET, &session(A).to_string()).unwrap();
        let mut c = Cloud::default();
        c.answer(&mut s, "recordingCloud", &[], None);
        let key = c.pending_key.clone();
        c.parse(
            &mut s,
            &key,
            "recordingCloud",
            Outcome::Failed {
                kind: FailureKind::Network,
                message: "offline".into(),
            },
        );
        assert_eq!(c.owner(), A);
        assert!(c.ready);
        c.answer(&mut s, "recordingCloud", &[], None);
        assert!(!c.error.is_empty());
    }
    #[test]
    fn callback_requires_exact_target_state_and_unexpired_verified_session() {
        let mut c = Cloud::default();
        let mut s = store();
        c.answer(&mut s, "recordingCloud", &[], None);
        let r = request(c.answer(
            &mut s,
            "connectRecordings",
            &[Value::str("alice.test"), Value::Number(1793500000000.)],
            None,
        ));
        assert!(r.is_auth());
        let flow: Json = serde_json::from_str(s.get(LOGIN).unwrap()).unwrap();
        let key = c.pending_key.clone();
        let fake = format!(
            "studio.zojer.evil:/oauth?state={}&code={}",
            flow["state"].as_str().unwrap(),
            "a".repeat(43)
        );
        c.parse(
            &mut s,
            &key,
            "connectRecordings",
            Outcome::Response(Response {
                status: 200,
                headers: vec![],
                body: fake.into_bytes(),
            }),
        );
        assert!(c.owner().is_empty());
        assert!(s.get(SECRET).is_none());
        c.pending = Some(Step::Exchange);
        reply(
            &mut c,
            &mut s,
            json!({"session":{"principal":A,"token":"x","expiresAt":1}}),
        );
        assert!(c.owner().is_empty());
    }
    #[test]
    fn download_checksum_failure_returns_no_replay_and_refusal_clears_identity() {
        let (mut c, mut s) = signed(A);
        c.pending_key = "download".into();
        c.pending = Some(Step::Download(digest("original")));
        let answer = c.parse(
            &mut s,
            "download",
            "loadCloudTake",
            Outcome::Response(Response {
                status: 200,
                headers: vec![],
                body: b"changed".to_vec(),
            }),
        );
        let Answer::Now(value) = answer else {
            panic!("expected refusal")
        };
        assert_eq!(
            value,
            loaded("", "Recording checksum failed. Replay was not loaded.")
        );
        c.pending = Some(Step::Download(digest("original")));
        c.parse(
            &mut s,
            "download",
            "loadCloudTake",
            Outcome::Response(Response {
                status: 401,
                headers: vec![],
                body: vec![],
            }),
        );
        assert!(c.owner().is_empty());
        assert!(s.get(SECRET).is_none());
    }
    #[test]
    fn uncertain_archive_outcome_checks_existing_content_after_restart() {
        let (mut c, mut s) = signed(A);
        let takes = vec![
            json!({"owner":A,"upload_state":"queued","label":"Take 1","expected":"Earth","trace":{"frames":[[0]],"expected":"Earth"}}),
        ];
        c.answer(
            &mut s,
            "syncRecordings",
            &[Value::Number(1.)],
            Some(takes.clone()),
        );
        reply(&mut c, &mut s, json!({"session":session(A)}));
        reply(&mut c, &mut s, json!({"data":null,"complete":true}));
        reply(&mut c, &mut s, json!({"asset":{"id":"asset1"}}));
        let key = c.pending_key.clone();
        c.parse(
            &mut s,
            &key,
            "syncRecordings",
            Outcome::Failed {
                kind: FailureKind::Network,
                message: "ack lost".into(),
            },
        );
        let mut c = Cloud::default();
        c.answer(&mut s, "recordingCloud", &[], None);
        reply(&mut c, &mut s, json!({"did":A}));
        c.answer(&mut s, "syncRecordings", &[Value::Number(2.)], Some(takes));
        reply(&mut c, &mut s, json!({"session":session(A)}));
        let receipt_digest = c.current_digest.clone();
        assert!(request(reply(
            &mut c,
            &mut s,
            json!({"data":{"id":"stored","owner":A,"digest":receipt_digest},"complete":true})
        ))
        .url
        .ends_with("/q/history"));
    }
}
