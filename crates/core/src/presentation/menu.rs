//! Platform-independent tray menu: labels, actions and selection state only.
use crate::i18n::Language;
use crate::settings::{Settings, ThemePreference};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuCommand {
    Refresh,
    TogglePercentage,
    SetLanguage(Language),
    SetTheme(ThemePreference),
    Quit,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MenuMark {
    #[default]
    None,
    Check(bool),
    Radio(bool),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MenuEntry {
    Action {
        label: &'static str,
        command: MenuCommand,
        mark: MenuMark,
    },
    Submenu {
        label: &'static str,
        entries: Vec<MenuEntry>,
    },
    Separator,
}

pub fn tray_menu(settings: Settings) -> Vec<MenuEntry> {
    use MenuEntry::*;
    let strings = settings.language.strings();
    vec![
        Action {
            label: strings.refresh,
            command: MenuCommand::Refresh,
            mark: MenuMark::None,
        },
        Action {
            label: strings.show_percent,
            command: MenuCommand::TogglePercentage,
            mark: MenuMark::Check(settings.show_percent_on_icon),
        },
        Submenu {
            label: strings.language,
            entries: Language::ALL
                .into_iter()
                .map(|language| Action {
                    label: language.autonym(),
                    command: MenuCommand::SetLanguage(language),
                    mark: MenuMark::Radio(language == settings.language),
                })
                .collect(),
        },
        Submenu {
            label: strings.theme,
            entries: ThemePreference::ALL
                .into_iter()
                .map(|theme| Action {
                    label: match theme {
                        ThemePreference::Auto => strings.theme_auto,
                        ThemePreference::Light => strings.theme_light,
                        ThemePreference::Dark => strings.theme_dark,
                        ThemePreference::NinjaForce => strings.theme_ninjaforce,
                    },
                    command: MenuCommand::SetTheme(theme),
                    mark: MenuMark::Radio(theme == settings.theme),
                })
                .collect(),
        },
        Separator,
        Action {
            label: strings.exit,
            command: MenuCommand::Quit,
            mark: MenuMark::None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_locale_and_preference_has_exactly_one_selected_language_and_theme() {
        for language in Language::ALL {
            for theme in ThemePreference::ALL {
                let entries = tray_menu(Settings {
                    language,
                    theme,
                    ..Settings::default()
                });
                for (index, expected) in [
                    (2, MenuCommand::SetLanguage(language)),
                    (3, MenuCommand::SetTheme(theme)),
                ] {
                    let MenuEntry::Submenu { entries, label } = &entries[index] else {
                        panic!("Expected submenu");
                    };
                    assert!(!label.is_empty());
                    let selected: Vec<_> = entries
                        .iter()
                        .filter_map(|entry| match entry {
                            MenuEntry::Action {
                                command,
                                mark: MenuMark::Radio(true),
                                label,
                            } => {
                                assert!(!label.is_empty());
                                Some(*command)
                            }
                            _ => None,
                        })
                        .collect();
                    assert_eq!(selected, [expected]);
                }
            }
        }
    }

    #[test]
    fn percentage_check_reflects_the_setting_and_does_not_change_the_action() {
        for selected in [false, true] {
            let entries = tray_menu(Settings {
                show_percent_on_icon: selected,
                ..Settings::default()
            });
            assert!(matches!(entries[1], MenuEntry::Action {
                command: MenuCommand::TogglePercentage,
                mark: MenuMark::Check(value), ..
            } if value == selected));
        }
    }
}
