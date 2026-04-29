mod analyzer;
mod args;
mod cli;
mod processor;
mod report;
mod scanner;
mod updater;

use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}
