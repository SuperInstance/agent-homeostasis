//! Comprehensive tests for agent-homeostasis.

use std::collections::HashMap;

use agent_homeostasis::{
    action::{Action, ActionRule, ActionResult},
    controller::PIDController,
    feedback::{FeedbackLoop, FeedbackPhase, FeedbackStatus},
    gauge::Gauge,
    organism::HomeostaticAgent,
    setpoint::{DynamicSetpoint, Setpoint, Adjustment},
};

// ============================================================
// Setpoint Tests
// ============================================================

#[test]
fn test_setpoint_contains() {
    let sp = Setpoint::new(0.5, 1.5, 1.0);
    assert!(sp.contains(1.0));
    assert!(sp.contains(0.5));
    assert!(sp.contains(1.5));
    assert!(!sp.contains(0.4));
    assert!(!sp.contains(1.6));
}

#[test]
fn test_setpoint_satisfaction_at_ideal() {
    let sp = Setpoint::new(0.0, 2.0, 1.0);
    let sat = sp.satisfaction(1.0);
    assert!((sat - 1.0).abs() < 0.001);
}

#[test]
fn test_setpoint_satisfaction_at_bounds() {
    let sp = Setpoint::new(0.0, 2.0, 1.0);
    // At min/max, satisfaction should be ~0
    assert!(sp.satisfaction(0.0).abs() < 0.001);
    assert!(sp.satisfaction(2.0).abs() < 0.001);
}

#[test]
fn test_setpoint_satisfaction_outside() {
    let sp = Setpoint::new(0.0, 2.0, 1.0);
    assert!(sp.satisfaction(-1.0) < 0.0);
    assert!(sp.satisfaction(3.0) < 0.0);
}

#[test]
fn test_setpoint_symmetric() {
    let sp = Setpoint::symmetric(50.0_f64, 10.0);
    assert_eq!(sp.min, 40.0);
    assert_eq!(sp.max, 60.0);
    assert_eq!(sp.ideal, 50.0);
}

#[test]
fn test_dynamic_setpoint_adjusts() {
    let base = Setpoint::new(0.0, 2.0, 1.0);
    let mut dyn_sp = DynamicSetpoint::new(base);
    dyn_sp.add_adjustment(Adjustment {
        condition: "high_load".to_string(),
        range_factor: 1.5,
        ideal_offset: 0.0,
        active: false,
    });

    // Before activation — should match base
    let resolved = dyn_sp.resolve();
    assert!((resolved.min - 0.0).abs() < 0.001);
    assert!((resolved.max - 2.0).abs() < 0.001);

    // After activation — range should widen
    dyn_sp.activate("high_load");
    let resolved = dyn_sp.resolve();
    assert!(resolved.min < 0.0);
    assert!(resolved.max > 2.0);
}

// ============================================================
// Gauge Tests
// ============================================================

#[test]
fn test_gauge_healthy() {
    let gauge = Gauge::new("test", 0.8, Setpoint::new(0.0, 1.0, 0.5));
    assert!(gauge.is_healthy());
}

#[test]
fn test_gauge_out_of_bounds_high() {
    let gauge = Gauge::new("test", 1.5, Setpoint::new(0.0, 1.0, 0.5));
    assert!(gauge.is_high());
    assert!(!gauge.is_healthy());
}

#[test]
fn test_gauge_out_of_bounds_low() {
    let gauge = Gauge::new("test", -0.5, Setpoint::new(0.0, 1.0, 0.5));
    assert!(gauge.is_low());
    assert!(!gauge.is_healthy());
}

#[test]
fn test_gauge_prebuilt_accuracy() {
    let gauge = Gauge::accuracy(0.95);
    assert_eq!(gauge.name, "accuracy");
    assert!(gauge.is_healthy());
}

#[test]
fn test_gauge_prebuilt_latency() {
    let gauge = Gauge::latency_ms(600.0);
    assert!(!gauge.is_healthy()); // 600ms > 500ms max
    assert!(gauge.is_high());
}

#[test]
fn test_gauge_history_records() {
    let mut gauge = Gauge::new("test", 0.5, Setpoint::new(0.0, 1.0, 0.5));
    gauge.update(0.6);
    gauge.update(0.7);
    assert_eq!(gauge.history.len(), 3); // initial + 2 updates
    assert!((gauge.value - 0.7_f64).abs() < 0.001);
}

#[test]
fn test_gauge_satisfaction() {
    let gauge = Gauge::new("test", 0.5, Setpoint::new(0.0, 1.0, 0.5));
    assert!((gauge.satisfaction() - 1.0).abs() < 0.001);
}

#[test]
fn test_gauge_error_from_ideal() {
    let gauge = Gauge::new("test", 0.8, Setpoint::new(0.0, 1.0, 0.5));
    assert!((gauge.error() - 0.3).abs() < 0.001);
}

// ============================================================
// PID Controller Tests
// ============================================================

#[test]
fn test_pid_proportional_only() {
    let mut pid = PIDController::proportional_only(1.0)
        .with_output_limits(-10.0, 10.0);
    let out = pid.compute(2.0);
    assert!((out.output - 2.0).abs() < 0.001);
    assert!((out.p_term - 2.0).abs() < 0.001);
}

#[test]
fn test_pid_converges_to_setpoint() {
    let mut pid = PIDController::new(0.8, 0.1, 0.3)
        .with_output_limits(-10.0, 10.0);

    let mut value = 5.0;
    let setpoint = 0.0;
    let learning_rate = 0.1;

    for _ in 0..100 {
        let error = value - setpoint;
        let output = pid.compute(error);
        value -= output.output * learning_rate;
    }

    // Should have converged close to setpoint
    assert!(value.abs() < 0.5, "Value should be near setpoint, got {}", value);
}

#[test]
fn test_pid_integral_accumulates() {
    let mut pid = PIDController::pi(0.0, 1.0)
        .with_output_limits(-100.0, 100.0);
    pid.compute(1.0);
    pid.compute(1.0);
    assert!(pid.integral > 0.0);
}

#[test]
fn test_pid_anti_windup() {
    let mut pid = PIDController::pi(0.0, 1.0)
        .with_integral_limit(5.0)
        .with_output_limits(-100.0, 100.0);

    for _ in 0..1000 {
        pid.compute(100.0);
    }
    assert!(pid.integral <= 5.0);
}

#[test]
fn test_pid_reset() {
    let mut pid = PIDController::new(1.0, 1.0, 1.0);
    pid.compute(5.0);
    pid.compute(3.0);
    assert!(pid.integral != 0.0);
    assert!(pid.prev_error != 0.0);

    pid.reset();
    assert_eq!(pid.integral, 0.0);
    assert_eq!(pid.prev_error, 0.0);
}

#[test]
fn test_pid_output_clamping() {
    let mut pid = PIDController::proportional_only(100.0)
        .with_output_limits(-1.0, 1.0);
    let out = pid.compute(10.0);
    assert_eq!(out.output, 1.0);
}

#[test]
fn test_pid_compute_with_dt() {
    let mut pid = PIDController::new(1.0, 1.0, 1.0)
        .with_output_limits(-10.0, 10.0);
    let out = pid.compute_with_dt(1.0, 1.0);
    assert!(out.output != 0.0);

    // dt = 0 should give zero
    let out_zero = pid.compute_with_dt(1.0, 0.0);
    assert_eq!(out_zero.output, 0.0);
}

// ============================================================
// Action Tests
// ============================================================

#[test]
fn test_action_rule_fires_on_high() {
    let rule = ActionRule::new("latency_ms", Action::ThrottleRequests, 0.3, true);
    // Gauge above ideal, satisfaction negative
    let satisfaction = -0.5;
    let fired = rule.should_fire(satisfaction, 600.0, 100.0);
    assert!(fired.is_some());
    assert!(fired.unwrap() > 0.0);
}

#[test]
fn test_action_rule_does_not_fire_wrong_direction() {
    let rule = ActionRule::new("latency_ms", Action::ThrottleRequests, 0.3, true);
    // Value below ideal, but rule fires on high
    let satisfaction = -0.5;
    let fired = rule.should_fire(satisfaction, 50.0, 100.0);
    assert!(fired.is_none());
}

#[test]
fn test_action_rule_respects_threshold() {
    let rule = ActionRule::new("test", Action::ReduceContext, 0.8, true);
    // Low severity, below threshold
    let fired = rule.should_fire(0.5, 1.1, 1.0);
    assert!(fired.is_none());
}

#[test]
fn test_action_result_success() {
    let result = ActionResult::success(Action::PauseAndRecover, 0.8, "memory");
    assert!(result.success);
    assert_eq!(result.action, Action::PauseAndRecover);
}

#[test]
fn test_action_names() {
    assert_eq!(Action::ThrottleRequests.name(), "throttle_requests");
    assert_eq!(Action::EscalateToHuman.name(), "escalate_to_human");
}

#[test]
fn test_action_default_severity() {
    assert!(Action::EscalateToHuman.default_severity() > Action::ThrottleRequests.default_severity());
}

// ============================================================
// Feedback Loop Tests
// ============================================================

#[test]
fn test_feedback_loop_creation() {
    let fl = FeedbackLoop::new();
    assert_eq!(fl.gauges.len(), 0);
    assert_eq!(fl.phase, FeedbackPhase::Measure);
}

#[test]
fn test_feedback_loop_add_gauge() {
    let mut fl = FeedbackLoop::new();
    fl.add_gauge(
        Gauge::accuracy(0.9),
        PIDController::default_tuned(),
    );
    assert_eq!(fl.gauges.len(), 1);
}

#[test]
fn test_feedback_loop_stable() {
    let mut fl = FeedbackLoop::new();
    fl.add_gauge(
        Gauge::accuracy(0.95),
        PIDController::default_tuned(),
    );

    let values = HashMap::from([("accuracy".to_string(), 0.95)]);
    fl.measure(&values);
    fl.compare();
    let _actions = fl.correct();
    let status = fl.verify();

    assert_eq!(status, FeedbackStatus::Stable);
}

#[test]
fn test_feedback_loop_correcting() {
    let mut fl = FeedbackLoop::new();
    fl.add_gauge(
        Gauge::latency_ms(600.0),
        PIDController::default_tuned(),
    );
    fl.add_action_rule(ActionRule::new(
        "latency_ms",
        Action::ThrottleRequests,
        0.1,
        true,
    ));

    let values = HashMap::from([("latency_ms".to_string(), 600.0)]);
    fl.measure(&values);
    fl.compare();
    let actions = fl.correct();

    assert!(!actions.is_empty());
}

#[test]
fn test_feedback_loop_run_cycle() {
    let mut fl = FeedbackLoop::new();
    fl.add_gauge(Gauge::accuracy(0.5), PIDController::default_tuned());

    let values = HashMap::from([("accuracy".to_string(), 0.5)]);
    let (actions, _status) = fl.run_cycle(&values);
    // Accuracy at 0.5 with ideal 0.95 should trigger something
    // But without action rules, no actions fire
    assert!(actions.is_empty());
}

#[test]
fn test_feedback_loop_system_health() {
    let mut fl = FeedbackLoop::new();
    fl.add_gauge(Gauge::accuracy(0.95), PIDController::default_tuned());
    fl.add_gauge(Gauge::latency_ms(100.0), PIDController::default_tuned());

    let health = fl.system_health();
    assert!(health > 0.5);
}

#[test]
fn test_feedback_loop_simulate_step() {
    let mut fl = FeedbackLoop::new();
    fl.add_gauge(Gauge::accuracy(0.5), PIDController::default_tuned());

    let new_vals = fl.simulate_step(0.1);
    // Value should have moved toward ideal (0.95)
    let new_accuracy = new_vals.get("accuracy").unwrap();
    assert!(new_accuracy > &0.5, "Should have moved toward setpoint");
}

// ============================================================
// Organism Tests
// ============================================================

#[test]
fn test_agent_creation() {
    let agent = HomeostaticAgent::new("test-agent");
    assert_eq!(agent.id, "test-agent");
    assert_eq!(agent.health, 1.0);
    assert!(!agent.paused);
}

#[test]
fn test_agent_health_decreases_on_drift() {
    let mut agent = HomeostaticAgent::new("test");
    agent.add_gauge(
        Gauge::accuracy(0.95),
        PIDController::default_tuned(),
        1.0,
    );

    let initial_health = agent.compute_health();

    // Drive accuracy way down
    agent.update_gauge("accuracy", 0.5).unwrap();
    let new_health = agent.compute_health();

    assert!(new_health < initial_health);
}

#[test]
fn test_agent_simulate_stabilization() {
    let mut agent = HomeostaticAgent::new("test");
    agent.add_gauge(
        Gauge::accuracy(0.5),
        PIDController::new(0.5, 0.1, 0.2).with_output_limits(-2.0, 2.0),
        1.0,
    );

    let history = agent.simulate_stabilization(200, 0.05);

    // Health should generally improve over time
    assert!(history.last().unwrap() > history.first().unwrap());
}

#[test]
fn test_agent_is_critical() {
    let mut agent = HomeostaticAgent::new("test");
    agent.add_gauge(
        Gauge::new("test", 0.5, Setpoint::new(0.0, 1.0, 0.8)),
        PIDController::default_tuned(),
        1.0,
    );
    agent.compute_health();
    // Satisfaction at 0.5 with ideal 0.8 should be < 1
    assert!(agent.is_critical(0.99));
}

#[test]
fn test_agent_default_agent() {
    let agent = HomeostaticAgent::default_agent("default");
    assert_eq!(agent.gauges.len(), 5);
    assert!(agent.health > 0.0);
}

#[test]
fn test_agent_update_nonexistent_gauge() {
    let mut agent = HomeostaticAgent::new("test");
    let result = agent.update_gauge("nonexistent", 1.0);
    assert!(result.is_err());
}

// ============================================================
// Integration / Stabilization Tests
// ============================================================

#[test]
fn test_pid_stabilizes_oscillating_system() {
    // Simulate an oscillating system and show PID damps it
    let mut pid = PIDController::new(0.6, 0.05, 0.4)
        .with_output_limits(-5.0, 5.0);

    let setpoint = 10.0;
    let mut value = 20.0; // Start far from setpoint
    let mut max_amplitude = 0.0_f64;
    let learning_rate = 0.1;

    for i in 0..200 {
        let error = value - setpoint;
        let output = pid.compute(error);
        value -= output.output * learning_rate;

        let amplitude = (value - setpoint).abs();
        if i > 100 {
            max_amplitude = max_amplitude.max(amplitude);
        }
    }

    // After 100 iterations, oscillation should be damped
    assert!(max_amplitude < 2.0, "Oscillation should be damped, max amplitude: {}", max_amplitude);
}

#[test]
fn test_full_feedback_stabilization() {
    let mut fl = FeedbackLoop::new();
    fl.add_gauge(
        Gauge::new("error_rate", 0.3, Setpoint::new(0.0, 0.1, 0.01)),
        PIDController::new(0.5, 0.1, 0.2).with_output_limits(-1.0, 1.0),
    );

    // Simulate 100 steps
    for _ in 0..100 {
        fl.simulate_step(0.05);
    }

    let gauge = fl.gauges.get("error_rate").unwrap();
    assert!(gauge.value < 0.3, "Error rate should have decreased, got {}", gauge.value);
}

#[test]
fn test_multi_gauge_agent_stabilization() {
    let mut agent = HomeostaticAgent::new("multi");
    agent.add_gauge(
        Gauge::accuracy(0.5),
        PIDController::new(0.5, 0.1, 0.2).with_output_limits(-1.0, 1.0),
        0.5,
    );
    agent.add_gauge(
        Gauge::latency_ms(400.0),
        PIDController::new(0.5, 0.1, 0.2).with_output_limits(-200.0, 200.0),
        0.5,
    );

    let history = agent.simulate_stabilization(200, 0.05);

    // Final health should be better than initial
    let initial = history.first().unwrap();
    let final_h = history.last().unwrap();
    assert!(final_h >= initial, "Health should improve: {} -> {}", initial, final_h);
}
