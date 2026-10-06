# Changelog

## [Unreleased]

### 2026-10-06
- **chore:** test-fixture credentials marked `not a secret` (inline, or `.github/secret_scanning.yml` for files that cannot hold a comment) — owner
- **fix:** `package.json`'s root export carries a `svelte` condition
  (`".": { "svelte": "./web/index.js", "default": "./web/index.js" }`), so
  vite-plugin-svelte no longer warns about a `svelte` field without one (#14).
- **test:** `web/test/check.sh` also builds a minimal host app
  (`web/test/host/`) with vite 6 + vite-plugin-svelte 5 against the packed
  package, failing on any vite-plugin-svelte warning (#14).
- **fix:** `Action.tone` renders everywhere: ComponentCard colours action
  buttons from `danger`, then `tone`, falling back to the action id
  (start/restart) only without a tone; DataGrid paints all four tones, not
  just ok/warn. `button.accent` and `button.muted` join themes.css; new
  `actionTone(action)` helper in `stormview/utils` (#4).
- **feat:** `LoginPanel` has an optional authenticator (TOTP) step and
  first-time enrolment: `onsubmit` may resolve to `{ step: 'totp' }` or
  `{ step: 'enroll', qr, secret }`, the code goes to the new
  `oncode(code, step)` prop; 6-digit auto-submit, QR with a show-the-key
  toggle, `← back`, and `restart: true` errors return to the password step (#1).

## [v0.5.0] — 2026-10-06

### Added
- `NODE_FEEDS` / `NodeFeed`: the stormcos node feed layout — every port that
  serves `GET /api/v1/components` — as one list for every reader (#11).
- `Action.tone` (`ok | warn | accent | muted`, optional): how a renderer
  colours an action.
- The stormcos test container (`test/`: short, medium and long suites, #8).

### Fixed
- `Cargo.toml` version aligned with `package.json`.
- The test container reads every port of the layout (it read five, one
  mislabelled), probed in parallel (#11).

### Documentation
- README, module docs and CLAUDE.md rewritten from the code (#2); Marp
  deck (#3); README feed layout table complete (#16).

## [v0.4.0] — 2026-08-26

### Added
- Four themes: One (One Dark) and Gruvbox (Material) dark; Frost (cool
  nordic) and Paper (warm sepia) light — twelve total, dark-then-light in
  the picker.
- `LoginPanel` asks for user + password (`onsubmit(username, password)`);
  `askUsername={false}` keeps the password-only form.

### Changed
- Catppuccin's brand color is peach instead of mauve, and the login panel's
  gradient thread drops the purple midpoint — per review.

## [v0.3.0] — 2026-08-26

### Added
- `LoginPanel` — the sign-in screen as a reusable, token-driven component
  (glyph, gradient thread, focus ring, inline error with shake).
- Catppuccin Mocha and Rosé Pine themes — both palettes built for long
  sessions and low eye strain.
- `setDefaultTheme(id)`: a server-configured default that yields to the
  viewer's own persisted pick.

### Changed
- The Storm default palette is rebased on Tokyo Night: low-glare indigo
  ground and softened accents instead of Dracula neon on near-black, which
  read harsh on the eyes.

## [v0.2.0] — 2026-08-26

### Added
- The npm half: `stormview` is now also a Svelte 5 UI-system package —
  `themes.css` (all tokens + the six themes), `DataGrid`, `ComponentCard`,
  `ComponentGrid`, `RelationPicker`, `HealthDot`, theme state
  (`stormview/theme`), and shared helpers (`stormview/utils`: byte/duration
  formatting, ANSI→HTML). Components are app-agnostic: hosts inject
  `resolve`/`invoke`, navigation is plain hash hrefs. Moved from stormd's
  `web/src/lib`, which now consumes this package.

## [v0.1.0] — 2026-08-26

### Added
- Initial contract, extracted from stormd: `ComponentSummary`, `Health`,
  `Metric`, `Action`, `Relation`/`RelationKind` (`has_one`, `has_many`,
  `belongs_to`), symmetric serde, and the shared `format_duration` /
  `format_bytes` helpers.

## [Unreleased]
<!-- New unreleased changes go here; the dated entries below shipped in v0.5.0. -->

### 2026-10-06
- **feat:** `NODE_FEEDS` / `NodeFeed`: every port a stormcos node serves
  `GET /api/v1/components` on (25: stormd 9080, the control plane's stormds
  9081–9085, the services, each service golden's stormd on its port + 100,
  incl. stormimds 8269, stormrdp 9201, stormcluster 9202), one shared list
  for every reader (#11).
- **fix:** test container: the default feed list is `NODE_FEEDS`, one
  `feed:<name>` result per port, probed in parallel. It read five ports, one
  wrongly labelled (8269 as stormd), and never the per-service stormds or
  stormconsole (#11).
- **docs:** README layout table adds 8269, 9201, 9202 and the registry
  goldens' stormds (#16); stormcentral#56 no longer listed as a blocker.

### 2026-09-27
- **docs:** README: the test container's environment table, from
  `test/src/env.rs`. It covers the suite argument and `STORM_SUITE` (default `short`),
  `STORM_NODE`, `STORMVIEW_FEEDS` (a repeated name adds a fallback),
  `STORM_TIMEOUT` (120 / 1800 / 28800 s, and the long suite's wave length) and
  `STORM_RESULTS` (`/results`), and notes which of the Job's variables are unread.
  Everything else since 2026-09-18 was checked against `test/` and is
  accurate. No new gaps: the open ones are #4, #5, #6 and #11.
- **docs:** refreshed again from the code since 2026-09-18. README: the test
  container's exit codes (0 / 1 test failed / 2 infrastructure only) and summary
  line, and its sc-build command. README and deck: #8's machine run now also
  waits on stormcos#135 (C2NR0Q2's sbregistry). No new gaps found.
- **docs:** work plan: #8's machine run is now blocked on C2NR0Q2's sbregistry
  (:5100, connection refused) and on stormcentral#56. The /readyz wait has cleared.
- **docs:** refreshed from the code since 2026-09-18 (`Action.tone`, the
  test container). The consumer list was re-checked on 2026-09-27: stormconsole's fleet plugin now
  reads node feeds. The README gains the stormcos feed port layout (from
  stormconsole's `NODE_PORTS`), and the deck gains the test container, #9–#11
  and status. Filed #11: the test container's default feed list doesn't
  match that layout.
- **test:** the stormcos test container (#8), per stormcentral
  `docs/test-standard.md`: `test/` is its own cargo workspace building one
  static `FROM scratch` image, `/test short|medium|long`. stormview runs
  nothing on a node, so the suites test the contract of the commit under
  test in the pod (documented wire form, defaults, refusals, builders,
  formatting) and read the components feeds the node's daemons serve
  (stormd, stormdrive, stormstorage, stormipmi) with it: `short` — the
  wire form and every feed reads; `medium` — the whole contract, plus each
  feed's integrity, exact read-back and websocket snapshot; `long` —
  overnight waves of readers sized from the pod, trended for slowdown and
  residue. Read-only, no API, `requires: []`.

### 2026-09-24
- **docs:** `docs/presentation.md` — a 12-slide Marp deck on stormview's
  purpose, place in stormcos, contract, UI system, interfaces, shipping,
  planned work and status, every claim drawn from the code (#3).
- **docs:** README, CLAUDE.md and module docs rewritten from the code (#2):
  every field, prop and default as the source has it; the consumer list
  checked against stormd, stormdrive, stormstorage, stormipmi, stormconsole
  and stormcentral; how it ships (library from `main`, no golden); known
  gaps filed as #4, #5, #6.
- **fix:** `Cargo.toml` version was still 0.1.0 while `package.json` and the
  tags were at 0.4.0 — aligned to 0.4.0.

### 2026-09-20
- **feat:** `Action.tone` (`ok | warn | accent | muted`, optional) — what a
  control means, separate from `danger`, which also gates a confirm.
  `DataGrid` renders the `ok`/`warn` tones.

### 2026-08-26
- **docs:** Consumers updated: stormdrive v0.4.0 and stormstorage v0.2.0
  now serve the components feed (+/ws/components) with real actions
<!-- New unreleased changes go here -->
