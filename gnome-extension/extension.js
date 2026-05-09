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

        // DBus proxy — async so we never block the compositor's main loop
        Gio.DBusProxy.new_for_bus(
            Gio.BusType.SESSION,
            Gio.DBusProxyFlags.DO_NOT_AUTO_START,
            null,
            DBUS_NAME,
            DBUS_PATH,
            DBUS_IFACE,
            null,
            (obj, res) => {
                try {
                    this._proxy = Gio.DBusProxy.new_for_bus_finish(res);
                } catch (e) {
                    console.error('[asus-mouse-battery] proxy creation failed:', e);
                    return;
                }

                if (!this._indicator)
                    return; // extension was disabled before proxy completed

                this._propsChangedId = this._proxy.connect('g-properties-changed', () => {
                    this._update();
                });
                this._nameOwnerId = this._proxy.connect('notify::g-name-owner', () => {
                    this._update();
                });

                this._update();
            }
        );
    }

    disable() {
        if (this._proxy) {
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
        this._indicator?.destroy();
        this._indicator = null;
    }

    _update() {
        try {
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
            this._statusItem.label.text  = `Status: ${_formatStatus(status)}`;
            this._timeItem.label.text    = `Time: ${_formatTime(status, ttf, tte)}`;
            this._voltageItem.label.text = `Voltage: ${vmv > 0 ? `${vmv} mV` : '—'}`;

            this._indicator.show();
        } catch (e) {
            console.error('[asus-mouse-battery] _update error:', e);
        }
    }
}
