//! Hardware-aware performance prediction for quartz launcher.
//!
//! Estimates how a modpack will run on the user's machine: FPS, RAM, bottlenecks,
//! and actionable recommendations. This is one tool among many — not the launcher focus.

pub mod hardware;
pub mod models;
pub mod prediction;

pub use hardware::{detect_hardware, score_cpu, score_gpu};
pub use models::*;
pub use prediction::predict_performance;
