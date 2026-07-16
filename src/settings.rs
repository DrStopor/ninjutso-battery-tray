use std::env;
use std::fs;
use std::path::PathBuf;

use crate::i18n::Language;

#[derive(Clone, Copy)]
pub struct Settings {
    pub show_percent_on_icon: bool,
    pub language: Language,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_percent_on_icon: true,
            language: Language::system_default(),
        }
    }
}

pub fn load() -> Settings {
    let default = Settings::default();
    let Some(path) = path() else {
        return default;
    };
    let Ok(text) = fs::read_to_string(path) else {
        return default;
    };

    parse(&text, default)
}

pub fn save(settings: Settings) -> Result<(), String> {
    let path = path().ok_or_else(|| "APPDATA is not available".to_string())?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Could not create the settings directory: {err}"))?;
    }

    let show_percent = if settings.show_percent_on_icon {
        "true"
    } else {
        "false"
    };
    let text = format!(
        "large_icon_percent={show_percent}\nlanguage={}\n",
        settings.language.code()
    );
    fs::write(path, text).map_err(|err| format!("Could not save settings: {err}"))
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

fn path() -> Option<PathBuf> {
    env::var_os("APPDATA").map(|appdata| {
        PathBuf::from(appdata)
            .join("NinjutsoBatteryTray")
            .join("settings.txt")
    })
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
