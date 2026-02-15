use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use log::error;

#[derive(Parser)]
pub struct GrepCommand {
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

impl GrepCommand {
    pub fn run(self) -> Result<()> {
        let path = self.path.unwrap_or(PathBuf::from("."));

        let thread_pool = grrs::core::threads::ThreadPool::all_cores();

        let pattern = Arc::new(self.pattern);

        let file_paths = grrs::core::ignore::walk(path, self.max_depth)?;
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
                let writer = grrs::core::writer::SynchronizedWriter::new(std::io::stdout(), header);

                match grrs::grep::matcher::find_matches(
                    reader,
                    writer,
                    &pattern,
                    grrs::grep::matcher::MatchOptions {
                        show_line_numbers: !self.no_line_numbers,
                        case_insensitive: self.ignore_case,
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
}
