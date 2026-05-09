import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

export default class AsusMouseBatteryExtension extends Extension {
    enable() {
        console.log('[asus-mouse-battery] enabled');
    }

    disable() {
        console.log('[asus-mouse-battery] disabled');
    }
}
