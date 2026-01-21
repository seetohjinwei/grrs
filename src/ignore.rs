use anyhow::{Context, Result};
use log::warn;
use regex::RegexSet;
use std::io::BufRead;
use std::path::{Path, PathBuf};

pub struct GitIgnore {
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
        regex.push_str("(?:^|.*/)");
    }

    for c in chars {
        match c {
            '.' | '+' | '(' | ')' | '[' | ']' | '|' | '^' | '$' | '{' | '}' => {
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

impl GitIgnore {
    pub fn empty() -> Self {
        let set = RegexSet::empty();
        Self { patterns: set }
    }

    pub fn from<R: BufRead>(reader: R) -> Result<Self> {
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

    pub fn new(ignore_path: &PathBuf) -> Result<Self> {
        let file_name = ignore_path
            .file_name()
            .and_then(|s| s.to_str())
            .context("failed to get file name")?;

        if file_name != ".gitignore" && file_name != ".ignore" {
            warn!("unsupported ignore file: {}", file_name);
            return Ok(Self::empty());
        }

        let f = std::fs::File::open(&ignore_path)
            .with_context(|| format!("could not read file {:?}", ignore_path))?;
        let reader = std::io::BufReader::new(f);

        Self::from(reader)
    }

    pub fn matches(&self, path: &Path) -> bool {
        self.patterns.is_match(&path.to_string_lossy())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_file_ignore() {
        // Unanchored: should match anywhere
        let gitignore_content = b"abc.txt";
        let ignore = GitIgnore::from(&gitignore_content[..]).unwrap();

        assert!(ignore.matches(&PathBuf::from("abc.txt")));
        assert!(ignore.matches(&PathBuf::from("src/abc.txt")));
        assert!(ignore.matches(&PathBuf::from("debug/logs/abc.txt")));
        assert!(!ignore.matches(&PathBuf::from("xyz.txt")));
    }

    #[test]
    fn test_anchored_ignore() {
        // Anchored with leading slash: should only match root
        let gitignore_content = b"/root_only.txt";
        let ignore = GitIgnore::from(&gitignore_content[..]).unwrap();

        assert!(ignore.matches(&PathBuf::from("root_only.txt")));
        assert!(!ignore.matches(&PathBuf::from("subdir/root_only.txt")));
    }

    #[test]
    fn test_extension_wildcard() {
        let gitignore_content = b"*.log";
        let ignore = GitIgnore::from(&gitignore_content[..]).unwrap();

        assert!(ignore.matches(&PathBuf::from("error.log")));
        assert!(ignore.matches(&PathBuf::from("build/output.log")));
        assert!(!ignore.matches(&PathBuf::from("log.txt")));
    }

    #[test]
    fn test_directory_ignore() {
        // Trailing slash: matches everything inside the folder
        let gitignore_content = b"target/";
        let ignore = GitIgnore::from(&gitignore_content[..]).unwrap();

        assert!(ignore.matches(&PathBuf::from("target/debug/app")));
        assert!(ignore.matches(&PathBuf::from("src/target/old_build")));
        assert!(ignore.matches(&PathBuf::from("target/")));
    }

    #[test]
    fn test_regex_escaping() {
        // Test that special regex characters in filenames are escaped
        let gitignore_content = b"data()[1].{txt}";
        let ignore = GitIgnore::from(&gitignore_content[..]).unwrap();

        assert!(ignore.matches(&PathBuf::from("data()[1].{txt}")));
    }

    #[test]
    fn test_ignore() {
        let gitignore_content = b"abc.txt";
        let ignore = GitIgnore::from(&gitignore_content[..]).unwrap();

        assert!(ignore.matches(&PathBuf::from("abc.txt")));
        assert!(!ignore.matches(&PathBuf::from("xyz.txt")));
    }
}
