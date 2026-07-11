# Quartz Launcher

Quartz Launcher is an enhanced fork of [PandoraLauncher](https://github.com/Moulberry/PandoraLauncher) — a modern, native Minecraft launcher built with Rust and GPUI. It keeps Pandora's full feature set and adds new tools on top.

<!-- readme:build-info:start -->
## Build info

This section is updated automatically after every build.

| | |
|---|---|
| **Version** | `5.2.38` |
| **Last built** | 2026-07-11 20:40 UTC |
| **Latest release** | [v5.2.38](https://github.com/itsinvin/QuartzLauncher/releases/tag/v5.2.38) |
| **Commit** | `c502c005` |
| **Changelog date** | 2026-07-11 |

### Recent changes (5.2.38)

- Instance launch failing with "Failed to load game libraries: Failed to perform I/O operation" (path-not-found during library writes; now uses atomic writes, mkdir retries, and includes the failing path in the error)
- Missing `${arch}` substitution for native library classifiers on older Minecraft/Forge versions
- Natives extraction skipping files when zip entries lack parent directories
- Empty Modrinth icon URLs causing image asset load errors
- Recommended cards showing empty state while still loading
- Folder resource packs using all-zero SHA1 hashes (caused Modrinth update 404s)
- Forge processor argument expansion panicking on unknown placeholders
- Opaque instance/library I/O errors that hid the underlying OS message

<!-- readme:build-info:end -->

## Downloads

Prebuilt releases are on [GitHub Releases](https://github.com/itsinvin/QuartzLauncher/releases).

Quartz Launcher is **Windows only** (x86_64).

| Installer | Portable |
|-----------|----------|
| `QuartzLauncher-Windows-x86_64-Setup.exe` | `QuartzLauncher-Windows-x86_64-Portable.exe` |

## Features

Everything from Pandora, including:

- Instance management with cards and list views
- Cross-instance file syncing (options, saves, resource packs, and more)
- Mod deduplication via hard links when installed through the launcher
- Secure account credential storage using platform keyrings
- Custom game output window
- Modrinth and CurseForge content browsers
- Automatic redaction of sensitive information in logs
- Import from other launchers
- Unique modpack management workflow

Quartz enhancements:

- **Performance estimator** (Tools → Performance) — hardware-aware FPS and RAM estimates for modded workloads. This is one utility among many, not the focus of the launcher.
- Rebranded UI and data paths under `QuartzLauncher`

## Building

Requires a recent Rust toolchain (edition 2024).

```bash
cargo build --release
python scripts/update_readme.py
```

The README **Build info** section is refreshed automatically in CI and release builds (`scripts/update_readme.py`).

Windows packaging: `scripts/build_windows.sh`

## Attribution

Quartz Launcher is based on [PandoraLauncher](https://github.com/Moulberry/PandoraLauncher) by Moulberry. Pandora is licensed under its original terms; see upstream for details.

## FAQ

### Where can I suggest a feature or report a bug?

Please use GitHub issues on this repository.

### Why Quartz instead of Pandora?

Quartz is a community fork that preserves Pandora's architecture and features while adding optional tools (like performance estimation) and independent branding. It is not affiliated with the original Pandora project.

### Will Quartz be monetized?

No. Quartz follows the same philosophy as Pandora: no ads, no monetization.
