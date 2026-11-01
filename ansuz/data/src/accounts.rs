//! Jera-style public account discovery. Suggestions never establish a session.
use super::*;
use exact_runner::Request;

fn input(args: &[Value]) -> &str {
    args.first().and_then(Value::as_str).unwrap_or("")
}
fn handle(value: &str) -> bool {
    let labels: Vec<_> = value.split('.').collect();
    value.len() <= 253
        && !value.eq_ignore_ascii_case("handle.invalid")
        && labels.len() >= 2
        && labels.iter().all(|s| {
            !s.is_empty()
                && s.len() <= 63
                && s.as_bytes()[0].is_ascii_alphanumeric()
                && s.as_bytes()[s.len() - 1].is_ascii_alphanumeric()
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        && labels
            .last()
            .unwrap()
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
}
pub fn login_input(args: &[Value]) -> Value {
    let text = input(args);
    let value = text.trim().strip_prefix('@').unwrap_or(text.trim());
    let valid = cloud::valid_did(value) || handle(value);
    Value::record(vec![
        Value::str(text),
        Value::str(&if value.starts_with("did:") {
            value.into()
        } else {
            value.to_ascii_lowercase()
        }),
        Value::Bool(valid),
    ])
}
fn view(query: &str, items: Vec<Value>, available: bool, complete: bool) -> Value {
    Value::record(vec![
        Value::str(query),
        Value::list(items),
        Value::Bool(available),
        Value::Bool(complete),
    ])
}
pub fn empty(args: &[Value], available: bool) -> Value {
    view(input(args), vec![], available, available)
}
pub fn answer(args: &[Value]) -> Answer {
    let query = input(args);
    let normalized = query.trim().trim_start_matches('@');
    if normalized.chars().count() < 2 || normalized.starts_with("did:") {
        return Answer::Now(empty(args, true));
    }
    if query.len() > 253 {
        return Answer::Now(empty(args, false));
    }
    // No bearer or credential is sent with public discovery. Keep this request
    // independent of Cloud's serialized authorization/upload state machine.
    Answer::Later(
        Request::post_json(
            &format!("{}/oauth/accounts", cloud::ORIGIN),
            &json!({"query":query}).to_string(),
        )
        .independent_http(65536)
        .timeout(15000),
    )
}
pub fn parse(args: &[Value], outcome: Outcome) -> Value {
    let Outcome::Response(response) = outcome else {
        return empty(args, false);
    };
    if response.status != 200 {
        return empty(args, false);
    }
    let Ok(data) = serde_json::from_slice::<Json>(&response.body) else {
        return empty(args, false);
    };
    // The runner fences superseded requests; also require the service's query
    // echo before making rows selectable for this input.
    if data["query"].as_str() != Some(input(args)) {
        return empty(args, false);
    }
    let Some(items) = data["items"].as_array() else {
        return empty(args, false);
    };
    let rows = items
        .iter()
        .filter_map(|a| {
            let did = a["did"].as_str()?;
            let name = a["handle"].as_str()?;
            if !cloud::valid_did(did) || !handle(name) {
                return None;
            }
            let display: String = a["displayName"]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(160)
                .collect();
            Some(Value::record(vec![
                Value::str(did),
                Value::str(name),
                Value::str(&display),
            ]))
        })
        .take(8)
        .collect();
    view(
        input(args),
        rows,
        data["available"] == true,
        data["complete"] == true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use exact_runner::{FailureKind, Response};
    #[test]
    fn names_search_but_only_full_handles_or_dids_can_continue() {
        for name in [
            "",
            "alice",
            "alice..test",
            "-alice.test",
            "alice.123",
            "handle.invalid",
        ] {
            let v = login_input(&[Value::str(name)]);
            assert_eq!(
                v,
                Value::record(vec![Value::str(name), Value::str(name), Value::Bool(false)])
            );
        }
        assert_eq!(
            login_input(&[Value::str(" @Alice.Test ")]),
            Value::record(vec![
                Value::str(" @Alice.Test "),
                Value::str("alice.test"),
                Value::Bool(true)
            ])
        );
        for value in ["", "a", "@a", "did:plc:aaaaaaaaaaaaaaaaaaaaaaaa"] {
            assert!(matches!(answer(&[Value::str(value)]), Answer::Now(_)));
        }
        let Answer::Later(r) = answer(&[Value::str("alice")]) else {
            panic!("expected public lookup")
        };
        assert!(!r.is_auth());
        assert!(!format!("{r:?}").to_lowercase().contains("bearer"));
    }
    #[test]
    fn mismatched_query_and_failed_lookup_return_no_selectable_accounts() {
        let args = [Value::str("alice")];
        let data = json!({"query":"older","available":true,"complete":true,"items":[{"did":"did:plc:aaaaaaaaaaaaaaaaaaaaaaaa","handle":"alice.test"}]});
        assert_eq!(
            parse(
                &args,
                Outcome::Response(Response {
                    status: 200,
                    headers: vec![],
                    body: data.to_string().into_bytes()
                })
            ),
            empty(&args, false)
        );
        assert_eq!(
            parse(
                &args,
                Outcome::Failed {
                    kind: FailureKind::Network,
                    message: "offline".into()
                }
            ),
            empty(&args, false)
        );
        let data = json!({"query":"alice","available":true,"complete":true,"items":[{"did":"did:plc:aaaaaaaaaaaaaaaaaaaaaaaa","handle":"alice.test","displayName":"Alice"},{"did":"unverified","handle":"other.test"}]});
        assert_eq!(
            parse(
                &args,
                Outcome::Response(Response {
                    status: 200,
                    headers: vec![],
                    body: data.to_string().into_bytes()
                })
            ),
            view(
                "alice",
                vec![Value::record(vec![
                    Value::str("did:plc:aaaaaaaaaaaaaaaaaaaaaaaa"),
                    Value::str("alice.test"),
                    Value::str("Alice")
                ])],
                true,
                true
            )
        );
    }
}
