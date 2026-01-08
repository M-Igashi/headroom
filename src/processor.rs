use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::analyzer::AudioAnalysis;

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

pub fn apply_gain(file_path: &Path, gain_db: f64) -> Result<()> {
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

pub fn process_file(
    file_path: &Path,
    analysis: &AudioAnalysis,
    base_dir: &Path,
    backup_dir: Option<&Path>,
) -> ProcessResult {
    let mut result = ProcessResult {
        success: false,
        error: None,
    };
    
    // Backup if requested
    if let Some(backup) = backup_dir {
        if let Err(e) = backup_file(file_path, base_dir, backup) {
            result.error = Some(format!("Backup failed: {}", e));
            return result;
        }
    }
    
    // Apply gain
    match apply_gain(file_path, analysis.headroom) {
        Ok(()) => result.success = true,
        Err(e) => {
            result.error = Some(format!("Gain adjustment failed: {}", e));
        }
    }
    
    result
}
