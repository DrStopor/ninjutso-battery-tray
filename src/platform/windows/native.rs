use std::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, CreateRoundRectRgn,
    DIB_RGB_COLORS, DeleteObject, GetMonitorInfoW, HGDIOBJ, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MonitorFromPoint, SetWindowRgn,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, GWL_EXSTYLE, GWL_STYLE, GetCursorPos, GetWindowLongPtrW, HICON,
    HWND_TOPMOST, ICONINFO, MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MessageBoxW, SWP_FRAMECHANGED,
    SWP_SHOWWINDOW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, WS_CLIPCHILDREN,
    WS_CLIPSIBLINGS, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    HTCLIENT, SW_HIDE, ShowWindow, WM_NCCALCSIZE, WM_NCDESTROY, WM_NCHITTEST,
};
use windows::core::PCWSTR;

use crate::presentation::icons::ICON_SIZE;
use crate::presentation::layout::Point;
use crate::presentation::theme::Rgba;

pub(super) struct GdiObject(pub HGDIOBJ);
impl Drop for GdiObject {
    fn drop(&mut self) {
        let _ = unsafe { DeleteObject(self.0) };
    }
}

pub(crate) fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

pub(crate) fn cursor_position() -> Point {
    let mut point = POINT::default();
    let _ = unsafe { GetCursorPos(&mut point) };
    Point {
        x: point.x,
        y: point.y,
    }
}

pub(crate) fn show_error(message: &str) {
    message_box(message, true);
}
pub(crate) fn show_information(message: &str) {
    message_box(message, false);
}

fn message_box(message: &str, error: bool) {
    let message = wide(message);
    let title = wide(crate::APP_NAME);
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK
                | if error {
                    MB_ICONERROR
                } else {
                    MB_ICONINFORMATION
                },
        );
    }
}

pub(crate) fn place_flyout(
    raw: isize,
    anchor: Point,
    width: f32,
    height: f32,
) -> Result<(), String> {
    let hwnd = HWND(raw as *mut c_void);
    let monitor = unsafe {
        MonitorFromPoint(
            POINT {
                x: anchor.x,
                y: anchor.y,
            },
            MONITOR_DEFAULTTONEAREST,
        )
    };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let has_work = unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool();
    let mut dpi = 96;
    let mut dpi_y = 96;
    let _ = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut dpi_y) };
    let scale = dpi as f32 / 96.0;
    let width = (width * scale).round() as i32;
    let height = (height * scale).round() as i32;
    let margin = (8.0 * scale).round() as i32;
    let mut x = anchor.x - width + 24;
    let mut y = anchor.y - height - 18;
    if has_work {
        let work = info.rcWork;
        if anchor.y >= work.bottom {
            y = work.bottom - height - margin;
        } else if anchor.y < work.top {
            y = work.top + margin;
        } else if anchor.x >= work.right {
            x = work.right - width - margin;
            y = anchor.y - height + 24;
        } else if anchor.x < work.left {
            x = work.left + margin;
            y = anchor.y - height + 24;
        }
        x = x.clamp(
            work.left + margin,
            (work.right - width - margin).max(work.left + margin),
        );
        y = y.clamp(
            work.top + margin,
            (work.bottom - height - margin).max(work.top + margin),
        );
    }
    unsafe {
        // GPUI 0.2.2 applies normal frame insets even to its PopUp windows.
        // This configures only our own window, on its owning thread.
        if !SetWindowSubclass(hwnd, Some(flyout_proc), 1, 0).as_bool() {
            return Err("Could not configure the frameless GPUI window".into());
        }
        let extended = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            (extended | WS_EX_TOOLWINDOW.0 as isize) & !(WS_EX_APPWINDOW.0 as isize),
        );
        SetWindowLongPtrW(
            hwnd,
            GWL_STYLE,
            (WS_POPUP | WS_CLIPCHILDREN | WS_CLIPSIBLINGS).0 as isize,
        );
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            width,
            height,
            SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        )
        .map_err(|error| error.to_string())?;
        let preference = DWMWCP_ROUND;
        if DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preference as *const _ as *const c_void,
            size_of_val(&preference) as u32,
        )
        .is_err()
        {
            let radius = (14.0 * scale).round() as i32;
            let region = CreateRoundRectRgn(0, 0, width + 1, height + 1, radius, radius);
            if SetWindowRgn(hwnd, Some(region), true) == 0 {
                let _ = DeleteObject(HGDIOBJ(region.0));
            }
        }
        let _ = SetForegroundWindow(hwnd);
    }
    Ok(())
}

unsafe extern "system" fn flyout_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    _: usize,
) -> LRESULT {
    match message {
        WM_NCCALCSIZE => LRESULT(0),
        WM_NCHITTEST => LRESULT(HTCLIENT as isize),
        WM_NCDESTROY => {
            let _ = unsafe { RemoveWindowSubclass(hwnd, Some(flyout_proc), id) };
            unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
        }
        _ => unsafe { DefSubclassProc(hwnd, message, wparam, lparam) },
    }
}

pub(crate) fn hide_flyout(raw: isize) {
    let _ = unsafe { ShowWindow(HWND(raw as *mut c_void), SW_HIDE) };
}

pub(crate) fn create_tray_icon(pixels: &[Rgba]) -> Result<HICON, String> {
    if pixels.len() != (ICON_SIZE * ICON_SIZE) as usize {
        return Err("Invalid tray icon dimensions".into());
    }
    let bgra: Vec<u8> = pixels
        .iter()
        .flat_map(|pixel| [pixel.b, pixel.g, pixel.r, pixel.a])
        .collect();
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: ICON_SIZE,
            biHeight: -ICON_SIZE,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: bgra.len() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    let bitmap = unsafe { CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0) }
        .map_err(|error| error.to_string())?;
    if bits.is_null() {
        let _ = unsafe { DeleteObject(HGDIOBJ(bitmap.0)) };
        return Err("CreateDIBSection returned no pixels".into());
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, bgra.len());
    }
    let mask = vec![0u8; (((ICON_SIZE + 15) / 16 * 2) * ICON_SIZE) as usize];
    let mask_bitmap = unsafe {
        CreateBitmap(
            ICON_SIZE,
            ICON_SIZE,
            1,
            1,
            Some(mask.as_ptr() as *const c_void),
        )
    };
    if mask_bitmap.is_invalid() {
        let _ = unsafe { DeleteObject(HGDIOBJ(bitmap.0)) };
        return Err("Could not create the tray icon mask".into());
    }
    let result = unsafe {
        CreateIconIndirect(&ICONINFO {
            fIcon: true.into(),
            hbmColor: bitmap,
            hbmMask: mask_bitmap,
            ..Default::default()
        })
    };
    unsafe {
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteObject(HGDIOBJ(mask_bitmap.0));
    }
    result.map_err(|error| error.to_string())
}

pub(crate) fn icon_anchor(hwnd: HWND, uid: u32) -> Point {
    use windows::Win32::UI::Shell::{NOTIFYICONIDENTIFIER, Shell_NotifyIconGetRect};
    let identifier = NOTIFYICONIDENTIFIER {
        cbSize: size_of::<NOTIFYICONIDENTIFIER>() as u32,
        hWnd: hwnd,
        uID: uid,
        ..Default::default()
    };
    if let Ok(rect) = unsafe { Shell_NotifyIconGetRect(&identifier) } {
        Point {
            x: (rect.left + rect.right) / 2,
            y: (rect.top + rect.bottom) / 2,
        }
    } else {
        cursor_position()
    }
}
