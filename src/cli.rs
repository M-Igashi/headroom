use anyhow::{Context, Result};
use console::{style, Style};
use dialoguer::{Confirm, Input, theme::ColorfulTheme};
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
    
    // Get target directory
    let target_dir = get_target_directory()?;
    
    // Scan for audio files
    let files = scanner::scan_audio_files(&target_dir);
    
    if files.is_empty() {
        println!(
            "\n{} No audio files found in {}",
            style("⚠").yellow(),
            target_dir.display()
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
    
    // Confirm analysis
    if !Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Proceed with analysis?")
        .default(true)
        .interact()?
    {
        println!("Cancelled.");
        return Ok(());
    }
    
    // Analyze files
    let analyses = analyze_files(&files)?;
    
    // Print results
    report::print_table(&analyses);
    report::print_summary(&analyses);
    
    // Export CSV
    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Export CSV report?")
        .default(true)
        .interact()?
    {
        let csv_path = report::generate_csv(&analyses, &target_dir)?;
        println!(
            "{} Saved to {}",
            style("✓").green(),
            csv_path.display()
        );
    }
    
    // Check if there are files to process
    let processable: Vec<_> = analyses
        .iter()
        .enumerate()
        .filter(|(_, a)| a.headroom > 0.0)
        .collect();
    
    if processable.is_empty() {
        println!(
            "\n{} No files with positive headroom to process.",
            style("ℹ").blue()
        );
        return Ok(());
    }
    
    // Ask about gain adjustment
    println!(
        "\n{} {} files can be boosted to -1 dBTP",
        style("ℹ").blue(),
        processable.len()
    );
    
    if !Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Apply gain adjustment to these files?")
        .default(false)
        .interact()?
    {
        println!("Done.");
        return Ok(());
    }
    
    // Ask about backup
    let create_backup = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Create backup before processing?")
        .default(true)
        .interact()?;
    
    // Process files
    let backup_dir = if create_backup {
        Some(processor::create_backup_dir(&target_dir)?)
    } else {
        None
    };
    
    process_files(&files, &analyses, backup_dir.as_deref())?;
    
    println!(
        "\n{} Done! {} files processed.",
        style("✓").green(),
        processable.len()
    );
    
    if let Some(dir) = backup_dir {
        println!(
            "  Backups saved to: {}",
            style(dir.display()).dim()
        );
    }
    
    Ok(())
}

fn print_banner() {
    let banner_style = Style::new().cyan().bold();
    println!();
    println!("{}", banner_style.apply_to("╭─────────────────────────────────────╮"));
    println!("{}", banner_style.apply_to("│          headroom v0.1.0            │"));
    println!("{}", banner_style.apply_to("│   Audio Loudness Analyzer & Gain    │"));
    println!("{}", banner_style.apply_to("╰─────────────────────────────────────╯"));
    println!();
}

fn get_target_directory() -> Result<PathBuf> {
    let input: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Target directory")
        .default(".".to_string())
        .interact_text()?;
    
    let path = PathBuf::from(shellexpand::tilde(&input).to_string());
    
    if !path.exists() {
        anyhow::bail!("Directory does not exist: {}", path.display());
    }
    
    if !path.is_dir() {
        anyhow::bail!("Not a directory: {}", path.display());
    }
    
    Ok(path.canonicalize().context("Failed to resolve path")?)
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
    backup_dir: Option<&std::path::Path>,
) -> Result<()> {
    let processable: Vec<_> = analyses
        .iter()
        .enumerate()
        .filter(|(_, a)| a.headroom > 0.0)
        .collect();
    
    let pb = ProgressBar::new(processable.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} Processing... [{bar:40.cyan/blue}] {pos}/{len}")
            .unwrap()
            .progress_chars("█▓░"),
    );
    
    for (idx, analysis) in &processable {
        let result = processor::process_file(&files[*idx], analysis, backup_dir);
        
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
