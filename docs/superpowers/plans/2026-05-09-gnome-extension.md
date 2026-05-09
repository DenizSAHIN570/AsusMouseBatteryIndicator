# GNOME Extension Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `asus-mouse-battery-icon@gnome` GNOME Shell extension that reads battery data from the `mouse-battery` daemon via DBus and shows a coloured icon + percentage in the top bar with a click popup.

**Architecture:** A single `Extension` class (GNOME 45+ ES module API) creates a `PanelMenu.Button` and a `Gio.DBusProxy` in `enable()`. The proxy watches `g-properties-changed` and `notify::g-name-owner` — no polling timer. The indicator is hidden when the daemon is not running or `IsPresent` is false.

**Tech Stack:** GJS ES modules, `gi://Gio`, `gi://St`, `gi://Clutter`, `resource:///org/gnome/shell/ui/panelMenu.js`, `resource:///org/gnome/shell/ui/popupMenu.js`. No GSettings. No prefs.js. GNOME Shell 45–49.

---

## File Map

```
gnome-extension/
├── metadata.json          — UUID, name, shell-version array
├── extension.js           — Extension class, indicator, DBus proxy, update logic
├── stylesheet.css         — .battery-green / .battery-orange / .battery-red
└── test-format.js         — Standalone gjs test for the two format helpers (not installed)
```

Symlink (created in Task 1):
```
~/.local/share/gnome-shell/extensions/asus-mouse-battery-icon@gnome
  → /home/deniz/Documents/Projects/AsusMouseBattery/gnome-extension
```

---

## Task 1: Extension scaffold + install symlink

**Files:**
- Create: `gnome-extension/metadata.json`
- Create: `gnome-extension/stylesheet.css`
- Create: `gnome-extension/extension.js` (stub)
- Run: symlink into extension directory, enable, check Shell loads it

- [ ] **Step 1: Create metadata.json**

```json
{
  "id": "asus-mouse-battery-icon@gnome",
  "name": "ASUS Mouse Battery",
  "description": "Shows ASUS wireless mouse battery level in the GNOME top bar.",
  "version": 1,
  "shell-version": ["45", "46", "47", "48", "49"]
}
```

Save to: `gnome-extension/metadata.json`

- [ ] **Step 2: Create stylesheet.css**

```css
.battery-green {
    color: #57e389;
}

.battery-orange {
    color: #ff7800;
}

.battery-red {
    color: #e01b24;
}
```

Save to: `gnome-extension/stylesheet.css`

- [ ] **Step 3: Create extension.js stub**

```javascript
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

export default class AsusMouseBatteryExtension extends Extension {
    enable() {
        console.log('[asus-mouse-battery] enabled');
    }

    disable() {
        console.log('[asus-mouse-battery] disabled');
    }
}
```

Save to: `gnome-extension/extension.js`

- [ ] **Step 4: Install symlink**

```bash
mkdir -p ~/.local/share/gnome-shell/extensions
ln -sfn /home/deniz/Documents/Projects/AsusMouseBattery/gnome-extension \
  ~/.local/share/gnome-shell/extensions/asus-mouse-battery-icon@gnome
```

- [ ] **Step 5: Restart GNOME Shell to pick up the new extension**

On X11: press `Alt+F2`, type `r`, press Enter.
On Wayland: log out and log back in.

- [ ] **Step 6: Enable the extension**

```bash
gnome-extensions enable asus-mouse-battery-icon@gnome
```

Expected output: no error.

- [ ] **Step 7: Verify the extension loaded without errors**

```bash
journalctl --user -b -g 'asus-mouse-battery' --no-pager
```

Expected: one line containing `[asus-mouse-battery] enabled`. No JS errors.

- [ ] **Step 8: Commit**

```bash
git -C /home/deniz/Documents/Projects/AsusMouseBattery add gnome-extension/metadata.json gnome-extension/stylesheet.css gnome-extension/extension.js
git -C /home/deniz/Documents/Projects/AsusMouseBattery commit -m "feat(extension): add extension scaffold"
```

---

## Task 2: Format helper functions + gjs tests

**Files:**
- Modify: `gnome-extension/extension.js` (add two module-level functions)
- Create: `gnome-extension/test-format.js` (standalone gjs test, not installed)

- [ ] **Step 1: Write the test file**

```javascript
// gnome-extension/test-format.js — run with: gjs gnome-extension/test-format.js

function _formatStatus(status) {
    const labels = {
        'charging': 'Charging',
        'discharging': 'Discharging',
        'fully-charged': 'Fully charged',
        'unknown': 'Unknown',
    };
    return labels[status] ?? status;
}

function _formatTime(status, timeToFull, timeToEmpty) {
    if (status === 'fully-charged' || status === 'unknown') return '—';
    const seconds = status === 'charging' ? timeToFull : timeToEmpty;
    const suffix = status === 'charging' ? 'to full' : 'to empty';
    if (seconds === 0) return 'Calculating…';
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    return h > 0 ? `${h}h ${m}m ${suffix}` : `${m}m ${suffix}`;
}

const TESTS = [
    [_formatTime('charging', 4980, 0),    '1h 23m to full'],
    [_formatTime('discharging', 0, 1800), '30m to empty'],
    [_formatTime('charging', 0, 0),       'Calculating…'],
    [_formatTime('fully-charged', 0, 0),  '—'],
    [_formatTime('unknown', 0, 0),        '—'],
    [_formatStatus('charging'),           'Charging'],
    [_formatStatus('fully-charged'),      'Fully charged'],
    [_formatStatus('discharging'),        'Discharging'],
];

let passed = 0;
for (const [got, expected] of TESTS) {
    if (got === expected) {
        print(`PASS: "${expected}"`);
        passed++;
    } else {
        print(`FAIL: got "${got}", expected "${expected}"`);
    }
}
print(`\n${passed}/${TESTS.length} passed`);
if (passed !== TESTS.length) imports.system.exit(1);
```

Save to: `gnome-extension/test-format.js`

- [ ] **Step 2: Run tests and verify all 8 pass**

```bash
gjs gnome-extension/test-format.js
```

Expected output:
```
PASS: "1h 23m to full"
PASS: "30m to empty"
PASS: "Calculating…"
PASS: "—"
PASS: "—"
PASS: "Charging"
PASS: "Fully charged"
PASS: "Discharging"

8/8 passed
```

- [ ] **Step 3: Add the two functions to extension.js above the class**

Replace the stub `extension.js` with:

```javascript
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

function _formatStatus(status) {
    const labels = {
        'charging': 'Charging',
        'discharging': 'Discharging',
        'fully-charged': 'Fully charged',
        'unknown': 'Unknown',
    };
    return labels[status] ?? status;
}

function _formatTime(status, timeToFull, timeToEmpty) {
    if (status === 'fully-charged' || status === 'unknown') return '—';
    const seconds = status === 'charging' ? timeToFull : timeToEmpty;
    const suffix = status === 'charging' ? 'to full' : 'to empty';
    if (seconds === 0) return 'Calculating…';
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    return h > 0 ? `${h}h ${m}m ${suffix}` : `${m}m ${suffix}`;
}

export default class AsusMouseBatteryExtension extends Extension {
    enable() {
        console.log('[asus-mouse-battery] enabled');
    }

    disable() {
        console.log('[asus-mouse-battery] disabled');
    }
}
```

- [ ] **Step 4: Commit**

```bash
git -C /home/deniz/Documents/Projects/AsusMouseBattery add gnome-extension/extension.js gnome-extension/test-format.js
git -C /home/deniz/Documents/Projects/AsusMouseBattery commit -m "feat(extension): add format helper functions with gjs tests"
```

---

## Task 3: DBus proxy + show/hide logic

**Files:**
- Modify: `gnome-extension/extension.js`

The proxy watches `notify::g-name-owner` to detect daemon start/stop, and `g-properties-changed` for every poll update. `DO_NOT_AUTO_START` prevents DBus from launching the daemon — systemd owns that.

- [ ] **Step 1: Replace extension.js with proxy skeleton**

```javascript
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import Gio from 'gi://Gio';

const DBUS_NAME  = 'com.mousewatch.Battery';
const DBUS_PATH  = '/com/mousewatch/Battery/device0';
const DBUS_IFACE = 'com.mousewatch.Battery1';

function _formatStatus(status) {
    const labels = {
        'charging': 'Charging',
        'discharging': 'Discharging',
        'fully-charged': 'Fully charged',
        'unknown': 'Unknown',
    };
    return labels[status] ?? status;
}

function _formatTime(status, timeToFull, timeToEmpty) {
    if (status === 'fully-charged' || status === 'unknown') return '—';
    const seconds = status === 'charging' ? timeToFull : timeToEmpty;
    const suffix = status === 'charging' ? 'to full' : 'to empty';
    if (seconds === 0) return 'Calculating…';
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    return h > 0 ? `${h}h ${m}m ${suffix}` : `${m}m ${suffix}`;
}

export default class AsusMouseBatteryExtension extends Extension {
    enable() {
        this._proxy = new Gio.DBusProxy({
            g_connection: Gio.DBus.session,
            g_name: DBUS_NAME,
            g_object_path: DBUS_PATH,
            g_interface_name: DBUS_IFACE,
            g_flags: Gio.DBusProxyFlags.DO_NOT_AUTO_START,
        });
        this._proxy.init(null);

        this._propsChangedId = this._proxy.connect('g-properties-changed', () => {
            this._update();
        });
        this._nameOwnerId = this._proxy.connect('notify::g-name-owner', () => {
            this._update();
        });

        this._update();
    }

    disable() {
        if (this._propsChangedId) {
            this._proxy.disconnect(this._propsChangedId);
            this._propsChangedId = null;
        }
        if (this._nameOwnerId) {
            this._proxy.disconnect(this._nameOwnerId);
            this._nameOwnerId = null;
        }
        this._proxy = null;
    }

    _update() {
        const hasOwner = Boolean(this._proxy.g_name_owner);
        console.log(`[asus-mouse-battery] _update: hasOwner=${hasOwner}`);
    }
}
```

- [ ] **Step 2: Reload the extension**

```bash
gnome-extensions disable asus-mouse-battery-icon@gnome
gnome-extensions enable asus-mouse-battery-icon@gnome
```

- [ ] **Step 3: Verify proxy connects when daemon is running**

```bash
journalctl --user -b -g 'asus-mouse-battery' --no-pager
```

Expected: `_update: hasOwner=true` (if daemon running) or `hasOwner=false` (if daemon stopped).

- [ ] **Step 4: Test daemon stop/start visibility signalling**

```bash
# Stop daemon
systemctl --user stop mouse-battery
journalctl --user -b -n 5 -g 'asus-mouse-battery' --no-pager
# Expected: _update: hasOwner=false

# Start daemon
systemctl --user start mouse-battery
journalctl --user -b -n 5 -g 'asus-mouse-battery' --no-pager
# Expected: _update: hasOwner=true
```

- [ ] **Step 5: Commit**

```bash
git -C /home/deniz/Documents/Projects/AsusMouseBattery add gnome-extension/extension.js
git -C /home/deniz/Documents/Projects/AsusMouseBattery commit -m "feat(extension): add DBus proxy with name-owner watching"
```

---

## Task 4: Top-bar indicator (icon + label + colour)

**Files:**
- Modify: `gnome-extension/extension.js`

- [ ] **Step 1: Replace extension.js with indicator UI added**

```javascript
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import St from 'gi://St';

const DBUS_NAME  = 'com.mousewatch.Battery';
const DBUS_PATH  = '/com/mousewatch/Battery/device0';
const DBUS_IFACE = 'com.mousewatch.Battery1';

function _formatStatus(status) {
    const labels = {
        'charging': 'Charging',
        'discharging': 'Discharging',
        'fully-charged': 'Fully charged',
        'unknown': 'Unknown',
    };
    return labels[status] ?? status;
}

function _formatTime(status, timeToFull, timeToEmpty) {
    if (status === 'fully-charged' || status === 'unknown') return '—';
    const seconds = status === 'charging' ? timeToFull : timeToEmpty;
    const suffix = status === 'charging' ? 'to full' : 'to empty';
    if (seconds === 0) return 'Calculating…';
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    return h > 0 ? `${h}h ${m}m ${suffix}` : `${m}m ${suffix}`;
}

export default class AsusMouseBatteryExtension extends Extension {
    enable() {
        // Build indicator
        this._indicator = new PanelMenu.Button(0.0, 'ASUS Mouse Battery', false);

        this._box = new St.BoxLayout({style_class: 'panel-status-menu-box'});
        this._icon = new St.Icon({
            icon_name: 'battery-missing-symbolic',
            style_class: 'system-status-icon',
        });
        this._label = new St.Label({
            text: '–',
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._box.add_child(this._icon);
        this._box.add_child(this._label);
        this._indicator.add_child(this._box);

        Main.panel.addToStatusArea(this.uuid, this._indicator);
        this._indicator.hide();

        // DBus proxy
        this._proxy = new Gio.DBusProxy({
            g_connection: Gio.DBus.session,
            g_name: DBUS_NAME,
            g_object_path: DBUS_PATH,
            g_interface_name: DBUS_IFACE,
            g_flags: Gio.DBusProxyFlags.DO_NOT_AUTO_START,
        });
        this._proxy.init(null);

        this._propsChangedId = this._proxy.connect('g-properties-changed', () => {
            this._update();
        });
        this._nameOwnerId = this._proxy.connect('notify::g-name-owner', () => {
            this._update();
        });

        this._update();
    }

    disable() {
        if (this._propsChangedId) {
            this._proxy.disconnect(this._propsChangedId);
            this._propsChangedId = null;
        }
        if (this._nameOwnerId) {
            this._proxy.disconnect(this._nameOwnerId);
            this._nameOwnerId = null;
        }
        this._proxy = null;
        this._indicator?.destroy();
        this._indicator = null;
    }

    _update() {
        if (!this._proxy.g_name_owner) {
            this._indicator.hide();
            return;
        }

        const isPresentVar = this._proxy.get_cached_property('IsPresent');
        if (!isPresentVar?.unpack()) {
            this._indicator.hide();
            return;
        }

        const pct    = this._proxy.get_cached_property('Percentage')?.unpack()  ?? 0;
        const status = this._proxy.get_cached_property('Status')?.unpack()      ?? 'unknown';

        // Icon
        let iconName;
        if (status === 'fully-charged') {
            iconName = 'battery-full-charged-symbolic';
        } else if (pct > 50) {
            iconName = 'battery-full-symbolic';
        } else if (pct > 10) {
            iconName = 'battery-good-symbolic';
        } else {
            iconName = 'battery-caution-symbolic';
        }
        this._icon.icon_name = iconName;

        // Colour class
        ['battery-green', 'battery-orange', 'battery-red'].forEach(c =>
            this._box.remove_style_class_name(c));
        if (status === 'fully-charged' || pct > 50) {
            this._box.add_style_class_name('battery-green');
        } else if (pct > 10) {
            this._box.add_style_class_name('battery-orange');
        } else {
            this._box.add_style_class_name('battery-red');
        }

        this._label.text = `${pct}%`;
        this._indicator.show();
    }
}
```

- [ ] **Step 2: Reload the extension**

```bash
gnome-extensions disable asus-mouse-battery-icon@gnome
gnome-extensions enable asus-mouse-battery-icon@gnome
```

- [ ] **Step 3: Verify indicator appears in top bar**

Look at the top bar. With the daemon running and mouse connected you should see a battery icon and a percentage label (e.g. `40%`) in green.

- [ ] **Step 4: Verify hide/show on daemon stop/start**

```bash
systemctl --user stop mouse-battery
# Indicator should disappear from top bar

systemctl --user start mouse-battery
# Indicator should reappear within ~35 seconds (next poll cycle)
```

- [ ] **Step 5: Check journalctl for no errors**

```bash
journalctl --user -b -g 'asus-mouse-battery\|gnome-shell' --no-pager | tail -20
```

Expected: no `TypeError`, `ReferenceError`, or `JS ERROR` lines.

- [ ] **Step 6: Commit**

```bash
git -C /home/deniz/Documents/Projects/AsusMouseBattery add gnome-extension/extension.js
git -C /home/deniz/Documents/Projects/AsusMouseBattery commit -m "feat(extension): add top-bar indicator with icon, label, and colour"
```

---

## Task 5: Popup menu + complete update wiring

**Files:**
- Modify: `gnome-extension/extension.js` (add popup rows, read all 7 properties in `_update`)

- [ ] **Step 1: Replace extension.js with full final version**

```javascript
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import St from 'gi://St';

const DBUS_NAME  = 'com.mousewatch.Battery';
const DBUS_PATH  = '/com/mousewatch/Battery/device0';
const DBUS_IFACE = 'com.mousewatch.Battery1';

function _formatStatus(status) {
    const labels = {
        'charging': 'Charging',
        'discharging': 'Discharging',
        'fully-charged': 'Fully charged',
        'unknown': 'Unknown',
    };
    return labels[status] ?? status;
}

function _formatTime(status, timeToFull, timeToEmpty) {
    if (status === 'fully-charged' || status === 'unknown') return '—';
    const seconds = status === 'charging' ? timeToFull : timeToEmpty;
    const suffix = status === 'charging' ? 'to full' : 'to empty';
    if (seconds === 0) return 'Calculating…';
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    return h > 0 ? `${h}h ${m}m ${suffix}` : `${m}m ${suffix}`;
}

export default class AsusMouseBatteryExtension extends Extension {
    enable() {
        // Indicator button
        this._indicator = new PanelMenu.Button(0.0, 'ASUS Mouse Battery', false);

        this._box = new St.BoxLayout({style_class: 'panel-status-menu-box'});
        this._icon = new St.Icon({
            icon_name: 'battery-missing-symbolic',
            style_class: 'system-status-icon',
        });
        this._label = new St.Label({
            text: '–',
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._box.add_child(this._icon);
        this._box.add_child(this._label);
        this._indicator.add_child(this._box);

        // Popup menu rows
        this._nameItem = new PopupMenu.PopupMenuItem('', {reactive: false});
        this._indicator.menu.addMenuItem(this._nameItem);
        this._indicator.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this._statusItem = new PopupMenu.PopupMenuItem('', {reactive: false});
        this._timeItem = new PopupMenu.PopupMenuItem('', {reactive: false});
        this._voltageItem = new PopupMenu.PopupMenuItem('', {reactive: false});
        this._indicator.menu.addMenuItem(this._statusItem);
        this._indicator.menu.addMenuItem(this._timeItem);
        this._indicator.menu.addMenuItem(this._voltageItem);

        Main.panel.addToStatusArea(this.uuid, this._indicator);
        this._indicator.hide();

        // DBus proxy
        this._proxy = new Gio.DBusProxy({
            g_connection: Gio.DBus.session,
            g_name: DBUS_NAME,
            g_object_path: DBUS_PATH,
            g_interface_name: DBUS_IFACE,
            g_flags: Gio.DBusProxyFlags.DO_NOT_AUTO_START,
        });
        this._proxy.init(null);

        this._propsChangedId = this._proxy.connect('g-properties-changed', () => {
            this._update();
        });
        this._nameOwnerId = this._proxy.connect('notify::g-name-owner', () => {
            this._update();
        });

        this._update();
    }

    disable() {
        if (this._propsChangedId) {
            this._proxy.disconnect(this._propsChangedId);
            this._propsChangedId = null;
        }
        if (this._nameOwnerId) {
            this._proxy.disconnect(this._nameOwnerId);
            this._nameOwnerId = null;
        }
        this._proxy = null;
        this._indicator?.destroy();
        this._indicator = null;
    }

    _update() {
        if (!this._proxy.g_name_owner) {
            this._indicator.hide();
            return;
        }

        const isPresentVar = this._proxy.get_cached_property('IsPresent');
        if (!isPresentVar?.unpack()) {
            this._indicator.hide();
            return;
        }

        const pct    = this._proxy.get_cached_property('Percentage')?.unpack()  ?? 0;
        const status = this._proxy.get_cached_property('Status')?.unpack()      ?? 'unknown';
        const ttf    = this._proxy.get_cached_property('TimeToFull')?.unpack()  ?? 0;
        const tte    = this._proxy.get_cached_property('TimeToEmpty')?.unpack() ?? 0;
        const vmv    = this._proxy.get_cached_property('VoltageMv')?.unpack()   ?? 0;
        const name   = this._proxy.get_cached_property('DeviceName')?.unpack()  ?? '';

        // Icon
        let iconName;
        if (status === 'fully-charged') {
            iconName = 'battery-full-charged-symbolic';
        } else if (pct > 50) {
            iconName = 'battery-full-symbolic';
        } else if (pct > 10) {
            iconName = 'battery-good-symbolic';
        } else {
            iconName = 'battery-caution-symbolic';
        }
        this._icon.icon_name = iconName;

        // Colour class
        ['battery-green', 'battery-orange', 'battery-red'].forEach(c =>
            this._box.remove_style_class_name(c));
        if (status === 'fully-charged' || pct > 50) {
            this._box.add_style_class_name('battery-green');
        } else if (pct > 10) {
            this._box.add_style_class_name('battery-orange');
        } else {
            this._box.add_style_class_name('battery-red');
        }

        // Top-bar label
        this._label.text = `${pct}%`;

        // Popup rows
        this._nameItem.label.text    = name;
        this._statusItem.label.text  = `Status:   ${_formatStatus(status)}`;
        this._timeItem.label.text    = `Time:     ${_formatTime(status, ttf, tte)}`;
        this._voltageItem.label.text = `Voltage:  ${vmv} mV`;

        this._indicator.show();
    }
}
```

- [ ] **Step 2: Reload the extension**

```bash
gnome-extensions disable asus-mouse-battery-icon@gnome
gnome-extensions enable asus-mouse-battery-icon@gnome
```

- [ ] **Step 3: Click the indicator and verify popup content**

Expected popup:
```
TUF GAMING MINI WL MOUSE MIKU
────────────────────────────────
Status:   Discharging
Time:     Xh Ym to empty   (or "Calculating…" on first run)
Voltage:  XXXX mV
```

- [ ] **Step 4: Verify time row updates after 2 poll cycles (60 seconds)**

Wait 60–70 seconds. Click the popup again.

Expected: `Time:` row now shows a real estimate (e.g. `4h 12m to empty`) instead of `Calculating…`.

- [ ] **Step 5: Check journalctl for no errors**

```bash
journalctl --user -b -g 'gnome-shell' --no-pager | grep -i 'error\|TypeError\|JS ERROR' | tail -10
```

Expected: no output.

- [ ] **Step 6: Commit**

```bash
git -C /home/deniz/Documents/Projects/AsusMouseBattery add gnome-extension/extension.js
git -C /home/deniz/Documents/Projects/AsusMouseBattery commit -m "feat(extension): add popup menu and full property update wiring"
```

---

## Task 6: End-to-end verification

No new code. Verify all states defined in the spec.

- [ ] **Step 1: Verify normal discharging state**

Mouse on wireless, daemon running.

Top bar: green battery icon + percentage (e.g. `40%`).
Click: popup shows device name, `Status: Discharging`, time to empty, voltage.

- [ ] **Step 2: Verify charging state**

Plug USB cable into mouse. Wait up to 35 seconds for next daemon poll.

Top bar: green icon + percentage (charging should be ≥ current level, cable gives ≥4100 mV).
Click: popup shows `Status: Charging`, `Time: Xh Ym to full`.

- [ ] **Step 3: Verify colour thresholds with busctl override**

Use busctl to simulate low battery by inspecting current values. The actual threshold test requires patience (or adjusting the daemon's threshold temporarily). Verify by reading the current percentage and confirming the icon/colour match the thresholds:

```bash
busctl --user get-property com.mousewatch.Battery \
  /com/mousewatch/Battery/device0 com.mousewatch.Battery1 Percentage
```

If current value is > 50 → icon should be green + `battery-full-symbolic`.
If 11–50 → orange + `battery-good-symbolic`.
If ≤ 10 → red + `battery-caution-symbolic`.

- [ ] **Step 4: Verify daemon stop → indicator hides**

```bash
systemctl --user stop mouse-battery
```

Expected: indicator disappears from top bar immediately (within one second of the name disappearing from the bus).

- [ ] **Step 5: Verify daemon start → indicator reappears**

```bash
systemctl --user start mouse-battery
```

Expected: indicator reappears within ~35 seconds (after first daemon poll emits `PropertiesChanged`).

- [ ] **Step 6: Verify GNOME Shell has no leaked resources after disable/enable cycle**

```bash
gnome-extensions disable asus-mouse-battery-icon@gnome
gnome-extensions enable asus-mouse-battery-icon@gnome
journalctl --user -b -g 'gnome-shell' --no-pager | grep -i 'error\|leak\|warn' | tail -10
```

Expected: no error or leak warnings.

- [ ] **Step 7: Final commit**

```bash
git -C /home/deniz/Documents/Projects/AsusMouseBattery add docs/
git -C /home/deniz/Documents/Projects/AsusMouseBattery commit -m "docs: add gnome extension design spec and implementation plan"
```
