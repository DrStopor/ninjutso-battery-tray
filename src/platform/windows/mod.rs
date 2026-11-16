#[cfg(not(feature = "gpui-ui"))]
mod app;
#[cfg(feature = "gpui-ui")]
mod context_menu;
mod hid;
#[cfg(feature = "gpui-ui")]
pub(crate) mod instance;
#[cfg(feature = "gpui-ui")]
mod menu_surface;
#[cfg(feature = "gpui-ui")]
pub(crate) mod native;
mod settings;
#[cfg(feature = "gpui-ui")]
pub(crate) mod tray;

#[cfg(feature = "gpui-ui")]
pub use crate::frontend::run;
#[cfg(not(feature = "gpui-ui"))]
pub use app::run;
pub use hid::read_mouse_battery;
pub use settings::{load as load_settings, save as save_settings};
