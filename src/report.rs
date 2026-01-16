use anyhow::{Context, Result};
use chrono::Local;
use console::Style;
use std::path::Path;

use crate::analyzer::{AudioAnalysis, GainMethod};

pub fn generate_csv(analyses: &[AudioAnalysis], output_dir: &Path) -> Result<std::path::PathBuf> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("headroom_report_{}.csv", timestamp);
    let output_path = output_dir.join(&filename);

    let mut writer = csv::Writer::from_path(&output_path).context("Failed to create CSV file")?;

    // Write header
    writer
        .write_record([
            "Filename",
            "Format",
            "Bitrate (kbps)",
            "LUFS",
            "True Peak (dBTP)",
            "Target (dBTP)",
            "Headroom (dB)",
            "Method",
            "Effective Gain (dB)",
        ])
        .context("Failed to write CSV header")?;

    // Write data
    for analysis in analyses {
        let format = if analysis.is_mp3 {
            "MP3"
        } else if analysis.is_aac {
            "AAC"
        } else {
            "Lossless"
        };
        let bitrate = analysis
            .bitrate_kbps
            .map(|b| b.to_string())
            .unwrap_or_else(|| "-".to_string());
        let method = match analysis.gain_method {
            GainMethod::FfmpegLossless => "ffmpeg",
            GainMethod::Mp3Lossless => "native",
            GainMethod::Mp3Reencode => "re-encode",
            GainMethod::AacReencode => "re-encode",
            GainMethod::None => "none",
        };

        writer
            .write_record([
                &analysis.filename,
                format,
                &bitrate,
                &format!("{:.1}", analysis.input_i),
                &format!("{:.1}", analysis.input_tp),
                &format!("{:.1}", analysis.target_tp),
                &format!("{:+.1}", analysis.headroom),
                method,
                &format!("{:+.1}", analysis.effective_gain),
            ])
            .context("Failed to write CSV record")?;
    }

    writer.flush().context("Failed to flush CSV")?;

    Ok(output_path)
}

pub fn print_analysis_report(analyses: &[AudioAnalysis]) {
    let header_style = Style::new().bold().cyan();
    let lossless_style = Style::new().green();
    let mp3_lossless_style = Style::new().yellow();
    let reencode_style = Style::new().magenta();
    let dim_style = Style::new().dim();

    let filter_by_method = |method: GainMethod| -> Vec<_> {
        analyses
            .iter()
            .filter(|a| a.gain_method == method)
            .collect()
    };

    let lossless_files = filter_by_method(GainMethod::FfmpegLossless);
    let mp3_lossless_files = filter_by_method(GainMethod::Mp3Lossless);
    let mp3_reencode_files = filter_by_method(GainMethod::Mp3Reencode);
    let aac_reencode_files = filter_by_method(GainMethod::AacReencode);

    // Calculate column width (use character count, not byte count)
    let all_processable: Vec<_> = analyses.iter().filter(|a| a.has_headroom()).collect();
    let filename_width = all_processable
        .iter()
        .map(|a| a.filename.chars().count())
        .max()
        .unwrap_or(8)
        .clamp(8, 40);

    println!();

    // Print lossless files section
    if !lossless_files.is_empty() {
        println!(
            "{} {} lossless files (ffmpeg, precise gain)",
            lossless_style.apply_to("●"),
            header_style.apply_to(format!("{}", lossless_files.len()))
        );
        print_file_table(&lossless_files, filename_width, &lossless_style);
        println!();
    }

    // Print MP3 lossless gain section
    if !mp3_lossless_files.is_empty() {
        println!(
            "{} {} MP3 files (native lossless, 1.5dB steps, target: -2.0 dBTP)",
            mp3_lossless_style.apply_to("●"),
            header_style.apply_to(format!("{}", mp3_lossless_files.len()))
        );
        print_file_table(&mp3_lossless_files, filename_width, &mp3_lossless_style);
        println!();
    }

    // Print MP3 re-encode section
    if !mp3_reencode_files.is_empty() {
        println!(
            "{} {} MP3 files (re-encode required for precise gain)",
            reencode_style.apply_to("●"),
            header_style.apply_to(format!("{}", mp3_reencode_files.len()))
        );
        print_file_table(&mp3_reencode_files, filename_width, &reencode_style);
        println!();
    }

    // Print AAC re-encode section
    if !aac_reencode_files.is_empty() {
        println!(
            "{} {} AAC/M4A files (re-encode required)",
            reencode_style.apply_to("●"),
            header_style.apply_to(format!("{}", aac_reencode_files.len()))
        );
        print_file_table(&aac_reencode_files, filename_width, &reencode_style);
        println!();
    }

    // Summary
    let total = lossless_files.len()
        + mp3_lossless_files.len()
        + mp3_reencode_files.len()
        + aac_reencode_files.len();
    if total == 0 {
        println!(
            "{} No files with available headroom found.",
            dim_style.apply_to("ℹ")
        );
    }
}

fn print_file_table(files: &[&AudioAnalysis], filename_width: usize, accent_style: &Style) {
    let dim_style = Style::new().dim();

    // Print header
    println!(
        "  {:<width$} {:>8} {:>12} {:>10} {:>12}",
        dim_style.apply_to("Filename"),
        dim_style.apply_to("LUFS"),
        dim_style.apply_to("True Peak"),
        dim_style.apply_to("Target"),
        dim_style.apply_to("Gain"),
        width = filename_width,
    );

    // Print rows
    for analysis in files {
        // Use character count instead of byte count to handle multi-byte UTF-8 characters
        let char_count = analysis.filename.chars().count();
        let display_name: String = if char_count > filename_width {
            let truncated: String = analysis.filename.chars().take(filename_width - 1).collect();
            format!("{}…", truncated)
        } else {
            analysis.filename.clone()
        };

        let gain_str = format!("{:+.1} dB", analysis.effective_gain);
        let target_str = format!("{:.1}", analysis.target_tp);

        println!(
            "  {:<width$} {:>8.1} {:>10.1} dBTP {:>8} dBTP {:>12}",
            display_name,
            analysis.input_i,
            analysis.input_tp,
            dim_style.apply_to(target_str),
            accent_style.apply_to(gain_str),
            width = filename_width,
        );
    }
}

/// Get summary counts for display
pub struct AnalysisSummary {
    pub lossless_count: usize,
    pub mp3_lossless_count: usize,
    pub mp3_reencode_count: usize,
    pub aac_reencode_count: usize,
}

impl AnalysisSummary {
    pub fn from_analyses(analyses: &[AudioAnalysis]) -> Self {
        let count =
            |method: GainMethod| analyses.iter().filter(|a| a.gain_method == method).count();

        Self {
            lossless_count: count(GainMethod::FfmpegLossless),
            mp3_lossless_count: count(GainMethod::Mp3Lossless),
            mp3_reencode_count: count(GainMethod::Mp3Reencode),
            aac_reencode_count: count(GainMethod::AacReencode),
        }
    }

    pub fn total_lossless(&self) -> usize {
        self.lossless_count + self.mp3_lossless_count
    }

    pub fn total_reencode(&self) -> usize {
        self.mp3_reencode_count + self.aac_reencode_count
    }

    pub fn total(&self) -> usize {
        self.lossless_count
            + self.mp3_lossless_count
            + self.mp3_reencode_count
            + self.aac_reencode_count
    }

    pub fn has_processable(&self) -> bool {
        self.total() > 0
    }
}
