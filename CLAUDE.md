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
- `docs/presentation.md` — Marp slide deck (purpose and functionality).
- Docs last refreshed from the code 2026-09-27.
- `web/components/*.svelte` — DataGrid, ComponentCard, ComponentGrid,
  RelationPicker, HealthDot, LoginPanel.
- `test/` — the stormcos test container (own cargo workspace; `build.sh`,
  `Containerfile`, `stormview-test.yaml`, `src/`: contract checks, feed
  reader, HTTP/websocket client, short/medium/long suites). sc-build it
  with `sc-build 'cargo test && cargo test --manifest-path test/Cargo.toml && test/build.sh'`.

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

### In progress
- [ ] #8 test container — built (`test/`, 4c4320f): 14 unit tests and
      the static musl `test/build.sh` pass on sc-build. **Left:** a real
      `stormcentral test run stormview short|medium` on a test machine.
      Blocked: run e993dc0dc0 errored before the image stage (C2NR0Q2's
      apiserver never answered /readyz). 2026-09-27: C2NR0Q2 on 11.50
      now answers /readyz, but its sbregistry (:5100) refuses connections
      (runs 43e9193e15, 2146f21771, dcae784bb9; noted on stormcentral#63),
      stormcentral#56 closed 2026-09-28. Rerun `short` and `medium` once
      stormcos#135 is fixed; close #8 on a passing run.

- Session state 2026-10-06: #11 done and released (v0.5.0). Next: #8
  rerun when stormcos#135 is fixed; else #4 (P2).

### Queued
- #10 Decide (owner): storage view — stormview reads storage.storm.io, or
  a daemon serves it as a feed; #9 waits on it.
- #4 ComponentCard ignores `Action.tone`; DataGrid renders only ok/warn.
- #5 JS `formatDuration` doesn't clamp negatives like Rust.
- #6 HealthDot glow colours hardcoded, not tokens.
- #1 LoginPanel: TOTP step and first-time enrolment.
