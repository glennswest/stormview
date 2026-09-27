//! What the runner hands the container (stormcentral `docs/test-standard.md`),
//! and which feeds to look for on the node.

use std::path::PathBuf;
use std::time::{Duration, Instant};

/// One daemon that may serve a components feed, and the `host:port`s to try
/// for it, first that answers wins.
#[derive(Clone, Debug, PartialEq)]
pub struct FeedAddr {
    pub name: String,
    pub candidates: Vec<String>,
}

/// The storm daemons that serve `GET /api/v1/components`, and the ports they
/// listen on by default. stormd is on 8269 on a stormcos node and 9080 by its
/// own default.
const DAEMONS: &[(&str, &[u16])] = &[
    ("stormd", &[8269, 9080]),
    ("stormdrive", &[9092]),
    ("stormstorage", &[9093]),
    ("stormipmi", &[9097]),
];

pub struct Env {
    pub suite: String,
    pub node: String,
    pub feeds: Vec<FeedAddr>,
    pub results: PathBuf,
    started: Instant,
    timeout: Duration,
}

impl Env {
    pub fn read() -> Env {
        let var = |k: &str| std::env::var(k).unwrap_or_default();
        // The runner passes the suite as the argument and in STORM_SUITE.
        let suite = std::env::args()
            .nth(1)
            .filter(|s| !s.is_empty())
            .or_else(|| Some(var("STORM_SUITE")).filter(|s| !s.is_empty()))
            .unwrap_or_else(|| "short".to_string());
        let default_timeout = match suite.as_str() {
            "short" => 120,
            "medium" => 1800,
            _ => 8 * 3600,
        };
        let node = var("STORM_NODE");
        let feeds = match std::env::var("STORMVIEW_FEEDS") {
            Ok(list) if !list.trim().is_empty() => parse_feeds(&list),
            _ => default_feeds(&node),
        };
        Env {
            timeout: Duration::from_secs(var("STORM_TIMEOUT").parse().unwrap_or(default_timeout)),
            suite,
            node,
            feeds,
            results: std::env::var_os("STORM_RESULTS").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/results")),
            started: Instant::now(),
        }
    }

    /// Time left of `STORM_TIMEOUT`.
    pub fn remaining(&self) -> Duration {
        self.timeout.saturating_sub(self.started.elapsed())
    }
}

/// `host:port`, bracketing a bare IPv6 address.
pub fn join(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

pub fn default_feeds(node: &str) -> Vec<FeedAddr> {
    if node.is_empty() {
        return Vec::new();
    }
    DAEMONS
        .iter()
        .map(|(name, ports)| FeedAddr { name: name.to_string(), candidates: ports.iter().map(|p| join(node, *p)).collect() })
        .collect()
}

/// `STORMVIEW_FEEDS=name=host:port,name=host:port` — for a hand run, or a
/// node whose daemons listen elsewhere. A repeated name adds a candidate.
pub fn parse_feeds(list: &str) -> Vec<FeedAddr> {
    let mut out: Vec<FeedAddr> = Vec::new();
    for item in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (name, addr) = item.split_once('=').unwrap_or((item, item));
        match out.iter_mut().find(|f| f.name == name) {
            Some(f) => f.candidates.push(addr.to_string()),
            None => out.push(FeedAddr { name: name.to_string(), candidates: vec![addr.to_string()] }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_the_node() {
        let f = default_feeds("10.0.0.5");
        assert_eq!(f[0].candidates, ["10.0.0.5:8269", "10.0.0.5:9080"]);
        assert_eq!(f.len(), DAEMONS.len());
        assert_eq!(default_feeds("fd00::5")[1].candidates, ["[fd00::5]:9092"]);
        assert!(default_feeds("").is_empty());
    }

    #[test]
    fn overrides_parse() {
        let f = parse_feeds("stormd=a:1, stormd=a:2,x=b:3,");
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].candidates, ["a:1", "a:2"]);
        assert_eq!(f[1], FeedAddr { name: "x".into(), candidates: vec!["b:3".into()] });
    }
}
