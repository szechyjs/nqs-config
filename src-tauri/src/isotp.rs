//! ISO-TP (ISO 15765-2) framing for the NQS ECU with KWP2000 physical addressing.
//!
//! # KWP2000 physical addressing on CAN
//!
//! The F430 NQS uses KWP2000 physical addressing embedded in CAN frames.
//! Confirmed by disassembly of the SDX VCI firmware (`sdxCanWrite` in
//! `swBaseSdx.asm`): the ECU address byte precedes the ISO-TP PCI byte in every
//! CAN frame payload.
//!
//! ```text
//! Tester → ECU (CAN ID 0x7B0):  [0x85, PCI, SID, data..., 0x00 padding]
//! ECU → Tester (CAN ID 0x7C3):  [0xF1, PCI, SID, data..., 0x00 padding]
//! ```
//!
//! This module handles the address byte transparently: it is prepended on TX and
//! stripped on RX.  Callers work with plain KWP2000 payloads (`[SID, data...]`).
//!
//! # Frame layout with addressing
//!
//! | Frame type         | CAN data bytes                                        |
//! |--------------------|-------------------------------------------------------|
//! | Single Frame (SF)  | `[addr, 0x0N, d0..d5]`  (N = payload length, ≤ 6)   |
//! | First Frame (FF)   | `[addr, 0x1n, len_lo, d0..d4]`  (n = len high nibble) |
//! | Consecutive Frame  | `[addr, 0x2n, d0..d5]`  (n = sequence counter)        |
//! | Flow Control (FC)  | `[addr, 0x30, 0, 0]`                                  |

use std::time::Duration;
use crate::slcan::{CanFrame, Slcan};

/// CAN ID used by the tester to send requests to the NQS ECU.
pub const TX_ID: u16 = 0x7B0;
/// CAN ID on which the NQS ECU sends its responses.
pub const RX_ID: u16 = 0x7C3;

/// KWP2000 physical address of the NQS ECU.
const ECU_ADDR: u8 = 0x85;
/// KWP2000 physical address of the tester (standard value 0xF1).
const TESTER_ADDR: u8 = 0xF1;

/// Maximum time to wait for the first response frame from the ECU.
///
/// KWP2000 P2 max is 50 ms; 1 000 ms allows for slow ECU startup and any
/// `0x78` response-pending retries without hitting the outer timeout.
const RESPONSE_TIMEOUT: Duration = Duration::from_millis(1_000);

/// Maximum time to wait for each consecutive frame after sending flow control.
const CF_TIMEOUT: Duration = Duration::from_millis(500);

/// Maximum number of `0x78` (responsePending) retries before giving up.
const MAX_PENDING_RETRIES: usize = 10;

/// Send a KWP2000 payload to the ECU and return the response payload.
///
/// The ECU address byte is prepended to the payload before ISO-TP framing and
/// stripped from the response before returning.  Callers provide and receive
/// plain KWP2000 payloads (`[SID, data...]`).
///
/// Handles single-frame and multi-frame exchanges transparently, including
/// `0x78` (requestCorrectlyReceivedResponsePending) retries.
pub fn transact(slcan: &mut Slcan, payload: &[u8]) -> Result<Vec<u8>, String> {
    send(slcan, payload)?;
    receive(slcan)
}

// ---------------------------------------------------------------------------
// Transmit
// ---------------------------------------------------------------------------

fn send(slcan: &mut Slcan, payload: &[u8]) -> Result<(), String> {
    // With the address byte occupying byte 0 and the PCI occupying byte 1,
    // a single frame holds at most 6 payload bytes (8 - 1 addr - 1 PCI = 6).
    if payload.len() <= 6 {
        send_single_frame(slcan, payload)
    } else {
        send_multi_frame(slcan, payload)
    }
}

/// Build and transmit a single-frame: `[ECU_ADDR, 0x0N, payload..., 0x00...]`.
fn send_single_frame(slcan: &mut Slcan, payload: &[u8]) -> Result<(), String> {
    let mut data = vec![0u8; 8];
    data[0] = ECU_ADDR;
    data[1] = payload.len() as u8; // PCI: SF, length
    data[2..2 + payload.len()].copy_from_slice(payload);
    slcan.transmit(&CanFrame { id: TX_ID, data })
}

/// Segment and transmit a multi-frame message.
///
/// FF carries 5 payload bytes; each CF carries 6 payload bytes.
fn send_multi_frame(slcan: &mut Slcan, payload: &[u8]) -> Result<(), String> {
    let total = payload.len();
    if total > 0xFFF {
        return Err(format!("ISO-TP payload too large: {total} bytes (max 4095)"));
    }

    // First Frame: [ECU_ADDR, 0x1n, len_lo, d0..d4]
    let mut ff = vec![0u8; 8];
    ff[0] = ECU_ADDR;
    ff[1] = 0x10 | ((total >> 8) as u8 & 0x0F);
    ff[2] = (total & 0xFF) as u8;
    ff[3..8].copy_from_slice(&payload[0..5]);
    slcan.transmit(&CanFrame { id: TX_ID, data: ff })?;

    // Wait for Flow Control from ECU.
    let fc = slcan.read_until(|f| f.id == RX_ID, RESPONSE_TIMEOUT)?;
    let fc_pci_offset = addr_offset(&fc.data);
    if fc.data.len() <= fc_pci_offset
        || (fc.data[fc_pci_offset] & 0xF0) != 0x30
    {
        return Err(format!(
            "expected Flow Control (0x3x), got: {}",
            hex::encode(&fc.data),
        ));
    }

    // Consecutive Frames: [ECU_ADDR, 0x2n, d0..d5]
    let mut sn: u8 = 1;
    let mut offset = 5usize;
    while offset < total {
        let chunk_end = (offset + 6).min(total);
        let chunk = &payload[offset..chunk_end];
        let mut cf = vec![0u8; 8];
        cf[0] = ECU_ADDR;
        cf[1] = 0x20 | (sn & 0x0F);
        cf[2..2 + chunk.len()].copy_from_slice(chunk);
        slcan.transmit(&CanFrame { id: TX_ID, data: cf })?;
        sn = sn.wrapping_add(1);
        offset += 6;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Receive
// ---------------------------------------------------------------------------

/// Receive and reassemble an ISO-TP response, handling `0x78` pending retries.
///
/// The tester address byte is stripped from every received frame before the
/// PCI byte is examined.
fn receive(slcan: &mut Slcan) -> Result<Vec<u8>, String> {
    for attempt in 0..=MAX_PENDING_RETRIES {
        let frame = slcan.read_until(|f| f.id == RX_ID, RESPONSE_TIMEOUT)?;
        let result = decode_response(slcan, &frame)?;

        // Check for 0x78 NRC (requestCorrectlyReceivedResponsePending).
        // The ECU is still processing; wait for the real response.
        if is_response_pending(&result) {
            if attempt == MAX_PENDING_RETRIES {
                return Err(format!(
                    "ECU sent 0x78 responsePending {MAX_PENDING_RETRIES} times; giving up"
                ));
            }
            continue; // re-enter loop, read next frame
        }

        return Ok(result);
    }
    unreachable!()
}

/// Decode a single received CAN frame (SF or start of FF) into the full payload.
fn decode_response(slcan: &mut Slcan, frame: &CanFrame) -> Result<Vec<u8>, String> {
    let offset = addr_offset(&frame.data);
    let data = frame
        .data
        .get(offset..)
        .filter(|d| !d.is_empty())
        .ok_or_else(|| format!("response frame too short: {}", hex::encode(&frame.data)))?;

    let pci_type = (data[0] & 0xF0) >> 4;

    match pci_type {
        0x0 => {
            // Single Frame: [PCI=0x0N, d0..d(N-1)]
            let len = (data[0] & 0x0F) as usize;
            if len == 0 || data.len() < 1 + len {
                return Err(format!(
                    "invalid SF length 0x{:02X} in frame: {}",
                    data[0],
                    hex::encode(&frame.data),
                ));
            }
            Ok(data[1..1 + len].to_vec())
        }

        0x1 => {
            // First Frame: [0x1n, len_lo, d0..d4]
            if data.len() < 3 {
                return Err(format!("FF too short: {}", hex::encode(&frame.data)));
            }
            let total = (((data[0] & 0x0F) as usize) << 8) | data[1] as usize;
            let mut payload = Vec::with_capacity(total);
            payload.extend_from_slice(&data[2..]);

            // Send Flow Control: ContinueToSend, no block limit, no STmin.
            let fc = CanFrame {
                id: TX_ID,
                data: vec![ECU_ADDR, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            };
            slcan.transmit(&fc)?;

            // Collect consecutive frames.
            let mut expected_sn: u8 = 1;
            while payload.len() < total {
                let cf = slcan.read_until(|f| f.id == RX_ID, CF_TIMEOUT)?;
                let cf_offset = addr_offset(&cf.data);
                let cf_data = cf
                    .data
                    .get(cf_offset..)
                    .filter(|d| !d.is_empty())
                    .ok_or_else(|| format!("CF too short: {}", hex::encode(&cf.data)))?;

                if (cf_data[0] & 0xF0) != 0x20 {
                    return Err(format!(
                        "expected consecutive frame (0x2x), got: {}",
                        hex::encode(&cf.data),
                    ));
                }

                let sn = cf_data[0] & 0x0F;
                if sn != (expected_sn & 0x0F) {
                    return Err(format!(
                        "CF sequence error: expected SN {}, got {}",
                        expected_sn & 0x0F,
                        sn,
                    ));
                }

                let remaining = total - payload.len();
                let chunk_len = remaining.min(cf_data.len() - 1);
                payload.extend_from_slice(&cf_data[1..1 + chunk_len]);
                expected_sn = expected_sn.wrapping_add(1);
            }

            Ok(payload)
        }

        _ => Err(format!(
            "unexpected ISO-TP PCI type 0x{pci_type:X} in response: {}",
            hex::encode(&frame.data),
        )),
    }
}

/// Returns the index of the PCI byte in a received CAN frame's data, skipping
/// the tester address byte (0xF1) if present.
///
/// Using a function rather than an inline conditional ensures the stripping
/// logic is applied consistently across SF, FF, and CF paths.
#[inline]
fn addr_offset(data: &[u8]) -> usize {
    if data.first() == Some(&TESTER_ADDR) { 1 } else { 0 }
}

/// Returns `true` if `payload` is a KWP2000 negative response with NRC `0x78`
/// (requestCorrectlyReceivedResponsePending).
#[inline]
fn is_response_pending(payload: &[u8]) -> bool {
    payload.len() >= 3 && payload[0] == 0x7F && payload[2] == 0x78
}
