use console::style;
use std::time::Duration;
use update_informer::{registry, Check};

const PKG_NAME: &str = "M-Igashi/headroom";
const CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60 * 24);

pub fn check_and_notify() {
    if std::env::var_os("HEADROOM_NO_UPDATE_CHECK").is_some() {
        return;
    }

    let current = env!("CARGO_PKG_VERSION");
    let informer = update_informer::new(registry::GitHub, PKG_NAME, current).interval(CHECK_INTERVAL);

    let Ok(Some(new_version)) = informer.check_version() else {
        return;
    };

    let new_str = new_version.to_string();
    let stripped = new_str.strip_prefix('v').unwrap_or(&new_str);

    println!(
        "{} Update available: {} → {}",
        style("↑").yellow().bold(),
        style(format!("v{}", current)).dim(),
        style(format!("v{}", stripped)).green().bold()
    );

    for line in update_commands() {
        println!("   {}", style(line).dim());
    }
    println!();
}

fn update_commands() -> Vec<String> {
    if let Some(method) = detect_install_method() {
        return vec![method.command().to_string()];
    }

    let mut lines = Vec::new();
    if cfg!(target_os = "macos") {
        lines.push(InstallMethod::Homebrew.command().to_string());
        lines.push(InstallMethod::Cargo.command().to_string());
    } else if cfg!(target_os = "windows") {
        lines.push(InstallMethod::Winget.command().to_string());
        lines.push(InstallMethod::Cargo.command().to_string());
    } else {
        lines.push(InstallMethod::Cargo.command().to_string());
    }
    lines
}

#[derive(Clone, Copy)]
enum InstallMethod {
    Homebrew,
    Winget,
    Cargo,
}

impl InstallMethod {
    fn command(self) -> &'static str {
        match self {
            InstallMethod::Homebrew => "brew upgrade headroom",
            InstallMethod::Winget => "winget upgrade M-Igashi.headroom",
            InstallMethod::Cargo => "cargo install headroom",
        }
    }
}

fn detect_install_method() -> Option<InstallMethod> {
    let exe = std::env::current_exe().ok()?;
    let path = exe.to_string_lossy().to_lowercase();

    if path.contains("/homebrew/") || path.contains("/cellar/") || path.contains("/linuxbrew/") {
        Some(InstallMethod::Homebrew)
    } else if path.contains("\\winget\\") || path.contains("/winget/") {
        Some(InstallMethod::Winget)
    } else if path.contains("/.cargo/") || path.contains("\\.cargo\\") {
        Some(InstallMethod::Cargo)
    } else {
        None
    }
}
