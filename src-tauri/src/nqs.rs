//! Ferrari F430 NQS (instrument cluster) configuration blocks `$22` and `$24`.
//!
//! # Block layout
//!
//! | LOCAL_ID | Size   | Contents                          |
//! |----------|--------|-----------------------------------|
//! | `$22`    | 1 byte | Engine variant (EngineVar)        |
//! | `$24`    | 2 bytes | Vehicle options (Configurazione) |
//!
//! Both blocks are bit-packed.  Layout confirmed from `conn_ECU_RLI_FC`
//! conditions in `SDx.sqlite` (ECU 209 / 325).
//!
//! # `$24` bit layout
//!
//! **Byte 0** (`b{0}`):
//!
//! | Bit | Mask | Field                | 0 =          | 1 =          |
//! |-----|------|----------------------|--------------|--------------|
//! | 0   | 0x01 | Distance unit        | Kilometres   | Miles        |
//! | 1   | 0x02 | Temperature unit     | °C           | °F           |
//! | 2   | 0x04 | Gearbox type         | F1           | Manual       |
//! | 3   | 0x08 | Nation               | Europe       | USA          |
//! | 4   | 0x10 | Clock format         | 24 hours     | 12 hours     |
//! | 5   | 0x20 | TPMS                 | Disabled     | Enabled      |
//! | 6   | 0x40 | Oil temperature sensor | Jaeger     | Bosch        |
//! | 7   | 0x80 | Spider version       | No (Coupé)   | Yes (Spider) |
//!
//! **Byte 1** (`b{1}`):
//!
//! | Bit | Mask | Field                 | 0 =              | 1 =              |
//! |-----|------|-----------------------|------------------|------------------|
//! | 0   | 0x01 | Configuration flag    | OK (configured)  | Not configured   |
//! | 1   | 0x02 | Driver side           | Left-hand drive  | Right-hand drive |
//! | 2   | 0x04 | Fuel                  | Unleaded only    | No restriction   |
//! | 3   | 0x08 | Seat belt buzzer      | Disabled         | Enabled          |
//! | 4   | 0x10 | Arabian buzzer        | Disabled         | Enabled          |
//! | 5   | 0x20 | Door open + key buzzer | Disabled        | Enabled          |
//! | 6   | 0x40 | Brake type            | Steel            | Ceramic          |
//! | 7   | 0x80 | Oil pressure sensor   | Normal           | Kavlico          |

use std::fmt;

// ---------------------------------------------------------------------------
// NqsSnapshot — serializable snapshot for Tauri commands
// ---------------------------------------------------------------------------

/// A fully decoded, serializable snapshot of the NQS ECU state.
/// Returned by read_config and write_config Tauri commands.
#[derive(serde::Serialize, Clone, Debug)]
pub struct NqsSnapshot {
    // $22 EngineVar
    pub engine_variant: String,
    pub engine_variant_raw: u8,
    // $24 byte 0 — in bit order
    pub distance_unit: String,
    pub temp_unit: String,
    pub gearbox_type: String,
    pub nation: String,
    pub clock_format: String,
    pub tpms: String,
    pub oil_temp_sensor: String,
    pub spider_version: String,
    // $24 byte 1 — in bit order
    pub configuration_flag: String,
    pub driver_side: String,
    pub fuel: String,
    pub seat_belt_buzzer: String,
    pub arabian_buzzer: String,
    pub door_key_buzzer: String,
    pub brake_type: String,
    pub oil_pressure_sensor: String,
    // raw bytes
    pub byte0_raw: String,
    pub byte1_raw: String,
}

impl NqsSnapshot {
    pub fn from(c22: &Config22, c24: &Config24) -> Self {
        Self {
            engine_variant: c22.engine_variant().to_string(),
            engine_variant_raw: c22.as_byte(),
            distance_unit: c24.distance_unit().to_string(),
            temp_unit: c24.temp_unit().to_string(),
            gearbox_type: c24.gearbox_type().to_string(),
            nation: c24.nation().to_string(),
            clock_format: c24.clock_format().to_string(),
            tpms: c24.tpms().to_string(),
            oil_temp_sensor: c24.oil_temp_sensor().to_string(),
            spider_version: c24.spider_version().to_string(),
            configuration_flag: c24.configuration_flag().to_string(),
            driver_side: c24.driver_side().to_string(),
            fuel: c24.fuel().to_string(),
            seat_belt_buzzer: c24.seat_belt_buzzer().to_string(),
            arabian_buzzer: c24.arabian_buzzer().to_string(),
            door_key_buzzer: c24.door_key_buzzer().to_string(),
            brake_type: c24.brake_type().to_string(),
            oil_pressure_sensor: c24.oil_pressure_sensor().to_string(),
            byte0_raw: format!("{:02X}", c24.as_bytes()[0]),
            byte1_raw: format!("{:02X}", c24.as_bytes()[1]),
        }
    }
}

// ---------------------------------------------------------------------------
// Config24 — Configurazione ($24)
// ---------------------------------------------------------------------------

/// Decoded NQS configuration from the `$24` block (2 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config24 {
    raw: [u8; 2],
}

impl Config24 {
    /// Decode a `$24` response payload.
    ///
    /// Only the first two bytes are used; any additional bytes are ignored
    /// (the ECU may pad the response).
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 2 {
            return Err(format!(
                "$24 response too short: {} byte(s), expected at least 2",
                data.len(),
            ));
        }
        Ok(Self {
            raw: [data[0], data[1]],
        })
    }

    /// Return the raw bytes, suitable for writing back to the ECU.
    pub fn as_bytes(&self) -> &[u8; 2] {
        &self.raw
    }

    // --- Byte 0 accessors ---

    /// Distance unit: Kilometres (0) or Miles (1).
    pub fn distance_unit(&self) -> &'static str {
        if self.raw[0] & 0x01 != 0 {
            "Miles"
        } else {
            "Kilometres"
        }
    }

    /// Temperature unit: °C (0) or °F (1).
    pub fn temp_unit(&self) -> &'static str {
        if self.raw[0] & 0x02 != 0 {
            "°F"
        } else {
            "°C"
        }
    }

    /// Gearbox type: F1 (0) or Manual (1).
    pub fn gearbox_type(&self) -> &'static str {
        if self.raw[0] & 0x04 != 0 {
            "Manual"
        } else {
            "F1"
        }
    }

    /// Nation / market: Europe (0) or USA (1).
    pub fn nation(&self) -> &'static str {
        if self.raw[0] & 0x08 != 0 {
            "USA"
        } else {
            "Europe"
        }
    }

    /// Clock format: 24 hours (0) or 12 hours (1).
    pub fn clock_format(&self) -> &'static str {
        if self.raw[0] & 0x10 != 0 {
            "12 hours"
        } else {
            "24 hours"
        }
    }

    /// TPMS: Disabled (0) or Enabled (1).
    pub fn tpms(&self) -> &'static str {
        if self.raw[0] & 0x20 != 0 {
            "Enabled"
        } else {
            "Disabled"
        }
    }

    /// Oil temperature sensor type: Jaeger (0) or Bosch (1).
    pub fn oil_temp_sensor(&self) -> &'static str {
        if self.raw[0] & 0x40 != 0 {
            "Bosch"
        } else {
            "Jaeger"
        }
    }

    /// Spider variant: No / Coupé (0) or Yes / Spider (1).
    pub fn spider_version(&self) -> &'static str {
        if self.raw[0] & 0x80 != 0 {
            "Yes (Spider)"
        } else {
            "No (Coupé)"
        }
    }

    // --- Byte 1 accessors ---

    /// Configuration flag: OK configured (0) or Not configured (1).
    ///
    /// Bit 0 of byte 1.  Set to 1 when the ECU EEPROM has not been
    /// initialised with valid data.  Should be cleared (0) after writing
    /// correct configuration values.
    pub fn configuration_flag(&self) -> &'static str {
        if self.raw[1] & 0x01 != 0 {
            "Not configured"
        } else {
            "OK (configured)"
        }
    }

    /// Driver side: Left-hand drive (0) or Right-hand drive (1).
    pub fn driver_side(&self) -> &'static str {
        if self.raw[1] & 0x02 != 0 {
            "Right-hand drive"
        } else {
            "Left-hand drive"
        }
    }

    /// Fuel: Unleaded only (0) or No restriction (1).
    pub fn fuel(&self) -> &'static str {
        if self.raw[1] & 0x04 != 0 {
            "No restriction"
        } else {
            "Unleaded only"
        }
    }

    /// Seat belt buzzer: Disabled (0) or Enabled (1).
    pub fn seat_belt_buzzer(&self) -> &'static str {
        if self.raw[1] & 0x08 != 0 {
            "Enabled"
        } else {
            "Disabled"
        }
    }

    /// Arabian market buzzer: Disabled (0) or Enabled (1).
    pub fn arabian_buzzer(&self) -> &'static str {
        if self.raw[1] & 0x10 != 0 {
            "Enabled"
        } else {
            "Disabled"
        }
    }

    /// Door open + key-in buzzer: Disabled (0) or Enabled (1).
    pub fn door_key_buzzer(&self) -> &'static str {
        if self.raw[1] & 0x20 != 0 {
            "Enabled"
        } else {
            "Disabled"
        }
    }

    /// Brake disc type: Steel (0) or Ceramic (1).
    pub fn brake_type(&self) -> &'static str {
        if self.raw[1] & 0x40 != 0 {
            "Ceramic"
        } else {
            "Steel"
        }
    }

    /// Oil pressure sensor type: Normal (0) or Kavlico (1).
    pub fn oil_pressure_sensor(&self) -> &'static str {
        if self.raw[1] & 0x80 != 0 {
            "Kavlico"
        } else {
            "Normal"
        }
    }

    // -------------------------------------------------------------------------
    // Write support
    // -------------------------------------------------------------------------

    /// Apply a single field assignment to the in-memory configuration.
    ///
    /// `field` is case-insensitive.  Returns an error if the field name or
    /// value is not recognised.
    ///
    /// # Valid fields and values
    ///
    /// | Field              | Values                        |
    /// |--------------------|-------------------------------|
    /// | `nation`           | `europe` \| `usa`             |
    /// | `distance_unit`    | `km` \| `miles`               |
    /// | `temp_unit`        | `c` \| `f`                    |
    /// | `clock`            | `24h` \| `12h`                |
    /// | `gearbox`          | `f1` \| `manual`              |
    /// | `tpms`             | `enabled` \| `disabled`       |
    /// | `oil_temp_sensor`  | `jaeger` \| `bosch`           |
    /// | `spider`           | `yes` \| `no`                 |
    /// | `configuration_flag` | `ok` \| `not_configured`   |
    /// | `driver_side`      | `lhd` \| `rhd`               |
    /// | `fuel`             | `unleaded` \| `unrestricted`  |
    /// | `seat_belt_buzzer` | `enabled` \| `disabled`       |
    /// | `arabian_buzzer`   | `enabled` \| `disabled`       |
    /// | `door_key_buzzer`  | `enabled` \| `disabled`       |
    /// | `brake_type`       | `steel` \| `ceramic`          |
    /// | `oil_pressure`     | `normal` \| `kavlico`         |
    pub fn set_field(&mut self, field: &str, value: &str) -> Result<(), String> {
        let f = field.to_lowercase();
        let v = value.to_lowercase();

        /// Set or clear a bit in `byte` based on `flag`.
        #[inline(always)]
        fn set_bit(byte: &mut u8, mask: u8, flag: bool) {
            if flag {
                *byte |= mask;
            } else {
                *byte &= !mask;
            }
        }

        /// Parse a two-option value, returning `true` for `on` and `false` for `off`.
        fn parse_bool(v: &str, on: &str, off: &str, field: &str) -> Result<bool, String> {
            if v == on {
                Ok(true)
            } else if v == off {
                Ok(false)
            } else {
                Err(format!(
                    "invalid value '{v}' for field '{field}': expected '{on}' or '{off}'"
                ))
            }
        }

        match f.as_str() {
            // Byte 0
            "distance_unit" => set_bit(&mut self.raw[0], 0x01, parse_bool(&v, "miles", "km", &f)?),
            "temp_unit" => set_bit(&mut self.raw[0], 0x02, parse_bool(&v, "f", "c", &f)?),
            "gearbox" => set_bit(&mut self.raw[0], 0x04, parse_bool(&v, "manual", "f1", &f)?),
            "nation" => set_bit(&mut self.raw[0], 0x08, parse_bool(&v, "usa", "europe", &f)?),
            "clock" => set_bit(&mut self.raw[0], 0x10, parse_bool(&v, "12h", "24h", &f)?),
            "tpms" => set_bit(
                &mut self.raw[0],
                0x20,
                parse_bool(&v, "enabled", "disabled", &f)?,
            ),
            "oil_temp_sensor" => set_bit(
                &mut self.raw[0],
                0x40,
                parse_bool(&v, "bosch", "jaeger", &f)?,
            ),
            "spider" => set_bit(&mut self.raw[0], 0x80, parse_bool(&v, "yes", "no", &f)?),
            // Byte 1
            // NOTE: configuration_flag bit=1 means NOT configured (inverted sense).
            "configuration_flag" => set_bit(
                &mut self.raw[1],
                0x01,
                parse_bool(&v, "not_configured", "ok", &f)?,
            ),
            "driver_side" => set_bit(&mut self.raw[1], 0x02, parse_bool(&v, "rhd", "lhd", &f)?),
            "fuel" => set_bit(
                &mut self.raw[1],
                0x04,
                parse_bool(&v, "unrestricted", "unleaded", &f)?,
            ),
            "seat_belt_buzzer" => set_bit(
                &mut self.raw[1],
                0x08,
                parse_bool(&v, "enabled", "disabled", &f)?,
            ),
            "arabian_buzzer" => set_bit(
                &mut self.raw[1],
                0x10,
                parse_bool(&v, "enabled", "disabled", &f)?,
            ),
            "door_key_buzzer" => set_bit(
                &mut self.raw[1],
                0x20,
                parse_bool(&v, "enabled", "disabled", &f)?,
            ),
            "brake_type" => set_bit(
                &mut self.raw[1],
                0x40,
                parse_bool(&v, "ceramic", "steel", &f)?,
            ),
            "oil_pressure" => set_bit(
                &mut self.raw[1],
                0x80,
                parse_bool(&v, "kavlico", "normal", &f)?,
            ),

            _ => {
                return Err(format!(
                    "unknown field '{field}'. Valid fields: nation, distance_unit, \
                 temp_unit, clock, gearbox, tpms, oil_temp_sensor, spider, \
                 configuration_flag, driver_side, fuel, seat_belt_buzzer, \
                 arabian_buzzer, door_key_buzzer, brake_type, oil_pressure"
                ))
            }
        }
        Ok(())
    }
}

impl fmt::Display for Config24 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  Byte 0  (raw: {:02X})", self.raw[0])?;
        writeln!(f, "    Distance unit          : {}", self.distance_unit())?;
        writeln!(f, "    Temperature unit       : {}", self.temp_unit())?;
        writeln!(f, "    Gearbox type           : {}", self.gearbox_type())?;
        writeln!(f, "    Nation                 : {}", self.nation())?;
        writeln!(f, "    Clock format           : {}", self.clock_format())?;
        writeln!(f, "    TPMS                   : {}", self.tpms())?;
        writeln!(f, "    Oil temperature sensor : {}", self.oil_temp_sensor())?;
        writeln!(f, "    Spider version         : {}", self.spider_version())?;
        writeln!(f)?;
        writeln!(f, "  Byte 1  (raw: {:02X})", self.raw[1])?;
        writeln!(
            f,
            "    Configuration flag     : {}",
            self.configuration_flag()
        )?;
        writeln!(f, "    Driver side            : {}", self.driver_side())?;
        writeln!(f, "    Fuel                   : {}", self.fuel())?;
        writeln!(
            f,
            "    Seat belt buzzer       : {}",
            self.seat_belt_buzzer()
        )?;
        writeln!(f, "    Arabian buzzer         : {}", self.arabian_buzzer())?;
        writeln!(f, "    Door open + key buzzer : {}", self.door_key_buzzer())?;
        writeln!(f, "    Brake type             : {}", self.brake_type())?;
        write!(
            f,
            "    Oil pressure sensor    : {}",
            self.oil_pressure_sensor()
        )
    }
}

// ---------------------------------------------------------------------------
// Config22 — EngineVar ($22)
// ---------------------------------------------------------------------------

/// Decoded NQS engine variant from the `$22` block (1 byte).
///
/// Writable via [`Config22::set_variant`]. The EngineVar is tied to the
/// gearbox/flywheel configuration in `$24` — [`consistency_warning`] checks
/// this pairing and surfaces a warning (which the caller may choose to
/// bypass) when they don't match the expected combination.
///
/// Note this tool does **not** coordinate the write with the `$2E` hardware
/// configuration block the way the full Proxi Programming IOLI (IOLI 83)
/// does, which writes `$22`, `$24`, and `$2E` atomically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config22 {
    raw: u8,
}

impl Config22 {
    /// Decode a `$22` response payload.
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        data.first()
            .copied()
            .map(|raw| Self { raw })
            .ok_or_else(|| "$22 response is empty".to_string())
    }

    /// Return the raw byte.
    pub fn as_byte(&self) -> u8 {
        self.raw
    }

    /// Set the engine variant by raw byte value.
    ///
    /// Valid values per the SDX database (`conn_ECU_RLI_FC` for ECU 209/325 `$22`):
    ///   `0x03` — 4.3L V8 1x Flywheel
    ///   `0x04` — 4.3L V8 4x Flywheel
    pub fn set_variant(&mut self, raw: u8) -> Result<(), String> {
        match raw {
            0x03 | 0x04 => {
                self.raw = raw;
                Ok(())
            }
            _ => Err(format!(
                "invalid engine variant 0x{raw:02X}: valid values are 0x03 (1x flywheel) \
                 and 0x04 (4x flywheel)"
            )),
        }
    }

    /// Human-readable engine variant description.
    ///
    /// Values are specific to the F430 (4.3L V8).  Other Ferrari models may
    /// use different codes.
    pub fn engine_variant(&self) -> &'static str {
        match self.raw {
            0x00 => "Not defined",
            0x03 => "4.3L V8, 1× flywheel",
            0x04 => "4.3L V8, 4× flywheel",
            0x05 => "4.3L V8 Scuderia variant",
            _ => "Unknown",
        }
    }
}

impl fmt::Display for Config22 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "  Engine variant : {}", self.engine_variant())
    }
}

// ---------------------------------------------------------------------------
// Consistency validation
// ---------------------------------------------------------------------------

/// Check whether the engine variant (`$22`) and gearbox type (`$24` bit 2) are
/// consistent with the expected SDX pairings.
///
/// Returns `Some(warning)` when the combination is unusual:
/// - 1× flywheel (`0x03`) paired with F1 gearbox (`$24` bit 2 = 0)
/// - 4× flywheel (`0x04`) paired with manual gearbox (`$24` bit 2 = 1)
///
/// Returns `None` when the combination is expected, or when the engine variant
/// has no defined pairing (0x00, 0x05, or unknown).
///
/// A `Some` result does not prevent writing — the caller decides whether to
/// proceed (e.g. for a manual-swapped car that retains the original flywheel).
pub fn consistency_warning(c22: &Config22, c24: &Config24) -> Option<&'static str> {
    let gearbox_is_manual = c24.as_bytes()[0] & 0x04 != 0;
    match c22.as_byte() {
        0x03 if !gearbox_is_manual => Some(
            "1× flywheel is typically paired with a manual gearbox, \
             but gearbox is set to F1.",
        ),
        0x04 if gearbox_is_manual => Some(
            "4× flywheel is typically paired with an F1 gearbox, \
             but gearbox is set to Manual.",
        ),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Config24::from_bytes ---

    #[test]
    fn config24_from_bytes_ok() {
        let c = Config24::from_bytes(&[0x40, 0x81]).unwrap();
        assert_eq!(c.as_bytes(), &[0x40, 0x81]);
    }

    #[test]
    fn config24_from_bytes_extra_bytes_ignored() {
        let c = Config24::from_bytes(&[0x40, 0x81, 0xFF, 0xFF]).unwrap();
        assert_eq!(c.as_bytes(), &[0x40, 0x81]);
    }

    #[test]
    fn config24_from_bytes_too_short() {
        assert!(Config24::from_bytes(&[]).is_err());
        assert!(Config24::from_bytes(&[0x40]).is_err());
    }

    // --- Byte 0 accessors ---

    #[test]
    fn config24_byte0_all_zero() {
        let c = Config24::from_bytes(&[0x00, 0x00]).unwrap();
        assert_eq!(c.distance_unit(), "Kilometres");
        assert_eq!(c.temp_unit(), "°C");
        assert_eq!(c.gearbox_type(), "F1");
        assert_eq!(c.nation(), "Europe");
        assert_eq!(c.clock_format(), "24 hours");
        assert_eq!(c.tpms(), "Disabled");
        assert_eq!(c.oil_temp_sensor(), "Jaeger");
        assert_eq!(c.spider_version(), "No (Coupé)");
    }

    #[test]
    fn config24_byte0_all_set() {
        let c = Config24::from_bytes(&[0xFF, 0x00]).unwrap();
        assert_eq!(c.distance_unit(), "Miles");
        assert_eq!(c.temp_unit(), "°F");
        assert_eq!(c.gearbox_type(), "Manual");
        assert_eq!(c.nation(), "USA");
        assert_eq!(c.clock_format(), "12 hours");
        assert_eq!(c.tpms(), "Enabled");
        assert_eq!(c.oil_temp_sensor(), "Bosch");
        assert_eq!(c.spider_version(), "Yes (Spider)");
    }

    // --- Byte 1 accessors ---

    #[test]
    fn config24_byte1_all_zero() {
        let c = Config24::from_bytes(&[0x00, 0x00]).unwrap();
        assert_eq!(c.configuration_flag(), "OK (configured)");
        assert_eq!(c.driver_side(), "Left-hand drive");
        assert_eq!(c.fuel(), "Unleaded only");
        assert_eq!(c.seat_belt_buzzer(), "Disabled");
        assert_eq!(c.arabian_buzzer(), "Disabled");
        assert_eq!(c.door_key_buzzer(), "Disabled");
        assert_eq!(c.brake_type(), "Steel");
        assert_eq!(c.oil_pressure_sensor(), "Normal");
    }

    #[test]
    fn config24_byte1_all_set() {
        let c = Config24::from_bytes(&[0x00, 0xFF]).unwrap();
        assert_eq!(c.configuration_flag(), "Not configured");
        assert_eq!(c.driver_side(), "Right-hand drive");
        assert_eq!(c.fuel(), "No restriction");
        assert_eq!(c.seat_belt_buzzer(), "Enabled");
        assert_eq!(c.arabian_buzzer(), "Enabled");
        assert_eq!(c.door_key_buzzer(), "Enabled");
        assert_eq!(c.brake_type(), "Ceramic");
        assert_eq!(c.oil_pressure_sensor(), "Kavlico");
    }

    // --- set_field round-trips ---

    #[test]
    fn set_field_nation_roundtrip() {
        let mut c = Config24::from_bytes(&[0x00, 0x00]).unwrap();
        c.set_field("nation", "usa").unwrap();
        assert_eq!(c.nation(), "USA");
        assert_eq!(c.raw[0] & 0x08, 0x08);
        c.set_field("nation", "europe").unwrap();
        assert_eq!(c.nation(), "Europe");
        assert_eq!(c.raw[0] & 0x08, 0x00);
    }

    #[test]
    fn set_field_preserves_other_bits() {
        // Start with all bits set.
        let mut c = Config24::from_bytes(&[0xFF, 0xFF]).unwrap();
        c.set_field("nation", "europe").unwrap();
        // Only bit 3 of byte 0 should be cleared.
        assert_eq!(c.raw[0], 0xFF & !0x08);
        assert_eq!(c.raw[1], 0xFF); // byte 1 unchanged
    }

    #[test]
    fn set_field_case_insensitive() {
        let mut c = Config24::from_bytes(&[0x00, 0x00]).unwrap();
        c.set_field("Nation", "USA").unwrap();
        assert_eq!(c.nation(), "USA");
    }

    #[test]
    fn set_field_configuration_flag_ok() {
        // Bit 0 of byte 1: 1 = not configured, 0 = ok.
        let mut c = Config24::from_bytes(&[0x00, 0x01]).unwrap();
        assert_eq!(c.configuration_flag(), "Not configured");
        c.set_field("configuration_flag", "ok").unwrap();
        assert_eq!(c.configuration_flag(), "OK (configured)");
        assert_eq!(c.raw[1] & 0x01, 0x00);
    }

    #[test]
    fn set_field_unknown_field() {
        let mut c = Config24::from_bytes(&[0x00, 0x00]).unwrap();
        let e = c.set_field("horsepower", "800").unwrap_err();
        assert!(e.contains("unknown field"));
    }

    #[test]
    fn set_field_invalid_value() {
        let mut c = Config24::from_bytes(&[0x00, 0x00]).unwrap();
        let e = c.set_field("nation", "france").unwrap_err();
        assert!(e.contains("invalid value"));
        assert!(e.contains("france"));
    }

    #[test]
    fn set_field_all_fields() {
        // Ensure every documented field can be set without error.
        let fields: &[(&str, &str)] = &[
            ("nation", "usa"),
            ("distance_unit", "miles"),
            ("temp_unit", "f"),
            ("clock", "12h"),
            ("gearbox", "manual"),
            ("tpms", "enabled"),
            ("oil_temp_sensor", "bosch"),
            ("spider", "yes"),
            ("configuration_flag", "not_configured"),
            ("driver_side", "rhd"),
            ("fuel", "unrestricted"),
            ("seat_belt_buzzer", "enabled"),
            ("arabian_buzzer", "enabled"),
            ("door_key_buzzer", "enabled"),
            ("brake_type", "ceramic"),
            ("oil_pressure", "kavlico"),
        ];
        let mut c = Config24::from_bytes(&[0x00, 0x00]).unwrap();
        for (field, value) in fields {
            c.set_field(field, value)
                .unwrap_or_else(|e| panic!("{field}={value}: {e}"));
        }
        // All bits should now be set.
        assert_eq!(c.raw, [0xFF, 0xFF]);
    }

    // --- Config22 ---

    #[test]
    fn config22_from_bytes_ok() {
        let c = Config22::from_bytes(&[0x04]).unwrap();
        assert_eq!(c.as_byte(), 0x04);
        assert_eq!(c.engine_variant(), "4.3L V8, 4× flywheel");
    }

    #[test]
    fn config22_from_bytes_empty() {
        assert!(Config22::from_bytes(&[]).is_err());
    }

    #[test]
    fn config22_variants() {
        assert_eq!(
            Config22::from_bytes(&[0x00]).unwrap().engine_variant(),
            "Not defined"
        );
        assert_eq!(
            Config22::from_bytes(&[0x03]).unwrap().engine_variant(),
            "4.3L V8, 1× flywheel"
        );
        assert_eq!(
            Config22::from_bytes(&[0x04]).unwrap().engine_variant(),
            "4.3L V8, 4× flywheel"
        );
        assert_eq!(
            Config22::from_bytes(&[0x05]).unwrap().engine_variant(),
            "4.3L V8 Scuderia variant"
        );
        assert_eq!(
            Config22::from_bytes(&[0xFF]).unwrap().engine_variant(),
            "Unknown"
        );
    }
}
