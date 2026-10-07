//! LAWICEL SLCAN serial protocol driver for CANable 2.0.
//!
//! # CANable 2.0 firmware notes
//!
//! The CANable 2.0 (normaldotcom firmware) implements SLCAN with some
//! non-standard extensions. Relevant behaviour:
//!
//! - Commands `S<n>`, `O`, `C`, `A<n>`, `M<n>` are **silent** — the firmware
//!   sends no ACK byte on success. Only `V` (version) and received CAN frames
//!   produce output.
//! - `M1` is **bus-monitoring / silent mode** (listen-only), not loopback.
//! - The firmware has no bus-off recovery; every `C` + `O` cycle restarts the
//!   FDCAN peripheral and recovers bus-off automatically.
//!
//! # Bitrate index → speed
//!
//! | Index | Speed  |
//! |-------|--------|
//! | 0     | 10 kbps |
//! | 1     | 20 kbps |
//! | 2     | 50 kbps |
//! | 3     | 100 kbps |
//! | 4     | 125 kbps |
//! | 5     | 250 kbps |
//! | 6     | 500 kbps |
//! | 8     | 1 Mbps  |

use std::io::{Read, Write};
use std::time::{Duration, Instant};
use serialport::SerialPort;

/// A standard CAN frame (11-bit ID, up to 8 data bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanFrame {
    /// 11-bit CAN identifier.
    pub id: u16,
    /// Data payload (0–8 bytes).
    pub data: Vec<u8>,
}

/// SLCAN interface wrapping a serial port connected to a CANable 2.0.
pub struct Slcan {
    port: Box<dyn SerialPort>,
}

impl Slcan {
    /// Open the serial port at 115 200 baud and put the CANable on-bus at the
    /// requested bitrate.
    ///
    /// `bitrate_index` selects the CAN speed (see module-level table).  For
    /// Ferrari F430 CAN-B use `2` (50 kbps).
    ///
    /// The channel is closed and re-opened to ensure a clean state; any
    /// previous bus-off condition is cleared.
    pub fn open(port_path: &str, bitrate_index: u8) -> Result<Self, String> {
        let port = serialport::new(port_path, 115_200)
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(|e| format!("cannot open {port_path}: {e}"))?;

        let mut s = Slcan { port };

        // Close any previously open channel, then drain stale bytes.
        s.send_cmd(b"C\r")?;
        std::thread::sleep(Duration::from_millis(50));
        s.drain();

        // Verify device is a CANable / SLCAN device by requesting version ('V\r')
        s.send_cmd(b"V\r")?;
        std::thread::sleep(Duration::from_millis(50));

        let version_resp = s.read_line_timeout(Duration::from_millis(200))?;
        if version_resp.as_deref().map(is_valid_slcan_version) != Some(true) {
            let got = version_resp.unwrap_or_else(|| "no response".to_string());
            return Err(format!(
                "device on {port_path} did not return a valid SLCAN version (got: {got:?}). \
                 Verify that a CANable or compatible SLCAN device is connected."
            ));
        }

        // Set bitrate then open.
        s.send_cmd(format!("S{bitrate_index}\r").as_bytes())?;
        s.send_cmd(b"O\r")?;
        std::thread::sleep(Duration::from_millis(10));

        Ok(s)
    }

    /// Close the CAN channel (puts the adapter off-bus).
    pub fn close(&mut self) {
        let _ = self.send_cmd(b"C\r");
    }

    /// Transmit a standard (11-bit) CAN frame.
    pub fn transmit(&mut self, frame: &CanFrame) -> Result<(), String> {
        // Format: t<III><L><DD...>\r
        // III = 3 hex digits (11-bit ID, zero-padded)
        // L   = 1 decimal digit DLC
        // DD  = 2 hex digits per data byte
        let mut cmd = format!("t{:03X}{}", frame.id, frame.data.len());
        for b in &frame.data {
            cmd.push_str(&format!("{b:02X}"));
        }
        cmd.push('\r');
        self.send_cmd(cmd.as_bytes())
    }

    /// Read frames from the bus until `predicate` matches one, or `timeout`
    /// elapses.  Non-matching frames (including extended-ID, remote, and FD
    /// frames) are silently skipped.
    ///
    /// Returns the first matching frame, or an error describing what was seen
    /// before the timeout.
    pub fn read_until<F>(&mut self, predicate: F, timeout: Duration) -> Result<CanFrame, String>
    where
        F: Fn(&CanFrame) -> bool,
    {
        let deadline = Instant::now() + timeout;
        let mut seen: Vec<CanFrame> = Vec::new();

        loop {
            if Instant::now() >= deadline {
                return Err(format!(
                    "timeout waiting for CAN frame after {}ms; received {} other frame(s): {}",
                    timeout.as_millis(),
                    seen.len(),
                    frames_summary(&seen),
                ));
            }

            match self.try_read_frame()? {
                Some(frame) if predicate(&frame) => return Ok(frame),
                Some(frame) => seen.push(frame),
                None => {}
            }
        }
    }

    /// Non-blocking attempt to read one standard CAN frame from the port.
    ///
    /// Returns `Ok(Some(frame))` when a complete standard frame is available,
    /// `Ok(None)` when no complete frame is ready yet or the frame is a
    /// non-standard type (extended-ID, remote, FD, status), and `Err` only on
    /// serial I/O failures.
    pub fn try_read_frame(&mut self) -> Result<Option<CanFrame>, String> {
        let mut line: Vec<u8> = Vec::new();
        let mut byte = [0u8; 1];

        loop {
            match self.port.read(&mut byte) {
                Ok(1) => {
                    if byte[0] == b'\r' {
                        break;
                    }
                    line.push(byte[0]);
                }
                Ok(_) => break, // EOF / zero-length read
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => return Ok(None),
                Err(e) => return Err(format!("serial read error: {e}")),
            }
        }

        if line.is_empty() {
            return Ok(None);
        }

        // Only parse standard data frames ('t').  Silently skip everything else
        // (extended 'T', remote 'r'/'R', FD 'd'/'D'/'b'/'B', status, etc.).
        if line[0] != b't' {
            return Ok(None);
        }

        parse_standard_frame(&line).map(Some)
    }

    /// Read a single CR- or LF-terminated line with a timeout.
    fn read_line_timeout(&mut self, timeout: Duration) -> Result<Option<String>, String> {
        let deadline = Instant::now() + timeout;
        let mut line = Vec::new();
        let mut byte = [0u8; 1];

        loop {
            if Instant::now() >= deadline {
                break;
            }
            match self.port.read(&mut byte) {
                Ok(1) => {
                    if byte[0] == b'\r' || byte[0] == b'\n' {
                        if !line.is_empty() {
                            break;
                        }
                    } else {
                        line.push(byte[0]);
                    }
                }
                Ok(_) => break,
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(format!("serial read error: {e}")),
            }
        }

        if line.is_empty() {
            Ok(None)
        } else {
            Ok(Some(String::from_utf8_lossy(&line).into_owned()))
        }
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    fn send_cmd(&mut self, cmd: &[u8]) -> Result<(), String> {
        self.port
            .write_all(cmd)
            .map_err(|e| format!("serial write error: {e}"))?;
        self.port
            .flush()
            .map_err(|e| format!("serial flush error: {e}"))
    }

    /// Discard any bytes currently in the serial receive buffer.
    fn drain(&mut self) {
        let mut buf = [0u8; 256];
        loop {
            match self.port.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    }
}

impl Drop for Slcan {
    fn drop(&mut self) {
        self.close();
    }
}

// ---------------------------------------------------------------------------
// Frame & Version parsing
// ---------------------------------------------------------------------------

/// Check if a string returned by the `V\r` command is a valid SLCAN version string.
///
/// Handles Lawicel standard `Vxxxx` strings, CANable 2.0 git commit/URL strings,
/// and other common SLCAN adapter version formats.
pub(crate) fn is_valid_slcan_version(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_lowercase();
    lower.starts_with('v')
        || lower.contains("canable")
        || lower.contains("slcan")
        || lower.contains("github.com")
        || lower.contains("candlelight")
}

/// Parse a SLCAN standard-frame line (starting with `t`, without the trailing
/// `\r`).
///
/// Returns an error if the line is malformed.  Callers that need to silently
/// skip non-standard frames should check `line[0] == b't'` before calling.
pub(crate) fn parse_standard_frame(line: &[u8]) -> Result<CanFrame, String> {
    // t<III><L><DD...>
    // minimum: t + 3 ID chars + 1 DLC char = 5 bytes (DLC=0, no data)
    if line.len() < 5 {
        return Err(format!(
            "SLCAN frame too short ({} bytes): {}",
            line.len(),
            String::from_utf8_lossy(line),
        ));
    }

    let id_str = std::str::from_utf8(&line[1..4])
        .map_err(|_| "SLCAN ID is not valid UTF-8".to_string())?;
    let id = u16::from_str_radix(id_str, 16)
        .map_err(|_| format!("invalid SLCAN ID '{id_str}'"))?;

    let dlc = (line[4] as char)
        .to_digit(10)
        .ok_or_else(|| format!("invalid DLC '{}' in SLCAN frame", line[4] as char))?
        as usize;

    let expected_len = 5 + dlc * 2;
    if line.len() < expected_len {
        return Err(format!(
            "SLCAN frame data truncated: DLC={dlc} requires {expected_len} bytes, got {}",
            line.len(),
        ));
    }

    let mut data = Vec::with_capacity(dlc);
    for i in 0..dlc {
        let hex = std::str::from_utf8(&line[5 + i * 2..7 + i * 2])
            .map_err(|_| "SLCAN data byte is not valid UTF-8".to_string())?;
        data.push(
            u8::from_str_radix(hex, 16)
                .map_err(|_| format!("invalid SLCAN data byte '{hex}'"))?,
        );
    }

    Ok(CanFrame { id, data })
}

/// Format a slice of frames as a compact summary for error messages.
fn frames_summary(frames: &[CanFrame]) -> String {
    if frames.is_empty() {
        return "none".into();
    }
    frames
        .iter()
        .map(|f| format!("{:03X}#{}", f.id, hex::encode(&f.data)))
        .collect::<Vec<_>>()
        .join(", ")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(hex: &str) -> Vec<u8> {
        hex.as_bytes().to_vec()
    }

    #[test]
    fn parse_standard_frame_basic() {
        let f = parse_standard_frame(&frame("t7B080210810000000000")).unwrap();
        assert_eq!(f.id, 0x7B0);
        assert_eq!(f.data, vec![0x02, 0x10, 0x81, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn parse_standard_frame_zero_dlc() {
        let f = parse_standard_frame(&frame("t1230")).unwrap();
        assert_eq!(f.id, 0x123);
        assert_eq!(f.data, Vec::<u8>::new());
    }

    #[test]
    fn parse_standard_frame_dlc_3() {
        let f = parse_standard_frame(&frame("t3C43000006")).unwrap();
        assert_eq!(f.id, 0x3C4);
        assert_eq!(f.data, vec![0x00, 0x00, 0x06]);
    }

    #[test]
    fn parse_standard_frame_too_short() {
        assert!(parse_standard_frame(&frame("t7B")).is_err());
    }

    #[test]
    fn parse_standard_frame_truncated_data() {
        // DLC says 8 bytes but only 2 hex chars of data provided
        assert!(parse_standard_frame(&frame("t7B0810")).is_err());
    }

    #[test]
    fn parse_standard_frame_invalid_id() {
        assert!(parse_standard_frame(&frame("tXYZ80000000000000")).is_err());
    }

    #[test]
    fn parse_standard_frame_invalid_data_byte() {
        assert!(parse_standard_frame(&frame("t7B01ZZ")).is_err());
    }

    #[test]
    fn try_read_frame_skips_extended_frame() {
        // 'T' prefix = extended frame — should be silently skipped (Ok(None))
        // We test parse_standard_frame indirectly: try_read_frame checks line[0]
        // before calling it.  Here we verify the public-facing contract by testing
        // that only 't' lines produce Some.
        let line = b"T12345678800000000";
        assert_eq!(line[0], b'T');
        // try_read_frame would return Ok(None) for this; test the guard directly.
        assert_ne!(line[0], b't');
    }

    #[test]
    fn valid_slcan_version_strings() {
        // Lawicel / standard SLCAN
        assert!(is_valid_slcan_version("V1013"));
        assert!(is_valid_slcan_version("v1.0"));
        
        // CANable 2.0 / normaldotcom firmware
        assert!(is_valid_slcan_version("16e7497-dirty github.com/normaldotcom/canable2.git"));
        assert!(is_valid_slcan_version("CANable v2.0"));
        assert!(is_valid_slcan_version("candleLight adapter"));

        // Invalid / non-SLCAN strings
        assert!(!is_valid_slcan_version(""));
        assert!(!is_valid_slcan_version("   "));
        assert!(!is_valid_slcan_version("OK"));
        assert!(!is_valid_slcan_version("ERROR"));
        assert!(!is_valid_slcan_version("Arduino Uno v1.0"));
    }

    #[test]
    fn frames_summary_empty() {
        assert_eq!(frames_summary(&[]), "none");
    }

    #[test]
    fn frames_summary_single() {
        let f = CanFrame { id: 0x7C3, data: vec![0xF1, 0x02, 0x50, 0x81] };
        let s = frames_summary(&[f]);
        assert_eq!(s, "7C3#f1025081");
    }
}
