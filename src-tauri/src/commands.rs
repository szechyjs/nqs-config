use std::collections::HashMap;
use serialport::SerialPortType;

use crate::{kwp, nqs, slcan};

fn is_canable_usb(info: &serialport::UsbPortInfo) -> bool {
    // Official Openlight Labs / CANable VID is 0x1D50
    if info.vid == 0x1D50 {
        return true;
    }

    let matches_str = |opt: &Option<String>| {
        if let Some(s) = opt {
            let lower = s.to_lowercase();
            lower.contains("canable")
                || lower.contains("slcan")
                || lower.contains("openlight")
                || lower.contains("candlelight")
        } else {
            false
        }
    };

    matches_str(&info.product) || matches_str(&info.manufacturer)
}

#[tauri::command]
pub fn list_ports() -> Vec<String> {
    let all_usb_ports: Vec<_> = serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .filter(|p| {
            if !matches!(p.port_type, SerialPortType::UsbPort(_)) {
                return false;
            }
            #[cfg(target_os = "macos")]
            {
                if p.port_name.contains("tty.") {
                    return false;
                }
            }
            true
        })
        .collect();

    // Prefer ports that explicitly match CANable USB descriptors
    let canable_ports: Vec<String> = all_usb_ports
        .iter()
        .filter(|p| {
            if let SerialPortType::UsbPort(info) = &p.port_type {
                is_canable_usb(info)
            } else {
                false
            }
        })
        .map(|p| p.port_name.clone())
        .collect();

    if !canable_ports.is_empty() {
        canable_ports
    } else {
        // Fall back to all USB serial ports if no explicit descriptor matched
        all_usb_ports.into_iter().map(|p| p.port_name).collect()
    }
}

#[tauri::command]
pub fn read_config(port: String) -> Result<nqs::NqsSnapshot, String> {
    let mut can = slcan::Slcan::open(&port, 2)?;

    kwp::start_session(&mut can)?;

    let data22 = kwp::read_data_by_local_id(&mut can, 0x22)?;
    let data24 = kwp::read_data_by_local_id(&mut can, 0x24)?;

    let c22 = nqs::Config22::from_bytes(&data22)?;
    let c24 = nqs::Config24::from_bytes(&data24)?;

    Ok(nqs::NqsSnapshot::from(&c22, &c24))
}

#[tauri::command]
pub async fn write_config(
    port: String,
    changes: HashMap<String, String>,
    force: bool,
) -> Result<nqs::NqsSnapshot, String> {
    let mut can = slcan::Slcan::open(&port, 2)?;

    kwp::start_session(&mut can)?;
    kwp::start_programming_session(&mut can)?;

    // --- Read current state ---
    let data22 = kwp::read_data_by_local_id(&mut can, 0x22)?;
    let data24 = kwp::read_data_by_local_id(&mut can, 0x24)?;
    let mut cfg22 = nqs::Config22::from_bytes(&data22)?;
    let mut cfg24 = nqs::Config24::from_bytes(&data24)?;

    // --- Apply $22 change if present ---
    let write_22 = if let Some(raw_str) = changes.get("engine_variant") {
        let raw: u8 = raw_str.parse().map_err(|_| {
            format!("invalid engine_variant value '{raw_str}': expected a decimal byte (3 or 4)")
        })?;
        cfg22.set_variant(raw)?;
        true
    } else {
        false
    };

    // --- Apply $24 changes ---
    let write_24 = !changes.keys()
        .filter(|k| k.as_str() != "engine_variant")
        .collect::<Vec<_>>()
        .is_empty();

    for (field, value) in &changes {
        if field == "engine_variant" {
            continue;
        }
        cfg24.set_field(field, value)?;
    }

    // --- Consistency check ---
    if !force {
        if let Some(warning) = nqs::consistency_warning(&cfg22, &cfg24) {
            return Err(format!("CONSISTENCY_WARNING:{warning}"));
        }
    }

    // --- Write to ECU (security access required for both $22 and $24) ---
    if write_22 || write_24 {
        kwp::security_access(&mut can)?;

        // SDX write order: $22 (EngineVar) before $24 (Configurazione).
        if write_22 {
            kwp::write_data_by_local_id(&mut can, 0x22, &[cfg22.as_byte()])?;
        }
        if write_24 {
            kwp::write_data_by_local_id(&mut can, 0x24, cfg24.as_bytes())?;
        }
    }

    // --- Re-read both blocks and return verified snapshot ---
    let verify22 = kwp::read_data_by_local_id(&mut can, 0x22)?;
    let verify24 = kwp::read_data_by_local_id(&mut can, 0x24)?;

    let c22 = nqs::Config22::from_bytes(&verify22)?;
    let c24 = nqs::Config24::from_bytes(&verify24)?;

    Ok(nqs::NqsSnapshot::from(&c22, &c24))
}
