//! XDG configuration and POSIX locale detection. No desktop toolkit dependency.
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use crate::i18n::Language;
use crate::settings::Settings;

pub(crate) fn config_directory() -> Result<PathBuf, String> {
    config_base(
        env::var_os("XDG_CONFIG_HOME").as_deref(),
        env::var_os("HOME").as_deref(),
    )
    .map(|base| base.join("ninjutso-battery-tray"))
    .ok_or_else(|| "Neither an absolute XDG_CONFIG_HOME nor HOME is available".into())
}

fn config_base(xdg: Option<&std::ffi::OsStr>, home: Option<&std::ffi::OsStr>) -> Option<PathBuf> {
    xdg.map(Path::new)
        .filter(|p| p.is_absolute())
        .map(Path::to_owned)
        .or_else(|| {
            home.map(Path::new)
                .filter(|p| p.is_absolute())
                .map(|p| p.join(".config"))
        })
}

pub(crate) fn private_directory(path: &Path) -> Result<(), String> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .map_err(|error| format!("Could not create {}: {error}", path.display()))
}

pub fn load() -> Settings {
    let locale = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|key| env::var(key).ok().filter(|s| !s.trim().is_empty()));
    let defaults = Settings {
        language: language_from_locale(locale.as_deref().unwrap_or("C")),
        ..Settings::default()
    };
    let Ok(directory) = config_directory() else {
        return defaults;
    };
    fs::read_to_string(directory.join("settings.txt"))
        .map(|text| Settings::parse(&text, defaults))
        .unwrap_or(defaults)
}

pub fn save(settings: Settings) -> Result<(), String> {
    let directory = config_directory()?;
    private_directory(&directory)?;
    let temporary = directory.join(format!(".settings.{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&temporary)
        .map_err(|error| format!("Could not create settings transaction: {error}"))?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(settings.serialize().as_bytes())?;
        file.sync_all()?;
        fs::rename(&temporary, directory.join("settings.txt"))
    })();
    // Only remove the temporary file created by this save attempt, never follow a symlink.
    if result.is_err() && temporary.is_file() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| format!("Could not save settings: {error}"))
}

fn language_from_locale(locale: &str) -> Language {
    let base = locale
        .split(['.', '@'])
        .next()
        .unwrap_or(locale)
        .replace('-', "_")
        .to_ascii_lowercase();
    match base.as_str() {
        "ru" | "ru_ru" => Language::Russian,
        "be" | "be_by" => Language::Belarusian,
        "kk" | "kk_kz" => Language::Kazakh,
        "zh_cn" | "zh_sg" | "zh_hans" | "zh_hans_cn" => Language::ChineseSimplified,
        _ if base.starts_with("ru_") => Language::Russian,
        _ if base.starts_with("be_") => Language::Belarusian,
        _ if base.starts_with("kk_") => Language::Kazakh,
        _ => Language::English,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locales_use_supported_languages_without_treating_traditional_chinese_as_simplified() {
        for (locale, language) in [
            ("ru_RU.UTF-8", Language::Russian),
            ("be_BY", Language::Belarusian),
            ("kk-KZ.UTF-8", Language::Kazakh),
            ("zh_CN.UTF-8", Language::ChineseSimplified),
            ("zh_Hans_CN", Language::ChineseSimplified),
            ("zh_TW.UTF-8", Language::English),
            ("C", Language::English),
        ] {
            assert_eq!(language_from_locale(locale), language);
        }
    }
    #[test]
    fn xdg_paths_must_be_absolute() {
        assert_eq!(
            config_base(Some("/config".as_ref()), Some("/home/user".as_ref())),
            Some("/config".into())
        );
        assert_eq!(
            config_base(Some("relative".as_ref()), Some("/home/user".as_ref())),
            Some("/home/user/.config".into())
        );
        assert_eq!(config_base(None, Some("relative".as_ref())), None);
    }
}
