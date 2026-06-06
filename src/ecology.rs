//! Multi-agent ecology — population-level homeostasis.
//!
//! In nature, organisms don't exist in isolation. They share an ecosystem,
//! compete for resources, cooperate on tasks, and maintain population-level
//! balance. This module models multiple agents as an **ecology**.
//!
//! # Biological Analogy
//!
//! An ecosystem maintains homeostasis at the population level:
//! - **Resource competition**: Plants compete for sunlight; agents compete for GPU
//! - **Cooperation**: Wolves hunt in packs; agents collaborate on complex tasks
//! - **Population balance**: Predator-prey dynamics; agent pool scales to demand
//! - **Fitness landscape**: Species evolve; agents are tuned for their niche

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::action::ActionResult;
use crate::feedback::FeedbackStatus;
use crate::organism::HomeostaticAgent;

/// A shared resource pool for the ecology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePool {
    /// Total GPU compute available (arbitrary units)
    pub gpu_compute: f64,
    /// Total memory available (GB)
    pub memory_gb: f64,
    /// Total API rate limit (requests per minute)
    pub rate_limit: f64,
    /// Total token budget (tokens per minute)
    pub token_budget: f64,
    /// Currently allocated GPU compute
    pub allocated_gpu: f64,
    /// Currently allocated memory
    pub allocated_memory: f64,
    /// Currently allocated rate limit
    pub allocated_rate: f64,
    /// Currently allocated token budget
    pub allocated_tokens: f64,
}

impl ResourcePool {
    /// Create a new resource pool with given capacities.
    pub fn new(gpu_compute: f64, memory_gb: f64, rate_limit: f64, token_budget: f64) -> Self {
        Self {
            gpu_compute,
            memory_gb,
            rate_limit,
            token_budget,
            allocated_gpu: 0.0,
            allocated_memory: 0.0,
            allocated_rate: 0.0,
            allocated_tokens: 0.0,
        }
    }

    /// Get remaining GPU compute.
    pub fn available_gpu(&self) -> f64 {
        (self.gpu_compute - self.allocated_gpu).max(0.0)
    }

    /// Get remaining memory.
    pub fn available_memory(&self) -> f64 {
        (self.memory_gb - self.allocated_memory).max(0.0)
    }

    /// Get remaining rate limit.
    pub fn available_rate(&self) -> f64 {
        (self.rate_limit - self.allocated_rate).max(0.0)
    }

    /// Get remaining token budget.
    pub fn available_tokens(&self) -> f64 {
        (self.token_budget - self.allocated_tokens).max(0.0)
    }

    /// Get total utilization (0.0 to 1.0).
    pub fn utilization(&self) -> f64 {
        let total = self.gpu_compute + self.memory_gb + self.rate_limit + self.token_budget;
        let allocated = self.allocated_gpu + self.allocated_memory + self.allocated_rate + self.allocated_tokens;
        if total == 0.0 { 0.0 } else { allocated / total }
    }
}

/// A resource allocation for a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// Agent ID
    pub agent_id: String,
    /// Allocated GPU compute
    pub gpu_compute: f64,
    /// Allocated memory
    pub memory_gb: f64,
    /// Allocated rate limit
    pub rate_limit: f64,
    /// Allocated token budget
    pub token_budget: f64,
}

impl ResourceAllocation {
    /// Create a new allocation.
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            gpu_compute: 0.0,
            memory_gb: 0.0,
            rate_limit: 0.0,
            token_budget: 0.0,
        }
    }
}

/// A fitness landscape value for an agent.
pub type FitnessScore = f64;

/// The multi-agent ecology.
///
/// Manages multiple agents sharing resources, with population-level health
/// monitoring and resource rebalancing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ecology {
    /// Agents in the ecology
    pub agents: Vec<HomeostaticAgent>,
    /// Shared resource pool
    pub resource_pool: ResourcePool,
    /// Fitness landscape — one score per agent
    pub fitness_landscape: Vec<FitnessScore>,
    /// Current resource allocations
    pub allocations: Vec<ResourceAllocation>,
}

impl Ecology {
    /// Create a new ecology with a resource pool.
    pub fn new(resource_pool: ResourcePool) -> Self {
        Self {
            agents: Vec::new(),
            resource_pool,
            fitness_landscape: Vec::new(),
            allocations: Vec::new(),
        }
    }

    /// Add an agent to the ecology.
    pub fn add_agent(&mut self, agent: HomeostaticAgent) {
        self.fitness_landscape.push(agent.health);
        self.allocations.push(ResourceAllocation::new(&agent.id));
        self.agents.push(agent);
    }

    /// Compute the ecology-level health (average of all agent healths).
    pub fn ecology_health(&self) -> f64 {
        if self.agents.is_empty() {
            return 1.0;
        }
        self.agents.iter().map(|a| a.health).sum::<f64>() / self.agents.len() as f64
    }

    /// Balance resource allocation across agents proportionally by health.
    ///
    /// Healthier agents get more resources (they're performing well, reward them).
    /// Unhealthy agents get enough to recover but not enough to waste.
    pub fn balance_resources(&mut self) -> Vec<ResourceAllocation> {
        let n = self.agents.len() as f64;
        if n == 0.0 {
            return Vec::new();
        }

        let total_health: f64 = self.agents.iter().map(|a| a.health).sum();
        let total_health = if total_health == 0.0 { n } else { total_health };

        // Reset allocations
        self.resource_pool.allocated_gpu = 0.0;
        self.resource_pool.allocated_memory = 0.0;
        self.resource_pool.allocated_rate = 0.0;
        self.resource_pool.allocated_tokens = 0.0;

        for (i, agent) in self.agents.iter().enumerate() {
            let share = agent.health / total_health;

            let alloc = &mut self.allocations[i];
            alloc.gpu_compute = self.resource_pool.gpu_compute * share;
            alloc.memory_gb = self.resource_pool.memory_gb * share;
            alloc.rate_limit = self.resource_pool.rate_limit * share;
            alloc.token_budget = self.resource_pool.token_budget * share;

            self.resource_pool.allocated_gpu += alloc.gpu_compute;
            self.resource_pool.allocated_memory += alloc.memory_gb;
            self.resource_pool.allocated_rate += alloc.rate_limit;
            self.resource_pool.allocated_tokens += alloc.token_budget;
        }

        // Update fitness landscape
        for (i, agent) in self.agents.iter().enumerate() {
            self.fitness_landscape[i] = agent.health;
        }

        self.allocations.clone()
    }

    /// Balance resources with a minimum floor for struggling agents.
    ///
    /// Ensures every agent gets at least `floor_fraction` of equal share,
    /// then distributes the remainder by health.
    pub fn balance_with_floor(&mut self, floor_fraction: f64) -> Vec<ResourceAllocation> {
        let n = self.agents.len() as f64;
        if n == 0.0 {
            return Vec::new();
        }

        let equal_share = 1.0 / n;
        let floor = equal_share * floor_fraction;
        let remaining_fraction = 1.0 - floor * n;

        let total_health: f64 = self.agents.iter().map(|a| a.health).sum();
        let total_health = if total_health == 0.0 { n } else { total_health };

        self.resource_pool.allocated_gpu = 0.0;
        self.resource_pool.allocated_memory = 0.0;
        self.resource_pool.allocated_rate = 0.0;
        self.resource_pool.allocated_tokens = 0.0;

        for (i, agent) in self.agents.iter().enumerate() {
            let bonus = if remaining_fraction > 0.0 {
                (agent.health / total_health) * remaining_fraction
            } else {
                0.0
            };
            let share = floor + bonus;

            let alloc = &mut self.allocations[i];
            alloc.gpu_compute = self.resource_pool.gpu_compute * share;
            alloc.memory_gb = self.resource_pool.memory_gb * share;
            alloc.rate_limit = self.resource_pool.rate_limit * share;
            alloc.token_budget = self.resource_pool.token_budget * share;

            self.resource_pool.allocated_gpu += alloc.gpu_compute;
            self.resource_pool.allocated_memory += alloc.memory_gb;
            self.resource_pool.allocated_rate += alloc.rate_limit;
            self.resource_pool.allocated_tokens += alloc.token_budget;
        }

        self.allocations.clone()
    }

    /// Run a feedback cycle for all agents.
    ///
    /// Returns per-agent action results and statuses.
    pub fn cycle_all(
        &mut self,
        values: &HashMap<String, HashMap<String, f64>>,
    ) -> Vec<(String, Vec<ActionResult>, FeedbackStatus)> {
        let mut results = Vec::new();

        for agent in &mut self.agents {
            let agent_values = values.get(&agent.id).cloned().unwrap_or_default();
            let (actions, status) = agent.cycle(&agent_values);
            results.push((agent.id.clone(), actions, status));
        }

        // Update fitness landscape
        for (i, agent) in self.agents.iter().enumerate() {
            if i < self.fitness_landscape.len() {
                self.fitness_landscape[i] = agent.health;
            }
        }

        results
    }

    /// Find agents in critical condition (below health threshold).
    pub fn critical_agents(&self, threshold: f64) -> Vec<&HomeostaticAgent> {
        self.agents.iter().filter(|a| a.health < threshold).collect()
    }

    /// Get the most fit agent (highest health).
    pub fn fittest(&self) -> Option<&HomeostaticAgent> {
        self.agents.iter().max_by(|a, b| a.health.partial_cmp(&b.health).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Get the least fit agent (lowest health).
    pub fn least_fit(&self) -> Option<&HomeostaticAgent> {
        self.agents.iter().min_by(|a, b| a.health.partial_cmp(&b.health).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Compute population diversity (variance of health scores).
    pub fn diversity(&self) -> f64 {
        if self.agents.len() < 2 {
            return 0.0;
        }
        let mean = self.ecology_health();
        let variance: f64 = self.agents
            .iter()
            .map(|a| (a.health - mean).powi(2))
            .sum::<f64>() / self.agents.len() as f64;
        variance
    }
}

#[cfg(test)]
mod tests;
