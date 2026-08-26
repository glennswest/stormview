# stormview

The storm view contract: one shape every storm daemon describes itself in,
and every storm UI renders generically.

A daemon serves a list of **component summaries** — `{id, kind, label,
health, detail, metrics, actions, relations, link}` — at
`GET /api/v1/components`, with full-snapshot pushes on `/ws/components`.
UIs (stormd's embedded web SPA, stormsh's TUI tiles, and later stormdrive
and stormconsole) render the feed without knowing what the components are:
a subsystem that reports a summary appears in every UI with no per-UI work,
and the UIs cannot drift apart because none of them owns the model.

## The shape

```json
{
  "id": "process:web",
  "kind": "process",
  "label": "web",
  "health": "ok",
  "detail": "running · pid 1234 · up 2h 3m",
  "metrics": [
    { "label": "restarts", "value": "0", "tone": "muted" }
  ],
  "actions": [
    { "id": "stop", "label": "Stop", "method": "POST",
      "path": "/api/v1/processes/web/stop", "enabled": true, "danger": true }
  ],
  "relations": [
    { "name": "system", "kind": "belongs_to", "targets": ["system"] },
    { "name": "logs", "kind": "has_one", "targets": ["logs"],
      "href": "#/logs?process=web" }
  ],
  "link": "#/process/web"
}
```

- **health** — `error | warn | ok | idle | unknown`, ordered broken-first.
- **metrics** — headline numbers; `tone` (`ok | warn | error | muted |
  accent`) is a rendering hint, health lives on the component.
- **actions** — real API method + path, so a renderer wires a button with no
  per-kind knowledge. `enabled` gates it, `danger` asks for confirmation.
- **relations** — typed edges between component ids in the same feed, in the
  ORM vocabulary: `has_one`, `has_many`, `belongs_to`. Renderers nest grids
  along `has_many`, follow `belongs_to` upward, and offer "select from a
  relationship" pickers. `href` overrides where following the edge goes.
- **kind** — a grouping noun, not an enum: renderers must not exhaust-match
  on it, so new daemons can add kinds freely.

## Consumers

```toml
[dependencies]
stormview = { git = "https://github.com/glennswest/stormview", branch = "main" }
```

- **stormd** — assembles the feed (`components.rs`) and serves it.
- **stormsh** — renders it as TUI tiles.
- **stormdrive / stormconsole** — planned: drives, volumes, nodes and fleets
  are components with relations like any other.

The crate also carries `format_duration` / `format_bytes` so every UI prints
the same numbers the same way. Everything serializes symmetrically
(Serialize + Deserialize) — the same types work on either side of the wire.

The reference web renderer (cards, nested `DataGrid`, `RelationPicker`)
lives in stormd's `web/src/lib/` as Svelte 5 components, written against
this contract only, so they lift into any storm web UI.
