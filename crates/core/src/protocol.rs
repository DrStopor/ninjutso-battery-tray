//! Ninjutso read-only feature reports. No settings-write commands are exposed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadRequest {
    DpiStage { profile: u8, stage: u8 },
    ActiveDpiStage { profile: u8 },
    PollingRate { profile: u8 },
    ActiveProfile,
    Charging,
    Battery,
    Online,
}

impl ReadRequest {
    pub const fn command(self) -> u8 {
        match self {
            Self::DpiStage { .. } => 0x04,
            Self::ActiveDpiStage { .. } => 0x1c,
            Self::PollingRate { .. } => 0x06,
            Self::ActiveProfile => 0x10,
            Self::Charging => 0x11,
            Self::Battery => 0x12,
            Self::Online => 0x13,
        }
    }

    pub fn encode(self) -> [u8; 16] {
        let mut report = [0; 16];
        report[0] = 0x06;
        report[1] = self.command();
        report[4] = 1;
        match self {
            Self::PollingRate { profile } | Self::ActiveDpiStage { profile } => report[7] = profile,
            Self::DpiStage { profile, stage } => {
                report[6] = 1; // One DPI slot to read; this is the GET opcode 0x04, not SET 0x03.
                report[7] = profile;
                report[8] = stage;
            }
            _ => {}
        }
        report
    }

    pub fn accepts(self, response: &[u8; 16]) -> bool {
        response[0] == 0x06 && response[1] == self.command()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Dpi, PollingRate};

    #[test]
    fn dpi_getter_reads_the_selected_profile_and_slot_without_changing_them() {
        for profile in [0, 1, 3, 255] {
            assert_eq!(
                ReadRequest::ActiveDpiStage { profile }.encode(),
                [6, 28, 0, 0, 1, 0, 0, profile, 0, 0, 0, 0, 0, 0, 0, 0]
            );
            for stage in 0..4 {
                assert_eq!(
                    ReadRequest::DpiStage { profile, stage }.encode(),
                    [6, 4, 0, 0, 1, 0, 1, profile, stage, 0, 0, 0, 0, 0, 0, 0]
                );
            }
        }
        let request = ReadRequest::DpiStage {
            profile: 3,
            stage: 2,
        };
        let mut response = request.encode();
        assert!(request.accepts(&response));
        response[1] = 3; // SET DPI must not be accepted as a GET response.
        assert!(!request.accepts(&response));
    }

    #[test]
    fn ten_dpi_decoding_is_bounded_and_never_wraps_corrupt_values() {
        assert_eq!(Dpi::from_device_level(21), Dpi::from_value(1100));
        assert_eq!(Dpi::from_device_level(1).unwrap().value(), 100);
        assert_eq!(Dpi::from_device_level(599).unwrap().value(), 30_000);
        for raw in [0, 600, u16::MAX] {
            assert_eq!(Dpi::from_device_level(raw), None);
        }
        for value in [0, 50, 99, 101, 1101, 30_050, u16::MAX] {
            assert_eq!(Dpi::from_value(value), None);
        }
    }

    #[test]
    fn polling_rate_request_uses_the_read_command_and_active_profile() {
        assert_eq!(
            ReadRequest::ActiveProfile.encode(),
            [6, 16, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        for profile in 0..=u8::MAX {
            assert_eq!(
                ReadRequest::PollingRate { profile }.encode(),
                [6, 6, 0, 0, 1, 0, 0, profile, 0, 0, 0, 0, 0, 0, 0, 0]
            );
        }
    }

    #[test]
    fn battery_read_frames_remain_unchanged_and_wrong_echoes_are_rejected() {
        for request in [
            ReadRequest::Online,
            ReadRequest::Battery,
            ReadRequest::Charging,
        ] {
            let command = request.command();
            assert_eq!(
                request.encode(),
                [6, command, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
            );
            let mut response = [0; 16];
            response[0] = 6;
            response[1] = command;
            assert!(request.accepts(&response));
            response[1] = 0x05; // Setter command must never be accepted as a getter response.
            assert!(!request.accepts(&response));
            response[1] = command;
            response[0] = 7;
            assert!(!request.accepts(&response));
        }
    }

    #[test]
    fn only_documented_polling_rate_codes_produce_a_value() {
        for code in 0..=u8::MAX {
            let expected = match code {
                1 => Some(1000),
                2 => Some(2000),
                3 => Some(4000),
                4 => Some(8000),
                _ => None,
            };
            assert_eq!(
                PollingRate::from_device_code(code).map(PollingRate::hz),
                expected
            );
            if let Some(hz) = expected {
                assert_eq!(
                    PollingRate::from_hz(hz),
                    PollingRate::from_device_code(code)
                );
            }
        }
        for hz in [0, 125, 500, 999, 1001, u16::MAX] {
            assert_eq!(PollingRate::from_hz(hz), None);
        }
    }
}
