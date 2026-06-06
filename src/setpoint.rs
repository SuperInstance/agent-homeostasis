//! Target ranges for gauge values.
//!
//! A **setpoint** defines the acceptable operating range for a gauge, analogous
//! to how the human body maintains a setpoint of ~37°C for core temperature.
//!
//! Each setpoint has:
//! - `min` — hard lower bound (below this is critical)
//! - `max` — hard upper bound (above this is critical)
//! - `ideal` — the target value the system optimizes toward

use serde::{Deserialize, Serialize};

/// Target range for a gauge value.
///
/// # Biological Analogy
///
/// Like the hypothalamus maintaining body temperature: it doesn't target a
/// single value but an acceptable range (36.1–37.2°C) with an ideal of 37°C.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Setpoint<T: Copy + PartialOrd> {
    /// Hard lower bound — below this is critical
    pub min: T,
    /// Hard upper bound — above this is critical
    pub max: T,
    /// Ideal target value
    pub ideal: T,
}

impl<T: Copy + PartialOrd> Setpoint<T> {
    /// Create a new setpoint with min, max, and ideal values.
    pub fn new(min: T, max: T, ideal: T) -> Self {
        Self { min, max, ideal }
    }

    /// Create a symmetric setpoint where ideal is the midpoint.
    pub fn symmetric(center: T, half_range: T) -> Self
    where
        T: std::ops::Sub<Output = T> + std::ops::Add<Output = T>,
    {
        Self {
            min: center - half_range,
            max: center + half_range,
            ideal: center,
        }
    }

    /// Check if a value is within the acceptable range.
    pub fn contains(&self, value: T) -> bool {
        value >= self.min && value <= self.max
    }

    /// Check if a value is below the minimum bound.
    pub fn is_below(&self, value: T) -> bool {
        value < self.min
    }

    /// Check if a value is above the maximum bound.
    pub fn is_above(&self, value: T) -> bool {
        value > self.max
    }
}

impl Setpoint<f64> {
    /// Compute satisfaction: 1.0 when at ideal, 0.0 at bounds, negative outside.
    ///
    /// Uses a quadratic falloff from ideal toward the bounds.
    pub fn satisfaction(&self, value: f64) -> f64 {
        if self.contains(value) {
            let range = self.max - self.min;
            if range == 0.0 {
                return if value == self.ideal { 1.0 } else { 0.0 };
            }
            let dist = (value - self.ideal).abs();
            let max_dist = ((self.max - self.ideal).abs()).max((self.ideal - self.min).abs());
            if max_dist == 0.0 {
                return 1.0;
            }
            1.0 - (dist / max_dist).powi(2)
        } else if value < self.min {
            let dist = self.min - value;
            let range = self.ideal - self.min;
            if range == 0.0 {
                return -1.0;
            }
            -(dist / range).min(1.0)
        } else {
            let dist = value - self.max;
            let range = self.max - self.ideal;
            if range == 0.0 {
                return -1.0;
            }
            -(dist / range).min(1.0)
        }
    }

    /// Compute a dynamic setpoint adjusted by a factor.
    ///
    /// The factor shifts the ideal and expands/contracts the range.
    /// A factor > 1.0 widens the range; < 1.0 narrows it.
    pub fn dynamic(&self, factor: f64) -> Self {
        let center = self.ideal;
        let half = (self.max - self.min) / 2.0 * factor;
        Self {
            min: center - half,
            max: center + half,
            ideal: center,
        }
    }
}

impl Default for Setpoint<f64> {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 1.0,
            ideal: 0.5,
        }
    }
}

/// A dynamic setpoint that adjusts based on external conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicSetpoint {
    /// Base setpoint
    pub base: Setpoint<f64>,
    /// Adjustment factors keyed by condition name
    pub adjustments: Vec<Adjustment>,
}

/// An adjustment to apply to a setpoint based on a condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adjustment {
    /// Name of the condition
    pub condition: String,
    /// Factor to apply to the range (1.0 = no change)
    pub range_factor: f64,
    /// Offset to apply to the ideal value
    pub ideal_offset: f64,
    /// Whether this adjustment is currently active
    pub active: bool,
}

impl DynamicSetpoint {
    /// Create a new dynamic setpoint from a base.
    pub fn new(base: Setpoint<f64>) -> Self {
        Self {
            base,
            adjustments: Vec::new(),
        }
    }

    /// Add an adjustment.
    pub fn add_adjustment(&mut self, adjustment: Adjustment) {
        self.adjustments.push(adjustment);
    }

    /// Activate an adjustment by name.
    pub fn activate(&mut self, condition: &str) {
        for adj in &mut self.adjustments {
            if adj.condition == condition {
                adj.active = true;
            }
        }
    }

    /// Deactivate an adjustment by name.
    pub fn deactivate(&mut self, condition: &str) {
        for adj in &mut self.adjustments {
            if adj.condition == condition {
                adj.active = false;
            }
        }
    }

    /// Resolve the current effective setpoint.
    pub fn resolve(&self) -> Setpoint<f64> {
        let mut factor = 1.0;
        let mut offset = 0.0;

        for adj in &self.adjustments {
            if adj.active {
                factor *= adj.range_factor;
                offset += adj.ideal_offset;
            }
        }

        let effective = self.base.dynamic(factor);
        Setpoint {
            min: effective.min + offset,
            max: effective.max + offset,
            ideal: effective.ideal + offset,
        }
    }
}
