//! The storm view contract.
//!
//! Every storm daemon that has something to show describes it in one shape —
//! the [`ComponentSummary`] — and every storm UI (stormd's web SPA, stormsh's
//! TUI tiles, stormconsole's aggregated fleet view) renders that shape
//! generically. Daemons serve it as a JSON array, by convention at
//! `GET /api/v1/components` (plus `/ws/components` snapshots); the endpoints
//! belong to the daemons, not to this crate. A subsystem that reports a
//! summary appears in every UI with no per-UI work, and the UIs cannot drift
//! apart because none of them owns the model.
//!
//! Components relate to each other with the ORM vocabulary — `has_one`,
//! `has_many`, `belongs_to` — as typed edges between component ids in the
//! same feed. A renderer can nest grids along `has_many` edges, follow
//! `belongs_to` upward, and offer "select from a relationship" pickers,
//! without knowing what the components are.
//!
//! Everything serializes symmetrically (Serialize + Deserialize), so the
//! same types work on whichever side of the wire a program sits.

use serde::{Deserialize, Serialize};

/// Component health, in the order a viewer sorts by: broken first.
/// Lowercase on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Health {
    Error,
    Warn,
    Ok,
    Idle,
    Unknown,
}

/// One headline number on a component's card. `tone` is a rendering hint
/// ("ok" | "warn" | "error" | "muted" | "accent"), not a semantic — health
/// lives on the component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metric {
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tone: Option<String>,
}

impl Metric {
    pub fn new(label: &str, value: impl Into<String>) -> Self {
        Self {
            label: label.to_string(),
            value: value.into(),
            unit: None,
            tone: None,
        }
    }

    pub fn unit(mut self, unit: &str) -> Self {
        self.unit = Some(unit.to_string());
        self
    }

    pub fn tone(mut self, tone: &str) -> Self {
        self.tone = Some(tone.to_string());
        self
    }
}

/// An operation a viewer may invoke on a component. The path is a real API
/// path, so a renderer needs no per-kind knowledge to wire a button.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub label: String,
    pub method: String,
    pub path: String,
    pub enabled: bool,
    pub danger: bool,
    /// How the control should read: `ok`, `warn`, `accent`, `muted`.
    ///
    /// The same vocabulary [`Metric::tone`] uses, and for the same reason —
    /// a renderer should be told what a thing *means* rather than what colour
    /// to paint. `danger` is the one tone that predates this and it stays as
    /// its own field, because it also gates a confirmation prompt: a tone is
    /// a suggestion about appearance, and that one is a behaviour.
    ///
    /// The case this arrived for: a "Make golden" button on a catalogue
    /// entry, which after the golden exists should say so at a glance
    /// instead of looking identical to every entry that has never been
    /// built. A label alone does not carry down a list of thirty.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tone: Option<String>,
}

impl Action {
    /// Set the tone. Chainable, like [`Metric::tone`].
    pub fn tone(mut self, tone: &str) -> Self {
        self.tone = Some(tone.to_string());
        self
    }
}

/// How one component relates to another.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    HasOne,
    HasMany,
    BelongsTo,
}

/// A named, typed edge to other components. `targets` are component ids from
/// the same feed; `href` optionally overrides where following the edge goes
/// (e.g. logs filtered to one process).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    pub name: String,
    pub kind: RelationKind,
    pub targets: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub href: Option<String>,
}

impl Relation {
    pub fn has_one(name: &str, target: impl Into<String>) -> Self {
        Self {
            name: name.to_string(),
            kind: RelationKind::HasOne,
            targets: vec![target.into()],
            href: None,
        }
    }

    pub fn has_many(name: &str, targets: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            kind: RelationKind::HasMany,
            targets,
            href: None,
        }
    }

    pub fn belongs_to(name: &str, target: impl Into<String>) -> Self {
        Self {
            name: name.to_string(),
            kind: RelationKind::BelongsTo,
            targets: vec![target.into()],
            href: None,
        }
    }

    pub fn href(mut self, href: String) -> Self {
        self.href = Some(href);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentSummary {
    /// Stable identity, e.g. "system", "process:web", "cron:backup".
    pub id: String,
    /// A short noun: "system", "process", "plugin", "cron", "storage",
    /// "logs", "updater" — and whatever other daemons add ("drive",
    /// "baremetalhost", …).
    /// Renderers treat it as a grouping label, not an enum.
    pub kind: String,
    pub label: String,
    pub health: Health,
    /// One human line: what a viewer would say this component is doing.
    pub detail: String,
    #[serde(default)]
    pub metrics: Vec<Metric>,
    #[serde(default)]
    pub actions: Vec<Action>,
    /// Typed edges to other components in the same feed.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub relations: Vec<Relation>,
    /// UI route within the serving app (hash route); a TUI ignores it.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub link: Option<String>,
}

// --- Where the feeds are on a stormcos node ---

/// One port on a stormcos node that serves `GET /api/v1/components`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeFeed {
    pub port: u16,
    /// The daemon the layout puts there; with `stormd`, the service whose
    /// golden that stormd supervises.
    pub service: &'static str,
    /// The feed is served by the service golden's own stormd (its API on the
    /// service port + 100, or the control plane's 9081–9085), not by the
    /// service itself.
    pub stormd: bool,
}

impl NodeFeed {
    /// A short name for the feed, safe in a test or metric name:
    /// `stormdrive`, `stormdrive.stormd`; the node's own stormd is `stormd`.
    pub fn name(&self) -> String {
        if self.stormd {
            format!("{}.stormd", self.service)
        } else {
            self.service.to_string()
        }
    }
}

const fn svc(port: u16, service: &'static str) -> NodeFeed {
    NodeFeed { port, service, stormd: false }
}

const fn sd(port: u16, service: &'static str) -> NodeFeed {
    NodeFeed { port, service, stormd: true }
}

/// Every port a stormcos node serves a components feed on, so the readers
/// (stormconsole's fleet view, stormview's test container) share one list.
/// From stormcos `deploy/build-goldens.sh` (`service_golden` puts each
/// service's stormd on its port + 100; the control plane's stormds are
/// 9081–9085) and the component registry for the goldens it doesn't build
/// (stormupdate, nfsop, nextnfs, minismbd). A worker runs fewer of these than
/// a control-plane node: a port that answers nothing is absent, not broken.
/// stormblock (9090), stormvm (9095) and sbregistry (5100) serve no feed.
pub const NODE_FEEDS: &[NodeFeed] = &[
    svc(9080, "stormd"),
    sd(9081, "fastetcd"),
    sd(9082, "kube-apiserver"),
    sd(9083, "kube-controller-manager"),
    sd(9084, "kube-scheduler"),
    sd(9085, "rustkube-node"),
    svc(9092, "stormdrive"),
    svc(9093, "stormstorage"),
    svc(9094, "stormconsole"),
    svc(9097, "stormipmi"),
    sd(180, "stormlb"),
    sd(8180, "nextnfs"),
    sd(8269, "stormimds"),
    sd(8545, "minismbd"),
    sd(9188, "stormupdate"),
    sd(9192, "stormdrive"),
    sd(9193, "stormstorage"),
    sd(9194, "stormconsole"),
    sd(9195, "stormvm"),
    sd(9196, "cadvisor"),
    sd(9197, "stormipmi"),
    sd(9198, "nfsop"),
    sd(9199, "vmcloud-image-operator"),
    sd(9201, "stormrdp"),
    sd(9202, "stormcluster"),
];

// --- Shared formatting, so every UI prints the same numbers the same way ---

/// Two largest units: `42s`, `1m 30s`, `1h 1m`, `1d 1h`. Negatives clamp to `0s`.
pub fn format_duration(secs: i64) -> String {
    let secs = secs.max(0);
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else if mins > 0 {
        format!("{}m {}s", mins, s)
    } else {
        format!("{}s", s)
    }
}

/// Binary (1024) steps, one decimal above bytes: `512 B`, `2.0 KB`, up to TB.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", value, UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_formats_by_magnitude() {
        assert_eq!(format_duration(42), "42s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3700), "1h 1m");
        assert_eq!(format_duration(90000), "1d 1h");
        assert_eq!(format_duration(-5), "0s");
    }

    #[test]
    fn bytes_format_by_magnitude() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn summary_roundtrips_through_json() {
        let c = ComponentSummary {
            id: "process:web".into(),
            kind: "process".into(),
            label: "web".into(),
            health: Health::Ok,
            detail: "running".into(),
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
            relations: vec![
                Relation::belongs_to("system", "system"),
                Relation::has_one("logs", "logs").href("#/logs?process=web".into()),
            ],
            link: Some("#/process/web".into()),
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: ComponentSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn node_feeds_are_distinct_and_feed_ports_only() {
        let mut ports: Vec<u16> = NODE_FEEDS.iter().map(|f| f.port).collect();
        ports.sort_unstable();
        ports.dedup();
        assert_eq!(ports.len(), NODE_FEEDS.len(), "duplicate port");
        let mut names: Vec<String> = NODE_FEEDS.iter().map(NodeFeed::name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), NODE_FEEDS.len(), "duplicate name");
        for p in [9090u16, 9095, 5100] {
            assert!(NODE_FEEDS.iter().all(|f| f.port != p), "{p} serves no feed");
        }
        // A service golden's stormd is its port + 100.
        for (svc_port, name) in [(9092, "stormdrive"), (9093, "stormstorage"), (9094, "stormconsole"), (9097, "stormipmi")] {
            let f = NODE_FEEDS.iter().find(|f| f.port == svc_port + 100).unwrap();
            assert_eq!((f.service, f.stormd), (name, true));
        }
        assert_eq!(NODE_FEEDS[0].name(), "stormd");
        assert_eq!(NODE_FEEDS.iter().find(|f| f.port == 8269).unwrap().name(), "stormimds.stormd");
    }
}
