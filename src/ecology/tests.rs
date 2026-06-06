use super::*;
use crate::Gauge;

fn make_agent(id: &str, health: f64) -> HomeostaticAgent {
    let mut agent = HomeostaticAgent::new(id);
    // Add a single gauge that we control
    agent.add_gauge(
        Gauge::new("test", health, crate::setpoint::Setpoint::new(0.0, 1.0, 0.8)),
        crate::controller::PIDController::default_tuned(),
        1.0,
    );
    agent.compute_health();
    agent
}

#[test]
fn test_ecology_creation() {
    let pool = ResourcePool::new(100.0, 64.0, 1000.0, 100_000.0);
    let ecology = Ecology::new(pool);
    assert_eq!(ecology.agents.len(), 0);
    assert_eq!(ecology.ecology_health(), 1.0);
}

#[test]
fn test_add_agents() {
    let mut ecology = Ecology::new(ResourcePool::new(100.0, 64.0, 1000.0, 100_000.0));
    ecology.add_agent(make_agent("a1", 0.9));
    ecology.add_agent(make_agent("a2", 0.7));
    assert_eq!(ecology.agents.len(), 2);
}

#[test]
fn test_ecology_health_average() {
    let mut ecology = Ecology::new(ResourcePool::new(100.0, 64.0, 1000.0, 100_000.0));
    ecology.add_agent(make_agent("a1", 0.9));
    ecology.add_agent(make_agent("a2", 0.7));
    // health = satisfaction of value 0.9 in [0.0, 1.0] ideal 0.8
    // plus satisfaction of 0.7 in same range
    // Not exact 0.8, but should be between 0 and 1
    let h = ecology.ecology_health();
    assert!(h >= 0.0 && h <= 1.0);
}

#[test]
fn test_resource_pool_utilization() {
    let mut pool = ResourcePool::new(100.0, 64.0, 1000.0, 100_000.0);
    assert_eq!(pool.utilization(), 0.0);
    pool.allocated_gpu = 50.0;
    pool.allocated_memory = 32.0;
    pool.allocated_rate = 500.0;
    pool.allocated_tokens = 50_000.0;
    assert!((pool.utilization() - 0.5).abs() < 0.01);
}

#[test]
fn test_resource_pool_available() {
    let mut pool = ResourcePool::new(100.0, 64.0, 1000.0, 100_000.0);
    pool.allocated_gpu = 75.0;
    assert_eq!(pool.available_gpu(), 25.0);
}

#[test]
fn test_balance_resources_proportional() {
    let mut ecology = Ecology::new(ResourcePool::new(100.0, 64.0, 1000.0, 100_000.0));
    ecology.add_agent(make_agent("a1", 0.9));
    ecology.add_agent(make_agent("a2", 0.3));
    let allocs = ecology.balance_resources();
    // Agent with higher health should get more GPU
    assert!(allocs[0].gpu_compute > allocs[1].gpu_compute);
}

#[test]
fn test_balance_with_floor() {
    let mut ecology = Ecology::new(ResourcePool::new(100.0, 64.0, 1000.0, 100_000.0));
    ecology.add_agent(make_agent("a1", 0.9));
    ecology.add_agent(make_agent("a2", 0.1));
    let allocs = ecology.balance_with_floor(0.3);
    // Both agents should get at least some resources
    assert!(allocs[0].gpu_compute > 0.0);
    assert!(allocs[1].gpu_compute > 0.0);
}

#[test]
fn test_fittest_and_least_fit() {
    let mut ecology = Ecology::new(ResourcePool::new(100.0, 64.0, 1000.0, 100_000.0));
    ecology.add_agent(make_agent("a1", 0.9));
    ecology.add_agent(make_agent("a2", 0.3));
    ecology.add_agent(make_agent("a3", 0.6));
    assert_eq!(ecology.fittest().unwrap().id, "a1");
    assert_eq!(ecology.least_fit().unwrap().id, "a2");
}

#[test]
fn test_critical_agents() {
    let mut ecology = Ecology::new(ResourcePool::new(100.0, 64.0, 1000.0, 100_000.0));
    ecology.add_agent(make_agent("a1", 0.9));
    // Use a value well outside bounds to guarantee low health
    let mut low_agent = HomeostaticAgent::new("a2");
    low_agent.add_gauge(
        crate::Gauge::new("test", -2.0, crate::Setpoint::new(0.0, 1.0, 0.8)),
        crate::PIDController::default_tuned(),
        1.0,
    );
    low_agent.compute_health();
    ecology.add_agent(low_agent);
    let critical = ecology.critical_agents(0.5);
    assert_eq!(critical.len(), 1);
    assert_eq!(critical[0].id, "a2"); }

#[test]
fn test_diversity() {
    let mut ecology = Ecology::new(ResourcePool::new(100.0, 64.0, 1000.0, 100_000.0));
    ecology.add_agent(make_agent("a1", 0.9));
    ecology.add_agent(make_agent("a2", 0.9));
    // Low diversity (all similar health)
    assert!(ecology.diversity() < 0.01);

    // Add a different agent
    ecology.add_agent(make_agent("a3", 0.3));
    assert!(ecology.diversity() > 0.01);
}

#[test]
fn test_cycle_all_agents() {
    let mut ecology = Ecology::new(ResourcePool::new(100.0, 64.0, 1000.0, 100_000.0));
    ecology.add_agent(make_agent("a1", 0.8));
    ecology.add_agent(make_agent("a2", 0.7));

    let mut values = HashMap::new();
    values.insert("a1".to_string(), {
        let mut v = HashMap::new();
        v.insert("test".to_string(), 0.9);
        v
    });
    values.insert("a2".to_string(), {
        let mut v = HashMap::new();
        v.insert("test".to_string(), 0.6);
        v
    });

    let results = ecology.cycle_all(&values);
    assert_eq!(results.len(), 2);
}
