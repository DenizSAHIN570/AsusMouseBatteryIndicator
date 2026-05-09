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
