use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use log::error;

#[derive(Parser)]
struct ProgramArgs {
    #[arg(short = 'v', long = "verbose", default_value_t = false)]
    verbose: bool,

    #[clap(subcommand)]
    subcommand: ProgramSubcommand,
}

#[derive(Subcommand)]
enum ProgramSubcommand {
    Grep(GrepCommand),
}

#[derive(Parser)]
struct GrepCommand {
    pattern: String,
    path: Option<PathBuf>,

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

    let args = ProgramArgs::parse();
    let args = match args.subcommand {
        ProgramSubcommand::Grep(cmd) => cmd,
    };

    let path = args.path.unwrap_or(PathBuf::from("."));

    let thread_pool = grrs::threads::ThreadPool::all_cores();

    let pattern = Arc::new(args.pattern);

    let file_paths = grrs::ignore::walk(path, args.max_depth)?;
    for file_path in file_paths {
        let pattern = Arc::clone(&pattern);

        thread_pool.execute(move || {
            let Ok(f) = std::fs::File::open(&file_path) else {
                error!("could not read file {:?}", file_path);
                return;
            };
            let reader = std::io::BufReader::new(f);

            // header will only be printed if something was actually written
            let header = format!("{}:", file_path.display());
            let writer = grrs::writer::SynchronizedWriter::new(std::io::stdout(), header);

            match grrs::matcher::find_matches(
                reader,
                writer,
                &pattern,
                grrs::matcher::MatchOptions {
                    show_line_numbers: !args.no_line_numbers,
                    case_insensitive: args.ignore_case,
                },
            ) {
                Ok(_) => {}
                Err(err) => error!(
                    "failed to read {}: {}",
                    file_path.display(),
                    err.root_cause()
                ),
            };
        });
    }

    thread_pool.wait();

    Ok(())
}
