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
