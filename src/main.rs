use anyhow::{Context, Result};
use clap::Parser;
use log::debug;

#[derive(Parser)]
struct Args {
    pattern: String,
    path: std::path::PathBuf,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    let f = std::fs::File::open(&args.path)
        .with_context(|| format!("could not read file {:?}", &args.path))?;
    let reader = std::io::BufReader::new(f);

    debug!("Searching for {}", &args.pattern);
    grrs::find_matches(reader, std::io::stdout(), &args.pattern)?;

    Ok(())
}
