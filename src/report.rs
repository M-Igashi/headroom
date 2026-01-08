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
    writer.write_record(["Filename", "LUFS", "True Peak (dBTP)", "Headroom (dB)"])
        .context("Failed to write CSV header")?;
    
    // Write data
    for analysis in analyses {
        writer.write_record([
            &analysis.filename,
            &format!("{:.1}", analysis.input_i),
            &format!("{:.1}", analysis.input_tp),
            &format!("{:+.1}", analysis.headroom),
        ]).context("Failed to write CSV record")?;
    }
    
    writer.flush().context("Failed to flush CSV")?;
    
    Ok(output_path)
}

pub fn print_table(analyses: &[AudioAnalysis]) {
    use console::Style;
    
    let header_style = Style::new().bold().cyan();
    let positive_style = Style::new().green();
    let negative_style = Style::new().red();
    let neutral_style = Style::new().dim();
    
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
        "{} {:>8} {:>12} {:>10}",
        header_style.apply_to(format!("{:<width$}", "Filename", width = filename_width)),
        header_style.apply_to("LUFS"),
        header_style.apply_to("True Peak"),
        header_style.apply_to("Headroom"),
    );
    println!("{}", "─".repeat(filename_width + 8 + 12 + 10 + 6));
    
    // Print rows
    for analysis in analyses {
        let headroom_str = format!("{:+.1} dB", analysis.headroom);
        let headroom_styled = if analysis.headroom > 0.0 {
            positive_style.apply_to(headroom_str)
        } else if analysis.headroom < 0.0 {
            negative_style.apply_to(headroom_str)
        } else {
            neutral_style.apply_to(headroom_str)
        };
        
        println!(
            "{:<width$} {:>8.1} {:>10.1} dBTP {:>10}",
            analysis.filename,
            analysis.input_i,
            analysis.input_tp,
            headroom_styled,
            width = filename_width,
        );
    }
    println!();
}

pub fn print_summary(analyses: &[AudioAnalysis]) {
    use console::Style;
    
    let info_style = Style::new().cyan();
    
    let positive_count = analyses.iter().filter(|a| a.headroom > 0.0).count();
    let negative_count = analyses.iter().filter(|a| a.headroom < 0.0).count();
    let zero_count = analyses.iter().filter(|a| a.headroom == 0.0).count();
    
    println!(
        "{} {} files with positive headroom (can be boosted)",
        info_style.apply_to("▸"),
        positive_count
    );
    println!(
        "{} {} files already at or above -1 dBTP",
        info_style.apply_to("▸"),
        negative_count + zero_count
    );
}
