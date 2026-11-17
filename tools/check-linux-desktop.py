#!/usr/bin/env python3
"""Opt-in behavior smoke on a real Linux desktop (not a pixel/visual test).

Run only in a dedicated test profile; sends keys/moves the pointer. First start
our debug preview as a user systemd unit with charging/72%/1000Hz/1100DPI/USB.
Set NINJUTSO_TEST_SYSTEMD_UNIT to that unit name, then run this script. No USB
access or sudo is used. Leaves the preview visible in Russian/NinjaForce.
"""
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import traceback

import dbus


def failed(kind, value, trace):
    partial = globals().get("report", {})
    partial["failed"] = str(value)
    print(json.dumps(partial, ensure_ascii=False, indent=2), flush=True)
    traceback.print_exception(kind, value, trace)


sys.excepthook = failed


def command(*args):
    return subprocess.run(args, text=True, capture_output=True, timeout=4)


def wait_for(test, message, timeout=5):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        result = test()
        if result:
            return result
        time.sleep(0.05)
    raise AssertionError(message)


unit = os.environ.get("NINJUTSO_TEST_SYSTEMD_UNIT")
if not unit:
    raise SystemExit("Set NINJUTSO_TEST_SYSTEMD_UNIT explicitly; this test operates a live desktop.")
pid = int(command("systemctl", "--user", "show", unit, "-p", "MainPID", "--value").stdout)
assert pid > 0 and os.stat(f"/proc/{pid}").st_uid == os.getuid()
assert Path(f"/proc/{pid}/exe").resolve().name.startswith("ninjutso-battery-tray"), "Not our application"
bus = dbus.SessionBus()
service = f"org.kde.StatusNotifierItem-{pid}-1"
wait_for(lambda: bus.name_has_owner(service), "Preview SNI did not start", timeout=15)
bus_properties = dbus.Interface(bus.get_object("org.freedesktop.DBus", "/org/freedesktop/DBus"), "org.freedesktop.DBus")
assert int(bus_properties.GetConnectionUnixProcessID(service)) == pid
item = bus.get_object(service, "/StatusNotifierItem")
properties = dbus.Interface(item, "org.freedesktop.DBus.Properties")
menu = bus.get_object(service, str(properties.Get("org.kde.StatusNotifierItem", "Menu")))


def nodes(node):
    yield node
    for child in node[2]:
        yield from nodes(child)


def find(label):
    root = menu.GetLayout(dbus.Int32(0), dbus.Int32(-1), dbus.Array([], signature="s"),
                          dbus_interface="com.canonical.dbusmenu", timeout=3)[1]
    return next((node for node in nodes(root) if node[1].get("label") == label), None)


def click(label):
    node = wait_for(lambda: find(label), f"Menu label missing: {label}")
    menu.Event(dbus.Int32(node[0]), "clicked", dbus.Int32(0), dbus.UInt32(0),
               dbus_interface="com.canonical.dbusmenu", timeout=3)


def visible():
    result = command("xdotool", "search", "--onlyvisible", "--all", "--pid", str(pid),
                     "--class", "DrStopor.NinjutsoBatteryTray")
    return result.stdout.splitlines()[0] if result.returncode == 0 else None


def activate():
    item.Activate(dbus.Int32(1200), dbus.Int32(50), dbus_interface="org.kde.StatusNotifierItem", timeout=3)
    return wait_for(visible, "SNI Activate did not show card")


def title():
    return str(properties.Get("org.kde.StatusNotifierItem", "Title"))


report = {"desktop": os.environ.get("XDG_CURRENT_DESKTOP", "unknown"), "session_type": os.environ.get("XDG_SESSION_TYPE", "unknown"), "pid": pid, "passed": []}
card = wait_for(visible, "Preview card did not appear", timeout=15)
report["window"] = card
pixmap = properties.Get("org.kde.StatusNotifierItem", "IconPixmap")
assert len(pixmap[0][2]) == 4096 and int(pixmap[0][0]) == 32 and int(pixmap[0][1]) == 32
report["passed"].append("SNI icon ARGB32 and real desktop host")
menu.AboutToShow(dbus.Int32(0), dbus_interface="com.canonical.dbusmenu", timeout=3)
time.sleep(0.15)
assert visible() == card, "DBusMenu prefetch must not hide the card"
report["passed"].append("DBusMenu metadata prefetch is side-effect-free")

for language, open_label, charging in [
    ("Русский", "Открыть карточку", "Заряжается"),
    ("English", "Open status card", "Charging"),
    ("Беларуская", "Адкрыць картку", "Зараджаецца"),
    ("Қазақша", "Күй карточкасын ашу", "Зарядталуда"),
    ("简体中文", "打开状态卡片", "充电中"),
]:
    click(language)
    wait_for(lambda: find(open_label) and charging in title(), f"Locale did not update: {language}")
    assert visible() == card and "72%" in title() and "1100 DPI" in title()
    report["passed"].append(f"live language: {language}")

click("English")
wait_for(lambda: find("Open status card"), "English menu not applied")
for theme in ["Auto", "Light", "Dark", "NinjaForce"]:
    click(theme)
    wait_for(lambda: find(theme)[1].get("toggle-state") == 1, f"Theme radio failed: {theme}")
    assert visible() == card
    report["passed"].append(f"theme radio and retained window: {theme}")

command("xdotool", "windowactivate", "--sync", card).check_returncode()
command("xdotool", "key", "Escape").check_returncode()
wait_for(lambda: not visible(), "Escape did not dismiss card")
report["passed"].append("Escape dismisses without stopping tray")
assert activate() == card
report["passed"].append("SNI Activate reuses same X11 window")

command("xdotool", "key", "Tab").check_returncode()
command("xdotool", "key", "Return").check_returncode()
wait_for(lambda: not visible(), "Keyboard close button did not dismiss")
report["passed"].append("Tab/Enter close button")
click("Open status card")
assert wait_for(visible, "Status menu route did not open") == card
report["passed"].append("GNOME explicit status menu route")

info = command("xwininfo", "-id", card).stdout
x = int(re.search(r"Absolute upper-left X:\s*(-?\d+)", info)[1])
y = int(re.search(r"Absolute upper-left Y:\s*(-?\d+)", info)[1])
command("xdotool", "mousemove", "--window", card, "275", "28").check_returncode()
cursor = command("xdotool", "getmouselocation", "--shell").stdout
coordinates = dict(line.split("=", 1) for line in cursor.splitlines() if "=" in line)
if int(coordinates["X"]) == x + 275 and int(coordinates["Y"]) == y + 28:
    command("xdotool", "click", "1").check_returncode()
    wait_for(lambda: not visible(), "Close button click did not dismiss")
    report["passed"].append("close button pointer click")
    click("Open status card")
    assert wait_for(visible, "Reopen after close click failed") == card
else:
    report["pointer_close"] = "not verified: compositor rejected XWayland pointer warp"

root = re.search(r"Window id: (0x[0-9a-f]+)", command("xwininfo", "-root").stdout)[1]
command("xdotool", "windowfocus", root).check_returncode()
wait_for(lambda: not visible(), "Focus loss did not dismiss card")
report["passed"].append("focus-loss dismissal")
click("Open status card")
assert wait_for(visible, "Reopen after focus loss failed") == card
click("Русский")
wait_for(lambda: find("Открыть карточку"), "Russian restore failed")
click("NinjaForce")
report["left_running"] = True
report["visual_validation"] = "Not checked: this script tests behavior, not rendered pixels"
print(json.dumps(report, ensure_ascii=False, indent=2), flush=True)
