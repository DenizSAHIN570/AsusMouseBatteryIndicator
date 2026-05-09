# Mouse Battery Monitor — Plan

## Overview

A two-component system:
1. **Rust daemon** (`mouse-battery`) — discovers wireless mice via hidraw, queries battery over HID, exposes data on DBus, sends system notifications. **Complete.**
2. **GNOME extension** (`asus-mouse-battery-icon@gnome`) — reads from DBus, shows battery icon + % in the top bar, popup on click. **Next.**

The daemon is desktop-agnostic and distro-agnostic. The GNOME extension is one possible frontend; others (Waybar, Polybar, KDE widget) could be built against the same DBus interface.

---

## All Decisions — Locked

| Decision | Choice |
|----------|--------|
| Language | Rust (daemon), GJS/JavaScript (GNOME extension) |
| Extension UUID | `asus-mouse-battery-icon@gnome` |
| Poll interval | 30s default, configurable via `MOUSE_BATTERY_INTERVAL` env var |
| Multiple mice | First responding candidate wins |
| Icon style | System `battery-*-symbolic` icons from active theme |
| Green threshold | > 50% |
| Orange threshold | 11–50% |
| Red threshold | ≤ 10% |
| Notification triggers | Crossing ≤ 10% (low), status → fully-charged |
| Daemon bus | Session bus (no root needed) |
| Permissions | `TAG+="uaccess"` udev rule |
| Install target | `~/.local/bin` (daemon), `~/.local/share/gnome-shell/extensions/` (extension) |
| GNOME Shell version | 49.5 (target 45–49, ES module API) |

---

## What Was Discovered (Protocol Reverse Engineering)

The ASUS TUF Gaming Mini WL Mouse MIKU (USB `0b05:1c57`) was investigated live against the physical device. All findings are empirically confirmed.

### Device Enumeration

The mouse exposes two USB identities:

| PID | Mode | hidraw Interface |
|-----|------|-----------------|
| `0x1C57` | Wireless via USB dongle | Interface 0 (`3-1:1.0`) |
| `0x1C56` | Wired USB cable (charging) | Interface 0 (`1-6:1.0`) |

When charging via cable, **both** PIDs can appear simultaneously. The dongle (`0x1C57`) stops responding to HID commands while the cable is connected — it returns `0xFF 0xAA` instead of echoing the command. The daemon tries candidates in hidraw index order and picks the first valid response.

### HID Command

Written to Interface 0 (vendor control, usage page `0xFF01`), 64 bytes:

```
[0x00, 0x12, 0x07, 0x00 × 61]
```

Byte 0 is the HID report ID prefix (required by hidraw write). The response echoes bytes [0–1] back; if they don't match `0x12 0x07`, the device is dead/unresponsive.

### Response Layout (64 bytes)

| Byte(s) | Value (examples) | Meaning |
|---------|-----------------|---------|
| [0] | `0x12` | Command echo — used to validate response |
| [1] | `0x07` | Subcommand echo — used to validate response |
| [4] | `0x28` (40) | **Battery percentage, 0–100** |
| [5] | `0x03` | Electrical status (unreliable for cable detection — see byte[9]) |
| [7–8] | `0xC8 0x0E` → 3784 mV | **Battery voltage, little-endian u16 in millivolts** |
| [9] | `0x01` / `0x00` | **Cable flag: 1 = USB cable connected, 0 = wireless** |

### Byte [5] Status Codes

| Value | Meaning |
|-------|---------|
| `0x01` | Charging |
| `0x02` | Fully charged |
| `0x03` | Discharging — **also reported when cable is connected; do not use alone** |

### Charging Detection — Dual Signal

The firmware always reports `0x03` for byte[5] even when the USB cable is connected and charge is flowing. Two signals are combined:

1. **byte[9] = 1** — cable is physically connected (necessary condition)
2. **Voltage trend ≥ −30 mV across the sliding window** — charge is actually flowing (sufficient condition)

If byte[9] is set but voltage is consistently falling across readings, the cable is too weak or faulty — status is downgraded to `discharging` and a warning is logged.

Voltage observations:
- Wireless/discharging at ~40%: **~3784 mV**
- Wired/charging at ~65%: **~4148 mV**
- LiPo full charge ceiling: ~4200 mV

---

## Architecture

```
┌─────────────────────────────────────────────┐
│       mouse-battery  (Rust daemon)          │
│                                             │
│  sysfs HID enumeration                      │
│    /sys/class/hidraw/*/device/uevent        │
│    match HID_ID vendor:product              │
│    verify ":1.0" in symlink → Interface 0   │
│    return all candidates (Vec)              │
│                                             │
│  Try each candidate in order                │
│    open via hidapi open_path                │
│    validate response echo bytes             │
│    first valid response wins                │
│                                             │
│  Poll loop (30s, MissedTickBehavior::Delay) │
│    query → BatteryReading                   │
│    dual-signal charging cross-check         │
│    feed BatteryPredictor (sliding window)   │
│    update Arc<Mutex<BatteryState>>          │
│    emit DBus PropertiesChanged + signals    │
│    notification state machine               │
│                                             │
│  DBus service (session bus)                 │
│    com.mousewatch.Battery                   │
│    /com/mousewatch/Battery/device0          │
└──────────────────┬──────────────────────────┘
                   │  DBus  (session bus)
                   ▼
┌─────────────────────────────────────────────┐
│   asus-mouse-battery-icon@gnome  (ext)      │  ← NEXT
│                                             │
│  PanelMenu.Button in top bar                │
│    St.Icon  — battery-*-symbolic            │
│    St.Label — "40%"                         │
│    CSS class: .battery-green/-orange/-red   │
│                                             │
│  PopupMenu on click                         │
│    Status / Voltage / Time estimates        │
│                                             │
│  System notifications at ≤10% and full     │
└─────────────────────────────────────────────┘
```

---

## DBus Interface (session bus) — Stable

**Service**: `com.mousewatch.Battery`
**Manager path**: `/com/mousewatch/Battery`
**Device path**: `/com/mousewatch/Battery/device0`

### Interface `com.mousewatch.Battery1`

| Property | Type | Description |
|----------|------|-------------|
| `DeviceName` | `s` | e.g. `"TUF GAMING MINI WL MOUSE MIKU"` |
| `Percentage` | `y` | 0–100 |
| `Status` | `s` | `"charging"` \| `"discharging"` \| `"fully-charged"` \| `"unknown"` |
| `TimeToFull` | `u` | Seconds, 0 = not applicable or still calculating |
| `TimeToEmpty` | `u` | Seconds, 0 = not applicable or still calculating |
| `VoltageMv` | `u` | Battery voltage in millivolts |
| `IsPresent` | `b` | False when device unplugged |

All properties emit `org.freedesktop.DBus.Properties.PropertiesChanged` on change.

| Signal | Signature | Fired when |
|--------|-----------|------------|
| `BatteryChanged` | `ys` | Every poll cycle |
| `BatteryLow` | `y` | Percentage crosses ≤10%, once per cycle |
| `BatteryFull` | — | Status becomes `fully-charged`, once per cycle |

### Interface `com.mousewatch.BatteryManager1`

| Method / Signal | Signature | Description |
|-----------------|-----------|-------------|
| `GetDevices()` | `→ ao` | Returns array of device object paths |
| `DeviceAdded` | `o` | Device connected |
| `DeviceRemoved` | `o` | Device disconnected |

---

## Colour Thresholds (extension)

| Range | Colour |
|-------|--------|
| 51–100% | Green |
| 11–50% | Orange |
| 0–10% | Red |

---

## Project Structure

```
mouse-battery/
├── daemon/                         ← COMPLETE
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                 poll loop, orchestration, notification state machine
│       ├── hid/
│       │   ├── mod.rs              sysfs enumeration, MouseDevice trait, BatteryReading
│       │   └── asus.rs             ASUS HID protocol, dual-signal charging detection
│       ├── dbus/
│       │   ├── mod.rs              connection builder, constants
│       │   ├── device.rs           BatteryDevice zbus object + BatteryState
│       │   └── manager.rs          BatteryManager zbus object
│       ├── predictor.rs            sliding-window time & voltage trend estimator
│       └── notification.rs         org.freedesktop.Notifications helper
│
├── gnome-extension/                ← NEXT
│   ├── metadata.json
│   ├── extension.js
│   ├── stylesheet.css
│   └── schemas/
│       └── org.gnome.shell.extensions.asus-mouse-battery-icon.gschema.xml
│
├── udev/
│   └── 99-mouse-battery.rules      TAG+="uaccess" for vendor 0b05
│
├── systemd/
│   └── mouse-battery.service       user-level service unit
│
├── PLAN.md                         this file
└── README.md
```

---

## Rust Crates

| Crate | Version | Purpose |
|-------|---------|---------|
| `hidapi` | 2.6 | HID enumeration and raw read/write |
| `zbus` | 5 (tokio feature) | Async DBus service |
| `zvariant` | 5 | DBus type support |
| `tokio` | 1 (full) | Async runtime |
| `tracing` | 0.1 | Structured logging |
| `tracing-subscriber` | 0.3 | Log output |
| `anyhow` | 1 | Error propagation |

---

## Udev Rule (to create)

```udev
# Grant active session user access to ASUS wireless mouse hidraw nodes
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0b05", TAG+="uaccess"
```

File: `/etc/udev/rules.d/99-mouse-battery.rules`

---

## Systemd User Service (to create)

```ini
[Unit]
Description=Mouse Battery Monitor Daemon
After=graphical-session.target dbus.socket
Wants=dbus.socket

[Service]
ExecStart=%h/.local/bin/mouse-battery
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
```

---

## GNOME Extension (next)

- **UUID**: `asus-mouse-battery-icon@gnome`
- **Target**: GNOME Shell 45–49 (ES module API, `gi://` imports)
- **GSettings schema**: `org.gnome.shell.extensions.asus-mouse-battery-icon`
  - `poll-interval` (uint32, default 30)
  - `low-threshold` (uint32, default 10)
- **DBus client**: `Gio.DBusProxy` on session bus, subscribe to `PropertiesChanged`
- **Notifications**: via `Gio.Notification` (GNOME Shell native)

---

## What Is NOT in Scope

- Multiple simultaneous mice
- GNOME preferences UI (`prefs.js`) — settings changeable via `gsettings` CLI
- Extension submission to extensions.gnome.org
- Non-ASUS devices (architecture supports adding; protocol impl not written)
