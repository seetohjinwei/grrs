use anyhow::Result;
use clap::{Parser, Subcommand};

mod grep;

#[derive(Parser)]
struct Application {
    #[arg(short = 'v', long = "verbose", default_value_t = false)]
    verbose: bool,

    #[clap(subcommand)]
    program: Program,
}

#[derive(Subcommand)]
enum Program {
    Grep(grep::GrepCommand),
}

pub fn run() -> Result<()> {
    let application = Application::parse();

    // TODO: Set up verbose

    match application.program {
        Program::Grep(cmd) => cmd.run(),
    }
}
