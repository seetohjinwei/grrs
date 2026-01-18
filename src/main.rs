use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser)]
struct Args {
    pattern: String,
    paths: Vec<std::path::PathBuf>,

    // Flags
    #[arg(short = 'd', long = "max-depth", default_value_t = u32::MAX - 1, help = "Limits the depth of directory traversal. -1 (default) to disable the maximum. 0 to disable recursion.")]
    max_depth: u32,
    #[arg(short = 'N', long = "no-line-number", default_value_t = false)]
    no_line_numbers: bool,
    #[arg(
        short = 'i',
        long = "ignore-case",
        default_value_t = false,
        help = "ignore case"
    )]
    ignore_case: bool,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    let paths = if args.paths.len() == 0 {
        vec![std::path::PathBuf::from(".")]
    } else {
        args.paths
    };
    // TODO: Respect gitignore!
    let walker = grrs::file::Walker::new();
    let file_paths = walker.collect_file_paths(paths, args.max_depth)?;

    // TODO: Parallelize this loop
    // but since we will be sharing std::io::stdout, we will have to create separate writers
    // and flush each writer in a single operation
    for file_path in file_paths {
        let f = std::fs::File::open(&file_path)
            .with_context(|| format!("could not read file {:?}", file_path))?;
        let reader = std::io::BufReader::new(f);

        // header will only be printed if something was actually written
        let header = format!("{}:", file_path.display().to_string());
        let writer = grrs::writer::LazyWriter::new(std::io::stdout(), header);

        match grrs::matcher::find_matches(
            reader,
            writer,
            &args.pattern,
            &grrs::matcher::MatchOptions {
                show_line_numbers: !args.no_line_numbers,
                case_insensitive: args.ignore_case,
            },
        ) {
            Ok(_) => {}
            Err(err) => eprintln!(
                "failed to read {}: {}",
                file_path.display().to_string(),
                err.root_cause()
            ),
        };
    }

    Ok(())
}
