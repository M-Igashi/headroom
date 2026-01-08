use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::analyzer::AudioAnalysis;

pub struct ProcessResult {
    pub original_path: PathBuf,
    pub backup_path: Option<PathBuf>,
    pub gain_applied: f64,
    pub success: bool,
    pub error: Option<String>,
}

pub fn create_backup_dir(base_dir: &Path) -> Result<PathBuf> {
    let backup_dir = base_dir.join("backup");
    fs::create_dir_all(&backup_dir)
        .context("Failed to create backup directory")?;
    Ok(backup_dir)
}

pub fn backup_file(file_path: &Path, backup_dir: &Path) -> Result<PathBuf> {
    let filename = file_path
        .file_name()
        .ok_or_else(|| anyhow!("Invalid filename"))?;
    let backup_path = backup_dir.join(filename);
    
    fs::copy(file_path, &backup_path)
        .context("Failed to backup file")?;
    
    Ok(backup_path)
}

pub fn apply_gain(file_path: &Path, gain_db: f64) -> Result<()> {
    let temp_path = file_path.with_extension("tmp.wav");
    
    // Determine output format based on extension
    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("wav")
        .to_lowercase();
    
    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        file_path.to_str().ok_or_else(|| anyhow!("Invalid path"))?.to_string(),
        "-af".to_string(),
        format!("volume={}dB", gain_db),
    ];
    
    // Add format-specific encoding options
    match extension.as_str() {
        "flac" => {
            args.extend([
                "-c:a".to_string(),
                "flac".to_string(),
            ]);
        }
        "aiff" | "aif" => {
            args.extend([
                "-c:a".to_string(),
                "pcm_s24be".to_string(),
            ]);
        }
        "wav" => {
            args.extend([
                "-c:a".to_string(),
                "pcm_s24le".to_string(),
            ]);
        }
        _ => {}
    }
    
    args.push(temp_path.to_str().ok_or_else(|| anyhow!("Invalid temp path"))?.to_string());
    
    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .context("Failed to execute ffmpeg for gain adjustment")?;
    
    if !output.status.success() {
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
    backup_dir: Option<&Path>,
) -> ProcessResult {
    let mut result = ProcessResult {
        original_path: file_path.to_path_buf(),
        backup_path: None,
        gain_applied: analysis.headroom,
        success: false,
        error: None,
    };
    
    // Skip if no positive headroom
    if analysis.headroom <= 0.0 {
        result.success = true;
        result.error = Some("No positive headroom, skipped".to_string());
        return result;
    }
    
    // Backup if requested
    if let Some(backup) = backup_dir {
        match backup_file(file_path, backup) {
            Ok(path) => result.backup_path = Some(path),
            Err(e) => {
                result.error = Some(format!("Backup failed: {}", e));
                return result;
            }
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
