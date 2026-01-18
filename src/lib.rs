use log::{debug};
use anyhow::{Context, Result};

use std::io::BufRead;

pub fn find_matches(pattern: String, path: &std::path::PathBuf) -> Result<()> {
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
