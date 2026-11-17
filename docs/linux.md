# Linux support

Version 0.3.0 introduces an **experimental** Linux desktop host. Debian 13/GNOME
with XWayland and a physical mouse through VMware USB passthrough has been checked;
Ubuntu 26.04, KDE and bare-metal desktop validation remain pending. Native Wayland
is not supported. If matching Linux assets are not provided on the release page,
build the packages from this source. Build and packaging scripts never publish
a release themselves.

## Scope

- Primary targets: Debian 13 and Ubuntu 26.04, x86-64.
- X11, including XWayland inside a Wayland session. **Native Wayland is not
  supported** by this GPUI 0.2.2 build; it is not silently advertised as native.
- A working Vulkan driver is required. Mesa software Vulkan can be used for
  isolated tests; normal hardware acceleration is preferable for daily use.
- Tray: StatusNotifierItem/DBusMenu on the user session bus. GNOME requires an
  AppIndicator extension; KDE Plasma and compatible panels provide a host.
- Wired/receiver battery, charging, configured polling rate and DPI use the same
  getter-only protocol and shared telemetry pipeline as Windows.
- All five languages and all four card themes are retained. Windows and Linux
  embed the same Source Sans 3 fonts and the official Noto Sans SC regional subset for
  Mainland Chinese. No Microsoft, Source Sans or Noto system-font installation is required.

The shared card keeps the same fixed metric axis on both platforms. Its
typography adapter measures real glyph outlines from the exact embedded font
bytes, avoiding GPUI 0.2.2's Linux advance-box placeholder. Native rasterization
can still differ slightly. Fonts retain SIL OFL 1.1, separately from the
application's MIT License; see [font provenance and redistribution](../assets/fonts/UPSTREAM.md).

The native tray menu follows the **desktop's** theme, font scaling, accessibility
and placement. An application cannot reliably recolor a host-rendered DBusMenu.
The application's Auto/Light/Dark/NinjaForce choice controls its GPUI card, not
other applications or the panel. Some GNOME hosts open the menu on primary click;
use **Open status card** there. Other hosts can open the card directly.

## Installation

Build or obtain the corresponding local `.deb`, then:

```sh
sudo apt install ./ninjutso-battery-tray_0.3.0_amd64.deb
```

The package installs the binary, desktop launcher, icon, project license,
third-party notices and a narrow udev rule. It **does not** launch the app as root,
enable extensions, change global desktop settings or opt you into autostart.
Reconnect the mouse/receiver after installing the rule. Start **Ninjutso Battery
Tray** from the application launcher or run `ninjutso-battery-tray`.

On GNOME, install your distribution's `gnome-shell-extension-appindicator` package
if it is missing, then enable that extension using the desktop's extension
manager. Ubuntu's desktop may already include it. A running user D-Bus session
and `$DISPLAY` are required; on Wayland the latter is supplied by XWayland.
If GNOME was already running when the extension was installed, log out and back
in before enabling it. Debian 13's package uses the extension identifier
`ubuntu-appindicators@ubuntu.com`.

## HID permissions

Never run the application with `sudo`, grant `0666` to all HID devices, detach a
kernel driver or add the user to the `input` group. The supplied rule grants the
active local seat user access to **only** `093a:e020` and `093a:eb01` hidraw nodes.

For a source/archive installation, install the same rule manually:

```sh
sudo install -m 644 packaging/linux/70-ninjutso-battery-tray.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
# Unplug and reconnect the mouse/receiver, then test as your normal user:
ninjutso-battery-tray --once
```

`--once` works without X11, D-Bus or a GPU. A remote SSH user without an active
local seat does not automatically receive a `uaccess` ACL. The app distinguishes
permission errors from a missing mouse and never substitutes fake battery/DPI.

### VMware physical-unplug caveat

In the Debian 13/GNOME test VM on Windows with VMware Workstation 26.0.1, physically
unplugging the receiver produced an invalid boot-keyboard input packet:
`81 06 00 22 00 00 48 00`. Those bytes match a USB GET_REPORT_DESCRIPTOR setup
header, not a normal keyboard report. GNOME interpreted it as Ctrl+Super+5 and
opened a new window of its fifth pinned application (Text Editor in that profile).

This was reproduced with the tray application completely stopped, including a
fresh receiver power cycle without any application feature requests. The faulty
packet originates in the testbed's USB/HID path; the exact controller, forwarding
or firmware component has not been isolated. This is not a confirmed bare-metal
Linux issue or a GPUI key binding.

In the owner-operated checks, using VMware **Removable Devices → Ninjutso Ten →
Disconnect (Connect to Host)** before physically unplugging did not trigger the
editor. Use that order in this testbed. Do not mask the symptom by globally
disabling GNOME shortcuts or the mouse's keyboard/macro interfaces. Changing the
VM's USB controller would be a separate configuration experiment, not an
application fix.

## Settings and autostart

Settings are saved atomically in
`${XDG_CONFIG_HOME:-$HOME/.config}/ninjutso-battery-tray/settings.txt`. The format
matches Windows, but no Windows paths are used. The initial language follows
`LC_ALL`, `LC_MESSAGES`, then `LANG`; an explicit app selection takes precedence.
A kernel file lock under `$XDG_RUNTIME_DIR` prevents duplicate instances and is
released on exit/crash. Without a runtime directory, the private config directory
is used. The lock inode is not deleted on normal exit, avoiding startup races.

To opt into autostart after installing the `.deb`:

```sh
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/autostart"
cp /usr/share/applications/io.github.DrStopor.NinjutsoBatteryTray.desktop \
  "${XDG_CONFIG_HOME:-$HOME/.config}/autostart/"
```

Remove only that copied `.desktop` file to disable autostart. Package removal
leaves your preferences and user-created autostart entry untouched.

## Building and verification

Use Rust **1.100.0**. The reproducible build image is based on Debian 12's glibc
2.36 to avoid unnecessarily requiring the latest glibc. This is an ABI baseline,
not a claim that every older distribution/desktop has been tested.

```sh
docker build -t ninjutso-linux-build:bookworm-rust1.100.0 \
  -f packaging/linux/Dockerfile packaging/linux
docker run --rm --memory=6g --cpus=4 \
  --mount "type=bind,source=$PWD,target=/work" \
  -e CARGO_HOME=/work/target/tooling/cargo-linux \
  ninjutso-linux-build:bookworm-rust1.100.0 bash tools/verify-linux.sh
docker run --rm --memory=6g --cpus=4 \
  --mount "type=bind,source=$PWD,target=/work" \
  -e CARGO_HOME=/work/target/tooling/cargo-linux \
  ninjutso-linux-build:bookworm-rust1.100.0 bash tools/package-linux.sh
```

The image lists native build dependencies. Packaging resolves actual ELF
dependencies with `dpkg-shlibdeps`, checks license notices and creates a `.deb`,
an installation-tree `.tar.gz`, and SHA-256 checksums under `target/linux-package`.
Nothing is installed on the host by these commands. The archive itself does not
install udev permissions or desktop integration.

The isolated desktop test uses a private D-Bus session, fake SNI host, Xvfb and
simulated data. Its raw X11 screenshot assertion is not a substitute for testing
a real compositor: a Vulkan/XWayland window can be visible while direct window
capture returns transparent pixels. The current Bookworm/Xvfb pixel check has
not passed; do not report that check as successful based on the VM results.

### Opt-in check on a real desktop

Use a separate test profile with an active graphical session and SNI host.
This check sends keys and moves the pointer; do not run it in a personal session
or interact with the desktop while it runs. It does not access the mouse over
USB, use sudo, or change the desktop theme. Start a **debug** preview with fixed
data (substitute the actual binary path):

```sh
systemd-run --user --unit=ninjutso-preview --collect \
  "$PWD/target/debug/ninjutso-battery-tray" \
  --preview --preview-language=ru --preview-theme=ninjaforce \
  --preview-state=charging --preview-percent=72 \
  --preview-polling-rate=1000 --preview-dpi=1100 --preview-connection=usb
NINJUTSO_TEST_SYSTEMD_UNIT=ninjutso-preview.service \
  python3 tools/check-linux-desktop.py
# When finished with the preview:
systemctl --user stop ninjutso-preview.service
```

The script needs `python3-dbus`, `xdotool` and `x11-utils`. It checks five live
language changes, four theme radio states, the tray icon, Escape, keyboard and
pointer close buttons, focus-loss dismissal and reopening the same X11 window.
It explicitly reports that it is **not** a rendered-pixel/visual regression test.

Initial real-desktop validation: Debian 13 / GNOME 48, Wayland session using
XWayland and Mesa llvmpipe software Vulkan. The owner confirmed the card is
visibly rendered, and the behavior check passed. This does not establish native
Wayland support, GPU performance or KDE coverage. Initial physical-device reads
through VMware USB passthrough confirmed receiver battery, polling rate and DPI,
and the active test user's uaccess ACL. A wired getter probe also confirmed
battery, charging, polling rate and DPI, with short (9/10-byte) responses. The
application now accepts those responses only when every required value byte and
the command echo are present. This is not yet a bare-metal Linux USB test.

## Protocol references

- [StatusNotifierItem specification](https://specifications.freedesktop.org/status-notifier-item/latest-single/)
- [HIDAPI Linux permissions](https://github.com/libusb/hidapi)
- [GNOME AppIndicator extension](https://github.com/ubuntu/gnome-shell-extension-appindicator)
