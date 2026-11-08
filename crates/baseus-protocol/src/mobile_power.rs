//! App-layer protocol primitives extracted from Baseus Intelligent 2.18.0 (185)
//! for the Blade 2-140 mobile-power profile.
//!
//! The app sends these bytes through its BLE SDK after loading model-specific GATT
//! configuration. This module deliberately does not guess service/characteristic
//! UUIDs, port-bit meanings, or device capability. It is not hardware-verified.

use thiserror::Error;

const PREFIX: [u8; 2] = [0x9A, 0xAA];
const READ: u8 = 0x03;
const WRITE: u8 = 0x10;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MobilePowerError {
    #[error("invalid response: {0}")]
    InvalidResponse(&'static str),
    #[error("CRC mismatch: computed {computed:#06X}, carried {carried:#06X}")]
    BadChecksum { computed: u16, carried: u16 },
    #[error("screen text is empty")]
    EmptyScreenText,
    #[error("screen text is {0} bytes; protocol length field supports at most 255")]
    ScreenTextTooLong(usize),
    #[error("theme index must be one of 0, 1 or 2")]
    InvalidTheme,
    #[error("timer must be at most 1440 minutes")]
    InvalidTimer,
}

/// Parsed common response fields. `value` remains raw because different codes have
/// different app-side units and offsets (for example temperatures subtract 100).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusResponse {
    pub code: u16,
    pub multiplier: u8,
    pub value: u16,
}

impl StatusResponse {
    /// The statistics screen divides the raw value by this response multiplier.
    /// A zero multiplier is reported as `None`, never as an infinity or guessed unit.
    pub fn scaled_value(self) -> Option<f32> {
        (self.multiplier != 0).then(|| f32::from(self.value) / f32::from(self.multiplier))
    }
}

/// Calculate the CRC16 used by `EnergyStorageUtil` in the Baseus app. The app's
/// table is the byte-swapped representation of the standard reflected 0xA001 table;
/// it formats the resulting u16 as four uppercase hex digits and appends that value
/// in network byte order to the hex-decoded application payload.
pub fn crc16(data: &[u8]) -> u16 {
    data.iter().fold(0xFFFF_u16, |mut crc, byte| {
        crc ^= u16::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xA001
            } else {
                crc >> 1
            };
        }
        crc
    })
}

fn append_crc(mut packet: Vec<u8>) -> Vec<u8> {
    let checksum = crc16(&packet);
    packet.extend_from_slice(&checksum.to_be_bytes());
    packet
}

/// Build the app-layer read request: `9AAA03 + code + 0001 + CRC`.
pub fn read_request(code: u16) -> Vec<u8> {
    let mut packet = Vec::with_capacity(9);
    packet.extend_from_slice(&PREFIX);
    packet.push(READ);
    packet.extend_from_slice(&code.to_be_bytes());
    packet.extend_from_slice(&[0x00, 0x01]);
    append_crc(packet)
}

/// Build the standard app-layer u16 write: `9AAA10 + code + 0001 + 0201 + value + CRC`.
/// This is intentionally a low-level builder; callers must only use code/value pairs
/// whose meaning is established for the connected model and firmware.
pub fn write_u16(code: u16, value: u16) -> Vec<u8> {
    let mut packet = Vec::with_capacity(13);
    packet.extend_from_slice(&PREFIX);
    packet.push(WRITE);
    packet.extend_from_slice(&code.to_be_bytes());
    packet.extend_from_slice(&[0x00, 0x01, 0x02, 0x01]);
    packet.extend_from_slice(&value.to_be_bytes());
    append_crc(packet)
}

/// Screen theme presets shown in Baseus Intelligent's Blade 2 theme picker.
/// These are three presets, not an arbitrary RGB color selector.
pub fn set_theme(index: u8) -> Result<Vec<u8>, MobilePowerError> {
    if index > 2 {
        return Err(MobilePowerError::InvalidTheme);
    }
    Ok(write_u16(0x0020, u16::from(index)))
}

/// Set the screen-saver on/off state through code 0022.
pub fn set_screen_saver(enabled: bool) -> Vec<u8> {
    write_u16(0x0022, if enabled { 1 } else { 0 })
}

/// Set Blade 2's custom display text. The vendor app sends UTF-8 bytes behind
/// `9AAA1000300001 + byte_length + 01`, followed by the shared CRC.
pub fn set_screen_text(text: &str) -> Result<Vec<u8>, MobilePowerError> {
    if text.trim().is_empty() {
        return Err(MobilePowerError::EmptyScreenText);
    }
    let bytes = text.as_bytes();
    let len =
        u8::try_from(bytes.len()).map_err(|_| MobilePowerError::ScreenTextTooLong(bytes.len()))?;

    let mut packet = Vec::with_capacity(10 + bytes.len());
    packet.extend_from_slice(&PREFIX);
    packet.push(WRITE);
    packet.extend_from_slice(&0x0030_u16.to_be_bytes());
    packet.extend_from_slice(&[0x00, 0x01, len, 0x01]);
    packet.extend_from_slice(bytes);
    Ok(append_crc(packet))
}

/// Build the app's countdown writes. Baseus clamps values above 1440 minutes when
/// reading state; this helper also rejects such values instead of silently changing
/// the requested timer.
pub fn set_countdown(code: Countdown, minutes: u16) -> Result<Vec<u8>, MobilePowerError> {
    if minutes > 1440 {
        return Err(MobilePowerError::InvalidTimer);
    }
    Ok(write_u16(code as u16, minutes))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Countdown {
    Reminder = 0x001E,
    AutomaticShutdown = 0x001F,
}

/// Decode and CRC-check a status notification before exposing its common fields.
/// Layout follows the app parser: `9AAA 03 code 00 multiplier value CRC16`.
pub fn decode_status(packet: &[u8]) -> Result<StatusResponse, MobilePowerError> {
    if packet.len() < 11 {
        return Err(MobilePowerError::InvalidResponse("too short"));
    }
    if packet[0..2] != PREFIX || packet[2] != READ {
        return Err(MobilePowerError::InvalidResponse("unexpected header"));
    }

    let (body, trailer) = packet.split_at(packet.len() - 2);
    let carried = u16::from_be_bytes([trailer[0], trailer[1]]);
    let computed = crc16(body);
    if computed != carried {
        return Err(MobilePowerError::BadChecksum { computed, carried });
    }

    let code = u16::from_be_bytes([packet[3], packet[4]]);
    let body_len = packet.len() - 2;
    if body_len != 9 {
        return Err(MobilePowerError::InvalidResponse(
            "unexpected status length",
        ));
    }
    if packet[5] != 0 {
        return Err(MobilePowerError::InvalidResponse("unexpected value prefix"));
    }
    let multiplier = packet[6];
    let value = u16::from_be_bytes([packet[7], packet[8]]);
    Ok(StatusResponse {
        code,
        multiplier,
        value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_standard_check_value() {
        assert_eq!(crc16(b"123456789"), 0x4B37);
    }

    #[test]
    fn request_builders_follow_the_vendor_payload_layout() {
        let read = read_request(0x000D);
        assert_eq!(&read[..7], &[0x9A, 0xAA, 0x03, 0x00, 0x0D, 0x00, 0x01]);
        assert_eq!(u16::from_be_bytes([read[7], read[8]]), crc16(&read[..7]));

        let write = write_u16(0x0020, 0x0002);
        assert_eq!(
            &write[..11],
            &[0x9A, 0xAA, 0x10, 0x00, 0x20, 0x00, 0x01, 0x02, 0x01, 0x00, 0x02]
        );
        assert_eq!(
            u16::from_be_bytes([write[11], write[12]]),
            crc16(&write[..11])
        );
    }

    #[test]
    fn screen_text_is_utf8_and_has_a_byte_length_prefix() {
        let packet = set_screen_text("OK").unwrap();
        assert_eq!(
            &packet[..9],
            &[0x9A, 0xAA, 0x10, 0x00, 0x30, 0x00, 0x01, 0x02, 0x01]
        );
        assert_eq!(&packet[9..11], b"OK");
        assert!(matches!(
            set_screen_text("  "),
            Err(MobilePowerError::EmptyScreenText)
        ));
        assert!(matches!(
            set_screen_text(&"x".repeat(256)),
            Err(MobilePowerError::ScreenTextTooLong(256))
        ));
    }

    #[test]
    fn response_decoding_checks_crc_and_preserves_raw_value() {
        let body = [0x9A, 0xAA, 0x03, 0x00, 0x0D, 0x00, 0x01, 0x00, 0x64];
        let packet = append_crc(body.to_vec());
        let parsed = decode_status(&packet).unwrap();
        assert_eq!(
            parsed,
            StatusResponse {
                code: 0x000D,
                multiplier: 1,
                value: 100
            }
        );
        assert_eq!(parsed.scaled_value(), Some(100.0));

        let mut corrupt = packet;
        corrupt[8] ^= 1;
        assert!(matches!(
            decode_status(&corrupt),
            Err(MobilePowerError::BadChecksum { .. })
        ));
    }

    #[test]
    fn typed_screen_and_timer_controls_enforce_known_ui_ranges() {
        assert!(set_theme(2).is_ok());
        assert!(matches!(set_theme(3), Err(MobilePowerError::InvalidTheme)));
        assert_eq!(set_screen_saver(true)[10], 0x01);
        assert!(set_countdown(Countdown::Reminder, 1440).is_ok());
        assert!(matches!(
            set_countdown(Countdown::AutomaticShutdown, 1441),
            Err(MobilePowerError::InvalidTimer)
        ));
    }
}
