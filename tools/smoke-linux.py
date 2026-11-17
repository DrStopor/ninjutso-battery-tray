#!/usr/bin/env python3
"""Real GPUI + SNI/DBusMenu integration, only under a private bus and Xvfb.

Run: dbus-run-session -- xvfb-run -a python3 tools/smoke-linux.py
No USB access, global desktop preferences, or installed application is touched.
"""
import os
import re
from pathlib import Path
import subprocess
import tempfile
import time

import dbus
import dbus.service
from dbus.mainloop.glib import DBusGMainLoop
from gi.repository import GLib

ROOT = Path(__file__).resolve().parents[1]
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
BINARY = Path(os.environ.get("NINJUTSO_TEST_BINARY", TARGET / "debug/ninjutso-battery-tray"))
OUTPUT = Path(os.environ.get("NINJUTSO_CAPTURE_DIR", ROOT / "target/verification/linux"))
OUTPUT.mkdir(parents=True, exist_ok=True)
DBusGMainLoop(set_as_default=True)
BUS = dbus.SessionBus()
CONTEXT = GLib.MainContext.default()


def pump():
    while CONTEXT.pending():
        CONTEXT.iteration(False)


def wait_for(test, message, timeout=15):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        pump()
        value = test()
        if value:
            return value
        time.sleep(0.03)
    raise AssertionError(message)


class Watcher(dbus.service.Object):
    def __init__(self):
        self.name = dbus.service.BusName("org.kde.StatusNotifierWatcher", BUS)
        super().__init__(BUS, "/StatusNotifierWatcher")
        self.services = []

    @dbus.service.method("org.kde.StatusNotifierWatcher", in_signature="s", out_signature="")
    def RegisterStatusNotifierItem(self, service):
        self.services.append(str(service))

    @dbus.service.method("org.kde.StatusNotifierWatcher", in_signature="s", out_signature="")
    def RegisterStatusNotifierHost(self, _service):
        pass

    @dbus.service.method("org.freedesktop.DBus.Properties", in_signature="ss", out_signature="v")
    def Get(self, _interface, name):
        return {"IsStatusNotifierHostRegistered": dbus.Boolean(True),
                "ProtocolVersion": dbus.Int32(0),
                "RegisteredStatusNotifierItems": dbus.Array(self.services, signature="s")}[name]


WATCHER = Watcher()


def window_for(process, visible=True):
    args = ["xdotool", "search"] + (["--onlyvisible"] if visible else [])
    result = subprocess.run(args + ["--pid", str(process.pid), "--name", "Ninjutso Ten:"],
                            text=True, capture_output=True)
    return result.stdout.splitlines()[0] if result.returncode == 0 and result.stdout.strip() else None


def layout(menu):
    pump()
    return menu.GetLayout(dbus.Int32(0), dbus.Int32(-1), dbus.Array([], signature="s"),
                          dbus_interface="com.canonical.dbusmenu")[1]


def flatten(node):
    yield node
    for child in node[2]:
        yield from flatten(child)


def find(menu, label):
    return next((n for n in flatten(layout(menu)) if n[1].get("label") == label), None)


def click(menu, label):
    node = wait_for(lambda: find(menu, label), f"Menu label missing: {label}")
    menu.Event(dbus.Int32(node[0]), "clicked", dbus.Int32(0), dbus.UInt32(0),
               dbus_interface="com.canonical.dbusmenu")


def run_case(language, config):
    before = len(WATCHER.services)
    env = os.environ.copy()
    env.update(XDG_CONFIG_HOME=str(config), XDG_RUNTIME_DIR=str(config / "runtime"),
               LANG="en_US.UTF-8", LC_ALL="C.UTF-8", NINJUTSO_LOG="info", RUST_BACKTRACE="1")
    Path(env["XDG_RUNTIME_DIR"]).mkdir(mode=0o700, parents=True, exist_ok=True)
    args = [str(BINARY), "--preview", f"--preview-language={language}",
            "--preview-theme=ninjaforce", "--preview-state=charging", "--preview-percent=72",
            "--preview-polling-rate=1000", "--preview-dpi=1100", "--preview-connection=usb"]
    log = (OUTPUT / f"{language}.log").open("w")
    process = subprocess.Popen(args, env=env, stdout=log, stderr=log)
    try:
        service = wait_for(lambda: WATCHER.services[-1] if len(WATCHER.services) > before else None,
                           "SNI did not register")
        card = wait_for(lambda: window_for(process), "GPUI card did not appear", timeout=90)
        obj = BUS.get_object(service, "/StatusNotifierItem")
        props = dbus.Interface(obj, "org.freedesktop.DBus.Properties")
        title = str(props.Get("org.kde.StatusNotifierItem", "Title"))
        assert "72%" in title and "1100 DPI" in title, title
        icons = props.Get("org.kde.StatusNotifierItem", "IconPixmap")
        assert len(icons) >= 1 and len(icons[0][2]) == 32 * 32 * 4
        menu_path = str(props.Get("org.kde.StatusNotifierItem", "Menu"))
        menu = BUS.get_object(service, menu_path)
        labels = [str(n[1].get("label", "")) for n in flatten(layout(menu))]
        assert all(label in labels for label in ["Русский", "English", "Беларуская", "Қазақша", "简体中文", "NinjaForce"])
        geometry = subprocess.check_output(["xdotool", "getwindowgeometry", "--shell", card], text=True)
        assert "WIDTH=304" in geometry and "HEIGHT=156" in geometry, geometry
        assert process.poll() is None, f"Application exited; see {language}.log"
        subprocess.run(["xdotool", "windowfocus", "--sync", card], check=True)
        time.sleep(0.25)
        screenshot = OUTPUT / f"flyout.{language}.png"
        def capture_frame():
            assert process.poll() is None, f"Application exited; see {language}.log"
            info = subprocess.check_output(["xwininfo", "-id", card], text=True)
            rectangle = ",".join(re.search(pattern, info)[1] for pattern in [
                r"Absolute upper-left X:\s*(-?\d+)", r"Absolute upper-left Y:\s*(-?\d+)",
                r"Width:\s*(\d+)", r"Height:\s*(\d+)"])
            # Popups need not be _NET_ACTIVE_WINDOW; capture the verified card
            # rectangle. Overwrite stale captures so a previous blank frame can
            # never be mistaken for a failure to present the new one.
            result = subprocess.run(["scrot", "-o", "-a", rectangle, str(screenshot)], capture_output=True, timeout=5)
            return result.returncode == 0 and screenshot.stat().st_size > 2000
        wait_for(capture_frame, "GPUI did not present a nonblank frame", timeout=90)
        subprocess.run(["xdotool", "key", "Escape"], check=True)
        wait_for(lambda: not window_for(process), "Escape did not hide the window")
        obj.Activate(dbus.Int32(800), dbus.Int32(400), dbus_interface="org.kde.StatusNotifierItem")
        assert wait_for(lambda: window_for(process), "SNI Activate did not reopen card") == card
        # Host menu activation must be reflected back to GPUI and D-Bus properties.
        click(menu, "English")
        wait_for(lambda: find(menu, "Open status card"), "Language change did not update DBusMenu")
        for theme in ["Auto", "Light", "Dark", "NinjaForce"]:
            click(menu, theme)
            wait_for(lambda: find(menu, theme)[1].get("toggle-state") == 1,
                     f"Theme radio state did not change: {theme}")
            assert window_for(process) == card
        # Explicit menu opening route is required on GNOME's menu-on-primary-click host.
        subprocess.run(["xdotool", "key", "Escape"], check=True)
        wait_for(lambda: not window_for(process), "Card stayed visible")
        click(menu, "Open status card")
        assert wait_for(lambda: window_for(process), "Menu did not reopen card") == card
        # Close button (inside the same fixed-size window), not a window-manager kill.
        subprocess.run(["xdotool", "mousemove", "--window", card, "275", "28", "click", "1"], check=True)
        wait_for(lambda: not window_for(process), "Close button did not hide the card")
        click(menu, "Open status card")
        wait_for(lambda: window_for(process), "Card did not reopen after X")
        root_window = re.search(r"Window id: (0x[0-9a-f]+)",
                                subprocess.check_output(["xwininfo", "-root"], text=True))[1]
        subprocess.run(["xdotool", "windowfocus", "--sync", root_window], check=True)
        wait_for(lambda: not window_for(process), "Focus loss did not dismiss card")
        click(menu, "Exit")
        wait_for(lambda: process.poll() is not None, "Tray Exit did not stop application")
        assert process.returncode == 0, process.returncode
        assert not (config / "ninjutso-battery-tray/settings.txt").exists(), "Preview wrote preferences"
        print(f"Linux SNI + GPUI passed: {language}, pixels, menus, radio states, reused window, Escape/X/Exit", flush=True)
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
        log.close()


def persistence_case(config):
    env = os.environ.copy()
    env.update(XDG_CONFIG_HOME=str(config), XDG_RUNTIME_DIR=str(config / "runtime"), LC_ALL="C.UTF-8")
    Path(env["XDG_RUNTIME_DIR"]).mkdir(mode=0o700, parents=True, exist_ok=True)
    settings = config / "ninjutso-battery-tray/settings.txt"
    for restart in [False, True]:
        before = len(WATCHER.services)
        with (OUTPUT / f"persistence-{restart}.log").open("w") as log:
            process = subprocess.Popen([str(BINARY)], env=env, stdout=log, stderr=log)
            try:
                service = wait_for(lambda: WATCHER.services[-1] if len(WATCHER.services) > before else None,
                                   "Production SNI did not start")
                obj = BUS.get_object(service, "/StatusNotifierItem")
                props = dbus.Interface(obj, "org.freedesktop.DBus.Properties")
                menu = BUS.get_object(service, str(props.Get("org.kde.StatusNotifierItem", "Menu")))
                if not restart:
                    click(menu, "NinjaForce")
                    wait_for(lambda: settings.exists() and "theme=ninjaforce" in settings.read_text(),
                             "Theme was not saved")
                    click(menu, "Русский")
                    wait_for(lambda: find(menu, "Открыть карточку"), "Language was not applied")
                    click(menu, "Показать процент")
                    wait_for(lambda: "large_icon_percent=false" in settings.read_text() and "language=ru" in settings.read_text(),
                             "Preferences were not preserved together")
                else:
                    assert find(menu, "Открыть карточку")
                    assert find(menu, "NinjaForce")[1].get("toggle-state") == 1
                    assert find(menu, "Показать процент")[1].get("toggle-state") == 0
                # A second normal launch must exit without registering another SNI.
                count = len(WATCHER.services)
                duplicate = subprocess.run([str(BINARY)], env=env, text=True, capture_output=True, timeout=5)
                assert duplicate.returncode == 0 and len(WATCHER.services) == count
                assert "уже запущен" in duplicate.stderr
                click(menu, "Открыть карточку")
                card = wait_for(lambda: window_for(process), "Production card did not open")
                time.sleep(1)
                info = subprocess.check_output(["xwininfo", "-id", card], text=True)
                rectangle = ",".join(re.search(pattern, info)[1] for pattern in [
                    r"Absolute upper-left X:\s*(-?\d+)", r"Absolute upper-left Y:\s*(-?\d+)",
                    r"Width:\s*(\d+)", r"Height:\s*(\d+)"])
                shot = OUTPUT / f"production-{restart}.png"
                subprocess.run(["scrot", "-o", "-a", rectangle, str(shot)], check=True)
                assert shot.stat().st_size > 2000, "Production card is blank"
                click(menu, "Выход")
                wait_for(lambda: process.poll() is not None, "Production app did not exit")
                assert process.returncode == 0
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=5)
    print("Linux production passed: atomic XDG persistence, locale override, single instance and restart", flush=True)


def main():
    # Openbox supplies focus/activation; Xvfb and this D-Bus session are isolated.
    wm = subprocess.Popen(["openbox"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        with tempfile.TemporaryDirectory(prefix="settings-", dir=OUTPUT) as private:
            if not os.environ.get("NINJUTSO_TEST_RELEASE"):
                for language in ["en", "ru", "be", "kk", "zh-CN"]:
                    run_case(language, Path(private) / language)
            persistence_case(Path(private) / "production")
    finally:
        wm.terminate()
        wm.wait(timeout=5)


if __name__ == "__main__":
    main()
