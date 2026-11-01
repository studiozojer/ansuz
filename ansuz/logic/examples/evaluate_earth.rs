//! Compare a private recording with both Earth recognizers without rewriting it.
//! cargo run --example evaluate_earth -- /private/path/take.json
use ansuz_logic::{elements::Elements, lab::Lab};
use serde_json::json;
fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("provide a gesture-take JSON path");
    let text = std::fs::read_to_string(path).expect("read trace");
    let mut lab = Lab::default();
    lab.load(&text).expect("validate trace");
    let mut results = Vec::new();
    for legacy in [true, false] {
        let mut earth = Elements::default();
        earth.reset(3);
        earth.earth_legacy = legacy;
        let mut triggers = Vec::new();
        let mut phases = Vec::new();
        let mut previous = 0;
        for (i, f) in lab.frames.iter().enumerate() {
            let count = earth.triggers;
            earth.step(&f.contact, f.viewport, 1. / 120., f.reduced_motion);
            if earth.triggers > count {
                triggers.push(i as f64 / 120.);
            }
            if earth.phase != previous {
                phases.push(json!({"seconds":i as f64/120.,"phase":earth.phase}));
                previous = earth.phase;
            }
        }
        results.push(json!({"model":if legacy {"ansuz.elements/1"}else{"ansuz.elements/2"},"triggers":earth.triggers,"trigger_seconds":triggers,"phase_changes":phases}));
    }
    println!("{}", serde_json::to_string_pretty(&json!({"frames":lab.frames.len(),"recorded_element":lab.source_element,"comparison":results})).unwrap());
}
