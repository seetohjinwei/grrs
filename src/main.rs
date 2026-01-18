use anyhow::{Context, Result};
use clap::Parser;
use log::{debug};

use std::io::BufRead;

#[derive(Parser)]
struct Args {
    pattern: String,
    path: std::path::PathBuf,
}

fn grep_file(pattern: String, path: &std::path::PathBuf) -> Result<()> {
    debug!("Searching for {} in {:?}", &pattern, &path);

    let f = std::fs::File::open(path).with_context(|| format!("could not read file {:?}", path))?;
    let reader = std::io::BufReader::new(f);

    for line in reader.lines() {
        let message = line.with_context(|| format!("could not read line from {:?}", path))?;
        if message.contains(&pattern) {
            println!("{}", message)
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    grep_file(args.pattern, &args.path)?;

    Ok(())
}
