//! The contract checked inside the container: the wire form the README
//! documents, its defaults and refusals, the builders, and the shared
//! formatting. These are what every daemon and UI compiled against this
//! commit will read and write, so a change that breaks them fails here
//! before any consumer updates its lock.

use serde_json::{json, Value};
use stormview::*;

fn eq<T: PartialEq + std::fmt::Debug>(what: &str, got: T, want: T) -> Result<(), String> {
    if got == want {
        Ok(())
    } else {
        Err(format!("{what}: got {got:?}, want {want:?}"))
    }
}

/// The example summary in the README, as the types build it.
pub fn example() -> ComponentSummary {
    ComponentSummary {
        id: "process:web".into(),
        kind: "process".into(),
        label: "web".into(),
        health: Health::Ok,
        detail: "running · pid 1234 · up 2h 3m".into(),
        metrics: vec![Metric::new("restarts", "0").tone("muted")],
        actions: vec![Action {
            id: "stop".into(),
            label: "Stop".into(),
            method: "POST".into(),
            path: "/api/v1/processes/web/stop".into(),
            enabled: true,
            danger: true,
            tone: None,
        }],
        relations: vec![Relation::belongs_to("system", "system"), Relation::has_one("logs", "logs").href("#/logs?process=web".into())],
        link: Some("#/process/web".into()),
    }
}

/// The README's example JSON, verbatim in meaning.
pub fn example_json() -> Value {
    json!({
        "id": "process:web",
        "kind": "process",
        "label": "web",
        "health": "ok",
        "detail": "running · pid 1234 · up 2h 3m",
        "metrics": [{ "label": "restarts", "value": "0", "tone": "muted" }],
        "actions": [{ "id": "stop", "label": "Stop", "method": "POST",
                      "path": "/api/v1/processes/web/stop", "enabled": true, "danger": true }],
        "relations": [
            { "name": "system", "kind": "belongs_to", "targets": ["system"] },
            { "name": "logs", "kind": "has_one", "targets": ["logs"], "href": "#/logs?process=web" }
        ],
        "link": "#/process/web"
    })
}

/// The documented example serializes to exactly the documented JSON, and
/// that JSON reads back to the same summary.
pub fn wire() -> Result<String, String> {
    let c = example();
    let v = serde_json::to_value(&c).map_err(|e| e.to_string())?;
    eq("serialized example", &v, &example_json())?;
    let back: ComponentSummary = serde_json::from_value(example_json()).map_err(|e| format!("reading the documented JSON: {e}"))?;
    eq("read back", &back, &c)?;
    Ok("the documented summary serializes to the documented JSON and reads back equal".into())
}

/// Only the five required fields: the lists default empty, the optionals
/// absent; metrics/actions are always written, relations/link only when set.
pub fn defaults() -> Result<String, String> {
    let min = json!({"id": "a", "kind": "k", "label": "A", "health": "idle", "detail": ""});
    let c: ComponentSummary = serde_json::from_value(min).map_err(|e| format!("minimal summary: {e}"))?;
    eq("metrics", c.metrics.len(), 0)?;
    eq("actions", c.actions.len(), 0)?;
    eq("relations", c.relations.len(), 0)?;
    eq("link", c.link.clone(), None)?;
    let out = serde_json::to_value(&c).map_err(|e| e.to_string())?;
    let mut keys: Vec<&str> = out.as_object().map(|o| o.keys().map(String::as_str).collect()).unwrap_or_default();
    keys.sort();
    eq("written keys", keys, vec!["actions", "detail", "health", "id", "kind", "label", "metrics"])?;
    let m: Metric = serde_json::from_value(json!({"label": "l", "value": "1"})).map_err(|e| e.to_string())?;
    eq("bare metric written", serde_json::to_value(&m).unwrap(), json!({"label": "l", "value": "1"}))?;
    let r: Relation = serde_json::from_value(json!({"name": "n", "kind": "has_many", "targets": []})).map_err(|e| e.to_string())?;
    eq("relation kind", r.kind, RelationKind::HasMany)?;
    Ok("required fields only: lists empty, optionals absent and omitted".into())
}

/// What the contract refuses: unknown health or relation kinds, wrong case,
/// a missing required field. A renderer relies on these never arriving.
pub fn refusals() -> Result<String, String> {
    let base = example_json();
    let mut cases: Vec<(&str, Value)> = Vec::new();
    let mut v = base.clone();
    v["health"] = json!("fine");
    cases.push(("health \"fine\"", v));
    let mut v = base.clone();
    v["health"] = json!("OK");
    cases.push(("health \"OK\"", v));
    let mut v = base.clone();
    v["relations"][0]["kind"] = json!("belongsTo");
    cases.push(("relation kind \"belongsTo\"", v));
    for field in ["id", "kind", "label", "health", "detail"] {
        let mut v = base.clone();
        v.as_object_mut().unwrap().remove(field);
        cases.push((field, v));
    }
    let mut v = base.clone();
    v["actions"][0].as_object_mut().unwrap().remove("danger");
    cases.push(("action without danger", v));
    let accepted: Vec<&str> = cases
        .iter()
        .filter(|(_, v)| serde_json::from_value::<ComponentSummary>(v.clone()).is_ok())
        .map(|(n, _)| *n)
        .collect();
    if !accepted.is_empty() {
        return Err(format!("accepted what the contract forbids: {accepted:?}"));
    }
    // `kind` is a grouping noun, not an enum: a new one must be accepted.
    let mut v = base;
    v["kind"] = json!("a-kind-no-renderer-knows");
    serde_json::from_value::<ComponentSummary>(v).map_err(|e| format!("an unknown component kind was refused: {e}"))?;
    Ok(format!("{} malformed summaries refused; an unknown kind accepted", cases.len()))
}

/// Health is lowercase on the wire, declared broken-first.
pub fn health() -> Result<String, String> {
    let all = [Health::Error, Health::Warn, Health::Ok, Health::Idle, Health::Unknown];
    let names: Vec<Value> = all.iter().map(|h| serde_json::to_value(h).unwrap()).collect();
    eq("health names", names, vec![json!("error"), json!("warn"), json!("ok"), json!("idle"), json!("unknown")])?;
    Ok("error, warn, ok, idle, unknown".into())
}

/// The builders set what they say, and chain.
pub fn builders() -> Result<String, String> {
    let m = Metric::new("used", "4.0").unit("GB").tone("warn");
    eq("metric", (m.unit.as_deref(), m.tone.as_deref()), (Some("GB"), Some("warn")))?;
    let r = Relation::has_many("drives", vec!["d1".into(), "d2".into()]).href("#/drives".into());
    eq("has_many", (r.kind, r.targets.len(), r.href.as_deref()), (RelationKind::HasMany, 2, Some("#/drives")))?;
    eq("has_one", Relation::has_one("x", "y").targets, vec!["y".to_string()])?;
    eq("belongs_to", Relation::belongs_to("x", "y").kind, RelationKind::BelongsTo)?;
    let a = example().actions.remove(0).tone("ok");
    eq("action tone", serde_json::to_value(&a).unwrap()["tone"].clone(), json!("ok"))?;
    Ok("Metric, Relation and Action builders".into())
}

pub fn format_duration_table() -> Result<String, String> {
    let table: &[(i64, &str)] = &[
        (i64::MIN, "0s"),
        (-1, "0s"),
        (0, "0s"),
        (59, "59s"),
        (60, "1m 0s"),
        (90, "1m 30s"),
        (3599, "59m 59s"),
        (3600, "1h 0m"),
        (3700, "1h 1m"),
        (86399, "23h 59m"),
        (86400, "1d 0h"),
        (90000, "1d 1h"),
        (400 * 86400 + 5 * 3600, "400d 5h"),
    ];
    for (s, want) in table {
        eq(&format!("format_duration({s})"), format_duration(*s).as_str(), *want)?;
    }
    Ok(format!("{} cases, negatives clamp to 0s", table.len()))
}

pub fn format_bytes_table() -> Result<String, String> {
    const K: u64 = 1024;
    let table: &[(u64, &str)] = &[
        (0, "0 B"),
        (512, "512 B"),
        (K - 1, "1023 B"),
        (K, "1.0 KB"),
        (2 * K, "2.0 KB"),
        (K * K - 1, "1024.0 KB"),
        (5 * K * K, "5.0 MB"),
        (3 * K * K * K, "3.0 GB"),
        (K * K * K * K, "1.0 TB"),
        (K * K * K * K * K, "1024.0 TB"),
    ];
    for (b, want) in table {
        eq(&format!("format_bytes({b})"), format_bytes(*b).as_str(), *want)?;
    }
    Ok(format!("{} cases, TB is the top unit", table.len()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_contract_checks_pass_against_this_commit() {
        for (name, f) in [
            ("wire", super::wire as fn() -> Result<String, String>),
            ("defaults", super::defaults),
            ("refusals", super::refusals),
            ("health", super::health),
            ("builders", super::builders),
            ("duration", super::format_duration_table),
            ("bytes", super::format_bytes_table),
        ] {
            if let Err(e) = f() {
                panic!("{name}: {e}");
            }
        }
    }
}
