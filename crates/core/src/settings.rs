use crate::i18n::Language;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ThemePreference {
    #[default]
    Auto,
    Light,
    Dark,
}

impl ThemePreference {
    pub const ALL: [Self; 3] = [Self::Auto, Self::Light, Self::Dark];

    pub const fn code(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|theme| theme.code().eq_ignore_ascii_case(code.trim()))
    }

    pub const fn is_dark(self, system_is_dark: bool) -> bool {
        match self {
            Self::Auto => system_is_dark,
            Self::Light => false,
            Self::Dark => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Settings {
    pub show_percent_on_icon: bool,
    pub language: Language,
    pub theme: ThemePreference,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_percent_on_icon: true,
            language: Language::English,
            theme: ThemePreference::Auto,
        }
    }
}

impl Settings {
    /// The platform supplies the user's default language on first launch.
    pub fn parse(text: &str, defaults: Self) -> Self {
        parse(text, defaults)
    }

    pub fn serialize(self) -> String {
        format!(
            "large_icon_percent={}\nlanguage={}\ntheme={}\n",
            self.show_percent_on_icon,
            self.language.code(),
            self.theme.code()
        )
    }
}

fn parse(text: &str, mut settings: Settings) -> Settings {
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "large_icon_percent" => {
                if let Some(parsed) = parse_bool(value) {
                    settings.show_percent_on_icon = parsed;
                }
            }
            "language" => {
                if let Some(language) = Language::from_code(value) {
                    settings.language = language;
                }
            }
            "theme" => {
                if let Some(theme) = ThemePreference::from_code(value) {
                    settings.theme = theme;
                }
            }
            _ => {}
        }
    }
    settings
}

fn parse_bool(value: &str) -> Option<bool> {
    if matches!(value, "true" | "1" | "yes" | "on") {
        Some(true)
    } else if matches!(value, "false" | "0" | "no" | "off") {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{Settings, ThemePreference, parse};
    use crate::i18n::Language;

    #[test]
    fn old_settings_file_keeps_working() {
        let settings = parse(
            "large_icon_percent=false\n",
            Settings {
                show_percent_on_icon: true,
                language: Language::Russian,
                ..Settings::default()
            },
        );

        assert!(!settings.show_percent_on_icon);
        assert_eq!(settings.language, Language::Russian);
        assert_eq!(settings.theme, ThemePreference::Auto);
    }

    #[test]
    fn language_is_loaded_with_icon_preference() {
        let settings = parse(
            "large_icon_percent=true\nlanguage=kk\n",
            Settings {
                show_percent_on_icon: false,
                language: Language::English,
                ..Settings::default()
            },
        );

        assert!(settings.show_percent_on_icon);
        assert_eq!(settings.language, Language::Kazakh);
    }

    #[test]
    fn theme_codes_round_trip_and_explicit_choices_ignore_the_system_appearance() {
        for theme in ThemePreference::ALL {
            assert_eq!(ThemePreference::from_code(theme.code()), Some(theme));
            for system_dark in [false, true] {
                let expected = match theme {
                    ThemePreference::Auto => system_dark,
                    ThemePreference::Light => false,
                    ThemePreference::Dark => true,
                };
                assert_eq!(theme.is_dark(system_dark), expected);
            }
        }
        assert_eq!(
            ThemePreference::from_code(" LIGHT "),
            Some(ThemePreference::Light)
        );
        assert_eq!(ThemePreference::from_code("unknown"), None);
    }

    #[test]
    fn invalid_or_missing_theme_keeps_existing_preferences() {
        let defaults = Settings {
            theme: ThemePreference::Dark,
            language: Language::Russian,
            ..Settings::default()
        };
        assert_eq!(
            Settings::parse("language=ru\ntheme=unknown", defaults),
            defaults
        );
        assert_eq!(Settings::parse("language=ru", defaults), defaults);
        let light = Settings::parse("theme=light\nlarge_icon_percent=false", defaults);
        assert_eq!(light.theme, ThemePreference::Light);
        assert!(!light.show_percent_on_icon);
        assert_eq!(
            Settings::parse(&light.serialize(), Settings::default()),
            light
        );
        assert_eq!(
            Settings::parse("theme=auto", light).theme,
            ThemePreference::Auto
        );
    }
}
