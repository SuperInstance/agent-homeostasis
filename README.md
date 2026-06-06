# 🧬 agent-homeostasis

**Homeostatic control for AI agent systems** — keeping agents in their operating envelope through feedback loops.

[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Inspired by **biological homeostasis**: just as living organisms maintain internal stability through feedback mechanisms (body temperature, blood sugar, blood pressure), AI agents need similar systems to stay within their operating envelope.

## 🫀 The Biological Analogy

| Biological System | Agent Equivalent | This Crate |
|---|---|---|
| Baroreceptors (blood pressure sensors) | Agent metrics collection | `gauge` |
| Hypothalamic setpoints (target temp, pH) | Target operating ranges | `setpoint` |
| Pancreatic insulin response (PID control) | Feedback correction engine | `controller` |
| Sweat / shiver / vasoconstriction | Throttle, switch model, escalate | `action` |
| Baroreceptor reflex arc | measure → compare → correct → verify | `feedback` |
| The whole organism (you) | The whole agent | `organism` |
| An ecosystem of organisms | Multi-agent resource sharing | `ecology` |

## 🏗️ Architecture

### Gauges — The Dashboard Indicators

Like baroreceptors tracking blood pressure, gauges are the sensory receptors of an AI agent:

```rust
use agent_homeostasis::{Gauge, Setpoint};

// Pre-built gauges for common metrics
let accuracy = Gauge::accuracy(0.95);       // 0.7–1.0, ideal 0.95
let latency = Gauge::latency_ms(150.0);     // 10–500ms, ideal 100ms
let errors = Gauge::error_rate(0.02);       // 0–0.1, ideal 0.01
let memory = Gauge::memory_pressure(0.5);   // 0–0.8, ideal 0.4
let temp = Gauge::temperature(0.7);         // 0–1.5, ideal 0.7 (creativity)

// Or build your own
let custom = Gauge::new("gpu_util", 0.85, Setpoint::new(0.3, 0.95, 0.7));
```

### Setpoints — The Target Ranges

Your hypothalamus doesn't target a single temperature — it maintains a range. Same here:

```rust
use agent_homeostasis::setpoint::{Setpoint, DynamicSetpoint, Adjustment};

let sp = Setpoint::new(0.7, 1.0, 0.95);  // min, max, ideal
assert!(sp.contains(0.85));
assert!(!sp.contains(0.5));

// Satisfaction: 1.0 at ideal, 0.0 at bounds, negative outside
assert_eq!(sp.satisfaction(0.95), 1.0);   // at ideal
assert!(sp.satisfaction(0.5) < 0.0);       // outside range

// Dynamic setpoints that adjust based on conditions
let mut dyn_sp = DynamicSetpoint::new(sp);
dyn_sp.add_adjustment(Adjustment {
    condition: "high_load".into(),
    range_factor: 1.5,   // widen acceptable range
    ideal_offset: 0.0,
    active: false,
});
dyn_sp.activate("high_load");  // widens range under high load
```

### PID Controller — The Feedback Engine

Your pancreas acts as a PID controller for blood sugar:

- **P**roportional: Current glucose vs. target → immediate insulin
- **I**ntegral: Persistent high glucose → sustained insulin production
- **D**erivative: Rapid glucose rise → aggressive early response

```rust
use agent_homeostasis::PIDController;

let mut pid = PIDController::new(0.5, 0.1, 0.2)  // kp, ki, kd
    .with_output_limits(-1.0, 1.0)
    .with_integral_limit(10.0);

let error = 0.3;  // current - ideal
let output = pid.compute(error);
println!("Correction: {}", output.output);
// Also available: output.p_term, output.i_term, output.d_term
```

### Corrective Actions — What To Do When Things Drift

When body temperature rises, you sweat. When it drops, you shiver:

```rust
use agent_homeostasis::action::{Action, ActionRule};

let rule = ActionRule::new(
    "latency_ms",
    Action::ThrottleRequests,
    0.3,    // fire when severity > 30%
    true,   // fire when value is HIGH (above ideal)
);
```

Available actions:
- `ThrottleRequests` — Reduce request rate
- `SwitchModel` — Switch to lighter/faster model
- `ReduceContext` — Trim conversation history
- `IncreaseParallelism` — Distribute work across workers
- `PauseAndRecover` — Pause for GC/cache flush
- `EscalateToHuman` — Alert a human operator

### Feedback Loop — The Reflex Arc

The baroreceptor reflex: measure → compare → correct → verify:

```rust
use agent_homeostasis::FeedbackLoop;

let mut loop_ = FeedbackLoop::new();
loop_.add_gauge(Gauge::latency_ms(100.0), PIDController::default_tuned());
loop_.add_action_rule(ActionRule::new("latency_ms", Action::ThrottleRequests, 0.2, true));

// Run a complete cycle
let values = HashMap::from([("latency_ms".to_string(), 600.0)]);
let (actions, status) = loop_.run_cycle(&values);
```

### Organism — The Whole Agent

An organism doesn't just regulate temperature — it simultaneously regulates temperature, blood sugar, hydration, oxygen, pH, and more:

```rust
use agent_homeostasis::HomeostaticAgent;

let mut agent = HomeostaticAgent::default_agent("my-agent");
// Comes pre-configured with: accuracy, latency, error_rate, memory_pressure, temperature

agent.update_gauge("latency_ms", 400.0)?;
agent.update_gauge("accuracy", 0.6)?;

let health = agent.compute_health();  // Weighted average of all gauge satisfactions
println!("Agent health: {:.1}%", health * 100.0);

// Simulate stabilization over 100 steps
let history = agent.simulate_stabilization(100, 0.05);
```

### Ecology — Multi-Agent Ecosystem

Organisms share an ecosystem — they compete for resources, cooperate on tasks, and maintain population-level balance:

```rust
use agent_homeostasis::ecology::{Ecology, ResourcePool};

let pool = ResourcePool::new(100.0, 64.0, 1000.0, 100_000.0);
// gpu_compute, memory_gb, rate_limit, token_budget

let mut ecology = Ecology::new(pool);
ecology.add_agent(agent1);
ecology.add_agent(agent2);

// Balance resources proportionally by health
let allocations = ecology.balance_resources();
// Healthier agents get more resources

// Or with a floor for struggling agents
let allocations = ecology.balance_with_floor(0.3);
// Every agent gets at least 30% of equal share, then bonus by health

println!("Ecology health: {:.1}%", ecology.ecology_health() * 100.0);
println!("Fittest: {}", ecology.fittest().unwrap().id);
```

## 🔬 Testing

54 tests covering all modules:

```bash
cargo test
```

Key test scenarios:
- PID controller converges to setpoint from arbitrary initial values
- Anti-windup prevents integral term explosion
- Gauges correctly detect out-of-bounds conditions
- Health score decreases when gauges drift from ideal
- Corrective actions fire at the right thresholds and directions
- Ecology balances resource allocation proportionally
- Feedback loop stabilizes oscillating systems
- Dynamic setpoints adjust correctly when conditions activate

## 📦 Dependencies

- `serde` — Serialization for all types
- `anyhow` — Error handling

**No external math dependencies** — PID is implemented from scratch.

## 🧪 Quick Start

```toml
[dependencies]
agent-homeostasis = "0.1"
```

```rust
use agent_homeostasis::*;
use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    // Create a fully-configured agent
    let mut agent = HomeostaticAgent::default_agent("production-agent");

    // Simulate receiving live metrics
    let metrics = HashMap::from([
        ("accuracy".to_string(), 0.82),
        ("latency_ms".to_string(), 350.0),
        ("error_rate".to_string(), 0.08),
    ]);

    let (actions, status) = agent.cycle(&metrics);

    println!("Health: {:.1}%", agent.health * 100.0);
    for action in &actions {
        println!("Action: {:?} (strength: {:.2})", action.action, action.strength);
    }

    Ok(())
}
```

## 📜 License

MIT
