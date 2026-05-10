# mouse-battery

Battery level monitor for ASUS wireless mice on Linux.

Shows charge percentage, voltage, charging status, and time estimates — in a GNOME Shell top-bar indicator with a click popup, and over a DBus interface any other tool can consume.

---

## Quick Install

```bash
git clone https://github.com/DenizSAHIN570/AsusMouseBatteryIndicator.git
cd AsusMouseBatteryIndicator
chmod +x install.sh
./install.sh
```

The script will:
1. Download the pre-built daemon binary (or build from source if no release exists)
2. Install a udev rule so the daemon can access `/dev/hidrawN` without running as root
3. Register and start the daemon as a systemd user service (auto-starts on login)
4. Install the GNOME Shell extension

After running, restart GNOME Shell (Wayland: log out and back in; X11: `Alt+F2` → `r`) and enable the extension:

```bash
gnome-extensions enable asus-mouse-battery-icon@gnome
```

---

## Components

| Component | Description |
|-----------|-------------|
| `daemon/` | Rust background service. Queries HID, exposes DBus, sends notifications. |
| `gnome-extension/` | GNOME Shell 45–49 indicator. Reads daemon via DBus. |
| `udev/` | udev rule for persistent hidraw permissions. |
| `systemd/` | User-level systemd service unit. |

---

## How It Works

### Device Discovery

The daemon scans `/sys/class/hidraw/*/device/uevent` for known USB vendor:product pairs, verifies the sysfs symlink contains `:1.0` (USB Interface 0 — the vendor control endpoint), then opens the matching `/dev/hidrawN` directly. No root required; a udev `TAG+="uaccess"` rule grants the active session user access.

When a wired cable and a wireless dongle are both present simultaneously, the daemon tries each candidate in order and picks the first that returns a valid response.

### HID Protocol

A 64-byte command is written to Interface 0 every 30 seconds (configurable):

```
Write: [0x00, 0x12, 0x07, 0x00 × 61]
```

The device echoes bytes [0–1] back. A `0xFF 0xAA` response means the device is unresponsive (e.g. dongle with no mouse connected) — the daemon falls through to the next candidate.

**Response byte map** (empirically confirmed):

| Byte(s) | Meaning |
|---------|---------|
| `[4]` | Battery percentage (0–100) |
| `[5]` | Electrical status — unreliable for cable detection, see `[9]` |
| `[7–8]` | Battery voltage, little-endian u16, millivolts |
| `[9]` | `0x01` = USB cable connected, `0x00` = wireless |

### Charging Detection

The firmware always reports `byte[5] = 0x03` (discharging) even when a cable is plugged in and charge is flowing. Two signals are combined to produce a reliable status:

1. **`byte[9] != 0`** — cable is physically connected
2. **Voltage trend ≥ −30 mV across the reading window** — charge is actually flowing

If both conditions hold → `"charging"`. If the cable is connected but voltage is consistently falling, the daemon reports `"discharging"` and logs a warning — the cable is too weak or faulty to overcome the mouse's own power draw.

Observed voltages on the TUF Gaming Mini WL:
- Discharging at ~40%: **~3784 mV**
- Charging at ~65%: **~4148 mV**
- LiPo full charge ceiling: ~4200 mV

### Time Estimation

A sliding window of the last 10 `(timestamp, percentage, voltage_mv)` readings is used to linearly extrapolate:
- `TimeToEmpty` when discharging
- `TimeToFull` when charging

Both return `0` until at least 2 readings are available (the extension shows "Calculating…"). The window resets on status change (charge ↔ discharge) to prevent stale rate data from polluting estimates.

---

## DBus Interface

**Session bus service**: `com.mousewatch.Battery`

```
Object: /com/mousewatch/Battery/device0
Interface: com.mousewatch.Battery1
```

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `DeviceName` | `s` | Human-readable device name |
| `Percentage` | `y` | Battery level 0–100 |
| `Status` | `s` | `"charging"` / `"discharging"` / `"fully-charged"` / `"unknown"` |
| `TimeToFull` | `u` | Seconds to full charge (0 = N/A or calculating) |
| `TimeToEmpty` | `u` | Seconds to empty (0 = N/A or calculating) |
| `VoltageMv` | `u` | Battery voltage in millivolts |
| `IsPresent` | `b` | False when mouse is unplugged |

All properties emit `org.freedesktop.DBus.Properties.PropertiesChanged` on every poll cycle.

### Signals

| Signal | Signature | Description |
|--------|-----------|-------------|
| `BatteryChanged` | `(y, s)` | Fired every poll cycle with current percentage and status |
| `BatteryLow` | `(y)` | Fired once per discharge cycle when percentage ≤ 10 |
| `BatteryFull` | `()` | Fired once when status transitions to `fully-charged` |

### Manager

```
Object: /com/mousewatch/Battery
Interface: com.mousewatch.BatteryManager1
```

| Method / Signal | Signature | Description |
|-----------------|-----------|-------------|
| `GetDevices()` | `→ ao` | Returns list of active device paths |
| `DeviceAdded` | `(o)` | Emitted when a device is detected |
| `DeviceRemoved` | `(o)` | Emitted when a device disappears |

### Quick inspection

```bash
# List all properties
busctl --user introspect com.mousewatch.Battery \
  /com/mousewatch/Battery/device0

# Read battery level
busctl --user get-property com.mousewatch.Battery \
  /com/mousewatch/Battery/device0 \
  com.mousewatch.Battery1 Percentage

# Watch live updates
busctl --user monitor com.mousewatch.Battery
```

---

## Supported Devices

| USB ID | Device | Mode |
|--------|--------|------|
| `0b05:1c57` | TUF Gaming Mini WL Mouse MIKU | Wireless (dongle) |
| `0b05:1c56` | TUF Gaming Mini WL Mouse MIKU | Wired (USB cable) |

### Adding a new device

The daemon uses a `MouseDevice` trait in `daemon/src/hid/mod.rs`. To add support for another mouse:

1. Add its `(vendor_id, product_id)` to `ASUS_KNOWN_IDS` in `daemon/src/hid/asus.rs`
2. Verify the response byte map matches (empirically, using `busctl --user monitor`)
3. Rebuild and reinstall the daemon binary

The response validation, fallback logic, time estimation, and DBus publishing all work automatically.

---

## Building from Source

### Requirements

- Rust 1.70+ (`rustup` recommended)
- `libhidapi-hidraw` development headers
  - Fedora/RHEL: `sudo dnf install hidapi-devel`
  - Debian/Ubuntu: `sudo apt install libhidapi-dev`

### Build

```bash
cd daemon
cargo build --release
# Binary at: daemon/target/release/mouse-battery
```

### Run (development)

```bash
# Permissions: your user needs access to /dev/hidrawN.
# Either run the udev rule install, or temporarily:
#   sudo chmod a+rw /dev/hidraw0  (not persistent)

RUST_LOG=mouse_battery=debug cargo run
```

### Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MOUSE_BATTERY_INTERVAL` | `30` | Poll interval in seconds |
| `RUST_LOG` | — | Log filter, e.g. `mouse_battery=debug` |

---

## Permissions

The daemon needs read/write access to `/dev/hidrawN` for the mouse's Interface 0 node. The udev rule grants this automatically on plug-in:

```bash
sudo install -Dm644 udev/99-mouse-battery.rules /etc/udev/rules.d/
sudo udevadm control --reload
sudo udevadm trigger
```

`TAG+="uaccess"` is the modern standard — udev + logind automatically grants the current console session user access. No hardcoded groups needed.

---

## Running as a Service

```bash
# Install binary
install -Dm755 daemon/target/release/mouse-battery ~/.local/bin/mouse-battery

# Install service unit
install -Dm644 systemd/mouse-battery.service ~/.config/systemd/user/

# Enable and start
systemctl --user daemon-reload
systemctl --user enable --now mouse-battery

# Check logs
journalctl --user -u mouse-battery -f
```

---

## GNOME Extension

The extension (`asus-mouse-battery-icon@gnome`) shows a coloured battery icon with percentage in the top bar. Click it to see:

- Device name
- Charging status
- Time to full / time to empty
- Battery voltage (mV)

Colour thresholds:

| Range | Colour |
|-------|--------|
| > 50% | Green |
| 11–50% | Orange |
| ≤ 10% | Red |

The indicator hides automatically when the daemon is not running or the mouse is disconnected, and reappears when either comes back.

---

## Notifications

Two desktop notifications are sent via `org.freedesktop.Notifications`:

| Trigger | Message |
|---------|---------|
| Percentage drops to 10% or below | "Mouse Battery Low — 10% remaining" |
| Status becomes fully charged | "Mouse Battery Full" |

Each fires at most once per charge cycle.

---

## Protocol Notes

This protocol was reverse-engineered by observing raw HID traffic against the physical device. No official documentation exists.

Key findings that differ from naive expectations:

- **`byte[5]` is not a reliable charging indicator.** The firmware reports `0x03` ("discharging") even when a cable is plugged in and charge is flowing. Use `byte[9]` for cable detection.
- **`byte[9]` is the cable flag, not a charging flag.** It tells you the USB cable is physically connected. It does not guarantee charge current is flowing (a weak or failing cable can be connected at USB level while not delivering enough power).
- **`byte[7–8]` is battery voltage** (little-endian u16, millivolts). Combine with `byte[9]` to confirm charging: cable connected + voltage not falling = actually charging.
- **The dongle returns `0xFF 0xAA`** when the mouse is connected via cable. Do not treat this as an error — fall through to the wired PID candidate.
