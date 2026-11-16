use ninjutso_battery_core::i18n::Language;
use ninjutso_battery_core::model::{
    AppModel, BatteryStatus, ConnectionKind, Dpi, MouseBattery, PollingRate, ReadError,
    ReadErrorKind,
};
use ninjutso_battery_core::presentation::interaction::{
    PopupLiveRefreshAction, TrayLeftClickAction, popup_live_refresh_action,
    popup_was_visible_for_tray_click, tray_left_click_action,
};
use ninjutso_battery_core::presentation::layout::{
    Bounds, FLYOUT_HEIGHT, FLYOUT_WIDTH, FlyoutLayout, Point, popup_position,
};
use ninjutso_battery_core::presentation::theme::{Rgb, Theme};
use ninjutso_battery_core::presentation::view::{StatusTone, popup_view, status_text};
use ninjutso_battery_core::settings::{Settings, ThemePreference};

fn reading(percent: u8, online: bool, charging: bool) -> BatteryStatus {
    BatteryStatus::Ready(MouseBattery {
        percent,
        online,
        charging,
        connection: ConnectionKind::Receiver,
        polling_rate: None,
        dpi: None,
    })
}

#[test]
fn localized_wired_tooltips_fit_the_windows_limit() {
    let status = BatteryStatus::Ready(MouseBattery {
        percent: 100,
        online: true,
        charging: true,
        connection: ConnectionKind::Wired,
        polling_rate: Some(PollingRate::Hz8000),
        dpi: Dpi::from_value(30_000),
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
fn polling_badge_uses_localized_units_for_each_supported_rate() {
    for language in Language::ALL {
        for hz in [1000, 2000, 4000, 8000] {
            let status = BatteryStatus::Ready(MouseBattery {
                percent: 72,
                online: true,
                charging: false,
                connection: ConnectionKind::Receiver,
                polling_rate: PollingRate::from_hz(hz),
                dpi: None,
            });
            let label = format!("{hz} {}", language.strings().hertz);
            let view = popup_view(&status, language);
            assert_eq!(view.polling_rate.as_deref(), Some(label.as_str()));
            assert!(status_text(&status, language).contains(&label));
            assert!(!language.strings().polling_rate_hint.is_empty());
        }
    }
}

#[test]
fn missing_polling_rate_preserves_the_battery_and_offline_hides_stale_rates() {
    for language in Language::ALL {
        let mut battery = MouseBattery {
            percent: 72,
            online: true,
            charging: false,
            connection: ConnectionKind::Receiver,
            polling_rate: None,
            dpi: None,
        };
        let view = popup_view(&BatteryStatus::Ready(battery), language);
        assert_eq!(view.percent, Some(72));
        assert_eq!(view.polling_rate, None);
        battery.polling_rate = Some(PollingRate::Hz8000);
        battery.online = false;
        let status = BatteryStatus::Ready(battery);
        assert_eq!(popup_view(&status, language).polling_rate, None);
        assert!(!status_text(&status, language).contains("8000"));
        assert_eq!(
            popup_view(&BatteryStatus::Reading, language).polling_rate,
            None
        );
        let missing =
            BatteryStatus::Unavailable(ReadError::new(ReadErrorKind::DeviceNotFound, "test"));
        assert_eq!(popup_view(&missing, language).polling_rate, None);
    }
}

#[test]
fn a_rate_change_alone_is_a_new_model_and_presentation() {
    let before = AppModel {
        status: BatteryStatus::Ready(MouseBattery {
            percent: 72,
            online: true,
            charging: false,
            connection: ConnectionKind::Receiver,
            polling_rate: Some(PollingRate::Hz1000),
            dpi: None,
        }),
        ..AppModel::new(Settings::default())
    };
    let mut after = before.clone();
    let BatteryStatus::Ready(battery) = &mut after.status else {
        unreachable!()
    };
    battery.polling_rate = Some(PollingRate::Hz4000);
    assert_ne!(before, after);
    assert_ne!(
        popup_view(&before.status, Language::English),
        popup_view(&after.status, Language::English)
    );
    assert_eq!(
        popup_view(&before.status, Language::English).percent,
        popup_view(&after.status, Language::English).percent
    );
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
            for theme in ThemePreference::ALL {
                let settings = Settings {
                    language,
                    show_percent_on_icon,
                    theme,
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
}

#[test]
fn dpi_is_shown_only_for_an_online_mouse_with_a_valid_reading() {
    for language in Language::ALL {
        let mut battery = MouseBattery {
            percent: 72,
            online: true,
            charging: false,
            connection: ConnectionKind::Receiver,
            polling_rate: Some(PollingRate::Hz1000),
            dpi: Dpi::from_value(1100),
        };
        assert_eq!(
            popup_view(&BatteryStatus::Ready(battery), language)
                .dpi
                .as_ref()
                .map(|dpi| (dpi.label, dpi.value.as_str())),
            Some(("DPI", "1100"))
        );
        assert!(status_text(&BatteryStatus::Ready(battery), language).contains("1100 DPI"));
        battery.dpi = None;
        let unknown = popup_view(&BatteryStatus::Ready(battery), language);
        assert_eq!(unknown.percent, Some(72));
        assert!(unknown.polling_rate.is_some());
        assert_eq!(unknown.dpi, None);
        battery.dpi = Dpi::from_value(1100);
        battery.online = false;
        assert_eq!(
            popup_view(&BatteryStatus::Ready(battery), language).dpi,
            None
        );
        assert!(!status_text(&BatteryStatus::Ready(battery), language).contains("DPI"));
        assert_eq!(popup_view(&BatteryStatus::Reading, language).dpi, None);
    }
}

#[test]
fn changing_only_dpi_updates_the_model_and_view_without_changing_the_charge() {
    let mut before = AppModel::new(Settings::default());
    before.status = BatteryStatus::Ready(MouseBattery {
        percent: 72,
        online: true,
        charging: false,
        connection: ConnectionKind::Receiver,
        polling_rate: Some(PollingRate::Hz1000),
        dpi: Dpi::from_value(1100),
    });
    let mut after = before.clone();
    let BatteryStatus::Ready(battery) = &mut after.status else {
        unreachable!()
    };
    battery.dpi = Dpi::from_value(1600);
    assert_ne!(before, after);
    let before = popup_view(&before.status, Language::English);
    let after = popup_view(&after.status, Language::English);
    assert_ne!(before.dpi, after.dpi);
    assert_eq!(before.percent, after.percent);
    assert_eq!(before.polling_rate, after.polling_rate);
}

#[test]
fn dpi_badge_has_a_separate_uppercase_caption_and_numeric_value_in_every_locale() {
    for language in Language::ALL {
        for value in [100, 1100, 30_000] {
            let status = BatteryStatus::Ready(MouseBattery {
                percent: 100,
                online: true,
                charging: false,
                connection: ConnectionKind::Receiver,
                polling_rate: Some(PollingRate::Hz1000),
                dpi: Dpi::from_value(value),
            });
            let badge = popup_view(&status, language).dpi.unwrap();
            assert_eq!(badge.label, "DPI");
            assert_eq!(badge.value, value.to_string());
            assert!(badge.value.bytes().all(|byte| byte.is_ascii_digit()));
        }
    }
}

#[test]
fn dpi_badge_prefers_the_midline_but_keeps_both_lines_readable() {
    use ninjutso_battery_core::presentation::layout::DpiBadgeLayout;
    for (charge_top, percent_middle) in [
        (12.0, 36.0),
        (12.23, 36.65),
        (14.72, 39.1),
        (9.8, 34.7),
        (13.3, 35.2),
        (12.0, 44.0),
        (12.0, 45.0),
        (12.0, 50.0),
    ] {
        let badge = DpiBadgeLayout::between_guides(charge_top, percent_middle);
        assert!(badge.top >= charge_top && badge.top < charge_top + 1.0);
        assert!((32.0..=34.0).contains(&badge.height));
        if percent_middle - badge.top >= 32.0 {
            assert!(badge.top + badge.height <= percent_middle);
        } else {
            assert_eq!(badge.height, 32.0);
        }
    }
}

#[test]
fn fixed_metric_columns_share_the_rate_axis_and_reserve_room_for_100_percent() {
    use ninjutso_battery_core::presentation::layout::ChargeRowLayout;
    for dpi_width in [32.0, 36.0, 40.0, 48.0] {
        let row = ChargeRowLayout::new(272.0, dpi_width);
        assert_eq!(row.power_left + row.power_width, 272.0);
        assert!((row.dpi_left + dpi_width / 2.0 - ChargeRowLayout::RATE_CENTER).abs() <= 0.5);
        for actual_charge_width in [25.0, 44.0, 68.0, 94.0] {
            assert!(row.dpi_left >= actual_charge_width + 8.0);
        }
        assert!(row.power_left - row.dpi_left - dpi_width >= 8.0);
    }
}

#[test]
fn theme_menu_labels_exist_and_are_distinct_in_every_language() {
    for language in Language::ALL {
        let strings = language.strings();
        assert!(!strings.theme.is_empty());
        let labels = [
            strings.theme_auto,
            strings.theme_light,
            strings.theme_dark,
            strings.theme_ninjaforce,
        ];
        assert!(labels.iter().all(|label| !label.is_empty()));
        for (index, label) in labels.iter().enumerate() {
            assert!(!labels[..index].contains(label));
        }
    }
}

#[test]
fn all_palettes_keep_primary_secondary_and_tray_text_readable() {
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
    for theme in [
        Theme::DARK,
        Theme::LIGHT,
        Theme::FLUENT_DARK,
        Theme::NINJAFORCE,
    ] {
        assert!(contrast(theme.flyout.foreground, theme.flyout.background) >= 4.5);
        assert!(contrast(theme.flyout.muted, theme.flyout.background) >= 4.5);
        for tone in [
            StatusTone::Healthy,
            StatusTone::Warning,
            StatusTone::Critical,
            StatusTone::Charging,
            StatusTone::Reading,
            StatusTone::Unavailable,
            StatusTone::Error,
        ] {
            assert!(contrast(theme.flyout.accent(tone), theme.flyout.background) >= 3.0);
        }
        let menu = theme.menu();
        assert!(contrast(menu.foreground, menu.background) >= 4.5);
        assert!(contrast(menu.foreground, menu.highlight) >= 4.5);
        assert!(contrast(menu.muted, menu.background) >= 4.5);
        let a = theme.tray.foreground;
        let b = theme.tray.numeric_surface;
        assert!(contrast(Rgb::new(a.r, a.g, a.b), Rgb::new(b.r, b.g, b.b)) >= 7.0);
    }
}

#[test]
fn menu_and_card_resolve_the_same_preference_without_platform_dependencies() {
    for system_dark in [false, true] {
        for preference in ThemePreference::ALL {
            let theme = Theme::resolve(preference, system_dark);
            let expected = match preference {
                ThemePreference::NinjaForce => Theme::NINJAFORCE,
                ThemePreference::Auto if system_dark => Theme::FLUENT_DARK,
                ThemePreference::Dark => Theme::FLUENT_DARK,
                ThemePreference::Auto | ThemePreference::Light => Theme::LIGHT,
            };
            assert_eq!(theme.flyout.background, expected.flyout.background);
            assert_eq!(theme.menu().background, theme.flyout.background);
            assert_eq!(theme.menu().foreground, theme.flyout.foreground);
        }
    }
}

#[test]
fn ninjaforce_preserves_the_companion_palette_without_changing_the_tray_contrast() {
    for system_dark in [false, true] {
        let theme = Theme::resolve(ThemePreference::NinjaForce, system_dark);
        assert_eq!(theme.flyout.background, Rgb::new(195, 189, 219));
        assert_eq!(theme.flyout.foreground, Rgb::new(30, 34, 170));
        assert_ne!(
            theme.flyout.background,
            Theme::FLUENT_DARK.flyout.background
        );
        assert_ne!(theme.flyout.background, Theme::LIGHT.flyout.background);
        assert_eq!(theme.tray.foreground, Theme::DARK.tray.foreground);
        assert_eq!(theme.tray.numeric_surface, Theme::DARK.tray.numeric_surface);
    }
}
