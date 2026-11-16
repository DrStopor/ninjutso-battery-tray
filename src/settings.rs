use crate::i18n::Language;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Settings {
    pub show_percent_on_icon: bool,
    pub language: Language,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_percent_on_icon: true,
            language: Language::English,
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
            "large_icon_percent={}\nlanguage={}\n",
            self.show_percent_on_icon,
            self.language.code()
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
    use super::{Settings, parse};
    use crate::i18n::Language;

    #[test]
    fn old_settings_file_keeps_working() {
        let settings = parse(
            "large_icon_percent=false\n",
            Settings {
                show_percent_on_icon: true,
                language: Language::Russian,
            },
        );

        assert!(!settings.show_percent_on_icon);
        assert_eq!(settings.language, Language::Russian);
    }

    #[test]
    fn language_is_loaded_with_icon_preference() {
        let settings = parse(
            "large_icon_percent=true\nlanguage=kk\n",
            Settings {
                show_percent_on_icon: false,
                language: Language::English,
            },
        );

        assert!(settings.show_percent_on_icon);
        assert_eq!(settings.language, Language::Kazakh);
    }
}
