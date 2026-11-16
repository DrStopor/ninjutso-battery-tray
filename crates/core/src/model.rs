use std::fmt;

use crate::settings::Settings;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionKind {
    Wired,
    Receiver,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PollingRate {
    Hz1000,
    Hz2000,
    Hz4000,
    Hz8000,
}

impl PollingRate {
    /// Values returned by Ninjutso's active-profile report, not measured input Hz.
    pub const fn from_device_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Hz1000),
            2 => Some(Self::Hz2000),
            3 => Some(Self::Hz4000),
            4 => Some(Self::Hz8000),
            _ => None,
        }
    }

    pub const fn hz(self) -> u16 {
        match self {
            Self::Hz1000 => 1000,
            Self::Hz2000 => 2000,
            Self::Hz4000 => 4000,
            Self::Hz8000 => 8000,
        }
    }

    pub const fn from_hz(hz: u16) -> Option<Self> {
        match hz {
            1000 => Some(Self::Hz1000),
            2000 => Some(Self::Hz2000),
            4000 => Some(Self::Hz4000),
            8000 => Some(Self::Hz8000),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Dpi(u16);

impl Dpi {
    /// Ninjutso Ten's supported settings, in 50-DPI steps.
    pub const fn from_value(value: u16) -> Option<Self> {
        if value >= 100 && value <= 30_000 && value.is_multiple_of(50) {
            Some(Self(value))
        } else {
            None
        }
    }

    pub const fn value(self) -> u16 {
        self.0
    }

    pub const fn from_device_level(level: u16) -> Option<Self> {
        // Widen before adding/multiplying so corrupt 0xffff never wraps into a valid DPI.
        let value = (level as u32 + 1) * 50;
        if value >= 100 && value <= 30_000 {
            Some(Self(value as u16))
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MouseBattery {
    pub percent: u8,
    pub online: bool,
    pub charging: bool,
    pub connection: ConnectionKind,
    pub polling_rate: Option<PollingRate>,
    pub dpi: Option<Dpi>,
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

/// Application data shared by native and GPUI frontends. No OS handles.
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
