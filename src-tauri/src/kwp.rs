//! KWP2000 (ISO 14230-3) service layer for the F430 NQS ECU.
//!
//! # Services implemented
//!
//! | SID  | Service                       |
//! |------|-------------------------------|
//! | 0x10 | StartDiagnosticSession        |
//! | 0x21 | ReadDataByLocalIdentifier     |
//! | 0x27 | SecurityAccess                |
//! | 0x3B | WriteDataByLocalIdentifier    |
//! | 0x3E | TesterPresent                 |
//!
//! Positive response SID = request SID | 0x40.
//! Negative response format: `[0x7F, request_sid, NRC]`.
//!
//! # Security access algorithm (F430 base, ECU 209)
//!
//! Source: `TestSecurityAccessQuadroPerAzzeramento` in `F458_LIB.cs`
//! (SDX WAYCON_LIB decompiled source).
//!
//! 1. `StartDiagnosticSession 0x81` (default) then `0x83` (programming).
//! 2. Send `0x27 0x07` → response `0x67 0x07 [seed_hi, seed_lo]`.
//! 3. Compute key:
//!    - `key_lo = (seed_lo ^ 0xFF).wrapping_add(3)`
//!    - `key_hi = (seed_hi ^ 0xFF).wrapping_add(3)`
//! 4. Send `0x27 0x08 key_lo key_hi` → response `0x67 0x08`.
//!
//! The `valoreDaAggiungere` constant (3) applies to F430 base (`ID_PROCEDURA_KEYs = 2`).
//! F430 Scuderia uses ECU 325 with the same parameters.

use automotive_diag::kwp2000::{KwpCommand, KwpError, KwpSessionType};
use crate::isotp;
use crate::slcan::Slcan;

const SID_NEGATIVE_RESPONSE: u8 = 0x7F;

// TesterPresent is available for callers that need keepalives during long
// operations, but not used in the current single-shot read/write flows.
#[allow(dead_code)]
const SID_TESTER_PRESENT: u8 = KwpCommand::TesterPresent as u8;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Open a KWP2000 default diagnostic session.
///
/// Required before reading live data or escalating to a programming session.
pub fn start_session(slcan: &mut Slcan) -> Result<(), String> {
    let sid = KwpCommand::StartDiagnosticSession as u8;
    let resp = isotp::transact(slcan, &[sid, KwpSessionType::Normal as u8])?;
    check_positive(&resp, sid)
}

/// Open a KWP2000 programming session.
///
/// Must be called after [`start_session`].  Required before security access
/// and write operations.
///
/// Uses session type 0x83 — a Ferrari-specific extended session mode not in
/// the standard KWP2000 session type enum.
pub fn start_programming_session(slcan: &mut Slcan) -> Result<(), String> {
    let sid = KwpCommand::StartDiagnosticSession as u8;
    let resp = isotp::transact(slcan, &[sid, 0x83])?;
    check_positive(&resp, sid)
}

/// Send a TesterPresent keepalive to prevent session timeout.
///
/// Uses sub-function 0x02 (suppressPosRspMsgIndicationBit), so the ECU does
/// not send a response.  Call this periodically (every ~2 s) during long
/// operations such as flashing or multi-step procedures.
#[allow(dead_code)]
pub fn tester_present(slcan: &mut Slcan) -> Result<(), String> {
    let sid = KwpCommand::TesterPresent as u8;
    isotp::transact(slcan, &[sid, 0x02])
        .map(|_| ())
        .or_else(|e| {
            // A timeout is acceptable since we suppressed the positive response.
            if e.contains("timeout") { Ok(()) } else { Err(e) }
        })
}

/// Read a data block by local identifier.
///
/// Returns the raw data bytes, stripping the response header `[0x61, local_id]`.
pub fn read_data_by_local_id(slcan: &mut Slcan, local_id: u8) -> Result<Vec<u8>, String> {
    let sid = KwpCommand::ReadDataByLocalIdentifier as u8;
    let resp = isotp::transact(slcan, &[sid, local_id])?;
    check_positive(&resp, sid)?;

    // Positive response: [0x61, local_id, data...]
    if resp.len() < 2 {
        return Err(format!(
            "ReadDataByLocalIdentifier response too short ({} bytes): {}",
            resp.len(),
            hex::encode(&resp),
        ));
    }
    if resp[1] != local_id {
        return Err(format!(
            "ReadDataByLocalIdentifier echo mismatch: sent 0x{local_id:02X}, \
             got 0x{:02X} in response: {}",
            resp[1],
            hex::encode(&resp),
        ));
    }
    Ok(resp[2..].to_vec())
}

/// Write a data block by local identifier.
///
/// Requires an active security access (call [`security_access`] first).
pub fn write_data_by_local_id(
    slcan: &mut Slcan,
    local_id: u8,
    data: &[u8],
) -> Result<(), String> {
    let sid = KwpCommand::WriteDataByLocalIdentifier as u8;
    let mut payload = vec![sid, local_id];
    payload.extend_from_slice(data);
    let resp = isotp::transact(slcan, &payload)?;
    check_positive(&resp, sid)?;

    // Positive response: [0x7B, local_id]
    if resp.len() >= 2 && resp[1] != local_id {
        return Err(format!(
            "WriteDataByLocalIdentifier echo mismatch: sent 0x{local_id:02X}, \
             got 0x{:02X}",
            resp[1],
        ));
    }
    Ok(())
}

/// Perform the F430 NQS security access handshake.
///
/// See module-level documentation for the algorithm.  Must be called while a
/// programming session is active.
pub fn security_access(slcan: &mut Slcan) -> Result<(), String> {
    let sid = KwpCommand::SecurityAccess as u8;

    // Step 1: request seed.
    let seed_resp = isotp::transact(slcan, &[sid, 0x07])?;
    check_positive(&seed_resp, sid)?;

    // Positive response: [0x67, 0x07, seed_hi, seed_lo]
    if seed_resp.len() < 4 {
        return Err(format!(
            "security access seed response too short ({} bytes): {}",
            seed_resp.len(),
            hex::encode(&seed_resp),
        ));
    }
    if seed_resp[1] != 0x07 {
        return Err(format!(
            "security access seed sub-function mismatch: expected 0x07, \
             got 0x{:02X}",
            seed_resp[1],
        ));
    }

    let seed_hi = seed_resp[2];
    let seed_lo = seed_resp[3];

    // All-zero seed means the ECU is already unlocked.
    if seed_hi == 0x00 && seed_lo == 0x00 {
        return Ok(());
    }

    // Step 2: compute key.
    // valoreDaAggiungere = 3 for F430 base (ID_PROCEDURA_KEYs = 2).
    let key_lo = (seed_lo ^ 0xFF).wrapping_add(3);
    let key_hi = (seed_hi ^ 0xFF).wrapping_add(3);

    // Step 3: send key.
    let key_resp = isotp::transact(slcan, &[sid, 0x08, key_lo, key_hi])?;
    check_positive(&key_resp, sid)?;

    if key_resp.len() >= 2 && key_resp[1] != 0x08 {
        return Err(format!(
            "security access key sub-function mismatch: expected 0x08, \
             got 0x{:02X}",
            key_resp[1],
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Verify that `resp` is the expected positive response for `request_sid`.
pub(crate) fn check_positive(resp: &[u8], request_sid: u8) -> Result<(), String> {
    if resp.is_empty() {
        return Err(format!("empty response to service 0x{request_sid:02X}"));
    }
    if resp[0] == SID_NEGATIVE_RESPONSE {
        let nrc = resp.get(2).copied().unwrap_or(0xFF);
        return Err(format!(
            "negative response to 0x{request_sid:02X}: NRC 0x{nrc:02X} ({})",
            nrc_name(nrc),
        ));
    }
    let expected = request_sid | 0x40;
    if resp[0] != expected {
        return Err(format!(
            "unexpected response SID 0x{:02X} (expected 0x{expected:02X}): {}",
            resp[0],
            hex::encode(resp),
        ));
    }
    Ok(())
}

/// Return the name of a KWP2000 NRC using `automotive_diag`, falling back to
/// `"unknown"` for codes not in the standard enum.
pub(crate) fn nrc_name(nrc: u8) -> String {
    match KwpError::from_repr(nrc) {
        Some(e) => format!("{e:?}"),
        None    => "unknown".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_positive_ok() {
        assert!(check_positive(&[0x50, 0x81], 0x10).is_ok());
        assert!(check_positive(&[0x61, 0x24, 0x40, 0x81], 0x21).is_ok());
        assert!(check_positive(&[0x7B, 0x24], 0x3B).is_ok());
    }

    #[test]
    fn check_positive_empty() {
        let e = check_positive(&[], 0x10).unwrap_err();
        assert!(e.contains("empty"));
    }

    #[test]
    fn check_positive_negative_response() {
        let e = check_positive(&[0x7F, 0x10, 0x22], 0x10).unwrap_err();
        assert!(e.contains("0x22"));
        assert!(e.contains("ConditionsNotCorrectRequestSequenceError"));
    }

    #[test]
    fn check_positive_wrong_sid() {
        let e = check_positive(&[0x61, 0x22], 0x10).unwrap_err();
        assert!(e.contains("unexpected response SID 0x61"));
        assert!(e.contains("expected 0x50"));
    }

    #[test]
    fn nrc_name_known() {
        assert!(nrc_name(0x22).contains("ConditionsNotCorrect"));
        assert!(nrc_name(0x33).contains("SecurityAccessDenied"));
        assert!(nrc_name(0x35).contains("InvalidKey"));
        assert!(nrc_name(0x78).contains("RequestCorrectlyReceived"));
    }

    #[test]
    fn nrc_name_unknown() {
        assert_eq!(nrc_name(0xFF), "unknown");
        assert_eq!(nrc_name(0x00), "unknown");
    }

    #[test]
    fn kwp_command_sids_correct() {
        // Spot-check that automotive_diag matches our expected SID values.
        assert_eq!(KwpCommand::StartDiagnosticSession as u8, 0x10);
        assert_eq!(KwpCommand::ReadDataByLocalIdentifier as u8, 0x21);
        assert_eq!(KwpCommand::SecurityAccess as u8, 0x27);
        assert_eq!(KwpCommand::WriteDataByLocalIdentifier as u8, 0x3B);
        assert_eq!(KwpCommand::TesterPresent as u8, 0x3E);
    }

    #[test]
    fn kwp_session_types_correct() {
        // Normal session (0x81) is used for the initial StartDiagnosticSession.
        assert_eq!(KwpSessionType::Normal as u8, 0x81);
        // 0x83 is a Ferrari-specific session type not in the standard enum;
        // it is used as a raw literal in start_programming_session().
    }

    #[test]
    fn security_access_key_algorithm() {
        let seed_hi: u8 = 0xAB;
        let seed_lo: u8 = 0xCD;
        let key_lo = (seed_lo ^ 0xFF).wrapping_add(3);
        let key_hi = (seed_hi ^ 0xFF).wrapping_add(3);
        // 0xCD ^ 0xFF = 0x32; 0x32 + 3 = 0x35
        assert_eq!(key_lo, 0x35);
        // 0xAB ^ 0xFF = 0x54; 0x54 + 3 = 0x57
        assert_eq!(key_hi, 0x57);
    }

    #[test]
    fn security_access_key_wraps() {
        // 0xFF ^ 0xFF = 0x00; 0x00 + 3 = 0x03 (no wrap)
        assert_eq!((0xFFu8 ^ 0xFF).wrapping_add(3), 0x03);
        // 0x00 ^ 0xFF = 0xFF; 0xFF + 3 wraps to 0x02
        assert_eq!((0x00u8 ^ 0xFF).wrapping_add(3), 0x02);
    }
}
