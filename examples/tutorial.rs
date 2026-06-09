//! Tutorial: PID-controlled agent homeostasis
//!
//! Shows how biological homeostasis applies to AI agents using PID controllers,
//! gauges, setpoints, and ecological resource pools.

use agent_homeostasis::controller::PIDController;
use agent_homeostasis::gauge::Gauge;
use agent_homeostasis::setpoint::Setpoint;
use agent_homeostasis::ecology::ResourcePool;

fn main() {
    println!("=== Agent Homeostasis Tutorial ===\n");

    // Part 1: PID controller — keep error at zero
    println!("Part 1: PID controller for error regulation");
    let mut pid = PIDController::new(1.0, 0.1, 0.05)
        .with_output_limits(-10.0, 10.0);
    
    let error = 20.0_f64; // too far from target
    let output = pid.compute(error);
    println!("  Error: {:.1}", error);
    println!("  PID → P={:.2} I={:.2} D={:.2} total={:.2}", 
        output.p, output.i, output.d, output.output);
    println!();

    // Part 2: Setpoint — ideal operating ranges
    println!("Part 2: Setpoints (min, max, ideal)");
    let cpu_setpoint: Setpoint<f64> = Setpoint::new(20.0, 80.0, 50.0);
    println!("  CPU: min={:.0}, max={:.0}, ideal={:.0}", 
        cpu_setpoint.min, cpu_setpoint.max, cpu_setpoint.ideal);
    println!();

    // Part 3: Gauge — measuring agent vitals with setpoints
    println!("Part 3: Agent vital gauges");
    let sp = Setpoint::new(0.0, 100.0, 50.0);
    let mut gauge = Gauge::new("cpu", 65.0, sp);
    gauge.record(70.0);
    gauge.record(55.0);
    println!("  Gauge '{}': healthy={}, low={}, high={}", 
        gauge.name(), gauge.is_healthy(), gauge.is_low(), gauge.is_high());
    println!("  Satisfaction: {:.2}", gauge.satisfaction());
    println!("  Rate of change: {:.2}", gauge.rate_of_change());
    println!();

    // Part 4: Pre-built gauge constructors
    println!("Part 4: Pre-built gauge types");
    let acc = Gauge::accuracy(0.95);
    let lat = Gauge::latency_ms(150.0);
    let tok = Gauge::token_usage(5000.0);
    println!("  Accuracy: satisfied={:.2}", acc.satisfaction());
    println!("  Latency:  satisfied={:.2}", lat.satisfaction());
    println!("  Tokens:   satisfied={:.2}", tok.satisfaction());
    println!();

    // Part 5: Ecological resource pool
    println!("Part 5: Ecological resource pool");
    let pool = ResourcePool::new(100.0, 32.0, 1000.0, 50000.0);
    println!("  GPU: {:.0}%, Memory: {:.0}GB", pool.available_gpu(), pool.available_memory());
    println!("  Rate: {:.0}/min, Tokens: {:.0}", pool.available_rate(), pool.available_tokens());
}
