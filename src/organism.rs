//! The whole agent as an organism — multiple systems working together.
//!
//! An agent is more than the sum of its gauges. Like a living organism, it has
//! multiple interconnected systems that must all stay in balance simultaneously.
//! The **health score** represents overall wellbeing as a weighted average of
//! all gauge satisfactions.
//!
//! # Biological Analogy
//!
//! An organism doesn't just regulate temperature — it simultaneously regulates
//! temperature, blood sugar, hydration, oxygen, pH, and more. The liver doesn't
//! wait for the kidneys to finish; they all run in parallel. If any single
//! system fails, the organism is in trouble.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::action::{Action, ActionRule, ActionResult};
use crate::controller::PIDController;
use crate::feedback::FeedbackStatus;
use crate::gauge::Gauge;


/// A homeostatic agent — an organism with multiple regulatory systems.
///
/// Each agent maintains multiple gauges, each with its own PID controller
/// and action rules. The overall health is a weighted average.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeostaticAgent {
    /// Unique agent identifier
    pub id: String,
    /// Gauges with their names
    pub gauges: HashMap<String, Gauge<f64>>,
    /// PID controllers for each gauge
    pub controllers: HashMap<String, PIDController>,
    /// Action rules
    pub actions: Vec<ActionRule>,
    /// Weights for each gauge in health calculation
    pub weights: HashMap<String, f64>,
    /// Overall health score (0.0 to 1.0)
    pub health: f64,
    /// Whether the agent is currently paused
    pub paused: bool,
    /// Number of feedback cycles completed
    pub cycles: usize,
}

impl HomeostaticAgent {
    /// Create a new agent with an ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            gauges: HashMap::new(),
            controllers: HashMap::new(),
            actions: Vec::new(),
            weights: HashMap::new(),
            health: 1.0,
            paused: false,
            cycles: 0,
        }
    }

    /// Add a gauge with its controller and weight.
    pub fn add_gauge(
        &mut self,
        gauge: Gauge<f64>,
        controller: PIDController,
        weight: f64,
    ) {
        let name = gauge.name.clone();
        self.weights.insert(name.clone(), weight);
        self.gauges.insert(name.clone(), gauge);
        self.controllers.insert(name, controller);
    }

    /// Add an action rule.
    pub fn add_action_rule(&mut self, rule: ActionRule) {
        self.actions.push(rule);
    }

    /// Compute the health score as a weighted average of gauge satisfactions.
    pub fn compute_health(&mut self) -> f64 {
        if self.gauges.is_empty() {
            self.health = 1.0;
            return 1.0;
        }

        let total_weight: f64 = self.weights.values().sum();
        if total_weight == 0.0 {
            self.health = 1.0;
            return 1.0;
        }

        let weighted_sum: f64 = self.gauges
            .iter()
            .map(|(name, gauge)| {
                let weight = self.weights.get(name).copied().unwrap_or(1.0);
                let satisfaction = gauge.satisfaction().max(0.0); // Clamp negative to 0 for health
                weight * satisfaction
            })
            .sum();

        self.health = (weighted_sum / total_weight).clamp(0.0, 1.0);
        self.health
    }

    /// Update a gauge's value.
    pub fn update_gauge(&mut self, name: &str, value: f64) -> anyhow::Result<()> {
        if let Some(gauge) = self.gauges.get_mut(name) {
            gauge.update(value);
            Ok(())
        } else {
            anyhow::bail!("Gauge '{}' not found", name)
        }
    }

    /// Run a single feedback cycle.
    ///
    /// Returns actions taken and the resulting feedback status.
    pub fn cycle(&mut self, values: &HashMap<String, f64>) -> (Vec<ActionResult>, FeedbackStatus) {
        self.cycles += 1;

        // Update gauge values
        for (name, value) in values {
            if let Some(gauge) = self.gauges.get_mut(name) {
                gauge.update(*value);
            }
        }

        // Compute PID outputs and determine corrections
        let mut actions = Vec::new();

        for (name, gauge) in &self.gauges {
            if let Some(controller) = self.controllers.get_mut(name) {
                let error = gauge.error();
                controller.compute(error);
            }
        }

        // Evaluate action rules
        for rule in &self.actions {
            if let Some(gauge) = self.gauges.get(&rule.gauge_name) {
                let satisfaction = gauge.satisfaction();
                if let Some(strength) = rule.should_fire(satisfaction, gauge.value, gauge.setpoint.ideal) {
                    actions.push(ActionResult::success(rule.action, strength, &rule.gauge_name));
                }
            }
        }

        // Check status
        let unhealthy: Vec<String> = self.gauges
            .iter()
            .filter(|(_, g)| !g.is_healthy())
            .map(|(n, _)| n.clone())
            .collect();

        let status = if unhealthy.is_empty() {
            FeedbackStatus::Stable
        } else {
            let corrective: Vec<Action> = self.actions
                .iter()
                .filter(|r| unhealthy.contains(&r.gauge_name))
                .map(|r| r.action)
                .collect();
            FeedbackStatus::Correcting(corrective)
        };

        self.compute_health();
        (actions, status)
    }

    /// Simulate the agent stabilizing over multiple steps.
    ///
    /// Applies PID corrections iteratively to bring gauges toward setpoints.
    /// Returns the health score at each step.
    pub fn simulate_stabilization(&mut self, steps: usize, learning_rate: f64) -> Vec<f64> {
        let mut health_history = Vec::new();

        for _ in 0..steps {
            let names: Vec<String> = self.gauges.keys().cloned().collect();
            for name in names {
                let error = if let Some(gauge) = self.gauges.get(&name) {
                    gauge.error()
                } else {
                    continue;
                };
                if let Some(controller) = self.controllers.get_mut(&name) {
                    let output = controller.compute(error);
                    let correction = output.output * learning_rate;
                    let new_value = self.gauges.get(&name).unwrap().value - correction;
                    if let Some(g) = self.gauges.get_mut(&name) {
                        g.update(new_value);
                    }
                }
            }

            self.compute_health();
            health_history.push(self.health);
        }

        health_history
    }

    /// Check if the agent is in a critical state (health below threshold).
    pub fn is_critical(&self, threshold: f64) -> bool {
        self.health < threshold
    }

    /// Create a well-configured agent with default gauges.
    pub fn default_agent(id: impl Into<String>) -> Self {
        let mut agent = Self::new(id);

        agent.add_gauge(
            Gauge::accuracy(0.95),
            PIDController::default_tuned(),
            0.3,
        );
        agent.add_gauge(
            Gauge::latency_ms(100.0),
            PIDController::default_tuned(),
            0.2,
        );
        agent.add_gauge(
            Gauge::error_rate(0.01),
            PIDController::default_tuned(),
            0.25,
        );
        agent.add_gauge(
            Gauge::memory_pressure(0.4),
            PIDController::default_tuned(),
            0.15,
        );
        agent.add_gauge(
            Gauge::temperature(0.7),
            PIDController::default_tuned(),
            0.1,
        );

        agent.compute_health();
        agent
    }
}
