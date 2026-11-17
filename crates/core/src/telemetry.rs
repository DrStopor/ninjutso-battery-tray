//! Shared read-only telemetry pipeline; transports only exchange verified reports.
use crate::model::{ConnectionKind, Dpi, MouseBattery, PollingRate};
use crate::protocol::ReadRequest;

pub fn query_battery_with(
    connection: ConnectionKind,
    mut query: impl FnMut(ReadRequest) -> Result<[u8; 16], String>,
) -> Result<MouseBattery, String> {
    let online = query(ReadRequest::Online)?;
    let battery = query(ReadRequest::Battery)?;
    let charge = query(ReadRequest::Charging)?;
    if battery[8] > 100 {
        return Err(format!("unexpected battery percentage: {}", battery[8]));
    }

    let online = online[8] == 1;
    // Optional telemetry must never turn a valid battery reading into an error.
    // Query the active profile every time; do not reuse a rate from another transport.
    let profile = if online {
        query(ReadRequest::ActiveProfile)
            .ok()
            .map(|response| response[8])
    } else {
        None
    };
    let polling_rate = profile.and_then(|profile| {
        query(ReadRequest::PollingRate { profile })
            .ok()
            .and_then(|response| PollingRate::from_device_code(response[8]))
    });
    let dpi = profile.and_then(|profile| {
        let stage = query(ReadRequest::ActiveDpiStage { profile }).ok()?[8];
        // Ten exposes four zero-based stages. Never issue a request for an unknown slot.
        if stage >= 4 {
            return None;
        }
        let response = query(ReadRequest::DpiStage { profile, stage }).ok()?;
        Dpi::from_device_level(u16::from_le_bytes([response[8], response[9]]))
    });
    Ok(MouseBattery {
        percent: battery[8],
        online,
        charging: charge[8] != 0,
        connection,
        polling_rate,
        dpi,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(request: ReadRequest) -> Result<[u8; 16], String> {
        let mut response = request.encode();
        response[8] = match request {
            ReadRequest::Battery => 72,
            ReadRequest::ActiveProfile => 3,
            ReadRequest::PollingRate { profile } => {
                assert_eq!(profile, 3);
                1
            }
            ReadRequest::ActiveDpiStage { profile } => {
                assert_eq!(profile, 3);
                2
            }
            ReadRequest::DpiStage { profile, stage } => {
                assert_eq!(profile, 3);
                assert_eq!(stage, 2);
                21 // (21 + 1) * 50 = 1100 DPI.
            }
            ReadRequest::Online => 1,
            ReadRequest::Charging => 0,
        };
        Ok(response)
    }

    #[test]
    fn reads_the_rate_of_the_active_profile_on_both_transports() {
        for connection in [ConnectionKind::Wired, ConnectionKind::Receiver] {
            let status = query_battery_with(connection, fixture).unwrap();
            assert_eq!(status.polling_rate, Some(PollingRate::Hz1000));
            assert_eq!(status.dpi, Dpi::from_value(1100));
            assert_eq!(status.connection, connection);
            assert_eq!(status.percent, 72);
        }
    }

    #[test]
    fn unsupported_or_failed_polling_reads_do_not_break_the_battery() {
        for fail_profile in [false, true] {
            let status = query_battery_with(ConnectionKind::Receiver, |request| {
                if matches!(request, ReadRequest::ActiveProfile) && fail_profile {
                    return Err("unsupported active-profile command".into());
                }
                if matches!(request, ReadRequest::PollingRate { .. }) {
                    return Err("unsupported polling-rate command".into());
                }
                fixture(request)
            })
            .unwrap();
            assert_eq!(status.percent, 72);
            assert_eq!(status.polling_rate, None);
        }
        let status = query_battery_with(ConnectionKind::Wired, |request| {
            let mut response = fixture(request)?;
            if matches!(request, ReadRequest::PollingRate { .. }) {
                response[8] = 0xff;
            }
            Ok(response)
        })
        .unwrap();
        assert_eq!(status.polling_rate, None);
    }

    #[test]
    fn offline_mouse_does_not_query_or_keep_a_polling_rate() {
        let status = query_battery_with(ConnectionKind::Receiver, |request| {
            assert!(!matches!(
                request,
                ReadRequest::ActiveProfile
                    | ReadRequest::PollingRate { .. }
                    | ReadRequest::ActiveDpiStage { .. }
                    | ReadRequest::DpiStage { .. }
            ));
            let mut response = fixture(request)?;
            if request == ReadRequest::Online {
                response[8] = 0;
            }
            Ok(response)
        })
        .unwrap();
        assert!(!status.online);
        assert_eq!(status.polling_rate, None);
        assert_eq!(status.dpi, None);
    }

    #[test]
    fn dpi_failure_keeps_the_battery_and_polling_rate_and_never_reads_an_invalid_slot() {
        for failed_stage in [true, false] {
            let status = query_battery_with(ConnectionKind::Receiver, |request| {
                if matches!(request, ReadRequest::ActiveDpiStage { .. }) && failed_stage {
                    return Err("DPI stage unavailable".into());
                }
                if matches!(request, ReadRequest::DpiStage { .. }) {
                    return Err("DPI unavailable".into());
                }
                fixture(request)
            })
            .unwrap();
            assert_eq!(status.percent, 72);
            assert_eq!(status.polling_rate, Some(PollingRate::Hz1000));
            assert_eq!(status.dpi, None);
        }
        let status = query_battery_with(ConnectionKind::Receiver, |request| {
            assert!(!matches!(request, ReadRequest::DpiStage { .. }));
            let mut response = fixture(request)?;
            if matches!(request, ReadRequest::ActiveDpiStage { .. }) {
                response[8] = 255;
            }
            Ok(response)
        })
        .unwrap();
        assert_eq!(status.dpi, None);
        assert_eq!(status.polling_rate, Some(PollingRate::Hz1000));
    }

    #[test]
    fn polling_failure_does_not_prevent_an_independent_dpi_read() {
        let status = query_battery_with(ConnectionKind::Receiver, |request| {
            if matches!(request, ReadRequest::PollingRate { .. }) {
                return Err("rate unavailable".into());
            }
            fixture(request)
        })
        .unwrap();
        assert_eq!(status.percent, 72);
        assert_eq!(status.polling_rate, None);
        assert_eq!(status.dpi, Dpi::from_value(1100));
    }
}
