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
        let status = if cable_connected {
            BatteryStatus::Charging
        } else {
            match buf[5] {
                0x01 => BatteryStatus::Charging,
                0x02 => BatteryStatus::FullyCharged,
                0x03 => BatteryStatus::Discharging,
                _    => BatteryStatus::Unknown,
            }
        };

        tracing::debug!(
            "ASUS query → {percentage}% {} voltage={voltage_mv}mV cable={cable_connected}",
            status.as_str()
        );
        Ok(BatteryReading { percentage, status, voltage_mv, cable_connected })
    }
}
