mod analyzer;
mod cli;
mod processor;
mod report;
mod scanner;

use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}
