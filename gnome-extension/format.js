// Pure helpers, no Shell imports, so they can be tested with `gjs -m test-format.js`.

export function formatStatus(status) {
    const labels = {
        'charging': 'Charging',
        'discharging': 'Discharging',
        'fully-charged': 'Fully charged',
        'unknown': 'Unknown',
    };
    return labels[status] ?? status;
}

export function batteryIconName(pct, status) {
    if (status === 'fully-charged')
        return 'battery-full-charged-symbolic';
    const level = pct > 50 ? 'full' : pct > 10 ? 'good' : 'caution';
    return status === 'charging'
        ? `battery-${level}-charging-symbolic`
        : `battery-${level}-symbolic`;
}

export function batteryColorClass(pct, status) {
    if (status === 'fully-charged' || pct > 50) return 'battery-green';
    return pct > 10 ? 'battery-orange' : 'battery-red';
}

/** Accent for the popup chip and battery segments: green while charging or full. */
export function accentClass(pct, status) {
    if (status === 'charging' || status === 'fully-charged') return 'mouse-accent-green';
    if (pct > 50) return 'mouse-accent-green';
    return pct > 10 ? 'mouse-accent-orange' : 'mouse-accent-red';
}

function formatDuration(seconds) {
    const d = Math.floor(seconds / 86400);
    const h = Math.floor((seconds % 86400) / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    if (d > 0) return `${d}d ${h}h ${m}m`;
    return h > 0 ? `${h}h ${m}m` : `${Math.max(m, 1)}m`;
}

/** Line under the battery number. 0 means the daemon has no estimate yet. */
export function formatEstimate(status, timeToEmpty, timeToFull) {
    if (status === 'fully-charged') return 'Fully charged';
    if (status === 'charging')
        return timeToFull > 0 ? `${formatDuration(timeToFull)} to full` : 'Calculating…';
    if (status === 'discharging')
        return timeToEmpty > 0 ? `${formatDuration(timeToEmpty)} to empty` : 'Calculating…';
    return '';
}

/** Header sub-line: battery voltage when the daemon reports it. */
export function formatVoltage(mv) {
    return mv > 0 ? `${(mv / 1000).toFixed(2)} V` : 'Connected';
}

/** Lit segments out of 20 (one per 5%). */
export function litSegments(pct) {
    return Math.min(Math.max(Math.ceil(pct / 5), 0), 20);
}
