#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::ffi::{OsStr, c_void};
use std::io;
use std::iter::once;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use std::sync::{Mutex, OnceLock};

mod hid;
mod i18n;
mod settings;

use hid::{ConnectionKind, MouseBattery, ReadError, ReadErrorKind, read_mouse_battery};
use i18n::Language;
use settings::Settings;

type Bool = i32;
type Dword = u32;
type Uint = u32;
type Wparam = usize;
type Lparam = isize;
type Lresult = isize;
type Handle = isize;
type Hwnd = isize;
type Hinstance = isize;
type Hicon = isize;
type Hmenu = isize;
type Hbitmap = isize;
type Hgdiobj = isize;
type Hdc = isize;
type Hbrush = isize;
type Hcursor = isize;
type Hfont = isize;
type Hmonitor = isize;
type Hrgn = isize;

const APP_NAME: &str = "Ninjutso Battery Tray";
const SINGLE_INSTANCE_MUTEX_NAME: &str = "Local\\NinjutsoBatteryTray.Singleton.v1";
const REFRESH_TIMER_ID: usize = 1;
const CHARGE_ANIMATION_TIMER_ID: usize = 2;
const POPUP_LIVE_REFRESH_TIMER_ID: usize = 3;
const REFRESH_INTERVAL_MS: u32 = 60_000;
const CHARGE_ANIMATION_INTERVAL_MS: u32 = 650;
const POPUP_LIVE_REFRESH_INTERVAL_MS: u32 = 1_000;
const TRAY_DEACTIVATION_GRACE_MS: u32 = 250;
const POPUP_WIDTH: i32 = 292;
const POPUP_HEIGHT: i32 = 140;
const POPUP_CORNER_RADIUS: i32 = 20;
const TRAY_UID: u32 = 1;
const TRAY_CALLBACK: u32 = WM_APP + 1;
const MENU_REFRESH: u32 = 1001;
const MENU_TOGGLE_PERCENT: u32 = 1002;
const MENU_EXIT: u32 = 1003;
const MENU_LANGUAGE_FIRST: u32 = 1100;

const WM_DESTROY: u32 = 0x0002;
const WM_ACTIVATE: u32 = 0x0006;
const WM_PAINT: u32 = 0x000F;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_KEYDOWN: u32 = 0x0100;
const WM_TIMER: u32 = 0x0113;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_RBUTTONUP: u32 = 0x0205;
const WM_LBUTTONDBLCLK: u32 = 0x0203;
const WM_APP: u32 = 0x8000;
const WA_INACTIVE: usize = 0;
const VK_ESCAPE: usize = 0x1b;

const WS_POPUP: u32 = 0x8000_0000;
const WS_EX_TOPMOST: u32 = 0x0000_0008;
const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
const CS_DROPSHADOW: u32 = 0x0002_0000;
const HWND_TOPMOST: Hwnd = -1isize;
const SW_HIDE: i32 = 0;
const SWP_SHOWWINDOW: u32 = 0x0040;
const MONITOR_DEFAULTTONEAREST: u32 = 0x0000_0002;

const DT_LEFT: u32 = 0x0000_0000;
const DT_VCENTER: u32 = 0x0000_0004;
const DT_WORDBREAK: u32 = 0x0000_0010;
const DT_SINGLELINE: u32 = 0x0000_0020;
const DT_CALCRECT: u32 = 0x0000_0400;
const DT_END_ELLIPSIS: u32 = 0x0000_8000;
const TRANSPARENT: i32 = 1;
const FW_NORMAL: i32 = 400;
const FW_BOLD: i32 = 700;
const CLEARTYPE_QUALITY: u32 = 5;

const NIM_ADD: u32 = 0x0000_0000;
const NIM_MODIFY: u32 = 0x0000_0001;
const NIM_DELETE: u32 = 0x0000_0002;
const NIF_MESSAGE: u32 = 0x0000_0001;
const NIF_ICON: u32 = 0x0000_0002;
const NIF_TIP: u32 = 0x0000_0004;

const MF_STRING: u32 = 0x0000_0000;
const MF_CHECKED: u32 = 0x0000_0008;
const MF_POPUP: u32 = 0x0000_0010;
const MF_RADIOCHECK: u32 = 0x0000_0200;
const MF_SEPARATOR: u32 = 0x0000_0800;
const TPM_RIGHTBUTTON: u32 = 0x0000_0002;
const TPM_RETURNCMD: u32 = 0x0000_0100;

const MB_OK: u32 = 0x0000_0000;
const MB_ICONERROR: u32 = 0x0000_0010;
const MB_ICONINFORMATION: u32 = 0x0000_0040;

const ERROR_ALREADY_EXISTS: u32 = 183;

const BI_RGB: u32 = 0;
const DIB_RGB_COLORS: u32 = 0;
const SRCCOPY: u32 = 0x00cc_0020;
const ICON_SIZE: i32 = 32;

static APP_STATE: OnceLock<Mutex<AppState>> = OnceLock::new();

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Rect {
    const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}

#[repr(C)]
struct MonitorInfo {
    cb_size: u32,
    monitor: Rect,
    work: Rect,
    flags: u32,
}

#[repr(C)]
struct PaintStruct {
    hdc: Hdc,
    erase: Bool,
    paint: Rect,
    restore: Bool,
    inc_update: Bool,
    reserved: [u8; 32],
}

#[repr(C)]
struct Msg {
    hwnd: Hwnd,
    message: u32,
    w_param: Wparam,
    l_param: Lparam,
    time: u32,
    pt: Point,
    l_private: u32,
}

#[repr(C)]
struct WndClassW {
    style: u32,
    lpfn_wnd_proc: Option<unsafe extern "system" fn(Hwnd, u32, Wparam, Lparam) -> Lresult>,
    cb_cls_extra: i32,
    cb_wnd_extra: i32,
    h_instance: Hinstance,
    h_icon: Hicon,
    h_cursor: Hcursor,
    hbr_background: Hbrush,
    lpsz_menu_name: *const u16,
    lpsz_class_name: *const u16,
}

#[repr(C)]
struct NotifyIconDataW {
    cb_size: u32,
    hwnd: Hwnd,
    uid: u32,
    flags: u32,
    callback_message: u32,
    icon: Hicon,
    tip: [u16; 128],
    state: u32,
    state_mask: u32,
    info: [u16; 256],
    version_or_timeout: u32,
    info_title: [u16; 64],
    info_flags: u32,
    guid_item: Guid,
    balloon_icon: Hicon,
}

#[repr(C)]
struct BitmapInfoHeader {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_pels_per_meter: i32,
    bi_y_pels_per_meter: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}

#[repr(C)]
struct RgbQuad {
    blue: u8,
    green: u8,
    red: u8,
    reserved: u8,
}

#[repr(C)]
struct BitmapInfo {
    header: BitmapInfoHeader,
    colors: [RgbQuad; 1],
}

#[repr(C)]
struct IconInfo {
    icon: Bool,
    x_hotspot: u32,
    y_hotspot: u32,
    mask: Hbitmap,
    color: Hbitmap,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(module_name: *const u16) -> Hinstance;
    fn CreateMutexW(mutex_attributes: *mut c_void, initial_owner: Bool, name: *const u16)
    -> Handle;
    fn GetLastError() -> Dword;
    fn CloseHandle(object: Handle) -> Bool;
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn Shell_NotifyIconW(message: Dword, data: *mut NotifyIconDataW) -> Bool;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassW(wnd_class: *const WndClassW) -> u16;
    fn CreateWindowExW(
        ex_style: Dword,
        class_name: *const u16,
        window_name: *const u16,
        style: Dword,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Hwnd,
        menu: Hmenu,
        instance: Hinstance,
        param: *mut c_void,
    ) -> Hwnd;
    fn DefWindowProcW(hwnd: Hwnd, msg: Uint, w_param: Wparam, l_param: Lparam) -> Lresult;
    fn DestroyWindow(hwnd: Hwnd) -> Bool;
    fn PostQuitMessage(exit_code: i32);
    fn GetMessageW(msg: *mut Msg, hwnd: Hwnd, min_filter: Uint, max_filter: Uint) -> Bool;
    fn TranslateMessage(msg: *const Msg) -> Bool;
    fn DispatchMessageW(msg: *const Msg) -> Lresult;
    fn SetTimer(hwnd: Hwnd, event_id: usize, elapsed_ms: Uint, timer_func: *mut c_void) -> usize;
    fn KillTimer(hwnd: Hwnd, event_id: usize) -> Bool;
    fn ShowWindow(hwnd: Hwnd, command_show: i32) -> Bool;
    fn SetWindowPos(
        hwnd: Hwnd,
        hwnd_insert_after: Hwnd,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: Uint,
    ) -> Bool;
    fn IsWindowVisible(hwnd: Hwnd) -> Bool;
    fn InvalidateRect(hwnd: Hwnd, rect: *const Rect, erase: Bool) -> Bool;
    fn UpdateWindow(hwnd: Hwnd) -> Bool;
    fn SetFocus(hwnd: Hwnd) -> Hwnd;
    fn BeginPaint(hwnd: Hwnd, paint: *mut PaintStruct) -> Hdc;
    fn EndPaint(hwnd: Hwnd, paint: *const PaintStruct) -> Bool;
    fn CreatePopupMenu() -> Hmenu;
    fn AppendMenuW(menu: Hmenu, flags: Uint, new_item_id: usize, new_item: *const u16) -> Bool;
    fn TrackPopupMenu(
        menu: Hmenu,
        flags: Uint,
        x: i32,
        y: i32,
        reserved: i32,
        hwnd: Hwnd,
        rect: *const c_void,
    ) -> Uint;
    fn DestroyMenu(menu: Hmenu) -> Bool;
    fn SetForegroundWindow(hwnd: Hwnd) -> Bool;
    fn GetMessageTime() -> i32;
    fn GetCursorPos(point: *mut Point) -> Bool;
    fn MonitorFromPoint(point: Point, flags: Dword) -> Hmonitor;
    fn GetMonitorInfoW(monitor: Hmonitor, info: *mut MonitorInfo) -> Bool;
    fn SetWindowRgn(hwnd: Hwnd, region: Hrgn, redraw: Bool) -> i32;
    fn CreateIconIndirect(icon_info: *mut IconInfo) -> Hicon;
    fn DestroyIcon(icon: Hicon) -> Bool;
    fn FillRect(dc: Hdc, rect: *const Rect, brush: Hbrush) -> i32;
    fn DrawTextW(dc: Hdc, text: *const u16, count: i32, rect: *mut Rect, format: Uint) -> i32;
    fn MessageBoxW(hwnd: Hwnd, text: *const u16, caption: *const u16, kind: Uint) -> i32;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateDIBSection(
        dc: Hdc,
        bitmap_info: *const BitmapInfo,
        usage: Uint,
        bits: *mut *mut c_void,
        section: Handle,
        offset: Dword,
    ) -> Hbitmap;
    fn CreateBitmap(
        width: i32,
        height: i32,
        planes: Uint,
        bit_count: Uint,
        bits: *const c_void,
    ) -> Hbitmap;
    fn CreateSolidBrush(color: Dword) -> Hbrush;
    fn CreateCompatibleDC(dc: Hdc) -> Hdc;
    fn CreateCompatibleBitmap(dc: Hdc, width: i32, height: i32) -> Hbitmap;
    fn BitBlt(
        destination: Hdc,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        source: Hdc,
        source_x: i32,
        source_y: i32,
        raster_operation: Dword,
    ) -> Bool;
    fn DeleteDC(dc: Hdc) -> Bool;
    fn CreateRoundRectRgn(
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        ellipse_width: i32,
        ellipse_height: i32,
    ) -> Hrgn;
    fn FillRgn(dc: Hdc, region: Hrgn, brush: Hbrush) -> Bool;
    fn CreateFontW(
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: Dword,
        underline: Dword,
        strike_out: Dword,
        char_set: Dword,
        out_precision: Dword,
        clip_precision: Dword,
        quality: Dword,
        pitch_and_family: Dword,
        face_name: *const u16,
    ) -> Hfont;
    fn SelectObject(dc: Hdc, object: Hgdiobj) -> Hgdiobj;
    fn SetTextColor(dc: Hdc, color: Dword) -> Dword;
    fn SetBkMode(dc: Hdc, mode: i32) -> i32;
    fn DeleteObject(object: Hgdiobj) -> Bool;
}

#[derive(Clone)]
enum TrayStatus {
    Reading,
    Ready(MouseBattery),
    Unavailable(ReadError),
}

struct AppState {
    hwnd: Hwnd,
    popup_hwnd: Hwnd,
    icon: Hicon,
    status: TrayStatus,
    show_percent_on_icon: bool,
    language: Language,
    animation_phase: u8,
    tray_click_popup_was_visible: Option<bool>,
    last_popup_deactivation_tick: u32,
}

struct SingleInstanceGuard(Handle);

impl SingleInstanceGuard {
    fn acquire() -> Result<Option<Self>, String> {
        Self::acquire_named(SINGLE_INSTANCE_MUTEX_NAME)
    }

    fn acquire_named(name: &str) -> Result<Option<Self>, String> {
        let name = wide_null(name);
        let handle = unsafe { CreateMutexW(null_mut(), 0, name.as_ptr()) };
        if handle == 0 {
            return Err(format!(
                "CreateMutexW failed: {}",
                io::Error::last_os_error()
            ));
        }

        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                CloseHandle(handle);
            }
            Ok(None)
        } else {
            Ok(Some(Self(handle)))
        }
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[derive(Clone, Copy)]
struct BatteryIconStyle {
    fill: Rgba,
    empty: Rgba,
    outline: Rgba,
    border: Rgba,
    charging: bool,
    animation_phase: u8,
}

#[derive(Clone, Copy)]
struct PopupTextStyle {
    face: &'static str,
    height: i32,
    weight: i32,
    color: Dword,
    flags: u32,
}

struct PopupView {
    percent: u8,
    percent_text: String,
    primary: &'static str,
    secondary: &'static str,
    connection: &'static str,
    accent: Dword,
    show_bar: bool,
}

impl Rgba {
    const fn transparent() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }

    const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--once") {
        let language = settings::load().language;
        match read_mouse_battery() {
            Ok(status) => println!("{}", status_text(&TrayStatus::Ready(status), language)),
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        return;
    }

    let preview = cfg!(debug_assertions) && args.iter().any(|arg| arg == "--preview");
    let preview_language = if cfg!(debug_assertions) {
        args.iter()
            .find_map(|arg| arg.strip_prefix("--preview-language="))
            .and_then(Language::from_code)
    } else {
        None
    };

    let _single_instance_guard = if preview {
        None
    } else {
        match SingleInstanceGuard::acquire() {
            Ok(Some(guard)) => Some(guard),
            Ok(None) => {
                show_information(settings::load().language.strings().already_running);
                return;
            }
            Err(err) => {
                show_error(&err);
                return;
            }
        }
    };

    if let Err(err) = run_tray(preview, preview_language) {
        show_error(&err);
    }
}

fn run_tray(preview: bool, preview_language: Option<Language>) -> Result<(), String> {
    let class_name = wide_null("NinjutsoMouseBatteryTrayWindow");
    let window_name = wide_null(APP_NAME);
    let instance = unsafe { GetModuleHandleW(null()) };

    let wnd_class = WndClassW {
        style: CS_DROPSHADOW,
        lpfn_wnd_proc: Some(window_proc),
        cb_cls_extra: 0,
        cb_wnd_extra: 0,
        h_instance: instance,
        h_icon: 0,
        h_cursor: 0,
        hbr_background: 0,
        lpsz_menu_name: null(),
        lpsz_class_name: class_name.as_ptr(),
    };

    let atom = unsafe { RegisterClassW(&wnd_class) };
    if atom == 0 {
        return Err(format!(
            "RegisterClassW failed: {}",
            io::Error::last_os_error()
        ));
    }

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_name.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            instance,
            null_mut(),
        )
    };

    if hwnd == 0 {
        return Err(format!(
            "CreateWindowExW failed: {}",
            io::Error::last_os_error()
        ));
    }

    let mut settings = settings::load();
    if let Some(language) = preview_language {
        settings.language = language;
    }
    let initial_icon = create_battery_icon(&TrayStatus::Reading, settings.show_percent_on_icon, 0)?;
    APP_STATE
        .set(Mutex::new(AppState {
            hwnd,
            popup_hwnd: 0,
            icon: initial_icon,
            status: TrayStatus::Reading,
            show_percent_on_icon: settings.show_percent_on_icon,
            language: settings.language,
            animation_phase: 0,
            tray_click_popup_was_visible: None,
            last_popup_deactivation_tick: 0,
        }))
        .map_err(|_| "Application state is already initialized".to_string())?;

    add_or_update_tray_icon(NIM_ADD)?;
    refresh_tray();
    if preview {
        show_status_popup(hwnd);
    }

    let timer = unsafe { SetTimer(hwnd, REFRESH_TIMER_ID, REFRESH_INTERVAL_MS, null_mut()) };
    if timer == 0 {
        return Err(format!("SetTimer failed: {}", io::Error::last_os_error()));
    }

    let animation_timer = unsafe {
        SetTimer(
            hwnd,
            CHARGE_ANIMATION_TIMER_ID,
            CHARGE_ANIMATION_INTERVAL_MS,
            null_mut(),
        )
    };
    if animation_timer == 0 {
        return Err(format!(
            "Animation SetTimer failed: {}",
            io::Error::last_os_error()
        ));
    }

    let mut msg: Msg = unsafe { zeroed() };
    while unsafe { GetMessageW(&mut msg, 0, 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}

unsafe extern "system" fn window_proc(
    hwnd: Hwnd,
    msg: u32,
    w_param: Wparam,
    l_param: Lparam,
) -> Lresult {
    match msg {
        WM_TIMER if w_param == REFRESH_TIMER_ID => {
            refresh_tray();
            0
        }
        WM_TIMER if w_param == CHARGE_ANIMATION_TIMER_ID => {
            tick_charge_animation();
            0
        }
        WM_TIMER if w_param == POPUP_LIVE_REFRESH_TIMER_ID => {
            match popup_live_refresh_action(is_status_popup_visible()) {
                PopupLiveRefreshAction::Refresh => refresh_tray(),
                PopupLiveRefreshAction::Stop => stop_popup_live_refresh(),
            }
            0
        }
        WM_PAINT if is_status_popup(hwnd) => {
            paint_status_popup(hwnd);
            0
        }
        WM_ERASEBKGND if is_status_popup(hwnd) => 1,
        WM_ACTIVATE if is_status_popup(hwnd) && (w_param & 0xffff) == WA_INACTIVE => {
            remember_popup_deactivation();
            hide_status_popup();
            0
        }
        WM_KEYDOWN if is_status_popup(hwnd) && w_param == VK_ESCAPE => {
            hide_status_popup();
            0
        }
        TRAY_CALLBACK => {
            let event = l_param as u32;
            if event == WM_RBUTTONUP {
                hide_status_popup();
                show_context_menu(hwnd);
            } else if event == WM_LBUTTONDOWN {
                remember_tray_left_button_down();
            } else if event == WM_LBUTTONUP {
                finish_tray_left_click(hwnd);
            } else if event == WM_LBUTTONDBLCLK {
                remember_tray_left_button_down();
                refresh_tray();
            }
            0
        }
        WM_DESTROY if !is_status_popup(hwnd) => {
            cleanup_tray();
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        WM_DESTROY => 0,
        _ => unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) },
    }
}

fn show_context_menu(hwnd: Hwnd) {
    unsafe {
        let menu = CreatePopupMenu();
        let language_menu = CreatePopupMenu();
        if menu == 0 || language_menu == 0 {
            if menu != 0 {
                DestroyMenu(menu);
            }
            if language_menu != 0 {
                DestroyMenu(language_menu);
            }
            return;
        }

        let language = current_language();
        let strings = language.strings();
        let refresh = wide_null(strings.refresh);
        let toggle_percent = wide_null(strings.show_percent);
        let language_label = wide_null(strings.language);
        let exit = wide_null(strings.exit);
        let toggle_flags = if current_show_percent_on_icon() {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };

        AppendMenuW(menu, MF_STRING, MENU_REFRESH as usize, refresh.as_ptr());
        AppendMenuW(
            menu,
            toggle_flags,
            MENU_TOGGLE_PERCENT as usize,
            toggle_percent.as_ptr(),
        );
        for (index, item_language) in Language::ALL.into_iter().enumerate() {
            let label = wide_null(item_language.autonym());
            let flags = if item_language == language {
                MF_STRING | MF_CHECKED | MF_RADIOCHECK
            } else {
                MF_STRING | MF_RADIOCHECK
            };
            AppendMenuW(
                language_menu,
                flags,
                (MENU_LANGUAGE_FIRST + index as u32) as usize,
                label.as_ptr(),
            );
        }
        AppendMenuW(
            menu,
            MF_POPUP,
            language_menu as usize,
            language_label.as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        AppendMenuW(menu, MF_STRING, MENU_EXIT as usize, exit.as_ptr());

        let mut point = Point { x: 0, y: 0 };
        if GetCursorPos(&mut point) != 0 {
            SetForegroundWindow(hwnd);
            let command = TrackPopupMenu(
                menu,
                TPM_RIGHTBUTTON | TPM_RETURNCMD,
                point.x,
                point.y,
                0,
                hwnd,
                null(),
            );
            if command == MENU_REFRESH {
                refresh_tray();
            } else if command == MENU_TOGGLE_PERCENT {
                toggle_percent_on_icon();
            } else if let Some(language) = language_from_menu_command(command) {
                set_language(language);
            } else if command == MENU_EXIT {
                DestroyWindow(hwnd);
            }
        }

        DestroyMenu(menu);
    }
}

fn language_from_menu_command(command: u32) -> Option<Language> {
    let index = command.checked_sub(MENU_LANGUAGE_FIRST)? as usize;
    Language::ALL.get(index).copied()
}

fn is_status_popup(hwnd: Hwnd) -> bool {
    hwnd != 0
        && APP_STATE
            .get()
            .and_then(|state_lock| state_lock.lock().ok())
            .map(|state| state.popup_hwnd == hwnd)
            .unwrap_or(false)
}

fn is_status_popup_visible() -> bool {
    let popup_hwnd = APP_STATE
        .get()
        .and_then(|state_lock| state_lock.lock().ok())
        .map(|state| state.popup_hwnd)
        .unwrap_or(0);

    popup_hwnd != 0 && unsafe { IsWindowVisible(popup_hwnd) } != 0
}

fn remember_popup_deactivation() {
    let tick = unsafe { GetMessageTime() } as u32;
    if let Some(state_lock) = APP_STATE.get()
        && let Ok(mut state) = state_lock.lock()
    {
        state.last_popup_deactivation_tick = tick;
    }
}

fn remember_tray_left_button_down() {
    let popup_visible = is_status_popup_visible();
    let message_tick = unsafe { GetMessageTime() } as u32;

    if let Some(state_lock) = APP_STATE.get()
        && let Ok(mut state) = state_lock.lock()
    {
        state.tray_click_popup_was_visible = Some(popup_was_visible_for_tray_click(
            popup_visible,
            state.last_popup_deactivation_tick,
            message_tick,
        ));
        state.last_popup_deactivation_tick = 0;
    }
}

fn popup_was_visible_for_tray_click(
    popup_visible: bool,
    last_deactivation_tick: u32,
    message_tick: u32,
) -> bool {
    popup_visible
        || (last_deactivation_tick != 0
            && message_tick.wrapping_sub(last_deactivation_tick) <= TRAY_DEACTIVATION_GRACE_MS)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TrayLeftClickAction {
    Hide,
    RefreshAndShow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PopupLiveRefreshAction {
    Refresh,
    Stop,
}

fn popup_live_refresh_action(popup_visible: bool) -> PopupLiveRefreshAction {
    if popup_visible {
        PopupLiveRefreshAction::Refresh
    } else {
        PopupLiveRefreshAction::Stop
    }
}

fn tray_left_click_action(popup_was_visible: bool) -> TrayLeftClickAction {
    if popup_was_visible {
        TrayLeftClickAction::Hide
    } else {
        TrayLeftClickAction::RefreshAndShow
    }
}

fn finish_tray_left_click(owner_hwnd: Hwnd) {
    let popup_was_visible = APP_STATE
        .get()
        .and_then(|state_lock| state_lock.lock().ok())
        .and_then(|mut state| state.tray_click_popup_was_visible.take())
        .unwrap_or_else(is_status_popup_visible);

    match tray_left_click_action(popup_was_visible) {
        TrayLeftClickAction::Hide => hide_status_popup(),
        TrayLeftClickAction::RefreshAndShow => {
            refresh_tray();
            show_status_popup(owner_hwnd);
        }
    }
}

fn show_status_popup(owner_hwnd: Hwnd) {
    let popup_hwnd = ensure_status_popup(owner_hwnd);
    if popup_hwnd == 0 {
        return;
    }

    let mut point = Point { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut point);
    }

    let (x, y) = popup_position(point);

    unsafe {
        InvalidateRect(popup_hwnd, null(), 0);
        SetWindowPos(
            popup_hwnd,
            HWND_TOPMOST,
            x,
            y,
            POPUP_WIDTH,
            POPUP_HEIGHT,
            SWP_SHOWWINDOW,
        );
        UpdateWindow(popup_hwnd);
        SetForegroundWindow(popup_hwnd);
        SetFocus(popup_hwnd);
    }
    start_popup_live_refresh(owner_hwnd);
}

fn popup_position(point: Point) -> (i32, i32) {
    let monitor = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) };
    let mut info = MonitorInfo {
        cb_size: size_of::<MonitorInfo>() as u32,
        monitor: Rect::new(0, 0, 0, 0),
        work: Rect::new(0, 0, 0, 0),
        flags: 0,
    };
    if monitor == 0 || unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return (
            (point.x - POPUP_WIDTH + 24).max(8),
            (point.y - POPUP_HEIGHT - 18).max(8),
        );
    }

    let margin = 8;
    let work = info.work;
    let mut x = point.x - POPUP_WIDTH + 24;
    let mut y = point.y - POPUP_HEIGHT - 18;

    if point.y >= work.bottom {
        y = work.bottom - POPUP_HEIGHT - margin;
    } else if point.y < work.top {
        y = work.top + margin;
    } else if point.x >= work.right {
        x = work.right - POPUP_WIDTH - margin;
        y = point.y - POPUP_HEIGHT + 24;
    } else if point.x < work.left {
        x = work.left + margin;
        y = point.y - POPUP_HEIGHT + 24;
    }

    let min_x = work.left + margin;
    let max_x = (work.right - POPUP_WIDTH - margin).max(min_x);
    let min_y = work.top + margin;
    let max_y = (work.bottom - POPUP_HEIGHT - margin).max(min_y);
    (x.clamp(min_x, max_x), y.clamp(min_y, max_y))
}

fn ensure_status_popup(owner_hwnd: Hwnd) -> Hwnd {
    if let Some(existing) = APP_STATE
        .get()
        .and_then(|state_lock| state_lock.lock().ok())
        .map(|state| state.popup_hwnd)
        && existing != 0
    {
        return existing;
    }

    let class_name = wide_null("NinjutsoMouseBatteryTrayWindow");
    let window_name = wide_null(APP_NAME);
    let instance = unsafe { GetModuleHandleW(null()) };
    let popup_hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            class_name.as_ptr(),
            window_name.as_ptr(),
            WS_POPUP,
            0,
            0,
            POPUP_WIDTH,
            POPUP_HEIGHT,
            owner_hwnd,
            0,
            instance,
            null_mut(),
        )
    };

    if popup_hwnd != 0 {
        let region = unsafe {
            CreateRoundRectRgn(
                0,
                0,
                POPUP_WIDTH + 1,
                POPUP_HEIGHT + 1,
                POPUP_CORNER_RADIUS,
                POPUP_CORNER_RADIUS,
            )
        };
        if region != 0 && unsafe { SetWindowRgn(popup_hwnd, region, 1) } == 0 {
            unsafe {
                DeleteObject(region as Hgdiobj);
            }
        }
        if let Some(state_lock) = APP_STATE.get()
            && let Ok(mut state) = state_lock.lock()
        {
            state.popup_hwnd = popup_hwnd;
        }
    }

    popup_hwnd
}

fn hide_status_popup() {
    let Some((owner_hwnd, popup_hwnd)) = APP_STATE
        .get()
        .and_then(|state_lock| state_lock.lock().ok())
        .map(|state| (state.hwnd, state.popup_hwnd))
    else {
        return;
    };

    stop_popup_live_refresh_for(owner_hwnd);
    unsafe {
        if popup_hwnd != 0 {
            ShowWindow(popup_hwnd, SW_HIDE);
        }
    }
}

fn start_popup_live_refresh(owner_hwnd: Hwnd) {
    if owner_hwnd != 0 {
        unsafe {
            SetTimer(
                owner_hwnd,
                POPUP_LIVE_REFRESH_TIMER_ID,
                POPUP_LIVE_REFRESH_INTERVAL_MS,
                null_mut(),
            );
        }
    }
}

fn stop_popup_live_refresh() {
    let owner_hwnd = APP_STATE
        .get()
        .and_then(|state_lock| state_lock.lock().ok())
        .map(|state| state.hwnd)
        .unwrap_or(0);
    stop_popup_live_refresh_for(owner_hwnd);
}

fn stop_popup_live_refresh_for(owner_hwnd: Hwnd) {
    if owner_hwnd != 0 {
        unsafe {
            KillTimer(owner_hwnd, POPUP_LIVE_REFRESH_TIMER_ID);
        }
    }
}

fn invalidate_status_popup() {
    let popup_hwnd = APP_STATE
        .get()
        .and_then(|state_lock| state_lock.lock().ok())
        .map(|state| state.popup_hwnd)
        .unwrap_or(0);

    if popup_hwnd != 0 {
        unsafe {
            InvalidateRect(popup_hwnd, null(), 0);
        }
    }
}

fn paint_status_popup(hwnd: Hwnd) {
    let (status, language) = APP_STATE
        .get()
        .and_then(|state_lock| state_lock.lock().ok())
        .map(|state| (state.status.clone(), state.language))
        .unwrap_or((TrayStatus::Reading, Language::English));

    let mut paint: PaintStruct = unsafe { zeroed() };
    let hdc = unsafe { BeginPaint(hwnd, &mut paint) };
    if hdc == 0 {
        return;
    }

    let buffer_dc = unsafe { CreateCompatibleDC(hdc) };
    let buffer_bitmap = if buffer_dc != 0 {
        unsafe { CreateCompatibleBitmap(hdc, POPUP_WIDTH, POPUP_HEIGHT) }
    } else {
        0
    };

    if buffer_dc != 0 && buffer_bitmap != 0 {
        let old_bitmap = unsafe { SelectObject(buffer_dc, buffer_bitmap as Hgdiobj) };
        draw_status_popup(buffer_dc, &status, language);
        unsafe {
            BitBlt(
                hdc,
                0,
                0,
                POPUP_WIDTH,
                POPUP_HEIGHT,
                buffer_dc,
                0,
                0,
                SRCCOPY,
            );
            SelectObject(buffer_dc, old_bitmap);
            DeleteObject(buffer_bitmap as Hgdiobj);
            DeleteDC(buffer_dc);
        }
    } else {
        if buffer_bitmap != 0 {
            unsafe {
                DeleteObject(buffer_bitmap as Hgdiobj);
            }
        }
        if buffer_dc != 0 {
            unsafe {
                DeleteDC(buffer_dc);
            }
        }
        draw_status_popup(hdc, &status, language);
    }

    unsafe {
        EndPaint(hwnd, &paint);
    }
}

fn draw_status_popup(hdc: Hdc, status: &TrayStatus, language: Language) {
    let view = popup_view(status, language);
    let background = rgb(16, 18, 22);
    let border = rgb(47, 53, 62);
    let text = rgb(247, 249, 252);
    let muted = rgb(143, 153, 166);
    let pill = rgb(28, 32, 38);
    let connection_style = PopupTextStyle {
        face: "Segoe UI Variable Text",
        height: -12,
        weight: FW_NORMAL,
        color: muted,
        flags: DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    };
    let connection_pill_left = if view.connection.is_empty() {
        276
    } else {
        let text_width = measure_dc_text_width(hdc, view.connection, connection_style);
        276 - (text_width + 36).clamp(58, 122)
    };

    fill_dc_rect(hdc, Rect::new(0, 0, POPUP_WIDTH, POPUP_HEIGHT), border);
    fill_dc_rect(
        hdc,
        Rect::new(1, 1, POPUP_WIDTH - 1, POPUP_HEIGHT - 1),
        background,
    );
    fill_dc_rect(hdc, Rect::new(30, 1, POPUP_WIDTH - 30, 2), rgb(69, 76, 88));

    draw_dc_text(
        hdc,
        "Ninjutso Ten",
        Rect::new(16, 9, connection_pill_left - 8, 35),
        PopupTextStyle {
            face: "Segoe UI Variable Text",
            height: -17,
            weight: FW_BOLD,
            color: text,
            flags: DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        },
    );

    if !view.connection.is_empty() {
        fill_dc_round_rect(hdc, Rect::new(connection_pill_left, 10, 276, 34), 14, pill);
        fill_dc_rect(
            hdc,
            Rect::new(connection_pill_left + 11, 20, connection_pill_left + 15, 24),
            view.accent,
        );
        draw_dc_text(
            hdc,
            view.connection,
            Rect::new(connection_pill_left + 24, 10, 268, 34),
            connection_style,
        );
    }

    draw_dc_text(
        hdc,
        &view.percent_text,
        Rect::new(16, 38, 136, 105),
        PopupTextStyle {
            face: "Segoe UI Variable Display",
            height: -45,
            weight: FW_BOLD,
            color: text,
            flags: DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        },
    );

    fill_dc_rect(hdc, Rect::new(145, 50, 148, 66), view.accent);
    draw_dc_text(
        hdc,
        view.primary,
        Rect::new(156, 42, 276, 72),
        PopupTextStyle {
            face: "Segoe UI Variable Text",
            height: -16,
            weight: FW_BOLD,
            color: text,
            flags: DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        },
    );

    draw_dc_text(
        hdc,
        view.secondary,
        Rect::new(145, 72, 276, 108),
        PopupTextStyle {
            face: "Segoe UI Variable Text",
            height: -12,
            weight: FW_NORMAL,
            color: muted,
            flags: DT_LEFT | DT_WORDBREAK,
        },
    );

    draw_popup_battery_bar(hdc, view.percent, view.accent, view.show_bar);
}

fn popup_view(status: &TrayStatus, language: Language) -> PopupView {
    let strings = language.strings();

    match status {
        TrayStatus::Ready(battery) => {
            let level_color = if battery.percent >= 50 {
                rgb(82, 211, 132)
            } else if battery.percent >= 20 {
                rgb(246, 190, 72)
            } else {
                rgb(255, 91, 98)
            };
            let source = match battery.connection {
                ConnectionKind::Wired => strings.wired,
                ConnectionKind::Receiver => strings.receiver,
            };
            if battery.online {
                let primary = if battery.charging {
                    strings.charging
                } else {
                    strings.on_battery
                };
                PopupView {
                    percent: battery.percent.min(100),
                    percent_text: format!("{}%", battery.percent.min(100)),
                    primary,
                    secondary: strings.connected,
                    connection: source,
                    accent: if battery.charging {
                        rgb(65, 225, 143)
                    } else {
                        level_color
                    },
                    show_bar: true,
                }
            } else {
                PopupView {
                    percent: 0,
                    percent_text: "—".to_string(),
                    primary: strings.disconnected,
                    secondary: strings.device_not_found_help,
                    connection: source,
                    accent: rgb(112, 123, 137),
                    show_bar: false,
                }
            }
        }
        TrayStatus::Reading => PopupView {
            percent: 0,
            percent_text: "···".to_string(),
            primary: strings.reading,
            secondary: "",
            connection: "",
            accent: rgb(91, 156, 255),
            show_bar: false,
        },
        TrayStatus::Unavailable(error) => match error.kind {
            ReadErrorKind::DeviceNotFound => PopupView {
                percent: 0,
                percent_text: "—".to_string(),
                primary: strings.device_not_found,
                secondary: strings.device_not_found_help,
                connection: "",
                accent: rgb(112, 123, 137),
                show_bar: false,
            },
            ReadErrorKind::NoBatteryData | ReadErrorKind::System => PopupView {
                percent: 0,
                percent_text: "—".to_string(),
                primary: strings.read_failed,
                secondary: strings.read_failed_help,
                connection: "",
                accent: rgb(255, 91, 98),
                show_bar: false,
            },
        },
    }
}

fn draw_popup_battery_bar(hdc: Hdc, percent: u8, accent: Dword, show_fill: bool) {
    const LEFT: i32 = 16;
    const RIGHT: i32 = 276;
    const WIDTH: u16 = (RIGHT - LEFT) as u16;

    fill_dc_round_rect(hdc, Rect::new(LEFT, 116, RIGHT, 124), 8, rgb(35, 40, 47));
    if show_fill && percent > 0 {
        let fill_width = (WIDTH * u16::from(percent)).div_ceil(100) as i32;
        fill_dc_round_rect(
            hdc,
            Rect::new(LEFT, 116, LEFT + fill_width.max(8).min(WIDTH as i32), 124),
            8,
            accent,
        );
    }
}

fn fill_dc_rect(hdc: Hdc, rect: Rect, color: Dword) {
    let brush = unsafe { CreateSolidBrush(color) };
    if brush == 0 {
        return;
    }

    unsafe {
        FillRect(hdc, &rect, brush);
        DeleteObject(brush as Hgdiobj);
    }
}

fn fill_dc_round_rect(hdc: Hdc, rect: Rect, corner_diameter: i32, color: Dword) {
    let region = unsafe {
        CreateRoundRectRgn(
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            corner_diameter,
            corner_diameter,
        )
    };
    let brush = unsafe { CreateSolidBrush(color) };
    if region == 0 || brush == 0 {
        if region != 0 {
            unsafe {
                DeleteObject(region as Hgdiobj);
            }
        }
        if brush != 0 {
            unsafe {
                DeleteObject(brush as Hgdiobj);
            }
        }
        return;
    }

    unsafe {
        FillRgn(hdc, region, brush);
        DeleteObject(region as Hgdiobj);
        DeleteObject(brush as Hgdiobj);
    }
}

fn draw_dc_text(hdc: Hdc, text: &str, mut rect: Rect, style: PopupTextStyle) {
    let font = create_popup_font(style);
    if font == 0 {
        return;
    }

    let text_wide = wide_null(text);
    unsafe {
        let old_font = SelectObject(hdc, font as Hgdiobj);
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, style.color);
        DrawTextW(
            hdc,
            text_wide.as_ptr(),
            text_wide.len().saturating_sub(1) as i32,
            &mut rect,
            style.flags,
        );
        SelectObject(hdc, old_font);
        DeleteObject(font as Hgdiobj);
    }
}

fn measure_dc_text_width(hdc: Hdc, text: &str, style: PopupTextStyle) -> i32 {
    let font = create_popup_font(style);
    if font == 0 {
        return 0;
    }

    let text_wide = wide_null(text);
    let mut rect = Rect::new(0, 0, 0, 0);
    unsafe {
        let old_font = SelectObject(hdc, font as Hgdiobj);
        DrawTextW(
            hdc,
            text_wide.as_ptr(),
            text_wide.len().saturating_sub(1) as i32,
            &mut rect,
            DT_CALCRECT | DT_SINGLELINE,
        );
        SelectObject(hdc, old_font);
        DeleteObject(font as Hgdiobj);
    }
    rect.right - rect.left
}

fn create_popup_font(style: PopupTextStyle) -> Hfont {
    let face = wide_null(style.face);
    unsafe {
        CreateFontW(
            style.height,
            0,
            0,
            0,
            style.weight,
            0,
            0,
            0,
            1,
            0,
            0,
            CLEARTYPE_QUALITY,
            0,
            face.as_ptr(),
        )
    }
}

fn rgb(red: u8, green: u8, blue: u8) -> Dword {
    red as Dword | ((green as Dword) << 8) | ((blue as Dword) << 16)
}

fn current_show_percent_on_icon() -> bool {
    APP_STATE
        .get()
        .and_then(|state_lock| state_lock.lock().ok())
        .map(|state| state.show_percent_on_icon)
        .unwrap_or(true)
}

fn current_language() -> Language {
    APP_STATE
        .get()
        .and_then(|state_lock| state_lock.lock().ok())
        .map(|state| state.language)
        .unwrap_or(Language::English)
}

fn current_settings() -> Settings {
    APP_STATE
        .get()
        .and_then(|state_lock| state_lock.lock().ok())
        .map(|state| Settings {
            show_percent_on_icon: state.show_percent_on_icon,
            language: state.language,
        })
        .unwrap_or_default()
}

fn current_icon_options() -> (bool, u8) {
    APP_STATE
        .get()
        .and_then(|state_lock| state_lock.lock().ok())
        .map(|state| (state.show_percent_on_icon, state.animation_phase))
        .unwrap_or((true, 0))
}

fn toggle_percent_on_icon() {
    let new_value = !current_show_percent_on_icon();

    if let Some(state_lock) = APP_STATE.get()
        && let Ok(mut state) = state_lock.lock()
    {
        state.show_percent_on_icon = new_value;
    }

    let _ = settings::save(current_settings());
    refresh_tray();
}

fn set_language(language: Language) {
    if let Some(state_lock) = APP_STATE.get()
        && let Ok(mut state) = state_lock.lock()
    {
        state.language = language;
    }

    let _ = settings::save(current_settings());
    let _ = add_or_update_tray_icon(NIM_MODIFY);
    invalidate_status_popup();
}

fn tick_charge_animation() {
    let Some(state_lock) = APP_STATE.get() else {
        return;
    };

    let Ok(mut state) = state_lock.lock() else {
        return;
    };

    let charging =
        matches!(&state.status, TrayStatus::Ready(battery) if battery.online && battery.charging);
    if !charging || state.show_percent_on_icon {
        state.animation_phase = 0;
        return;
    }

    state.animation_phase = state.animation_phase.wrapping_add(1);
    let status = state.status.clone();
    let show_percent_on_icon = state.show_percent_on_icon;
    let animation_phase = state.animation_phase;
    drop(state);

    replace_tray_icon(status, show_percent_on_icon, animation_phase, false);
}

fn cleanup_tray() {
    let (hwnd, icon) = if let Some(state_lock) = APP_STATE.get() {
        if let Ok(mut state) = state_lock.lock() {
            let hwnd = state.hwnd;
            let icon = state.icon;
            state.icon = 0;
            (hwnd, icon)
        } else {
            (0, 0)
        }
    } else {
        (0, 0)
    };

    if hwnd != 0 {
        unsafe {
            KillTimer(hwnd, REFRESH_TIMER_ID);
            KillTimer(hwnd, CHARGE_ANIMATION_TIMER_ID);
            KillTimer(hwnd, POPUP_LIVE_REFRESH_TIMER_ID);
        }
        let _ = delete_tray_icon(hwnd);
    }

    if icon != 0 {
        unsafe {
            DestroyIcon(icon);
        }
    }
}

fn refresh_tray() {
    let status = match read_mouse_battery() {
        Ok(battery) => TrayStatus::Ready(battery),
        Err(err) => TrayStatus::Unavailable(err),
    };

    let (show_percent_on_icon, animation_phase) = current_icon_options();
    replace_tray_icon(status, show_percent_on_icon, animation_phase, true);
}

fn replace_tray_icon(
    status: TrayStatus,
    show_percent_on_icon: bool,
    animation_phase: u8,
    invalidate_popup: bool,
) {
    let icon_phase = if matches!(&status, TrayStatus::Ready(battery) if battery.online && battery.charging)
    {
        animation_phase
    } else {
        0
    };
    let Ok(new_icon) = create_battery_icon(&status, show_percent_on_icon, icon_phase) else {
        return;
    };

    if let Some(state_lock) = APP_STATE.get()
        && let Ok(mut state) = state_lock.lock()
    {
        let old_icon = state.icon;
        state.icon = new_icon;
        state.status = status;
        state.animation_phase = icon_phase;
        drop(state);

        let _ = add_or_update_tray_icon(NIM_MODIFY);
        if invalidate_popup {
            invalidate_status_popup();
        }

        if old_icon != 0 {
            unsafe {
                DestroyIcon(old_icon);
            }
        }
    }
}

fn add_or_update_tray_icon(message: u32) -> Result<(), String> {
    let state_lock = APP_STATE
        .get()
        .ok_or_else(|| "Application state is not initialized".to_string())?;
    let state = state_lock
        .lock()
        .map_err(|_| "Application state lock is poisoned".to_string())?;

    let mut data: NotifyIconDataW = unsafe { zeroed() };
    data.cb_size = size_of::<NotifyIconDataW>() as u32;
    data.hwnd = state.hwnd;
    data.uid = TRAY_UID;
    data.flags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.callback_message = TRAY_CALLBACK;
    data.icon = state.icon;
    write_wide_fixed(&mut data.tip, &status_text(&state.status, state.language));

    let ok = unsafe { Shell_NotifyIconW(message, &mut data) };
    if ok == 0 {
        Err(format!(
            "Shell_NotifyIconW failed: {}",
            io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

fn delete_tray_icon(hwnd: Hwnd) -> Result<(), String> {
    let mut data: NotifyIconDataW = unsafe { zeroed() };
    data.cb_size = size_of::<NotifyIconDataW>() as u32;
    data.hwnd = hwnd;
    data.uid = TRAY_UID;

    let ok = unsafe { Shell_NotifyIconW(NIM_DELETE, &mut data) };
    if ok == 0 {
        Err(format!(
            "Shell_NotifyIconW delete failed: {}",
            io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

fn status_text(status: &TrayStatus, language: Language) -> String {
    let strings = language.strings();
    match status {
        TrayStatus::Reading => format!("Ninjutso Ten: {}", strings.reading),
        TrayStatus::Ready(battery) => {
            let source = match battery.connection {
                ConnectionKind::Wired => strings.wired,
                ConnectionKind::Receiver => strings.receiver,
            };
            if !battery.online {
                return format!("Ninjutso Ten: {} · {source}", strings.disconnected);
            }
            let charge = if battery.charging {
                strings.charging
            } else {
                strings.not_charging
            };
            format!("Ninjutso Ten: {}% · {charge} · {source}", battery.percent)
        }
        TrayStatus::Unavailable(error) => match error.kind {
            ReadErrorKind::DeviceNotFound => {
                format!("Ninjutso Ten: {}", strings.device_not_found)
            }
            ReadErrorKind::NoBatteryData | ReadErrorKind::System => {
                format!("Ninjutso Ten: {}", strings.read_failed)
            }
        },
    }
}

fn create_battery_icon(
    status: &TrayStatus,
    show_percent_on_icon: bool,
    animation_phase: u8,
) -> Result<Hicon, String> {
    create_icon_from_pixels(&render_battery_icon_pixels(
        status,
        show_percent_on_icon,
        animation_phase,
    ))
}

fn render_battery_icon_pixels(
    status: &TrayStatus,
    show_percent_on_icon: bool,
    animation_phase: u8,
) -> Vec<Rgba> {
    let mut pixels = vec![Rgba::transparent(); (ICON_SIZE * ICON_SIZE) as usize];

    let (percent, fill_color, empty_color, charging) = match status {
        TrayStatus::Ready(battery) if !battery.online => (
            100,
            Rgba::opaque(120, 120, 120),
            Rgba::opaque(42, 48, 52),
            false,
        ),
        TrayStatus::Ready(battery) => {
            let color = if battery.percent >= 50 {
                Rgba::opaque(35, 190, 85)
            } else if battery.percent >= 20 {
                Rgba::opaque(238, 180, 38)
            } else {
                Rgba::opaque(220, 55, 55)
            };
            (
                battery.percent.min(100),
                color,
                Rgba::opaque(38, 43, 47),
                battery.online && battery.charging,
            )
        }
        TrayStatus::Reading => (
            100,
            Rgba::opaque(100, 145, 205),
            Rgba::opaque(42, 48, 52),
            false,
        ),
        TrayStatus::Unavailable(_) => (
            100,
            Rgba::opaque(120, 120, 120),
            Rgba::opaque(42, 48, 52),
            false,
        ),
    };

    let black = Rgba::opaque(0, 0, 0);
    let white = Rgba::opaque(255, 255, 255);

    if show_percent_on_icon
        && let TrayStatus::Ready(battery) = status
        && battery.online
    {
        draw_numeric_battery_badge(
            &mut pixels,
            battery.percent.min(100),
            fill_color,
            black,
            white,
        );
    } else {
        draw_large_battery(
            &mut pixels,
            percent,
            BatteryIconStyle {
                fill: fill_color,
                empty: empty_color,
                outline: black,
                border: white,
                charging,
                animation_phase,
            },
        );
    }

    pixels
}

fn fill_rect(pixels: &mut [Rgba], x: i32, y: i32, width: i32, height: i32, color: Rgba) {
    for yy in y..(y + height) {
        for xx in x..(x + width) {
            set_pixel(pixels, xx, yy, color);
        }
    }
}

fn set_pixel(pixels: &mut [Rgba], x: i32, y: i32, color: Rgba) {
    if (0..ICON_SIZE).contains(&x) && (0..ICON_SIZE).contains(&y) {
        let idx = (y * ICON_SIZE + x) as usize;
        pixels[idx] = color;
    }
}

fn draw_large_battery(pixels: &mut [Rgba], percent: u8, style: BatteryIconStyle) {
    let center_x = 16;
    let inner_top = 5;
    let inner_bottom = 28;
    let inner_height = inner_bottom - inner_top + 1;
    let fill_height = (inner_height as u16 * u16::from(percent)).div_ceil(100) as i32;
    let fill_top = inner_bottom + 1 - fill_height.clamp(1, inner_height);

    for y in 0..ICON_SIZE {
        let half = mouse_body_half_width(y);
        if half <= 0 {
            continue;
        }
        for x in (center_x - half)..=(center_x + half) {
            set_pixel(pixels, x, y, style.outline);
        }
    }

    for y in 0..ICON_SIZE {
        let half = mouse_body_half_width(y) - 1;
        if half <= 0 {
            continue;
        }
        for x in (center_x - half)..=(center_x + half) {
            set_pixel(pixels, x, y, style.border);
        }
    }

    for y in 0..ICON_SIZE {
        let half = mouse_body_half_width(y) - 3;
        if half <= 0 {
            continue;
        }
        for x in (center_x - half)..=(center_x + half) {
            let color = if y >= fill_top {
                style.fill
            } else {
                style.empty
            };
            set_pixel(pixels, x, y, color);
        }
    }

    if style.charging {
        draw_mouse_charge_wave(pixels, fill_top, style.fill, style.animation_phase);
    }

    fill_rect(pixels, center_x - 1, 5, 2, 7, style.outline);
    fill_rect(pixels, center_x - 1, 6, 2, 4, style.border);
}

fn mouse_body_half_width(y: i32) -> i32 {
    match y {
        0 => 0,
        1 => 3,
        2 => 6,
        3 => 8,
        4..=5 => 10,
        6..=22 => 11,
        23..=24 => 10,
        25 => 9,
        26 => 8,
        27 => 7,
        28 => 5,
        29 => 2,
        _ => 0,
    }
}

fn draw_mouse_charge_wave(
    pixels: &mut [Rgba],
    fill_top: i32,
    fill_color: Rgba,
    animation_phase: u8,
) {
    let shine = mix_color(fill_color, Rgba::opaque(255, 255, 255), 2, 5);
    let offset = (animation_phase as i32 * 5) % 34 - 12;

    for y in fill_top..=28 {
        let half = mouse_body_half_width(y) - 3;
        if half <= 0 {
            continue;
        }
        for thickness in 0..5 {
            let x = 16 - half + offset + thickness + (y - fill_top) / 3;
            if x >= 16 - half && x <= 16 + half {
                set_pixel(pixels, x, y, shine);
            }
        }
    }
}

fn mix_color(base: Rgba, overlay: Rgba, overlay_parts: u16, total_parts: u16) -> Rgba {
    let base_parts = total_parts - overlay_parts;
    Rgba {
        r: ((base.r as u16 * base_parts + overlay.r as u16 * overlay_parts) / total_parts) as u8,
        g: ((base.g as u16 * base_parts + overlay.g as u16 * overlay_parts) / total_parts) as u8,
        b: ((base.b as u16 * base_parts + overlay.b as u16 * overlay_parts) / total_parts) as u8,
        a: 255,
    }
}

fn draw_numeric_battery_badge(
    pixels: &mut [Rgba],
    percent: u8,
    accent: Rgba,
    outline: Rgba,
    text_color: Rgba,
) {
    let surface = Rgba::opaque(19, 23, 27);

    for y in 0..ICON_SIZE {
        let half = numeric_badge_half_width(y);
        if half <= 0 {
            continue;
        }

        for x in (16 - half)..=(16 + half) {
            let distance_to_edge = half - (x - 16).abs();
            let color = if distance_to_edge == 0 {
                outline
            } else if distance_to_edge <= 2 || y <= 2 || y >= 29 {
                accent
            } else {
                surface
            };
            set_pixel(pixels, x, y, color);
        }
    }

    draw_percent_label(pixels, percent, text_color);
}

fn numeric_badge_half_width(y: i32) -> i32 {
    match y {
        0 => 0,
        1 => 9,
        2 => 12,
        3 => 14,
        4..=26 => 15,
        27 => 14,
        28 => 12,
        29 => 9,
        30 => 5,
        _ => 0,
    }
}

fn draw_percent_label(pixels: &mut [Rgba], percent: u8, color: Rgba) {
    let text = percent.to_string();
    let scale = 3;
    let width = tiny_text_width(&text, scale);
    let height = 7 * scale;
    let x = ((ICON_SIZE - width) / 2).max(0);
    let y = ((ICON_SIZE - height) / 2).max(0);

    draw_tiny_text(pixels, &text, x, y, scale, color);
}

fn tiny_text_width(text: &str, scale: i32) -> i32 {
    let chars = text.chars().count() as i32;
    if chars == 0 {
        0
    } else {
        chars * 3 * scale + (chars - 1)
    }
}

fn draw_tiny_text(pixels: &mut [Rgba], text: &str, x: i32, y: i32, scale: i32, color: Rgba) {
    let mut cursor_x = x;
    for ch in text.chars() {
        draw_tiny_char(pixels, ch, cursor_x, y, scale, color);
        cursor_x += 3 * scale + 1;
    }
}

fn draw_tiny_char(pixels: &mut [Rgba], ch: char, x: i32, y: i32, scale: i32, color: Rgba) {
    let pattern = tiny_char_pattern(ch);
    for (row, line) in pattern.iter().enumerate() {
        for (col, bit) in line.as_bytes().iter().enumerate() {
            if *bit == b'1' {
                fill_rect(
                    pixels,
                    x + col as i32 * scale,
                    y + row as i32 * scale,
                    scale,
                    scale,
                    color,
                );
            }
        }
    }
}

fn tiny_char_pattern(ch: char) -> [&'static str; 7] {
    match ch {
        '0' => ["111", "101", "101", "101", "101", "101", "111"],
        '1' => ["010", "110", "010", "010", "010", "010", "111"],
        '2' => ["111", "001", "001", "111", "100", "100", "111"],
        '3' => ["111", "001", "001", "111", "001", "001", "111"],
        '4' => ["101", "101", "101", "111", "001", "001", "001"],
        '5' => ["111", "100", "100", "111", "001", "001", "111"],
        '6' => ["111", "100", "100", "111", "101", "101", "111"],
        '7' => ["111", "001", "001", "010", "010", "010", "010"],
        '8' => ["111", "101", "101", "111", "101", "101", "111"],
        '9' => ["111", "101", "101", "111", "001", "001", "111"],
        _ => ["000", "000", "000", "000", "000", "000", "000"],
    }
}

fn create_icon_from_pixels(pixels: &[Rgba]) -> Result<Hicon, String> {
    let mut bgra = Vec::with_capacity((ICON_SIZE * ICON_SIZE * 4) as usize);
    for pixel in pixels {
        bgra.push(pixel.b);
        bgra.push(pixel.g);
        bgra.push(pixel.r);
        bgra.push(pixel.a);
    }

    let bitmap_info = BitmapInfo {
        header: BitmapInfoHeader {
            bi_size: size_of::<BitmapInfoHeader>() as u32,
            bi_width: ICON_SIZE,
            bi_height: -ICON_SIZE,
            bi_planes: 1,
            bi_bit_count: 32,
            bi_compression: BI_RGB,
            bi_size_image: bgra.len() as u32,
            bi_x_pels_per_meter: 0,
            bi_y_pels_per_meter: 0,
            bi_clr_used: 0,
            bi_clr_important: 0,
        },
        colors: [RgbQuad {
            blue: 0,
            green: 0,
            red: 0,
            reserved: 0,
        }],
    };

    let mut bits: *mut c_void = null_mut();
    let color_bitmap =
        unsafe { CreateDIBSection(0, &bitmap_info, DIB_RGB_COLORS, &mut bits, 0, 0) };
    if color_bitmap == 0 || bits.is_null() {
        return Err(format!(
            "CreateDIBSection failed: {}",
            io::Error::last_os_error()
        ));
    }

    unsafe {
        std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, bgra.len());
    }

    let mask_stride = ((ICON_SIZE + 15) / 16 * 2) as usize;
    let mask = vec![0u8; mask_stride * ICON_SIZE as usize];
    let mask_bitmap =
        unsafe { CreateBitmap(ICON_SIZE, ICON_SIZE, 1, 1, mask.as_ptr() as *const c_void) };
    if mask_bitmap == 0 {
        unsafe {
            DeleteObject(color_bitmap as Hgdiobj);
        }
        return Err(format!(
            "CreateBitmap mask failed: {}",
            io::Error::last_os_error()
        ));
    }

    let mut icon_info = IconInfo {
        icon: 1,
        x_hotspot: 0,
        y_hotspot: 0,
        mask: mask_bitmap,
        color: color_bitmap,
    };
    let icon = unsafe { CreateIconIndirect(&mut icon_info) };

    unsafe {
        DeleteObject(color_bitmap as Hgdiobj);
        DeleteObject(mask_bitmap as Hgdiobj);
    }

    if icon == 0 {
        Err(format!(
            "CreateIconIndirect failed: {}",
            io::Error::last_os_error()
        ))
    } else {
        Ok(icon)
    }
}

fn write_wide_fixed(target: &mut [u16], text: &str) {
    target.fill(0);
    let wide = wide_null(text);
    let len = wide
        .len()
        .saturating_sub(1)
        .min(target.len().saturating_sub(1));
    target[..len].copy_from_slice(&wide[..len]);
}

fn wide_null(text: &str) -> Vec<u16> {
    OsStr::new(text).encode_wide().chain(once(0)).collect()
}

fn show_error(message: &str) {
    let text = wide_null(message);
    let caption = wide_null(APP_NAME);
    unsafe {
        MessageBoxW(0, text.as_ptr(), caption.as_ptr(), MB_OK | MB_ICONERROR);
    }
}

fn show_information(message: &str) {
    let text = wide_null(message);
    let caption = wide_null(APP_NAME);
    unsafe {
        MessageBoxW(
            0,
            text.as_ptr(),
            caption.as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ConnectionKind, ICON_SIZE, Language, MENU_LANGUAGE_FIRST, MouseBattery,
        PopupLiveRefreshAction, Rgba, SingleInstanceGuard, TrayLeftClickAction, TrayStatus,
        language_from_menu_command, popup_live_refresh_action, popup_view,
        popup_was_visible_for_tray_click, render_battery_icon_pixels, status_text, tiny_text_width,
        tray_left_click_action,
    };

    #[test]
    fn every_language_has_a_menu_command() {
        for (index, language) in Language::ALL.into_iter().enumerate() {
            assert_eq!(
                language_from_menu_command(MENU_LANGUAGE_FIRST + index as u32),
                Some(language)
            );
        }
    }

    #[test]
    fn localized_wired_tooltips_fit_the_windows_limit() {
        let status = TrayStatus::Ready(MouseBattery {
            percent: 100,
            online: true,
            charging: true,
            connection: ConnectionKind::Wired,
        });

        for language in Language::ALL {
            let tooltip = status_text(&status, language);
            assert!(tooltip.encode_utf16().count() < 128, "{tooltip}");
        }
        assert!(status_text(&status, Language::ChineseSimplified).contains("充电中"));
    }

    #[test]
    fn tray_click_keeps_the_pre_deactivation_visibility() {
        assert!(popup_was_visible_for_tray_click(true, 0, 1_000));
        assert!(popup_was_visible_for_tray_click(false, 900, 1_000));
        assert!(!popup_was_visible_for_tray_click(false, 500, 1_000));
    }

    #[test]
    fn opening_the_flyout_refreshes_connection_data() {
        assert_eq!(
            tray_left_click_action(false),
            TrayLeftClickAction::RefreshAndShow
        );
        assert_eq!(tray_left_click_action(true), TrayLeftClickAction::Hide);
    }

    #[test]
    fn live_refresh_runs_only_while_the_flyout_is_visible() {
        assert_eq!(
            popup_live_refresh_action(true),
            PopupLiveRefreshAction::Refresh
        );
        assert_eq!(
            popup_live_refresh_action(false),
            PopupLiveRefreshAction::Stop
        );
    }

    #[test]
    fn disconnected_mouse_has_an_explicit_flyout_state_in_every_language() {
        let status = TrayStatus::Ready(MouseBattery {
            percent: 73,
            online: false,
            charging: false,
            connection: ConnectionKind::Receiver,
        });

        for language in Language::ALL {
            let view = popup_view(&status, language);
            assert_eq!(view.percent_text, "—");
            assert_eq!(view.primary, language.strings().disconnected);
            assert_eq!(view.connection, language.strings().receiver);
            assert!(!view.show_bar);
        }
    }

    #[test]
    fn every_battery_percentage_fits_the_numeric_tray_badge() {
        for percent in 0..=100 {
            assert!(tiny_text_width(&percent.to_string(), 3) <= ICON_SIZE - 2);
        }
    }

    #[test]
    fn numeric_tray_badge_is_high_contrast_and_does_not_animate() {
        let status = TrayStatus::Ready(MouseBattery {
            percent: 100,
            online: true,
            charging: true,
            connection: ConnectionKind::Wired,
        });

        let first = render_battery_icon_pixels(&status, true, 0);
        let later = render_battery_icon_pixels(&status, true, 17);
        assert_eq!(first, later);

        let white = Rgba::opaque(255, 255, 255);
        let dark_surface = Rgba::opaque(19, 23, 27);
        assert!(first.iter().filter(|pixel| **pixel == white).count() > 100);
        assert!(first.iter().filter(|pixel| **pixel == dark_surface).count() > 100);
    }

    #[test]
    fn named_mutex_allows_only_one_instance() {
        let name = format!(
            "Local\\NinjutsoBatteryTray.Test.Singleton.{}",
            std::process::id()
        );
        let first = SingleInstanceGuard::acquire_named(&name)
            .expect("first mutex creation should succeed")
            .expect("first mutex must be acquired");
        assert!(
            SingleInstanceGuard::acquire_named(&name)
                .expect("second mutex lookup should succeed")
                .is_none()
        );
        drop(first);
        assert!(
            SingleInstanceGuard::acquire_named(&name)
                .expect("mutex should be reusable after drop")
                .is_some()
        );
    }
}
