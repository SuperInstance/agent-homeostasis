//! Internal state gauges — the dashboard indicators of an agent.
//!
//! A **gauge** tracks a single internal state variable, like a dashboard indicator
//! in a car (temperature, fuel, RPM). Each gauge has a current value, a setpoint
//! (acceptable range), and a history of recent readings.
//!
//! # Biological Analogy
//!
/// Just as baroreceptors track blood pressure and chemoreceptors track blood CO₂,
/// gauges are the sensory receptors of an AI agent. They don't make decisions —
/// they *measure*.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::setpoint::Setpoint;

/// An internal state gauge tracking a single variable.
///
/// # Type Parameter
///
/// `T` is the numeric type of the gauge value (typically `f64`).
///
/// # History
///
/// The gauge maintains a history of readings with timestamps, enabling
/// rate-of-change calculations and trend detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gauge<T: Copy + PartialOrd> {
    /// Human-readable name (e.g., "accuracy", "latency_ms")
    pub name: String,
    /// Current value
    pub value: T,
    /// Target range
    pub setpoint: Setpoint<T>,
    /// History of readings with timestamps
    #[serde(skip)]
    pub history: Vec<(Instant, T)>,
    /// Maximum history length
    pub max_history: usize,
}

impl<T: Copy + PartialOrd> Gauge<T> {
    /// Create a new gauge with a name, initial value, and setpoint.
    pub fn new(name: impl Into<String>, value: T, setpoint: Setpoint<T>) -> Self {
        let mut gauge = Self {
            name: name.into(),
            value,
            setpoint,
            history: Vec::new(),
            max_history: 1000,
        };
        gauge.record(value);
        gauge
    }

    /// Record a new reading.
    pub fn record(&mut self, value: T) {
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push((Instant::now(), value));
        self.value = value;
    }

    /// Update the current value and record it in history.
    pub fn update(&mut self, value: T) {
        self.record(value);
    }

    /// Check if the current value is within bounds.
    pub fn is_healthy(&self) -> bool {
        self.setpoint.contains(self.value)
    }

    /// Check if the current value is below minimum.
    pub fn is_low(&self) -> bool {
        self.setpoint.is_below(self.value)
    }

    /// Check if the current value is above maximum.
    pub fn is_high(&self) -> bool {
        self.setpoint.is_above(self.value)
    }

    /// Clear the history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

impl Gauge<f64> {
    /// Compute the rate of change over recent history.
    ///
    /// Returns the average rate of change per second over the last N readings.
    /// Returns 0.0 if there are fewer than 2 readings.
    pub fn rate_of_change(&self) -> f64 {
        if self.history.len() < 2 {
            return 0.0;
        }

        let (t1, v1) = self.history.first().unwrap();
        let (t2, v2) = self.history.last().unwrap();

        let dt = t2.duration_since(*t1).as_secs_f64();
        if dt == 0.0 {
            return 0.0;
        }

        (v2 - v1) / dt
    }

    /// Compute satisfaction score (delegates to setpoint).
    pub fn satisfaction(&self) -> f64 {
        self.setpoint.satisfaction(self.value)
    }

    /// Get the error from ideal.
    pub fn error(&self) -> f64 {
        self.value - self.setpoint.ideal
    }

    /// Pre-built gauge types for common agent metrics.

    /// Accuracy gauge (0.0 to 1.0, ideal 0.95).
    pub fn accuracy(value: f64) -> Self {
        Self::new("accuracy", value, Setpoint::new(0.7, 1.0, 0.95))
    }

    /// Latency gauge in milliseconds (ideal 100ms).
    pub fn latency_ms(value: f64) -> Self {
        Self::new("latency_ms", value, Setpoint::new(10.0, 500.0, 100.0))
    }

    /// Token usage gauge (tokens per request).
    pub fn token_usage(value: f64) -> Self {
        Self::new("token_usage", value, Setpoint::new(100.0, 4000.0, 1000.0))
    }

    /// Error rate gauge (0.0 to 1.0).
    pub fn error_rate(value: f64) -> Self {
        Self::new("error_rate", value, Setpoint::new(0.0, 0.1, 0.01))
    }

    /// Memory pressure gauge (0.0 to 1.0, fraction of available memory).
    pub fn memory_pressure(value: f64) -> Self {
        Self::new("memory_pressure", value, Setpoint::new(0.0, 0.8, 0.4))
    }

    /// Temperature gauge — creativity/exploration parameter (0.0 to 2.0).
    pub fn temperature(value: f64) -> Self {
        Self::new("temperature", value, Setpoint::new(0.0, 1.5, 0.7))
    }
}
