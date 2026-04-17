use clap::Parser;
use std::path::PathBuf;

/// Audio loudness analyzer and gain adjustment tool.
///
/// Run without arguments for interactive mode in the current directory.
/// Provide paths or any flag to run in non-interactive (scriptable) mode.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Files, directories, or glob patterns to process. Defaults to current directory.
    pub paths: Vec<String>,

    /// Apply lossless gain adjustment (default in non-interactive mode)
    #[arg(long, conflicts_with = "no_lossless")]
    pub lossless: bool,

    /// Skip lossless gain adjustment
    #[arg(long)]
    pub no_lossless: bool,

    /// Apply re-encoding for MP3/AAC files needing precise gain
    #[arg(long, conflicts_with = "no_reencode")]
    pub reencode: bool,

    /// Skip re-encoding (default in non-interactive mode)
    #[arg(long)]
    pub no_reencode: bool,

    /// Create backup before processing (optional DIR; default: <target>/backup)
    #[arg(long, value_name = "DIR", num_args = 0..=1, default_missing_value = "")]
    pub backup: Option<PathBuf>,

    /// Generate CSV report at PATH (default: <target>/headroom_report_<timestamp>.csv)
    #[arg(long, value_name = "PATH", num_args = 0..=1, default_missing_value = "", conflicts_with = "no_report")]
    pub report: Option<PathBuf>,

    /// Skip CSV report
    #[arg(long)]
    pub no_report: bool,

    /// Analyze files only, do not modify anything
    #[arg(long)]
    pub analyze_only: bool,
}

impl Cli {
    /// Returns true if any non-interactive option or path was provided.
    pub fn is_non_interactive(&self) -> bool {
        !self.paths.is_empty()
            || self.lossless
            || self.no_lossless
            || self.reencode
            || self.no_reencode
            || self.backup.is_some()
            || self.report.is_some()
            || self.no_report
            || self.analyze_only
    }

    /// Whether lossless processing is enabled in non-interactive mode (default: true).
    pub fn lossless_enabled(&self) -> bool {
        !self.no_lossless
    }

    /// Whether re-encode processing is enabled in non-interactive mode (default: false).
    pub fn reencode_enabled(&self) -> bool {
        self.reencode && !self.no_reencode
    }

    /// Whether CSV report should be generated in non-interactive mode (default: true).
    pub fn report_enabled(&self) -> bool {
        !self.no_report
    }
}
