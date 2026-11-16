//! Messages and flyout lifecycle shared by platform services and frontends.
use crate::i18n::Language;
use crate::model::BatteryStatus;
use crate::presentation::layout::Point;
use crate::presentation::menu::MenuCommand;
use crate::settings::ThemePreference;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DismissReason {
    FocusLoss,
    Escape,
    CloseButton,
}

#[derive(Clone, Debug)]
pub enum AppEvent {
    ToggleFlyout {
        anchor: Point,
        keyboard: bool,
    },
    Dismiss {
        generation: u64,
        reason: DismissReason,
    },
    ShowContextMenu {
        anchor: Point,
    },
    Refresh,
    TogglePercentage,
    SetLanguage(Language),
    SetTheme(ThemePreference),
    Battery(BatteryStatus),
    Quit,
}

impl From<MenuCommand> for AppEvent {
    fn from(command: MenuCommand) -> Self {
        match command {
            MenuCommand::Refresh => Self::Refresh,
            MenuCommand::TogglePercentage => Self::TogglePercentage,
            MenuCommand::SetLanguage(language) => Self::SetLanguage(language),
            MenuCommand::SetTheme(theme) => Self::SetTheme(theme),
            MenuCommand::Quit => Self::Quit,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlyoutAction {
    Open(u64),
    Close,
    Ignore,
}

#[derive(Default, Debug)]
pub struct FlyoutSession {
    visible: bool,
    generation: u64,
    last_focus_loss_ms: Option<u64>,
}

impl FlyoutSession {
    pub fn toggle(&mut self, now_ms: u64, keyboard: bool) -> FlyoutAction {
        if self.visible {
            self.visible = false;
            self.last_focus_loss_ms = None;
            return FlyoutAction::Close;
        }
        if !keyboard
            && self
                .last_focus_loss_ms
                .is_some_and(|tick| now_ms.saturating_sub(tick) <= 250)
        {
            self.last_focus_loss_ms = None;
            return FlyoutAction::Ignore;
        }
        self.last_focus_loss_ms = None;
        self.generation = self.generation.wrapping_add(1).max(1);
        self.visible = true;
        FlyoutAction::Open(self.generation)
    }

    pub fn dismiss(&mut self, generation: u64, reason: DismissReason, now_ms: u64) -> bool {
        if !self.visible || generation != self.generation {
            return false;
        }
        self.visible = false;
        self.last_focus_loss_ms = (reason == DismissReason::FocusLoss).then_some(now_ms);
        true
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.last_focus_loss_ms = None;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_click_after_deactivation_does_not_reopen_the_flyout() {
        let mut session = FlyoutSession::default();
        assert_eq!(session.toggle(1000, false), FlyoutAction::Open(1));
        assert!(session.dismiss(1, DismissReason::FocusLoss, 1100));
        assert_eq!(session.toggle(1120, false), FlyoutAction::Ignore);
        assert_eq!(session.toggle(1400, false), FlyoutAction::Open(2));
        assert_eq!(session.toggle(1450, false), FlyoutAction::Close);
    }

    #[test]
    fn stale_window_callbacks_never_close_a_new_window() {
        let mut session = FlyoutSession::default();
        session.toggle(1000, false);
        session.dismiss(1, DismissReason::CloseButton, 1010);
        assert_eq!(session.toggle(1020, false), FlyoutAction::Open(2));
        assert!(!session.dismiss(1, DismissReason::FocusLoss, 1030));
        assert!(session.is_visible());
    }

    #[test]
    fn escape_close_and_keyboard_activation_are_immediately_reusable() {
        let mut session = FlyoutSession::default();
        session.toggle(1000, false);
        session.dismiss(1, DismissReason::Escape, 1010);
        assert_eq!(session.toggle(1020, false), FlyoutAction::Open(2));
        session.dismiss(2, DismissReason::FocusLoss, 1030);
        assert_eq!(session.toggle(1040, true), FlyoutAction::Open(3));
    }
}
