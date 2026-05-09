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
