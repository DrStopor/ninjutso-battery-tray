pub const TRAY_DEACTIVATION_GRACE_MS: u32 = 250;

pub fn popup_was_visible_for_tray_click(
    popup_visible: bool,
    last_deactivation_tick: u32,
    message_tick: u32,
) -> bool {
    popup_visible
        || (last_deactivation_tick != 0
            && message_tick.wrapping_sub(last_deactivation_tick) <= TRAY_DEACTIVATION_GRACE_MS)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrayLeftClickAction {
    Hide,
    RefreshAndShow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PopupLiveRefreshAction {
    Refresh,
    Stop,
}

pub fn popup_live_refresh_action(popup_visible: bool) -> PopupLiveRefreshAction {
    if popup_visible {
        PopupLiveRefreshAction::Refresh
    } else {
        PopupLiveRefreshAction::Stop
    }
}

pub fn tray_left_click_action(popup_was_visible: bool) -> TrayLeftClickAction {
    if popup_was_visible {
        TrayLeftClickAction::Hide
    } else {
        TrayLeftClickAction::RefreshAndShow
    }
}
