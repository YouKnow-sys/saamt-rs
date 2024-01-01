use clap::Parser;

use commands::*;
use reporter::CliReporter;

mod commands;
mod reporter;

#[derive(Debug, Parser)]
#[command(name = "SAAMT CLI", author, about = include_str!("../logo.txt"), version = None)]
pub struct Cli {
    #[command(subcommand)]
    commands: Commands,
    /// Program log level
    #[arg(short, long, value_enum, global = true, default_value_t = LogLevel::All)]
    log_level: LogLevel,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let reporter = CliReporter::new(args.log_level);

    args.commands.command(reporter)
}
