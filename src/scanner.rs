use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const AUDIO_EXTENSIONS: &[&str] = &["flac", "aiff", "aif", "wav"];

pub fn scan_audio_files(dir: &Path) -> Vec<PathBuf> {
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
                .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

pub fn get_supported_extensions() -> &'static [&'static str] {
    AUDIO_EXTENSIONS
}
