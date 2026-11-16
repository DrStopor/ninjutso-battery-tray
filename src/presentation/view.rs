use crate::i18n::Language;
use crate::model::{BatteryStatus, ConnectionKind, ReadErrorKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatusTone {
    Healthy,
    Warning,
    Critical,
    Charging,
    Reading,
    Unavailable,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlyoutView {
    /// None means unknown, reading, or disconnected; never display it as 0%.
    pub percent: Option<u8>,
    pub percent_text: String,
    pub primary: &'static str,
    pub secondary: &'static str,
    pub connection: &'static str,
    pub tone: StatusTone,
}

pub fn popup_view(status: &BatteryStatus, language: Language) -> FlyoutView {
    let strings = language.strings();
    match status {
        BatteryStatus::Ready(battery) => {
            let source = match battery.connection {
                ConnectionKind::Wired => strings.wired,
                ConnectionKind::Receiver => strings.receiver,
            };
            if !battery.online {
                return FlyoutView {
                    percent: None,
                    percent_text: "—".to_string(),
                    primary: strings.disconnected,
                    secondary: strings.device_not_found_help,
                    connection: source,
                    tone: StatusTone::Unavailable,
                };
            }
            let percent = battery.percent.min(100);
            FlyoutView {
                percent: Some(percent),
                percent_text: format!("{percent}%"),
                primary: if battery.charging {
                    strings.charging
                } else {
                    strings.on_battery
                },
                secondary: strings.connected,
                connection: source,
                tone: if battery.charging {
                    StatusTone::Charging
                } else if percent >= 50 {
                    StatusTone::Healthy
                } else if percent >= 20 {
                    StatusTone::Warning
                } else {
                    StatusTone::Critical
                },
            }
        }
        BatteryStatus::Reading => FlyoutView {
            percent: None,
            percent_text: "···".to_string(),
            primary: strings.reading,
            secondary: "",
            connection: "",
            tone: StatusTone::Reading,
        },
        BatteryStatus::Unavailable(error) => {
            let missing = error.kind == ReadErrorKind::DeviceNotFound;
            FlyoutView {
                percent: None,
                percent_text: "—".to_string(),
                primary: if missing {
                    strings.device_not_found
                } else {
                    strings.read_failed
                },
                secondary: if missing {
                    strings.device_not_found_help
                } else {
                    strings.read_failed_help
                },
                connection: "",
                tone: if missing {
                    StatusTone::Unavailable
                } else {
                    StatusTone::Error
                },
            }
        }
    }
}

pub fn status_text(status: &BatteryStatus, language: Language) -> String {
    let strings = language.strings();
    match status {
        BatteryStatus::Reading => format!("Ninjutso Ten: {}", strings.reading),
        BatteryStatus::Ready(battery) => {
            let source = match battery.connection {
                ConnectionKind::Wired => strings.wired,
                ConnectionKind::Receiver => strings.receiver,
            };
            if !battery.online {
                return format!("Ninjutso Ten: {} · {source}", strings.disconnected);
            }
            let charge = if battery.charging {
                strings.charging
            } else {
                strings.not_charging
            };
            format!("Ninjutso Ten: {}% · {charge} · {source}", battery.percent)
        }
        BatteryStatus::Unavailable(error) => match error.kind {
            ReadErrorKind::DeviceNotFound => format!("Ninjutso Ten: {}", strings.device_not_found),
            ReadErrorKind::NoBatteryData | ReadErrorKind::System => {
                format!("Ninjutso Ten: {}", strings.read_failed)
            }
        },
    }
}
