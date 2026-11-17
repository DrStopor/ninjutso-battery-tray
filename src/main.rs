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

    #[cfg(target_os = "linux")]
    ninjutso_battery_tray::platform::linux::run();

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        eprintln!("The tray application supports Windows and Linux.");
        std::process::exit(1);
    }
}
