use std::collections::VecDeque;
use std::time::Instant;

use crate::hid::BatteryStatus;

struct Sample {
    time: Instant,
    percentage: u8,
    voltage_mv: u16,
}

pub struct BatteryPredictor {
    window: VecDeque<Sample>,
    last_status: Option<BatteryStatus>,
}

impl BatteryPredictor {
    const CAPACITY: usize = 10;

    /// If voltage drops more than this across the window while the cable is
    /// connected, we consider the cable too weak to deliver a net charge.
    const WEAK_CABLE_THRESHOLD_MV: i32 = 30;

    pub fn new() -> Self {
        Self {
            window: VecDeque::with_capacity(Self::CAPACITY),
            last_status: None,
        }
    }

    pub fn reset(&mut self) {
        self.window.clear();
        self.last_status = None;
    }

    /// Record a new reading. Clears the window when charging direction changes.
    pub fn push(&mut self, percentage: u8, voltage_mv: u16, status: &BatteryStatus) {
        let direction_changed = self
            .last_status
            .as_ref()
            .map(|prev| prev != status)
            .unwrap_or(false);

        if direction_changed {
            self.window.clear();
        }

        self.last_status = Some(status.clone());

        if self.window.len() == Self::CAPACITY {
            self.window.pop_front();
        }
        self.window.push_back(Sample { time: Instant::now(), percentage, voltage_mv });
    }

    /// Returns true when we have enough voltage history to confirm the cable is
    /// delivering charge. False means "not enough data yet" (caller should trust
    /// byte[9] optimistically). Never returns false just because the cable is absent.
    pub fn cable_is_charging(&self) -> Option<bool> {
        if self.window.len() < 2 {
            return None; // not enough history to decide
        }
        let oldest = self.window.front()?;
        let newest = self.window.back()?;
        let delta_mv = (newest.voltage_mv as i32) - (oldest.voltage_mv as i32);
        // Cable is delivering charge if voltage is rising or not dropping badly.
        Some(delta_mv >= -Self::WEAK_CABLE_THRESHOLD_MV)
    }

    /// Seconds until empty. Returns 0 when insufficient data or rate is non-positive.
    pub fn time_to_empty(&self) -> u32 {
        self.estimate_drain().unwrap_or(0)
    }

    /// Seconds until full. Returns 0 when insufficient data or rate is non-positive.
    pub fn time_to_full(&self) -> u32 {
        self.estimate_charge().unwrap_or(0)
    }

    fn endpoints(&self) -> Option<(Instant, u8, Instant, u8)> {
        if self.window.len() < 2 {
            return None;
        }
        let front = self.window.front()?;
        let back = self.window.back()?;
        Some((front.time, front.percentage, back.time, back.percentage))
    }

    fn estimate_drain(&self) -> Option<u32> {
        let (t0, p0, t1, p1) = self.endpoints()?;
        let delta_t = t1.duration_since(t0).as_secs_f64();
        let drain = (p0 as f64) - (p1 as f64);
        if delta_t <= 0.0 || drain <= 0.0 {
            return None;
        }
        Some((p1 as f64 / (drain / delta_t)) as u32)
    }

    fn estimate_charge(&self) -> Option<u32> {
        let (t0, p0, t1, p1) = self.endpoints()?;
        let delta_t = t1.duration_since(t0).as_secs_f64();
        let gain = (p1 as f64) - (p0 as f64);
        if delta_t <= 0.0 || gain <= 0.0 {
            return None;
        }
        let remaining = 100u8.saturating_sub(p1) as f64;
        Some((remaining / (gain / delta_t)) as u32)
    }
}
