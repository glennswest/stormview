# stormview

The storm view contract and UI system: one shape every storm daemon
describes itself in, and the shared pieces every storm UI renders it with.

This repo is two packages from one tree:

- **The Rust crate `stormview`** (`src/lib.rs`) — the contract types
  (`ComponentSummary`, `Health`, `Metric`, `Action`, `Relation`,
  `RelationKind`) and the shared `format_duration` / `format_bytes`
  helpers. Daemons build their components feed from these types; UIs
  written in Rust (stormsh, stormconsole's aggregator) deserialize the same
  types.
- **The npm package `stormview`** (`web/`) — Svelte 5 components, the theme
  tokens and twelve themes, theme-selection state, and JS helpers. Web UIs
  import it and render any feed without knowing what the components are.

A subsystem that reports a summary appears in every UI with no per-UI
work, and the UIs cannot drift apart because none of them owns the model.

A slide overview lives in [`docs/presentation.md`](docs/presentation.md)
(Marp: `npx @marp-team/marp-cli docs/presentation.md`).

## What it is not

stormview is a library. It has **no binary, no ports, no config file, no
health or metrics endpoints**, and it does not serve anything — the daemons
that depend on it do. It is not a stormcos component and produces no
golden; see [How it ships](#how-it-ships).

## The shape

A daemon serves a JSON array of component summaries. The convention every
current server follows is `GET /api/v1/components`, plus full-snapshot
pushes on `/ws/components` where the daemon has a websocket (the endpoints
belong to each daemon, not to this crate):

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

### `ComponentSummary`

| Field | Type | Required | Notes |
|---|---|---|---|
| `id` | string | yes | Stable identity, e.g. `system`, `process:web`, `cron:backup`. |
| `kind` | string | yes | A grouping noun, **not an enum**: renderers must not exhaust-match on it, so new daemons add kinds freely. |
| `label` | string | yes | Display name. |
| `health` | `Health` | yes | See below. |
| `detail` | string | yes | One human line: what the component is doing. |
| `metrics` | `Metric[]` | no — defaults to `[]` | Always serialized. |
| `actions` | `Action[]` | no — defaults to `[]` | Always serialized. |
| `relations` | `Relation[]` | no — defaults to `[]` | Omitted from the JSON when empty. |
| `link` | string | no | Hash route in the serving app's UI; a TUI ignores it. Omitted when absent. |

### `Health`

`error | warn | ok | idle | unknown` (lowercase on the wire), declared in
the order a viewer sorts by: broken first.

### `Metric`

`{label, value, unit?, tone?}` — a headline number on the card. `value` is
a preformatted string; `unit` is appended to it as-is. `tone` is a
rendering hint — `ok | warn | error | muted | accent` — not a semantic;
health lives on the component. Builder: `Metric::new(label, value)
.unit("…").tone("…")`.

### `Action`

`{id, label, method, path, enabled, danger, tone?}` — an operation on the
component, with the **real** API method and path, so a renderer wires a
button with no per-kind knowledge.

- `enabled` gates the button.
- `danger` asks for confirmation before invoking (and paints the button
  in the error colours). It is its own field because it is a behaviour.
- `tone` (`ok | warn | accent | muted`, optional, omitted when absent) says
  what the control *means* — e.g. a "Make golden" button that already has
  a golden. It is a suggestion about appearance only. Builder:
  `action.tone("ok")`. Renderer support is partial today, see
  [Known gaps](#known-gaps).

There is no `Action::new`; build the struct literally.

### `Relation` / `RelationKind`

`{name, kind, targets, href?}` — a named, typed edge to other component
ids **in the same feed**, in the ORM vocabulary: `has_one`, `has_many`,
`belongs_to`. Renderers nest grids along `has_one`/`has_many`, follow
`belongs_to` upward, and offer "select from a relationship" pickers.
`href` overrides where following the edge goes (e.g. logs filtered to one
process). Builders: `Relation::has_one(name, id)`,
`Relation::has_many(name, ids)`, `Relation::belongs_to(name, id)`,
`.href(String)`.

### Serde

Every type derives `Serialize + Deserialize` symmetrically, so the same
types work on either side of the wire; `summary_roundtrips_through_json`
in `src/lib.rs` pins that. The only dependency is `serde` (derive);
there are no cargo features.

### Formatting helpers

Shared so every UI prints the same numbers the same way:

- `format_duration(secs: i64)` → `42s`, `1m 30s`, `1h 1m`, `1d 1h`
  (two largest units; negatives clamp to `0s`).
- `format_bytes(bytes: u64)` → `512 B`, `2.0 KB`, `5.0 MB` (binary 1024
  steps, one decimal, up to TB).

The JS side has the same two as `formatBytes` / `formatDuration`
(which also return `-` for `null`).

## Who serves and renders it

Checked against each repo's code on 2026-09-24.

**Rust crate** (`stormview = { git = "https://github.com/glennswest/stormview", branch = "main" }`):

| Component | Role |
|---|---|
| **stormd** | Assembles its feed (`crates/stormd/src/components.rs`), serves `GET /api/v1/components` and `/ws/components`. |
| **stormsh** (in the stormd repo) | Renders stormd's feed as TUI tiles. |
| **stormdrive** | Drives and shelves, with locate/fleet/test/designation actions; `GET /api/v1/components` + `/ws/components`. |
| **stormstorage** | The federation — nodes, pools, distributed volumes with legs as relations; `GET /api/v1/components` + `/ws/components`. |
| **stormipmi** | Its bare-metal hosts (`kind: baremetalhost`, power actions) on `GET /api/v1/components` (no websocket). |
| **stormconsole** | Aggregates the stormd, stormdrive and stormstorage feeds (ids re-prefixed, actions proxied through the console) and serves the result on `GET /api/v1/components` + `/ws/components`. |

**npm package** (`"stormview": "github:glennswest/stormview#main"`):
stormd's `web/` (the reference host app: routing, auth, stores, views),
stormconsole's `web/`, and stormcentral's `web/` (themes, `HealthDot`,
theme picker — stormcentral's own `/api/v1/components` is its project
list, not a stormview feed).

## The UI system (npm package)

App-agnostic and written against the contract only: no router, no API
client baked in. Components ship as **Svelte 5 source** — the host app's
Vite/svelte plugin compiles them; `svelte ^5` is a peer dependency. There
is no build step in this repo.

```json
"dependencies": {
  "stormview": "github:glennswest/stormview#main"
}
```

```js
import 'stormview/themes.css'                                     // tokens + 12 themes + base controls
import { initTheme, setDefaultTheme, applyTheme, THEMES, theme } from 'stormview/theme'
import { formatBytes, formatDuration, timeAgo, escapeHtml, ansiToHtml } from 'stormview/utils'
import DataGrid from 'stormview/components/DataGrid.svelte'
import ComponentCard from 'stormview/components/ComponentCard.svelte'
import ComponentGrid from 'stormview/components/ComponentGrid.svelte'
import RelationPicker from 'stormview/components/RelationPicker.svelte'
import HealthDot from 'stormview/components/HealthDot.svelte'
import LoginPanel from 'stormview/components/LoginPanel.svelte'
```

The package root (`import … from 'stormview'`) re-exports the theme API
and the utils; components are always imported by path.

### Themes — `themes.css` and `stormview/theme`

`themes.css` holds every design token on `:root` (surfaces, text, semantic
colours and their `-bg`/`-border` surfaces, `--brand`, `--accent`, the
`--ansi-*` palette, fonts, radii) and each other theme as a
`[data-theme='…']` block of token overrides — a new theme is a new block.
It also styles base controls (`button`, `button.ok|warn|danger`, `select`,
text inputs, links), the `.term-output` log pane, and `.content`.

Twelve themes, dark then light, as listed in `THEMES`:

| id | label | |
|---|---|---|
| `storm` | Storm | default — Tokyo Night based; it is the `:root` tokens, so no `data-theme` attribute is set |
| `one` | One | dark |
| `gruvbox` | Gruvbox | dark |
| `catppuccin` | Catppuccin | dark (Mocha) |
| `rose` | Rosé | dark (Rosé Pine) |
| `midnight` | Midnight | dark |
| `nord` | Nord | dark |
| `solar` | Solar | dark |
| `phosphor` | Phosphor | dark |
| `light` | Light | light |
| `frost` | Frost | light |
| `paper` | Paper | light |

Selection (`theme.svelte.js`) sets `data-theme` on `<html>`. Priority: the
viewer's own pick (localStorage key **`storm-theme`**, per browser) beats
the server's configured default, which beats `storm`.

- `initTheme()` — apply the current choice; call once at startup.
- `setDefaultTheme(id)` — the server's default; ignored if the viewer has
  a stored pick, never persisted.
- `applyTheme(id)` — the viewer picked; applied and persisted.
- `theme.current` — reactive (`$state`) current id, for pickers.
- Unknown ids are ignored everywhere.

### Components

**`DataGrid`** — the generic grid; everything is injected.

| Prop | Default | |
|---|---|---|
| `columns` | `[]` | `[{key, label, width?, render?, sortable?}]`; `render` is `'text'` (default), `'mono'`, `'health'`, `'metrics'`, `'actions'`, or `(row) => string` |
| `rows` | `[]` | objects with a stable `id`; cells read `row[key]` |
| `getChildren` | `null` | `(row) => [{title, rows, columns?, getChildren?}]`; non-empty ⇒ the row gets an expander and nested grids |
| `selectable` | `null` | `null`, `'single'` or `'multi'` (multi adds select-all) |
| `selected` | `[]` | bindable array of ids, shared with nested grids |
| `onaction` | `null` | `(row, action)` from `'actions'` cells |
| `onrowclick` | `null` | `(row)` |

Columns sort on header click (toggle direction) unless `sortable: false`.
Action buttons honour `enabled`, `danger`, and `tone` `ok`/`warn`.
DataGrid does not confirm `danger` actions itself — `onaction` decides.

**`ComponentCard`** — one `ComponentSummary` as a card: health dot, a kind
icon (known kinds: system, process, plugin, cron, storage, logs, updater;
anything else gets `•`), label (linked when `link` is set), detail,
metrics with tone colours, relations, actions.

| Prop | Default | |
|---|---|---|
| `component` | — | the summary |
| `resolve` | `() => undefined` | `(id) => ComponentSummary` from the host's feed |
| `invoke` | `null` | `async (action)`; default is `fetch(action.path, {method})` |

`has_one`/`belongs_to` relations render as chips (follow `href`, else the
first target's `link`); `has_many` as a `RelationPicker` plus a ⊞ link to
`#/grid?id=<id>&rel=<name>`. `danger` actions confirm first; buttons are
disabled while one runs.

**`ComponentGrid`** — the feed as a relational grid on top of `DataGrid`.

| Prop | Default | |
|---|---|---|
| `components` | `[]` | the whole feed |
| `rootIds` | `null` | explicit top rows; otherwise every component with no `belongs_to` edge to something present in the feed |
| `invoke` | `null` | as ComponentCard |

`has_one`/`has_many` edges expand into nested grids (ancestors excluded,
so cycles terminate). Rows are multi-selectable; with two or more selected,
a bulk bar offers **start / stop / restart** when every selected row has
that action id enabled, and confirms before running. Clicking a row
follows its `link`. The host implements the `#/grid` route itself.

**`RelationPicker`** — a `has_many` edge as a `<select>` labelled
`name (count)`. Props: `relation`, `resolve`, `onpick = null` (default:
follow the picked component's `link`).

**`HealthDot`** — props `health = 'unknown'`, `size = 10` (px); `error`
pulses.

**`LoginPanel`** — the sign-in screen, token-driven. Props:
`title = 'storm'`, `subtitle = 'sign in to continue'`,
`askUsername = true` (false ⇒ password-only form), and
`onsubmit(username, password)` — async; a thrown error's `message` shows
inline, the password clears, the panel shakes. There is no TOTP step yet
(#1).

### JS helpers — `stormview/utils`

`formatBytes`, `formatDuration` (see above), `timeAgo(ts)` (`42s ago` …
`3d ago`), `escapeHtml(s)`, and `ansiToHtml(text)` — SGR bold, dim, italic,
underline and the 16 foreground colours, coloured through the theme's
`--ansi-*` tokens so output re-colours with the theme; cursor-movement
escapes are stripped and all text is HTML-escaped.

## Building and testing

Never on the VM and never as root: push, then

```sh
sc-build            # cargo build && cargo test on dev.g8.lo, from the pushed commit
```

The tests cover the formatting helpers and a JSON round-trip of a full
summary. The Svelte half has no build or test step here; it is exercised
by the host apps' builds.

## How it ships

There is no golden and no stormcos component for stormview. Both halves are
consumed from GitHub `main`: the crate as a Cargo git dependency, the
package as `github:glennswest/stormview#main`. A change reaches a running
system when a consuming daemon updates its lock (`cargo update -p
stormview`, or reinstalling the npm dependency), rebuilds, and ships its own
golden. Releases here are tags (`vX.Y.Z`) with matching `Cargo.toml` and
`package.json` versions, for the changelog's sake.

## Known gaps

What the contract or docs promise that the renderers don't do yet:

- `ComponentCard` ignores `Action.tone` (colours by action id), and
  `DataGrid` renders only the `ok`/`warn` tones — #4.
- JS `formatDuration` prints negative durations where Rust clamps to
  `0s` — #5.
- `HealthDot`'s glow uses hardcoded colours, not theme tokens — #6.
