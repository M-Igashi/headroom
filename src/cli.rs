use anyhow::{Context, Result};
use console::{style, Style};
use dialoguer::{Confirm, theme::ColorfulTheme};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

use crate::analyzer::{self, AudioAnalysis};
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
    
    // Confirm to proceed
    if !Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Scan this directory?")
        .default(true)
        .interact()?
    {
        println!("Cancelled.");
        return Ok(());
    }
    
    // Scan for audio files
    let files = scanner::scan_audio_files(&target_dir);
    
    if files.is_empty() {
        println!(
            "\n{} No audio files found",
            style("⚠").yellow()
        );
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
    
    // Filter to only files with positive headroom
    let processable: Vec<_> = all_analyses
        .iter()
        .enumerate()
        .filter(|(_, a)| a.headroom > 0.0)
        .collect();
    
    if processable.is_empty() {
        println!(
            "\n{} No files with positive headroom found.",
            style("ℹ").blue()
        );
        println!("  All files are already at or above the target ceiling.");
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
    let csv_path = report::generate_csv(&processable_analyses, &target_dir)?;
    println!(
        "{} Report saved: {}",
        style("✓").green(),
        csv_path.display()
    );
    
    // Summary with ceiling info
    let has_aggressive = processable_analyses.iter().any(|a| a.target_tp == -0.5);
    let has_conservative = processable_analyses.iter().any(|a| a.target_tp == -1.0);
    
    println!(
        "\n{} {} files can be boosted",
        style("ℹ").blue(),
        processable.len()
    );
    
    if has_aggressive && has_conservative {
        println!(
            "  {} Lossless/high-bitrate: -0.5 dBTP ceiling",
            style("•").dim()
        );
        println!(
            "  {} Low-bitrate: -1.0 dBTP ceiling",
            style("•").dim()
        );
    } else if has_aggressive {
        println!(
            "  {} Target ceiling: -0.5 dBTP (lossless/high-bitrate)",
            style("•").dim()
        );
    } else {
        println!(
            "  {} Target ceiling: -1.0 dBTP",
            style("•").dim()
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
        let dir = processor::create_backup_dir(&target_dir)?;
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
    
    process_files(&processable_files, &processable_analyses, &target_dir, backup_dir.as_deref())?;
    
    println!(
        "\n{} Done! {} files processed.",
        style("✓").green().bold(),
        processable.len()
    );
    
    Ok(())
}

fn print_banner() {
    let banner_style = Style::new().cyan().bold();
    println!();
    println!("{}", banner_style.apply_to("╭─────────────────────────────────────╮"));
    println!("{}", banner_style.apply_to("│          headroom v0.2.0            │"));
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
    
    let mut analyses = Vec::new();
    
    for file in files {
        match analyzer::analyze_file(file) {
            Ok(analysis) => analyses.push(analysis),
            Err(e) => {
                pb.println(format!(
                    "{} Failed to analyze {}: {}",
                    style("⚠").yellow(),
                    file.display(),
                    e
                ));
            }
        }
        pb.inc(1);
    }
    
    pb.finish_and_clear();
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
