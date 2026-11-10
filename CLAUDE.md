# CLAUDE.md — stormview

The storm view contract (Rust crate) and UI system (npm package, Svelte 5)
shared by every storm daemon and web UI. Cross-project rules live in
`~/src/CLAUDE.md`; this file is the project's own state.

## Version

- Current: **v0.5.0** (tag `v0.5.0`).
- Version locations — all must match:
  - `Cargo.toml` → `version`
  - `package.json` → `version`
  - `CHANGELOG.md` → release heading, and the tag `vX.Y.Z`

## What ships, and how

- A library, not a service: no binary, no ports, no health or metrics
  endpoints, no golden, no stormcos component. Consumers pull it straight
  from GitHub `main` (Cargo git dependency; npm `github:glennswest/stormview#main`).
  A change here reaches a daemon when that daemon updates its lock and
  builds its own golden.
- Build/test: `sc-build` (runs `cargo build && cargo test` on dev.g8.lo
  from the pushed commit). The Svelte half has no build step here — hosts
  compile the source with their own Vite/svelte plugin.

## Layout

- `src/lib.rs` — the contract types + `format_duration` / `format_bytes`,
  with round-trip tests.
- `web/index.js` — theme + helper re-exports.
- `web/theme.svelte.js` — theme list and selection state.
- `web/themes.css` — tokens, 12 themes, base control styles.
- `web/utils.js` — JS formatting helpers and `ansiToHtml`.
- `docs/presentation.md` — Marp slide deck (purpose and functionality);
  `sc-build deploy/check-slides.sh` fails if a slide overflows (copied
  from stormcentral, #77).
- Docs last refreshed from the code 2026-09-27.
- `web/components/*.svelte` — DataGrid, ComponentCard, ComponentGrid,
  RelationPicker, HealthDot, LoginPanel.
- `web/test/` — the Svelte half's check: `sc-build web/test/check.sh`
  compiles every component (Svelte 5 compiler; any warning fails) and drives LoginPanel's
  steps under jsdom, then builds `web/test/host/` with vite 6 +
  vite-plugin-svelte 5 against the packed package (fails on plugin
  warnings); installs everything into `$TMPDIR` on the box.
- `test/` — a feed/contract checker from before the owner's 2026-10-06
  scope (library only, no test container); not run on test machines.

## Work plan

### Done
- [x] #2 docs: README / CLAUDE.md / module docs rewritten from the code;
      Cargo.toml version aligned with package.json (0.4.0); `Action.tone`
      changelogged (unreleased); doc/code gaps filed as #4, #5, #6.
- [x] #3 docs: Marp deck at `docs/presentation.md` (12 slides).
- [x] #11 `NODE_FEEDS` (stormcos node feed layout, 25 ports) in the
      crate; the test's default feed list is built from it, probed in
      parallel (884527f, sc-build passed); README table fixed (#16);
      stormconsole#90 filed to adopt it. Released v0.5.0.
- [x] #1 LoginPanel authenticator step + enrolment (`oncode(code, step)`);
      `web/test/check.sh` compiles every component and drives LoginPanel
      under jsdom (21 checks, sc-build passed). stormcentral#123 adopts it.
- [x] #4 action tones on ComponentCard + DataGrid via `actionTone(a)`
      (utils.js); id fallback on the card only; `button.accent`/`.muted`
      in themes.css; 16 tone checks in web/test/check.mjs (1c2bc79).
- [x] #14 root export carries a `svelte` condition (e029d83); check.sh's
      vite 6 + vite-plugin-svelte 5 host build fails on plugin warnings —
      passes at dc33be1, and fails with the #14 warning when the old export
      is put back (negative control, 2026-11-07).
- [x] #6 HealthDot glow mixed from the dot's token (`color-mix`), no fixed
      colours (006cdae); 4 checks in web/test/check.mjs pass on sc-build and
      fail with the old rgba put back (negative control, 2026-11-07).
- [x] #5 JS `formatDuration` clamps negatives to `0s` like Rust (827effa);
      9 checks in web/test/check.mjs pass on sc-build, and -5/-90 fail
      with the clamp removed (negative control, 2026-11-07).
- [x] #15 DataGrid action buttons stop the row click themselves; the
      actions span has no handler (no a11y warnings). check.mjs now fails on
      any Svelte compiler warning — failed on the old span (2 warnings,
      negative control), passes at the fix, 2026-11-07.
- [x] #12 DataGrid select-all scoped to its own grid's rows (adds/removes
      only them; header checked when all of them are, indeterminate when
      some); 15 checks in web/test/check.mjs pass on sc-build at bd9854f and
      9 fail with the old code put back (negative control, 2026-11-10).
      stormdrive#41 can drop its workaround once it takes this commit.
- [x] #13 DataGrid: a section's `getChildren: null` is a leaf grid (no
      expander column); absent/undefined inherits the parent's. 4 checks in
      web/test/check.mjs pass on sc-build at b4462ea; the leaf check fails
      with the old `??` put back (negative control, 2026-11-10).

### In progress
- [x] #8 closed by the owner 2026-10-06: stormview is a library, exempt
      from the test-container rule; sc-build's tests are its tests.

- Session state 2026-10-06: #11 done and released (v0.5.0). #1 and #4 done (unreleased); next #5/#6 (P3).

### In progress (#17)
- [x] #17 docs: stormipmi feed is `kind: machine`; stormcluster added as a
      consumer; presentation's stale stormcentral#56 / test-container status
      replaced with the owner's 2026-10-06 scope (library only, no test
      container, #8 closed).

### In progress (#22)
- [ ] #22 LoginPanel `remember` prop (`null` | `{ label, checked }`): a
      checkbox on the totp/enroll steps (min-height 44px), passed as
      `oncode(code, step, remembered)` (`undefined` when unset); checks in
      web/test/check.mjs; README. stormcentral#123 then drops its copy.

### Queued
- #10 Decide (owner): storage view — stormview reads storage.storm.io, or
  a daemon serves it as a feed; #9 waits on it. 2026-10-06: both labelled
  `needs-owner`, #9 parked with `stormcentral wait-owner`; option 1 also
  waits on rustkube#83 (open). Do not start #9 until #10 is answered.
