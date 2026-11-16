#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!(
            "{} {}",
            ninjutso_battery_tray::APP_NAME,
            ninjutso_battery_tray::APP_VERSION
        );
        return;
    }
    #[cfg(windows)]
    ninjutso_battery_tray::platform::windows::run();

    #[cfg(not(windows))]
    {
        eprintln!("The tray application currently supports Windows only.");
        std::process::exit(1);
    }
}
