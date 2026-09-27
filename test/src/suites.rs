//! `short` and `medium`. `long` is in `long.rs`.

use crate::contract;
use crate::env::Env;
use crate::feeds::{self, Feed, Probe, WsOutcome};
use crate::report::{Outcome, Report};
use std::time::{Duration, Instant};

const T: Duration = Duration::from_secs(5);

/// Look for every feed on the node, record one `feed:<name>` line each, and
/// return those that answered. If none did — not even behind a login — the
/// node side could not be tested, and that is an infrastructure result.
pub fn probe_all(env: &Env, r: &mut Report) -> Vec<Feed> {
    if env.feeds.is_empty() {
        r.record("feeds", Outcome::Infra("STORM_NODE is not set, and no STORMVIEW_FEEDS were given".into()), 0, None);
        return Vec::new();
    }
    let mut found = Vec::new();
    let mut answered = 0;
    let mut absent = Vec::new();
    for f in &env.feeds {
        let t = Instant::now();
        let (outcome, feed) = match feeds::probe(f, T) {
            Probe::Found(feed) => {
                answered += 1;
                let d = format!("{} summaries from {}{} read with this commit's contract", feed.summaries.len(), feed.addr, feeds::PATH);
                (Outcome::Pass(d), Some(feed))
            }
            Probe::Auth(d) => {
                answered += 1;
                (Outcome::Skip(d), None)
            }
            Probe::Bad(d) => {
                answered += 1;
                (Outcome::Fail(d), None)
            }
            Probe::Absent(d) => {
                absent.push(d.clone());
                (Outcome::Skip(format!("not served on this node ({d})")), None)
            }
        };
        r.record(&format!("feed:{}", f.name), outcome, t.elapsed().as_millis(), None);
        found.extend(feed);
    }
    if answered == 0 {
        r.record("feeds", Outcome::Infra(format!("no components feed answered on {}: {}", env.node, absent.join("; "))), 0, None);
    }
    found
}

/// Up and doing its main job: the contract of this commit writes and reads
/// its documented shape, and reads the feeds the node's daemons serve now.
pub fn short(env: &Env, r: &mut Report) {
    r.check("contract-wire", contract::wire);
    probe_all(env, r);
}

/// Every feature and failure path of the contract, and the node's feeds
/// checked as a renderer would rely on them.
pub fn medium(env: &Env, r: &mut Report) {
    r.check("contract-wire", contract::wire);
    r.check("contract-defaults", contract::defaults);
    r.check("contract-refusals", contract::refusals);
    r.check("contract-health", contract::health);
    r.check("contract-builders", contract::builders);
    r.check("format-duration", contract::format_duration_table);
    r.check("format-bytes", contract::format_bytes_table);
    for f in probe_all(env, r) {
        r.check(&format!("integrity:{}", f.name), || {
            let p = feeds::integrity(&f.summaries);
            if p.is_empty() {
                Ok(format!("{} summaries: ids unique, relations inside the feed, actions, tones and links well formed", f.summaries.len()))
            } else {
                Err(format!("{} problem(s): {}", p.len(), p.join("; ")))
            }
        });
        r.check(&format!("roundtrip:{}", f.name), || {
            let d = feeds::drift(&f.raw, &f.summaries);
            if d.is_empty() {
                Ok(format!("{} summaries read back exactly as sent", f.summaries.len()))
            } else {
                Err(format!("{} summaries don't read back as sent: {}", d.len(), d.join("; ")))
            }
        });
        r.run(&format!("ws:{}", f.name), || match feeds::ws_matches(&f, Duration::from_secs(10)) {
            Ok(WsOutcome::Same(n)) => Outcome::Pass(format!("the first {} snapshot reads, and holds the same {n} components as GET", feeds::WS_PATH)),
            Ok(WsOutcome::Refused(s)) => Outcome::Skip(format!("{} answers {s}: no websocket feed here (it is optional)", feeds::WS_PATH)),
            Ok(WsOutcome::Differs(d)) => Outcome::Fail(d),
            Err(e) => Outcome::Fail(e),
        });
    }
}
