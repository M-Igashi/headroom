use anyhow::{Context, Result};
use console::{style, Style};
use dialoguer::{theme::ColorfulTheme, Confirm};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::analyzer::{self, AudioAnalysis, GainMethod};
use crate::processor;
use crate::report::{self, AnalysisSummary};
use crate::scanner;

pub fn run() -> Result<()> {
    print_banner();

    // Check ffmpeg
    analyzer::check_ffmpeg()?;

    // Use current directory
    let target_dir = std::env::current_dir().context("Failed to get current directory")?;

    println!(
        "{} Target directory: {}",
        style("▸").cyan(),
        style(target_dir.display()).bold()
    );

    // Scan for audio files (always includes MP3)
    let files = scanner::scan_audio_files(&target_dir);

    if files.is_empty() {
        println!("\n{} No audio files found", style("⚠").yellow());
        println!(
            "  Supported formats: {}",
            scanner::get_supported_extensions().join(", ")
        );
        return Ok(());
    }

    println!(
        "\n{} Found {} audio files",
        style("✓").green(),
        style(files.len()).cyan()
    );

    // Analyze files
    let all_analyses = analyze_files(&files)?;

    // Get summary
    let summary = AnalysisSummary::from_analyses(&all_analyses);

    if !summary.has_processable() {
        println!(
            "\n{} No files with enough headroom found.",
            style("ℹ").blue()
        );
        println!("  All files are already at or above the target ceiling.");
        return Ok(());
    }

    // Print categorized report
    report::print_analysis_report(&all_analyses);

    // Export CSV (only processable files)
    let processable_analyses: Vec<_> = all_analyses
        .iter()
        .filter(|a| a.has_headroom())
        .cloned()
        .collect();

    let csv_path = report::generate_csv(&processable_analyses, &target_dir)?;
    println!(
        "{} Report saved: {}",
        style("✓").green(),
        csv_path.display()
    );

    // Process based on available files
    let has_lossless = summary.total_lossless() > 0;
    let has_reencode = summary.total_reencode() > 0;

    // First dialog: Lossless processing
    if has_lossless && !prompt_lossless_processing(&summary)? {
        println!("Done. No files were modified.");
        return Ok(());
    }

    // Second dialog: Re-encode processing (if applicable)
    let allow_reencode = if has_reencode {
        prompt_reencode_processing(&summary)?
    } else {
        false
    };

    // Ask about backup
    let create_backup = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Create backup before processing?")
        .default(true)
        .interact()?;

    // Create backup directory if needed
    let backup_dir = if create_backup {
        let dir = processor::create_backup_dir(&target_dir)?;
        println!("{} Backup directory: {}", style("✓").green(), dir.display());
        Some(dir)
    } else {
        None
    };

    // Filter files to process
    let files_to_process: Vec<_> = all_analyses
        .iter()
        .filter(|a| match a.gain_method {
            GainMethod::FfmpegLossless => true,
            GainMethod::Mp3Lossless => true,
            GainMethod::Mp3Reencode => allow_reencode,
            GainMethod::AacReencode => allow_reencode,
            GainMethod::None => false,
        })
        .collect();

    if files_to_process.is_empty() {
        println!("No files to process.");
        return Ok(());
    }

    // Process files
    process_files(
        &files_to_process,
        &target_dir,
        backup_dir.as_deref(),
        allow_reencode,
    )?;

    // Final summary
    println!(
        "\n{} Done! {} files processed.",
        style("✓").green().bold(),
        files_to_process.len()
    );

    for (method, label) in [
        (GainMethod::FfmpegLossless, "lossless files (ffmpeg)"),
        (GainMethod::Mp3Lossless, "MP3 files (native, lossless)"),
        (GainMethod::Mp3Reencode, "MP3 files (re-encoded)"),
        (GainMethod::AacReencode, "AAC/M4A files (re-encoded)"),
    ] {
        let count = files_to_process
            .iter()
            .filter(|a| a.gain_method == method)
            .count();
        if count > 0 {
            println!("  {} {} {}", style("•").dim(), count, label);
        }
    }

    Ok(())
}

fn prompt_lossless_processing(summary: &AnalysisSummary) -> Result<bool> {
    let mut prompt_parts = Vec::new();

    if summary.lossless_count > 0 {
        prompt_parts.push(format!("{} lossless", summary.lossless_count));
    }
    if summary.mp3_lossless_count > 0 {
        prompt_parts.push(format!(
            "{} MP3 (lossless gain)",
            summary.mp3_lossless_count
        ));
    }

    let prompt = format!(
        "Apply lossless gain adjustment to {} files?",
        prompt_parts.join(" + ")
    );

    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(&prompt)
        .default(false)
        .interact()
        .map_err(Into::into)
}

fn prompt_reencode_processing(summary: &AnalysisSummary) -> Result<bool> {
    let mut reencode_parts = Vec::new();
    if summary.mp3_reencode_count > 0 {
        reencode_parts.push(format!("{} MP3", summary.mp3_reencode_count));
    }
    if summary.aac_reencode_count > 0 {
        reencode_parts.push(format!("{} AAC/M4A", summary.aac_reencode_count));
    }

    println!(
        "\n{} {} files have headroom but require re-encoding for precise gain.",
        style("ℹ").magenta(),
        reencode_parts.join(" + ")
    );
    println!(
        "  {} Re-encoding causes minor quality loss (inaudible at 256kbps+)",
        style("•").dim()
    );
    println!("  {} Original bitrate will be preserved", style("•").dim());

    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Also process these files with re-encoding?")
        .default(false)
        .interact()
        .map_err(Into::into)
}

fn print_banner() {
    let banner_style = Style::new().cyan().bold();
    let version = env!("CARGO_PKG_VERSION");
    let title = format!("headroom v{}", version);
    let padding = (37 - title.len() - 2) / 2;
    let title_line = format!(
        "│{:padding$}{}{:padding$}│",
        "",
        title,
        "",
        padding = padding
    );
    // Ensure exactly 39 chars wide
    let title_line = format!("{:<39}", title_line);
    println!();
    println!(
        "{}",
        banner_style.apply_to("╭─────────────────────────────────────╮")
    );
    println!("{}", banner_style.apply_to(&title_line));
    println!(
        "{}",
        banner_style.apply_to("│   Audio Loudness Analyzer & Gain    │")
    );
    println!(
        "{}",
        banner_style.apply_to("╰─────────────────────────────────────╯")
    );
    println!();
}

fn analyze_files(files: &[PathBuf]) -> Result<Vec<AudioAnalysis>> {
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} Analyzing... [{bar:40.cyan/blue}] {pos}/{len}")
            .unwrap()
            .progress_chars("█▓░"),
    );

    // Thread-safe collection for results with index to preserve order
    let results: Mutex<Vec<(usize, Option<AudioAnalysis>)>> = Mutex::new(Vec::new());
    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());

    // Parallel analysis using rayon
    files.par_iter().enumerate().for_each(|(idx, file)| {
        match analyzer::analyze_file(file) {
            Ok(analysis) => {
                results.lock().unwrap().push((idx, Some(analysis)));
            }
            Err(e) => {
                results.lock().unwrap().push((idx, None));
                errors.lock().unwrap().push(format!(
                    "{} Failed to analyze {}: {}",
                    style("⚠").yellow(),
                    file.display(),
                    e
                ));
            }
        }
        pb.inc(1);
    });

    pb.finish_and_clear();

    // Print any errors
    for err in errors.lock().unwrap().iter() {
        println!("{}", err);
    }

    // Sort by original index and extract successful analyses
    let mut indexed_results = results.into_inner().unwrap();
    indexed_results.sort_by_key(|(idx, _)| *idx);
    let analyses: Vec<AudioAnalysis> = indexed_results
        .into_iter()
        .filter_map(|(_, analysis)| analysis)
        .collect();

    println!("{} Analyzed {} files", style("✓").green(), analyses.len());

    Ok(analyses)
}

fn process_files(
    analyses: &[&AudioAnalysis],
    base_dir: &std::path::Path,
    backup_dir: Option<&std::path::Path>,
    allow_reencode: bool,
) -> Result<()> {
    let pb = ProgressBar::new(analyses.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} Processing... [{bar:40.cyan/blue}] {pos}/{len}")
            .unwrap()
            .progress_chars("█▓░"),
    );

    for analysis in analyses {
        let result = processor::process_file(
            &analysis.path,
            analysis,
            base_dir,
            backup_dir,
            allow_reencode,
        );

        if !result.success {
            if let Some(err) = result.error {
                pb.println(format!(
                    "{} {}: {}",
                    style("⚠").yellow(),
                    analysis.filename,
                    err
                ));
            }
        }
        pb.inc(1);
    }

    pb.finish_and_clear();

    Ok(())
}
