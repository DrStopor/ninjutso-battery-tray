//! Narrow OS boundary for the shared GPUI frontend. Native handles stay here.
#[cfg(windows)]
use gpui::Window;
#[cfg(windows)]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

#[cfg(windows)]
use crate::presentation::layout::Point;

#[cfg(windows)]
pub(crate) use super::windows::native::{cursor_position, show_error, show_information};
#[cfg(windows)]
pub(crate) use super::windows::{
    instance::SingleInstanceGuard, load_settings, read_mouse_battery, save_settings,
    tray::TrayService,
};

#[cfg(target_os = "linux")]
pub(crate) use super::linux::surface::{cursor_position, hide_flyout, show_flyout};
#[cfg(target_os = "linux")]
pub(crate) use super::linux::{
    instance::SingleInstanceGuard, load_settings, read_mouse_battery, save_settings,
    tray::TrayService,
};

#[cfg(target_os = "linux")]
pub(crate) fn show_error(message: &str) {
    eprintln!("Ninjutso Battery Tray: {message}");
}
#[cfg(target_os = "linux")]
pub(crate) fn show_information(message: &str) {
    eprintln!("{message}");
}

#[cfg(windows)]
pub(crate) fn show_flyout(
    window: &mut Window,
    anchor: Point,
    width: f32,
    height: f32,
) -> Result<(), String> {
    match HasWindowHandle::window_handle(window)
        .map_err(|e| e.to_string())?
        .as_raw()
    {
        RawWindowHandle::Win32(handle) => {
            super::windows::native::place_flyout(handle.hwnd.get(), anchor, width, height)
        }
        _ => Err("GPUI did not create a Win32 window".into()),
    }
}

#[cfg(windows)]
pub(crate) fn hide_flyout(window: &mut Window) {
    if let Ok(raw) = HasWindowHandle::window_handle(window)
        && let RawWindowHandle::Win32(handle) = raw.as_raw()
    {
        super::windows::native::hide_flyout(handle.hwnd.get());
    }
}
