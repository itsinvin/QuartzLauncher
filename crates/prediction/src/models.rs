//! Shared data types. These mirror `src/lib/types.ts` and serialize as camelCase so
//! the same JSON crosses the Tauri command boundary unchanged.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GpuTier {
    Integrated,
    Entry,
    Mainstream,
    High,
    Enthusiast,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StorageKind {
    Ssd,
    Hdd,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HardwareSource {
    Detected,
    Manual,
    Estimated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub brand: String,
    pub vendor: String,
    pub physical_cores: u32,
    pub logical_cores: u32,
    pub base_ghz: f64,
    pub single_thread_score: u32,
    pub multi_thread_score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub model: String,
    pub vendor: String,
    pub vram_mb: Option<u32>,
    pub score: u32,
    pub tier: GpuTier,
    pub integrated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareProfile {
    pub cpu: CpuInfo,
    pub gpu: GpuInfo,
    pub total_ram_mb: u64,
    pub os: String,
    pub arch: String,
    pub storage: StorageKind,
    pub source: HardwareSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkloadProfile {
    pub name: String,
    pub mc_version: String,
    pub loader: String,
    pub mod_count: u32,
    pub allocated_ram_mb: u32,
    pub render_distance: u32,
    pub shaders: bool,
    pub optimization_mods: bool,
    pub heavy_mods: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PerfRating {
    Unplayable,
    Choppy,
    Playable,
    Smooth,
    Excellent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Bottleneck {
    Cpu,
    Gpu,
    Ram,
    Storage,
    Balanced,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RecommendationSeverity {
    Info,
    Tip,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub severity: RecommendationSeverity,
    pub title: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_fps_gain: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PredictionFactor {
    pub label: String,
    pub impact: f64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformancePrediction {
    pub avg_fps: u32,
    pub fps_range: [u32; 2],
    pub low_fps: u32,
    pub ram_usage_mb: u32,
    pub ram_headroom_mb: i32,
    pub ram_sufficient: bool,
    pub recommended_ram_mb: u32,
    pub load_time_sec: u32,
    pub rating: PerfRating,
    pub rating_score: u32,
    pub bottleneck: Bottleneck,
    pub worldgen_lag_risk: RiskLevel,
    pub confidence: Confidence,
    pub recommendations: Vec<Recommendation>,
    pub factors: Vec<PredictionFactor>,
}
