//! StatusNotifierItem + DBusMenu. The desktop owns menu appearance/accessibility.
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use async_channel::Sender as EventSender;
use ksni::Tray;
use ksni::blocking::TrayMethods;
use ksni::menu::{CheckmarkItem, MenuItem, RadioGroup, RadioItem, StandardItem, SubMenu};

use crate::application::AppEvent;
use crate::model::AppModel;
use crate::presentation::icons::{ICON_SIZE, render_battery_icon_pixels};
use crate::presentation::layout::Point;
use crate::presentation::menu::{MenuCommand, MenuEntry, MenuMark, tray_menu_with_status};
use crate::presentation::theme::{MenuPalette, Theme};
use crate::presentation::view::status_text;
use crate::settings::Settings;

enum Command {
    Update(AppModel),
    Stop,
}
pub(crate) struct TrayService {
    sender: Sender<Command>,
    _thread: JoinHandle<()>,
}

impl TrayService {
    pub(crate) fn start(model: AppModel, events: EventSender<AppEvent>) -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("ninjutso-sni".into())
            .spawn(move || {
                // Autostart may precede the panel. ksni re-registers when its watcher
                // appears/restarts; never launch a second copy or poll D-Bus on the UI.
                let handle = match (TrayState {
                    model,
                    events,
                    activation_anchor: None,
                })
                .assume_sni_available(true)
                .spawn()
                {
                    Ok(handle) => handle,
                    Err(error) => {
                        let _ = ready_tx.send(Err(format!("D-Bus tray: {error}")));
                        return;
                    }
                };
                let _ = ready_tx.send(Ok(()));
                while let Ok(command) = receiver.recv() {
                    match command {
                        Command::Update(model) => {
                            handle.update(|tray| tray.model = model);
                        }
                        Command::Stop => break,
                    }
                }
                handle.shutdown().wait();
            })
            .map_err(|e| e.to_string())?;
        ready_rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|e| format!("Tray startup: {e}"))??;
        Ok(Self {
            sender,
            _thread: thread,
        })
    }
    pub(crate) fn update(&self, model: &AppModel) {
        let _ = self.sender.send(Command::Update(model.clone()));
    }
    pub(crate) fn show_menu(&self, _settings: Settings, _anchor: Point, _palette: MenuPalette) {
        // The SNI host displays the exported DBusMenu. Do not open a duplicate.
    }
}
impl Drop for TrayService {
    fn drop(&mut self) {
        let _ = self.sender.send(Command::Stop);
    }
}

struct TrayState {
    model: AppModel,
    events: EventSender<AppEvent>,
    activation_anchor: Option<Point>,
}

impl TrayState {
    fn dispatch(&self, command: MenuCommand) {
        let event = if command == MenuCommand::ShowStatus {
            AppEvent::ToggleFlyout {
                anchor: self
                    .activation_anchor
                    .unwrap_or_else(super::surface::context_menu_anchor),
                keyboard: true,
            }
        } else {
            command.into()
        };
        let _ = self.events.try_send(event);
    }
}

impl Tray for TrayState {
    fn id(&self) -> String {
        "ninjutso-battery-tray".into()
    }
    fn title(&self) -> String {
        status_text(&self.model.status, self.model.settings.language)
    }
    fn category(&self) -> ksni::Category {
        ksni::Category::Hardware
    }
    fn activate(&mut self, x: i32, y: i32) {
        // These coordinates come from the panel host, unlike XQueryPointer
        // while interacting with a native Wayland context menu.
        if (x, y) != (0, 0) {
            self.activation_anchor = Some(Point { x, y });
        }
        let _ = self.events.try_send(AppEvent::ToggleFlyout {
            anchor: Point { x, y },
            keyboard: false,
        });
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![ksni::Icon {
            width: ICON_SIZE,
            height: ICON_SIZE,
            data: render_battery_icon_pixels(
                &self.model.status,
                self.model.settings.show_percent_on_icon,
                self.model.animation_phase,
                &Theme::DARK,
            )
            .into_iter()
            .flat_map(|p| [p.a, p.r, p.g, p.b])
            .collect(),
        }]
    }
    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: crate::APP_NAME.into(),
            description: self.title(),
            ..Default::default()
        }
    }
    fn menu(&self) -> Vec<MenuItem<Self>> {
        convert(tray_menu_with_status(self.model.settings))
    }
    // Leave menu_about_to_show at its default: hosts may prefetch DBusMenu
    // metadata without opening a menu. Actual menu focus dismisses the card
    // through the shared activation observer, not through a metadata request.
    fn watcher_offline(&self, reason: ksni::OfflineReason) -> bool {
        eprintln!(
            "System tray unavailable ({reason:?}). GNOME needs the AppIndicator extension; waiting for a StatusNotifierItem host."
        );
        true
    }
}

fn convert(entries: Vec<MenuEntry>) -> Vec<MenuItem<TrayState>> {
    if entries.iter().all(|e| {
        matches!(
            e,
            MenuEntry::Action {
                mark: MenuMark::Radio(_),
                ..
            }
        )
    }) && !entries.is_empty()
    {
        let mut options = Vec::new();
        let mut commands = Vec::new();
        let mut selected = 0;
        for (index, entry) in entries.into_iter().enumerate() {
            if let MenuEntry::Action {
                label,
                command,
                mark: MenuMark::Radio(checked),
            } = entry
            {
                if checked {
                    selected = index;
                }
                options.push(RadioItem {
                    label: label.into(),
                    ..Default::default()
                });
                commands.push(command);
            }
        }
        return vec![
            RadioGroup {
                selected,
                options,
                select: Box::new(move |tray: &mut TrayState, index| {
                    if let Some(command) = commands.get(index) {
                        tray.dispatch(*command);
                    }
                }),
            }
            .into(),
        ];
    }
    entries
        .into_iter()
        .map(|entry| match entry {
            MenuEntry::Separator => MenuItem::Separator,
            MenuEntry::Submenu { label, entries } => SubMenu {
                label: label.into(),
                submenu: convert(entries),
                ..Default::default()
            }
            .into(),
            MenuEntry::Action {
                label,
                command,
                mark: MenuMark::Check(checked),
            } => CheckmarkItem {
                label: label.into(),
                checked,
                activate: Box::new(move |tray: &mut TrayState| tray.dispatch(command)),
                ..Default::default()
            }
            .into(),
            MenuEntry::Action { label, command, .. } => StandardItem {
                label: label.into(),
                activate: Box::new(move |tray: &mut TrayState| tray.dispatch(command)),
                ..Default::default()
            }
            .into(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{i18n::Language, settings::ThemePreference};

    #[test]
    fn menu_prefetch_does_not_dismiss_the_card() {
        let (events, received) = async_channel::unbounded();
        let mut tray = TrayState {
            model: AppModel::new(Settings::default()),
            events,
            activation_anchor: None,
        };
        Tray::menu_about_to_show(&mut tray);
        assert!(received.try_recv().is_err());
    }

    #[test]
    fn sni_exports_localized_status_action_and_selected_radio_groups() {
        for language in Language::ALL {
            for theme in ThemePreference::ALL {
                let menu = convert(tray_menu_with_status(Settings {
                    language,
                    theme,
                    ..Settings::default()
                }));
                assert!(
                    matches!(&menu[0],MenuItem::Standard(item) if item.label == language.strings().show_status)
                );
                for (index, count, selected) in [
                    (
                        3,
                        5,
                        Language::ALL.iter().position(|v| *v == language).unwrap(),
                    ),
                    (
                        4,
                        4,
                        ThemePreference::ALL
                            .iter()
                            .position(|v| *v == theme)
                            .unwrap(),
                    ),
                ] {
                    let MenuItem::SubMenu(sub) = &menu[index] else {
                        panic!("Missing submenu")
                    };
                    let MenuItem::RadioGroup(group) = &sub.submenu[0] else {
                        panic!("Missing radio group")
                    };
                    assert_eq!(group.options.len(), count);
                    assert_eq!(group.selected, selected);
                }
            }
        }
    }
}
