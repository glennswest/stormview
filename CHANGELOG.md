# Changelog

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
