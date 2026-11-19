//! Simulation time: integer microseconds, so it never drifts.

use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct SimTime(pub u64);

impl SimTime {
    pub const ZERO: SimTime = SimTime(0);

    pub fn secs(self) -> f64 {
        self.0 as f64 / 1e6
    }

    pub fn plus(self, micros: u64) -> SimTime {
        SimTime(self.0 + micros)
    }
}

/// Converts seconds to microseconds. Negative durations become zero.
pub fn micros(secs: f64) -> u64 {
    (secs.max(0.0) * 1e6).round() as u64
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TickConfig {
    /// Length of one server tick.
    pub tick_us: u64,
    /// Round every scheduled time up to a tick boundary.
    pub snap: bool,
}

impl Default for TickConfig {
    fn default() -> Self {
        TickConfig {
            tick_us: 33_333,
            snap: true,
        }
    }
}

impl TickConfig {
    pub fn snap_up(&self, t: SimTime) -> SimTime {
        if !self.snap {
            return t;
        }
        SimTime(t.0.div_ceil(self.tick_us) * self.tick_us)
    }
}
