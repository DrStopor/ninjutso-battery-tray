//! The Windows notification area has its own message pump, never the GPUI thread.
use std::cell::RefCell;
use std::ffi::c_void;
use std::mem::size_of;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use async_channel::Sender as EventSender;
use windows::Win32::Foundation::{GetLastError, HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{GUID, PCWSTR};

use super::context_menu;
use super::native::{create_tray_icon, icon_anchor, wide};
use crate::application::AppEvent;
use crate::model::{AppModel, BatteryStatus};
use crate::presentation::icons::render_battery_icon_pixels;
use crate::presentation::layout::Point;
use crate::presentation::menu::tray_menu;
use crate::presentation::theme::{MenuPalette, Theme};
use crate::presentation::view::status_text;
use crate::settings::Settings;

const TRAY_ID: u32 = 1;
const TRAY_CALLBACK: u32 = WM_APP + 1;
const SERVICE_COMMAND: u32 = WM_APP + 2;
const ANIMATION_TIMER: usize = 1;
const DEVICE_TIMER: usize = 2;
// The SDK spells this as a macro, which windows-rs does not export.
const NIN_KEYSELECT: u32 = NIN_SELECT | 1;

enum Command {
    Update(AppModel),
    ShowMenu {
        settings: Settings,
        anchor: Point,
        palette: MenuPalette,
    },
    Stop,
}

pub(crate) struct TrayService {
    hwnd: isize,
    sender: Sender<Command>,
    done: Receiver<()>,
    thread: Option<JoinHandle<()>>,
}

impl TrayService {
    pub(crate) fn start(model: AppModel, events: EventSender<AppEvent>) -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (done_tx, done) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("ninjutso-tray".into())
            .spawn(move || {
                if let Err(error) = message_loop(model, events.clone(), receiver, &ready_tx) {
                    let _ = ready_tx.send(Err(error));
                    let _ = events.try_send(AppEvent::Quit);
                }
                let _ = done_tx.send(());
            })
            .map_err(|error| error.to_string())?;
        let hwnd = ready_rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|error| format!("Tray startup: {error}"))??;
        Ok(Self {
            hwnd,
            sender,
            done,
            thread: Some(thread),
        })
    }

    pub(crate) fn update(&self, model: &AppModel) {
        if self.sender.send(Command::Update(model.clone())).is_ok() {
            self.wake();
        }
    }

    pub(crate) fn show_menu(&self, settings: Settings, anchor: Point, palette: MenuPalette) {
        if self
            .sender
            .send(Command::ShowMenu {
                settings,
                anchor,
                palette,
            })
            .is_ok()
        {
            self.wake();
        }
    }

    fn wake(&self) {
        let _ = unsafe {
            PostMessageW(
                Some(HWND(self.hwnd as *mut c_void)),
                SERVICE_COMMAND,
                WPARAM(0),
                LPARAM(0),
            )
        };
    }
}

impl Drop for TrayService {
    fn drop(&mut self) {
        let _ = self.sender.send(Command::Stop);
        self.wake();
        // A native context menu or shell extension must not block shutdown forever.
        if self.done.recv_timeout(Duration::from_millis(500)).is_ok()
            && let Some(thread) = self.thread.take()
        {
            let _ = thread.join();
        }
    }
}

struct TrayState {
    model: AppModel,
    events: EventSender<AppEvent>,
    commands: Receiver<Command>,
    icon: HICON,
    taskbar_created: u32,
    version4: bool,
    window_alive: bool,
}

impl TrayState {
    fn emit(&self, event: AppEvent) {
        let _ = self.events.try_send(event);
    }

    fn update_icon(&mut self, hwnd: HWND, add: bool) -> Result<(), String> {
        let pixels = render_battery_icon_pixels(
            &self.model.status,
            self.model.settings.show_percent_on_icon,
            self.model.animation_phase,
            &Theme::DARK,
        );
        let icon = create_tray_icon(&pixels)?;
        let mut data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ID,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
            uCallbackMessage: TRAY_CALLBACK,
            hIcon: icon,
            ..Default::default()
        };
        let tooltip = wide(&status_text(
            &self.model.status,
            self.model.settings.language,
        ));
        for (target, source) in data.szTip.iter_mut().take(127).zip(tooltip) {
            *target = source;
        }
        if !unsafe { Shell_NotifyIconW(if add { NIM_ADD } else { NIM_MODIFY }, &data) }.as_bool() {
            let _ = unsafe { DestroyIcon(icon) };
            return Err(format!("Shell_NotifyIconW: {:?}", unsafe {
                GetLastError()
            }));
        }
        if add {
            data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            self.version4 = unsafe { Shell_NotifyIconW(NIM_SETVERSION, &data) }.as_bool();
        }
        if !self.icon.is_invalid() {
            let _ = unsafe { DestroyIcon(self.icon) };
        }
        self.icon = icon;
        let animate = !self.model.settings.show_percent_on_icon
            && matches!(self.model.status, BatteryStatus::Ready(b) if b.online && b.charging);
        unsafe {
            if animate {
                SetTimer(Some(hwnd), ANIMATION_TIMER, 650, None);
            } else {
                let _ = KillTimer(Some(hwnd), ANIMATION_TIMER);
            }
        }
        Ok(())
    }
}

fn message_loop(
    model: AppModel,
    events: EventSender<AppEvent>,
    commands: Receiver<Command>,
    ready: &mpsc::SyncSender<Result<isize, String>>,
) -> Result<(), String> {
    let class = wide("NinjutsoBatteryTray.GpuiHost.v1");
    let title = wide(crate::APP_NAME);
    let module = unsafe { GetModuleHandleW(None) }.map_err(|error| error.to_string())?;
    let instance = module.into();
    let atom = unsafe {
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: PCWSTR(class.as_ptr()),
            ..Default::default()
        })
    };
    if atom == 0 {
        return Err(format!("RegisterClassW: {:?}", unsafe { GetLastError() }));
    }
    // A hidden top-level window, not HWND_MESSAGE: Explorer broadcasts TaskbarCreated.
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            PCWSTR(class.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance),
            None,
        )
    }
    .map_err(|error| error.to_string())?;
    let taskbar = wide("TaskbarCreated");
    let state = Box::new(RefCell::new(TrayState {
        model,
        events,
        commands,
        icon: HICON::default(),
        taskbar_created: unsafe { RegisterWindowMessageW(PCWSTR(taskbar.as_ptr())) },
        version4: false,
        window_alive: true,
    }));
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, &*state as *const _ as isize);
    }
    if let Err(error) = state.borrow_mut().update_icon(hwnd, true) {
        let _ = unsafe { DestroyWindow(hwnd) };
        return Err(error);
    }
    let filter = DEV_BROADCAST_DEVICEINTERFACE_W {
        dbcc_size: size_of::<DEV_BROADCAST_DEVICEINTERFACE_W>() as u32,
        dbcc_devicetype: DBT_DEVTYP_DEVICEINTERFACE.0,
        dbcc_classguid: GUID::from_u128(0x4d1e55b2_f16f_11cf_88cb_001111000030),
        ..Default::default()
    };
    let notification = unsafe {
        RegisterDeviceNotificationW(
            HANDLE(hwnd.0),
            &filter as *const _ as *const c_void,
            DEVICE_NOTIFY_WINDOW_HANDLE,
        )
    }
    .ok();
    let _ = ready.send(Ok(hwnd.0 as isize));
    let mut message = MSG::default();
    loop {
        let result = unsafe { GetMessageW(&mut message, None, 0, 0) }.0;
        if result <= 0 {
            break;
        }
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    if let Some(notification) = notification {
        let _ = unsafe { UnregisterDeviceNotification(notification) };
    }
    let data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ID,
        ..Default::default()
    };
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
        // GetMessage can fail or receive WM_QUIT without WM_DESTROY. Do not leave
        // a window procedure pointing into the Box after the message loop ends.
        if state.borrow().window_alive {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(hwnd);
        }
        if !state.borrow().icon.is_invalid() {
            let _ = DestroyIcon(state.borrow().icon);
        }
        let _ = UnregisterClassW(PCWSTR(class.as_ptr()), Some(instance));
    }
    Ok(())
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if let Some(result) = unsafe { context_menu::handle_message(message, wparam, lparam) } {
        return result;
    }
    if message == WM_DESTROY {
        unsafe {
            PostQuitMessage(0);
        }
        return LRESULT(0);
    }
    let pointer = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const RefCell<TrayState>;
    if pointer.is_null() {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    // OS calls can synchronously reenter this procedure; never alias a mutable state reference.
    let Ok(mut state) = (unsafe { &*pointer }).try_borrow_mut() else {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    };
    if message == SERVICE_COMMAND {
        let mut stop = false;
        let mut menu = None;
        while let Ok(command) = state.commands.try_recv() {
            match command {
                Command::Update(model) => {
                    state.model = model;
                    let _ = state.update_icon(hwnd, false);
                }
                Command::ShowMenu {
                    settings,
                    anchor,
                    palette,
                } => {
                    menu = Some((settings, anchor, palette));
                }
                Command::Stop => {
                    stop = true;
                    break;
                }
            }
        }
        let events = state.events.clone();
        drop(state);
        if stop {
            let _ = unsafe { EndMenu() };
            let _ = unsafe { DestroyWindow(hwnd) };
        } else if let Some((settings, anchor, palette)) = menu
            && let Some(command) = context_menu::show(hwnd, &tray_menu(settings), anchor, palette)
        {
            let _ = events.try_send(command.into());
        }
        return LRESULT(0);
    }
    if message == state.taskbar_created {
        let _ = state.update_icon(hwnd, true);
        return LRESULT(0);
    }
    match message {
        WM_NCDESTROY => {
            state.window_alive = false;
            drop(state);
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
        }
        TRAY_CALLBACK => {
            let raw = (lparam.0 & 0xffff) as u32;
            let event = match raw {
                WM_LBUTTONUP if !state.version4 => NIN_SELECT,
                WM_RBUTTONUP if !state.version4 => WM_CONTEXTMENU,
                _ => raw,
            };
            match event {
                NIN_SELECT | NIN_KEYSELECT => state.emit(AppEvent::ToggleFlyout {
                    anchor: icon_anchor(hwnd, TRAY_ID),
                    keyboard: event == NIN_KEYSELECT,
                }),
                WM_CONTEXTMENU => {
                    state.emit(AppEvent::ShowContextMenu {
                        anchor: icon_anchor(hwnd, TRAY_ID),
                    });
                }
                _ => {}
            }
        }
        WM_TIMER if wparam.0 == ANIMATION_TIMER => {
            state.model.animation_phase = (state.model.animation_phase + 1) % 3;
            let _ = state.update_icon(hwnd, false);
        }
        WM_DEVICECHANGE if matches!(wparam.0, 0x8000 | 0x8004 | 7) => {
            // Let SetupAPI settle and coalesce the burst for a composite HID device.
            unsafe {
                SetTimer(Some(hwnd), DEVICE_TIMER, 300, None);
            }
        }
        WM_TIMER if wparam.0 == DEVICE_TIMER => {
            let _ = unsafe { KillTimer(Some(hwnd), DEVICE_TIMER) };
            state.emit(AppEvent::Refresh);
        }
        WM_POWERBROADCAST if matches!(wparam.0, 7 | 18) => state.emit(AppEvent::Refresh),
        WM_COMMAND if lparam.0 == 0 => {
            if let Some(event) = menu_event((wparam.0 & 0xffff) as u32) {
                state.emit(event);
            }
        }
        WM_CLOSE => {
            state.emit(AppEvent::Quit);
        }
        _ => {
            drop(state);
            return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
        }
    }
    LRESULT(0)
}

fn menu_event(command: u32) -> Option<AppEvent> {
    context_menu::command_from_id(command).map(Into::into)
}
