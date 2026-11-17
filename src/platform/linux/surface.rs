//! X11/XWayland lifecycle for our own retained GPUI window. No global hooks.
use gpui::Window;
use x11rb::connection::Connection;
use x11rb::errors::ReplyError;
use x11rb::protocol::{
    ErrorKind,
    randr::ConnectionExt as _,
    xproto::{self, ConnectionExt as _},
};
use x11rb::wrapper::ConnectionExt as _;

use crate::presentation::layout::{Bounds, Point};

// Another X11 client can destroy a window while we walk the window tree.
// Ignore only that race; permission/protocol/connection errors stay visible.
fn live_window<T>(result: Result<T, ReplyError>) -> anyhow::Result<Option<T>> {
    match result {
        Ok(reply) => Ok(Some(reply)),
        Err(ReplyError::X11Error(error)) if error.error_kind == ErrorKind::Window => Ok(None),
        Err(error) => Err(error.into()),
    }
}

// GPUI 0.2.2 leaves HasWindowHandle for X11Window unimplemented. Identify
// only our PID + application-class window using documented X11 properties;
// never inspect memory, hook input, or map/unmap another application's window.
fn own_window(
    connection: &x11rb::rust_connection::RustConnection,
    root: u32,
) -> anyhow::Result<u32> {
    let pid_atom = connection.intern_atom(false, b"_NET_WM_PID")?.reply()?.atom;
    let mut pending = vec![(root, 0)];
    while let Some((parent, depth)) = pending.pop() {
        let Some(tree) = live_window(connection.query_tree(parent)?.reply())? else {
            continue;
        };
        for id in tree.children {
            let Some(pid) = live_window(
                connection
                    .get_property(false, id, pid_atom, xproto::AtomEnum::CARDINAL, 0, 1)?
                    .reply(),
            )?
            else {
                continue;
            };
            if pid.value32().and_then(|mut values| values.next()) == Some(std::process::id()) {
                let Some(class) = live_window(
                    connection
                        .get_property(
                            false,
                            id,
                            xproto::AtomEnum::WM_CLASS,
                            xproto::AtomEnum::STRING,
                            0,
                            256,
                        )?
                        .reply(),
                )?
                else {
                    continue;
                };
                if class
                    .value
                    .split(|byte| *byte == 0)
                    .any(|value| value == b"DrStopor.NinjutsoBatteryTray")
                {
                    return Ok(id);
                }
            }
            if depth < 3 {
                pending.push((id, depth + 1));
            }
        }
    }
    anyhow::bail!("Could not find this process's Ninjutso status window")
}

pub(crate) fn cursor_position() -> Point {
    let Ok((connection, screen)) = x11rb::connect(None) else {
        return Point { x: 0, y: 0 };
    };
    let root = connection.setup().roots[screen].root;
    connection
        .query_pointer(root)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .map_or(Point { x: 0, y: 0 }, |p| Point {
            x: i32::from(p.root_x),
            y: i32::from(p.root_y),
        })
}

fn monitor_work_area(
    connection: &x11rb::rust_connection::RustConnection,
    screen: &xproto::Screen,
    anchor: Point,
) -> anyhow::Result<(Bounds, Bounds)> {
    let mut monitor = Bounds::new(
        0,
        0,
        i32::from(screen.width_in_pixels),
        i32::from(screen.height_in_pixels),
    );
    if let Ok(cookie) = connection.randr_get_monitors(screen.root, true)
        && let Ok(reply) = cookie.reply()
        && let Some(m) = reply.monitors.iter().find(|m| {
            anchor.x >= i32::from(m.x)
                && anchor.y >= i32::from(m.y)
                && anchor.x < i32::from(m.x) + i32::from(m.width)
                && anchor.y < i32::from(m.y) + i32::from(m.height)
        })
    {
        monitor = Bounds::new(
            i32::from(m.x),
            i32::from(m.y),
            i32::from(m.x) + i32::from(m.width),
            i32::from(m.y) + i32::from(m.height),
        );
    }
    let mut work = monitor;
    let atom = connection
        .intern_atom(false, b"_NET_WORKAREA")?
        .reply()?
        .atom;
    if let Ok(reply) = connection
        .get_property(false, screen.root, atom, xproto::AtomEnum::CARDINAL, 0, 4)?
        .reply()
        && let Some(values) = reply.value32()
    {
        let values: Vec<_> = values.collect();
        if values.len() == 4 {
            let left = work.left.max(values[0] as i32);
            let top = work.top.max(values[1] as i32);
            let right = work.right.min(values[0].saturating_add(values[2]) as i32);
            let bottom = work.bottom.min(values[1].saturating_add(values[3]) as i32);
            if right > left && bottom > top {
                work = Bounds::new(left, top, right, bottom);
            }
        }
    }
    Ok((monitor, work))
}

pub(crate) fn context_menu_anchor() -> Point {
    let result = (|| -> anyhow::Result<Point> {
        let (connection, screen) = x11rb::connect(None)?;
        let screen = &connection.setup().roots[screen];
        // XWayland can retain the last pointer position inside our X11 card
        // while GNOME's native menu is being used. Use it only to select the
        // monitor, NEVER as the next popup's position (e.g. its old close button).
        let (monitor, work) = monitor_work_area(&connection, screen, cursor_position())?;
        let right = work.right - 16;
        let top = work.top + 16;
        let edges = [
            (
                work.top - monitor.top,
                Point {
                    x: right,
                    y: work.top - 1,
                },
            ),
            (
                monitor.bottom - work.bottom,
                Point {
                    x: right,
                    y: work.bottom + 1,
                },
            ),
            (
                work.left - monitor.left,
                Point {
                    x: work.left - 1,
                    y: top,
                },
            ),
            (
                monitor.right - work.right,
                Point {
                    x: work.right + 1,
                    y: top,
                },
            ),
        ];
        Ok(edges
            .into_iter()
            .filter(|(reserved, _)| *reserved > 0)
            .max_by_key(|(reserved, _)| *reserved)
            .map_or(
                Point {
                    x: right,
                    y: work.top,
                },
                |(_, anchor)| anchor,
            ))
    })();
    result.unwrap_or_else(|error| {
        log::warn!("Could not locate tray panel for status menu: {error}");
        Point { x: 16, y: 16 }
    })
}

pub(crate) fn show_flyout(
    window: &mut Window,
    mut anchor: Point,
    _width: f32,
    _height: f32,
) -> Result<(), String> {
    let result = (|| -> anyhow::Result<()> {
        log::debug!("Presenting retained GPUI card");
        let (connection, screen) = x11rb::connect(None)?;
        let screen = &connection.setup().roots[screen];
        let id = own_window(&connection, screen.root)?;
        log::debug!("Own X11 window: {id}");
        // titlebar: None is not honored by every X11 window manager in GPUI
        // 0.2.2. A documented Motif hint disables only this window's decoration.
        let motif = connection
            .intern_atom(false, b"_MOTIF_WM_HINTS")?
            .reply()?
            .atom;
        connection
            .change_property32(
                xproto::PropMode::REPLACE,
                id,
                motif,
                motif,
                &[2, 0, 0, 0, 0],
            )?
            .check()?;
        let geometry = connection.get_geometry(id)?.reply()?;
        if anchor == (Point { x: 0, y: 0 }) {
            anchor = cursor_position();
        }
        let (_, work) = monitor_work_area(&connection, screen, anchor)?;
        let point = position(
            anchor,
            work,
            i32::from(geometry.width),
            i32::from(geometry.height),
        );
        connection
            .configure_window(
                id,
                &xproto::ConfigureWindowAux::new()
                    .x(point.x)
                    .y(point.y)
                    .stack_mode(xproto::StackMode::ABOVE),
            )?
            .check()?;
        connection.map_window(id)?.check()?;
        window.activate_window();
        window.refresh();
        // GPUI 0.2.2's synchronous XCB activation can buffer MapNotify before
        // its event loop starts. An actual exposure wakes that connection so
        // the queued map event starts GPUI's own frame clock (no repaint timer).
        connection.clear_area(true, id, 0, 0, 0, 0)?.check()?;
        connection.flush()?;
        log::debug!("Card activation and repaint requested");
        Ok(())
    })();
    result.map_err(|e| format!("Could not present status card: {e}"))
}

pub(crate) fn hide_flyout(_window: &mut Window) {
    let result = (|| -> anyhow::Result<()> {
        let (connection, screen) = x11rb::connect(None)?;
        let id = own_window(&connection, connection.setup().roots[screen].root)?;
        // This is a short-lived connection. Wait for the server to execute the
        // unmap before disconnecting; a flushed request alone is not an ack.
        connection.unmap_window(id)?.check()?;
        connection.flush()?;
        Ok(())
    })();
    if let Err(error) = result {
        log::error!("Could not hide status card: {error}");
    }
}

fn position(anchor: Point, work: Bounds, width: i32, height: i32) -> Point {
    let left = work.left + 8;
    let top = work.top + 8;
    Point {
        x: (anchor.x - width / 2).clamp(left, (work.right - width - 8).max(left)),
        y: if anchor.y - height - 12 >= top {
            anchor.y - height - 12
        } else {
            anchor.y + 12
        }
        .clamp(top, (work.bottom - height - 8).max(top)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scaled_cards_stay_inside_the_active_monitors_work_area() {
        for work in [
            Bounds::new(0, 32, 1920, 1080),
            Bounds::new(-1920, 0, 0, 1080),
        ] {
            for anchor in [
                Point {
                    x: work.left,
                    y: work.top,
                },
                Point {
                    x: work.right,
                    y: work.bottom,
                },
            ] {
                for scale in [1, 2] {
                    let point = position(anchor, work, 304 * scale, 156 * scale);
                    assert!(point.x >= work.left + 8 && point.x + 304 * scale <= work.right - 8);
                    assert!(point.y >= work.top + 8 && point.y + 156 * scale <= work.bottom - 8);
                }
            }
        }
    }
}
