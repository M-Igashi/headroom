use anyhow::{Context, Result};
use chrono::Local;
use std::path::Path;

use crate::analyzer::AudioAnalysis;

pub fn generate_csv(analyses: &[AudioAnalysis], output_dir: &Path) -> Result<std::path::PathBuf> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("headroom_report_{}.csv", timestamp);
    let output_path = output_dir.join(&filename);
    
    let mut writer = csv::Writer::from_path(&output_path)
        .context("Failed to create CSV file")?;
    
    // Write header
    writer.write_record(["Filename", "LUFS", "True Peak (dBTP)", "Target (dBTP)", "Headroom (dB)"])
        .context("Failed to write CSV header")?;
    
    // Write data
    for analysis in analyses {
        writer.write_record([
            &analysis.filename,
            &format!("{:.1}", analysis.input_i),
            &format!("{:.1}", analysis.input_tp),
            &format!("{:.1}", analysis.target_tp),
            &format!("{:+.1}", analysis.headroom),
        ]).context("Failed to write CSV record")?;
    }
    
    writer.flush().context("Failed to flush CSV")?;
    
    Ok(output_path)
}

pub fn print_table(analyses: &[AudioAnalysis]) {
    use console::Style;
    
    let header_style = Style::new().bold().cyan();
    let value_style = Style::new().green();
    let target_style = Style::new().dim();
    
    // Calculate column widths
    let filename_width = analyses
        .iter()
        .map(|a| a.filename.len())
        .max()
        .unwrap_or(8)
        .max(8);
    
    // Print header
    println!();
    println!(
        "{} {:>8} {:>12} {:>10} {:>12}",
        header_style.apply_to(format!("{:<width$}", "Filename", width = filename_width)),
        header_style.apply_to("LUFS"),
        header_style.apply_to("True Peak"),
        header_style.apply_to("Target"),
        header_style.apply_to("Headroom"),
    );
    println!("{}", "─".repeat(filename_width + 8 + 12 + 10 + 12 + 8));
    
    // Print rows
    for analysis in analyses {
        let headroom_str = format!("{:+.1} dB", analysis.headroom);
        let target_str = format!("{:.1}", analysis.target_tp);
        
        println!(
            "{:<width$} {:>8.1} {:>10.1} dBTP {:>8} dBTP {:>12}",
            analysis.filename,
            analysis.input_i,
            analysis.input_tp,
            target_style.apply_to(target_str),
            value_style.apply_to(headroom_str),
            width = filename_width,
        );
    }
    println!();
}
