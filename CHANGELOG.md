# Changelog

## Unreleased

### Changed

- Prepare version 0.2.1 with Rust 1.100.0 pinned in rust-toolchain.toml and Inno Setup 6.7.4 as the minimum installer compiler.
- Extract renderer-independent application state, localization, settings serialization, flyout geometry, typography, palettes, interaction rules, and tray pixels into a reusable library.
- Isolate Windows HID access, locale detection, settings storage, and the native tray shell under platform/windows as preparation for a GPUI frontend.
- Add dark and light presentation palettes; keep the shipped dark appearance active in the current native frontend.
- Add a shared verification script, portable presentation contract tests, and a pixel regression test covering the existing tray renderer.
- Synchronize source, Rust, installer and license badges across all five README translations, and check their versions and local links during verification.

### Added

- Windows executable version metadata and --version / -V output.
- Linux compilation checks for the reusable core and presentation tests; the desktop application still supports Windows only.

### Fixed

- Stop release packaging when Cargo or the installer compiler fails, verify the EXE version, and stage packages before copying them to the output directory.
- Restrict the Windows installer to systems capable of running the x64 application.

## 0.2.0 - 2026-07-17

### Added

- Battery reading while Ninjutso Ten is connected and charging over USB (`PID_E020`).
- Russian, English, Belarusian, Kazakh, and Simplified Chinese (`zh-CN`) interface languages.
- A multilingual installer wizard in the same five languages, with Windows UI language detection and an explicit language picker.
- A language submenu with persisted selection and Windows UI language detection on first launch.
- Connection source (`USB` or receiver) in the popup and tray tooltip.
- Localized README files and matching flyout screenshots for every interface language.

### Changed

- Redesigned the status popup with clearer hierarchy, compact battery visualization, rounded corners, and multi-monitor-aware placement.
- Added double-buffered popup rendering and stopped unnecessary popup redraws during tray charging animation.
- Rebuilt the status flyout as a smaller, quieter card with improved typography, spacing, connection badge, rounded battery bar, and light-dismiss behavior.
- Refresh battery and connection data before opening the flyout and once per second while it remains visible, so USB, receiver, and disconnected transitions appear without reopening it.
- Added single-instance protection with a localized message when the utility is already running.
- Split HID access, localization, and persisted settings into dedicated modules.
- Reworked the tray icon: numeric mode now uses stable, high-contrast digits on a quiet surface, while graphical mode uses a simpler mouse silhouette.
- Clarified at the top of the license text that the MIT terms apply only together with the Commons Clause and are not the MIT License alone.
- Made release packaging produce clean GitHub assets and GitFlic-compatible zip wrappers in one run.

## 0.1.0 - 2026-07-09

Initial public release.

### Added

- Tray indicator for Ninjutso Ten battery level.
- Popup with battery percentage, connection state, and charging state.
- Automatic battery refresh every minute.
- Manual refresh and percent display toggle from the tray menu.
- Per-user settings stored in `%APPDATA%\NinjutsoBatteryTray\settings.txt`.
- Windows installer with current-user install by default, optional all-users install, autostart option, and standard uninstall support.
- Portable zip distribution.
- MIT License with Commons Clause License Condition v1.0.
