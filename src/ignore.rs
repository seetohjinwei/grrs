use anyhow::{Context, Result};
use log::warn;
use regex::RegexSet;
use std::io::BufRead;
use std::path::{Path, PathBuf};

pub struct GitIgnore {
    root_path: PathBuf,
    patterns: RegexSet,
}

// TODO: Support negative matches `!abc.txt` by maintaining a `include_patterns`
// current `patterns` should be `exclude_patterns`

fn glob_to_regex(glob: &str) -> String {
    let mut regex = String::from("");
    let mut chars = glob.chars();

    if glob.starts_with('/') {
        chars.next();
        regex.push_str("^");
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

    regex.push_str(".*");

    regex
}

impl GitIgnore {
    pub fn empty() -> Self {
        let set = RegexSet::empty();
        Self {
            root_path: PathBuf::new(),
            patterns: set,
        }
    }

    pub fn from<R: BufRead>(ignore_path: PathBuf, reader: R) -> Result<Self> {
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
        Ok(Self {
            root_path: ignore_path,
            patterns: set,
        })
    }

    pub fn new(ignore_path: PathBuf) -> Result<Self> {
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

        Self::from(
            ignore_path
                .parent()
                .and_then(|p| Option::Some(p.to_path_buf()))
                .unwrap_or(PathBuf::new()),
            reader,
        )
    }

    pub fn matches(&self, path: &Path) -> bool {
        if self.root_path.as_os_str().is_empty() {
            return self.patterns.is_match(&path.to_string_lossy());
        }

        println!("strip prefix {:?} {:?}", path, self.root_path);

        let Ok(path) = path.strip_prefix("./") else {
            return false;
        };
        let Ok(path) = path.strip_prefix(&self.root_path) else {
            return false;
        };

        println!(" -> {:?}", path);

        self.patterns.is_match(&path.to_string_lossy())
    }

    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_file_ignore() {
        // Unanchored: should match anywhere
        let gitignore_content = b"abc.txt";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.matches(&PathBuf::from("abc.txt")));
        assert!(ignore.matches(&PathBuf::from("src/abc.txt")));
        assert!(ignore.matches(&PathBuf::from("debug/logs/abc.txt")));
        assert!(!ignore.matches(&PathBuf::from("xyz.txt")));
    }

    #[test]
    fn test_anchored_ignore() {
        // Anchored with leading slash: should only match root
        let gitignore_content = b"/root_only.txt";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.matches(&PathBuf::from("root_only.txt")));
        assert!(!ignore.matches(&PathBuf::from("subdir/root_only.txt")));
    }

    #[test]
    fn test_extension_wildcard() {
        let gitignore_content = b"*.log";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.matches(&PathBuf::from("error.log")));
        assert!(ignore.matches(&PathBuf::from("build/output.log")));
        assert!(!ignore.matches(&PathBuf::from("log.txt")));
    }

    #[test]
    fn test_directory_ignore() {
        // Trailing slash: matches everything inside the folder
        let gitignore_content = b"target/";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.matches(&PathBuf::from("target/debug/app")));
        assert!(ignore.matches(&PathBuf::from("src/target/old_build")));
        assert!(ignore.matches(&PathBuf::from("target/")));
    }

    #[test]
    fn test_regex_escaping() {
        // Test that special regex characters in filenames are escaped
        let gitignore_content = b"data()[1].{txt}";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.matches(&PathBuf::from("data()[1].{txt}")));
    }

    #[test]
    fn test_ignore() {
        let gitignore_content = b"abc.txt";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.matches(&PathBuf::from("abc.txt")));
        assert!(!ignore.matches(&PathBuf::from("xyz.txt")));
    }
}

// TODO: IGNORE ISN'T MATCHING PROPERLY!!
// I think it's because of relative paths and stuff...
// Gotta figure out how exactly Rust is doing this!!
