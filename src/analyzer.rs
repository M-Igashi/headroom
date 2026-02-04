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

/// Minimum effective gain threshold (dB)
/// Files with less headroom than this are skipped
const MIN_EFFECTIVE_GAIN: f64 = 0.05;

/// Processing method for the file
#[derive(Debug, Clone, PartialEq)]
pub enum GainMethod {
    /// Lossless files processed with ffmpeg volume filter
    FfmpegLossless,
    /// MP3 files with enough headroom for lossless gain (1.5dB steps)
    Mp3Lossless,
    /// MP3 files requiring re-encode for precise gain
    Mp3Reencode,
    /// AAC/M4A files (always require re-encode)
    AacReencode,
    /// No processing needed (no headroom)
    None,
}

#[derive(Debug, Clone)]
pub struct AudioAnalysis {
    pub filename: String,
    pub path: std::path::PathBuf,
    pub input_i: f64,              // Integrated loudness (LUFS)
    pub input_tp: f64,             // True peak (dBTP)
    pub is_mp3: bool,              // Whether file is MP3
    pub is_aac: bool,              // Whether file is AAC/M4A
    pub bitrate_kbps: Option<u32>, // Bitrate for lossy files

    // Gain calculation results
    pub target_tp: f64,          // Target True Peak ceiling for re-encode (dBTP)
    pub headroom: f64,           // Available gain to target_tp (dB)
    pub gain_method: GainMethod, // How this file should be processed
    pub effective_gain: f64,     // Actual gain to apply
    pub mp3_gain_steps: i32,     // For MP3 lossless: number of gain steps
}

impl AudioAnalysis {
    /// Returns true if this file can be processed with lossless methods
    #[allow(dead_code)]
    pub fn can_lossless_process(&self) -> bool {
        matches!(
            self.gain_method,
            GainMethod::FfmpegLossless | GainMethod::Mp3Lossless
        )
    }

    /// Returns true if this file requires re-encoding
    pub fn requires_reencode(&self) -> bool {
        matches!(
            self.gain_method,
            GainMethod::Mp3Reencode | GainMethod::AacReencode
        )
    }

    /// Returns true if this file has any available headroom
    pub fn has_headroom(&self) -> bool {
        !matches!(self.gain_method, GainMethod::None)
    }
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
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            path.to_str()?,
        ])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let probe: FfprobeOutput = serde_json::from_str(&stdout).ok()?;

    probe
        .format
        .bit_rate
        .and_then(|br| br.parse::<u32>().ok())
        .map(|bps| bps / 1000) // Convert to kbps
}

fn get_target_true_peak(is_lossy: bool, bitrate_kbps: Option<u32>) -> f64 {
    if !is_lossy {
        return TARGET_TRUE_PEAK_HIGH_QUALITY;
    }

    match bitrate_kbps {
        Some(kbps) if kbps >= HIGH_BITRATE_THRESHOLD => TARGET_TRUE_PEAK_HIGH_QUALITY,
        _ => TARGET_TRUE_PEAK_LOW_BITRATE,
    }
}

/// Extract loudnorm JSON from ffmpeg stderr output.
/// The loudnorm filter outputs JSON after "[Parsed_loudnorm_0 @" marker.
/// This is more reliable than searching for the first '{' which may match
/// binary data in GEOB/PRIV ID3v2 frames.
fn extract_loudnorm_json(stderr: &str, path: &Path) -> Result<LoudnormOutput> {
    // Look for the loudnorm marker that precedes the JSON output
    let marker = "[Parsed_loudnorm_0 @";

    if let Some(marker_pos) = stderr.find(marker) {
        // Find the JSON block after the marker
        let after_marker = &stderr[marker_pos..];
        if let Some(json_start) = after_marker.find('{') {
            let json_section = &after_marker[json_start..];
            // Find the matching closing brace by counting braces
            // The loudnorm JSON is a simple flat object, but we handle nesting just in case
            let mut depth = 0;
            let mut json_end = None;
            for (i, ch) in json_section.char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            json_end = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(end) = json_end {
                let json_str = &json_section[..=end];
                return serde_json::from_str(json_str).with_context(|| {
                    format!(
                        "Failed to parse loudnorm JSON. Run: ffmpeg -nostdin -i \"{}\" -map 0:a:0 -af loudnorm=print_format=json -f null - 2>&1 | tail -20",
                        path.display()
                    )
                });
            }
        }
    }

    // Fallback: try to find any valid loudnorm JSON (for older ffmpeg versions)
    // Search backwards from the end of output for JSON blocks containing "input_i"
    // This avoids matching binary data in GEOB/PRIV frames that may contain '{' characters
    if let Some(input_i_pos) = stderr.rfind("\"input_i\"") {
        // Find the opening brace before "input_i"
        if let Some(json_start) = stderr[..input_i_pos].rfind('{') {
            let json_section = &stderr[json_start..];
            // Find the matching closing brace
            let mut depth = 0;
            let mut json_end = None;
            for (i, ch) in json_section.char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            json_end = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(end) = json_end {
                let json_str = &json_section[..=end];
                if let Ok(loudnorm) = serde_json::from_str::<LoudnormOutput>(json_str) {
                    return Ok(loudnorm);
                }
            }
        }
    }

    // No valid loudnorm JSON found
    Err(anyhow!(
        "No loudnorm data found in ffmpeg output. \
         This may be caused by:\n\
         1. Problematic ID3v2 metadata (GEOB/PRIV frames from DJ software)\n\
         2. Corrupted or unsupported audio file\n\
         3. Very old ffmpeg version\n\n\
         Try: ffmpeg -nostdin -i \"{}\" -map 0:a:0 -af loudnorm=print_format=json -f null - 2>&1 | tail -30\n\
         Or remove DJ metadata: eyeD3 --remove-all-objects \"{}\"",
        path.display(),
        path.display()
    ))
}

pub fn analyze_file(path: &Path) -> Result<AudioAnalysis> {
    let output = Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-i",
            path.to_str().ok_or_else(|| anyhow!("Invalid path"))?,
            "-map",
            "0:a:0",
            "-af",
            "loudnorm=print_format=json",
            "-f",
            "null",
            "-",
        ])
        .output()
        .context("Failed to execute ffmpeg. Is ffmpeg installed?")?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Extract loudnorm JSON from ffmpeg output
    // The loudnorm filter outputs JSON after "[Parsed_loudnorm_0 @" marker
    let loudnorm: LoudnormOutput = extract_loudnorm_json(&stderr, path)?;

    let input_i: f64 = loudnorm
        .input_i
        .parse()
        .context("Failed to parse input_i")?;
    let input_tp: f64 = loudnorm
        .input_tp
        .parse()
        .context("Failed to parse input_tp")?;

    let is_mp3 = scanner::is_mp3(path);
    let is_aac = scanner::is_aac(path);

    // Get bitrate for lossy files (MP3 and AAC)
    let bitrate_kbps = if is_mp3 || is_aac {
        get_bitrate(path)
    } else {
        None
    };

    let is_lossy = is_mp3 || is_aac;
    let target_tp = get_target_true_peak(is_lossy, bitrate_kbps);
    let headroom = target_tp - input_tp;

    let (gain_method, effective_gain, mp3_gain_steps) = if is_aac {
        // AAC: always requires re-encode
        if headroom >= MIN_EFFECTIVE_GAIN {
            (GainMethod::AacReencode, headroom, 0)
        } else {
            (GainMethod::None, 0.0, 0)
        }
    } else if !is_lossy {
        // Lossless: use ffmpeg
        if headroom >= MIN_EFFECTIVE_GAIN {
            (GainMethod::FfmpegLossless, headroom, 0)
        } else {
            (GainMethod::None, 0.0, 0)
        }
    } else {
        // MP3 file: check if lossless gain is possible
        // Use bitrate-aware ceiling: high bitrate targets -0.5 dBTP, low bitrate targets -1.0 dBTP
        let lossless_ceiling = if bitrate_kbps.unwrap_or(0) >= HIGH_BITRATE_THRESHOLD {
            TARGET_TRUE_PEAK_HIGH_QUALITY // -0.5 dBTP
        } else {
            TARGET_TRUE_PEAK_LOW_BITRATE // -1.0 dBTP
        };
        let lossless_headroom = lossless_ceiling - input_tp;
        let lossless_steps = (lossless_headroom / MP3_GAIN_STEP).floor() as i32;

        if lossless_steps >= 1 {
            // Can use lossless MP3 gain (at least 1.5dB gain possible within bitrate-aware ceiling)
            let effective = lossless_steps as f64 * MP3_GAIN_STEP;
            (GainMethod::Mp3Lossless, effective, lossless_steps)
        } else if headroom >= MIN_EFFECTIVE_GAIN {
            // Has headroom but not enough for lossless, needs re-encode
            (GainMethod::Mp3Reencode, headroom, 0)
        } else {
            (GainMethod::None, 0.0, 0)
        }
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
        is_mp3,
        is_aac,
        bitrate_kbps,
        target_tp,
        headroom,
        gain_method,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Test JSON extraction with GEOB/PRIV frames containing '{' and '}' characters
    /// This reproduces the issue reported in GitHub issue #10
    #[test]
    fn test_extract_loudnorm_json_with_traktor_metadata() {
        let stderr = r#"        encoder         : LAME3.100
        id3v2_priv.TRAKTOR4: DMRT\xf4{\x00\x00\x02\x00\x00\x00RDH 0\x00\x00\x00\x03\x00\x00\x00SKHC\x04\x00\x00\x00\x00\x00\x00\x00i?"\x00DOMF\x04\x00\x00\x00\x00\x00\x00\x00\x14\x0a\xe8\x07NSRV\x04\x00\x00\x00\x00\x00\x00\x00\x07\x00\x00\x00ATAD\xac{\x00\x00\x17\x00\x00\x00BDNA\x04\
        encoder         : Lavf62.3.100
    [Parsed_loudnorm_0 @ 0xc8f448a80] N/A speed=30.3x elapsed=0:00:10.57
    {
    	"input_i" : "-6.83",
    	"input_tp" : "2.55",
    	"input_lra" : "4.40",
    	"input_thresh" : "-16.87",
    	"output_i" : "-23.34",
    	"output_tp" : "-11.73",
    	"output_lra" : "4.30",
    	"output_thresh" : "-33.37",
    	"normalization_type" : "dynamic",
    	"target_offset" : "-0.66"
    }
    [out#0/null @ 0xc8f448300] video:0KiB audio:244683KiB"#;

        let path = PathBuf::from("/test/Habstrakt - Eat Me.mp3");
        let result = extract_loudnorm_json(stderr, &path);

        assert!(result.is_ok(), "Should successfully parse loudnorm JSON");
        let loudnorm = result.unwrap();
        assert_eq!(loudnorm.input_i, "-6.83");
        assert_eq!(loudnorm.input_tp, "2.55");
    }

    /// Test JSON extraction with standard output (no problematic metadata)
    #[test]
    fn test_extract_loudnorm_json_standard() {
        let stderr = r#"    [Parsed_loudnorm_0 @ 0x12345678]
    {
    	"input_i" : "-14.00",
    	"input_tp" : "-1.00",
    	"input_lra" : "5.00",
    	"input_thresh" : "-24.00",
    	"output_i" : "-24.00",
    	"output_tp" : "-2.00",
    	"output_lra" : "5.00",
    	"output_thresh" : "-34.00",
    	"normalization_type" : "dynamic",
    	"target_offset" : "0.00"
    }"#;

        let path = PathBuf::from("/test/normal.mp3");
        let result = extract_loudnorm_json(stderr, &path);

        assert!(result.is_ok());
        let loudnorm = result.unwrap();
        assert_eq!(loudnorm.input_i, "-14.00");
        assert_eq!(loudnorm.input_tp, "-1.00");
    }

    /// Test fallback when marker is not present (older ffmpeg versions)
    #[test]
    fn test_extract_loudnorm_json_fallback() {
        let stderr = r#"Some other output
    {
    	"input_i" : "-10.00",
    	"input_tp" : "0.50",
    	"input_lra" : "3.00",
    	"input_thresh" : "-20.00",
    	"output_i" : "-23.00",
    	"output_tp" : "-10.00",
    	"output_lra" : "3.00",
    	"output_thresh" : "-33.00",
    	"normalization_type" : "dynamic",
    	"target_offset" : "-1.00"
    }
    More output"#;

        let path = PathBuf::from("/test/old_ffmpeg.mp3");
        let result = extract_loudnorm_json(stderr, &path);

        assert!(result.is_ok());
        let loudnorm = result.unwrap();
        assert_eq!(loudnorm.input_i, "-10.00");
        assert_eq!(loudnorm.input_tp, "0.50");
    }
}
