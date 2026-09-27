//! The contract on the node: the components feeds its daemons serve, read
//! with this commit's types. This is where a stormview change meets the
//! daemons already running — a field renamed here, or one a daemon invented
//! on its own, shows up as a feed that no longer reads, or no longer reads
//! back the same.

use crate::env::FeedAddr;
use crate::http::{self, Ws};
use serde_json::Value;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use stormview::ComponentSummary;

pub const PATH: &str = "/api/v1/components";
pub const WS_PATH: &str = "/ws/components";

/// A feed that answered.
pub struct Feed {
    pub name: String,
    pub addr: String,
    pub raw: Value,
    pub summaries: Vec<ComponentSummary>,
}

pub enum Probe {
    Found(Feed),
    /// Something answered on every candidate that wasn't this feed, or
    /// nothing did.
    Absent(String),
    /// It is there, behind a login (401/403). The runner hands no
    /// credentials, and none are baked in.
    Auth(String),
    /// It is there and doesn't read with this contract.
    Bad(String),
}

/// Parse a feed body: JSON, an array, every element a summary.
pub fn parse(body: &[u8]) -> Result<(Value, Vec<ComponentSummary>), String> {
    let raw: Value = serde_json::from_slice(body).map_err(|e| format!("not JSON: {e}"))?;
    let arr = raw.as_array().ok_or("not a JSON array")?;
    let mut out = Vec::with_capacity(arr.len());
    for (i, v) in arr.iter().enumerate() {
        let c: ComponentSummary = serde_json::from_value(v.clone()).map_err(|e| {
            let id = v.get("id").and_then(Value::as_str).unwrap_or("?");
            format!("summary #{i} (id {id:?}) does not read: {e}")
        })?;
        out.push(c);
    }
    Ok((raw, out))
}

pub fn probe(f: &FeedAddr, timeout: Duration) -> Probe {
    let mut tried = Vec::new();
    for addr in &f.candidates {
        match http::get(addr, PATH, timeout) {
            Err(e) => tried.push(format!("{addr}: {e}")),
            Ok(r) if r.status == 401 || r.status == 403 => {
                return Probe::Auth(format!("{addr}{PATH} answers {} (behind a login)", r.status))
            }
            Ok(r) if r.status == 404 => tried.push(format!("{addr}: 404, not a components feed")),
            Ok(r) if r.status != 200 => return Probe::Bad(format!("{addr}{PATH} answers {}", r.status)),
            Ok(r) => {
                return match parse(&r.body) {
                    Ok((raw, summaries)) => Probe::Found(Feed { name: f.name.clone(), addr: addr.clone(), raw, summaries }),
                    Err(e) => Probe::Bad(format!("{addr}{PATH}: {e}")),
                }
            }
        }
    }
    Probe::Absent(tried.join("; "))
}

const METRIC_TONES: &[&str] = &["ok", "warn", "error", "muted", "accent"];
const ACTION_TONES: &[&str] = &["ok", "warn", "accent", "muted"];
const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE"];

/// What the contract promises about a feed beyond its types: ids unique and
/// non-empty, relation targets in the same feed, actions with a real method
/// and an absolute path, tones from the documented vocabulary, links that
/// are hash routes. Every problem, not just the first.
pub fn integrity(feed: &[ComponentSummary]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut ids = HashSet::new();
    for c in feed {
        if c.id.is_empty() {
            problems.push(format!("a {:?} component has an empty id", c.kind));
        } else if !ids.insert(c.id.as_str()) {
            problems.push(format!("{}: id repeated", c.id));
        }
    }
    for c in feed {
        let id = &c.id;
        if c.kind.is_empty() || c.label.is_empty() {
            problems.push(format!("{id}: empty kind or label"));
        }
        for m in &c.metrics {
            if let Some(t) = m.tone.as_deref().filter(|t| !METRIC_TONES.contains(t)) {
                problems.push(format!("{id}: metric {:?} tone {t:?} is not one of {METRIC_TONES:?}", m.label));
            }
        }
        let mut action_ids = HashSet::new();
        for a in &c.actions {
            if !action_ids.insert(a.id.as_str()) {
                problems.push(format!("{id}: action {:?} repeated", a.id));
            }
            if !METHODS.contains(&a.method.as_str()) {
                problems.push(format!("{id}: action {:?} method {:?}", a.id, a.method));
            }
            if !a.path.starts_with('/') {
                problems.push(format!("{id}: action {:?} path {:?} is not an absolute API path", a.id, a.path));
            }
            if let Some(t) = a.tone.as_deref().filter(|t| !ACTION_TONES.contains(t)) {
                problems.push(format!("{id}: action {:?} tone {t:?} is not one of {ACTION_TONES:?}", a.id));
            }
        }
        for r in &c.relations {
            if r.name.is_empty() {
                problems.push(format!("{id}: a relation with no name"));
            }
            for t in r.targets.iter().filter(|t| !ids.contains(t.as_str())) {
                problems.push(format!("{id}: relation {:?} targets {t:?}, which is not in the feed", r.name));
            }
        }
        if let Some(l) = c.link.as_deref().filter(|l| !l.starts_with('#')) {
            problems.push(format!("{id}: link {l:?} is not a hash route"));
        }
    }
    problems
}

/// What a summary's wire form holds that this commit's types don't read
/// back: fields a daemon invented, or values the types would change. Nulls
/// and an empty `relations` are what the types omit, so they don't count.
pub fn drift(raw: &Value, feed: &[ComponentSummary]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(arr) = raw.as_array() else { return vec!["not an array".into()] };
    for (v, c) in arr.iter().zip(feed) {
        let back = serde_json::to_value(c).unwrap_or(Value::Null);
        let norm = normalize(v);
        if norm != back {
            out.push(format!("{}: {}", c.id, diff(&norm, &back, "")));
        }
    }
    out
}

/// Drop nulls everywhere, and an empty `relations`, the way the types write.
fn normalize(v: &Value) -> Value {
    match v {
        Value::Object(o) => Value::Object(
            o.iter()
                .filter(|(k, v)| !v.is_null() && !(k.as_str() == "relations" && v.as_array().is_some_and(Vec::is_empty)))
                .map(|(k, v)| (k.clone(), normalize(v)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(normalize).collect()),
        other => other.clone(),
    }
}

/// The first place two values differ, as a JSON path.
fn diff(sent: &Value, read: &Value, at: &str) -> String {
    match (sent, read) {
        (Value::Object(a), Value::Object(b)) => {
            if let Some(k) = a.keys().find(|k| !b.contains_key(*k)) {
                return format!("{at}.{k} is not in the contract");
            }
            if let Some(k) = b.keys().find(|k| !a.contains_key(*k)) {
                return format!("{at}.{k} was not sent");
            }
            a.iter().find(|(k, v)| b[k.as_str()] != **v).map(|(k, v)| diff(v, &b[k.as_str()], &format!("{at}.{k}"))).unwrap_or_default()
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            a.iter().zip(b).enumerate().find(|(_, (x, y))| x != y).map(|(i, (x, y))| diff(x, y, &format!("{at}[{i}]"))).unwrap_or_default()
        }
        _ => format!("{} sent {sent}, reads back as {read}", if at.is_empty() { "." } else { at }),
    }
}

pub enum WsOutcome {
    Same(usize),
    Refused(u16),
    Differs(String),
}

/// The first `/ws/components` snapshot reads with the contract and holds the
/// same components as `GET` (read again if the first comparison misses: the
/// feed may change in between).
pub fn ws_matches(feed: &Feed, timeout: Duration) -> Result<WsOutcome, String> {
    let body = match http::ws_first_text(&feed.addr, WS_PATH, timeout).map_err(|e| e.to_string())? {
        Ws::Refused(s) => return Ok(WsOutcome::Refused(s)),
        Ws::Text(b) => b,
    };
    let (_, snap) = parse(&body).map_err(|e| format!("snapshot: {e}"))?;
    let ids = |f: &[ComponentSummary]| f.iter().map(|c| c.id.clone()).collect::<HashSet<_>>();
    let got = ids(&snap);
    if got == ids(&feed.summaries) {
        return Ok(WsOutcome::Same(snap.len()));
    }
    let again = http::get(&feed.addr, PATH, timeout).map_err(|e| e.to_string())?;
    let (_, now) = parse(&again.body)?;
    let want = ids(&now);
    if got == want {
        return Ok(WsOutcome::Same(snap.len()));
    }
    let mut only_ws: Vec<_> = got.difference(&want).cloned().collect();
    let mut only_get: Vec<_> = want.difference(&got).cloned().collect();
    only_ws.sort();
    only_get.sort();
    Ok(WsOutcome::Differs(format!("only in the snapshot: {only_ws:?}; only in GET: {only_get:?}")))
}

/// Read a feed once, timed: (microseconds, component count).
pub fn read_timed(addr: &str, timeout: Duration) -> Result<(u64, usize), String> {
    let t = Instant::now();
    let r = http::get(addr, PATH, timeout).map_err(|e| e.to_string())?;
    if r.status != 200 {
        return Err(format!("{addr}{PATH} answers {}", r.status));
    }
    let (_, f) = parse(&r.body)?;
    Ok((t.elapsed().as_micros() as u64, f.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{example, example_json};
    use serde_json::json;

    #[test]
    fn a_documented_feed_is_clean() {
        let mut sys = example();
        sys.id = "system".into();
        sys.relations.clear();
        let mut logs = sys.clone();
        logs.id = "logs".into();
        let feed = vec![example(), sys, logs];
        assert_eq!(integrity(&feed), Vec::<String>::new());
        let raw = serde_json::to_value(&feed).unwrap();
        assert!(drift(&raw, &feed).is_empty());
    }

    #[test]
    fn integrity_finds_every_problem() {
        let mut c = example(); // its relation targets are not in a one-item feed
        c.actions[0].method = "post".into();
        c.link = Some("/x".into());
        c.metrics[0].tone = Some("red".into());
        let p = integrity(&[c.clone(), c]);
        assert!(p.iter().any(|p| p.contains("id repeated")), "{p:?}");
        assert!(p.iter().any(|p| p.contains("targets \"system\"")), "{p:?}");
        assert!(p.iter().any(|p| p.contains("method \"post\"")), "{p:?}");
        assert!(p.iter().any(|p| p.contains("hash route")), "{p:?}");
        assert!(p.iter().any(|p| p.contains("tone \"red\"")), "{p:?}");
    }

    #[test]
    fn drift_names_invented_fields_and_ignores_omissions() {
        let mut v = example_json();
        v["relations"] = json!([]);
        v["link"] = Value::Null;
        let (raw, feed) = parse(&serde_json::to_vec(&json!([v])).unwrap()).unwrap();
        assert!(drift(&raw, &feed).is_empty());
        let mut v = example_json();
        v["metrics"][0]["colour"] = json!("red");
        let (raw, feed) = parse(&serde_json::to_vec(&json!([v])).unwrap()).unwrap();
        assert_eq!(drift(&raw, &feed), ["process:web: .metrics[0].colour is not in the contract"]);
    }

    #[test]
    fn parse_names_the_summary_that_fails() {
        let e = parse(br#"[{"id":"x","kind":"k","label":"l","health":"great","detail":""}]"#).unwrap_err();
        assert!(e.contains("\"x\""), "{e}");
        assert!(parse(b"{}").is_err());
    }
}
