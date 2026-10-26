import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import Pango from 'gi://Pango';
import St from 'gi://St';

import {
    batteryIconName, batteryColorClass, accentClass, formatStatus,
    formatEstimate, formatVoltage, litSegments,
} from './format.js';

const DBUS_NAME  = 'com.mousewatch.Battery';
const DBUS_PATH  = '/com/mousewatch/Battery/device0';
const DBUS_IFACE = 'com.mousewatch.Battery1';

const POPUP_WIDTH = 380;
const SEGMENTS = 20;
const ACCENTS = ['mouse-accent-green', 'mouse-accent-orange', 'mouse-accent-red'];

function setAccent(actor, accent) {
    ACCENTS.forEach(c => actor.remove_style_class_name(c));
    actor.add_style_class_name(accent);
}

function makeLabel(text, styleClass) {
    const label = new St.Label({text, style_class: styleClass, y_align: Clutter.ActorAlign.CENTER});
    // Never cut text short with an ellipsis; the rows have room.
    label.clutter_text.set_ellipsize(Pango.EllipsizeMode.NONE);
    return label;
}

function makeSpacer() {
    return new St.Widget({x_expand: true});
}

function makeRow(...children) {
    const row = new St.BoxLayout({x_expand: true});
    children.forEach(c => row.add_child(c));
    return row;
}

export default class AsusMouseBatteryExtension extends Extension {
    enable() {
        // Indicator button
        this._indicator = new PanelMenu.Button(0.5, 'ASUS Mouse Battery', false);

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

        this._buildPopup(this._indicator.menu);

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
            (_obj, res) => {
                try {
                    this._proxy = Gio.DBusProxy.new_for_bus_finish(res);
                } catch (e) {
                    console.error('[asus-mouse-battery] proxy creation failed:', e);
                    this._proxy = null;
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
        this._w = null;
    }

    /** Header, battery card and footer, as one non-interactive item. */
    _buildPopup(menu) {
        // GNOME draws the popup in .popup-menu-content (20px radius); this class squares it
        // off and drops its padding so our own card layout supplies the spacing.
        menu.box.add_style_class_name('mouse-menu-box');

        const w = this._w = {};
        const pop = new St.BoxLayout({vertical: true, style_class: 'mouse-pop', width: POPUP_WIDTH});

        // Header: name + voltage on the left, status chip on the right
        const titles = new St.BoxLayout({vertical: true});
        w.name = makeLabel('', 'mouse-name');
        w.sub = makeLabel('', 'mouse-sub');
        titles.add_child(w.name);
        titles.add_child(w.sub);
        w.chip = makeLabel('', 'mouse-chip');
        pop.add_child(makeRow(titles, makeSpacer(), w.chip));

        // Battery card: big number left, 20 segments (5% each) right
        const cards = new St.BoxLayout({vertical: true, style_class: 'mouse-cards'});
        const battery = new St.BoxLayout({style_class: 'mouse-card'});
        const batteryText = new St.BoxLayout({vertical: true});
        batteryText.add_child(makeLabel('BATTERY', 'mouse-lbl'));
        w.big = new St.Label({style_class: 'mouse-big', y_align: Clutter.ActorAlign.CENTER});
        batteryText.add_child(w.big);
        w.estimate = makeLabel('', 'mouse-est');
        batteryText.add_child(w.estimate);
        w.segments = new St.BoxLayout({style_class: 'mouse-segments', y_align: Clutter.ActorAlign.CENTER});
        w.segs = [];
        for (let i = 0; i < SEGMENTS; i++) {
            const seg = new St.Widget({style_class: 'mouse-seg'});
            w.segs.push(seg);
            w.segments.add_child(seg);
        }
        battery.add_child(batteryText);
        battery.add_child(makeSpacer());
        battery.add_child(w.segments);
        cards.add_child(battery);
        pop.add_child(cards);

        w.footer = makeLabel('', 'mouse-footer');
        pop.add_child(w.footer);

        const item = new PopupMenu.PopupBaseMenuItem({reactive: false, can_focus: false});
        item.add_style_class_name('mouse-menu-item');
        item.add_child(pop);
        menu.addMenuItem(item);
    }

    _update() {
        try {
            if (!this._proxy || !this._indicator) return;
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

            // Top bar
            this._icon.icon_name = batteryIconName(pct, status);
            ['battery-green', 'battery-orange', 'battery-red'].forEach(c =>
                this._box.remove_style_class_name(c));
            this._box.add_style_class_name(batteryColorClass(pct, status));
            this._label.text = `${pct}%`;

            // Popup
            const w = this._w;
            const accent = accentClass(pct, status);
            w.name.text = name;
            w.sub.text = formatVoltage(vmv);
            w.chip.text = formatStatus(status);
            setAccent(w.chip, accent);
            w.big.clutter_text.set_markup(`${pct}<span size="smaller">%</span>`);
            setAccent(w.segments, accent);
            const lit = litSegments(pct);
            w.segs.forEach((seg, i) => {
                if (i < lit)
                    seg.add_style_class_name('mouse-seg-on');
                else
                    seg.remove_style_class_name('mouse-seg-on');
            });
            w.estimate.text = formatEstimate(status, tte, ttf);

            const daemonVer = this._proxy.get_cached_property('DaemonVersion')?.unpack() ?? '—';
            const extVer = this.metadata['version-name'] ?? this.metadata.version;
            w.footer.text = `Daemon v${daemonVer}  ·  Extension v${extVer}`;

            this._indicator.show();
        } catch (e) {
            console.error('[asus-mouse-battery] _update error:', e);
        }
    }
}
