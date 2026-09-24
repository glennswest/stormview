---
marp: true
theme: default
paginate: true
title: stormview
description: The storm view contract and UI system — purpose and functionality
---

<!--
Render: npx @marp-team/marp-cli docs/presentation.md          (HTML)
        npx @marp-team/marp-cli --pdf docs/presentation.md    (PDF)
Every claim here is checkable against the code as of v0.4.0 + Unreleased
(src/lib.rs, web/). See README.md for the full reference.
-->

# stormview

**One shape for every storm component, one UI kit to render it**

The storm view contract (Rust crate) and UI system (npm package, Svelte 5)

v0.4.0 · github.com/glennswest/stormview

---

## What it is, and the problem it solves

Every storm daemon has things to show — processes, drives, pools, hosts —
and several UIs want to show them: stormd's web app, stormsh's TUI,
stormconsole's fleet view.

Without a shared model each UI learns each daemon, and they drift apart.

**stormview is that shared model:** a daemon describes each thing as a
`ComponentSummary`; every UI renders summaries generically.
A new subsystem that reports summaries shows up everywhere, with no
per-UI work — and no UI owns the model, so none can drift.

---

## Where it sits in stormcos

Group **ui**. Depends on no other component (only `serde`).
Depended on by (stormcentral's relationships graph):

| Component | Group | Uses |
|---|---|---|
| stormd (+ stormsh) | tooling | crate: serves the feed; TUI renders it · npm: reference web app |
| stormdrive | storage | crate: drives and shelves feed |
| stormstorage | storage | crate: nodes, pools, volumes feed |
| stormipmi | node | crate: bare-metal hosts feed |
| stormconsole | ui | crate: aggregates feeds · npm: web console |
| stormcentral | ui | npm: themes, `HealthDot`, theme picker |

---

## How it works

```
  daemon (stormd, stormdrive, stormstorage, stormipmi)
    builds Vec<stormview::ComponentSummary>
    serves  GET /api/v1/components   (+ /ws/components snapshots)
         │  JSON — the same serde types on both sides
         ▼
  ┌───────────────────────┬─────────────────────────────────┐
  │ Rust UIs              │ web UIs (npm stormview)          │
  │ stormsh TUI tiles     │ ComponentCard / ComponentGrid    │
  │ stormconsole          │   └ DataGrid, RelationPicker,    │
  │  aggregator: re-      │     HealthDot                    │
  │  prefixes ids,        │ themes.css + stormview/theme     │
  │  proxies actions      │ host injects resolve / invoke    │
  └───────────────────────┴─────────────────────────────────┘
```

The endpoints belong to the daemons; stormview supplies the shape and
the renderers.

---

## The contract (Rust crate)

```json
{ "id": "process:web", "kind": "process", "label": "web",
  "health": "ok", "detail": "running · pid 1234 · up 2h 3m",
  "metrics":   [{ "label": "restarts", "value": "0", "tone": "muted" }],
  "actions":   [{ "id": "stop", "label": "Stop", "method": "POST",
                  "path": "/api/v1/processes/web/stop",
                  "enabled": true, "danger": true }],
  "relations": [{ "name": "system", "kind": "belongs_to", "targets": ["system"] }],
  "link": "#/process/web" }
```

- **health** `error | warn | ok | idle | unknown` — broken sorts first
- **kind** a grouping noun, never an enum — new daemons add kinds freely
- **actions** carry the real method + path: a button needs no per-kind code
- **relations** `has_one | has_many | belongs_to` between ids in the feed
- Symmetric serde, round-trip tested; `format_duration` / `format_bytes`

---

## What it does today — the contract

- `ComponentSummary`, `Health`, `Metric`, `Action`, `Relation`,
  `RelationKind`, all `Serialize + Deserialize`
- Builders: `Metric::new().unit().tone()`, `Relation::has_one / has_many /
  belongs_to().href()`, `Action.tone()`
- Optional fields stay off the wire when absent (`unit`, `tone`, `href`,
  `link`, empty `relations`)
- `Action.danger` — confirm before invoking; `Action.tone`
  (`ok | warn | accent | muted`, Unreleased) — what a control means
- `format_duration` (`1h 1m`, clamps negatives) and `format_bytes`
  (`2.0 KB`, 1024 steps) so every UI prints numbers alike
- Tests: helper formatting and a full-summary JSON round-trip

---

## What it does today — the UI system

- **DataGrid** — injected columns/rows; sort; single/multi selection;
  nested child grids via `getChildren`; cell renderers `text | mono |
  health | metrics | actions | fn`
- **ComponentCard** — any summary as a card: health, metrics with tones,
  relation chips, `has_many` pickers, actions (danger confirms)
- **ComponentGrid** — the feed as a relational grid: roots = no
  `belongs_to` in feed, nests along `has_one`/`has_many` (cycle-safe),
  bulk start/stop/restart over a multi-selection
- **RelationPicker**, **HealthDot**, **LoginPanel** (user + password or
  password-only, inline error)
- App-agnostic: no router, no API client — hosts pass `resolve` / `invoke`

---

## What it does today — themes and helpers

- `themes.css`: every design token on `:root`; each theme is a
  `[data-theme]` block of overrides — a new theme is a new block
- **12 themes** — dark: Storm (Tokyo Night, default), One, Gruvbox,
  Catppuccin, Rosé, Midnight, Nord, Solar, Phosphor · light: Light,
  Frost, Paper
- `stormview/theme`: viewer's pick (localStorage `storm-theme`) beats
  the server default (`setDefaultTheme`) beats Storm
- `stormview/utils`: `formatBytes`, `formatDuration`, `timeAgo`,
  `escapeHtml`, `ansiToHtml` (SGR colours via `--ansi-*` tokens, so
  process output re-colours with the theme)

---

## Interfaces

| | |
|---|---|
| **Rust** | `stormview = { git = "https://github.com/glennswest/stormview", branch = "main" }` |
| **npm** | `"stormview": "github:glennswest/stormview#main"`; exports `.`, `./themes.css`, `./theme`, `./utils`, `./components/*` |
| **Peer dep** | `svelte ^5` — components ship as source, the host compiles |
| **Feed convention** | `GET /api/v1/components`, `/ws/components` — served by daemons |
| **Ports / CLI / config** | none — it is a library |
| **Health / metrics** | none of its own; it *defines* the health vocabulary |

---

## How it ships and is operated

- **No golden, not a stormcos component**, nothing to start or run
- Consumers track GitHub `main`: a change reaches a running system when a
  consumer updates its lock (`cargo update -p stormview` / npm reinstall),
  rebuilds, and ships **its own** golden
- Build and test: `sc-build` on dev.g8.lo (`cargo build && cargo test`);
  the Svelte half is exercised by the host apps' builds
- Releases are tags `vX.Y.Z` with matching `Cargo.toml` / `package.json`
  versions and a CHANGELOG entry

---

## Planned (not in the code yet)

- **#1** LoginPanel: authenticator (TOTP) step and first-time enrolment
- **#4** ComponentCard honours `Action.tone`; DataGrid renders all four
  tones (today: card colours by action id, grid only `ok`/`warn`)
- **#5** JS `formatDuration` clamps negatives like the Rust one
- **#6** HealthDot's glow from theme tokens instead of fixed colours

---

## Status

- **v0.4.0** tagged; `Action.tone` merged, unreleased
- Contract stable in practice: six repos build against it, nothing
  exhaust-matches `kind`, fields are added optional
- Docs refreshed from the code (#2): README, CLAUDE.md, module docs
- Open issues that matter: #4 (tone rendering — stormconsole's vmimages
  catalogue already sets `tone: "warn"` on its golden "Retry" action),
  #1 (TOTP login for every web UI)
