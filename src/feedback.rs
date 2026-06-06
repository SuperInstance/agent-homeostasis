//! Feedback loop manager — the measure → compare → correct → verify cycle.
//!
//! The feedback loop is the core of homeostatic control. It runs continuously:
//!
//! 1. **Measure**: Read current gauge values
//! 2. **Compare**: Evaluate against setpoints
//! 3. **Correct**: Fire corrective actions if needed
//! 4. **Verify**: Check that corrections had the desired effect
//!
//! # Biological Analogy
//!
//! The baroreceptor reflex: blood pressure drops → baroreceptors detect it →
//! sympathetic nervous system fires (heart rate up, vessels constrict) →
//! pressure rises → baroreceptors confirm → system stabilizes.

use std::collections::HashMap;

use crate::action::{Action, ActionRule, ActionResult};
use crate::controller::PIDController;
use crate::gauge::Gauge;

/// The phase of the feedback cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeedbackPhase {
    /// Reading current gauge values
    Measure,
    /// Comparing against setpoints
    Compare,
    /// Applying corrective actions
    Correct,
    /// Verifying corrections took effect
    Verify,
}

/// Status of the feedback loop.
#[derive(Debug, Clone, PartialEq)]
pub enum FeedbackStatus {
    /// System is stable — all gauges within bounds
    Stable,
    /// System is correcting — actions have been fired
    Correcting(Vec<Action>),
    /// System is unstable — corrections aren't working
    Unstable { failed_gauges: Vec<String> },
}

/// The feedback loop manager.
///
/// Orchestrates the measure → compare → correct → verify cycle for all gauges.
#[derive(Debug, Clone)]
pub struct FeedbackLoop {
    /// Gauges being monitored
    pub gauges: HashMap<String, Gauge<f64>>,
    /// PID controllers for each gauge
    pub controllers: HashMap<String, PIDController>,
    /// Action rules
    pub action_rules: Vec<ActionRule>,
    /// Current phase
    pub phase: FeedbackPhase,
    /// Maximum number of correction cycles before declaring unstable
    pub max_correction_cycles: usize,
    /// Number of consecutive correction cycles
    pub correction_count: usize,
    /// History of actions taken
    pub action_history: Vec<ActionResult>,
    /// PID outputs for debugging
    pub last_outputs: HashMap<String, f64>,
}

impl FeedbackLoop {
    /// Create a new empty feedback loop.
    pub fn new() -> Self {
        Self {
            gauges: HashMap::new(),
            controllers: HashMap::new(),
            action_rules: Vec::new(),
            phase: FeedbackPhase::Measure,
            max_correction_cycles: 10,
            correction_count: 0,
            action_history: Vec::new(),
            last_outputs: HashMap::new(),
        }
    }

    /// Add a gauge with its PID controller.
    pub fn add_gauge(&mut self, gauge: Gauge<f64>, controller: PIDController) {
        let name = gauge.name.clone();
        self.gauges.insert(name.clone(), gauge);
        self.controllers.insert(name, controller);
    }

    /// Add an action rule.
    pub fn add_action_rule(&mut self, rule: ActionRule) {
        self.action_rules.push(rule);
    }

    /// Phase 1: Measure — update all gauges with new values.
    pub fn measure(&mut self, values: &HashMap<String, f64>) {
        self.phase = FeedbackPhase::Measure;
        for (name, value) in values {
            if let Some(gauge) = self.gauges.get_mut(name) {
                gauge.update(*value);
            }
        }
    }

    /// Phase 2: Compare — evaluate all gauges against their setpoints.
    ///
    /// Returns gauges that are out of bounds and their errors.
    pub fn compare(&mut self) -> Vec<(String, f64, f64)> {
        self.phase = FeedbackPhase::Compare;
        let mut deviations = Vec::new();

        for (name, gauge) in &self.gauges {
            if !gauge.is_healthy() {
                deviations.push((name.clone(), gauge.error(), gauge.satisfaction()));
            }
        }

        deviations
    }

    /// Phase 3: Correct — compute PID outputs and determine actions.
    ///
    /// Returns the list of actions to take.
    pub fn correct(&mut self) -> Vec<ActionResult> {
        self.phase = FeedbackPhase::Correct;
        let mut actions = Vec::new();

        // Compute PID outputs for all gauges
        for (name, gauge) in &self.gauges {
            if let Some(controller) = self.controllers.get_mut(name) {
                let error = gauge.error();
                let output = controller.compute(error);
                self.last_outputs.insert(name.clone(), output.output);
            }
        }

        // Evaluate action rules
        for rule in &self.action_rules {
            if let Some(gauge) = self.gauges.get(&rule.gauge_name) {
                let satisfaction = gauge.satisfaction();
                let value = gauge.value;
                let ideal = gauge.setpoint.ideal;

                if let Some(strength) = rule.should_fire(satisfaction, value, ideal) {
                    let result = ActionResult::success(
                        rule.action,
                        strength,
                        &rule.gauge_name,
                    );
                    actions.push(result);
                }
            }
        }

        self.action_history.extend(actions.clone());
        if !actions.is_empty() {
            self.correction_count += 1;
        }

        actions
    }

    /// Phase 4: Verify — check if corrections stabilized the system.
    pub fn verify(&mut self) -> FeedbackStatus {
        self.phase = FeedbackPhase::Verify;

        let deviations = self.gauges
            .iter()
            .filter(|(_, g)| !g.is_healthy())
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();

        if deviations.is_empty() {
            self.correction_count = 0;
            FeedbackStatus::Stable
        } else if self.correction_count >= self.max_correction_cycles {
            FeedbackStatus::Unstable { failed_gauges: deviations }
        } else {
            let corrective_actions: Vec<Action> = self.action_rules
                .iter()
                .filter(|rule| deviations.contains(&rule.gauge_name))
                .map(|rule| rule.action)
                .collect();
            FeedbackStatus::Correcting(corrective_actions)
        }
    }

    /// Run a complete feedback cycle: measure → compare → correct → verify.
    pub fn run_cycle(&mut self, values: &HashMap<String, f64>) -> (Vec<ActionResult>, FeedbackStatus) {
        self.measure(values);
        self.compare();
        let actions = self.correct();
        let status = self.verify();
        (actions, status)
    }

    /// Run a simulation step: apply PID corrections to gauge values.
    ///
    /// This simulates the effect of corrections by moving gauge values
    /// toward their setpoints based on PID output.
    ///
    /// Returns the updated gauge values.
    pub fn simulate_step(&mut self, learning_rate: f64) -> HashMap<String, f64> {
        let mut new_values = HashMap::new();

        for (name, gauge) in &self.gauges {
            if let Some(controller) = self.controllers.get_mut(name) {
                let error = gauge.error();
                let output = controller.compute(error);
                self.last_outputs.insert(name.clone(), output.output);

                // Apply correction: move value toward ideal
                let correction = output.output * learning_rate;
                let new_value = gauge.value - correction;
                new_values.insert(name.clone(), new_value);
            }
        }

        // Update gauges
        for (name, value) in &new_values {
            if let Some(gauge) = self.gauges.get_mut(name) {
                gauge.update(*value);
            }
        }

        new_values
    }

    /// Get the overall system health (average satisfaction across all gauges).
    pub fn system_health(&self) -> f64 {
        if self.gauges.is_empty() {
            return 1.0;
        }

        let total: f64 = self.gauges.values().map(|g| g.satisfaction()).sum();
        let count = self.gauges.len() as f64;
        (total / count).clamp(0.0, 1.0)
    }
}

impl Default for FeedbackLoop {
    fn default() -> Self {
        Self::new()
    }
}
