# Changelog

All notable changes to Quartz Launcher are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
