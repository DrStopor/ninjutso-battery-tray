use super::theme::{Rgba, Theme};
use crate::model::BatteryStatus as TrayStatus;

pub const ICON_SIZE: i32 = 32;

#[derive(Clone, Copy)]
struct BatteryIconStyle {
    fill: Rgba,
    empty: Rgba,
    outline: Rgba,
    border: Rgba,
    charging: bool,
    animation_phase: u8,
}

pub fn render_battery_icon_pixels(
    status: &TrayStatus,
    show_percent_on_icon: bool,
    animation_phase: u8,
    theme: &Theme,
) -> Vec<Rgba> {
    let mut pixels = vec![Rgba::transparent(); (ICON_SIZE * ICON_SIZE) as usize];

    let (percent, fill_color, empty_color, charging) = match status {
        TrayStatus::Ready(battery) if !battery.online => (
            100,
            theme.tray.unavailable,
            theme.tray.unavailable_empty,
            false,
        ),
        TrayStatus::Ready(battery) => {
            let color = if battery.percent >= 50 {
                theme.tray.healthy
            } else if battery.percent >= 20 {
                theme.tray.warning
            } else {
                theme.tray.critical
            };
            (
                battery.percent.min(100),
                color,
                theme.tray.empty,
                battery.online && battery.charging,
            )
        }
        TrayStatus::Reading => (100, theme.tray.reading, theme.tray.unavailable_empty, false),
        TrayStatus::Unavailable(_) => (
            100,
            theme.tray.unavailable,
            theme.tray.unavailable_empty,
            false,
        ),
    };

    let black = theme.tray.outline;
    let white = theme.tray.foreground;

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
            theme.tray.numeric_surface,
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
    surface: Rgba,
) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ConnectionKind, MouseBattery, ReadError, ReadErrorKind};

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
        let first = render_battery_icon_pixels(&status, true, 0, &Theme::DARK);
        assert_eq!(
            first,
            render_battery_icon_pixels(&status, true, 17, &Theme::DARK)
        );
        assert!(
            first
                .iter()
                .filter(|p| **p == Theme::DARK.tray.foreground)
                .count()
                > 100
        );
        assert!(
            first
                .iter()
                .filter(|p| **p == Theme::DARK.tray.numeric_surface)
                .count()
                > 100
        );
    }

    #[test]
    fn dark_icons_match_the_pre_refactor_pixel_baseline() {
        // Captured from v0.2.0's renderer across every percentage, both modes,
        // offline/charging combinations and three animation phases.
        let mut statuses = vec![TrayStatus::Reading];
        statuses.push(TrayStatus::Unavailable(ReadError::new(
            ReadErrorKind::DeviceNotFound,
            "missing",
        )));
        statuses.push(TrayStatus::Unavailable(ReadError::new(
            ReadErrorKind::System,
            "error",
        )));
        for percent in 0..=100 {
            for online in [false, true] {
                for charging in [false, true] {
                    statuses.push(TrayStatus::Ready(MouseBattery {
                        percent,
                        online,
                        charging,
                        connection: ConnectionKind::Receiver,
                    }));
                }
            }
        }
        let mut hash = 0xcbf29ce484222325u64;
        for status in &statuses {
            for numeric in [false, true] {
                for phase in [0, 1, 17] {
                    for p in render_battery_icon_pixels(status, numeric, phase, &Theme::DARK) {
                        for byte in [p.r, p.g, p.b, p.a] {
                            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
                        }
                    }
                }
            }
        }
        assert_eq!(hash, 0x013c105a26573847);
    }
}
