//! Desktop host and platform adapters. Portable logic lives in the core crate.
pub use ninjutso_battery_core::{
    application, battery_service, i18n, model, presentation, protocol, settings,
};

#[cfg(all(windows, feature = "gpui-ui"))]
mod frontend;

#[cfg(windows)]
pub mod platform;

pub const APP_NAME: &str = "Ninjutso Battery Tray";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
