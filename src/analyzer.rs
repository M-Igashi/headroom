use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

const TARGET_TRUE_PEAK: f64 = -1.0;

#[derive(Debug, Clone)]
pub struct AudioAnalysis {
    pub filename: String,
    pub input_i: f64,      // Integrated loudness (LUFS)
    pub input_tp: f64,     // True peak (dBTP)
    pub headroom: f64,     // Available gain (dB)
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
    
    let headroom = TARGET_TRUE_PEAK - input_tp;

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
    })
}

pub fn check_ffmpeg() -> Result<()> {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .context("ffmpeg not found. Please install ffmpeg first.")?;
    Ok(())
}
