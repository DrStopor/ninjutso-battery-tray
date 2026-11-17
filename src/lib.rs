//! Desktop host and platform adapters. Portable logic lives in the core crate.
pub use ninjutso_battery_core::{
    application, battery_service, i18n, model, presentation, protocol, settings, telemetry,
};

#[cfg(all(any(windows, target_os = "linux"), feature = "gpui-ui"))]
mod frontend;

#[cfg(any(windows, target_os = "linux"))]
pub mod platform;

pub const APP_NAME: &str = "Ninjutso Battery Tray";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
