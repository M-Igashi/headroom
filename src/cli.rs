use anyhow::{Context, Result};
use console::{style, Style};
use dialoguer::{Confirm, theme::ColorfulTheme};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::analyzer::{self, AudioAnalysis, MP3_GAIN_STEP};
use crate::processor;
use crate::report;
use crate::scanner;

pub fn run() -> Result<()> {
    print_banner();
    
    // Check ffmpeg
    analyzer::check_ffmpeg()?;
    
    // Use current directory
    let target_dir = std::env::current_dir()
        .context("Failed to get current directory")?;
    
    println!(
        "{} Target directory: {}",
        style("▸").cyan(),
        style(target_dir.display()).bold()
    );
    
    // Ask about MP3 support
    let include_mp3 = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Include MP3 files? (uses mp3gain, 1.5dB steps)")
        .default(false)
        .interact()?;
    
    // Check mp3gain if needed
    if include_mp3 {
        if let Err(_) = analyzer::check_mp3gain() {
            println!(
                "\n{} mp3gain not found. Install with: {}",
                style("⚠").yellow(),
                style("brew install mp3gain").cyan()
            );
            println!("  Continuing without MP3 support...\n");
            return run_scan(&target_dir, false);
        }
    }
    
    run_scan(&target_dir, include_mp3)
}

fn run_scan(target_dir: &std::path::Path, include_mp3: bool) -> Result<()> {
    // Scan for audio files
    let files = scanner::scan_audio_files(target_dir, include_mp3);
    
    if files.is_empty() {
        println!(
            "\n{} No audio files found",
            style("⚠").yellow()
        );
        println!(
            "  Supported formats: {}",
            scanner::get_supported_extensions(include_mp3).join(", ")
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
    
    // Filter to only files with positive effective gain
    // (for MP3, effective_gain is already rounded down to 1.5dB steps)
    let processable: Vec<_> = all_analyses
        .iter()
        .enumerate()
        .filter(|(_, a)| a.effective_gain > 0.0)
        .collect();
    
    if processable.is_empty() {
        println!(
            "\n{} No files with enough headroom found.",
            style("ℹ").blue()
        );
        println!("  All files are already at or above the target ceiling.");
        if include_mp3 {
            println!(
                "  {} MP3 files require at least {:.1} dB headroom (1.5dB steps)",
                style("ℹ").dim(),
                MP3_GAIN_STEP
            );
        }
        return Ok(());
    }
    
    // Extract only processable analyses for display and report
    let processable_analyses: Vec<_> = processable
        .iter()
        .map(|(_, a)| (*a).clone())
        .collect();
    
    // Print results (only processable files)
    report::print_table(&processable_analyses);
    
    // Always export CSV
    let csv_path = report::generate_csv(&processable_analyses, target_dir)?;
    println!(
        "{} Report saved: {}",
        style("✓").green(),
        csv_path.display()
    );
    
    // Summary with ceiling info
    let lossless_count = processable_analyses.iter().filter(|a| !a.is_mp3).count();
    let mp3_high_bitrate = processable_analyses.iter()
        .filter(|a| a.is_mp3 && a.target_tp == -0.5)
        .count();
    let mp3_low_bitrate = processable_analyses.iter()
        .filter(|a| a.is_mp3 && a.target_tp == -1.0)
        .count();
    
    println!(
        "\n{} {} files can be boosted",
        style("ℹ").blue(),
        processable.len()
    );
    
    if lossless_count > 0 {
        println!(
            "  {} {} lossless files → -0.5 dBTP (ffmpeg)",
            style("•").dim(),
            lossless_count
        );
    }
    if mp3_high_bitrate > 0 {
        println!(
            "  {} {} MP3 files (≥256kbps) → -0.5 dBTP (mp3gain, 1.5dB steps)",
            style("•").dim(),
            mp3_high_bitrate
        );
    }
    if mp3_low_bitrate > 0 {
        println!(
            "  {} {} MP3 files (<256kbps) → -1.0 dBTP (mp3gain, 1.5dB steps)",
            style("•").dim(),
            mp3_low_bitrate
        );
    }
    
    // Confirm processing
    if !Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Apply gain adjustment to these files?")
        .default(false)
        .interact()?
    {
        println!("Done. No files were modified.");
        return Ok(());
    }
    
    // Ask about backup
    let create_backup = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Create backup before processing?")
        .default(true)
        .interact()?;
    
    // Create backup directory if needed
    let backup_dir = if create_backup {
        let dir = processor::create_backup_dir(target_dir)?;
        println!(
            "{} Backup directory: {}",
            style("✓").green(),
            dir.display()
        );
        Some(dir)
    } else {
        None
    };
    
    // Process files
    let processable_files: Vec<_> = processable
        .iter()
        .map(|(idx, _)| files[*idx].clone())
        .collect();
    
    process_files(&processable_files, &processable_analyses, target_dir, backup_dir.as_deref())?;
    
    let mp3_count = mp3_high_bitrate + mp3_low_bitrate;
    
    println!(
        "\n{} Done! {} files processed.",
        style("✓").green().bold(),
        processable.len()
    );
    
    if mp3_count > 0 {
        println!(
            "  {} MP3 gain is lossless and reversible (undo info saved in tags)",
            style("ℹ").dim()
        );
    }
    
    Ok(())
}

fn print_banner() {
    let banner_style = Style::new().cyan().bold();
    println!();
    println!("{}", banner_style.apply_to("╭─────────────────────────────────────╮"));
    println!("{}", banner_style.apply_to("│          headroom v0.4.0            │"));
    println!("{}", banner_style.apply_to("│   Audio Loudness Analyzer & Gain    │"));
    println!("{}", banner_style.apply_to("╰─────────────────────────────────────╯"));
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
    
    println!(
        "{} Analyzed {} files",
        style("✓").green(),
        analyses.len()
    );
    
    Ok(analyses)
}

fn process_files(
    files: &[PathBuf],
    analyses: &[AudioAnalysis],
    base_dir: &std::path::Path,
    backup_dir: Option<&std::path::Path>,
) -> Result<()> {
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} Processing... [{bar:40.cyan/blue}] {pos}/{len}")
            .unwrap()
            .progress_chars("█▓░"),
    );
    
    for (file, analysis) in files.iter().zip(analyses.iter()) {
        let result = processor::process_file(file, analysis, base_dir, backup_dir);
        
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
