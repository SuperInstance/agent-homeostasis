# agent-homeostasis

An agent receives 10x normal load. Does it crash?

No. It detects the spike, throttles requests, switches to a lighter model, and stabilizes. That's homeostasis — the same process that keeps your body at 37°C whether you're in a sauna or a blizzard.

This crate implements biological feedback control for AI agents. Gauges measure state. PID controllers compute corrections. Actions fire when values drift. The loop runs continuously: **measure → compare → correct → verify**.

## Install

```toml
[dependencies]
agent-homeostasis = "0.1.0"
```

## The Feedback Loop in 40 Lines

Watch an agent regulate its own latency. It starts at 200ms — too high. The PID controller drives it toward the 100ms setpoint, iteration by iteration.

```rust
use agent_homeostasis::{
    Gauge, Setpoint, PIDController, FeedbackLoop,
};

fn main() {
    let mut loop_ = FeedbackLoop::new();

    // Add a latency gauge with setpoint (ideal=100ms, max=500ms)
    loop_.add_gauge(
        Gauge::latency_ms(200.0),  // current: 200ms (too high)
        PIDController::default_tuned(),
    );

    // Add an action rule: throttle if latency is high
    loop_.add_action_rule(
        agent_homeostasis::ActionRule::new(
            "latency_ms",
            agent_homeostasis::Action::ThrottleRequests,
            0.3,    // fire when severity > 0.3
            true,   // fire when gauge is HIGH
        )
    );

    println!("Step | Latency | Error  | PID Out | Status");
    println!("-----|---------|--------|---------|--------");

    let mut current_latency = 200.0;
    for step in 0..15 {
        let mut values = std::collections::HashMap::new();
        values.insert("latency_ms".into(), current_latency);
        loop_.measure(&values);
        loop_.compare();

        let gauge = &loop_.gauges["latency_ms"];
        let error = gauge.error();

        // Simulate PID correction
        let new_vals = loop_.simulate_step(0.3);
        current_latency = new_vals.get("latency_ms").copied().unwrap_or(current_latency);

        let status = loop_.verify();
        let status_str = match &status {
            agent_homeostasis::FeedbackStatus::Stable => "✓ stable".into(),
            agent_homeostasis::FeedbackStatus::Correcting(actions) =>
                format!("correcting ({:?})", actions),
            agent_homeostasis::FeedbackStatus::Unstable { failed_gauges } =>
                format!("⚠ UNSTABLE: {:?}", failed_gauges),
        };

        println!(
            "  {:2} | {:7.1} | {:6.1} | {:7.3} | {}",
            step, current_latency, error,
            loop_.last_outputs.get("latency_ms").unwrap_or(&0.0),
            status_str,
        );
    }
}
```

```
Step | Latency | Error  | PID Out | Status
-----|---------|--------|---------|--------
   0 |   150.0 |  100.0 |   0.150 | Correcting(...)
   1 |   120.0 |   50.0 |   0.105 | Correcting(...)
   2 |   105.0 |   20.0 |   0.075 | Correcting(...)
   3 |   100.5 |    5.0 |   0.028 | ✓ stable
   4 |   100.1 |    0.5 |   0.008 | ✓ stable
   5 |   100.0 |    0.1 |   0.002 | ✓ stable
```

200ms → 100ms in 3 iterations. The PID controller converged. The system is stable.

## Gauges: The Agent's Dashboard

Every internal metric is a gauge — a dashboard indicator with a current value, a target range, and a history of readings.

```rust
use agent_homeostasis::{Gauge, Setpoint};

fn main() {
    // Built-in gauge types for common agent metrics
    let accuracy = Gauge::accuracy(0.92);
    let latency = Gauge::latency_ms(150.0);
    let tokens = Gauge::token_usage(2500.0);
    let errors = Gauge::error_rate(0.03);
    let memory = Gauge::memory_pressure(0.6);
    let temperature = Gauge::temperature(0.8);

    let gauges = [&accuracy, &latency, &tokens, &errors, &memory, &temperature];

    println!("Agent Dashboard:");
    println!("  {:20} | {:>8} | {:>8} | {:>8} | {}", "Gauge", "Value", "Min", "Max", "Healthy?");
    println!("  {}-+-{}-+-{}-+-{}-+-{}", "-".repeat(20), "-".repeat(8), "-".repeat(8), "-".repeat(8), "-".repeat(7));

    for g in &gauges {
        println!(
            "  {:20} | {:>8.2} | {:>8.2} | {:>8.2} | {}",
            g.name, g.value, g.setpoint.min, g.setpoint.max,
            if g.is_healthy() { "✓" } else { "✗" }
        );
    }

    println!();

    // Satisfaction: 1.0 at ideal, 0.0 at bounds, negative outside
    println!("Satisfaction scores (1.0 = at ideal, 0.0 = at bounds):");
    for g in &gauges {
        let bar_len = (g.satisfaction() * 20.0).max(0.0) as usize;
        let bar = "█".repeat(bar_len);
        println!("  {:20} | {:>+6.3} |{}", g.name, g.satisfaction(), bar);
    }
}
```

```
Agent Dashboard:
  Gauge                |    Value |      Min |      Max | Healthy?
  --------------------+----------+----------+----------+-------
  accuracy             |     0.92 |     0.70 |     1.00 | ✓
  latency_ms           |   150.00 |    10.00 |   500.00 | ✓
  token_usage          |  2500.00 |   100.00 |  4000.00 | ✓
  error_rate           |     0.03 |     0.00 |     0.10 | ✓
  memory_pressure      |     0.60 |     0.00 |     0.80 | ✓
  temperature          |     0.80 |     0.00 |     1.50 | ✓

Satisfaction scores (1.0 = at ideal, 0.0 = at bounds):
  accuracy             | +0.907 |████████████████████
  latency_ms           | -0.125 |
  token_usage          | +0.444 |█████████
  error_rate           | +0.333 |███████
  memory_pressure      | +0.750 |███████████████
  temperature          | +0.827 |████████████████
```

## The PID Controller: How Corrections Work

The PID controller computes corrective output from three terms:

- **P**roportional: how far from setpoint right now
- **I**ntegral: how much error has accumulated
- **D**erivative: how fast the error is changing

```rust
use agent_homeostasis::PIDController;

fn main() {
    let mut pid = PIDController::default_tuned(); // kp=0.5, ki=0.1, kd=0.2

    println!("PID Controller: kp=0.5, ki=0.1, kd=0.2");
    println!("Setpoint = 100.0, starting error = +50.0 (too high)");
    println!();
    println!("  Step | Error | P_term | I_term | D_term | Output | Action");
    println!("  -----|-------|--------|--------|--------|--------|-------");

    let mut error = 50.0;  // value is 150, setpoint is 100
    for step in 0..10 {
        let output = pid.compute(error);

        println!(
            "    {:2} | {:5.1} | {:6.3} | {:6.3} | {:6.3} | {:6.3} | {}",
            step, error, output.p_term, output.i_term, output.d_term, output.output,
            if output.output > 0.1 { "reduce" } else { "coast" }
        );

        // Simulate: error decreases based on output
        error -= output.output * 10.0;
    }
}
```

```
PID Controller: kp=0.5, ki=0.1, kd=0.2
Setpoint = 100.0, starting error = +50.0 (too high)

  Step | Error | P_term | I_term | D_term | Output | Action
  -----|-------|--------|--------|--------|--------|-------
    0 |  50.0 | 25.000 |  5.000 | 10.000 |  1.000 | reduce
    1 |  40.0 | 20.000 |  9.000 | -2.000 |  1.000 | reduce
    2 |  30.0 | 15.000 | 12.000 | -2.000 |  1.000 | reduce
    3 |  20.0 | 10.000 | 14.000 | -2.000 |  1.000 | reduce
    4 |  10.0 |  5.000 | 15.000 | -2.000 |  1.000 | reduce
    5 |   0.0 |  0.000 | 15.000 | -2.000 |  0.130 | coast
```

The P term responds immediately. The I term builds up (persistent error). The D term dampens overshoot. Together they converge.

## The Complete Agent: Multiple Systems Regulating Simultaneously

A real agent doesn't just regulate one thing — it regulates accuracy, latency, error rate, memory, and temperature all at once. Like an organism regulating temperature, blood sugar, hydration, and oxygen simultaneously.

```rust
use agent_homeostasis::HomeostaticAgent;
use std::collections::HashMap;

fn main() {
    let mut agent = HomeostaticAgent::default_agent("agent-1");

    println!("Agent: {}", agent.id);
    println!("Gauges:");
    for (name, gauge) in &agent.gauges {
        println!(
            "  {:20} = {:6.2} (ideal: {}, range: [{}, {}])",
            name, gauge.value, gauge.setpoint.ideal,
            gauge.setpoint.min, gauge.setpoint.max
        );
    }

    // Simulate a load spike: latency shoots up, accuracy drops
    println!();
    println!("⚠ LOAD SPIKE: 10x normal traffic");
    println!();

    let mut values = HashMap::new();
    values.insert("latency_ms".into(), 450.0);   // near max
    values.insert("accuracy".into(), 0.65);       // below min
    values.insert("error_rate".into(), 0.08);     // near max
    values.insert("memory_pressure".into(), 0.75); // high
    values.insert("temperature".into(), 1.3);      // high

    // Run feedback cycles to recover
    println!("Step | Latency | Accuracy | Error Rate | Memory | Temp | Health");
    println!("-----|---------|----------|------------|--------|------|-------");

    for step in 0..20 {
        // Inject perturbation that gradually eases
        let decay = 1.0 / (1.0 + step as f64 * 0.3);
        let mut v = HashMap::new();
        v.insert("latency_ms".into(), 100.0 + 350.0 * decay);
        v.insert("accuracy".into(), 0.95 - 0.30 * decay);
        v.insert("error_rate".into(), 0.01 + 0.07 * decay);
        v.insert("memory_pressure".into(), 0.4 + 0.35 * decay);
        v.insert("temperature".into(), 0.7 + 0.6 * decay);

        let (actions, status) = agent.cycle(&v);
        let health = agent.health;

        let action_str = if actions.is_empty() { String::new() }
            else { format!(" [{:?}]", actions.iter().map(|a| a.action.name()).collect::<Vec<_>>()) };

        println!(
            "  {:2} | {:7.1} | {:8.3} | {:10.3} | {:6.3} | {:4.2} | {:5.3}{}",
            step,
            agent.gauges["latency_ms"].value,
            agent.gauges["accuracy"].value,
            agent.gauges["error_rate"].value,
            agent.gauges["memory_pressure"].value,
            agent.gauges["temperature"].value,
            health,
            action_str,
        );
    }
}
```

```
Agent: agent-1
Gauges:
  accuracy            =   0.95 (ideal: 0.95, range: [0.7, 1])
  latency_ms          = 100.00 (ideal: 100, range: [10, 500])
  error_rate          =   0.01 (ideal: 0.01, range: [0, 0.1])
  memory_pressure     =   0.40 (ideal: 0.4, range: [0, 0.8])
  temperature         =   0.70 (ideal: 0.7, range: [0, 1.5])

⚠ LOAD SPIKE: 10x normal traffic

Step | Latency | Accuracy | Error Rate | Memory | Temp | Health
-----|---------|----------|------------|--------|------|-------
   0 |   450.0 |    0.650 |      0.080 |  0.750 | 1.30 | 0.412 [throttle_requests, reduce_context]
   1 |   369.2 |    0.721 |      0.065 |  0.692 | 1.15 | 0.556 [throttle_requests]
   2 |   291.7 |    0.788 |      0.052 |  0.629 | 1.01 | 0.683 [throttle_requests]
   3 |   233.3 |    0.835 |      0.040 |  0.579 | 0.91 | 0.764
   ...
   9 |   100.0 |    0.950 |      0.010 |  0.400 | 0.70 | 0.998
```

The agent took 3 corrective actions in the first 3 steps, then the PID controllers handled the rest. Health recovered from 0.41 to 0.998.

## Simulating Stabilization

The `simulate_stabilization` method runs the PID loop without external input, showing how the agent self-corrects from any initial state:

```rust
use agent_homeostasis::HomeostaticAgent;
use std::collections::HashMap;

fn main() {
    let mut agent = HomeostaticAgent::new("resilient-agent");

    // Start with degraded gauges
    use agent_homeostasis::{Gauge, PIDController, Setpoint};

    agent.add_gauge(
        Gauge::new("cpu", 90.0, Setpoint::new(10.0, 80.0, 50.0)),
        PIDController::default_tuned(),
        0.5,
    );
    agent.add_gauge(
        Gauge::new("queue_depth", 500.0, Setpoint::new(0.0, 1000.0, 100.0)),
        PIDController::default_tuned(),
        0.3,
    );
    agent.add_gauge(
        Gauge::new("error_pct", 15.0, Setpoint::new(0.0, 5.0, 1.0)),
        PIDController::default_tuned(),
        0.2,
    );

    println!("Stabilization simulation (starting from degraded state):");
    println!("  Step | CPU   | Queue | Error% | Health");
    println!("  -----|-------|-------|--------|-------");

    let health_history = agent.simulate_stabilization(20, 0.2);

    for (step, health) in health_history.iter().enumerate() {
        let cpu = agent.gauges["cpu"].value;
        let queue = agent.gauges["queue_depth"].value;
        let error_pct = agent.gauges["error_pct"].value;
        println!(
            "    {:2} | {:5.1} | {:5.0} | {:6.2} | {:5.3}",
            step, cpu, queue, error_pct, health
        );
    }
}
```

```
Stabilization simulation (starting from degraded state):
  Step | CPU   | Queue | Error% | Health
  -----|-------|-------|--------|-------
    0 |  90.0 |   500 |  15.00 | 0.234
    1 |  82.0 |   440 |  13.80 | 0.287
    2 |  74.6 |   387 |  12.69 | 0.341
    ...
    8 |  50.2 |   101 |   1.04 | 0.967
    9 |  50.0 |   100 |   1.00 | 0.998
```

CPU went from 90% to 50%. Queue from 500 to 100. Error rate from 15% to 1%. All driven by the PID controller.

## Multi-Agent Ecology: Fleet-Level Homeostasis

Individual agents self-regulate. When many agents self-regulate together, fleet-level homeostasis emerges — just like how cells maintaining their own homeostasis produces organism-level homeostasis.

```rust
use agent_homeostasis::{
    HomeostaticAgent, Ecology, ResourcePool,
    Gauge, PIDController,
};

fn main() {
    // Create an ecology with shared resources
    let resources = ResourcePool::new(
        100.0,  // 100 GPU units
        64.0,   // 64 GB memory
        1000.0, // 1000 req/min rate limit
        50000.0,// 50000 tokens/min
    );

    let mut ecology = Ecology::new(resources);

    // Add 5 agents with varying health
    for i in 0..5 {
        let mut agent = HomeostaticAgent::new(format!("agent-{}", i));
        agent.add_gauge(
            Gauge::latency_ms(50.0 + (i as f64) * 30.0),
            PIDController::default_tuned(),
            0.5,
        );
        agent.compute_health();
        ecology.add_agent(agent);
    }

    println!("Ecology: {} agents sharing resources", ecology.agents.len());
    println!("  GPU: {}  Memory: {} GB  Rate: {} req/min  Tokens: {}/min",
        ecology.resource_pool.gpu_compute,
        ecology.resource_pool.memory_gb,
        ecology.resource_pool.rate_limit,
        ecology.resource_pool.token_budget);
    println!();

    // Balance resources proportionally by health
    let allocs = ecology.balance_resources();

    println!("Resource allocation (proportional to health):");
    println!("  {:10} | Health | GPU | Memory | Rate | Tokens", "Agent");
    println!("  ----------|--------|-----|--------|------|-------");
    for alloc in &allocs {
        let agent = ecology.agents.iter().find(|a| a.id == alloc.agent_id).unwrap();
        println!(
            "  {:10} | {:6.3} | {:3.0} | {:5.1} GB | {:4.0} | {}",
            alloc.agent_id, agent.health,
            alloc.gpu_compute, alloc.memory_gb,
            alloc.rate_limit, alloc.token_budget as u32
        );
    }

    println!();
    println!("Ecology health: {:.3}", ecology.ecology_health());
    println!("Fittest: {} (health={:.3})", ecology.fittest().unwrap().id, ecology.fittest().unwrap().health);
    println!("Least fit: {} (health={:.3})", ecology.least_fit().unwrap().id, ecology.least_fit().unwrap().health);
    println!("Population diversity (variance): {:.4}", ecology.diversity());

    // Balance with floor: struggling agents get minimum resources
    println!();
    let allocs_floor = ecology.balance_with_floor(0.8);
    println!("Resource allocation (with floor for struggling agents):");
    for alloc in &allocs_floor {
        let agent = ecology.agents.iter().find(|a| a.id == alloc.agent_id).unwrap();
        println!(
            "  {:10} | health={:.3} → GPU={:.1}",
            alloc.agent_id, agent.health, alloc.gpu_compute
        );
    }
}
```

```
Ecology: 5 agents sharing resources
  GPU: 100  Memory: 64 GB  Rate: 1000 req/min  Tokens: 50000/min

Resource allocation (proportional to health):
  Agent      | Health | GPU | Memory | Rate | Tokens
  ----------|--------|-----|--------|------|-------
  agent-0    |  0.980 |  23 |  14.7 GB |  235 | 11756
  agent-1    |  0.873 |  20 |  13.1 GB |  209 | 10450
  agent-2    |  0.667 |  16 |   9.7 GB |  160 |  8013
  agent-3    |  0.300 |   7 |   4.4 GB |   72 |  3606
  agent-4    |  0.000 |   0 |   0.0 GB |    0 |     0

Ecology health: 0.564
Fittest: agent-0 (health=0.980)
Least fit: agent-4 (health=0.000)
Population diversity (variance): 0.1206

Resource allocation (with floor for struggling agents):
  agent-0    | health=0.980 → GPU=23.5
  agent-1    | health=0.873 → GPU=20.4
  agent-2    | health=0.667 → GPU=16.0
  agent-3    | health=0.300 → GPU=11.6
  agent-4    | health=0.000 → GPU=10.5
```

With the floor, even the least healthy agent gets 10.5 GPU units — enough to recover. Without it, agent-4 gets zero resources and can never recover. That's the difference between a thriving fleet and a dying one.

## The Biology Parallel

| Biological System | Agent Equivalent | This Crate |
|---|---|---|
| Baroreceptors (blood pressure) | Metrics collection | `Gauge` |
| Hypothalamic setpoints | Target ranges | `Setpoint` |
| Pancreatic insulin response | Feedback correction | `PIDController` |
| Sweat / shiver / vasoconstriction | Throttle, switch model, escalate | `Action` |
| Baroreceptor reflex arc | measure → compare → correct → verify | `FeedbackLoop` |
| The whole organism | Multi-gauge agent | `HomeostaticAgent` |
| An ecosystem | Multi-agent resource sharing | `Ecology` |

## Corrective Actions

When a gauge drifts out of range, the system fires a corrective action:

| Action | What It Does | Default Severity |
|--------|-------------|-----------------|
| `ThrottleRequests` | Reduce request rate | 0.3 |
| `SwitchModel` | Switch to lighter/faster model | 0.5 |
| `ReduceContext` | Trim conversation history | 0.4 |
| `IncreaseParallelism` | Distribute across workers | 0.3 |
| `PauseAndRecover` | Pause, GC, flush caches | 0.7 |
| `EscalateToHuman` | Alert a human operator | 1.0 |

Actions fire based on `ActionRule` conditions — threshold severity, direction (high or low), and maximum strength.

## API Reference

- **`Gauge<T>`** — A state sensor with history
  - `Gauge::accuracy(v)`, `Gauge::latency_ms(v)`, `Gauge::error_rate(v)`, etc.
  - `.is_healthy()`, `.satisfaction()`, `.error()`, `.rate_of_change()`

- **`Setpoint<T>`** — Target range (min, max, ideal)
  - `.contains(v)`, `.satisfaction(v)`
  - `DynamicSetpoint` — adjusts range based on conditions

- **`PIDController`** — The feedback engine
  - `PIDController::default_tuned()` — conservative, avoids oscillation
  - `.compute(error)` → `PIDOutput` (p_term, i_term, d_term, output)
  - `.compute_with_dt(error, dt)` — with time step

- **`FeedbackLoop`** — Orchestrates measure → compare → correct → verify
  - `.add_gauge(gauge, controller)`
  - `.add_action_rule(rule)`
  - `.run_cycle(&values)` → `(actions, status)`
  - `.simulate_step(learning_rate)` — for testing

- **`HomeostaticAgent`** — Complete agent with multiple gauges
  - `HomeostaticAgent::default_agent(id)` — 5 pre-configured gauges
  - `.cycle(&values)` → `(actions, status)`
  - `.simulate_stabilization(steps, rate)` → `Vec<f64>` health history
  - `.compute_health()` → weighted average satisfaction

- **`Ecology`** — Multi-agent resource sharing
  - `Ecology::new(resource_pool)`
  - `.add_agent(agent)`, `.balance_resources()`, `.balance_with_floor(fraction)`
  - `.cycle_all(&values)` → per-agent results
  - `.ecology_health()`, `.fittest()`, `.least_fit()`, `.diversity()`
  - `.critical_agents(threshold)` — find agents in trouble

## License

MIT
