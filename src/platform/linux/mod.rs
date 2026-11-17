mod hid;
#[cfg(feature = "gpui-ui")]
pub(crate) mod instance;
mod logging;
mod settings;
#[cfg(feature = "gpui-ui")]
pub(crate) mod surface;
#[cfg(feature = "gpui-ui")]
pub(crate) mod tray;

pub use hid::read_mouse_battery;
pub use settings::{load as load_settings, save as save_settings};

pub fn run() {
    logging::init();
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "Ninjutso Battery Tray\n\n--once       Read battery/rate/DPI and exit (no desktop needed)\n--version    Print version\n\nDesktop: X11/XWayland, Vulkan and a StatusNotifierItem host.\nUSB access: install the supplied udev rule; never run the app with sudo."
        );
        return;
    }
    if std::env::args().any(|arg| arg == "--once") {
        match read_mouse_battery() {
            Ok(value) => println!(
                "{}",
                crate::presentation::view::status_text(
                    &crate::model::BatteryStatus::Ready(value),
                    load_settings().language
                )
            ),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    #[cfg(feature = "gpui-ui")]
    {
        if std::env::var_os("DISPLAY").is_none_or(|value| value.is_empty()) {
            eprintln!(
                "No X11 display. In a Wayland session enable XWayland; native Wayland is not supported by this build. Use --once for headless reading."
            );
            std::process::exit(1);
        }
        crate::frontend::run();
    }
    #[cfg(not(feature = "gpui-ui"))]
    {
        eprintln!(
            "This is a CLI-only build. Use --once, or rebuild with the default gpui-ui feature."
        );
        std::process::exit(1);
    }
}
