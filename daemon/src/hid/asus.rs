use anyhow::{bail, Result};
use std::ffi::CString;

use super::{BatteryReading, BatteryStatus, MouseDevice};

/// All known ASUS wireless mouse (vendor_id, product_id) pairs.
///
/// 0x1C57 — TUF Gaming Mini WL Mouse MIKU, wireless via USB dongle
/// 0x1C56 — same mouse connected via USB cable (wired/charging mode)
pub const ASUS_KNOWN_IDS: &[(u16, u16)] = &[
    (0x0B05, 0x1C57),
    (0x0B05, 0x1C56),
];

pub struct AsusDevice {
    device: hidapi::HidDevice,
}

impl AsusDevice {
    pub fn open(dev_node: &str) -> Result<Self> {
        let api = hidapi::HidApi::new()?;
        let path = CString::new(dev_node)?;
        let device = api.open_path(path.as_ref())?;
        Ok(Self { device })
    }
}

/// Below this level a "fully charged" status byte is not believed.
const FULL_MIN_PERCENT: u8 = 95;

/// Status from the raw electrical byte. The firmware has been seen reporting 0x02 (fully
/// charged) at 70% / 3.94 V while on battery, so that claim only counts near the top.
fn decode_status(byte: u8, cable_connected: bool, percentage: u8) -> BatteryStatus {
    if cable_connected {
        return BatteryStatus::Charging;
    }
    match byte {
        0x01 => BatteryStatus::Charging,
        0x02 if percentage >= FULL_MIN_PERCENT => BatteryStatus::FullyCharged,
        0x02 | 0x03 => BatteryStatus::Discharging,
        _ => BatteryStatus::Unknown,
    }
}

impl MouseDevice for AsusDevice {
    fn query_battery(&self) -> Result<BatteryReading> {
        // 64-byte write: report-ID prefix (0x00) + command 0x12 0x07 + 61 zero bytes.
        let mut cmd = [0u8; 64];
        cmd[1] = 0x12;
        cmd[2] = 0x07;

        let written = self.device.write(&cmd)?;
        if written != cmd.len() {
            bail!("short HID write: expected {} bytes, wrote {}", cmd.len(), written);
        }

        let mut buf = [0u8; 64];
        let read = self.device.read_timeout(&mut buf, 1000)?;
        if read < 6 {
            bail!("short HID read: got {read} bytes, need at least 6");
        }

        // Validate response: the device echoes the command bytes back.
        // A dead dongle returns 0xFF 0xAA instead — treat that as an error
        // so the caller can fall through to the next candidate device.
        if buf[0] != 0x12 || buf[1] != 0x07 {
            bail!(
                "unexpected response header: 0x{:02x} 0x{:02x} (expected 0x12 0x07)",
                buf[0], buf[1]
            );
        }

        let percentage = buf[4].min(100);

        // Response layout (confirmed by testing against physical device):
        //   [4]   = battery percentage 0–100
        //   [5]   = electrical status: 0x01=charging, 0x02=fully-charged, 0x03=discharging
        //   [7–8] = little-endian u16 battery voltage in mV (e.g. 0x0ec8 = 3784 mV)
        //   [9]   = cable flag: 0x01 when USB cable is plugged in, 0x00 when wireless
        //
        // The firmware always reports byte[5]=0x03 even when charging via cable. byte[9]
        // tells us the cable is connected, but voltage trend is needed to confirm charge
        // is actually flowing (the poll loop does that cross-check with history).
        let voltage_mv = u16::from_le_bytes([buf[7], buf[8]]);
        let cable_connected = buf[9] != 0;

        // Initial status from the raw bytes; the poll loop may override this
        // to Discharging if voltage is falling despite the cable being connected.
        let status = decode_status(buf[5], cable_connected, percentage);

        tracing::debug!(
            "ASUS query → {percentage}% {} voltage={voltage_mv}mV cable={cable_connected}",
            status.as_str()
        );
        Ok(BatteryReading { percentage, status, voltage_mv, cable_connected })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_byte_at_70_percent_is_not_fully_charged() {
        // Seen live: byte 0x02, 70%, 3940 mV, no cable.
        assert_eq!(decode_status(0x02, false, 70), BatteryStatus::Discharging);
    }

    #[test]
    fn full_byte_near_top_is_fully_charged() {
        assert_eq!(decode_status(0x02, false, 95), BatteryStatus::FullyCharged);
        assert_eq!(decode_status(0x02, false, 100), BatteryStatus::FullyCharged);
        assert_eq!(decode_status(0x02, false, 94), BatteryStatus::Discharging);
    }

    #[test]
    fn cable_always_means_charging() {
        assert_eq!(decode_status(0x03, true, 50), BatteryStatus::Charging);
        assert_eq!(decode_status(0x02, true, 100), BatteryStatus::Charging);
    }

    #[test]
    fn other_bytes() {
        assert_eq!(decode_status(0x01, false, 50), BatteryStatus::Charging);
        assert_eq!(decode_status(0x03, false, 50), BatteryStatus::Discharging);
        assert_eq!(decode_status(0x07, false, 50), BatteryStatus::Unknown);
    }
}
