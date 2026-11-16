**English** · [Русский](README.ru.md) · [Беларуская](README.be.md) · [Қазақша](README.kk.md) · [简体中文](README.zh-CN.md)

# Ninjutso Battery Tray

[![Source version 0.3.0](https://img.shields.io/badge/source-v0.3.0-2563eb)](Cargo.toml)
[![Latest release](https://img.shields.io/github/v/release/DrStopor/ninjutso-battery-tray?display_name=tag&sort=semver&label=release)](https://github.com/DrStopor/ninjutso-battery-tray/releases/latest)
![Windows x64](https://img.shields.io/badge/Windows-x64-0078D4)
[![Rust 1.100.0](https://img.shields.io/badge/Rust-1.100.0-000000?logo=rust&logoColor=white)](rust-toolchain.toml)
[![GPUI 0.2.2](https://img.shields.io/badge/GPUI-0.2.2-00b4b6)](https://crates.io/crates/gpui/0.2.2)
[![Inno Setup 6.7.4+](https://img.shields.io/badge/Inno%20Setup-6.7.4%2B-2ea44f)](installer/ninjutso-battery-tray.iss)
[![MIT with Commons Clause](https://img.shields.io/badge/license-MIT%20%2B%20Commons%20Clause-f59e0b)](LICENSE)

A focused Windows tray utility that keeps your Ninjutso Ten battery level visible without opening a full companion app.

![GPUI status card in English: 72 percent, charging over USB, 1000 Hz, 1100 DPI](docs/assets/screenshots/flyout.en.png)

Illustrative preview data; the running app reads your mouse.

## Highlights

- **Exact battery level in the tray.** High-contrast digits remain legible at the notification area's real icon size.
- **Receiver and wired USB support.** The app reads the battery through the wireless receiver and while the mouse is charging over USB.
- **A compact GPUI card.** A 304 × 156 logical-pixel layout with crisp vector icons, automatic light/dark appearance, and live connection states.
- **Polling rate beside the device name.** An outlined badge shows the active profile's configured rate read from the mouse. If unavailable, the badge hides without affecting the battery reading.
- **Dedicated DPI badge.** A two-line outlined badge shows a large `DPI` caption above a right-aligned numeric reading. It sits between the charge and power status; unavailable readings are hidden.
- **Automatic and manual refresh.** Battery data updates every minute, with an on-demand refresh in the tray menu.
- **Local and private.** No cloud service, account, network access, or telemetry.
- **Single-instance protection.** Starting the app again never creates duplicate tray icons.
- **Five interface languages.** English, Russian, Belarusian, Kazakh, and Simplified Chinese.

## Supported hardware

Ninjutso Battery Tray currently supports the **Ninjutso Ten** through:

- its wireless receiver;
- a wired USB connection, including while the mouse is charging.

If the mouse is not detected, connect it by USB or turn it on, then select **Refresh** from the tray menu.

## Installation

[**Download the latest release from GitHub.**](https://github.com/DrStopor/ninjutso-battery-tray/releases/latest)

- For most people, use `NinjutsoBatteryTraySetup-*.exe`. It provides autostart and standard Windows uninstall support.
- For a no-install option, use `ninjutso-battery-tray-*-portable.zip`.
- `SHA256SUMS.txt` contains checksums for verifying downloaded files.

The installer targets the current user by default and does not require administrator rights. Its wizard can also enable autostart or switch to an all-users installation. The wizard is available in English, Russian, Belarusian, Kazakh, and Simplified Chinese.

After launch, the app appears only in the system tray. Windows may place a new icon behind the hidden-icons arrow; drag it into the main notification area if you want it always visible.

## Using the app

Left-click the tray icon to open the status card. Close it with ×, `Esc`, another tray click, or a click outside. Clicking the status content keeps it open. While visible, data refreshes every second; USB, receiver, charging, and disconnected states change without reopening the window.

Right-click the icon to open the menu:

- **Refresh** — read the battery again;
- **Show percentage** — switch between the numeric and graphical tray icons;
- **Language** — change the interface language;
- **Theme** — Auto (follow Windows), Light, or Dark;
- **Exit** — close the app.

In numeric mode, the colored edge communicates the battery range: green for 50–100%, amber for 20–49%, and red for 0–19%. The exact charging and connection states remain available in the flyout and tooltip.

The selected language, icon mode, and theme are saved automatically. The card and tray menu, including submenus, share the same theme. Auto follows the Windows app appearance; explicit Light or Dark choices override it without restarting.

## Languages

The interface includes English, Russian, Belarusian, Kazakh, and Simplified Chinese (`zh-CN`). On first launch, the app follows the Windows display language when it is supported and otherwise falls back to English.

The `zh-CN` translation uses Simplified Chinese for mainland China.

## Managing the installation

### Autostart

Run the installer again to enable or disable **Start with Windows**.

### Updating

Download the new installer and run it over the existing installation. Your install mode and app settings are preserved.

### Uninstalling

Open **Windows Settings → Apps → Installed apps**, find **Ninjutso Battery Tray**, and select **Uninstall**.

## Privacy

Battery data is read locally through the Windows HID API. The app does not send data anywhere and contains no analytics, advertising, or background update checker.

## Support

[Open an issue on GitHub](https://github.com/DrStopor/ninjutso-battery-tray/issues) and include your Windows version, connection type (receiver or USB), and app version.

## License

The source may be used, copied, modified, and redistributed under the terms in [LICENSE](LICENSE). The Commons Clause forbids selling the utility itself or a product or service whose value derives substantially from it. This is a source-available project, not OSI open source.

Third-party components, including GPUI, retain their own licenses; see [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt).

<details>
<summary><strong>Build from source</strong></summary>

Requirements:

- Windows x64; the GPUI frontend uses DirectX 11;
- MSVC C++ Build Tools;
- Rust 1.100.0 with Cargo, selected by [rust-toolchain.toml](rust-toolchain.toml);
- Windows SDK with `rc.exe` and `fxc.exe` (release shader compiler);
- Inno Setup 6.7.4 or newer for building the installer.

```powershell
cargo test --workspace
cargo build --release
```

Portable logic lives in the dependency-free [core package](crates/core/Cargo.toml), where unsafe code is forbidden. Run `cargo test -p ninjutso-battery-core` to test it independently on Windows or Linux; the Linux desktop application is not implemented yet.

The first GPUI build downloads more dependencies. If `fxc.exe` is not on PATH, set `GPUI_FXC_PATH` to its full path. The packaging and verification scripts also detect installed SDKs.

The previous Win32 frontend is available as a fallback build:

```powershell
cargo build --no-default-features --target-dir target/native
```

Visual smoke checks and reproducible previews for all five languages:

```powershell
.\tools\smoke-ui.ps1
```

To regenerate third-party notices after changing dependencies, install `cargo-about 0.9.3` with `--features cli`, then run [tools/update-licenses.ps1](tools/update-licenses.ps1). Normal builds and packaging use the checked-in notices and do not need this tool.

Build every release artifact with:

```powershell
.\tools\package-release.ps1
```

The release executable is written to `target\release\ninjutso-battery-tray.exe`; packaged artifacts are written to `dist`. Protocol notes for contributors are in [docs/hid-protocol.md](docs/hid-protocol.md), and release changes are tracked in [CHANGELOG.md](CHANGELOG.md).

Run the complete checks with [tools/verify.ps1](tools/verify.ps1), including README versions, badges, and local links.

</details>
