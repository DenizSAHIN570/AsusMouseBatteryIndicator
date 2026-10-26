// gnome-extension/test-format.js — run with: gjs -m gnome-extension/test-format.js
import {
    formatStatus, batteryIconName, batteryColorClass, accentClass,
    formatEstimate, formatVoltage, litSegments,
} from './format.js';

const TESTS = [
    [formatStatus('charging'),            'Charging'],
    [formatStatus('fully-charged'),       'Fully charged'],
    [formatStatus('discharging'),         'Discharging'],
    [formatStatus('unknown'),             'Unknown'],
    [formatStatus('weird'),               'weird'],
    [formatEstimate('charging', 0, 4980),     '1h 23m to full'],
    [formatEstimate('discharging', 1800, 0),  '30m to empty'],
    [formatEstimate('discharging', 90000, 0), '1d 1h 0m to empty'],
    [formatEstimate('charging', 0, 0),        'Calculating…'],
    [formatEstimate('discharging', 0, 0),     'Calculating…'],
    [formatEstimate('fully-charged', 0, 0),   'Fully charged'],
    [formatEstimate('unknown', 0, 0),         ''],
    [formatVoltage(4123),                 '4.12 V'],
    [formatVoltage(0),                    'Connected'],
    [batteryIconName(35, 'discharging'),  'battery-good-symbolic'],
    [batteryIconName(35, 'charging'),     'battery-good-charging-symbolic'],
    [batteryIconName(80, 'discharging'),  'battery-full-symbolic'],
    [batteryIconName(5, 'discharging'),   'battery-caution-symbolic'],
    [batteryIconName(5, 'charging'),      'battery-caution-charging-symbolic'],
    [batteryIconName(100, 'fully-charged'), 'battery-full-charged-symbolic'],
    [batteryColorClass(35, 'discharging'), 'battery-orange'],
    [batteryColorClass(80, 'discharging'), 'battery-green'],
    [batteryColorClass(10, 'discharging'), 'battery-red'],
    [batteryColorClass(10, 'fully-charged'), 'battery-green'],
    [accentClass(35, 'charging'),         'mouse-accent-green'],
    [accentClass(35, 'discharging'),      'mouse-accent-orange'],
    [accentClass(10, 'discharging'),      'mouse-accent-red'],
    [accentClass(80, 'discharging'),      'mouse-accent-green'],
    [litSegments(0),                      0],
    [litSegments(1),                      1],
    [litSegments(35),                     7],
    [litSegments(100),                    20],
    [litSegments(250),                    20],
];

let passed = 0;
for (const [got, expected] of TESTS) {
    if (got === expected) {
        passed++;
    } else {
        print(`FAIL: got "${got}", expected "${expected}"`);
    }
}
print(`${passed}/${TESTS.length} passed`);
if (passed !== TESTS.length) imports.system.exit(1);
