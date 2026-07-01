# Changelog

All notable changes to Quartz Launcher are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [5.2.10] - 2026-07-01

### Fixed
- Instance card return type after adding double-click handler
- Menu item title move/borrow error

## [5.2.9] - 2026-07-01

### Fixed
- CI compile errors: menu imports, spawn closures, borrow order, double-click trait
- Added `check.yml` workflow to catch compile errors on `main` before tagging releases

## [5.2.8] - 2026-07-01

### Fixed
- CI build: missing closing brace on `impl InstanceList` in instance card renderer

## [5.2.7] - 2026-07-01

### Fixed
- CI build syntax error in instance card renderer (mismatched delimiters)
- Stale `Cargo.lock` workspace versions causing `--frozen` build failures on CI

## [5.2.6] - 2026-07-01

### Added
- Quartz block theme — off-white crystal accents on a cool dark base (Prism/Modrinth-style dark launcher)
- Smooth page fade-in when switching tabs (async animation, no crash)
- Running instances section in the sidebar with live green indicator
- Instance card polish: running badge, double-click to play, quartz hover borders
- Breathing animation on the welcome-screen quartz logo
- Debounced instance search and full-width search bar
- Localized relative last-played times

### Changed
- Sidebar menu typography and hover states
- Instance cards use elevated muted panels with clearer version/loader layout

## [5.2.5] - 2026-07-01

### Fixed
- Quartz block logo showing a magenta missing-texture checkerboard (asset was WebP data saved as `.png`; replaced with a real 16×16 PNG and regenerated app icons)

## [5.2.4] - 2026-07-01

### Fixed
- Tab switching crash caused by page fade animation mutating UI state during render
- Red error toast when no update manifest is published (404 is now handled silently)
- Quartz block logo rendering for crisp pixel-art display in the sidebar and empty state
- Sidebar header text now shows **Quartz** instead of "Quartz Launcher"
- CI build failure in quartz logo component (embedded asset path and `Pixels` conversion)

### Removed
- Page fade transition on tab switch (stability fix)

## [5.2.3] - 2026-07-01

### Added
- Quartz branding theme with Minecraft font header
- Minecraft Block of Quartz logo and regenerated app icons
- Instance search, last-played sort/display, and welcome empty state
- Sidebar labels and locale strings for new UI features

### Fixed
- Styled text rendering (italic, underline, strikethrough)
- Theme cold-start on first launch
- CD release build errors in frontend and CI scripts

## [5.2.2] - 2026-07-01

### Added
- GitHub Releases with Windows installer, portable builds, and multi-platform artifacts
- CD workflow for tagged releases (Linux, Windows, macOS)
