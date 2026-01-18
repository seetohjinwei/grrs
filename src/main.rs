use anyhow::{Context, Result};
use clap::Parser;
use log::debug;

#[derive(Parser)]
struct Args {
    pattern: String,
    path: Option<std::path::PathBuf>,

    // Flags
    #[arg(short = 'd', long = "max-depth", default_value_t = -1, help = "Limits the depth of directory traversal. -1 (default) to disable the maximum. 0 to disable recursion.")]
    max_depth: i32,
    #[arg(short = 'N', long = "no-line-number", default_value_t = false)]
    no_line_numbers: bool,
    #[arg(short = 'i', long = "ignore-case", default_value_t = false, help = "ignore case")]
    ignore_case: bool,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    let path = args.path.unwrap_or(std::env::current_dir()?);

    let f = std::fs::File::open(&path)
        .with_context(|| format!("could not read file {:?}", path))?;
    let reader = std::io::BufReader::new(f);

    // TODO: Handle directories and max-depth

    debug!("Searching for {} in {:?}", &args.pattern, path);
    grrs::find_matches(
        reader,
        std::io::stdout(),
        &args.pattern,
        &grrs::MatchOptions {
            show_line_numbers: !args.no_line_numbers,
            case_insensitive: args.ignore_case,
        },
    )?;

    Ok(())
}
