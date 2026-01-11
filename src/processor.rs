use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::analyzer::{AudioAnalysis, GainMethod};


pub struct ProcessResult {
    pub success: bool,
    pub error: Option<String>,
}

pub fn create_backup_dir(base_dir: &Path) -> Result<PathBuf> {
    let backup_dir = base_dir.join("backup");
    fs::create_dir_all(&backup_dir)
        .context("Failed to create backup directory")?;
    Ok(backup_dir)
}

pub fn backup_file(file_path: &Path, base_dir: &Path, backup_dir: &Path) -> Result<PathBuf> {
    // Calculate relative path from base_dir to preserve directory structure
    let relative_path = file_path
        .strip_prefix(base_dir)
        .unwrap_or(file_path.file_name().map(Path::new).unwrap_or(file_path));
    
    let backup_path = backup_dir.join(relative_path);
    
    // Create parent directories if needed
    if let Some(parent) = backup_path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create backup subdirectory")?;
    }
    
    fs::copy(file_path, &backup_path)
        .context("Failed to backup file")?;
    
    Ok(backup_path)
}

/// Apply gain to lossless files using ffmpeg volume filter
pub fn apply_gain_ffmpeg(file_path: &Path, gain_db: f64) -> Result<()> {
    // Create temp file with same extension
    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("wav");
    
    let temp_path = file_path.with_extension(format!("tmp.{}", extension));
    
    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        file_path.to_str().ok_or_else(|| anyhow!("Invalid path"))?.to_string(),
        "-af".to_string(),
        format!("volume={}dB", gain_db),
    ];
    
    // Add format-specific encoding options
    match extension.to_lowercase().as_str() {
        "flac" => {
            args.extend(["-c:a".to_string(), "flac".to_string()]);
        }
        "aiff" | "aif" => {
            args.extend(["-c:a".to_string(), "pcm_s24be".to_string()]);
        }
        "wav" => {
            args.extend(["-c:a".to_string(), "pcm_s24le".to_string()]);
        }
        _ => {}
    }
    
    args.push(temp_path.to_str().ok_or_else(|| anyhow!("Invalid temp path"))?.to_string());
    
    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .context("Failed to execute ffmpeg for gain adjustment")?;
    
    if !output.status.success() {
        // Clean up temp file if it exists
        let _ = fs::remove_file(&temp_path);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("ffmpeg failed: {}", stderr));
    }
    
    // Replace original with processed file
    fs::remove_file(file_path).context("Failed to remove original file")?;
    fs::rename(&temp_path, file_path).context("Failed to rename processed file")?;
    
    Ok(())
}

/// Apply gain to MP3 files using mp3rgain CLI tool (lossless, 1.5dB steps)
pub fn apply_gain_mp3_native(file_path: &Path, gain_steps: i32) -> Result<()> {
    if gain_steps == 0 {
        return Ok(());
    }
    
    let output = Command::new("mp3rgain")
        .args([
            "apply",
            "-g",
            &gain_steps.to_string(),
            file_path.to_str().ok_or_else(|| anyhow!("Invalid path"))?,
        ])
        .output()
        .context("Failed to execute mp3rgain. Is it installed? (brew install M-Igashi/tap/mp3rgain)")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("mp3rgain failed: {}", stderr));
    }
    
    Ok(())
}

/// Apply gain to MP3 files by re-encoding (lossy, but precise control)
pub fn apply_gain_mp3_reencode(file_path: &Path, gain_db: f64, bitrate_kbps: Option<u32>) -> Result<()> {
    let temp_path = file_path.with_extension("tmp.mp3");
    
    // Determine target bitrate (preserve original or use 320k for high quality)
    let bitrate = match bitrate_kbps {
        Some(kbps) if kbps >= 256 => format!("{}k", kbps),
        Some(kbps) => format!("{}k", kbps),
        None => "320k".to_string(),
    };
    
    let args = vec![
        "-y".to_string(),
        "-i".to_string(),
        file_path.to_str().ok_or_else(|| anyhow!("Invalid path"))?.to_string(),
        "-af".to_string(),
        format!("volume={}dB", gain_db),
        "-c:a".to_string(),
        "libmp3lame".to_string(),
        "-b:a".to_string(),
        bitrate,
        "-q:a".to_string(),
        "0".to_string(),  // Highest quality VBR
        temp_path.to_str().ok_or_else(|| anyhow!("Invalid temp path"))?.to_string(),
    ];
    
    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .context("Failed to execute ffmpeg for MP3 re-encode")?;
    
    if !output.status.success() {
        let _ = fs::remove_file(&temp_path);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("ffmpeg MP3 re-encode failed: {}", stderr));
    }
    
    // Replace original with processed file
    fs::remove_file(file_path).context("Failed to remove original file")?;
    fs::rename(&temp_path, file_path).context("Failed to rename processed file")?;
    
    Ok(())
}

pub fn process_file(
    file_path: &Path,
    analysis: &AudioAnalysis,
    base_dir: &Path,
    backup_dir: Option<&Path>,
    allow_reencode: bool,
) -> ProcessResult {
    let mut result = ProcessResult {
        success: false,
        error: None,
    };
    
    // Skip if no effective gain to apply
    if !analysis.has_headroom() {
        result.success = true;
        return result;
    }
    
    // Skip re-encode files if not allowed
    if analysis.requires_reencode() && !allow_reencode {
        result.success = true;
        return result;
    }
    
    // Backup if requested
    if let Some(backup) = backup_dir {
        if let Err(e) = backup_file(file_path, base_dir, backup) {
            result.error = Some(format!("Backup failed: {}", e));
            return result;
        }
    }
    
    // Apply gain based on method
    let apply_result = match analysis.gain_method {
        GainMethod::FfmpegLossless => {
            apply_gain_ffmpeg(file_path, analysis.effective_gain)
        }
        GainMethod::Mp3Lossless => {
            apply_gain_mp3_native(file_path, analysis.mp3_gain_steps)
        }
        GainMethod::Mp3Reencode => {
            apply_gain_mp3_reencode(file_path, analysis.effective_gain, analysis.bitrate_kbps)
        }
        GainMethod::None => Ok(()),
    };
    
    match apply_result {
        Ok(()) => result.success = true,
        Err(e) => {
            result.error = Some(format!("Gain adjustment failed: {}", e));
        }
    }
    
    result
}
