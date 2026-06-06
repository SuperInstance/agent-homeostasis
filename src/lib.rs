//! # Agent Homeostasis
//!
//! Homeostatic control for AI agent systems — keeping agents in their operating
//! envelope through feedback loops inspired by biological homeostasis.
//!
//! Just as living organisms maintain internal stability through feedback mechanisms
//! (body temperature, blood sugar, blood pressure), AI agents need similar systems
//! to stay within their operating envelope.
//!
//! # Core Concept
//!
//! An agent has internal **gauges** (state variables) that must remain within
//! **setpoint** ranges. A **PID controller** monitors each gauge and fires
//! **corrective actions** when values drift. The **feedback loop** runs
//! continuously: measure → compare → correct → verify.
//!
//! # Modules
//!
//! - [`gauge`] — Internal state gauges (dashboard indicators)
//! - [`setpoint`] — Target ranges for each gauge
//! - [`controller`] — PID controllers for feedback correction
//! - [`action`] — Corrective actions to restore balance
//! - [`feedback`] — Feedback loop manager
//! - [`organism`] — The whole agent as an organism
//! - [`ecology`] — Multi-agent ecology

pub mod action;
pub mod controller;
pub mod ecology;
pub mod feedback;
pub mod gauge;
pub mod organism;
pub mod setpoint;

pub use action::{Action, ActionRule, ActionResult};
pub use controller::PIDController;
pub use ecology::{Ecology, ResourcePool, ResourceAllocation};
pub use feedback::FeedbackLoop;
pub use gauge::Gauge;
pub use organism::HomeostaticAgent;
pub use setpoint::Setpoint;
