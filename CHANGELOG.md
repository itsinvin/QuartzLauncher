# Changelog

All notable changes to Quartz Launcher are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [5.2.38] - 2026-07-11

### Fixed
- Instance launch failing with "Failed to load game libraries: Failed to perform I/O operation" (path-not-found during library writes; now uses atomic writes, mkdir retries, and includes the failing path in the error)
- Missing `${arch}` substitution for native library classifiers on older Minecraft/Forge versions
- Natives extraction skipping files when zip entries lack parent directories
- Empty Modrinth icon URLs causing image asset load errors
- Recommended cards showing empty state while still loading
- Folder resource packs using all-zero SHA1 hashes (caused Modrinth update 404s)
- Forge processor argument expansion panicking on unknown placeholders
- Opaque instance/library I/O errors that hid the underlying OS message

### Changed
- Forge installer temp directory is now unique per install to avoid clashes

## [5.2.37] - 2026-07-02

### Changed
- Home page now shows Recommended modpacks instead of favorite/recommended mods
- Recommended mods, resource packs, and shaders appear at the bottom of each instance content tab

### Added
- Modpack recommendation algorithm based on your instances and installed mods/modpacks

## [5.2.36] - 2026-07-02

### Added
- Remove installed mods from Modrinth/CurseForge browse pages when adding to an instance
- Pre-launch conflict detection on Mods, Resource Packs, and Shaders tabs (duplicate IDs, projects, files, names, loader mismatches)
- Mixin conflict detection for Fabric mods (multiple mods targeting the same class)

## [5.2.35] - 2026-07-02

### Fixed
- Suppress modpack extraction log spam when launching (only runs for actual modpack files)
- Favorite star uses gray outline when unfavorited and yellow filled star when favorited

## [5.2.34] - 2026-07-02

### Added
- Separate Favorite mods and Recommended mods sections on the home page
- Mod recommendation algorithm based on your loader, Minecraft version, and installed mods
- Recent skins grid beside the home page player preview

## [5.2.33] - 2026-07-02

### Fixed
- CI release build failure (`gpui-component` vendoring lockfile conflict)

## [5.2.32] - 2026-07-02

### Added
- Discord Rich Presence (shows launcher status and active instance)
- Home page player skin preview with auto-rotate (controls removed)
- Yellow star icon when a mod is favorited on Modrinth/CurseForge

### Fixed
- Block instance names/paths containing `!` (fixes Fabric `client-intermediary.jar.tmp` launch failures)
- `UniqueBytes` reentrant lock compile error on CI
- Modrinth project descriptions render as plain text (avoids tree-sitter deadlocks)

## [5.2.31] - 2026-07-02

### Fixed
- v5.2.30 failed to build (broken `gpui-component` vendor patch); Modrinth descriptions now render as plain text instead of markdown to avoid tree-sitter deadlocks

## [5.2.30] - 2026-07-02

### Fixed
- Deadlock when viewing Modrinth project descriptions with fenced code blocks (tree-sitter JSON highlighting disabled for markdown code blocks; `UniqueBytes` interning uses a reentrant lock)

## [5.2.29] - 2026-07-02

### Fixed
- Missing `IntoElement` import in refresh icon animation helper

## [5.2.28] - 2026-07-02

### Fixed
- Refresh icon helper return type and favorite thumbnail lifetimes on Home page

## [5.2.27] - 2026-07-02

### Fixed
- Home page section builders now return `AnyElement` to satisfy the borrow checker

## [5.2.26] - 2026-07-02

### Fixed
- Home page borrow-checker errors in modpack and recommendation sections

## [5.2.25] - 2026-07-02

### Fixed
- Home page compile errors (imports, borrows, styling traits, and refresh button id)

## [5.2.24] - 2026-07-02

### Fixed
- Compile error in Home page module imports

## [5.2.23] - 2026-07-02

### Added
- **Home page** — quick play for your last modpack, player skin preview, library stats, modpack showcase, and favorite-mod recommendations
- **Refresh button animation** — content tabs spin the refresh icon when rescanning folders

## [5.2.22] - 2026-07-02

### Fixed
- CD workflow no longer fails the release when README auto-update cannot push to `main`

## [5.2.21] - 2026-07-02

### Fixed
- CI compile errors in modpack folder import, game output window handle, and import page labels

## [5.2.20] - 2026-07-02

### Added
- **Import modpack folder** — import Modrinth/CurseForge pack folders or extracted `.minecraft`-style directories as new instances
- **What's new** screen on startup after updates
- **Link manual mods** — connect manually installed mods to Modrinth or CurseForge for update checks
- **README build info** — auto-updated version, build time, and changelog highlights after every CI/release build

### Changed
- **Game output window** closes automatically when the game exits
- **Instance tabs** are cached after first visit (faster switching between Mods, Performance, etc.)
- **Hardware detection** is cached globally; Performance tabs reuse it until you click Refresh
- **Automatic launcher update checks** on startup and every 30 minutes

## [5.2.19] - 2026-07-02

### Fixed
- Backend compile errors for modpack install-on-extract (`ApplyModpackAffectedFolders`, `dot_minecraft_path`)

## [5.2.18] - 2026-07-02

### Fixed
- Compile error from missing `ImportPage` / `SyncingPage` imports in navigation

## [5.2.17] - 2026-07-02

### Added
- **Modpack extract installs to instance** — extracted mods, resource packs, shaders, and overrides go into their folders; the `.mrpack` file is removed afterward
- **Refresh button** on instance Mods, Resource Packs, and Shaders tabs to rescan for new files

### Changed
- **Import** section embedded at the bottom of the Instances page (sidebar Import item removed)
- **Syncing** moved into Settings as a Sync tab (sidebar Syncing item removed)

## [5.2.16] - 2026-07-02

### Fixed
- Backend compile error in modpack extraction handler (`InstanceID` move and `mark_content_dirty` argument)

## [5.2.15] - 2026-07-02

### Fixed
- Compile errors in modpack extraction log modal and instance performance tab (missing trait imports)

## [5.2.14] - 2026-07-02

### Added
- **Modpack extraction progress** — overall progress bar and scrollable log while extracting `.mrpack` contents (including per-child downloads)
- **Performance tab** on instances — estimates FPS, rating, and bottlenecks from installed mods, loader, RAM, and your hardware

## [5.2.13] - 2026-07-02

### Fixed
- Compile error comparing favorite project IDs (`String` vs `Arc<str>`)

## [5.2.12] - 2026-07-02

### Added
- **Extract** button on imported modpacks — download all pack contents without expanding the file list
- **Favorite mods** on Modrinth and CurseForge browse pages, with **Favorites only** sidebar filter
- Minecraft font applied globally with increased line height for readability

### Changed
- Modernized mod browse cards and instance content page layout (panels, shadows, hover accents)

## [5.2.11] - 2026-07-01

### Changed
- CI/CD is Windows-only — faster releases, no Linux/macOS build jobs

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
