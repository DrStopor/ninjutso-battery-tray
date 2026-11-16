#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Language {
    Russian,
    English,
    Belarusian,
    Kazakh,
    ChineseSimplified,
}

impl Language {
    pub const ALL: [Self; 5] = [
        Self::Russian,
        Self::English,
        Self::Belarusian,
        Self::Kazakh,
        Self::ChineseSimplified,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::Russian => "ru",
            Self::English => "en",
            Self::Belarusian => "be",
            Self::Kazakh => "kk",
            Self::ChineseSimplified => "zh-CN",
        }
    }

    pub const fn autonym(self) -> &'static str {
        match self {
            Self::Russian => "Русский",
            Self::English => "English",
            Self::Belarusian => "Беларуская",
            Self::Kazakh => "Қазақша",
            Self::ChineseSimplified => "简体中文",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        let code = code.trim();
        if code.eq_ignore_ascii_case("ru") {
            Some(Self::Russian)
        } else if code.eq_ignore_ascii_case("en") {
            Some(Self::English)
        } else if code.eq_ignore_ascii_case("be") {
            Some(Self::Belarusian)
        } else if code.eq_ignore_ascii_case("kk") {
            Some(Self::Kazakh)
        } else if code.eq_ignore_ascii_case("zh-cn") || code.eq_ignore_ascii_case("zh_cn") {
            Some(Self::ChineseSimplified)
        } else {
            None
        }
    }

    pub fn from_windows_lang_id(lang_id: u16) -> Self {
        const LANG_CHINESE_SIMPLIFIED: u16 = 0x0804;
        const LANG_CHINESE_SINGAPORE: u16 = 0x1004;

        match lang_id & 0x03ff {
            0x19 => Self::Russian,
            0x23 => Self::Belarusian,
            0x3f => Self::Kazakh,
            0x09 => Self::English,
            0x04 if matches!(lang_id, LANG_CHINESE_SIMPLIFIED | LANG_CHINESE_SINGAPORE) => {
                Self::ChineseSimplified
            }
            _ => Self::English,
        }
    }

    pub const fn strings(self) -> Strings {
        match self {
            Self::Russian => Strings {
                refresh: "Обновить",
                show_percent: "Показать процент",
                language: "Язык",
                theme: "Тема",
                theme_auto: "Авто",
                theme_light: "Светлая",
                theme_dark: "Тёмная",
                theme_ninjaforce: "NinjaForce",
                dpi_hint: "Текущий DPI активного уровня профиля мыши.",
                exit: "Выход",
                close: "Закрыть",
                hertz: "Гц",
                polling_rate_hint: "Частота опроса из настроек мыши, не измерение потока событий.",
                reading: "Чтение заряда…",
                connected: "Подключена",
                disconnected: "Не подключена",
                charging: "Заряжается",
                not_charging: "Не заряжается",
                on_battery: "От батареи",
                wired: "USB",
                receiver: "Приёмник",
                device_not_found: "Мышь не найдена",
                device_not_found_help: "Подключите мышь по USB или включите её",
                read_failed: "Не удалось прочитать заряд",
                read_failed_help: "Нажмите «Обновить», чтобы повторить",
                already_running: "Ninjutso Battery Tray уже запущен.",
            },
            Self::English => Strings {
                refresh: "Refresh",
                show_percent: "Show percentage",
                language: "Language",
                theme: "Theme",
                theme_auto: "Auto",
                theme_light: "Light",
                theme_dark: "Dark",
                theme_ninjaforce: "NinjaForce",
                dpi_hint: "Current DPI of the active stage in the mouse profile.",
                exit: "Exit",
                close: "Close",
                hertz: "Hz",
                polling_rate_hint: "Configured polling rate, not a live measurement of input events.",
                reading: "Reading battery…",
                connected: "Connected",
                disconnected: "Disconnected",
                charging: "Charging",
                not_charging: "Not charging",
                on_battery: "On battery",
                wired: "USB",
                receiver: "Receiver",
                device_not_found: "Mouse not found",
                device_not_found_help: "Connect the mouse by USB or turn it on",
                read_failed: "Could not read the battery",
                read_failed_help: "Select Refresh to try again",
                already_running: "Ninjutso Battery Tray is already running.",
            },
            Self::Belarusian => Strings {
                refresh: "Абнавіць",
                show_percent: "Паказваць працэнт",
                language: "Мова",
                theme: "Тэма",
                theme_auto: "Аўта",
                theme_light: "Светлая",
                theme_dark: "Цёмная",
                theme_ninjaforce: "NinjaForce",
                dpi_hint: "Бягучы DPI актыўнага ўзроўню профілю мышы.",
                exit: "Выхад",
                close: "Закрыць",
                hertz: "Гц",
                polling_rate_hint: "Частата апытання з налад мышы, не вымярэнне патоку падзей.",
                reading: "Чытанне зараду…",
                connected: "Падключана",
                disconnected: "Не падключана",
                charging: "Зараджаецца",
                not_charging: "Не зараджаецца",
                on_battery: "Ад батарэі",
                wired: "USB",
                receiver: "Прыёмнік",
                device_not_found: "Мыш не знойдзена",
                device_not_found_help: "Падключыце мыш праз USB або ўключыце яе",
                read_failed: "Не ўдалося прачытаць зарад",
                read_failed_help: "Націсніце «Абнавіць», каб паўтарыць",
                already_running: "Ninjutso Battery Tray ужо запушчаны.",
            },
            Self::Kazakh => Strings {
                refresh: "Жаңарту",
                show_percent: "Пайызды көрсету",
                language: "Тіл",
                theme: "Тақырып",
                theme_auto: "Авто",
                theme_light: "Ашық",
                theme_dark: "Күңгірт",
                theme_ninjaforce: "NinjaForce",
                dpi_hint: "Тінтуір профиліндегі белсенді деңгейдің ағымдағы DPI мәні.",
                exit: "Шығу",
                close: "Жабу",
                hertz: "Гц",
                polling_rate_hint: "Тінтуір параметрлеріндегі сұрау жиілігі, оқиғалар ағынының өлшемі емес.",
                reading: "Заряд оқылуда…",
                connected: "Қосылған",
                disconnected: "Қосылмаған",
                charging: "Зарядталуда",
                not_charging: "Зарядталмайды",
                on_battery: "Батареядан",
                wired: "USB",
                receiver: "Қабылдағыш",
                device_not_found: "Тінтуір табылмады",
                device_not_found_help: "Тінтуірді USB арқылы жалғаңыз немесе оны қосыңыз",
                read_failed: "Зарядты оқу мүмкін болмады",
                read_failed_help: "Қайталау үшін «Жаңарту» түймесін басыңыз",
                already_running: "Ninjutso Battery Tray қазірдің өзінде іске қосылған.",
            },
            Self::ChineseSimplified => Strings {
                refresh: "刷新",
                show_percent: "显示电量百分比",
                language: "语言",
                theme: "主题",
                theme_auto: "跟随系统",
                theme_light: "浅色",
                theme_dark: "深色",
                theme_ninjaforce: "NinjaForce",
                dpi_hint: "鼠标当前配置文件中所选档位的 DPI。",
                exit: "退出",
                close: "关闭",
                hertz: "Hz",
                polling_rate_hint: "鼠标设置的轮询率，并非实时测量的输入事件频率。",
                reading: "正在读取电量…",
                connected: "已连接",
                disconnected: "未连接",
                charging: "充电中",
                not_charging: "未充电",
                on_battery: "使用电池",
                wired: "USB",
                receiver: "接收器",
                device_not_found: "未找到鼠标",
                device_not_found_help: "请通过 USB 连接鼠标或将其开启",
                read_failed: "无法读取电量",
                read_failed_help: "点击“刷新”重试",
                already_running: "Ninjutso Battery Tray 已在运行。",
            },
        }
    }
}

#[derive(Clone, Copy)]
pub struct Strings {
    pub refresh: &'static str,
    pub show_percent: &'static str,
    pub language: &'static str,
    pub theme: &'static str,
    pub theme_auto: &'static str,
    pub theme_light: &'static str,
    pub theme_dark: &'static str,
    pub theme_ninjaforce: &'static str,
    pub dpi_hint: &'static str,
    pub exit: &'static str,
    pub close: &'static str,
    pub hertz: &'static str,
    pub polling_rate_hint: &'static str,
    pub reading: &'static str,
    pub connected: &'static str,
    pub disconnected: &'static str,
    pub charging: &'static str,
    pub not_charging: &'static str,
    pub on_battery: &'static str,
    pub wired: &'static str,
    pub receiver: &'static str,
    pub device_not_found: &'static str,
    pub device_not_found_help: &'static str,
    pub read_failed: &'static str,
    pub read_failed_help: &'static str,
    pub already_running: &'static str,
}

#[cfg(test)]
mod tests {
    use super::Language;

    #[test]
    fn language_codes_round_trip() {
        for language in Language::ALL {
            assert_eq!(Language::from_code(language.code()), Some(language));
        }
    }

    #[test]
    fn mainland_chinese_locale_uses_simplified_chinese() {
        assert_eq!(
            Language::from_windows_lang_id(0x0804),
            Language::ChineseSimplified
        );
        assert_eq!(Language::from_windows_lang_id(0x0404), Language::English);
    }
}
