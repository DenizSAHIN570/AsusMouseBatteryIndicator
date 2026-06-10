//! Percentage-only notification state machine.
//!
//! Decides when to fire the low-battery and fully-charged desktop
//! notifications based solely on the reported battery percentage. The
//! charging/discharging status byte is deliberately ignored: the firmware's
//! status reporting is unreliable (see README), and percentage alone is
//! sufficient — 100% can only be reached while charging, and recovery above
//! the re-arm thresholds implies a charge happened.

/// Fire the low notification at or below this percentage.
const LOW_THRESHOLD: u8 = 10;
/// Re-arm the low notification once charge recovers above this percentage.
const LOW_REARM: u8 = 20;
/// Fire the full notification at this percentage.
const FULL_THRESHOLD: u8 = 100;
/// Re-arm the full notification once charge drops below this percentage,
/// with margin so 99↔100 jitter near the end of a charge can't re-fire it.
const FULL_REARM: u8 = 95;

#[derive(Debug, PartialEq, Eq)]
pub struct Notifications {
    pub low: bool,
    pub full: bool,
}

pub struct NotificationPolicy {
    low_notified: bool,
    full_notified: bool,
}

impl NotificationPolicy {
    pub fn new() -> Self {
        Self {
            low_notified: false,
            full_notified: false,
        }
    }

    /// Feed one battery reading; returns which notifications to fire now.
    pub fn update(&mut self, percentage: u8) -> Notifications {
        let low = percentage <= LOW_THRESHOLD && !self.low_notified;
        if low {
            self.low_notified = true;
        } else if percentage > LOW_REARM {
            self.low_notified = false;
        }

        let full = percentage >= FULL_THRESHOLD && !self.full_notified;
        if full {
            self.full_notified = true;
        } else if percentage < FULL_REARM {
            self.full_notified = false;
        }

        Notifications { low, full }
    }

    /// Forget notification state (device disconnected / query failures).
    pub fn reset(&mut self) {
        self.low_notified = false;
        self.full_notified = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_fire() -> Notifications {
        Notifications { low: false, full: false }
    }

    #[test]
    fn fires_low_at_threshold() {
        let mut p = NotificationPolicy::new();
        assert_eq!(p.update(10), Notifications { low: true, full: false });
    }

    #[test]
    fn fires_low_below_threshold() {
        let mut p = NotificationPolicy::new();
        assert_eq!(p.update(7), Notifications { low: true, full: false });
    }

    #[test]
    fn does_not_fire_low_above_threshold() {
        let mut p = NotificationPolicy::new();
        assert_eq!(p.update(11), no_fire());
        assert_eq!(p.update(50), no_fire());
    }

    #[test]
    fn fires_low_only_once_while_low() {
        let mut p = NotificationPolicy::new();
        assert_eq!(p.update(10).low, true);
        assert_eq!(p.update(9), no_fire());
        assert_eq!(p.update(8), no_fire());
    }

    #[test]
    fn low_does_not_rearm_in_hysteresis_band() {
        let mut p = NotificationPolicy::new();
        assert_eq!(p.update(10).low, true);
        // Recovers into the 11–20 band: must NOT re-arm.
        assert_eq!(p.update(15), no_fire());
        assert_eq!(p.update(10), no_fire());
    }

    #[test]
    fn low_rearms_after_recovery_above_rearm_threshold() {
        let mut p = NotificationPolicy::new();
        assert_eq!(p.update(10).low, true);
        assert_eq!(p.update(21), no_fire());
        assert_eq!(p.update(10).low, true);
    }

    #[test]
    fn fires_full_at_100() {
        let mut p = NotificationPolicy::new();
        assert_eq!(p.update(100), Notifications { low: false, full: true });
    }

    #[test]
    fn fires_full_only_once_while_full() {
        let mut p = NotificationPolicy::new();
        assert_eq!(p.update(100).full, true);
        assert_eq!(p.update(100), no_fire());
        assert_eq!(p.update(100), no_fire());
    }

    #[test]
    fn full_does_not_rearm_on_jitter_near_full() {
        let mut p = NotificationPolicy::new();
        assert_eq!(p.update(100).full, true);
        // 99↔100 jitter near end of charge must not re-fire.
        assert_eq!(p.update(99), no_fire());
        assert_eq!(p.update(100), no_fire());
        assert_eq!(p.update(97), no_fire());
        assert_eq!(p.update(100), no_fire());
    }

    #[test]
    fn full_rearms_after_dropping_below_rearm_threshold() {
        let mut p = NotificationPolicy::new();
        assert_eq!(p.update(100).full, true);
        assert_eq!(p.update(94), no_fire());
        assert_eq!(p.update(100).full, true);
    }

    #[test]
    fn reset_rearms_both() {
        let mut p = NotificationPolicy::new();
        assert_eq!(p.update(10).low, true);
        p.reset();
        assert_eq!(p.update(10).low, true);

        let mut p = NotificationPolicy::new();
        assert_eq!(p.update(100).full, true);
        p.reset();
        assert_eq!(p.update(100).full, true);
    }

    #[test]
    fn drain_then_charge_full_cycle() {
        let mut p = NotificationPolicy::new();
        // Draining: 50 → 10 fires low once.
        assert_eq!(p.update(50), no_fire());
        assert_eq!(p.update(25), no_fire());
        assert_eq!(p.update(10).low, true);
        assert_eq!(p.update(9), no_fire());
        // Plugged in, charging up (status irrelevant — percentage only).
        assert_eq!(p.update(30), no_fire());
        assert_eq!(p.update(80), no_fire());
        // Hits full: fires full once.
        assert_eq!(p.update(100).full, true);
        assert_eq!(p.update(100), no_fire());
        // Unplugged, drains again: low fires again at threshold.
        assert_eq!(p.update(60), no_fire());
        assert_eq!(p.update(10).low, true);
    }
}
