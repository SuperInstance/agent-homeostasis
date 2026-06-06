//! Corrective actions — what the agent does when gauges drift.
//!
//! When the feedback loop detects a gauge out of bounds, it selects and fires
//! a corrective action. Each action has conditions (when to fire) and strength
//! (how aggressively to apply).
//!
//! # Biological Analogy
//!
//! When body temperature rises, the body sweats (cooling action). When it drops,
//! it shivers (heating action). These are corrective actions with variable
//! intensity based on severity.

use serde::{Deserialize, Serialize};

/// The type of corrective action to take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    /// Reduce request rate (throttle throughput)
    ThrottleRequests,
    /// Switch to a different model (e.g., smaller/faster)
    SwitchModel,
    /// Reduce context window or conversation history
    ReduceContext,
    /// Increase parallel processing
    IncreaseParallelism,
    /// Pause the agent and allow recovery (garbage collect, flush caches)
    PauseAndRecover,
    /// Escalate to a human operator
    EscalateToHuman,
}

impl Action {
    /// Get a human-readable name for the action.
    pub fn name(&self) -> &'static str {
        match self {
            Action::ThrottleRequests => "throttle_requests",
            Action::SwitchModel => "switch_model",
            Action::ReduceContext => "reduce_context",
            Action::IncreaseParallelism => "increase_parallelism",
            Action::PauseAndRecover => "pause_and_recover",
            Action::EscalateToHuman => "escalate_to_human",
        }
    }

    /// Get a description of the action.
    pub fn description(&self) -> &'static str {
        match self {
            Action::ThrottleRequests => "Reduce the rate of incoming requests to lower load",
            Action::SwitchModel => "Switch to a lighter/faster model to reduce resource usage",
            Action::ReduceContext => "Trim conversation history and context window",
            Action::IncreaseParallelism => "Distribute work across more parallel workers",
            Action::PauseAndRecover => "Temporarily pause and perform recovery (GC, cache flush)",
            Action::EscalateToHuman => "Alert a human operator for intervention",
        }
    }

    /// Get the default severity level for this action.
    pub fn default_severity(&self) -> f64 {
        match self {
            Action::ThrottleRequests => 0.3,
            Action::SwitchModel => 0.5,
            Action::ReduceContext => 0.4,
            Action::IncreaseParallelism => 0.3,
            Action::PauseAndRecover => 0.7,
            Action::EscalateToHuman => 1.0,
        }
    }
}

/// A rule that maps gauge conditions to corrective actions.
///
/// When a gauge's error exceeds a threshold, the rule fires its action
/// with a strength proportional to the severity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRule {
    /// Which gauge this rule monitors
    pub gauge_name: String,
    /// The action to take
    pub action: Action,
    /// Error threshold at which to fire (0.0 to 1.0)
    pub threshold: f64,
    /// Whether to fire when gauge is above range (true) or below (false)
    pub fire_on_high: bool,
    /// Maximum strength of the action (0.0 to 1.0)
    pub max_strength: f64,
}

impl ActionRule {
    /// Create a new action rule.
    pub fn new(gauge_name: impl Into<String>, action: Action, threshold: f64, fire_on_high: bool) -> Self {
        Self {
            gauge_name: gauge_name.into(),
            action,
            threshold,
            fire_on_high,
            max_strength: 1.0,
        }
    }

    /// Check if this rule should fire for the given gauge error and value.
    ///
    /// Returns the strength if it should fire, None otherwise.
    pub fn should_fire(&self, satisfaction: f64, value: f64, ideal: f64) -> Option<f64> {
        let is_high = value > ideal;

        // Check direction matches
        if is_high != self.fire_on_high {
            return None;
        }

        // Satisfaction is negative when out of bounds, 0-1 when in bounds
        let severity = if satisfaction < 0.0 {
            satisfaction.abs()
        } else {
            // In bounds but far from ideal
            1.0 - satisfaction
        };

        if severity >= self.threshold {
            let strength = (severity / self.threshold).min(1.0) * self.max_strength;
            Some(strength)
        } else {
            None
        }
    }

    /// Set the maximum strength.
    pub fn with_max_strength(mut self, strength: f64) -> Self {
        self.max_strength = strength;
        self
    }
}

/// The result of executing a corrective action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// The action that was taken
    pub action: Action,
    /// The strength at which it was applied
    pub strength: f64,
    /// Which gauge triggered it
    pub gauge_name: String,
    /// Whether the action was successful
    pub success: bool,
    /// Optional message
    pub message: String,
}

impl ActionResult {
    /// Create a successful action result.
    pub fn success(action: Action, strength: f64, gauge_name: impl Into<String>) -> Self {
        Self {
            action,
            strength,
            gauge_name: gauge_name.into(),
            success: true,
            message: String::new(),
        }
    }

    /// Create a failed action result.
    pub fn failure(action: Action, strength: f64, gauge_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            action,
            strength,
            gauge_name: gauge_name.into(),
            success: false,
            message: message.into(),
        }
    }
}
