use std::fmt;

use crate::settings::Settings;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionKind {
    Wired,
    Receiver,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MouseBattery {
    pub percent: u8,
    pub online: bool,
    pub charging: bool,
    pub connection: ConnectionKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadErrorKind {
    DeviceNotFound,
    NoBatteryData,
    System,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadError {
    pub kind: ReadErrorKind,
    detail: String,
}

impl ReadError {
    pub fn new(kind: ReadErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatteryStatus {
    Reading,
    Ready(MouseBattery),
    Unavailable(ReadError),
}

/// Application data shared by native and future GPUI frontends. No OS handles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppModel {
    pub status: BatteryStatus,
    pub settings: Settings,
    pub animation_phase: u8,
}

impl AppModel {
    pub fn new(settings: Settings) -> Self {
        Self {
            status: BatteryStatus::Reading,
            settings,
            animation_phase: 0,
        }
    }
}
