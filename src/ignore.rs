use anyhow::{Context, Result};
use log::warn;
use regex::RegexSet;
use std::io::BufRead;
use std::path::{Path, PathBuf};

// Check why a file is ignored.
// git check-ignore -v <FILE> [FILE...]

const DIR_SEP: char = '/';

pub struct GitIgnore {
    root_path: PathBuf,
    patterns: RegexSet,
}

// TODO: Support negative matches `!abc.txt` by maintaining a `include_patterns`
// current `patterns` should be `exclude_patterns`

/// Removes comment from a pattern.
fn remove_comment(mut pattern: String) -> String {
    // Finds a comment from a pattern.
    let Some(comment_index) = crate::escaped_strings::find_char(&pattern, '#') else {
        return pattern;
    };

    pattern.truncate(comment_index);
    pattern
}

/// Cleans a pattern by removing comments and trailing spaces.
fn clean_pattern(pattern: String) -> String {
    let pattern = remove_comment(pattern);

    // Leading spaces should not be ignored
    crate::escaped_strings::trim_end(pattern)
}

/// Converts pattern from gitignore syntax to regex syntax.
///
/// Reference link: https://git-scm.com/docs/gitignore
fn convert_pattern(pattern: &str) -> String {
    const ASTERISK: char = '*';
    const DOUBLE_ASTERISK: &str = "**";

    let parts: Vec<&str> = pattern.split(DIR_SEP).collect();

    let mut regex = String::from("");

    // Has separator at the beginning or middle (or both)
    // => has a non-ending separator
    // => has multiple separators or the only separator is not an ending separator
    let has_multiple_separators = parts.len() >= 3;
    let has_non_ending_separator = parts.len() >= 2 && !parts[parts.len() - 1].is_empty();

    for (i, part) in parts.iter().enumerate() {
        let is_leading = i == 0;
        let is_trailing = i == parts.len() - 1;

        println!(
            "{} {}: {} {} {} {}",
            i, part,
            is_leading,
            has_multiple_separators,
            has_non_ending_separator,
            *part != DOUBLE_ASTERISK
        );

        if is_leading
            && (has_multiple_separators || has_non_ending_separator)
            && *part != DOUBLE_ASTERISK
        {
            // 1. If there is a separator at the beginning or middle (or both) of the pattern, then the pattern is relative to the directory level of the particular .gitignore file itself. Otherwise the pattern may also match at any level below the .gitignore level.
            // e.g. `dir/a.txt` matches `dir/a.txt` but not `dir2/dir/a.txt`.
            // e.g. `dir/`      matches `dir/`      and     `dir2/dir/`.
            // 2. leading `**/` overrides rule 1
            regex.push('^');
        }

        if is_leading && *part == DOUBLE_ASTERISK {}
    }

    // 3. trailing `/**` matches everything inside some directory
    // e.g. `abc/**` matches all files inside `abc` recursively
    // 4. a slash followed by two consecutive asterisks then a slash matches zero or more directories
    // e.g. `a/**/b` matches `a/b`, `a/x/b` and `a/x/y/b`
    // 5. other consecutive asterisks are considered regular asterisks!

    // 1. `*` matches anything except a slash (e.g. r"[^\\]*")
    // 2. `?` matches any one character except a slash (e.g. r"[^\\]")
    // 3. backslash `\` escapes special characters
    // 4. range notation `[a-zA-Z]`
    // 5. a backslash at the end of a pattern is an invalid pattern that never matches!

    // TODO: Implement this function!

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

        for pattern in reader.lines() {
            let pattern = pattern?;
            let pattern = clean_pattern(pattern);

            // A blank line matches no files
            if pattern.is_empty() {
                continue;
            }

            patterns.push(convert_pattern(&pattern));
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
    fn test_remove_comment() {
        // Empty pattern has no comment
        assert_eq!(remove_comment(String::from("")), "");
        // Empty comment at start of line
        assert_eq!(remove_comment(String::from("#")), "");
        // Comment at start of line
        assert_eq!(remove_comment(String::from("# ABC")), "");
        // Empty comment after some pattern
        assert_eq!(
            remove_comment(String::from("/build/  # Build files!")),
            "/build/  "
        );
        // Comment after some pattern
        assert_eq!(remove_comment(String::from("/build/  #")), "/build/  ");
        // Multiple hashtags
        assert_eq!(
            remove_comment(String::from("/build/  # COMMENT! #")),
            "/build/  "
        );
        // Escaped hashtags without a comment
        assert_eq!(
            remove_comment(String::from(r"/\#hashtag\#/")),
            r"/\#hashtag\#/"
        );
        // Escaped hashtags with comment
        assert_eq!(
            remove_comment(String::from(r"/\#hashtag\#/  # COMMENT! #")),
            r"/\#hashtag\#/  "
        );
    }

    #[test]
    fn test_convert_pattern() {
        // Empty pattern
        assert_eq!(convert_pattern(&String::from("")), r"");
        // Basic file
        assert_eq!(convert_pattern(&String::from("abc.txt")), r"");
        // Handle beginning separator
        assert_eq!(convert_pattern(&String::from("/abc")), r"^");
        // Handle middle separator
        assert_eq!(convert_pattern(&String::from("dir/a.txt")), r"^");
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn test_basic_file_ignore() {
//         // Unanchored: should match anywhere
//         let gitignore_content = b"abc.txt";
//         let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();
//
//         assert!(ignore.matches(&PathBuf::from("abc.txt")));
//         assert!(ignore.matches(&PathBuf::from("src/abc.txt")));
//         assert!(ignore.matches(&PathBuf::from("debug/logs/abc.txt")));
//         assert!(!ignore.matches(&PathBuf::from("xyz.txt")));
//     }
//
//     #[test]
//     fn test_anchored_ignore() {
//         // Anchored with leading slash: should only match root
//         let gitignore_content = b"/root_only.txt";
//         let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();
//
//         assert!(ignore.matches(&PathBuf::from("root_only.txt")));
//         assert!(!ignore.matches(&PathBuf::from("subdir/root_only.txt")));
//     }
//
//     #[test]
//     fn test_extension_wildcard() {
//         let gitignore_content = b"*.log";
//         let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();
//
//         assert!(ignore.matches(&PathBuf::from("error.log")));
//         assert!(ignore.matches(&PathBuf::from("build/output.log")));
//         assert!(!ignore.matches(&PathBuf::from("log.txt")));
//     }
//
//     #[test]
//     fn test_directory_ignore() {
//         // Trailing slash: matches everything inside the folder
//         let gitignore_content = b"target/";
//         let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();
//
//         assert!(ignore.matches(&PathBuf::from("target/debug/app")));
//         assert!(ignore.matches(&PathBuf::from("src/target/old_build")));
//         assert!(ignore.matches(&PathBuf::from("target/")));
//     }
//
//     #[test]
//     fn test_regex_escaping() {
//         // Test that special regex characters in filenames are escaped
//         let gitignore_content = b"data()[1].{txt}";
//         let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();
//
//         assert!(ignore.matches(&PathBuf::from("data()[1].{txt}")));
//     }
//
//     #[test]
//     fn test_ignore() {
//         let gitignore_content = b"abc.txt";
//         let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();
//
//         assert!(ignore.matches(&PathBuf::from("abc.txt")));
//         assert!(!ignore.matches(&PathBuf::from("xyz.txt")));
//     }
// }

// TODO: IGNORE ISN'T MATCHING PROPERLY!!
// I think it's because of relative paths and stuff...
// Gotta figure out how exactly Rust is doing this!!
