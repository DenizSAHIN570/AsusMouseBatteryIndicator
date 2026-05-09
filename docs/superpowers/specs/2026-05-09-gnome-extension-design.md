# GNOME Extension Design — asus-mouse-battery-icon@gnome

**Date:** 2026-05-09
**Status:** Approved

---

## Overview

A GNOME Shell extension that reads battery data from the running `mouse-battery` daemon via DBus and displays a coloured battery icon with percentage in the top bar. Clicking opens a popup with full status details. The daemon owns all notification logic; the extension is display-only.

---

## Files

```
gnome-extension/
├── metadata.json      — UUID, name, shell-version (45–49)
├── extension.js       — ES module: indicator widget + DBus proxy
└── stylesheet.css     — .battery-green / .battery-orange / .battery-red colour classes
```

No GSettings schema. There are no user-configurable settings in the extension — the daemon owns the poll interval (env var) and notifications; colour thresholds are locked by product decision.

---

## Architecture

`extension.js` exports a single `Extension` class (GNOME Shell 45+ ES module API, `gi://` imports):

- **`enable()`** — creates the `PanelMenu.Button` indicator and starts a `Gio.DBusProxy` on the session bus
- **`disable()`** — destroys the indicator and tears down the proxy cleanly

The extension is purely reactive: no timer, no polling. All updates are driven by signals from the daemon.

---

## DBus Connection (Option A — Gio.DBusProxy)

```
Bus:        session
Service:    com.mousewatch.Battery
Path:       /com/mousewatch/Battery/device0
Interface:  com.mousewatch.Battery1
```

Properties consumed: `Percentage (y)`, `Status (s)`, `TimeToFull (u)`, `TimeToEmpty (u)`, `VoltageMv (u)`, `IsPresent (b)`, `DeviceName (s)`.

The proxy is created with `Gio.DBusProxy.new_for_bus()`. Two signals drive all UI updates:

| Signal | Action |
|--------|--------|
| `g-properties-changed` | Read cached properties, update icon/label/popup |
| `notify::g-name-owner` | Show indicator when daemon appears; hide when it stops |

---

## Data Flow

```
Daemon polls HID every 30s
  → emits PropertiesChanged on session bus

Gio.DBusProxy receives g-properties-changed
  → reads: Percentage, Status, TimeToFull/Empty, VoltageMv, IsPresent, DeviceName
  → IsPresent = false or no name owner → hide indicator
  → else → update icon class + label text + popup rows
```

---

## Top-Bar Indicator

A `PanelMenu.Button` containing two children side-by-side:

```
[ battery-icon ]  [ 40% ]
```

**Icon** — system symbolic icon selected by state:

| State | Icon |
|-------|------|
| `fully-charged` | `battery-full-charged-symbolic` |
| `> 50%` | `battery-full-symbolic` |
| `11–50%` | `battery-good-symbolic` |
| `≤ 10%` | `battery-caution-symbolic` |

**Label** — `"40%"` while data is available; `"–"` before first reading.

**Colour** — CSS class applied to the button box, tinting both icon and label:

| Range | Class |
|-------|-------|
| `> 50%` | `.battery-green` |
| `11–50%` | `.battery-orange` |
| `≤ 10%` | `.battery-red` |

Fully-charged uses `.battery-green`.

---

## Click Popup

A `PopupMenu` with four rows, updated on every `g-properties-changed` event:

```
TUF GAMING MINI WL MOUSE MIKU
────────────────────────────────
Status:   Charging
Time:     1h 23m to full
Voltage:  4148 mV
```

**Time row logic:**
- Status `charging` and `TimeToFull > 0` → `"Xh Ym to full"`
- Status `discharging` and `TimeToEmpty > 0` → `"Xh Ym to empty"`
- Status active but time value is `0` → `"Calculating…"`
- Status `fully-charged` or `unknown` → `"—"`

---

## Error Handling

| Condition | Behaviour |
|-----------|-----------|
| Daemon not running at `enable()` | Proxy has no name owner → indicator hidden |
| Daemon starts later | `notify::g-name-owner` fires → indicator appears |
| Mouse disconnected (`IsPresent = false`) | Daemon emits `PropertiesChanged` → extension hides indicator |
| Mouse reconnects | Next daemon poll sets `IsPresent = true` → indicator reappears |
| DBus proxy creation throws | Error logged via `console.error`; extension stays hidden — Shell does not crash |

No retry loops or timers needed — `Gio.DBusProxy` handles name-watching and reconnection internally.

---

## Out of Scope

- `prefs.js` preferences UI (settings changeable via `gsettings` CLI if ever needed)
- Multiple simultaneous mice
- Duplicate notifications from the extension (daemon owns all notifications)
- DBus activation of the daemon (the systemd user service is the correct start mechanism)
