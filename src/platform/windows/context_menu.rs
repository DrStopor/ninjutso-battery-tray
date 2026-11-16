//! Win32 adapter for the shared menu model. No device access or settings writes.
//!
//! Documented owner drawing keeps native submenus, dismissal and keyboard/MSAA
//! behavior. No process-wide theme changes, private UxTheme exports or hooks.
use std::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Accessibility::{
    HCF_HIGHCONTRASTON, HIGHCONTRASTW, MSAA_MENU_SIG, MSAAMENUINFO,
};
use windows::Win32::UI::Controls::{
    DRAWITEMSTRUCT, MEASUREITEMSTRUCT, ODS_CHECKED, ODS_DISABLED, ODS_GRAYED, ODS_SELECTED,
    ODT_MENU,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI, SystemParametersInfoForDpi};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PWSTR;

use super::menu_surface;
use super::native::{GdiObject, wide};
use crate::i18n::Language;
use crate::presentation::layout::Point;
use crate::presentation::menu::{MenuCommand, MenuEntry, MenuMark};
use crate::presentation::theme::{MenuPalette, Rgb};
use crate::settings::ThemePreference;

const REFRESH: u32 = 1001;
const PERCENTAGE: u32 = 1002;
const EXIT: u32 = 1003;
const LANGUAGE_FIRST: u32 = 1100;
const THEME_FIRST: u32 = 1200;

pub(super) fn command_id(command: MenuCommand) -> u32 {
    match command {
        MenuCommand::Refresh => REFRESH,
        MenuCommand::TogglePercentage => PERCENTAGE,
        MenuCommand::Quit => EXIT,
        MenuCommand::SetLanguage(language) => {
            LANGUAGE_FIRST + Language::ALL.iter().position(|&v| v == language).unwrap() as u32
        }
        MenuCommand::SetTheme(theme) => {
            THEME_FIRST
                + ThemePreference::ALL
                    .iter()
                    .position(|&v| v == theme)
                    .unwrap() as u32
        }
    }
}

pub(super) fn command_from_id(id: u32) -> Option<MenuCommand> {
    match id {
        REFRESH => Some(MenuCommand::Refresh),
        PERCENTAGE => Some(MenuCommand::TogglePercentage),
        EXIT => Some(MenuCommand::Quit),
        value if (LANGUAGE_FIRST..LANGUAGE_FIRST + Language::ALL.len() as u32).contains(&value) => {
            Some(MenuCommand::SetLanguage(
                Language::ALL[(value - LANGUAGE_FIRST) as usize],
            ))
        }
        value
            if (THEME_FIRST..THEME_FIRST + ThemePreference::ALL.len() as u32).contains(&value) =>
        {
            Some(MenuCommand::SetTheme(
                ThemePreference::ALL[(value - THEME_FIRST) as usize],
            ))
        }
        _ => None,
    }
}

struct OwnedMenu(HMENU);
impl OwnedMenu {
    fn new() -> windows::core::Result<Self> {
        unsafe { CreatePopupMenu().map(Self) }
    }
}
impl Drop for OwnedMenu {
    fn drop(&mut self) {
        let _ = unsafe { DestroyMenu(self.0) };
    }
}

pub(super) fn color(rgb: Rgb) -> COLORREF {
    COLORREF(u32::from(rgb.r) | (u32::from(rgb.g) << 8) | (u32::from(rgb.b) << 16))
}
pub(super) fn scaled(value: i32, dpi: u32) -> i32 {
    ((i64::from(value) * i64::from(dpi) + 48) / 96) as i32
}

struct Painting {
    font: GdiObject,
    background: GdiObject,
    palette: MenuPalette,
    dpi: u32,
}
impl Painting {
    fn new(anchor: Point, palette: MenuPalette) -> Option<Self> {
        let monitor = unsafe {
            MonitorFromPoint(
                POINT {
                    x: anchor.x,
                    y: anchor.y,
                },
                MONITOR_DEFAULTTONEAREST,
            )
        };
        let (mut dpi, mut dpi_y) = (96, 96);
        let _ = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut dpi_y) };
        let mut metrics = NONCLIENTMETRICSW {
            cbSize: size_of::<NONCLIENTMETRICSW>() as u32,
            ..Default::default()
        };
        unsafe {
            SystemParametersInfoForDpi(
                SPI_GETNONCLIENTMETRICS.0,
                metrics.cbSize,
                Some(&mut metrics as *mut _ as *mut c_void),
                0,
                dpi,
            )
            .ok()?;
        }
        // Respect a larger system menu font (including accessibility text sizing).
        metrics.lfMenuFont.lfHeight = -metrics.lfMenuFont.lfHeight.abs().max(scaled(13, dpi));
        let font = unsafe { CreateFontIndirectW(&metrics.lfMenuFont) };
        if font.is_invalid() {
            return None;
        }
        let font = GdiObject(HGDIOBJ(font.0));
        let background = unsafe { CreateSolidBrush(color(palette.background)) };
        if background.is_invalid() {
            return None;
        }
        Some(Self {
            font,
            background: GdiObject(HGDIOBJ(background.0)),
            palette,
            dpi,
        })
    }
}

// MSAA requires this exact first field and C layout. Box addresses and UTF-16
// buffers stay stable until AFTER DestroyMenu, including all nested modal loops.
#[repr(C)]
struct DrawItem {
    accessible: MSAAMENUINFO,
    label: Vec<u16>,
    font: HFONT,
    background: HBRUSH,
    palette: MenuPalette,
    dpi: u32,
    size: SIZE,
    separator: bool,
    radio: bool,
}
impl DrawItem {
    fn new(label: &str, separator: bool, radio: bool, paint: &Painting) -> Box<Self> {
        let mut label = wide(label);
        let accessible = MSAAMENUINFO {
            dwMSAASignature: MSAA_MENU_SIG as u32,
            cchWText: (label.len() - 1) as u32,
            pszWText: PWSTR(label.as_mut_ptr()),
        };
        let font = HFONT(paint.font.0.0);
        let mut size = SIZE::default();
        let dc = unsafe { GetDC(None) };
        if !dc.is_invalid() {
            unsafe {
                let old = SelectObject(dc, paint.font.0);
                let _ = GetTextExtentPoint32W(dc, &label[..label.len() - 1], &mut size);
                SelectObject(dc, old);
                ReleaseDC(None, dc);
            }
        }
        size.cx = (size.cx + scaled(64, paint.dpi)).max(scaled(156, paint.dpi));
        size.cy = if separator {
            scaled(9, paint.dpi)
        } else {
            (size.cy + scaled(12, paint.dpi)).max(scaled(30, paint.dpi))
        };
        Box::new(Self {
            accessible,
            label,
            font,
            background: HBRUSH(paint.background.0.0),
            palette: paint.palette,
            dpi: paint.dpi,
            size,
            separator,
            radio,
        })
    }

    fn draw(&self, item: &DRAWITEMSTRUCT) {
        let dc = item.hDC;
        // Windows draws submenu arrows after WM_DRAWITEM using system colors.
        // Finish those glyphs in our own popup's paint callback, not via a hook
        // or a process-wide/private dark-mode setting.
        menu_surface::configure(unsafe { WindowFromDC(dc) }, self.palette, self.dpi);
        let saved = unsafe { SaveDC(dc) };
        if saved == 0 {
            return;
        }
        let rect = item.rcItem;
        unsafe {
            FillRect(dc, &rect, self.background);
            if self.separator {
                let brush = GdiObject(HGDIOBJ(CreateSolidBrush(color(self.palette.border)).0));
                let y = (rect.top + rect.bottom) / 2;
                FillRect(
                    dc,
                    &RECT {
                        left: rect.left + scaled(12, self.dpi),
                        right: rect.right - scaled(12, self.dpi),
                        top: y,
                        bottom: y + 1,
                    },
                    HBRUSH(brush.0.0),
                );
            } else {
                if item.itemState.0 & ODS_SELECTED.0 != 0 {
                    let brush =
                        GdiObject(HGDIOBJ(CreateSolidBrush(color(self.palette.highlight)).0));
                    SelectObject(dc, brush.0);
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
                    // Deselect before deleting this brush.
                    SelectObject(dc, HGDIOBJ(self.background.0));
                }
                let foreground =
                    color(if item.itemState.0 & (ODS_DISABLED.0 | ODS_GRAYED.0) != 0 {
                        self.palette.muted
                    } else {
                        self.palette.foreground
                    });
                SelectObject(dc, HGDIOBJ(self.font.0));
                SetBkMode(dc, TRANSPARENT);
                SetTextColor(dc, foreground);
                let mut text_rect = RECT {
                    left: rect.left + scaled(32, self.dpi),
                    right: rect.right - scaled(26, self.dpi),
                    ..rect
                };
                let mut text = self.label[..self.label.len() - 1].to_vec();
                DrawTextW(
                    dc,
                    &mut text,
                    &mut text_rect,
                    DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
                );
                if item.itemState.0 & ODS_CHECKED.0 != 0 {
                    let x = rect.left + scaled(16, self.dpi);
                    let y = (rect.top + rect.bottom) / 2;
                    if self.radio {
                        let brush = GdiObject(HGDIOBJ(CreateSolidBrush(foreground).0));
                        SelectObject(dc, brush.0);
                        SelectObject(dc, GetStockObject(NULL_PEN));
                        let r = scaled(3, self.dpi);
                        let _ = Ellipse(dc, x - r, y - r, x + r, y + r);
                        SelectObject(dc, HGDIOBJ(self.background.0));
                    } else {
                        let pen = GdiObject(HGDIOBJ(
                            CreatePen(PS_SOLID, scaled(2, self.dpi), foreground).0,
                        ));
                        let old = SelectObject(dc, pen.0);
                        let _ = Polyline(
                            dc,
                            &[
                                POINT {
                                    x: x - scaled(4, self.dpi),
                                    y,
                                },
                                POINT {
                                    x: x - scaled(1, self.dpi),
                                    y: y + scaled(3, self.dpi),
                                },
                                POINT {
                                    x: x + scaled(5, self.dpi),
                                    y: y - scaled(4, self.dpi),
                                },
                            ],
                        );
                        SelectObject(dc, old);
                    }
                }
            }
            let _ = RestoreDC(dc, saved);
        }
    }
}

/// Falls back to the untouched system menu for high contrast/screen readers.
fn needs_system_menu() -> bool {
    let mut contrast = HIGHCONTRASTW {
        cbSize: size_of::<HIGHCONTRASTW>() as u32,
        ..Default::default()
    };
    let mut screen_reader = 0i32;
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            contrast.cbSize,
            Some(&mut contrast as *mut _ as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        let _ = SystemParametersInfoW(
            SPI_GETSCREENREADER,
            0,
            Some(&mut screen_reader as *mut _ as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
    contrast.dwFlags.contains(HCF_HIGHCONTRASTON) || screen_reader != 0
}

struct ContextMenu {
    // Field order matters: destroy native menus before releasing item data/GDI.
    root: OwnedMenu,
    // Separate stable allocations are required by dwItemData, not Vec entries.
    #[allow(clippy::vec_box)]
    items: Vec<Box<DrawItem>>,
    paint: Option<Painting>,
}
impl ContextMenu {
    fn build(
        entries: &[MenuEntry],
        anchor: Point,
        palette: MenuPalette,
    ) -> windows::core::Result<Self> {
        let mut menu = Self {
            root: OwnedMenu::new()?,
            items: Vec::new(),
            paint: if needs_system_menu() {
                None
            } else {
                Painting::new(anchor, palette)
            },
        };
        menu.populate(menu.root.0, entries)?;
        Ok(menu)
    }

    fn populate(&mut self, menu: HMENU, entries: &[MenuEntry]) -> windows::core::Result<()> {
        if let Some(paint) = &self.paint {
            unsafe {
                SetMenuInfo(
                    menu,
                    &MENUINFO {
                        cbSize: size_of::<MENUINFO>() as u32,
                        fMask: MIM_BACKGROUND | MIM_STYLE,
                        dwStyle: MNS_NOCHECK,
                        hbrBack: HBRUSH(paint.background.0.0),
                        ..Default::default()
                    },
                )?;
            }
        }
        for (position, entry) in entries.iter().enumerate() {
            let mut child = None;
            let (label, command, mark, separator) = match entry {
                MenuEntry::Action {
                    label,
                    command,
                    mark,
                } => (*label, command_id(*command), *mark, false),
                MenuEntry::Submenu { label, entries } => {
                    let submenu = OwnedMenu::new()?;
                    self.populate(submenu.0, entries)?;
                    child = Some(submenu);
                    (*label, 0, MenuMark::None, false)
                }
                MenuEntry::Separator => ("", 0, MenuMark::None, true),
            };
            let mut wide_label = wide(label);
            let mut info = MENUITEMINFOW {
                cbSize: size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_FTYPE | MIIM_STATE | MIIM_ID | MIIM_STRING | MIIM_SUBMENU,
                fType: if separator { MFT_SEPARATOR } else { MFT_STRING },
                fState: if matches!(mark, MenuMark::Check(true) | MenuMark::Radio(true)) {
                    MFS_CHECKED
                } else {
                    MFS_UNCHECKED
                },
                wID: command,
                hSubMenu: child.as_ref().map_or(HMENU::default(), |menu| menu.0),
                dwTypeData: PWSTR(wide_label.as_mut_ptr()),
                cch: (wide_label.len() - 1) as u32,
                ..Default::default()
            };
            if matches!(mark, MenuMark::Radio(_)) {
                info.fType |= MFT_RADIOCHECK;
            }
            if let Some(paint) = &self.paint {
                let data =
                    DrawItem::new(label, separator, matches!(mark, MenuMark::Radio(_)), paint);
                info.fType |= MFT_OWNERDRAW;
                info.fMask |= MIIM_DATA;
                info.dwItemData = &*data as *const DrawItem as usize;
                self.items.push(data);
            }
            unsafe {
                InsertMenuItemW(menu, position as u32, true, &info)?;
            }
            // Parent owns this submenu only after successful insertion.
            if let Some(child) = child {
                std::mem::forget(child);
            }
        }
        Ok(())
    }
}

pub(super) fn show(
    hwnd: HWND,
    entries: &[MenuEntry],
    anchor: Point,
    palette: MenuPalette,
) -> Option<MenuCommand> {
    let menu = ContextMenu::build(entries, anchor, palette).ok()?;
    let command = unsafe {
        let _ = SetForegroundWindow(hwnd);
        TrackPopupMenu(
            menu.root.0,
            TPM_RIGHTBUTTON | TPM_RETURNCMD,
            anchor.x,
            anchor.y,
            Some(0),
            hwnd,
            None,
        )
        .0 as u32
    };
    menu_surface::clear();
    let _ = unsafe { PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) };
    command_from_id(command)
}

/// Only call with OS-delivered owner-draw messages for menus built above.
pub(super) unsafe fn handle_message(
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<LRESULT> {
    if wparam.0 != 0 || lparam.0 == 0 {
        return None;
    }
    match message {
        WM_MEASUREITEM => {
            let info = unsafe { &mut *(lparam.0 as *mut MEASUREITEMSTRUCT) };
            if info.CtlType != ODT_MENU || info.itemData == 0 {
                return None;
            }
            let item = unsafe { &*(info.itemData as *const DrawItem) };
            info.itemWidth = item.size.cx as u32;
            info.itemHeight = item.size.cy as u32;
            Some(LRESULT(1))
        }
        WM_DRAWITEM => {
            let info = unsafe { &*(lparam.0 as *const DRAWITEMSTRUCT) };
            if info.CtlType != ODT_MENU || info.itemData == 0 {
                return None;
            }
            unsafe { &*(info.itemData as *const DrawItem) }.draw(info);
            Some(LRESULT(1))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_ids_round_trip_without_overlapping() {
        let commands = [
            MenuCommand::Refresh,
            MenuCommand::TogglePercentage,
            MenuCommand::Quit,
        ]
        .into_iter()
        .chain(Language::ALL.into_iter().map(MenuCommand::SetLanguage))
        .chain(ThemePreference::ALL.into_iter().map(MenuCommand::SetTheme));
        for command in commands {
            assert_eq!(command_from_id(command_id(command)), Some(command));
        }
        for id in [
            0,
            LANGUAGE_FIRST - 1,
            LANGUAGE_FIRST + 5,
            THEME_FIRST - 1,
            THEME_FIRST + 3,
            u32::MAX,
        ] {
            assert_eq!(command_from_id(id), None);
        }
    }
}
