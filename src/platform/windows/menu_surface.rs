//! Final native-menu decoration, isolated from the shared menu/palette model.
use std::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Threading::{GetCurrentProcessId, GetCurrentThreadId};
use windows::Win32::UI::Shell::{
    DefSubclassProc, GetWindowSubclass, RemoveWindowSubclass, SetWindowSubclass,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use super::context_menu::{color, scaled};
use super::native::GdiObject;
use crate::presentation::theme::MenuPalette;

#[derive(Clone, Copy)]
struct Surface {
    palette: MenuPalette,
    dpi: u32,
}
const SUBCLASS_ID: usize = 0x4e42544d;

/// Native popup windows can be cached. Do not leave custom styling attached
/// after a menu session, especially before a later accessibility-mode menu.
pub(super) fn clear() {
    unsafe extern "system" fn detach(hwnd: HWND, _: LPARAM) -> windows::core::BOOL {
        let mut data = 0;
        if unsafe { GetWindowSubclass(hwnd, Some(window_proc), SUBCLASS_ID, Some(&mut data)) }
            .as_bool()
            && unsafe { RemoveWindowSubclass(hwnd, Some(window_proc), SUBCLASS_ID) }.as_bool()
        {
            drop(unsafe { Box::from_raw(data as *mut Surface) });
        }
        true.into()
    }
    let _ = unsafe { EnumThreadWindows(GetCurrentThreadId(), Some(detach), LPARAM(0)) };
}

pub(super) fn configure(hwnd: HWND, palette: MenuPalette, dpi: u32) {
    let mut process = 0;
    // Never subclass another process's or thread's windows (including Explorer).
    if hwnd.is_invalid()
        || unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut process)) != GetCurrentThreadId()
                || process != GetCurrentProcessId()
        }
    {
        return;
    }
    let mut data = 0;
    if unsafe { GetWindowSubclass(hwnd, Some(window_proc), SUBCLASS_ID, Some(&mut data)) }.as_bool()
    {
        // The native menu implementation may reuse popup windows between opens.
        unsafe {
            *(data as *mut Surface) = Surface { palette, dpi };
        }
    } else {
        let data = Box::into_raw(Box::new(Surface { palette, dpi }));
        if !unsafe { SetWindowSubclass(hwnd, Some(window_proc), SUBCLASS_ID, data as usize) }
            .as_bool()
        {
            drop(unsafe { Box::from_raw(data) });
            return;
        }
        // configure can run during the popup's very first WM_PAINT.
        let _ = unsafe { PostMessageW(Some(hwnd), WM_PAINT, WPARAM(0), LPARAM(0)) };
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    data: usize,
) -> LRESULT {
    if message == WM_NCDESTROY {
        unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(window_proc), id);
        }
        drop(unsafe { Box::from_raw(data as *mut Surface) });
        return unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
    }
    // Copy before entering Win32: drawing can synchronously reenter the window.
    let surface = unsafe { *(data as *const Surface) };
    let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
    if matches!(message, WM_PAINT | WM_PRINTCLIENT | WM_MOUSEMOVE) {
        let dc = if message == WM_PRINTCLIENT {
            HDC(wparam.0 as *mut c_void)
        } else {
            unsafe { GetDC(Some(hwnd)) }
        };
        if !dc.is_invalid() {
            surface.finish_arrows(hwnd, dc);
            if message != WM_PRINTCLIENT {
                unsafe {
                    ReleaseDC(Some(hwnd), dc);
                }
            }
        }
    }
    result
}

impl Surface {
    fn finish_arrows(self, hwnd: HWND, dc: HDC) {
        let mut menu = MENUBARINFO {
            cbSize: size_of::<MENUBARINFO>() as u32,
            ..Default::default()
        };
        if unsafe { GetMenuBarInfo(hwnd, OBJID_CLIENT, 0, &mut menu) }.is_err() {
            return;
        }
        let background = GdiObject(HGDIOBJ(
            unsafe { CreateSolidBrush(color(self.palette.background)) }.0,
        ));
        let highlight = GdiObject(HGDIOBJ(
            unsafe { CreateSolidBrush(color(self.palette.highlight)) }.0,
        ));
        let pen = GdiObject(HGDIOBJ(
            unsafe {
                CreatePen(
                    PS_SOLID,
                    scaled(1, self.dpi).max(1),
                    color(self.palette.foreground),
                )
            }
            .0,
        ));
        for position in 0..unsafe { GetMenuItemCount(Some(menu.hMenu)) }.max(0) as u32 {
            let mut info = MENUITEMINFOW {
                cbSize: size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_SUBMENU | MIIM_STATE,
                ..Default::default()
            };
            if unsafe { GetMenuItemInfoW(menu.hMenu, position, true, &mut info) }.is_err()
                || info.hSubMenu.is_invalid()
            {
                continue;
            }
            let mut rect = RECT::default();
            if unsafe { GetMenuItemRect(None, menu.hMenu, position, &mut rect) }.is_err() {
                continue;
            }
            let mut points = [
                POINT {
                    x: rect.left,
                    y: rect.top,
                },
                POINT {
                    x: rect.right,
                    y: rect.bottom,
                },
            ];
            unsafe {
                MapWindowPoints(None, Some(hwnd), &mut points);
            }
            rect = RECT {
                left: points[0].x,
                top: points[0].y,
                right: points[1].x,
                bottom: points[1].y,
            };
            let saved = unsafe { SaveDC(dc) };
            if saved == 0 {
                continue;
            }
            unsafe {
                // Paint only the arrow column, preserving the menu's text/checks.
                IntersectClipRect(
                    dc,
                    rect.right - scaled(24, self.dpi),
                    rect.top,
                    rect.right,
                    rect.bottom,
                );
                FillRect(dc, &rect, HBRUSH(background.0.0));
                if info.fState.contains(MFS_HILITE) {
                    SelectObject(dc, highlight.0);
                    SelectObject(dc, GetStockObject(NULL_PEN));
                    let _ = RoundRect(
                        dc,
                        rect.left + scaled(3, self.dpi),
                        rect.top + scaled(1, self.dpi),
                        rect.right - scaled(3, self.dpi),
                        rect.bottom - scaled(1, self.dpi),
                        scaled(8, self.dpi),
                        scaled(8, self.dpi),
                    );
                }
                SelectObject(dc, pen.0);
                let x = rect.right - scaled(10, self.dpi);
                let y = (rect.top + rect.bottom) / 2;
                let _ = Polyline(
                    dc,
                    &[
                        POINT {
                            x: x - scaled(2, self.dpi),
                            y: y - scaled(3, self.dpi),
                        },
                        POINT {
                            x: x + scaled(1, self.dpi),
                            y,
                        },
                        POINT {
                            x: x - scaled(2, self.dpi),
                            y: y + scaled(3, self.dpi),
                        },
                    ],
                );
                let _ = RestoreDC(dc, saved);
            }
        }
    }
}
