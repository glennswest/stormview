//! The storm view contract.
//!
//! Every storm daemon that has something to show describes it in one shape —
//! the [`ComponentSummary`] — and every storm UI (stormd's web SPA, stormsh's
//! TUI tiles, and later stormdrive and stormconsole) renders that shape
//! generically. A subsystem that reports a summary appears in every UI with
//! no per-UI work, and the UIs cannot drift apart because none of them owns
//! the model.
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
    /// "logs", "updater" — and whatever stormdrive and stormconsole add.
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

// --- Shared formatting, so every UI prints the same numbers the same way ---

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
}
