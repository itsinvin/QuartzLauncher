//! Performance prediction engine — a 1:1 port of `src/lib/prediction.ts`.
//!
//! See the TypeScript file for the full rationale behind every coefficient. Keeping
//! the two in sync guarantees identical numbers whether prediction runs in the
//! browser or natively.

use crate::models::*;

const REF_GPU_SCORE: f64 = 38.0;
const REF_CPU_SINGLE: f64 = 72.0;
const REF_RENDER_DISTANCE: f64 = 12.0;
const REF_BASE_FPS: f64 = 260.0;

fn clampf(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}

fn estimate_ram_needed_mb(w: &WorkloadProfile) -> f64 {
    let base = 900.0;
    let per_mod = 6.5;
    let render_overhead = (w.render_distance as f64).powi(2) * 3.0;
    let shader_overhead = if w.shaders { 700.0 } else { 0.0 };
    let heavy_overhead = if w.heavy_mods { 800.0 } else { 0.0 };
    let opt_mult = if w.optimization_mods { 0.82 } else { 1.0 };
    ((base + w.mod_count as f64 * per_mod + render_overhead + shader_overhead + heavy_overhead) * opt_mult).round()
}

pub fn predict_performance(hw: &HardwareProfile, w: &WorkloadProfile) -> PerformancePrediction {
    let gpu_factor = clampf(hw.gpu.score as f64 / REF_GPU_SCORE, 0.05, 6.0);
    let cpu_factor = clampf(hw.cpu.single_thread_score as f64 / REF_CPU_SINGLE, 0.1, 2.2);

    let gpu_weight = clampf(
        0.38 + (w.render_distance as f64 - 8.0) * 0.028 + if w.shaders { 0.3 } else { 0.0 },
        0.3,
        0.85,
    );
    let cpu_weight = 1.0 - gpu_weight;
    let combined_factor = 1.0 / (gpu_weight / gpu_factor + cpu_weight / cpu_factor);

    let rd_factor = (REF_RENDER_DISTANCE / (w.render_distance as f64).max(2.0)).powf(1.3);
    let mod_factor = 1.0 / (1.0 + w.mod_count as f64 * 0.0035);
    let opt_factor = if w.optimization_mods { 2.4 } else { 1.0 };
    let shader_factor = if w.shaders {
        clampf(0.25 + gpu_factor * 0.1, 0.18, 0.55)
    } else {
        1.0
    };
    let heavy_factor = if w.heavy_mods { 0.82 } else { 1.0 };

    let ram_needed = estimate_ram_needed_mb(w);
    let ram_sufficient = w.allocated_ram_mb as f64 >= ram_needed;
    let ram_ratio = w.allocated_ram_mb as f64 / ram_needed.max(1.0);
    let ram_factor = if ram_sufficient {
        1.0
    } else {
        clampf(0.3 + ram_ratio * 0.6, 0.3, 0.95)
    };

    let avg_fps_raw = REF_BASE_FPS
        * combined_factor
        * rd_factor
        * mod_factor
        * opt_factor
        * shader_factor
        * heavy_factor
        * ram_factor;
    let avg_fps = clampf(avg_fps_raw.round(), 1.0, 2000.0) as u32;

    let low_ratio = clampf(
        0.42 + if ram_sufficient { 0.1 } else { -0.12 } + (cpu_factor - 1.0) * 0.08
            + if w.optimization_mods { 0.08 } else { 0.0 },
        0.25,
        0.72,
    );
    let low_fps = clampf((avg_fps as f64 * low_ratio).round(), 1.0, avg_fps as f64) as u32;
    let fps_range = [
        (avg_fps as f64 * 0.82).round() as u32,
        (avg_fps as f64 * 1.22).round() as u32,
    ];

    let bounded = ram_needed.min(w.allocated_ram_mb as f64);
    let ram_usage_mb = (bounded * 0.92 + bounded * 0.08).round() as u32;
    let max_alloc = (((hw.total_ram_mb as f64 - 2048.0) / 512.0).floor() * 512.0).max(1024.0);
    let recommended_ram_mb = clampf((ram_needed * 1.3 / 512.0).ceil() * 512.0, 1024.0, max_alloc) as u32;
    let ram_headroom_mb = w.allocated_ram_mb as i32 - ram_needed as i32;

    let storage_mult = match hw.storage {
        StorageKind::Hdd => 2.2,
        StorageKind::Unknown => 1.3,
        StorageKind::Ssd => 1.0,
    };
    let load_time_sec = ((4.0 + w.mod_count as f64 * 0.18 + if w.shaders { 4.0 } else { 0.0 })
        * storage_mult
        / clampf(cpu_factor, 0.4, 2.0).sqrt())
    .round() as u32;

    let rating_score = compute_rating_score(avg_fps, low_fps, ram_sufficient);
    let rating = rating_from_score(rating_score);
    let bottleneck = identify_bottleneck(gpu_factor, cpu_factor, ram_sufficient, ram_ratio, w.shaders);
    let worldgen_lag_risk = worldgen_risk(hw.cpu.single_thread_score, w, ram_sufficient);
    let confidence = compute_confidence(hw);
    let recommendations = build_recommendations(
        hw,
        w,
        avg_fps,
        ram_needed,
        ram_sufficient,
        recommended_ram_mb,
        bottleneck,
    );
    let factors = build_factors(
        hw, w, rd_factor, mod_factor, opt_factor, shader_factor, ram_factor, gpu_factor, cpu_factor,
    );

    PerformancePrediction {
        avg_fps,
        fps_range,
        low_fps,
        ram_usage_mb,
        ram_headroom_mb,
        ram_sufficient,
        recommended_ram_mb,
        load_time_sec,
        rating,
        rating_score,
        bottleneck,
        worldgen_lag_risk,
        confidence,
        recommendations,
        factors,
    }
}

fn compute_rating_score(avg_fps: u32, low_fps: u32, ram_sufficient: bool) -> u32 {
    let fps_score = clampf((avg_fps as f64 / 12.0).log2() * 26.0, 0.0, 100.0);
    let low_score = clampf(((low_fps.max(1) as f64) / 8.0).log2() * 22.0, 0.0, 100.0);
    let mut score = fps_score * 0.6 + low_score * 0.4;
    if !ram_sufficient {
        score *= 0.78;
    }
    clampf(score.round(), 0.0, 100.0) as u32
}

fn rating_from_score(score: u32) -> PerfRating {
    match score {
        s if s >= 88 => PerfRating::Excellent,
        s if s >= 70 => PerfRating::Smooth,
        s if s >= 50 => PerfRating::Playable,
        s if s >= 32 => PerfRating::Choppy,
        _ => PerfRating::Unplayable,
    }
}

fn identify_bottleneck(
    gpu_factor: f64,
    cpu_factor: f64,
    ram_sufficient: bool,
    ram_ratio: f64,
    shaders: bool,
) -> Bottleneck {
    if !ram_sufficient && ram_ratio < 0.85 {
        return Bottleneck::Ram;
    }
    let ratio = gpu_factor / cpu_factor;
    if shaders && gpu_factor < cpu_factor {
        return Bottleneck::Gpu;
    }
    if ratio < 0.75 {
        return Bottleneck::Gpu;
    }
    if ratio > 1.4 {
        return Bottleneck::Cpu;
    }
    Bottleneck::Balanced
}

fn worldgen_risk(cpu_single: u32, w: &WorkloadProfile, ram_sufficient: bool) -> RiskLevel {
    let mut risk = 0;
    if cpu_single < 55 {
        risk += 2;
    } else if cpu_single < 72 {
        risk += 1;
    }
    if w.heavy_mods {
        risk += 1;
    }
    if w.mod_count > 180 {
        risk += 1;
    }
    if !ram_sufficient {
        risk += 1;
    }
    if risk >= 3 {
        RiskLevel::High
    } else if risk >= 1 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    }
}

fn compute_confidence(hw: &HardwareProfile) -> Confidence {
    if hw.source == HardwareSource::Manual {
        return Confidence::Medium;
    }
    let gpu_known = hw.gpu.model != "Unknown GPU" && hw.gpu.score > 0;
    let cpu_known = hw.cpu.brand != "Unknown CPU";
    if hw.source == HardwareSource::Detected && gpu_known && cpu_known {
        Confidence::High
    } else if gpu_known || cpu_known {
        Confidence::Medium
    } else {
        Confidence::Low
    }
}

fn build_recommendations(
    hw: &HardwareProfile,
    w: &WorkloadProfile,
    avg_fps: u32,
    ram_needed: f64,
    ram_sufficient: bool,
    recommended_ram_mb: u32,
    bottleneck: Bottleneck,
) -> Vec<Recommendation> {
    let mut recs = Vec::new();
    let fabric_family = w.loader == "fabric" || w.loader == "quilt" || w.loader == "neoforge";

    if !w.optimization_mods {
        recs.push(Recommendation {
            severity: RecommendationSeverity::Tip,
            title: if fabric_family {
                "Install Sodium + Lithium".into()
            } else {
                "Install Embeddium + performance mods".into()
            },
            detail: if fabric_family {
                "Sodium (rendering), Lithium (game logic) and FerriteCore (memory) typically more than double FPS and cut RAM use. quartz can add them in one click.".into()
            } else {
                "For Forge, Embeddium + FerriteCore + ModernFix give large rendering and memory gains.".into()
            },
            estimated_fps_gain: Some(2.4),
        });
    }

    if !ram_sufficient {
        recs.push(Recommendation {
            severity: RecommendationSeverity::Critical,
            title: format!("Increase allocated RAM to {:.1} GB", recommended_ram_mb as f64 / 1024.0),
            detail: format!(
                "This workload needs ~{:.1} GB but only {:.1} GB is allocated. Too little heap causes constant garbage-collection stutter.",
                ram_needed / 1024.0,
                w.allocated_ram_mb as f64 / 1024.0
            ),
            estimated_fps_gain: None,
        });
    } else if w.allocated_ram_mb as f64 > ram_needed * 2.2 && w.allocated_ram_mb >= 8192 {
        recs.push(Recommendation {
            severity: RecommendationSeverity::Warning,
            title: "Lower allocated RAM".into(),
            detail: format!(
                "Allocating {:.1} GB is far more than this pack needs (~{:.1} GB). Oversized heaps cause longer GC pauses. {:.1} GB is ideal.",
                w.allocated_ram_mb as f64 / 1024.0,
                ram_needed / 1024.0,
                recommended_ram_mb as f64 / 1024.0
            ),
            estimated_fps_gain: None,
        });
    }

    if bottleneck == Bottleneck::Gpu && w.shaders {
        recs.push(Recommendation {
            severity: RecommendationSeverity::Tip,
            title: "Use a lighter shader preset".into(),
            detail: "Shaders are your bottleneck. Switch to a performance shader or lower shadow quality to recover a large amount of FPS.".into(),
            estimated_fps_gain: Some(1.7),
        });
    }

    if bottleneck == Bottleneck::Gpu && w.render_distance > 12 {
        recs.push(Recommendation {
            severity: RecommendationSeverity::Tip,
            title: "Reduce render distance to 12 chunks".into(),
            detail: format!(
                "Render distance {} is GPU-heavy. Dropping toward 12 chunks raises FPS significantly with little visual impact.",
                w.render_distance
            ),
            estimated_fps_gain: Some(1.3),
        });
    }

    if bottleneck == Bottleneck::Cpu
        || worldgen_risk(hw.cpu.single_thread_score, w, ram_sufficient) == RiskLevel::High
    {
        recs.push(Recommendation {
            severity: RecommendationSeverity::Info,
            title: "CPU is the limiting factor".into(),
            detail: "Minecraft is single-thread heavy. Lithium/C2ME help, and lowering simulation distance reduces CPU load during exploration.".into(),
            estimated_fps_gain: None,
        });
    }

    if hw.storage == StorageKind::Hdd {
        recs.push(Recommendation {
            severity: RecommendationSeverity::Warning,
            title: "Move the instance to an SSD".into(),
            detail: "You appear to be on a hard drive. SSDs dramatically cut launch time and chunk-load stutter.".into(),
            estimated_fps_gain: None,
        });
    }

    recs.push(Recommendation {
        severity: RecommendationSeverity::Info,
        title: "Apply optimised JVM flags".into(),
        detail: "quartz can apply tuned G1GC flags (Aikar-style) for smoother frame times. Enabled by default for new instances.".into(),
        estimated_fps_gain: None,
    });

    if avg_fps < 25 && hw.gpu.integrated {
        recs.push(Recommendation {
            severity: RecommendationSeverity::Warning,
            title: "Integrated graphics detected".into(),
            detail: "A dedicated GPU would give the biggest uplift. Until then keep render distance <= 8, avoid shaders, and rely on Sodium.".into(),
            estimated_fps_gain: None,
        });
    }

    recs
}

#[allow(clippy::too_many_arguments)]
fn build_factors(
    hw: &HardwareProfile,
    w: &WorkloadProfile,
    rd_factor: f64,
    mod_factor: f64,
    opt_factor: f64,
    shader_factor: f64,
    ram_factor: f64,
    gpu_factor: f64,
    cpu_factor: f64,
) -> Vec<PredictionFactor> {
    let as_impact = |mult: f64| clampf(mult.log2() / 2.0, -1.0, 1.0);
    let mut factors = vec![
        PredictionFactor {
            label: "GPU".into(),
            impact: as_impact(gpu_factor),
            detail: format!("{} (score {})", hw.gpu.model, hw.gpu.score),
        },
        PredictionFactor {
            label: "CPU (single-thread)".into(),
            impact: as_impact(cpu_factor),
            detail: format!("{} (score {})", hw.cpu.brand, hw.cpu.single_thread_score),
        },
        PredictionFactor {
            label: "Render distance".into(),
            impact: as_impact(rd_factor),
            detail: format!("{} chunks", w.render_distance),
        },
        PredictionFactor {
            label: "Mod count".into(),
            impact: as_impact(mod_factor),
            detail: format!("{} mods", w.mod_count),
        },
        PredictionFactor {
            label: "Optimization mods".into(),
            impact: as_impact(opt_factor),
            detail: if w.optimization_mods {
                "Present (Sodium-class)".into()
            } else {
                "None installed".into()
            },
        },
        PredictionFactor {
            label: "RAM allocation".into(),
            impact: as_impact(ram_factor),
            detail: if ram_factor >= 1.0 {
                "Sufficient".into()
            } else {
                "Too low — GC thrash".into()
            },
        },
    ];
    if w.shaders {
        factors.push(PredictionFactor {
            label: "Shaders".into(),
            impact: as_impact(shader_factor),
            detail: "Enabled".into(),
        });
    }
    factors.sort_by(|a, b| b.impact.abs().partial_cmp(&a.impact.abs()).unwrap());
    factors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::{score_cpu, score_gpu};

    fn hw() -> HardwareProfile {
        HardwareProfile {
            cpu: score_cpu("AMD Ryzen 5 5600X", 6, 12, 3.7),
            gpu: score_gpu("NVIDIA GeForce RTX 3060", Some(12288)),
            total_ram_mb: 16384,
            os: "Windows 10/11".into(),
            arch: "x86_64".into(),
            storage: StorageKind::Ssd,
            source: HardwareSource::Detected,
        }
    }

    fn work() -> WorkloadProfile {
        WorkloadProfile {
            name: "test".into(),
            mc_version: "1.20.1".into(),
            loader: "fabric".into(),
            mod_count: 100,
            allocated_ram_mb: 4096,
            render_distance: 12,
            shaders: false,
            optimization_mods: false,
            heavy_mods: false,
        }
    }

    #[test]
    fn complete_and_sane() {
        let p = predict_performance(&hw(), &work());
        assert!(p.avg_fps > 0);
        assert!(p.fps_range[0] <= p.avg_fps && p.fps_range[1] >= p.avg_fps);
        assert!(p.low_fps <= p.avg_fps);
        assert!(p.recommended_ram_mb > 0);
    }

    #[test]
    fn optimization_raises_fps() {
        let base = predict_performance(&hw(), &work());
        let mut w = work();
        w.optimization_mods = true;
        let opt = predict_performance(&hw(), &w);
        assert!(opt.avg_fps as f64 > base.avg_fps as f64 * 1.5);
        assert!(base
            .recommendations
            .iter()
            .any(|r| r.title.to_lowercase().contains("sodium") || r.title.to_lowercase().contains("performance")));
    }

    #[test]
    fn shaders_reduce_fps() {
        let mut a = work();
        a.optimization_mods = true;
        let mut b = a.clone();
        b.shaders = true;
        assert!(predict_performance(&hw(), &b).avg_fps < predict_performance(&hw(), &a).avg_fps);
    }

    #[test]
    fn more_mods_reduce_fps() {
        let mut few = work();
        few.mod_count = 30;
        let mut many = work();
        many.mod_count = 400;
        assert!(predict_performance(&hw(), &many).avg_fps < predict_performance(&hw(), &few).avg_fps);
    }

    #[test]
    fn render_distance_reduces_fps() {
        let mut low = work();
        low.render_distance = 8;
        let mut high = work();
        high.render_distance = 24;
        assert!(predict_performance(&hw(), &high).avg_fps < predict_performance(&hw(), &low).avg_fps);
    }

    #[test]
    fn insufficient_ram_is_bottleneck() {
        let mut w = work();
        w.mod_count = 400;
        w.heavy_mods = true;
        w.allocated_ram_mb = 1536;
        let p = predict_performance(&hw(), &w);
        assert!(!p.ram_sufficient);
        assert_eq!(p.bottleneck, Bottleneck::Ram);
        assert!(p.recommendations.iter().any(|r| r.severity == RecommendationSeverity::Critical));
    }

    #[test]
    fn stronger_hardware_higher_fps() {
        let weak = HardwareProfile {
            gpu: score_gpu("Intel UHD Graphics 630", None),
            cpu: score_cpu("Intel Core i5-8400", 6, 6, 2.8),
            ..hw()
        };
        let strong = HardwareProfile {
            gpu: score_gpu("RTX 4090", None),
            cpu: score_cpu("AMD Ryzen 7 7800X3D", 8, 16, 4.2),
            ..hw()
        };
        let mut w = work();
        w.optimization_mods = true;
        let pw = predict_performance(&weak, &w);
        let ps = predict_performance(&strong, &w);
        assert!(ps.avg_fps > pw.avg_fps);
        assert!(ps.rating_score > pw.rating_score);
    }

    #[test]
    fn igpu_heavy_pack_rated_poorly() {
        let weak = HardwareProfile {
            gpu: score_gpu("Intel UHD Graphics 630", None),
            total_ram_mb: 8192,
            ..hw()
        };
        let mut w = work();
        w.mod_count = 250;
        w.heavy_mods = true;
        w.shaders = true;
        let p = predict_performance(&weak, &w);
        assert!(matches!(p.rating, PerfRating::Unplayable | PerfRating::Choppy));
    }

    #[test]
    fn confidence_levels() {
        let known = predict_performance(&hw(), &work());
        let unknown_hw = HardwareProfile {
            gpu: score_gpu("Unknown GPU", None),
            cpu: score_cpu("Unknown CPU", 4, 8, 3.0),
            source: HardwareSource::Estimated,
            ..hw()
        };
        let unknown = predict_performance(&unknown_hw, &work());
        assert_eq!(known.confidence, Confidence::High);
        assert_ne!(unknown.confidence, Confidence::High);
    }
}
