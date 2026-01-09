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
    writer.write_record(["Filename", "Format", "LUFS", "True Peak (dBTP)", "Target (dBTP)", "Headroom (dB)", "Effective Gain (dB)"])
        .context("Failed to write CSV header")?;
    
    // Write data
    for analysis in analyses {
        let format = if analysis.is_mp3 { "MP3" } else { "Lossless" };
        writer.write_record([
            &analysis.filename,
            format,
            &format!("{:.1}", analysis.input_i),
            &format!("{:.1}", analysis.input_tp),
            &format!("{:.1}", analysis.target_tp),
            &format!("{:+.1}", analysis.headroom),
            &format!("{:+.1}", analysis.effective_gain),
        ]).context("Failed to write CSV record")?;
    }
    
    writer.flush().context("Failed to flush CSV")?;
    
    Ok(output_path)
}

pub fn print_table(analyses: &[AudioAnalysis]) {
    use console::Style;
    
    let header_style = Style::new().bold().cyan();
    let value_style = Style::new().green();
    let mp3_style = Style::new().yellow();
    let target_style = Style::new().dim();
    
    // Calculate column widths
    let filename_width = analyses
        .iter()
        .map(|a| a.filename.len())
        .max()
        .unwrap_or(8)
        .max(8)
        .min(40); // Cap at 40 chars
    
    // Check if any MP3 files
    let has_mp3 = analyses.iter().any(|a| a.is_mp3);
    
    // Print header
    println!();
    if has_mp3 {
        println!(
            "{} {:>8} {:>12} {:>10} {:>12} {:>14}",
            header_style.apply_to(format!("{:<width$}", "Filename", width = filename_width)),
            header_style.apply_to("LUFS"),
            header_style.apply_to("True Peak"),
            header_style.apply_to("Target"),
            header_style.apply_to("Headroom"),
            header_style.apply_to("Effective"),
        );
    } else {
        println!(
            "{} {:>8} {:>12} {:>10} {:>12}",
            header_style.apply_to(format!("{:<width$}", "Filename", width = filename_width)),
            header_style.apply_to("LUFS"),
            header_style.apply_to("True Peak"),
            header_style.apply_to("Target"),
            header_style.apply_to("Headroom"),
        );
    }
    
    let separator_len = if has_mp3 {
        filename_width + 8 + 12 + 10 + 12 + 14 + 10
    } else {
        filename_width + 8 + 12 + 10 + 12 + 8
    };
    println!("{}", "─".repeat(separator_len));
    
    // Print rows
    for analysis in analyses {
        let display_name: String = if analysis.filename.len() > filename_width {
            format!("{}…", &analysis.filename[..filename_width-1])
        } else {
            analysis.filename.clone()
        };
        
        let headroom_str = format!("{:+.1} dB", analysis.headroom);
        let target_str = format!("{:.1}", analysis.target_tp);
        
        if has_mp3 {
            let effective_str = if analysis.is_mp3 {
                format!("{:+.1} dB", analysis.effective_gain)
            } else {
                format!("{:+.1} dB", analysis.effective_gain)
            };
            
            let effective_display = if analysis.is_mp3 && analysis.effective_gain != analysis.headroom {
                mp3_style.apply_to(effective_str)
            } else {
                value_style.apply_to(effective_str)
            };
            
            println!(
                "{:<width$} {:>8.1} {:>10.1} dBTP {:>8} dBTP {:>12} {:>14}",
                display_name,
                analysis.input_i,
                analysis.input_tp,
                target_style.apply_to(target_str),
                value_style.apply_to(headroom_str),
                effective_display,
                width = filename_width,
            );
        } else {
            println!(
                "{:<width$} {:>8.1} {:>10.1} dBTP {:>8} dBTP {:>12}",
                display_name,
                analysis.input_i,
                analysis.input_tp,
                target_style.apply_to(target_str),
                value_style.apply_to(headroom_str),
                width = filename_width,
            );
        }
    }
    println!();
}
