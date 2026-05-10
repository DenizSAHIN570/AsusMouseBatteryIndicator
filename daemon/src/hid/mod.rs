pub mod asus;

use anyhow::Result;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatteryStatus {
    Charging,
    FullyCharged,
    Discharging,
    Unknown,
}

impl BatteryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Charging     => "charging",
            Self::FullyCharged => "fully-charged",
            Self::Discharging  => "discharging",
            Self::Unknown      => "unknown",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BatteryReading {
    pub percentage: u8,
    pub status: BatteryStatus,
    /// Battery voltage in millivolts, parsed from response bytes [7–8] (little-endian u16).
    pub voltage_mv: u16,
    /// True when the USB cable is physically connected (response byte[9] != 0).
    /// Does not guarantee charge is flowing — use voltage trend to confirm.
    pub cable_connected: bool,
}

pub trait MouseDevice: Send + 'static {
    fn query_battery(&self) -> Result<BatteryReading>;
}

pub struct HidrawMatch {
    pub dev_node: String,
}

/// Find all hidraw nodes matching known (vendor, product) pairs at USB Interface 0,
/// sorted by hidraw index (hidraw0 < hidraw9 < ...).
///
/// Returns multiple matches so the caller can try each one and pick the first that
/// actually responds — needed when e.g. a dead dongle and an active wired connection
/// are both present simultaneously.
///
/// Discovery is done via sysfs to avoid calling hidapi::enumerate(), which would
/// require permissions on every hidraw node in the system.
pub fn find_hidraw_nodes(known_ids: &[(u16, u16)]) -> Result<Vec<HidrawMatch>> {
    let base = Path::new("/sys/class/hidraw");

    let mut entries: Vec<_> = fs::read_dir(base)?.flatten().collect();
    entries.sort_by_key(|e| e.file_name());

    let mut matches = Vec::new();

    for entry in entries {
        let node_name = entry.file_name();
        let node_str = match node_name.to_str() {
            Some(s) => s.to_owned(),
            None => continue,
        };

        let uevent_path = base.join(&node_str).join("device/uevent");
        let content = match fs::read_to_string(&uevent_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Find: HID_ID=0003:00000B05:00001C57
        let hid_id_line = match content.lines().find(|l| l.starts_with("HID_ID=")) {
            Some(l) => l,
            None => continue,
        };

        let parts: Vec<&str> = hid_id_line
            .trim_start_matches("HID_ID=")
            .split(':')
            .collect();
        if parts.len() != 3 {
            continue;
        }

        let Ok(vid) = u32::from_str_radix(parts[1], 16) else { continue };
        let Ok(pid) = u32::from_str_radix(parts[2], 16) else { continue };
        let vid = vid as u16;
        let pid = pid as u16;

        if !known_ids.iter().any(|&(v, p)| v == vid && p == pid) {
            continue;
        }

        // Verify this is Interface 0 by inspecting the sysfs symlink target.
        // The path component for the USB interface looks like "3-1:1.0" where
        // the trailing ".0" is the interface number. Interface 0 → ":1.0".
        let symlink = base.join(&node_str);
        let resolved = match fs::read_link(&symlink) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let resolved_str = resolved.to_string_lossy();
        if !resolved_str.contains(":1.0") {
            continue;
        }

        let dev_node = format!("/dev/{}", node_str);
        tracing::debug!("Candidate: {dev_node} (VID={vid:#06x} PID={pid:#06x})");
        matches.push(HidrawMatch { dev_node });
    }

    Ok(matches)
}
