# CLAUDE.md — stormview

The storm view contract (Rust crate) and UI system (npm package, Svelte 5)
shared by every storm daemon and web UI. Cross-project rules live in
`~/src/CLAUDE.md`; this file is the project's own state.

## Version

- Current: **v0.4.0** (tag `v0.4.0`).
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

### In progress
- [ ] #8 test container — built (`test/`, 4c4320f): 14 unit tests and
      the static musl `test/build.sh` pass on sc-build. **Left:** a real
      `stormcentral test run stormview short|medium` on a test machine.
      Blocked: run e993dc0dc0 errored before the image stage (C2NR0Q2's
      apiserver never answered /readyz), and every run currently dies on
      stormcentral#56 (`@@RESULT` quoting). Rerun once #56 is fixed and a
      test machine is up; close #8 on a passing run.

### Queued
- #4 ComponentCard ignores `Action.tone`; DataGrid renders only ok/warn.
- #5 JS `formatDuration` doesn't clamp negatives like Rust.
- #6 HealthDot glow colours hardcoded, not tokens.
- #1 LoginPanel: TOTP step and first-time enrolment.
