use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

/// True Peak ceiling for high bitrate files (>= 256 kbps)
/// Based on AES TD1008: "High rate (e.g., 256 kbps) coders may work satisfactorily with as little as −0.5 dB TP"
const TARGET_TRUE_PEAK_HIGH_BITRATE: f64 = -0.5;

/// True Peak ceiling for low bitrate files (< 256 kbps) or unknown bitrate
/// More conservative to account for codec overshoot in lower bitrate encoding
const TARGET_TRUE_PEAK_LOW_BITRATE: f64 = -1.0;

/// Bitrate threshold in kbps
const HIGH_BITRATE_THRESHOLD: u32 = 256;

#[derive(Debug, Clone)]
pub struct AudioAnalysis {
    pub filename: String,
    pub input_i: f64,      // Integrated loudness (LUFS)
    pub input_tp: f64,     // True peak (dBTP)
    pub headroom: f64,     // Available gain (dB)
    pub target_tp: f64,    // Target True Peak ceiling (dBTP)
    pub bit_rate_kbps: Option<u32>, // Detected bitrate (kbps)
}

#[derive(Debug, Deserialize)]
struct LoudnormOutput {
    input_i: String,
    input_tp: String,
    input_lra: String,
    input_thresh: String,
    output_i: String,
    output_tp: String,
    output_lra: String,
    output_thresh: String,
    normalization_type: String,
    target_offset: String,
}

/// Get audio bitrate using ffprobe
fn get_bitrate(path: &Path) -> Option<u32> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-select_streams", "a:0",
            "-show_entries", "stream=bit_rate",
            "-of", "csv=p=0",
            path.to_str()?,
        ])
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bit_rate_str = stdout.trim();
    
    // ffprobe returns bit_rate in bps, convert to kbps
    if let Ok(bps) = bit_rate_str.parse::<u64>() {
        Some((bps / 1000) as u32)
    } else {
        // For some lossless formats, ffprobe may not report bitrate
        // In that case, we assume high quality and use aggressive ceiling
        None
    }
}

/// Determine target True Peak ceiling based on bitrate
fn get_target_true_peak(bit_rate_kbps: Option<u32>, extension: &str) -> f64 {
    // Lossless formats (FLAC, AIFF, WAV) are always high quality
    // They will typically be transcoded to high-bitrate lossy formats
    let is_lossless = matches!(
        extension.to_lowercase().as_str(),
        "flac" | "aiff" | "aif" | "wav"
    );
    
    if is_lossless {
        // Lossless files: use aggressive ceiling (-0.5 dBTP)
        // Rationale: These are master-quality files that will be
        // distributed via high-bitrate streaming (Spotify Premium 320kbps,
        // Apple Music 256kbps AAC, etc.)
        return TARGET_TRUE_PEAK_HIGH_BITRATE;
    }
    
    // For lossy formats, check actual bitrate
    match bit_rate_kbps {
        Some(kbps) if kbps >= HIGH_BITRATE_THRESHOLD => TARGET_TRUE_PEAK_HIGH_BITRATE,
        Some(_) => TARGET_TRUE_PEAK_LOW_BITRATE,
        None => TARGET_TRUE_PEAK_LOW_BITRATE, // Unknown bitrate: be conservative
    }
}

pub fn analyze_file(path: &Path) -> Result<AudioAnalysis> {
    let output = Command::new("ffmpeg")
        .args([
            "-i",
            path.to_str().ok_or_else(|| anyhow!("Invalid path"))?,
            "-af",
            "loudnorm=print_format=json",
            "-f",
            "null",
            "-",
        ])
        .output()
        .context("Failed to execute ffmpeg. Is ffmpeg installed?")?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Extract JSON from ffmpeg output
    let json_start = stderr
        .find('{')
        .ok_or_else(|| anyhow!("No JSON found in ffmpeg output"))?;
    let json_end = stderr
        .rfind('}')
        .ok_or_else(|| anyhow!("Invalid JSON in ffmpeg output"))?;
    
    let json_str = &stderr[json_start..=json_end];
    let loudnorm: LoudnormOutput = serde_json::from_str(json_str)
        .context("Failed to parse loudnorm JSON")?;

    let input_i: f64 = loudnorm.input_i.parse()
        .context("Failed to parse input_i")?;
    let input_tp: f64 = loudnorm.input_tp.parse()
        .context("Failed to parse input_tp")?;
    
    // Get bitrate and determine target ceiling
    let bit_rate_kbps = get_bitrate(path);
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let target_tp = get_target_true_peak(bit_rate_kbps, extension);
    
    let headroom = target_tp - input_tp;

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(AudioAnalysis {
        filename,
        input_i,
        input_tp,
        headroom,
        target_tp,
        bit_rate_kbps,
    })
}

pub fn check_ffmpeg() -> Result<()> {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .context("ffmpeg not found. Please install ffmpeg first.")?;
    Ok(())
}
