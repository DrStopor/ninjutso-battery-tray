use crate::i18n::Language;
use crate::model::{BatteryStatus, ConnectionKind, PollingRate, ReadErrorKind};

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
pub struct DpiBadgeView {
    pub label: &'static str,
    /// Numeric text only; units belong to the separate caption.
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlyoutView {
    /// None means unknown, reading, or disconnected; never display it as 0%.
    pub percent: Option<u8>,
    pub percent_text: String,
    pub primary: &'static str,
    pub secondary: &'static str,
    pub connection: &'static str,
    pub polling_rate: Option<String>,
    pub dpi: Option<DpiBadgeView>,
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
                    polling_rate: None,
                    dpi: None,
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
                polling_rate: battery
                    .polling_rate
                    .map(|rate| polling_rate_text(rate, language)),
                dpi: battery.dpi.map(|dpi| DpiBadgeView {
                    label: "DPI",
                    value: dpi.value().to_string(),
                }),
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
            polling_rate: None,
            dpi: None,
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
                polling_rate: None,
                dpi: None,
                tone: if missing {
                    StatusTone::Unavailable
                } else {
                    StatusTone::Error
                },
            }
        }
    }
}

fn polling_rate_text(rate: PollingRate, language: Language) -> String {
    format!("{} {}", rate.hz(), language.strings().hertz)
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
            let text = format!("Ninjutso Ten: {}% · {charge} · {source}", battery.percent);
            let text = if let Some(rate) = battery.polling_rate {
                format!("{text} · {}", polling_rate_text(rate, language))
            } else {
                text
            };
            if let Some(dpi) = battery.dpi {
                format!("{text} · {} DPI", dpi.value())
            } else {
                text
            }
        }
        BatteryStatus::Unavailable(error) => match error.kind {
            ReadErrorKind::DeviceNotFound => format!("Ninjutso Ten: {}", strings.device_not_found),
            ReadErrorKind::NoBatteryData | ReadErrorKind::System => {
                format!("Ninjutso Ten: {}", strings.read_failed)
            }
        },
    }
}
