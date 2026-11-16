use ninjutso_battery_tray::i18n::Language;
use ninjutso_battery_tray::model::{
    AppModel, BatteryStatus, ConnectionKind, MouseBattery, ReadError, ReadErrorKind,
};
use ninjutso_battery_tray::presentation::interaction::{
    PopupLiveRefreshAction, TrayLeftClickAction, popup_live_refresh_action,
    popup_was_visible_for_tray_click, tray_left_click_action,
};
use ninjutso_battery_tray::presentation::layout::{
    Bounds, FLYOUT_HEIGHT, FLYOUT_WIDTH, FlyoutLayout, Point, popup_position,
};
use ninjutso_battery_tray::presentation::theme::{Rgb, Theme};
use ninjutso_battery_tray::presentation::view::{StatusTone, popup_view, status_text};
use ninjutso_battery_tray::settings::Settings;

fn reading(percent: u8, online: bool, charging: bool) -> BatteryStatus {
    BatteryStatus::Ready(MouseBattery {
        percent,
        online,
        charging,
        connection: ConnectionKind::Receiver,
    })
}

#[test]
fn localized_wired_tooltips_fit_the_windows_limit() {
    let status = BatteryStatus::Ready(MouseBattery {
        percent: 100,
        online: true,
        charging: true,
        connection: ConnectionKind::Wired,
    });
    for language in Language::ALL {
        let tooltip = status_text(&status, language);
        assert!(tooltip.encode_utf16().count() < 128, "{tooltip}");
        assert!(tooltip.contains("USB"));
    }
    assert!(status_text(&status, Language::ChineseSimplified).contains("充电中"));
}

#[test]
fn disconnected_mouse_has_an_explicit_flyout_state_in_every_language() {
    for language in Language::ALL {
        let view = popup_view(&reading(73, false, true), language);
        assert_eq!(view.percent, None);
        assert_eq!(view.percent_text, "—");
        assert_eq!(view.primary, language.strings().disconnected);
        assert_eq!(view.connection, language.strings().receiver);
        assert_eq!(view.tone, StatusTone::Unavailable);
    }
}

#[test]
fn zero_percent_is_distinct_from_missing_reading_and_read_error() {
    for language in Language::ALL {
        let zero = popup_view(&reading(0, true, false), language);
        assert_eq!(zero.percent, Some(0));
        assert_eq!(zero.percent_text, "0%");
        assert_eq!(zero.tone, StatusTone::Critical);
        for kind in [
            ReadErrorKind::DeviceNotFound,
            ReadErrorKind::NoBatteryData,
            ReadErrorKind::System,
        ] {
            let unavailable = popup_view(
                &BatteryStatus::Unavailable(ReadError::new(kind, "test")),
                language,
            );
            assert_eq!(unavailable.percent, None);
            assert_eq!(unavailable.percent_text, "—");
            assert_eq!(
                unavailable.primary,
                if kind == ReadErrorKind::DeviceNotFound {
                    language.strings().device_not_found
                } else {
                    language.strings().read_failed
                }
            );
        }
        assert_eq!(popup_view(&BatteryStatus::Reading, language).percent, None);
    }
}

#[test]
fn charging_and_battery_thresholds_do_not_depend_on_the_renderer() {
    for (percent, tone) in [
        (0, StatusTone::Critical),
        (19, StatusTone::Critical),
        (20, StatusTone::Warning),
        (49, StatusTone::Warning),
        (50, StatusTone::Healthy),
        (100, StatusTone::Healthy),
    ] {
        assert_eq!(
            popup_view(&reading(percent, true, false), Language::English).tone,
            tone
        );
        assert_eq!(
            popup_view(&reading(percent, true, true), Language::English).tone,
            StatusTone::Charging
        );
    }
}

#[test]
fn tray_click_keeps_the_pre_deactivation_visibility() {
    assert!(popup_was_visible_for_tray_click(true, 0, 1_000));
    assert!(popup_was_visible_for_tray_click(false, 900, 1_000));
    assert!(!popup_was_visible_for_tray_click(false, 500, 1_000));
    assert!(popup_was_visible_for_tray_click(false, u32::MAX - 10, 20));
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
fn connection_badge_never_overlaps_the_device_title() {
    for measured_width in [0, 20, 60, 120, 1_000, i32::MAX] {
        let layout = FlyoutLayout::new(Some(measured_width));
        let badge = layout.badge.unwrap();
        assert!(layout.title.right + 8 <= badge.left);
        assert!(layout.title.width() > 100);
        assert!(badge.right < FLYOUT_WIDTH);
    }
    assert!(FlyoutLayout::new(None).badge.is_none());
}

#[test]
fn battery_meter_is_bounded_and_never_fills_for_unknown_or_zero_charge() {
    let layout = FlyoutLayout::new(None);
    assert_eq!(layout.meter_fill(None), None);
    assert_eq!(layout.meter_fill(Some(0)), None);
    let mut previous = 0;
    for percent in 1..=100 {
        let fill = layout.meter_fill(Some(percent)).unwrap();
        assert!(fill.width() >= previous);
        assert!(fill.left >= layout.meter.left && fill.right <= layout.meter.right);
        previous = fill.width();
    }
    assert_eq!(layout.meter_fill(Some(100)), Some(layout.meter));
}

#[test]
fn popup_stays_in_each_monitor_work_area_for_all_taskbar_edges() {
    for work in [
        Bounds::new(0, 0, 1920, 1040),
        Bounds::new(-1920, 0, 0, 1080),
        Bounds::new(0, -1080, 1920, 0),
    ] {
        let points = [
            Point {
                x: work.left,
                y: work.top - 10,
            },
            Point {
                x: work.right - 20,
                y: work.bottom + 10,
            },
            Point {
                x: work.left - 10,
                y: work.top + 100,
            },
            Point {
                x: work.right + 10,
                y: work.top + 100,
            },
        ];
        for point in points {
            let placed = popup_position(point, Some(work));
            assert!(placed.x >= work.left + 8 && placed.x + FLYOUT_WIDTH <= work.right - 8);
            assert!(placed.y >= work.top + 8 && placed.y + FLYOUT_HEIGHT <= work.bottom - 8);
        }
    }
}

#[test]
fn persisted_settings_round_trip_in_every_language_and_keep_platform_defaults() {
    for language in Language::ALL {
        for show_percent_on_icon in [false, true] {
            let settings = Settings {
                language,
                show_percent_on_icon,
            };
            assert_eq!(
                Settings::parse(&settings.serialize(), Settings::default()),
                settings
            );
            assert_eq!(
                Settings::parse("language=unknown\nlarge_icon_percent=invalid", settings),
                settings
            );
            assert_eq!(AppModel::new(settings).settings, settings);
        }
    }
}

#[test]
fn both_palettes_keep_primary_secondary_and_tray_text_readable() {
    fn luminance(c: Rgb) -> f64 {
        let channel = |v: u8| {
            let n = f64::from(v) / 255.0;
            if n <= 0.04045 {
                n / 12.92
            } else {
                ((n + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
    }
    fn contrast(a: Rgb, b: Rgb) -> f64 {
        let a = luminance(a);
        let b = luminance(b);
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
    for theme in [Theme::DARK, Theme::LIGHT] {
        assert!(contrast(theme.flyout.foreground, theme.flyout.background) >= 4.5);
        assert!(contrast(theme.flyout.muted, theme.flyout.background) >= 4.5);
        let a = theme.tray.foreground;
        let b = theme.tray.numeric_surface;
        assert!(contrast(Rgb::new(a.r, a.g, a.b), Rgb::new(b.r, b.g, b.b)) >= 7.0);
    }
}
