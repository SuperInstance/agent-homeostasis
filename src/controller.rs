//! PID controllers — the feedback correction engine.
//!
//! A **PID controller** computes a corrective signal based on three components:
//!
//! - **P**roportional: How far are we from the setpoint? (immediate error)
//! - **I**ntegral: How much error has accumulated over time? (persistent drift)
//! - **D**erivative: How fast is the error changing? (momentum/trend)
//!
//! # Biological Analogy
//!
//! Your pancreas acts as a PID controller for blood sugar:
//! - **P**: Current glucose level vs. target → immediate insulin release
//! - **I**: Persistent high glucose over hours → sustained insulin production
//! - **D**: Rapid rise in glucose → aggressive early response to prevent spikes
//!
//! # From Scratch
//!
//! No external math dependencies. The PID formula:
//!
//! ```text
//! output = Kp * error + Ki * integral + Kd * derivative
//! ```

use serde::{Deserialize, Serialize};

/// A Proportional-Integral-Derivative controller.
///
/// Tuning parameters:
/// - `kp` (proportional gain): How aggressively to respond to current error
/// - `ki` (integral gain): How aggressively to correct accumulated error
/// - `kd` (derivative gain): How aggressively to dampen rate of change
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PIDController {
    /// Proportional gain
    pub kp: f64,
    /// Integral gain
    pub ki: f64,
    /// Derivative gain
    pub kd: f64,
    /// Accumulated integral of error
    pub integral: f64,
    /// Previous error (for derivative calculation)
    pub prev_error: f64,
    /// Maximum integral term (anti-windup)
    pub integral_limit: f64,
    /// Output limits
    pub output_min: f64,
    pub output_max: f64,
}

impl PIDController {
    /// Create a new PID controller with given gains.
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            prev_error: 0.0,
            integral_limit: 100.0,
            output_min: -1.0,
            output_max: 1.0,
        }
    }

    /// Create a proportional-only controller.
    pub fn proportional_only(kp: f64) -> Self {
        Self::new(kp, 0.0, 0.0)
    }

    /// Create a PI controller (no derivative).
    pub fn pi(kp: f64, ki: f64) -> Self {
        Self::new(kp, ki, 0.0)
    }

    /// Create a well-tuned controller for typical agent gauges.
    ///
    /// Conservative tuning that avoids oscillation.
    pub fn default_tuned() -> Self {
        Self::new(0.5, 0.1, 0.2)
    }

    /// Set the integral anti-windup limit.
    pub fn with_integral_limit(mut self, limit: f64) -> Self {
        self.integral_limit = limit;
        self
    }

    /// Set output limits.
    pub fn with_output_limits(mut self, min: f64, max: f64) -> Self {
        self.output_min = min;
        self.output_max = max;
        self
    }

    /// Compute the PID output for a given error.
    ///
    /// The error is `current_value - setpoint_ideal`.
    ///
    /// Returns the control output (correction strength) and the individual
    /// term components for debugging.
    pub fn compute(&mut self, error: f64) -> PIDOutput {
        // Proportional term
        let p_term = self.kp * error;

        // Integral term (with anti-windup)
        self.integral += error;
        self.integral = self.integral.clamp(-self.integral_limit, self.integral_limit);
        let i_term = self.ki * self.integral;

        // Derivative term
        let derivative = error - self.prev_error;
        let d_term = self.kd * derivative;
        self.prev_error = error;

        // Combined output (clamped)
        let output = (p_term + i_term + d_term).clamp(self.output_min, self.output_max);

        PIDOutput {
            output,
            p_term,
            i_term,
            d_term,
        }
    }

    /// Compute PID output with a time step (for more accurate integral/derivative).
    ///
    /// `dt` is in seconds.
    pub fn compute_with_dt(&mut self, error: f64, dt: f64) -> PIDOutput {
        if dt <= 0.0 {
            return PIDOutput {
                output: 0.0,
                p_term: 0.0,
                i_term: 0.0,
                d_term: 0.0,
            };
        }

        let p_term = self.kp * error;

        self.integral += error * dt;
        self.integral = self.integral.clamp(-self.integral_limit, self.integral_limit);
        let i_term = self.ki * self.integral;

        let derivative = (error - self.prev_error) / dt;
        let d_term = self.kd * derivative;
        self.prev_error = error;

        let output = (p_term + i_term + d_term).clamp(self.output_min, self.output_max);

        PIDOutput {
            output,
            p_term,
            i_term,
            d_term,
        }
    }

    /// Reset the controller state (integral and previous error).
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_error = 0.0;
    }
}

/// Output of a PID computation, with individual terms for introspection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PIDOutput {
    /// Total control output
    pub output: f64,
    /// Proportional term contribution
    pub p_term: f64,
    /// Integral term contribution
    pub i_term: f64,
    /// Derivative term contribution
    pub d_term: f64,
}

impl Default for PIDOutput {
    fn default() -> Self {
        Self {
            output: 0.0,
            p_term: 0.0,
            i_term: 0.0,
            d_term: 0.0,
        }
    }
}
