use anyhow::{Context, Result};
use clap::Parser;
use console::{style, Style};
use dialoguer::{theme::ColorfulTheme, Confirm};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::analyzer::{self, AudioAnalysis, GainMethod};
use crate::args::Cli;
use crate::processor;
use crate::report::{self, AnalysisSummary};
use crate::scanner;

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    print_banner();

    // Check ffmpeg
    analyzer::check_ffmpeg()?;

    if cli.is_non_interactive() {
        run_scriptable(&cli)
    } else {
        run_interactive()
    }
}

fn run_interactive() -> Result<()> {
    // Use current directory
    let target_dir = std::env::current_dir().context("Failed to get current directory")?;

    println!(
        "{} Target directory: {}",
        style("▸").cyan(),
        style(target_dir.display()).bold()
    );

    // Scan for audio files
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
        .collect();

    let csv_path = report::generate_csv(&processable_analyses, &target_dir, None)?;
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
        .filter(|a| a.has_headroom() && (!a.requires_reencode() || allow_reencode))
        .collect();

    if files_to_process.is_empty() {
        println!("No files to process.");
        return Ok(());
    }

    // Process files
    process_files(&files_to_process, &target_dir, backup_dir.as_deref())?;

    print_final_summary(&files_to_process);

    Ok(())
}

fn run_scriptable(cli: &Cli) -> Result<()> {
    // Resolve input paths (default: current dir)
    let (files, base_dir) = if cli.paths.is_empty() {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        (scanner::scan_audio_files(&cwd), cwd)
    } else {
        let files = scanner::resolve_inputs(&cli.paths)?;
        let base = common_base_dir(&files)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        (files, base)
    };

    if files.is_empty() {
        println!("{} No audio files matched.", style("⚠").yellow());
        println!(
            "  Supported formats: {}",
            scanner::get_supported_extensions().join(", ")
        );
        return Ok(());
    }

    println!(
        "{} Found {} audio files",
        style("✓").green(),
        style(files.len()).cyan()
    );

    let all_analyses = analyze_files(&files)?;

    let summary = AnalysisSummary::from_analyses(&all_analyses);

    if !summary.has_processable() {
        println!(
            "\n{} No files with enough headroom found.",
            style("ℹ").blue()
        );
        return Ok(());
    }

    report::print_analysis_report(&all_analyses);

    let processable_analyses: Vec<_> = all_analyses.iter().filter(|a| a.has_headroom()).collect();

    // Report generation
    if cli.report_enabled() {
        let explicit_path = cli.report.as_ref().and_then(|p| {
            if p.as_os_str().is_empty() {
                None
            } else {
                Some(p.as_path())
            }
        });
        let csv_path = report::generate_csv(&processable_analyses, &base_dir, explicit_path)?;
        println!(
            "{} Report saved: {}",
            style("✓").green(),
            csv_path.display()
        );
    }

    if cli.analyze_only {
        println!("{} Analyze-only mode; no files modified.", style("ℹ").blue());
        return Ok(());
    }

    let lossless_on = cli.lossless_enabled();
    let reencode_on = cli.reencode_enabled();

    let files_to_process: Vec<_> = all_analyses
        .iter()
        .filter(|a| {
            if !a.has_headroom() {
                return false;
            }
            if a.requires_reencode() {
                reencode_on
            } else {
                lossless_on
            }
        })
        .collect();

    if files_to_process.is_empty() {
        println!("{} No files to process with current flags.", style("ℹ").blue());
        return Ok(());
    }

    // Backup directory resolution
    let backup_dir = if let Some(path) = &cli.backup {
        let dir = if path.as_os_str().is_empty() {
            processor::create_backup_dir(&base_dir)?
        } else {
            std::fs::create_dir_all(path).context("Failed to create backup directory")?;
            path.clone()
        };
        println!("{} Backup directory: {}", style("✓").green(), dir.display());
        Some(dir)
    } else {
        None
    };

    process_files(&files_to_process, &base_dir, backup_dir.as_deref())?;

    print_final_summary(&files_to_process);

    Ok(())
}

fn common_base_dir(files: &[PathBuf]) -> Option<PathBuf> {
    let mut iter = files.iter().filter_map(|f| f.parent().map(Path::to_path_buf));
    let first = iter.next()?;
    let base = iter.fold(first, |acc, p| common_prefix(&acc, &p));
    Some(base)
}

fn common_prefix(a: &Path, b: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for (x, y) in a.components().zip(b.components()) {
        if x == y {
            out.push(x);
        } else {
            break;
        }
    }
    out
}

fn print_final_summary(files_to_process: &[&AudioAnalysis]) {
    println!(
        "\n{} Done! {} files processed.",
        style("✓").green().bold(),
        files_to_process.len()
    );

    for (method, label) in [
        (GainMethod::FfmpegLossless, "lossless files (ffmpeg)"),
        (GainMethod::Mp3Lossless, "MP3 files (native, lossless)"),
        (GainMethod::AacLossless, "AAC/M4A files (native, lossless)"),
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
    if summary.aac_lossless_count > 0 {
        prompt_parts.push(format!(
            "{} AAC/M4A (lossless gain)",
            summary.aac_lossless_count
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
) -> Result<()> {
    let pb = ProgressBar::new(analyses.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} Processing... [{bar:40.cyan/blue}] {pos}/{len}")
            .unwrap()
            .progress_chars("█▓░"),
    );

    for analysis in analyses {
        if let Err(e) = processor::process_file(&analysis.path, analysis, base_dir, backup_dir) {
            pb.println(format!(
                "{} {}: {}",
                style("⚠").yellow(),
                analysis.filename,
                e
            ));
        }
        pb.inc(1);
    }

    pb.finish_and_clear();

    Ok(())
}
