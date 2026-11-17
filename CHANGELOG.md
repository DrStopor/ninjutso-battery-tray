# Changelog

## Unreleased

## 0.3.0 - 2026-11-17

### Changed

- Remove Commons Clause from the current source and use the standard MIT License,
  permitting redistribution and sale, including in software bundles. Update Cargo
  metadata, all five README license sections/badges and package notices. Previously
  published archives are not modified by this source change.
- Embed pinned Source Sans 3 and the official Noto Sans SC Chinese regional
  subset in both GPUI hosts; retain complete SIL OFL 1.1 notices and verify asset
  hashes. Share outline-based metric measurement without OS font installation.
- Use a GPUI 0.2.2 frontend; keep Rust 1.100.0 pinned and Inno Setup 6.7.4 as the minimum installer compiler.
- Increase card text sizes by one logical pixel while retaining the 304 x 156 window and fixed metric axes.
- Extract renderer-independent application state, localization, settings serialization, flyout geometry, typography, palettes, interaction rules, and tray pixels into a reusable library.
- Isolate Windows HID access, locale detection, settings storage, and the native tray shell under platform/windows behind a shared desktop boundary.
- Rebuild the status card in GPUI: 304 x 156 logical pixels, vector icons, automatic Windows light/dark appearance, a close button, and keyboard focus support.
- Add a shared verification script, portable presentation contract tests, and a pixel regression test covering the existing tray renderer.
- Synchronize source, Rust, installer and license badges across all five README translations, and check their versions and local links during verification.

### Added

- Add a Linux host for X11/XWayland: hidraw access with a scoped udev rule, StatusNotifierItem/DBusMenu, XDG settings, locale detection and a per-user kernel lock. Target Debian 13 and Ubuntu 26.04 on x86-64; native Wayland remains out of scope for the pinned GPUI version.
- Reuse the GPUI controller/card and getter-only telemetry pipeline across Windows and Linux, with platform-specific window lifecycle behind a narrow desktop boundary and shared application-owned fonts.
- Add isolated Linux desktop integration checks, Debian-baseline builds, `.deb`/archive packaging, desktop integration and Windows/Linux license notices.
- Backport Linux ENODATA handling in GPUI's old xattr dependency; retain upstream sources, tests and licenses and document the compatibility patch.
- Add a fourth NinjaForce theme with light lilac surfaces and blue text/control accents inspired by the companion app; apply it to the card and native tray menus, persist the choice, and keep labels, status warnings and tray digits readable.
- Read the active DPI stage from the mouse and show it in a dedicated two-line outlined badge between the charge and power status, without increasing the flyout size; unavailable or invalid DPI never hides battery or polling-rate data.
- Size the numeric DPI reading within its larger uppercase caption and align it right; use glyph measurements for charge-top alignment and pixel checks for both themes and all five locales.
- Keep DPI geometry and typography fixed on the polling-rate badge's vertical axis, with a reserved right-aligned power-status column; single-digit/100% readings and localized status changes never move the badge.
- Add a localized Theme menu with Auto, Light and Dark choices, immediate rendering updates, persisted preferences and backward-compatible settings files.
- Apply the same theme to the native tray context menu and both submenus; retain native keyboard/MSAA behavior and system rendering for accessibility modes.
- Share the menu tree, typed commands, selection state and theme resolution across platform adapters in preparation for Linux support.
- Move portable logic and contract tests into a dependency-free core workspace package with unsafe code forbidden; keep GUI, HID and platform bindings in the desktop host.
- Verify DPI-only live updates and theme selection/persistence with GUI smoke tests in an isolated APPDATA; never change the owner's settings during tests.
- Read the active profile's configured polling rate through verified getter commands and show a localized outlined badge beside the device name, without increasing the 304 x 156 card.
- Hide unavailable/unknown polling rates without losing valid battery data; refresh the badge in place when only the rate changes and include it in the tray tooltip.
- Keep HID polling off the UI thread, with one serialized reader, one-second visible polling, device/resume notifications, and minute polling while hidden.
- Separate the native notification-area message pump from GPUI; restore the tray icon after Explorer restarts.
- Add portable flyout lifecycle tests, stale-dismiss protection, and a reproducible GUI smoke test for all five languages and live simulated states.
- Retain the Win32 frontend as a --no-default-features fallback build.
- Add third-party license notices, include them in both distributions, and reject stale notices during verification and packaging.
- Refresh localized README screenshots and verify the pinned GPUI badge in all five translations.
- Windows executable version metadata and --version / -V output.
- Linux compilation checks for the reusable core and presentation tests, plus native Linux host and desktop verification scripts.

### Fixed

- Accept verified short wired HID replies without inventing missing value bytes;
  restrict Linux telemetry to the vendor collection instead of probing keyboard,
  mouse and consumer-control interfaces.
- Anchor the Linux status-card menu action to the panel, not XWayland's stale
  pointer position inside the previous card, preventing downward drift on reopen.
- Acknowledge X11 unmap requests so Escape, close and focus loss reliably hide the Linux card; keep DBusMenu metadata prefetch from dismissing it.
- Reuse and hide the GPUI card so closing it does not terminate the tray application on Windows.
- Remove GPUI's normal-window frame insets from the frameless popup and prevent a tray click after deactivation from reopening it.
- Stop release packaging when Cargo or the installer compiler fails, verify the EXE version, and stage packages before copying them to the output directory.
- Restrict the Windows installer to systems capable of running the x64 application.

### Known limitations

- Linux support is experimental and requires X11/XWayland. Debian 13/GNOME with
  a physical mouse through VMware USB passthrough has been checked; Ubuntu 26.04,
  KDE and bare-metal Linux validation remain pending. See [the Linux guide](docs/linux.md).
- In the test VM, physically unplugging the receiver can generate a spurious
  keyboard shortcut even with the app stopped. Disconnect it through VMware
  before unplugging; the Linux guide records the evidence and workaround.
- Linux tray-menu appearance is controlled by the desktop, independently of
  the application's card theme.

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
