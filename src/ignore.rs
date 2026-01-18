use anyhow::{Context, Result};
use log::debug;
use regex::RegexSet;
use std::io::BufRead;
use std::path::{Path, PathBuf};

pub struct Ignore {
    patterns: RegexSet,
}

// TODO: Support negative matches `!abc.txt` by maintaining a `include_patterns`
// current `patterns` should be `exclude_patterns`

fn glob_to_regex(glob: &str) -> String {
    let mut regex = String::from("^");
    let mut chars = glob.chars();

    if glob.starts_with('/') {
        chars.next();
    } else {
        regex.push_str("(?:.*/)");
    }

    for c in chars {
        match c {
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' => {
                regex.push('\\');
                regex.push(c);
            }
            '*' => regex.push_str(".*"),
            '?' => regex.push_str("."),
            '/' => regex.push_str("/"),
            _ => regex.push(c),
        }
    }

    if glob.ends_with('/') {
        regex.push_str(".*");
    } else {
        regex.push_str("$");
    }

    regex
}

impl Ignore {
    pub fn empty() -> Self {
        let set = RegexSet::empty();
        Self { patterns: set }
    }

    pub fn new(ignore_path: &PathBuf) -> Result<Self> {
        let file_name = ignore_path
            .file_name()
            .and_then(|s| s.to_str())
            .context("failed to get file name")?;

        if file_name != ".gitignore" && file_name != ".ignore" {
            debug!("unsupported ignore file: {}", file_name);

            return Ok(Self::empty());
        }

        let f = std::fs::File::open(&ignore_path)
            .with_context(|| format!("could not read file {:?}", ignore_path))?;
        let reader = std::io::BufReader::new(f);

        let mut patterns = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            patterns.push(glob_to_regex(line));
        }

        let set = RegexSet::new(patterns)?;
        Ok(Self { patterns: set })
    }

    pub fn is_ignored(&self, path: &Path) -> bool {
        self.patterns.is_match(&path.to_string_lossy())
    }
}
