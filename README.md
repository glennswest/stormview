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

## The UI system (npm package)

The same repo is also the npm package `stormview` — the reusable web UI
system, written against this contract only and app-agnostic (no router, no
API client baked in):

```json
"dependencies": {
  "stormview": "git+ssh://git@github.com/glennswest/stormview.git#main"
}
```

```js
import 'stormview/themes.css'                                  // tokens + 6 themes
import { initTheme, THEMES, applyTheme } from 'stormview/theme' // theme picker state
import { ansiToHtml, formatBytes } from 'stormview/utils'       // shared helpers
import DataGrid from 'stormview/components/DataGrid.svelte'
import ComponentCard from 'stormview/components/ComponentCard.svelte'
import ComponentGrid from 'stormview/components/ComponentGrid.svelte'
import RelationPicker from 'stormview/components/RelationPicker.svelte'
import HealthDot from 'stormview/components/HealthDot.svelte'
```

- **`themes.css`** — every design token and eight themes (Storm — Tokyo
  Night-based default, Midnight, Catppuccin Mocha, Rosé Pine, Nord, Solar,
  Phosphor, Light) as `[data-theme]` token-override blocks. A new theme is
  a new block. ANSI output and charts read tokens too. A server-configured
  default is applied via `setDefaultTheme(id)` — it yields to a viewer's
  own pick, which `applyTheme(id)` persists per browser.
- **`DataGrid`** — the generic grid: injected columns/rows, sortable,
  single/multi selection, and nested child grids via `getChildren(row)`.
- **`ComponentCard` / `ComponentGrid`** — render `ComponentSummary` values:
  cards with metrics/relations/actions; the relational grid with nesting
  along `has_many`/`has_one`, bulk actions over the selection, and
  relation pickers. Host apps pass `resolve(id)` (feed lookup) and
  optionally `invoke(action)`; navigation is plain `#/…` hash hrefs.
- Components ship as Svelte 5 source — the host app's Vite/svelte plugin
  compiles them; `svelte` is a peer dependency.

stormd's `web/` is the reference host app (routing, auth, stores, views).
