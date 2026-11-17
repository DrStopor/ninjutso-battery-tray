//! GPUI owns the UI thread. The notification area and HID reader are independent services.
mod assets;
mod card;
mod fonts;
mod typography;

use async_channel::Sender;
use gpui::{
    App, AppContext, Application, Bounds, Context, WindowAppearance, WindowBounds, WindowHandle,
    WindowKind, WindowOptions, px, size,
};
use std::time::Instant;

use crate::application::{AppEvent, FlyoutAction, FlyoutSession};
use crate::battery_service::BatteryService;
use crate::i18n::Language;
use crate::model::{AppModel, BatteryStatus, Dpi, PollingRate};
use crate::platform::desktop::{
    self as shell, SingleInstanceGuard, TrayService, load_settings, read_mouse_battery,
    save_settings,
};
use crate::presentation::layout::Point;
use crate::presentation::theme::Theme;
use crate::presentation::view::status_text;
use crate::settings::ThemePreference;
use card::BatteryCard;

pub fn run() {
    let args: Vec<_> = std::env::args().collect();
    let mut settings = load_settings();
    if args.iter().any(|arg| arg == "--once") {
        match read_mouse_battery() {
            Ok(reading) => println!(
                "{}",
                status_text(&BatteryStatus::Ready(reading), settings.language)
            ),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    let preview = cfg!(debug_assertions) && args.iter().any(|arg| arg == "--preview");
    let option = |name: &str| args.iter().find_map(|arg| arg.strip_prefix(name));
    if preview && let Some(language) = option("--preview-language=").and_then(Language::from_code) {
        settings.language = language;
    }
    let _instance = if preview {
        None
    } else {
        match SingleInstanceGuard::acquire() {
            Ok(Some(instance)) => Some(instance),
            Ok(None) => {
                shell::show_information(settings.language.strings().already_running);
                return;
            }
            Err(error) => {
                shell::show_error(&error);
                return;
            }
        }
    };
    if preview && let Some(theme) = option("--preview-theme=").and_then(ThemePreference::from_code)
    {
        settings.theme = theme;
    }
    let preview_status = if preview { preview_status(&args) } else { None };
    let cycle = preview && args.iter().any(|arg| arg == "--preview-cycle");
    let mut model = AppModel::new(settings);
    if let Some(status) = preview_status.clone() {
        model.status = status;
    }
    let (sender, receiver) = async_channel::unbounded();
    let tray = match TrayService::start(model.clone(), sender.clone()) {
        Ok(tray) => tray,
        Err(error) => {
            shell::show_error(&error);
            return;
        }
    };
    let worker = if preview_status.is_none() || cycle {
        let battery_events = sender.clone();
        let cycle_clock = Instant::now();
        match BatteryService::start(
            move || {
                if cycle {
                    preview_cycle(cycle_clock.elapsed().as_secs())
                } else {
                    read_mouse_battery()
                        .map_or_else(BatteryStatus::Unavailable, BatteryStatus::Ready)
                }
            },
            move |status| battery_events.try_send(AppEvent::Battery(status)).is_ok(),
        ) {
            Ok(worker) => Some(worker),
            Err(error) => {
                shell::show_error(&error.to_string());
                return;
            }
        }
    } else {
        None
    };
    Application::new()
        .with_assets(assets::Assets)
        .run(move |cx: &mut App| {
            if let Err(error) = fonts::register(cx) {
                shell::show_error(&error.to_string());
                cx.quit();
                return;
            }
            let controller = cx.new(|_| Controller {
                model,
                session: FlyoutSession::default(),
                flyout: None,
                tray: Some(tray),
                worker,
                events: sender.clone(),
                clock: Instant::now(),
                preview,
            });
            if preview {
                controller.update(cx, |controller, cx| {
                    controller.event(
                        AppEvent::ToggleFlyout {
                            anchor: shell::cursor_position(),
                            keyboard: true,
                        },
                        cx,
                    )
                });
            }
            cx.spawn(async move |cx| {
                while let Ok(event) = receiver.recv().await {
                    let quit = matches!(event, AppEvent::Quit);
                    if controller
                        .update(cx, |controller, cx| controller.event(event, cx))
                        .is_err()
                        || quit
                    {
                        break;
                    }
                }
            })
            .detach();
        });
}

fn preview_cycle(seconds: u64) -> BatteryStatus {
    use crate::model::{ConnectionKind, MouseBattery, ReadError, ReadErrorKind};
    let phase = seconds % 9;
    match phase {
        0..=4 => BatteryStatus::Ready(MouseBattery {
            percent: if phase <= 2 { 72 } else { 70 + phase as u8 },
            online: true,
            charging: phase == 3,
            connection: if phase <= 2 {
                ConnectionKind::Receiver
            } else {
                ConnectionKind::Wired
            },
            polling_rate: Some(if (1..=2).contains(&phase) {
                PollingRate::Hz4000
            } else {
                PollingRate::Hz1000
            }),
            dpi: Dpi::from_value(if phase == 2 { 1600 } else { 1100 }),
        }),
        5 => BatteryStatus::Ready(MouseBattery {
            percent: 74,
            online: false,
            charging: false,
            connection: ConnectionKind::Receiver,
            polling_rate: None,
            dpi: None,
        }),
        6 => BatteryStatus::Unavailable(ReadError::new(ReadErrorKind::DeviceNotFound, "Preview")),
        7 => BatteryStatus::Unavailable(ReadError::new(ReadErrorKind::NoBatteryData, "Preview")),
        _ => BatteryStatus::Reading,
    }
}

struct Controller {
    model: AppModel,
    session: FlyoutSession,
    flyout: Option<WindowHandle<BatteryCard>>,
    tray: Option<TrayService>,
    worker: Option<BatteryService>,
    events: Sender<AppEvent>,
    clock: Instant,
    preview: bool,
}

impl Controller {
    fn event(&mut self, event: AppEvent, cx: &mut Context<Self>) {
        let now = self.clock.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        match event {
            AppEvent::ToggleFlyout { anchor, keyboard } => match self.session.toggle(now, keyboard)
            {
                FlyoutAction::Open(generation) => {
                    if let Err(error) = self.open(anchor, generation, cx) {
                        self.session.hide();
                        shell::show_error(&error);
                    }
                }
                FlyoutAction::Close => self.close(cx),
                FlyoutAction::Ignore => {}
            },
            AppEvent::Dismiss { generation, reason } => {
                if self.session.dismiss(generation, reason, now) {
                    self.close(cx);
                }
            }
            AppEvent::ShowContextMenu { anchor } => {
                self.session.hide();
                self.close(cx);
                if let Some(tray) = &self.tray {
                    let palette = Theme::resolve(
                        self.model.settings.theme,
                        matches!(
                            cx.window_appearance(),
                            WindowAppearance::Dark | WindowAppearance::VibrantDark
                        ),
                    )
                    .menu();
                    tray.show_menu(self.model.settings, anchor, palette);
                }
            }
            AppEvent::Refresh => {
                if let Some(worker) = &self.worker {
                    worker.refresh();
                }
            }
            AppEvent::Battery(status) => {
                if self.model.status != status {
                    self.model.status = status;
                    self.publish(cx);
                }
            }
            AppEvent::TogglePercentage => {
                self.model.settings.show_percent_on_icon =
                    !self.model.settings.show_percent_on_icon;
                self.settings_changed(cx);
            }
            AppEvent::SetLanguage(language) => {
                if self.model.settings.language != language {
                    self.model.settings.language = language;
                    self.settings_changed(cx);
                }
            }
            AppEvent::SetTheme(theme) => {
                if self.model.settings.theme != theme {
                    self.model.settings.theme = theme;
                    self.settings_changed(cx);
                }
            }
            AppEvent::Quit => {
                self.close(cx);
                self.worker.take();
                self.tray.take();
                cx.quit();
            }
        }
    }

    fn open(
        &mut self,
        anchor: Point,
        generation: u64,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let bounds = Bounds::centered(None, size(px(card::WIDTH), px(card::HEIGHT)), cx);
        let model = self.model.clone();
        let events = self.events.clone();
        let handle = if let Some(handle) = self.flyout {
            handle
        } else {
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: None,
                    kind: WindowKind::PopUp,
                    is_movable: false,
                    is_resizable: false,
                    is_minimizable: false,
                    focus: false,
                    show: false,
                    app_id: Some("DrStopor.NinjutsoBatteryTray".into()),
                    ..Default::default()
                },
                move |window, cx| {
                    cx.new(|cx| BatteryCard::new(model, generation, events, window, cx))
                },
            )
            .map_err(|error| error.to_string())?
        };
        self.flyout = Some(handle);
        handle
            .update(cx, |card, window, cx| {
                card.reopen(self.model.clone(), generation, window, cx);
                shell::show_flyout(window, anchor, card::WIDTH, card::HEIGHT)
            })
            .map_err(|error| error.to_string())??;
        if let Some(worker) = &self.worker {
            worker.set_visible(true);
        }
        Ok(())
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        if let Some(worker) = &self.worker {
            worker.set_visible(false);
        }
        // GPUI quits when the last window is destroyed. Reusing one
        // hidden card avoids an extra keepalive window and repeated GPU allocation.
        if let Some(handle) = self.flyout {
            let _ = handle.update(cx, |_, window, _| {
                shell::hide_flyout(window);
            });
        }
    }

    fn publish(&self, cx: &mut Context<Self>) {
        if let Some(tray) = &self.tray {
            tray.update(&self.model);
        }
        if self.session.is_visible()
            && let Some(window) = self.flyout
        {
            let _ = window.update(cx, |card, window, cx| {
                card.set_model(self.model.clone(), window, cx)
            });
        }
    }

    fn settings_changed(&self, cx: &mut Context<Self>) {
        if !self.preview
            && let Err(error) = save_settings(self.model.settings)
        {
            shell::show_error(&error);
        }
        self.publish(cx);
    }
}

fn preview_status(args: &[String]) -> Option<BatteryStatus> {
    use crate::model::{ConnectionKind, MouseBattery, ReadError, ReadErrorKind};
    let option = |name: &str| args.iter().find_map(|arg| arg.strip_prefix(name));
    let state = option("--preview-state=");
    let percent = option("--preview-percent=")
        .and_then(|value| value.parse::<u8>().ok())
        .map(|value| value.min(100));
    if state.is_none()
        && percent.is_none()
        && option("--preview-polling-rate=").is_none()
        && option("--preview-dpi=").is_none()
    {
        return None;
    }
    Some(match state {
        Some("reading") => BatteryStatus::Reading,
        Some("missing") => {
            BatteryStatus::Unavailable(ReadError::new(ReadErrorKind::DeviceNotFound, "Preview"))
        }
        Some("error") => {
            BatteryStatus::Unavailable(ReadError::new(ReadErrorKind::NoBatteryData, "Preview"))
        }
        _ => BatteryStatus::Ready(MouseBattery {
            percent: percent.unwrap_or(72),
            online: state != Some("offline"),
            charging: state == Some("charging"),
            connection: if option("--preview-connection=") == Some("usb") {
                ConnectionKind::Wired
            } else {
                ConnectionKind::Receiver
            },
            polling_rate: option("--preview-polling-rate=")
                .and_then(|value| value.parse::<u16>().ok())
                .and_then(PollingRate::from_hz),
            dpi: option("--preview-dpi=")
                .and_then(|value| value.parse::<u16>().ok())
                .and_then(Dpi::from_value),
        }),
    })
}
