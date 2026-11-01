//! Common trace contract for local persistence and replay. No renderer dependency.
use serde_json::Value;
use std::collections::BTreeSet;
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
fn numeric(v: &Value) -> bool {
    v.as_f64().is_some_and(|n| n.is_finite() && n.abs() < 1e7)
}
fn integer(v: &Value) -> bool {
    v.as_u64().is_some_and(|n| n <= 9_007_199_254_740_991)
}
fn vector(v: &Value, n: usize) -> bool {
    v.as_array()
        .is_some_and(|a| a.len() == n && a.iter().all(numeric))
}
fn quaternion(v: &[Value]) -> bool {
    v.len() == 4
        && v.iter().all(numeric)
        && (0.9801..=1.0201).contains(
            &v.iter()
                .map(|x| {
                    let n = x.as_f64().unwrap();
                    n * n
                })
                .sum::<f64>(),
        )
}
pub fn validate(j: &Value) -> Result<(), String> {
    let fail = || Err("Invalid or unsupported gesture recording.".into());
    let Some(version) = j["version"].as_u64().filter(|n| (1..=4).contains(n)) else {
        return fail();
    };
    if j["schema"] != "ansuz.gesture-take" || j["hz"] != 120 {
        return fail();
    }
    if !vector(&j["viewport"], 2)
        || j["viewport"][0].as_f64().unwrap() < 1.
        || j["viewport"][1].as_f64().unwrap() < 1.
    {
        return fail();
    }
    if !j["mode"].as_u64().is_some_and(|m| m < 2) || !j["reduced_motion"].is_boolean() {
        return fail();
    }
    let n = &j["initial"];
    if !n["orientation"].as_array().is_some_and(|a| quaternion(a))
        || !vector(&n["velocity"], 2)
        || !vector(&n["position"], 2)
        || !n["down"].is_boolean()
        || !integer(&n["pointer"])
        || !numeric(&n["quiet"])
        || n["quiet"].as_f64().unwrap() < 0.
    {
        return fail();
    }
    if version >= 3
        && (!j["element"].as_u64().is_some_and(|n| n <= 4)
            || !matches!(
                j["element_model"].as_str(),
                Some("ansuz.elements/1" | "ansuz.elements/2")
            ))
    {
        return fail();
    }
    let Some(frames) = j["frames"]
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= 2400)
    else {
        return fail();
    };
    for row in frames {
        let Some(a) = row.as_array() else {
            return fail();
        };
        if !((version >= 3 && a.len() == 22)
            || (version < 3 && a.len() == 18)
            || (version == 1 && a.len() == 15))
        {
            return fail();
        }
        if !a[0].as_u64().is_some_and(|n| n < 16)
            || !integer(&a[1])
            || !a[2].is_boolean()
            || !a[3..a.len().min(17)].iter().all(numeric)
            || !quaternion(&a[9..13])
        {
            return fail();
        }
        if a.len() >= 18
            && (a[15].as_f64().unwrap() < 1. || a[16].as_f64().unwrap() < 1. || !a[17].is_boolean())
        {
            return fail();
        }
        if a.len() == 22
            && (!a[18]
                .as_f64()
                .is_some_and(|n| n.is_finite() && (0. ..=1.).contains(&n))
                || !a[19].as_u64().is_some_and(|n| n <= 4)
                || !a[20..22].iter().all(numeric))
        {
            return fail();
        }
    }
    let Some(observed) = j["observed"].as_array().filter(|a| a.len() <= 4800) else {
        return fail();
    };
    let mut tick = 0;
    for row in observed {
        let Some(a) = row.as_array().filter(|a| a.len() == 7) else {
            return fail();
        };
        if !a[0]
            .as_u64()
            .is_some_and(|n| n >= tick && n <= frames.len() as u64)
            || !a[1].as_f64().is_some_and(|n| n.is_finite() && n >= 0.)
            || !a[2].as_u64().is_some_and(|n| n <= 2)
            || !integer(&a[3])
            || !a[4..6].iter().all(numeric)
            || !a[6].as_u64().is_some_and(|n| n <= u32::MAX as u64)
        {
            return fail();
        }
        tick = a[0].as_u64().unwrap();
    }
    if version == 4 {
        if j["delivered_columns"]
            != serde_json::json!([
                "tick",
                "host_event_ms",
                "phase_0_down_1_move_2_up_3_cancel_4_blur",
                "pointer_id",
                "x",
                "y",
                "dx",
                "dy",
                "buttons"
            ])
        {
            return fail();
        }
        if !matches!(
            j["capture"]["stop_reason"].as_str(),
            Some("manual" | "duration_limit" | "input_limit" | "interrupted")
        ) || !j["capture"]["dropped_events"]
            .as_u64()
            .is_some_and(|n| n <= u32::MAX as u64)
            || j["capture"]["max_contacts"] != 10
        {
            return fail();
        }
        if j["capture"]["dropped_events"] != 0 && j["capture"]["stop_reason"] != "input_limit" {
            return fail();
        }
        let Some(initial) = j["initial_contacts"].as_array().filter(|a| a.len() <= 10) else {
            return fail();
        };
        let mut active = BTreeSet::new();
        for row in initial {
            let Some(a) = row.as_array().filter(|a| a.len() == 3) else {
                return fail();
            };
            if !integer(&a[0])
                || !a[1..3].iter().all(numeric)
                || !active.insert(a[0].as_u64().unwrap())
            {
                return fail();
            }
        }
        let Some(delivered) = j["delivered"].as_array().filter(|a| a.len() <= 8000) else {
            return fail();
        };
        tick = 0;
        for row in delivered {
            let Some(a) = row.as_array().filter(|a| a.len() == 9) else {
                return fail();
            };
            if !a[0]
                .as_u64()
                .is_some_and(|n| n >= tick && n <= frames.len() as u64)
                || !a[1].as_f64().is_some_and(|n| n.is_finite() && n >= 0.)
                || !a[2].as_u64().is_some_and(|n| n <= 4)
                || !integer(&a[3])
                || !a[4..8].iter().all(numeric)
                || !a[8].as_u64().is_some_and(|n| n <= u32::MAX as u64)
            {
                return fail();
            }
            tick = a[0].as_u64().unwrap();
            let id = a[3].as_u64().unwrap();
            match a[2].as_u64().unwrap() {
                0 => {
                    active.insert(id);
                    if active.len() > 10 {
                        return fail();
                    }
                }
                1 => {
                    if !active.contains(&id) {
                        return fail();
                    }
                }
                2 if a[8] != 0 => {
                    if !active.contains(&id) {
                        return fail();
                    }
                }
                2 | 3 => {
                    if !active.remove(&id) {
                        return fail();
                    }
                }
                4 => active.clear(),
                _ => unreachable!(),
            }
        }
    }
    Ok(())
}
