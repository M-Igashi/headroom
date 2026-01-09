use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

use crate::scanner;

/// True Peak ceiling for lossless files and high-bitrate (≥256kbps) lossy files
/// Based on AES TD1008: high-rate codecs work satisfactorily with -0.5 dBTP
const TARGET_TRUE_PEAK_HIGH_QUALITY: f64 = -0.5;

/// True Peak ceiling for low-bitrate (<256kbps) lossy files
/// Based on AES TD1008: lower bit rate codecs tend to overshoot peaks more
const TARGET_TRUE_PEAK_LOW_BITRATE: f64 = -1.0;

/// Bitrate threshold in kbps (AES TD1008 uses 256kbps as reference)
const HIGH_BITRATE_THRESHOLD: u32 = 256;

/// MP3 gain step size in dB (fixed by MP3 format specification)
pub const MP3_GAIN_STEP: f64 = 1.5;

#[derive(Debug, Clone)]
pub struct AudioAnalysis {
    pub filename: String,
    pub path: std::path::PathBuf,
    pub input_i: f64,           // Integrated loudness (LUFS)
    pub input_tp: f64,          // True peak (dBTP)
    pub headroom: f64,          // Available gain (dB)
    pub target_tp: f64,         // Target True Peak ceiling (dBTP)
    pub is_mp3: bool,           // Whether file is MP3
    pub bitrate_kbps: Option<u32>, // Bitrate for lossy files
    pub effective_gain: f64,    // Actual gain to apply (for MP3: rounded to 1.5dB steps)
    pub mp3_gain_steps: i32,    // For MP3: number of gain steps to apply
}

#[derive(Debug, Deserialize)]
struct LoudnormOutput {
    input_i: String,
    input_tp: String,
    #[allow(dead_code)]
    input_lra: String,
    #[allow(dead_code)]
    input_thresh: String,
    #[allow(dead_code)]
    output_i: String,
    #[allow(dead_code)]
    output_tp: String,
    #[allow(dead_code)]
    output_lra: String,
    #[allow(dead_code)]
    output_thresh: String,
    #[allow(dead_code)]
    normalization_type: String,
    #[allow(dead_code)]
    target_offset: String,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    bit_rate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    format: FfprobeFormat,
}

fn get_bitrate(path: &Path) -> Option<u32> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            path.to_str()?,
        ])
        .output()
        .ok()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let probe: FfprobeOutput = serde_json::from_str(&stdout).ok()?;
    
    probe.format.bit_rate
        .and_then(|br| br.parse::<u32>().ok())
        .map(|bps| bps / 1000) // Convert to kbps
}

fn get_target_true_peak(is_mp3: bool, bitrate_kbps: Option<u32>) -> f64 {
    if !is_mp3 {
        // Lossless files always use -0.5 dBTP
        return TARGET_TRUE_PEAK_HIGH_QUALITY;
    }
    
    // MP3: check bitrate
    match bitrate_kbps {
        Some(kbps) if kbps >= HIGH_BITRATE_THRESHOLD => TARGET_TRUE_PEAK_HIGH_QUALITY,
        _ => TARGET_TRUE_PEAK_LOW_BITRATE,
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
    
    let is_mp3 = scanner::is_mp3(path);
    
    // Get bitrate for MP3 files
    let bitrate_kbps = if is_mp3 {
        get_bitrate(path)
    } else {
        None
    };
    
    // Determine target ceiling based on format and bitrate
    let target_tp = get_target_true_peak(is_mp3, bitrate_kbps);
    
    let headroom = target_tp - input_tp;
    
    // Calculate effective gain (for MP3, round down to 1.5dB steps)
    let (effective_gain, mp3_gain_steps) = if is_mp3 {
        let steps = (headroom / MP3_GAIN_STEP).floor() as i32;
        let effective = steps as f64 * MP3_GAIN_STEP;
        (effective, steps)
    } else {
        (headroom, 0)
    };

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(AudioAnalysis {
        filename,
        path: path.to_path_buf(),
        input_i,
        input_tp,
        headroom,
        target_tp,
        is_mp3,
        bitrate_kbps,
        effective_gain,
        mp3_gain_steps,
    })
}

pub fn check_ffmpeg() -> Result<()> {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .context("ffmpeg not found. Please install ffmpeg first.")?;
    Ok(())
}

pub fn check_mp3gain() -> Result<()> {
    Command::new("mp3gain")
        .arg("-v")
        .output()
        .context("mp3gain not found. Please install mp3gain first (brew install mp3gain).")?;
    Ok(())
}
