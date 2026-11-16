//! Platform-independent application data and presentation for tray frontends.
pub mod i18n;
pub mod model;
pub mod presentation;
pub mod settings;

#[cfg(windows)]
pub mod platform;

pub const APP_NAME: &str = "Ninjutso Battery Tray";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
