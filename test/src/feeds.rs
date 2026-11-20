//! The contract on the node: the components feeds its daemons serve, read
//! with this commit's types. This is where a stormview change meets the
//! daemons already running — a field renamed here, or one a daemon invented
//! on its own, shows up as a feed that no longer reads, or no longer reads
//! back the same.

use crate::env::FeedAddr;
use crate::http::{self, Ws};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
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
            // Nothing there: try the next candidate. Anything that connected
            // and then broke (malformed HTTP, a truncated body, a read
            // timeout) is a live feed that is broken, not an absent one.
            Err(e @ http::Error::Connect(_)) => tried.push(format!("{addr}: {e}")),
            Err(e @ http::Error::Protocol(_)) => return Probe::Bad(format!("{addr}{PATH}: {e}")),
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
/// same components as `GET`, value for value (read again if the first
/// comparison misses: the feed may change in between).
pub fn ws_matches(feed: &Feed, timeout: Duration) -> Result<WsOutcome, String> {
    let body = match http::ws_first_text(&feed.addr, WS_PATH, timeout).map_err(|e| e.to_string())? {
        Ws::Refused(s) => return Ok(WsOutcome::Refused(s)),
        Ws::Text(b) => b,
    };
    let (_, snap) = parse(&body).map_err(|e| format!("snapshot: {e}"))?;
    if compare(&snap, &feed.summaries).is_none() {
        return Ok(WsOutcome::Same(snap.len()));
    }
    let again = http::get(&feed.addr, PATH, timeout).map_err(|e| e.to_string())?;
    if again.status != 200 {
        return Err(format!("{}{PATH} answers {} on the second read", feed.addr, again.status));
    }
    let (_, now) = parse(&again.body)?;
    Ok(match compare(&snap, &now) {
        None => WsOutcome::Same(snap.len()),
        Some(d) => WsOutcome::Differs(d),
    })
}

/// How a snapshot differs from a GET: components only in one, then each
/// component whose summary differs, by id, with the first differing field.
/// `None` when they hold the same summaries (order aside).
pub fn compare(snap: &[ComponentSummary], get: &[ComponentSummary]) -> Option<String> {
    let keyed = |f: &[ComponentSummary]| {
        f.iter().map(|c| (c.id.clone(), serde_json::to_value(c).unwrap_or(Value::Null))).collect::<BTreeMap<_, _>>()
    };
    let (ws, want) = (keyed(snap), keyed(get));
    if ws == want {
        return None;
    }
    let mut out = Vec::new();
    let only_ws: Vec<_> = ws.keys().filter(|k| !want.contains_key(*k)).collect();
    let only_get: Vec<_> = want.keys().filter(|k| !ws.contains_key(*k)).collect();
    if !only_ws.is_empty() || !only_get.is_empty() {
        out.push(format!("only in the snapshot: {only_ws:?}; only in GET: {only_get:?}"));
    }
    for (id, v) in &ws {
        if let Some(w) = want.get(id).filter(|w| *w != v) {
            out.push(format!("{id}: {}", diff_values(v, w, "")));
        }
    }
    Some(out.join("; "))
}

/// The first place a snapshot's summary and GET's differ, as a JSON path
/// with both values.
fn diff_values(ws: &Value, get: &Value, at: &str) -> String {
    match (ws, get) {
        (Value::Object(a), Value::Object(b)) => {
            if let Some(k) = a.keys().chain(b.keys()).find(|k| a.get(*k) != b.get(*k)) {
                let null = Value::Null;
                return diff_values(a.get(k).unwrap_or(&null), b.get(k).unwrap_or(&null), &format!("{at}.{k}"));
            }
            String::new()
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => a
            .iter()
            .zip(b)
            .enumerate()
            .find(|(_, (x, y))| x != y)
            .map(|(i, (x, y))| diff_values(x, y, &format!("{at}[{i}]")))
            .unwrap_or_default(),
        _ => format!("{} is {ws} in the snapshot, {get} in GET", if at.is_empty() { "." } else { at }),
    }
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

    use std::io::{Read, Write};
    use std::net::TcpListener;

    const T: Duration = Duration::from_secs(5);

    /// A local server: each connection's request head is handed to `answer`,
    /// whose bytes are written back before the connection closes.
    fn serve(answer: impl Fn(&str) -> Vec<u8> + Send + 'static) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            for s in l.incoming() {
                let Ok(mut s) = s else { continue };
                let mut req = Vec::new();
                let mut b = [0u8; 1];
                while !req.ends_with(b"\r\n\r\n") && s.read(&mut b).unwrap_or(0) == 1 {
                    req.push(b[0]);
                }
                let _ = s.write_all(&answer(&String::from_utf8_lossy(&req)));
            }
        });
        addr
    }

    /// An address nothing listens on.
    fn dead() -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().to_string()
    }

    fn ok(body: &[u8]) -> Vec<u8> {
        let mut r = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len()).into_bytes();
        r.extend(body);
        r
    }

    fn feed_addr(candidates: &[&str]) -> FeedAddr {
        FeedAddr { name: "t".into(), candidates: candidates.iter().map(|c| c.to_string()).collect() }
    }

    fn one(c: &ComponentSummary) -> Vec<u8> {
        serde_json::to_vec(&[c]).unwrap()
    }

    // 087732050848: only a failed connect is an absent candidate.
    #[test]
    fn a_connect_failure_is_absent_and_falls_through() {
        let gone = dead();
        assert!(matches!(probe(&feed_addr(&[&gone]), T), Probe::Absent(_)));
        let body = one(&example());
        let live = serve(move |_| ok(&body));
        match probe(&feed_addr(&[&gone, &live]), T) {
            Probe::Found(f) => assert_eq!(f.addr, live),
            _ => panic!("the second candidate answers"),
        }
    }

    #[test]
    fn malformed_http_is_bad_not_absent() {
        let a = serve(|_| b"garbage\r\n".to_vec());
        match probe(&feed_addr(&[&a, &dead()]), T) {
            Probe::Bad(d) => assert!(d.contains("not an HTTP response"), "{d}"),
            _ => panic!("a feed that answers garbage is broken, not absent"),
        }
    }

    #[test]
    fn a_truncated_body_is_bad_not_absent() {
        let a = serve(|_| b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n[]".to_vec());
        assert!(matches!(probe(&feed_addr(&[&a]), T), Probe::Bad(_)));
    }

    #[test]
    fn a_read_timeout_is_bad_not_absent() {
        let a = serve(|_| {
            std::thread::sleep(Duration::from_millis(1500));
            Vec::new()
        });
        match probe(&feed_addr(&[&a]), Duration::from_millis(300)) {
            Probe::Bad(_) => {}
            _ => panic!("a feed that connects and never answers is broken"),
        }
    }

    // 136367370430: the snapshot is compared value for value, by id.
    fn ws_frame(body: &[u8]) -> Vec<u8> {
        let mut f = vec![0x81];
        if body.len() < 126 {
            f.push(body.len() as u8);
        } else {
            f.push(126);
            f.extend((body.len() as u16).to_be_bytes());
        }
        f.extend(body);
        f
    }

    /// A feed whose websocket pushes `snap` and whose GET serves `get`.
    fn ws_feed(snap: Vec<u8>, get: Vec<u8>) -> String {
        serve(move |req| {
            if req.to_ascii_lowercase().contains("upgrade: websocket") {
                let mut r = b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
                    Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n"
                    .to_vec();
                r.extend(ws_frame(&snap));
                r
            } else {
                ok(&get)
            }
        })
    }

    fn feed_at(addr: String, summaries: Vec<ComponentSummary>) -> Feed {
        Feed { name: "t".into(), addr, raw: Value::Null, summaries }
    }

    #[test]
    fn a_snapshot_with_stale_values_differs() {
        let get = example();
        let mut stale = get.clone();
        stale.health = stormview::Health::Warn;
        stale.metrics[0].value = "3".into();
        let addr = ws_feed(one(&stale), one(&get));
        match ws_matches(&feed_at(addr, vec![get]), T).unwrap() {
            WsOutcome::Differs(d) => assert!(d.contains("process:web: .health is \"warn\" in the snapshot, \"ok\" in GET"), "{d}"),
            _ => panic!("same ids, different health: not the same feed"),
        }
    }

    #[test]
    fn a_snapshot_matching_the_second_get_is_same() {
        let first = example();
        let mut now = first.clone();
        now.detail = "changed between reads".into();
        let addr = ws_feed(one(&now), one(&now));
        assert!(matches!(ws_matches(&feed_at(addr, vec![first]), T).unwrap(), WsOutcome::Same(1)));
    }

    #[test]
    fn compare_ignores_order_and_names_every_difference() {
        let a = example();
        let mut b = a.clone();
        b.id = "system".into();
        assert_eq!(compare(&[a.clone(), b.clone()], &[b.clone(), a.clone()]), None);
        let mut a2 = a.clone();
        a2.actions[0].enabled = !a2.actions[0].enabled;
        let d = compare(&[a2, b], &[a]).unwrap();
        assert!(d.contains("only in the snapshot: [\"system\"]"), "{d}");
        assert!(d.contains("process:web: .actions[0].enabled"), "{d}");
    }

    #[test]
    fn parse_names_the_summary_that_fails() {
        let e = parse(br#"[{"id":"x","kind":"k","label":"l","health":"great","detail":""}]"#).unwrap_err();
        assert!(e.contains("\"x\""), "{e}");
        assert!(parse(b"{}").is_err());
    }
}
