use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const LOSSLESS_EXTENSIONS: &[&str] = &["flac", "aiff", "aif", "wav"];
const MP3_EXTENSIONS: &[&str] = &["mp3"];

pub fn scan_audio_files(dir: &Path, include_mp3: bool) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            // Skip macOS AppleDouble/resource fork files
            let filename = e.file_name().to_string_lossy();
            if filename.starts_with("._") {
                return false;
            }
            
            // Check extension
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    let ext_lower = ext.to_lowercase();
                    if LOSSLESS_EXTENSIONS.contains(&ext_lower.as_str()) {
                        return true;
                    }
                    if include_mp3 && MP3_EXTENSIONS.contains(&ext_lower.as_str()) {
                        return true;
                    }
                    false
                })
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

pub fn get_supported_extensions(include_mp3: bool) -> Vec<&'static str> {
    let mut exts: Vec<&str> = LOSSLESS_EXTENSIONS.to_vec();
    if include_mp3 {
        exts.extend(MP3_EXTENSIONS);
    }
    exts
}

pub fn is_mp3(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| MP3_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn is_lossless(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| LOSSLESS_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}
