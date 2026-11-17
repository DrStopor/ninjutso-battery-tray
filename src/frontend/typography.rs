//! Shared optical geometry measured from the exact embedded font bytes.
//! In particular, do not use GPUI 0.2.2's placeholder Linux advance boxes.
use gpui::{FontId, Pixels, WindowTextSystem, px};

use super::fonts;

const SIZE_INCREASE: f32 = 1.0;

/// Shared card/tooltip sizes in logical pixels; boxes and icons stay unchanged.
pub(super) struct TypeScale;

impl TypeScale {
    pub const CHARGE: f32 = 44.0 + SIZE_INCREASE;
    pub const PERCENT: f32 = 20.0 + SIZE_INCREASE;
    pub const DPI_LABEL: f32 = 20.0 + SIZE_INCREASE;
    pub const DPI_VALUE: f32 = 11.0 + SIZE_INCREASE;
    pub const TITLE: f32 = 13.0 + SIZE_INCREASE;
    pub const STATUS: f32 = 12.0 + SIZE_INCREASE;
    pub const MESSAGE: f32 = 13.0 + SIZE_INCREASE;
    pub const DETAIL: f32 = 12.0 + SIZE_INCREASE;
    pub const RATE: f32 = 10.0 + SIZE_INCREASE;
    pub const FOOTER: f32 = 11.0 + SIZE_INCREASE;
    pub const TOOLTIP: f32 = 12.0 + SIZE_INCREASE;
}

pub(super) struct TextInk {
    pub advance: Pixels,
    pub left: Pixels,
    pub right: Pixels,
    pub ascent: Pixels,
}

pub(super) fn device_title_size(system: &WindowTextSystem, font: FontId, text: &str) -> Pixels {
    let nominal = px(TypeScale::TITLE);
    // Keep a fixed title slot on both platforms; current rate/charge/DPI must
    // never move the shared badge axis.
    measure_ink(system, font, nominal, text).map_or(nominal, |ink| {
        nominal * (76.0 / f32::from(ink.right - ink.left).max(1.0)).min(1.0)
    })
}

pub(super) fn measure_ink(
    system: &WindowTextSystem,
    font: FontId,
    size: Pixels,
    text: &str,
) -> Option<TextInk> {
    let descriptor = system.get_font_for_id(font)?;
    let face = fonts::face(&descriptor)?;
    let units = face.units_per_em();
    if u32::from(units) != system.units_per_em(font) {
        return None;
    }
    let scale = f32::from(size) / f32::from(units);
    let mut advance = px(0.0);
    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    let mut ascent = 0.0_f32;
    for character in text.chars() {
        let glyph = face.glyph_index(character)?;
        let actual_advance = system.advance(font, size, character).ok()?.width;
        let native_advance = f32::from(face.glyph_hor_advance(glyph)?) * scale;
        // Detect a different system face/version instead of measuring it with
        // our font's outlines. The caller can fall back to GPUI cap metrics.
        if (native_advance - f32::from(actual_advance)).abs() > 0.25 {
            return None;
        }
        if let Some(bounds) = face.glyph_bounding_box(glyph) {
            left = left.min(f32::from(advance) + f32::from(bounds.x_min) * scale);
            right = right.max(f32::from(advance) + f32::from(bounds.x_max) * scale);
            ascent = ascent.max(f32::from(bounds.y_max) * scale);
        } else if !character.is_whitespace() {
            return None;
        }
        // Spaces advance the title but have no outline of their own.
        advance += actual_advance;
    }
    left.is_finite().then_some(TextInk {
        advance,
        left: px(left),
        right: px(right),
        ascent: px(ascent),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_glyph_bounds_are_ink_not_the_line_advance_box() {
        let descriptor = gpui::Font {
            weight: gpui::FontWeight::SEMIBOLD,
            ..gpui::font(fonts::METRIC_FAMILY)
        };
        let face = fonts::face(&descriptor).unwrap();
        let glyph = face.glyph_index('8').unwrap();
        let ink = face.glyph_bounding_box(glyph).unwrap();
        let scale = TypeScale::CHARGE / f32::from(face.units_per_em());
        assert!((20.0..40.0).contains(&(f32::from(ink.y_max) * scale)));
        assert!((20.0..40.0).contains(&(f32::from(ink.y_max - ink.y_min) * scale)));
        let glyph = face.glyph_index('I').unwrap();
        let ink = face.glyph_bounding_box(glyph).unwrap();
        assert!(ink.x_max - ink.x_min < face.glyph_hor_advance(glyph).unwrap() as i16);
    }
}
