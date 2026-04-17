mod cli;
mod config;
mod handlers;
mod makefile;
mod picker;
mod prompt;
mod shell;
mod ssh;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    handlers::dispatch(cli.command)
}
