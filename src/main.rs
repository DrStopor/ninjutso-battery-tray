#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::ffi::{OsStr, c_void};
use std::fs;
use std::io;
use std::iter::once;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

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

const APP_NAME: &str = "Заряд мыши Ninjutso";
const DEVICE_FILTER: &str = "vid_093a&pid_eb01";
const REFRESH_TIMER_ID: usize = 1;
const CHARGE_ANIMATION_TIMER_ID: usize = 2;
const POPUP_HIDE_TIMER_ID: usize = 3;
const REFRESH_INTERVAL_MS: u32 = 60_000;
const CHARGE_ANIMATION_INTERVAL_MS: u32 = 650;
const POPUP_HIDE_MS: u32 = 6_000;
const TRAY_UID: u32 = 1;
const TRAY_CALLBACK: u32 = WM_APP + 1;
const MENU_REFRESH: u32 = 1001;
const MENU_TOGGLE_PERCENT: u32 = 1002;
const MENU_EXIT: u32 = 1003;

const INVALID_HANDLE_VALUE: Handle = -1isize;
const ERROR_NO_MORE_ITEMS: u32 = 259;

const DIGCF_PRESENT: u32 = 0x0000_0002;
const DIGCF_DEVICEINTERFACE: u32 = 0x0000_0010;
const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const FILE_SHARE_READ: u32 = 0x0000_0001;
const FILE_SHARE_WRITE: u32 = 0x0000_0002;
const OPEN_EXISTING: u32 = 3;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x0000_0080;

const WM_DESTROY: u32 = 0x0002;
const WM_PAINT: u32 = 0x000F;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_TIMER: u32 = 0x0113;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_RBUTTONUP: u32 = 0x0205;
const WM_LBUTTONDBLCLK: u32 = 0x0203;
const WM_APP: u32 = 0x8000;

const WS_POPUP: u32 = 0x8000_0000;
const WS_EX_TOPMOST: u32 = 0x0000_0008;
const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
const HWND_TOPMOST: Hwnd = -1isize;
const SW_HIDE: i32 = 0;
const SWP_NOACTIVATE: u32 = 0x0010;
const SWP_SHOWWINDOW: u32 = 0x0040;

const DT_CENTER: u32 = 0x0000_0001;
const DT_VCENTER: u32 = 0x0000_0004;
const DT_SINGLELINE: u32 = 0x0000_0020;
const TRANSPARENT: i32 = 1;
const FW_NORMAL: i32 = 400;
const FW_BOLD: i32 = 700;

const NIM_ADD: u32 = 0x0000_0000;
const NIM_MODIFY: u32 = 0x0000_0001;
const NIM_DELETE: u32 = 0x0000_0002;
const NIF_MESSAGE: u32 = 0x0000_0001;
const NIF_ICON: u32 = 0x0000_0002;
const NIF_TIP: u32 = 0x0000_0004;

const MF_STRING: u32 = 0x0000_0000;
const MF_CHECKED: u32 = 0x0000_0008;
const MF_SEPARATOR: u32 = 0x0000_0800;
const TPM_RIGHTBUTTON: u32 = 0x0000_0002;
const TPM_RETURNCMD: u32 = 0x0000_0100;

const MB_OK: u32 = 0x0000_0000;
const MB_ICONERROR: u32 = 0x0000_0010;

const BI_RGB: u32 = 0;
const DIB_RGB_COLORS: u32 = 0;
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
struct SpDeviceInterfaceData {
    cb_size: u32,
    interface_class_guid: Guid,
    flags: u32,
    reserved: usize,
}

#[repr(C)]
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

#[link(name = "hid")]
unsafe extern "system" {
    fn HidD_GetHidGuid(hid_guid: *mut Guid);
    fn HidD_SetFeature(device: Handle, report_buffer: *mut u8, report_buffer_length: u32) -> Bool;
    fn HidD_GetFeature(device: Handle, report_buffer: *mut u8, report_buffer_length: u32) -> Bool;
}

#[link(name = "setupapi")]
unsafe extern "system" {
    fn SetupDiGetClassDevsW(
        class_guid: *const Guid,
        enumerator: *const u16,
        hwnd_parent: Hwnd,
        flags: Dword,
    ) -> Handle;
    fn SetupDiEnumDeviceInterfaces(
        device_info_set: Handle,
        device_info_data: *mut c_void,
        interface_class_guid: *const Guid,
        member_index: Dword,
        device_interface_data: *mut SpDeviceInterfaceData,
    ) -> Bool;
    fn SetupDiGetDeviceInterfaceDetailW(
        device_info_set: Handle,
        device_interface_data: *mut SpDeviceInterfaceData,
        device_interface_detail_data: *mut c_void,
        device_interface_detail_data_size: Dword,
        required_size: *mut Dword,
        device_info_data: *mut c_void,
    ) -> Bool;
    fn SetupDiDestroyDeviceInfoList(device_info_set: Handle) -> Bool;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLastError() -> Dword;
    fn GetModuleHandleW(module_name: *const u16) -> Hinstance;
    fn CreateFileW(
        file_name: *const u16,
        desired_access: Dword,
        share_mode: Dword,
        security_attributes: *mut c_void,
        creation_disposition: Dword,
        flags_and_attributes: Dword,
        template_file: Handle,
    ) -> Handle;
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
    fn InvalidateRect(hwnd: Hwnd, rect: *const Rect, erase: Bool) -> Bool;
    fn UpdateWindow(hwnd: Hwnd) -> Bool;
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
    fn GetCursorPos(point: *mut Point) -> Bool;
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

#[derive(Clone, Copy)]
struct MouseBattery {
    percent: u8,
    online: bool,
    charging: bool,
}

#[derive(Clone)]
enum TrayStatus {
    Reading,
    Ready(MouseBattery),
    Unavailable(String),
}

struct AppState {
    hwnd: Hwnd,
    popup_hwnd: Hwnd,
    icon: Hicon,
    status: TrayStatus,
    show_percent_on_icon: bool,
    animation_phase: u8,
}

struct Settings {
    show_percent_on_icon: bool,
}

struct OwnedHandle(Handle);

impl OwnedHandle {
    fn new(handle: Handle) -> Result<Self, String> {
        if handle == INVALID_HANDLE_VALUE || handle == 0 {
            Err(format!(
                "CreateFileW failed: {}",
                io::Error::last_os_error()
            ))
        } else {
            Ok(Self(handle))
        }
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[derive(Clone, Copy)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
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
    if std::env::args().any(|arg| arg == "--once") {
        match read_mouse_battery() {
            Ok(status) => println!("{}", status_text(&TrayStatus::Ready(status))),
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        return;
    }

    if let Err(err) = run_tray() {
        show_error(&err);
    }
}

fn run_tray() -> Result<(), String> {
    let class_name = wide_null("NinjutsoMouseBatteryTrayWindow");
    let window_name = wide_null(APP_NAME);
    let instance = unsafe { GetModuleHandleW(null()) };

    let wnd_class = WndClassW {
        style: 0,
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

    let settings = load_settings();
    let initial_icon = create_battery_icon(&TrayStatus::Reading, settings.show_percent_on_icon, 0)?;
    APP_STATE
        .set(Mutex::new(AppState {
            hwnd,
            popup_hwnd: 0,
            icon: initial_icon,
            status: TrayStatus::Reading,
            show_percent_on_icon: settings.show_percent_on_icon,
            animation_phase: 0,
        }))
        .map_err(|_| "Application state is already initialized".to_string())?;

    add_or_update_tray_icon(NIM_ADD)?;
    refresh_tray();

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
        WM_TIMER if w_param == POPUP_HIDE_TIMER_ID => {
            hide_status_popup();
            0
        }
        WM_PAINT if is_status_popup(hwnd) => {
            paint_status_popup(hwnd);
            0
        }
        WM_ERASEBKGND if is_status_popup(hwnd) => 1,
        WM_LBUTTONDOWN if is_status_popup(hwnd) => {
            hide_status_popup();
            0
        }
        TRAY_CALLBACK => {
            let event = l_param as u32;
            if event == WM_RBUTTONUP {
                show_context_menu(hwnd);
            } else if event == WM_LBUTTONUP {
                show_status_popup(hwnd);
            } else if event == WM_LBUTTONDBLCLK {
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
        if menu == 0 {
            return;
        }

        let refresh = wide_null("Обновить");
        let toggle_percent = wide_null("Показать процент");
        let exit = wide_null("Выход");
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
            } else if command == MENU_EXIT {
                DestroyWindow(hwnd);
            }
        }

        DestroyMenu(menu);
    }
}

fn is_status_popup(hwnd: Hwnd) -> bool {
    hwnd != 0
        && APP_STATE
            .get()
            .and_then(|state_lock| state_lock.lock().ok())
            .map(|state| state.popup_hwnd == hwnd)
            .unwrap_or(false)
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

    let width = 280;
    let height = 150;
    let x = (point.x - width + 24).max(8);
    let y = (point.y - height - 18).max(8);

    unsafe {
        InvalidateRect(popup_hwnd, null(), 0);
        SetWindowPos(
            popup_hwnd,
            HWND_TOPMOST,
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        UpdateWindow(popup_hwnd);
        SetTimer(owner_hwnd, POPUP_HIDE_TIMER_ID, POPUP_HIDE_MS, null_mut());
    }
}

fn ensure_status_popup(owner_hwnd: Hwnd) -> Hwnd {
    if let Some(existing) = APP_STATE
        .get()
        .and_then(|state_lock| state_lock.lock().ok())
        .map(|state| state.popup_hwnd)
    {
        if existing != 0 {
            return existing;
        }
    }

    let class_name = wide_null("NinjutsoMouseBatteryTrayWindow");
    let window_name = wide_null(APP_NAME);
    let instance = unsafe { GetModuleHandleW(null()) };
    let popup_hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            window_name.as_ptr(),
            WS_POPUP,
            0,
            0,
            280,
            150,
            owner_hwnd,
            0,
            instance,
            null_mut(),
        )
    };

    if popup_hwnd != 0 {
        if let Some(state_lock) = APP_STATE.get() {
            if let Ok(mut state) = state_lock.lock() {
                state.popup_hwnd = popup_hwnd;
            }
        }
    }

    popup_hwnd
}

fn hide_status_popup() {
    let Some((main_hwnd, popup_hwnd)) = APP_STATE
        .get()
        .and_then(|state_lock| state_lock.lock().ok())
        .map(|state| (state.hwnd, state.popup_hwnd))
    else {
        return;
    };

    unsafe {
        KillTimer(main_hwnd, POPUP_HIDE_TIMER_ID);
        if popup_hwnd != 0 {
            ShowWindow(popup_hwnd, SW_HIDE);
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
    let status = APP_STATE
        .get()
        .and_then(|state_lock| state_lock.lock().ok())
        .map(|state| state.status.clone())
        .unwrap_or(TrayStatus::Reading);

    let mut paint: PaintStruct = unsafe { zeroed() };
    let hdc = unsafe { BeginPaint(hwnd, &mut paint) };
    if hdc == 0 {
        return;
    }

    draw_status_popup(hdc, &status);

    unsafe {
        EndPaint(hwnd, &paint);
    }
}

fn draw_status_popup(hdc: Hdc, status: &TrayStatus) {
    let background = rgb(22, 25, 29);
    let panel = rgb(34, 39, 45);
    let text = rgb(245, 247, 250);
    let muted = rgb(170, 178, 188);

    fill_dc_rect(hdc, Rect::new(0, 0, 280, 150), background);
    fill_dc_rect(hdc, Rect::new(8, 8, 272, 142), panel);

    draw_dc_text(
        hdc,
        "Мышь Ninjutso",
        Rect::new(18, 12, 262, 34),
        -18,
        FW_BOLD,
        text,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );

    let (percent, percent_text, status_text, fill_color) = match status {
        TrayStatus::Ready(battery) => {
            let fill = if battery.percent >= 50 {
                rgb(35, 190, 85)
            } else if battery.percent >= 20 {
                rgb(238, 180, 38)
            } else {
                rgb(220, 55, 55)
            };
            let connection = if battery.online {
                "подключена"
            } else {
                "не подключена"
            };
            let charge = if battery.charging {
                "заряжается"
            } else {
                "не заряжается"
            };
            (
                battery.percent.min(100),
                format!("{}%", battery.percent.min(100)),
                format!("{connection} · {charge}"),
                fill,
            )
        }
        TrayStatus::Reading => (
            0,
            "...".to_string(),
            "чтение заряда".to_string(),
            rgb(100, 145, 205),
        ),
        TrayStatus::Unavailable(err) => {
            let short = err.lines().next().unwrap_or("не доступно");
            (0, "--".to_string(), short.to_string(), rgb(120, 120, 120))
        }
    };

    draw_dc_text(
        hdc,
        &percent_text,
        Rect::new(18, 42, 138, 98),
        -52,
        FW_BOLD,
        text,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );

    draw_dc_text(
        hdc,
        &status_text,
        Rect::new(144, 42, 260, 70),
        -17,
        FW_NORMAL,
        muted,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );

    draw_popup_battery_bar(hdc, percent, fill_color);

    draw_dc_text(
        hdc,
        "ЛКМ по окну скрывает его",
        Rect::new(18, 120, 262, 138),
        -13,
        FW_NORMAL,
        muted,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );
}

fn draw_popup_battery_bar(hdc: Hdc, percent: u8, fill_color: Dword) {
    let border = rgb(238, 242, 246);
    let empty = rgb(20, 23, 27);
    let shadow = rgb(0, 0, 0);

    fill_dc_rect(hdc, Rect::new(146, 80, 254, 114), shadow);
    fill_dc_rect(hdc, Rect::new(148, 78, 252, 112), border);
    fill_dc_rect(hdc, Rect::new(153, 83, 247, 107), empty);
    fill_dc_rect(hdc, Rect::new(252, 88, 262, 102), border);
    fill_dc_rect(hdc, Rect::new(252, 92, 258, 98), empty);

    let fill_width = ((94u16 * percent as u16 + 99) / 100) as i32;
    if fill_width > 0 {
        fill_dc_rect(
            hdc,
            Rect::new(153, 83, 153 + fill_width.min(94), 107),
            fill_color,
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

fn draw_dc_text(
    hdc: Hdc,
    text: &str,
    mut rect: Rect,
    font_height: i32,
    weight: i32,
    color: Dword,
    flags: u32,
) {
    let face = wide_null("Segoe UI");
    let font = unsafe {
        CreateFontW(
            font_height,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            1,
            0,
            0,
            0,
            0,
            face.as_ptr(),
        )
    };
    if font == 0 {
        return;
    }

    let text_wide = wide_null(text);
    unsafe {
        let old_font = SelectObject(hdc, font as Hgdiobj);
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, color);
        DrawTextW(
            hdc,
            text_wide.as_ptr(),
            text_wide.len().saturating_sub(1) as i32,
            &mut rect,
            flags,
        );
        SelectObject(hdc, old_font);
        DeleteObject(font as Hgdiobj);
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

fn current_icon_options() -> (bool, u8) {
    APP_STATE
        .get()
        .and_then(|state_lock| state_lock.lock().ok())
        .map(|state| (state.show_percent_on_icon, state.animation_phase))
        .unwrap_or((true, 0))
}

fn toggle_percent_on_icon() {
    let new_value = !current_show_percent_on_icon();

    if let Some(state_lock) = APP_STATE.get() {
        if let Ok(mut state) = state_lock.lock() {
            state.show_percent_on_icon = new_value;
        }
    }

    let _ = save_settings(&Settings {
        show_percent_on_icon: new_value,
    });
    refresh_tray();
}

fn tick_charge_animation() {
    let Some(state_lock) = APP_STATE.get() else {
        return;
    };

    let Ok(mut state) = state_lock.lock() else {
        return;
    };

    let charging = matches!(&state.status, TrayStatus::Ready(battery) if battery.charging);
    if !charging {
        state.animation_phase = 0;
        return;
    }

    state.animation_phase = state.animation_phase.wrapping_add(1);
    let status = state.status.clone();
    let show_percent_on_icon = state.show_percent_on_icon;
    let animation_phase = state.animation_phase;
    drop(state);

    replace_tray_icon(status, show_percent_on_icon, animation_phase);
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
            KillTimer(hwnd, POPUP_HIDE_TIMER_ID);
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
    replace_tray_icon(status, show_percent_on_icon, animation_phase);
}

fn replace_tray_icon(status: TrayStatus, show_percent_on_icon: bool, animation_phase: u8) {
    let icon_phase = if matches!(&status, TrayStatus::Ready(battery) if battery.charging) {
        animation_phase
    } else {
        0
    };
    let Ok(new_icon) = create_battery_icon(&status, show_percent_on_icon, icon_phase) else {
        return;
    };

    if let Some(state_lock) = APP_STATE.get() {
        if let Ok(mut state) = state_lock.lock() {
            let old_icon = state.icon;
            state.icon = new_icon;
            state.status = status;
            state.animation_phase = icon_phase;
            drop(state);

            let _ = add_or_update_tray_icon(NIM_MODIFY);
            invalidate_status_popup();

            if old_icon != 0 {
                unsafe {
                    DestroyIcon(old_icon);
                }
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
    write_wide_fixed(&mut data.tip, &status_text(&state.status));

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

fn status_text(status: &TrayStatus) -> String {
    match status {
        TrayStatus::Reading => "Мышь Ninjutso: чтение заряда...".to_string(),
        TrayStatus::Ready(battery) => {
            let online = if battery.online {
                "подключена"
            } else {
                "не подключена"
            };
            let charging = if battery.charging {
                "заряжается"
            } else {
                "не заряжается"
            };
            format!("Мышь Ninjutso: {}% ({online}, {charging})", battery.percent)
        }
        TrayStatus::Unavailable(err) => {
            let short = err.lines().next().unwrap_or("not available");
            format!("Заряд мыши Ninjutso: {short}")
        }
    }
}

fn read_mouse_battery() -> Result<MouseBattery, String> {
    let paths = enumerate_hid_paths(DEVICE_FILTER)?;
    if paths.is_empty() {
        return Err("device not found".to_string());
    }

    let mut last_error = None;
    for path in paths {
        match query_battery_path(&path) {
            Ok(status) => return Ok(status),
            Err(err) => last_error = Some(err),
        }
    }

    Err(last_error.unwrap_or_else(|| "no HID collection returned battery data".to_string()))
}

fn query_battery_path(path: &str) -> Result<MouseBattery, String> {
    let online = query_feature(path, 19)?;
    let battery = query_feature(path, 18)?;
    let charge = query_feature(path, 17)?;

    if online[1] != 19 || battery[1] != 18 || charge[1] != 17 {
        return Err("unexpected HID report response".to_string());
    }

    Ok(MouseBattery {
        percent: battery[8],
        online: online[8] == 1,
        charging: charge[8] != 0,
    })
}

fn query_feature(path: &str, command: u8) -> Result<[u8; 16], String> {
    let wide_path = wide_null(path);
    let handle = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            0,
        )
    };
    let handle = OwnedHandle::new(handle)?;

    let mut out_report = [0u8; 16];
    out_report[0] = 6;
    out_report[1] = command;
    out_report[4] = 1;

    let ok = unsafe { HidD_SetFeature(handle.0, out_report.as_mut_ptr(), out_report.len() as u32) };
    if ok == 0 {
        return Err(format!(
            "HidD_SetFeature command {command} failed: {}",
            io::Error::last_os_error()
        ));
    }

    thread::sleep(Duration::from_millis(30));

    let mut in_report = [0u8; 16];
    in_report[0] = 6;
    let ok = unsafe { HidD_GetFeature(handle.0, in_report.as_mut_ptr(), in_report.len() as u32) };
    if ok == 0 {
        return Err(format!(
            "HidD_GetFeature command {command} failed: {}",
            io::Error::last_os_error()
        ));
    }

    Ok(in_report)
}

fn enumerate_hid_paths(filter: &str) -> Result<Vec<String>, String> {
    let mut hid_guid = Guid::default();
    unsafe {
        HidD_GetHidGuid(&mut hid_guid);
    }

    let info = unsafe {
        SetupDiGetClassDevsW(&hid_guid, null(), 0, DIGCF_PRESENT | DIGCF_DEVICEINTERFACE)
    };
    if info == INVALID_HANDLE_VALUE {
        return Err(format!(
            "SetupDiGetClassDevsW failed: {}",
            io::Error::last_os_error()
        ));
    }

    let mut paths = Vec::new();
    let mut index = 0;
    loop {
        let mut iface = SpDeviceInterfaceData {
            cb_size: size_of::<SpDeviceInterfaceData>() as u32,
            interface_class_guid: Guid::default(),
            flags: 0,
            reserved: 0,
        };

        let ok =
            unsafe { SetupDiEnumDeviceInterfaces(info, null_mut(), &hid_guid, index, &mut iface) };
        if ok == 0 {
            let err = unsafe { GetLastError() };
            if err == ERROR_NO_MORE_ITEMS {
                break;
            }
            unsafe {
                SetupDiDestroyDeviceInfoList(info);
            }
            return Err(format!(
                "SetupDiEnumDeviceInterfaces failed: {}",
                io::Error::from_raw_os_error(err as i32)
            ));
        }

        let mut required = 0u32;
        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                info,
                &mut iface,
                null_mut(),
                0,
                &mut required,
                null_mut(),
            );
        }

        if required > 0 {
            let mut detail = vec![0u8; required as usize];
            let cb_size = if size_of::<usize>() == 8 { 8u32 } else { 6u32 };
            unsafe {
                (detail.as_mut_ptr() as *mut u32).write_unaligned(cb_size);
            }

            let ok = unsafe {
                SetupDiGetDeviceInterfaceDetailW(
                    info,
                    &mut iface,
                    detail.as_mut_ptr() as *mut c_void,
                    required,
                    &mut required,
                    null_mut(),
                )
            };

            if ok != 0 {
                let path = unsafe { wide_ptr_to_string(detail.as_ptr().add(4) as *const u16) };
                if path.to_ascii_lowercase().contains(filter) {
                    paths.push(path);
                }
            }
        }

        index += 1;
    }

    unsafe {
        SetupDiDestroyDeviceInfoList(info);
    }
    Ok(paths)
}

unsafe fn wide_ptr_to_string(ptr: *const u16) -> String {
    let mut len = 0usize;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16_lossy(slice)
}

fn create_battery_icon(
    status: &TrayStatus,
    show_percent_on_icon: bool,
    animation_phase: u8,
) -> Result<Hicon, String> {
    let mut pixels = vec![Rgba::transparent(); (ICON_SIZE * ICON_SIZE) as usize];

    let (percent, fill_color, empty_color, charging) = match status {
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
                battery.charging,
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

    draw_large_battery(
        &mut pixels,
        percent,
        fill_color,
        empty_color,
        black,
        white,
        charging,
        animation_phase,
    );

    if show_percent_on_icon {
        if let TrayStatus::Ready(battery) = status {
            draw_percent_label(&mut pixels, battery.percent.min(100), black, white);
        }
    }

    create_icon_from_pixels(&pixels)
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

fn draw_large_battery(
    pixels: &mut [Rgba],
    percent: u8,
    fill_color: Rgba,
    empty_color: Rgba,
    outline: Rgba,
    border: Rgba,
    charging: bool,
    animation_phase: u8,
) {
    let center_x = 16;
    let inner_top = 5;
    let inner_bottom = 28;
    let inner_height = inner_bottom - inner_top + 1;
    let fill_height = ((inner_height as u16 * percent as u16 + 99) / 100) as i32;
    let fill_top = inner_bottom + 1 - fill_height.clamp(1, inner_height);

    for y in 0..ICON_SIZE {
        let half = mouse_body_half_width(y);
        if half <= 0 {
            continue;
        }
        for x in (center_x - half)..=(center_x + half) {
            set_pixel(pixels, x, y, outline);
        }
    }

    for y in 0..ICON_SIZE {
        let half = mouse_body_half_width(y) - 2;
        if half <= 0 {
            continue;
        }
        for x in (center_x - half)..=(center_x + half) {
            set_pixel(pixels, x, y, border);
        }
    }

    for y in 0..ICON_SIZE {
        let half = mouse_body_half_width(y) - 4;
        if half <= 0 {
            continue;
        }
        for x in (center_x - half)..=(center_x + half) {
            let color = if y >= fill_top {
                fill_color
            } else {
                empty_color
            };
            set_pixel(pixels, x, y, color);
        }
    }

    if charging {
        draw_mouse_charge_wave(pixels, fill_top, fill_color, animation_phase);
    }

    fill_rect(pixels, center_x - 1, 4, 2, 7, outline);
    fill_rect(pixels, center_x - 6, 11, 13, 2, outline);
    fill_rect(pixels, center_x - 1, 13, 2, 8, outline);
    fill_rect(pixels, center_x - 1, 5, 2, 5, border);
}

fn mouse_body_half_width(y: i32) -> i32 {
    match y {
        0 => 0,
        1 => 4,
        2 => 6,
        3 => 8,
        4..=22 => 11,
        23..=24 => 10,
        25 => 9,
        26 => 8,
        27 => 6,
        28 => 5,
        29 => 3,
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
        let half = mouse_body_half_width(y) - 4;
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

fn draw_percent_label(pixels: &mut [Rgba], percent: u8, shadow: Rgba, color: Rgba) {
    let text = percent.to_string();
    let scale = if text.len() >= 3 { 3 } else { 4 };
    let width = tiny_text_width(&text, scale);
    let height = 7 * scale;
    let x = ((ICON_SIZE - width) / 2).max(0);
    let y = ((ICON_SIZE - height) / 2).max(0);

    draw_tiny_text(pixels, &text, x - 1, y - 1, scale, shadow);
    draw_tiny_text(pixels, &text, x, y - 1, scale, shadow);
    draw_tiny_text(pixels, &text, x + 1, y - 1, scale, shadow);
    draw_tiny_text(pixels, &text, x - 1, y, scale, shadow);
    draw_tiny_text(pixels, &text, x + 1, y, scale, shadow);
    draw_tiny_text(pixels, &text, x - 1, y + 1, scale, shadow);
    draw_tiny_text(pixels, &text, x + 1, y + 1, scale, shadow);
    draw_tiny_text(pixels, &text, x, y + 1, scale, shadow);
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

fn load_settings() -> Settings {
    let default = Settings {
        show_percent_on_icon: true,
    };

    let Some(path) = settings_path() else {
        return default;
    };

    let Ok(text) = fs::read_to_string(path) else {
        return default;
    };

    for line in text.lines() {
        if let Some(value) = line.trim().strip_prefix("large_icon_percent=") {
            return Settings {
                show_percent_on_icon: matches!(value.trim(), "true" | "1" | "yes" | "on"),
            };
        }
    }

    default
}

fn save_settings(settings: &Settings) -> Result<(), String> {
    let path = settings_path().ok_or_else(|| "APPDATA is not available".to_string())?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Не удалось создать папку настроек: {err}"))?;
    }

    let value = if settings.show_percent_on_icon {
        "true"
    } else {
        "false"
    };
    fs::write(path, format!("large_icon_percent={value}\n"))
        .map_err(|err| format!("Не удалось сохранить настройки: {err}"))
}

fn settings_path() -> Option<PathBuf> {
    env::var_os("APPDATA").map(|appdata| {
        PathBuf::from(appdata)
            .join("NinjutsoBatteryTray")
            .join("settings.txt")
    })
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
