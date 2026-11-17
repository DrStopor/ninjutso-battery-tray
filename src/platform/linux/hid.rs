//! Unprivileged hidraw transport. Never detach drivers or change mouse settings.
use hidapi::{HidApi, HidError};

use crate::model::{ConnectionKind, MouseBattery, ReadError, ReadErrorKind};
use crate::telemetry::query_battery_with;

pub fn read_mouse_battery() -> Result<MouseBattery, ReadError> {
    let api = HidApi::new().map_err(|e| ReadError::new(ReadErrorKind::System, e.to_string()))?;
    let found = api
        .device_list()
        .any(|d| d.vendor_id() == 0x093a && matches!(d.product_id(), 0xe020 | 0xeb01));
    let mut offline = None;
    let mut denied = false;
    let mut last_error = "No supported vendor HID collection (usage ff01:0001)".to_string();
    // Prefer an online USB mouse, then the receiver; an offline receiver must not
    // mask a working wired device (or vice versa).
    for (product, connection) in [
        (0xe020, ConnectionKind::Wired),
        (0xeb01, ConnectionKind::Receiver),
    ] {
        for info in api
            .device_list()
            .filter(|d| d.vendor_id() == 0x093a && d.product_id() == product)
            // Both Ten transports expose report 0x06 in vendor usage ff01:0001.
            // hidraw paths may also enumerate keyboard/mouse/consumer usages;
            // never send telemetry requests to those input collections.
            .filter(|d| d.usage_page() == 0xff01 && d.usage() == 1)
        {
            let device = match info.open_device(&api) {
                Ok(device) => device,
                Err(error) => {
                    denied |= matches!(&error, HidError::IoError { error } if error.kind() == std::io::ErrorKind::PermissionDenied);
                    last_error = error.to_string();
                    continue;
                }
            };
            let reading = query_battery_with(connection, |request| {
                device
                    .send_feature_report(&request.encode())
                    .map_err(|e| e.to_string())?;
                std::thread::sleep(std::time::Duration::from_millis(30));
                let mut response = [0; 16];
                response[0] = 6;
                let count = device
                    .get_feature_report(&mut response)
                    .map_err(|e| e.to_string())?;
                if !response
                    .get(..count)
                    .is_some_and(|report| request.accepts(report))
                {
                    return Err(format!(
                        "Invalid feature report for command {} ({count} bytes)",
                        request.command()
                    ));
                }
                Ok(response)
            });
            match reading {
                Ok(value) if value.online => return Ok(value),
                Ok(value) => {
                    offline.get_or_insert(value);
                }
                Err(error) => last_error = error,
            }
        }
    }
    if let Some(value) = offline {
        return Ok(value);
    }
    if !found {
        Err(ReadError::new(
            ReadErrorKind::DeviceNotFound,
            "Ninjutso Ten not found (093a:e020 / 093a:eb01)",
        ))
    } else if denied {
        Err(ReadError::new(
            ReadErrorKind::PermissionDenied,
            "HID access denied. Install 70-ninjutso-battery-tray.rules and reconnect the mouse/receiver; do not run the app as root.",
        ))
    } else {
        Err(ReadError::new(ReadErrorKind::NoBatteryData, last_error))
    }
}
