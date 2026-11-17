//! App-owned typefaces. No installation, network or platform font selection.
use std::{borrow::Cow, sync::OnceLock};

use gpui::{App, Font, FontStyle};
use ttf_parser::Face;

use crate::i18n::Language;

pub(super) const METRIC_FAMILY: &str = "Source Sans 3";
pub(super) const CHINESE_FAMILY: &str = "Noto Sans SC";

struct FontAsset {
    family: &'static str,
    weight: u16,
    bytes: &'static [u8],
}

const FONT_COUNT: usize = 5;
static ASSETS: [FontAsset; FONT_COUNT] = [
    FontAsset {
        family: METRIC_FAMILY,
        weight: 400,
        bytes: include_bytes!("../../assets/fonts/source-sans-3/SourceSans3-Regular.otf"),
    },
    FontAsset {
        family: METRIC_FAMILY,
        weight: 500,
        bytes: include_bytes!("../../assets/fonts/source-sans-3/SourceSans3-Medium.otf"),
    },
    FontAsset {
        family: METRIC_FAMILY,
        weight: 600,
        bytes: include_bytes!("../../assets/fonts/source-sans-3/SourceSans3-Semibold.otf"),
    },
    FontAsset {
        family: CHINESE_FAMILY,
        weight: 400,
        bytes: include_bytes!("../../assets/fonts/noto-sans-sc/NotoSansSC-Regular.otf"),
    },
    FontAsset {
        family: CHINESE_FAMILY,
        weight: 500,
        bytes: include_bytes!("../../assets/fonts/noto-sans-sc/NotoSansSC-Medium.otf"),
    },
];

fn faces() -> &'static [Face<'static>; FONT_COUNT] {
    static FACES: OnceLock<[Face<'static>; FONT_COUNT]> = OnceLock::new();
    FACES.get_or_init(|| {
        std::array::from_fn(|i| Face::parse(ASSETS[i].bytes, 0).expect("Invalid bundled font"))
    })
}

pub(super) fn register(cx: &App) -> anyhow::Result<()> {
    let _ = faces();
    cx.text_system().add_fonts(
        ASSETS
            .iter()
            .map(|asset| Cow::Borrowed(asset.bytes))
            .collect(),
    )?;
    let names = cx.text_system().all_font_names();
    for family in [METRIC_FAMILY, CHINESE_FAMILY] {
        anyhow::ensure!(
            names.iter().any(|name| name == family),
            "Bundled font not loaded: {family}"
        );
    }
    Ok(())
}

pub(super) fn family(language: Language) -> &'static str {
    if language == Language::ChineseSimplified {
        CHINESE_FAMILY
    } else {
        METRIC_FAMILY
    }
}

pub(super) fn face(font: &Font) -> Option<&'static Face<'static>> {
    if font.style != FontStyle::Normal {
        return None;
    }
    let (index, _) = ASSETS
        .iter()
        .enumerate()
        .filter(|(_, asset)| asset.family == font.family.as_ref())
        .min_by(|(_, a), (_, b)| {
            (f32::from(a.weight) - font.weight.0)
                .abs()
                .total_cmp(&(f32::from(b.weight) - font.weight.0).abs())
        })?;
    Some(&faces()[index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_fonts_have_the_expected_families_weights_and_static_outlines() {
        for (asset, face) in ASSETS.iter().zip(faces()) {
            assert_eq!(face.weight().to_number(), asset.weight);
            assert!(!face.is_variable());
            assert!(
                face.names().into_iter().any(|name| {
                    matches!(name.name_id, 1 | 16)
                        && name.to_string().as_deref() == Some(asset.family)
                }),
                "Missing family: {}",
                asset.family
            );
            let glyph = face.glyph_index('8').unwrap();
            let bounds = face.glyph_bounding_box(glyph).unwrap();
            assert!(bounds.y_max > bounds.y_min);
        }
    }

    #[test]
    fn all_card_strings_and_language_specific_letters_have_bundled_glyphs() {
        for language in Language::ALL {
            let s = language.strings();
            let text = [
                language.autonym(),
                s.close,
                s.hertz,
                s.polling_rate_hint,
                s.dpi_hint,
                s.reading,
                s.connected,
                s.disconnected,
                s.charging,
                s.on_battery,
                s.wired,
                s.receiver,
                s.device_not_found,
                s.device_not_found_help,
                s.read_failed,
                s.read_failed_help,
                s.permission_denied,
                s.permission_denied_help,
            ]
            .join(" ");
            for (asset, face) in ASSETS
                .iter()
                .zip(faces())
                .filter(|(a, _)| a.family == family(language))
            {
                for ch in text.chars().filter(|ch| !ch.is_whitespace()) {
                    assert!(
                        face.glyph_index(ch).is_some(),
                        "{} weight {} missing {ch} ({ch:?}) for {}",
                        asset.family,
                        asset.weight,
                        language.code()
                    );
                }
            }
        }
        let latin = &faces()[0];
        for ch in "ЎўІіӘәҒғҚқҢңӨөҰұҮүҺһ".chars() {
            assert!(
                latin.glyph_index(ch).is_some(),
                "Missing Belarusian/Kazakh glyph: {ch}"
            );
        }
    }
}
