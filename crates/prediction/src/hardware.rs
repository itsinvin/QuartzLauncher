//! GPU/CPU benchmark scoring and best-effort hardware detection.
//!
//! The scoring tables mirror `src/lib/prediction.ts` so native and browser modes agree.

use crate::models::{CpuInfo, GpuInfo, GpuTier, HardwareProfile, HardwareSource, StorageKind};

fn clampf(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}

/// (tokens, score, integrated)
const GPU_TABLE: &[(&[&str], u32, bool)] = &[
    (&["4090"], 100, false),
    (&["4080", "super"], 88, false),
    (&["4080"], 84, false),
    (&["4070", "ti", "super"], 74, false),
    (&["4070", "ti"], 70, false),
    (&["4070", "super"], 66, false),
    (&["4070"], 60, false),
    (&["4060", "ti"], 48, false),
    (&["4060"], 42, false),
    (&["3090", "ti"], 82, false),
    (&["3090"], 78, false),
    (&["3080", "ti"], 76, false),
    (&["3080"], 72, false),
    (&["3070", "ti"], 62, false),
    (&["3070"], 58, false),
    (&["3060", "ti"], 50, false),
    (&["3060"], 38, false),
    (&["3050"], 28, false),
    (&["2080", "ti"], 60, false),
    (&["2080", "super"], 54, false),
    (&["2080"], 50, false),
    (&["2070", "super"], 48, false),
    (&["2070"], 44, false),
    (&["2060", "super"], 40, false),
    (&["2060"], 36, false),
    (&["1660", "ti"], 32, false),
    (&["1660", "super"], 30, false),
    (&["1660"], 28, false),
    (&["1650", "super"], 22, false),
    (&["1650"], 18, false),
    (&["1080", "ti"], 48, false),
    (&["1080"], 40, false),
    (&["1070", "ti"], 38, false),
    (&["1070"], 34, false),
    (&["1060"], 26, false),
    (&["1050", "ti"], 16, false),
    (&["1050"], 13, false),
    (&["7900", "xtx"], 92, false),
    (&["7900", "xt"], 82, false),
    (&["7800", "xt"], 66, false),
    (&["7700", "xt"], 56, false),
    (&["7600"], 42, false),
    (&["6950", "xt"], 80, false),
    (&["6900", "xt"], 74, false),
    (&["6800", "xt"], 70, false),
    (&["6800"], 62, false),
    (&["6750", "xt"], 56, false),
    (&["6700", "xt"], 52, false),
    (&["6650", "xt"], 44, false),
    (&["6600", "xt"], 42, false),
    (&["6600"], 36, false),
    (&["6500", "xt"], 20, false),
    (&["5700", "xt"], 44, false),
    (&["5600", "xt"], 38, false),
    (&["rx", "580"], 26, false),
    (&["rx", "570"], 22, false),
    (&["arc", "a770"], 46, false),
    (&["arc", "a750"], 42, false),
    (&["arc", "a580"], 36, false),
    (&["arc", "a380"], 20, false),
    (&["m3", "max"], 78, true),
    (&["m3", "pro"], 55, true),
    (&["m3"], 38, true),
    (&["m2", "ultra"], 90, true),
    (&["m2", "max"], 68, true),
    (&["m2", "pro"], 50, true),
    (&["m2"], 34, true),
    (&["m1", "ultra"], 76, true),
    (&["m1", "max"], 62, true),
    (&["m1", "pro"], 45, true),
    (&["m1"], 30, true),
    (&["m4"], 44, true),
    (&["780m"], 30, true),
    (&["760m"], 24, true),
    (&["680m"], 22, true),
    (&["660m"], 16, true),
    (&["vega", "8"], 9, true),
    (&["vega"], 8, true),
    (&["iris", "xe"], 12, true),
    (&["iris"], 9, true),
    (&["uhd", "770"], 7, true),
    (&["uhd", "630"], 5, true),
    (&["uhd"], 4, true),
    (&["hd", "graphics"], 3, true),
];

/// (tokens, single-thread score)
const CPU_TABLE: &[(&[&str], u32)] = &[
    (&["7800x3d"], 100),
    (&["7950x3d"], 100),
    (&["7900x3d"], 98),
    (&["5800x3d"], 84),
    (&["7950x"], 98),
    (&["7900x"], 95),
    (&["7700x"], 92),
    (&["7700"], 89),
    (&["7600x"], 90),
    (&["7600"], 86),
    (&["5950x"], 82),
    (&["5900x"], 80),
    (&["5800x"], 78),
    (&["5700x"], 74),
    (&["5600x"], 72),
    (&["5600"], 70),
    (&["5500"], 64),
    (&["3900x"], 66),
    (&["3700x"], 64),
    (&["3600x"], 62),
    (&["3600"], 60),
    (&["14900k"], 100),
    (&["14700k"], 97),
    (&["14600k"], 92),
    (&["13900k"], 98),
    (&["13700k"], 94),
    (&["13600k"], 90),
    (&["13400"], 80),
    (&["12900k"], 90),
    (&["12700k"], 86),
    (&["12600k"], 82),
    (&["12400"], 74),
    (&["11700k"], 76),
    (&["11600k"], 72),
    (&["10700k"], 70),
    (&["10600k"], 66),
    (&["10400"], 58),
    (&["9900k"], 66),
    (&["9700k"], 64),
    (&["8700k"], 58),
    (&["m3", "max"], 96),
    (&["m3", "pro"], 92),
    (&["m3"], 90),
    (&["m2", "max"], 86),
    (&["m2", "pro"], 84),
    (&["m2"], 82),
    (&["m1", "max"], 74),
    (&["m1", "pro"], 73),
    (&["m1"], 70),
];

fn normalize(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    // Drop common marketing words, keep alphanumerics and spaces.
    let cleaned = lower
        .replace("nvidia", " ")
        .replace("geforce", " ")
        .replace("amd", " ")
        .replace("radeon", " ")
        .replace("(r)", " ")
        .replace("(tm)", " ")
        .replace("corporation", " ");
    let mut prev_space = false;
    for ch in cleaned.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_space = false;
        } else if !prev_space {
            out.push(' ');
            prev_space = true;
        }
    }
    out.trim().to_string()
}

fn has_token(norm_spaced: &str, token: &str) -> bool {
    // Token must appear as a whole word.
    norm_spaced.split(' ').any(|w| w == token)
}

pub fn tier_from_score(score: u32) -> GpuTier {
    match score {
        s if s >= 70 => GpuTier::Enthusiast,
        s if s >= 48 => GpuTier::High,
        s if s >= 28 => GpuTier::Mainstream,
        s if s >= 14 => GpuTier::Entry,
        _ => GpuTier::Integrated,
    }
}

pub fn score_gpu(model: &str, vram_mb: Option<u32>) -> GpuInfo {
    let norm = normalize(model);
    let mut matched: Option<(u32, bool)> = None;
    for (tokens, score, integrated) in GPU_TABLE {
        if tokens.iter().all(|t| has_token(&norm, t)) {
            matched = Some((*score, *integrated));
            break;
        }
    }

    let lower = model.to_lowercase();
    let vendor = if lower.contains("nvidia")
        || lower.contains("geforce")
        || lower.contains("rtx")
        || lower.contains("gtx")
    {
        "NVIDIA"
    } else if lower.contains("amd") || lower.contains("radeon") || lower.contains(" rx") {
        "AMD"
    } else if lower.contains("intel") || lower.contains("arc") || lower.contains("iris") || lower.contains("uhd") {
        "Intel"
    } else if lower.contains("apple") || lower.contains("m1") || lower.contains("m2") || lower.contains("m3") || lower.contains("m4") {
        "Apple"
    } else {
        "Unknown"
    };

    let model_str = if model.trim().is_empty() {
        "Unknown GPU".to_string()
    } else {
        model.trim().to_string()
    };

    if let Some((score, integrated)) = matched {
        return GpuInfo {
            model: model_str,
            vendor: vendor.to_string(),
            vram_mb,
            score,
            tier: tier_from_score(score),
            integrated,
        };
    }

    // Unknown model: estimate from VRAM, default mid-low.
    let guess = match vram_mb {
        Some(v) => clampf(8.0 + (v as f64 / 1024.0) * 7.0, 8.0, 55.0),
        None => 22.0,
    };
    let integrated = ["iris", "uhd", "vega", "apple", "integrated"]
        .iter()
        .any(|k| lower.contains(k));
    let score = if integrated { guess.min(30.0) } else { guess } as u32;
    GpuInfo {
        model: model_str,
        vendor: vendor.to_string(),
        vram_mb,
        score,
        tier: tier_from_score(guess as u32),
        integrated,
    }
}

pub fn score_cpu(brand: &str, physical_cores: u32, logical_cores: u32, base_ghz: f64) -> CpuInfo {
    let norm = normalize(brand);
    let norm_nospace = norm.replace(' ', "");
    let mut single: Option<u32> = None;
    for (tokens, s) in CPU_TABLE {
        if tokens.iter().all(|t| norm_nospace.contains(t) || has_token(&norm, t)) {
            single = Some(*s);
            break;
        }
    }

    let lower = brand.to_lowercase();
    let vendor = if lower.contains("intel") {
        "Intel"
    } else if lower.contains("amd") || lower.contains("ryzen") {
        "AMD"
    } else if lower.contains("apple") || lower.contains("m1") || lower.contains("m2") || lower.contains("m3") {
        "Apple"
    } else {
        "Unknown"
    };

    let single = single.unwrap_or_else(|| clampf((base_ghz / 5.1) * 60.0 + 18.0, 14.0, 90.0) as u32);

    let cores = physical_cores.max(1) as f64;
    let multi = clampf(
        single as f64 * (1.0 + (cores.log2()) * 0.42),
        single as f64,
        100.0 + cores,
    )
    .min(130.0) as u32;

    CpuInfo {
        brand: if brand.trim().is_empty() {
            "Unknown CPU".to_string()
        } else {
            brand.trim().to_string()
        },
        vendor: vendor.to_string(),
        physical_cores,
        logical_cores: logical_cores.max(physical_cores),
        base_ghz,
        single_thread_score: single,
        multi_thread_score: multi,
    }
}

/// Best-effort GPU model string from the OS (no extra crates; shells out to native tools).
fn detect_gpu_model() -> String {
    use std::process::Command;

    #[cfg(target_os = "linux")]
    {
        if let Ok(out) = Command::new("sh")
            .arg("-c")
            .arg("lspci 2>/dev/null | grep -Ei 'vga|3d|display'")
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(line) = text.lines().next() {
                // Format: "01:00.0 VGA compatible controller: NVIDIA Corporation GA106 [GeForce RTX 3060] (rev a1)"
                if let Some(idx) = line.find(": ") {
                    let rest = &line[idx + 2..];
                    let model = rest.split(": ").last().unwrap_or(rest);
                    // Prefer the bracketed marketing name when present.
                    if let (Some(a), Some(b)) = (model.find('['), model.find(']')) {
                        if b > a {
                            return model[a + 1..b].trim().to_string();
                        }
                    }
                    return model.trim().to_string();
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = Command::new("sh")
            .arg("-c")
            .arg("system_profiler SPDisplaysDataType 2>/dev/null | grep -m1 'Chipset Model:'")
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(line) = text.lines().next() {
                if let Some(idx) = line.find(':') {
                    return line[idx + 1..].trim().to_string();
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(out) = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_VideoController | Select-Object -First 1 -ExpandProperty Name)",
            ])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            let line = text.trim();
            if !line.is_empty() {
                return line.to_string();
            }
        }
    }

    "Unknown GPU".to_string()
}

/// Detect the real hardware profile using sysinfo + native GPU probing.
pub fn detect_hardware() -> HardwareProfile {
    use sysinfo::System;

    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();

    let total_ram_mb = sys.total_memory() / 1024 / 1024;
    let cpus = sys.cpus();
    let logical = cpus.len() as u32;
    let physical = sys.physical_core_count().map(|c| c as u32).unwrap_or(logical.max(1));
    let brand = cpus
        .first()
        .map(|c| c.brand().trim().to_string())
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| "Unknown CPU".to_string());
    let base_ghz = cpus
        .first()
        .map(|c| c.frequency() as f64 / 1000.0)
        .filter(|f| *f > 0.5)
        .unwrap_or(3.5);

    let cpu = score_cpu(&brand, physical, logical, base_ghz);
    let gpu = score_gpu(&detect_gpu_model(), None);

    let os = System::long_os_version().unwrap_or_else(|| System::name().unwrap_or_else(|| "Unknown OS".into()));
    let arch = System::cpu_arch().unwrap_or_else(|| std::env::consts::ARCH.to_string());

    HardwareProfile {
        cpu,
        gpu,
        total_ram_mb,
        os,
        arch,
        storage: StorageKind::Unknown,
        source: HardwareSource::Detected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_ordering() {
        assert!(score_gpu("NVIDIA GeForce RTX 4090", None).score > score_gpu("NVIDIA GeForce RTX 3060", None).score);
        assert!(score_gpu("RTX 3060 Ti", None).score > score_gpu("RTX 3060", None).score);
        assert!(score_gpu("RTX 3060", None).score > score_gpu("Intel UHD Graphics 630", None).score);
    }

    #[test]
    fn gpu_integrated_flag() {
        assert!(score_gpu("Intel Iris Xe Graphics", None).integrated);
        assert!(score_gpu("Apple M1", None).integrated);
        assert!(!score_gpu("NVIDIA GeForce RTX 4070", None).integrated);
    }

    #[test]
    fn gpu_unknown_is_bounded() {
        let g = score_gpu("SomeBrand Mystery GPU 9000", Some(8192));
        assert!(g.score > 0 && g.score < 60);
    }

    #[test]
    fn cpu_high_end() {
        assert!(score_cpu("AMD Ryzen 7 7800X3D", 8, 16, 4.2).single_thread_score > 90);
        assert!(score_cpu("Intel Core i9-14900K", 24, 32, 3.2).single_thread_score > 90);
    }

    #[test]
    fn cpu_fallback() {
        let c = score_cpu("Unknown CPU", 4, 8, 3.0);
        assert!(c.single_thread_score > 10 && c.single_thread_score < 90);
    }
}
