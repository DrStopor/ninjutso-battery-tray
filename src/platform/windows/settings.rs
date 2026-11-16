use std::env;
use std::fs;
use std::path::PathBuf;

use crate::i18n::Language;
use crate::settings::Settings;

pub fn load() -> Settings {
    let defaults = Settings {
        language: Language::from_windows_lang_id(unsafe { GetUserDefaultUILanguage() }),
        ..Settings::default()
    };
    let Some(path) = path() else { return defaults };
    let Ok(text) = fs::read_to_string(path) else {
        return defaults;
    };
    Settings::parse(&text, defaults)
}

pub fn save(settings: Settings) -> Result<(), String> {
    let path = path().ok_or_else(|| "APPDATA is not available".to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create the settings directory: {error}"))?;
    }
    fs::write(path, settings.serialize())
        .map_err(|error| format!("Could not save settings: {error}"))
}

fn path() -> Option<PathBuf> {
    env::var_os("APPDATA").map(|appdata| {
        PathBuf::from(appdata)
            .join("NinjutsoBatteryTray")
            .join("settings.txt")
    })
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetUserDefaultUILanguage() -> u16;
}
