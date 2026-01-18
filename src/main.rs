use anyhow::{Result};
use clap::Parser;

#[derive(Parser)]
struct Args {
    pattern: String,
    path: std::path::PathBuf,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    grrs::find_matches(args.pattern, &args.path)?;

    Ok(())
}
