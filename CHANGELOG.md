# Changelog

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
<!-- New unreleased changes go here -->
