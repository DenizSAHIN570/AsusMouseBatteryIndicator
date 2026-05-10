use std::collections::VecDeque;
use std::time::Instant;

use crate::hid::BatteryStatus;

const WINDOW_CAPACITY: usize = 720;     // 1 hour at 5 s/poll
const MIN_WINDOW_SECS: f64 = 120.0;    // require ≥ 2 min of history before reporting
const RATE_ALPHA: f64 = 0.05;          // EWMA α for dV/dt (~20-sample effective window)
const CAL_ALPHA: f64 = 0.10;           // EWMA α for mV/pct self-calibration
const DEFAULT_MV_PER_PCT: f64 = 7.0;   // ~700 mV usable range / 100% (empirical, TUF Mini WL)
const MIN_RATE_PCT_PER_HR: f64 = 0.1;  // below this is noise
const MAX_RATE_PCT_PER_HR: f64 = 50.0; // above this is a spike
const WEAK_CABLE_MV: i32 = 30;

struct Sample {
    time: Instant,
    percentage: u8,
    voltage_mv: u16,
}

pub struct BatteryPredictor {
    window: VecDeque<Sample>,
    last_status: Option<BatteryStatus>,
    /// Smoothed dV/dt in mV/s. Negative = draining, positive = charging.
    rate_ema: Option<f64>,
    /// Self-calibrating mV per percentage point. Preserved across status resets so
    /// calibration accumulated while discharging survives a plug-in event.
    mv_per_pct: f64,
}

impl BatteryPredictor {
    pub fn new() -> Self {
        Self {
            window: VecDeque::with_capacity(WINDOW_CAPACITY),
            last_status: None,
            rate_ema: None,
            mv_per_pct: DEFAULT_MV_PER_PCT,
        }
    }

    pub fn push(&mut self, percentage: u8, voltage_mv: u16, status: &BatteryStatus) {
        let direction_changed = self
            .last_status
            .as_ref()
            .map(|prev| prev != status)
            .unwrap_or(false);

        if direction_changed {
            self.window.clear();
            self.rate_ema = None;
        }
        self.last_status = Some(status.clone());

        // Update mV/pct calibration: compare the incoming reading against the oldest
        // sample. Only update when both signals agree on direction and pct has moved
        // by at least 1 (i.e., a real percentage tick, not quantisation noise).
        if let Some(oldest) = self.window.front() {
            let delta_p = (percentage as i16) - (oldest.percentage as i16);
            let delta_v = (voltage_mv as i32) - (oldest.voltage_mv as i32);
            if delta_p.abs() >= 1 && (delta_p.signum() as i32) == delta_v.signum() {
                let observed = delta_v.unsigned_abs() as f64 / delta_p.unsigned_abs() as f64;
                self.mv_per_pct = CAL_ALPHA * observed + (1.0 - CAL_ALPHA) * self.mv_per_pct;
            }
        }

        if self.window.len() == WINDOW_CAPACITY {
            self.window.pop_front();
        }
        self.window.push_back(Sample { time: Instant::now(), percentage, voltage_mv });

        // Recompute regression slope and update EWMA.
        if let Some(slope) = self.voltage_slope() {
            self.rate_ema = Some(match self.rate_ema {
                Some(prev) => RATE_ALPHA * slope + (1.0 - RATE_ALPHA) * prev,
                None => slope,
            });
        }
    }

    /// Least-squares linear regression of voltage over time.
    /// Returns dV/dt in mV/s (negative when draining, positive when charging).
    fn voltage_slope(&self) -> Option<f64> {
        if self.window.len() < 2 {
            return None;
        }
        let t0 = self.window.front()?.time;
        let span = self.window.back()?.time.duration_since(t0).as_secs_f64();
        if span < 1.0 {
            return None;
        }

        let n = self.window.len() as f64;
        let (mut sum_t, mut sum_v, mut sum_tt, mut sum_tv) = (0.0f64, 0.0, 0.0, 0.0);
        for s in &self.window {
            let t = s.time.duration_since(t0).as_secs_f64();
            let v = s.voltage_mv as f64;
            sum_t += t;
            sum_v += v;
            sum_tt += t * t;
            sum_tv += t * v;
        }
        let denom = n * sum_tt - sum_t * sum_t;
        if denom.abs() < 1e-9 {
            return None;
        }
        Some((n * sum_tv - sum_t * sum_v) / denom)
    }

    /// Returns the validated |rate| in %/s, or None if the window is too short
    /// or the rate falls outside the plausible range.
    fn validated_rate_pct_per_s(&self) -> Option<f64> {
        let span = match (self.window.front(), self.window.back()) {
            (Some(f), Some(b)) => b.time.duration_since(f.time).as_secs_f64(),
            _ => 0.0,
        };
        if span < MIN_WINDOW_SECS {
            return None;
        }
        let rate_pct_per_s = self.rate_ema?.abs() / self.mv_per_pct;
        let rate_pct_per_hr = rate_pct_per_s * 3600.0;
        if !(MIN_RATE_PCT_PER_HR..=MAX_RATE_PCT_PER_HR).contains(&rate_pct_per_hr) {
            return None;
        }
        Some(rate_pct_per_s)
    }

    /// Seconds until empty. Returns 0 when insufficient data or not discharging.
    pub fn time_to_empty(&self) -> u32 {
        self.compute_time_to_empty().unwrap_or(0)
    }

    fn compute_time_to_empty(&self) -> Option<u32> {
        if self.rate_ema? >= 0.0 {
            return None;
        }
        let rate = self.validated_rate_pct_per_s()?;
        let pct = self.window.back()?.percentage as f64;
        Some((pct / rate) as u32)
    }

    /// Seconds until full. Returns 0 when insufficient data or not charging.
    pub fn time_to_full(&self) -> u32 {
        self.compute_time_to_full().unwrap_or(0)
    }

    fn compute_time_to_full(&self) -> Option<u32> {
        if self.rate_ema? <= 0.0 {
            return None;
        }
        let rate = self.validated_rate_pct_per_s()?;
        let pct = self.window.back()?.percentage as f64;
        Some(((100.0 - pct) / rate) as u32)
    }

    /// Returns true when the cable is confirmed delivering charge,
    /// false when voltage is consistently falling despite the cable,
    /// None when there is insufficient history to decide.
    pub fn cable_is_charging(&self) -> Option<bool> {
        if self.window.len() < 2 {
            return None;
        }
        let oldest = self.window.front()?;
        let newest = self.window.back()?;
        let delta_mv = (newest.voltage_mv as i32) - (oldest.voltage_mv as i32);
        Some(delta_mv >= -WEAK_CABLE_MV)
    }
}
