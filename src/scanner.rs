use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const LOSSLESS_EXTENSIONS: &[&str] = &["flac", "aiff", "aif", "wav"];
const MP3_EXTENSIONS: &[&str] = &["mp3"];
const AAC_EXTENSIONS: &[&str] = &["m4a", "aac", "mp4"];

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
                .map(|ext| {
                    let ext_lower = ext.to_lowercase();
                    LOSSLESS_EXTENSIONS.contains(&ext_lower.as_str())
                        || MP3_EXTENSIONS.contains(&ext_lower.as_str())
                        || AAC_EXTENSIONS.contains(&ext_lower.as_str())
                })
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

pub fn get_supported_extensions() -> Vec<&'static str> {
    let mut exts: Vec<&str> = LOSSLESS_EXTENSIONS.to_vec();
    exts.extend(MP3_EXTENSIONS);
    exts.extend(AAC_EXTENSIONS);
    exts
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn is_mp3(path: &Path) -> bool {
    has_extension(path, MP3_EXTENSIONS)
}

#[allow(dead_code)]
pub fn is_lossless(path: &Path) -> bool {
    has_extension(path, LOSSLESS_EXTENSIONS)
}

pub fn is_aac(path: &Path) -> bool {
    has_extension(path, AAC_EXTENSIONS)
}
