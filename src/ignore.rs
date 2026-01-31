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

/// Converts a part of a pattern.
/// If a part is invalid, None is returned.
fn convert_part(part: &str) -> Option<String> {
    let mut regex = String::new();

    let mut is_escaped = false;

    for c in part.chars() {
        // 1. `*` and `**` matches anything except a slash (e.g. r"[^\\]*")
        //    We don't actually have to handle double asterisk specially tho
        // 2. `?` matches any one character except a slash (e.g. r"[^\\]")
        // 3. backslash `\` escapes special characters
        // 4. range notation `[a-zA-Z]`

        if is_escaped {
            regex.push(c);
            is_escaped = false;
            continue;
        }

        match c {
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' => {
                regex.push('\\');
                regex.push(c);
            }
            '*' => regex.push_str(r"[^\\]*"),
            '?' => regex.push_str(r"[^\\]"),
            '\\' => {
                regex.push(c);
                is_escaped = !is_escaped;
            }
            _ => regex.push(c),
        }
    }

    // > a backslash at the end of a pattern is an invalid pattern that never matches!
    if is_escaped { None } else { Some(regex) }
}

/// Converts pattern from gitignore syntax to regex syntax.
/// Invalid patterns will return an empty string which will match nothing.
///
/// Reference link: https://git-scm.com/docs/gitignore
fn convert_pattern(pattern: &str) -> String {
    let parts: Vec<String> = crate::escaped_strings::split(pattern, DIR_SEP).collect();

    let mut regex = String::new();

    // Has separator at the beginning or middle (or both)
    // => has a non-ending separator
    // => has multiple separators or the only separator is not an ending separator
    let has_multiple_separators = parts.len() >= 3;
    let has_non_ending_separator = parts.len() >= 2 && !parts[parts.len() - 1].is_empty();

    for (i, part) in parts.iter().enumerate() {
        let is_leading = i == 0;
        let is_trailing = i == parts.len() - 1;
        let is_double_asterisk = *part == "**";

        // NOTE: We're using a match statement to prove that these conditions are mutually exclusive :)
        match (
            is_leading,
            is_trailing,
            (has_multiple_separators || has_non_ending_separator),
            is_double_asterisk,
        ) {
            (true, _, true, false) => {
                // 1. If there is a separator at the beginning or middle (or both) of the pattern, then the pattern is relative to the directory level of the particular .gitignore file itself. Otherwise the pattern may also match at any level below the .gitignore level.
                // e.g. `dir/a.txt` matches `dir/a.txt` but not `dir2/dir/a.txt`.
                // e.g. `dir/`      matches `dir/`      and     `dir2/dir/`.
                // 2. leading `**/` overrides rule 1
                regex.push('^');
                if part.is_empty() {
                    // Avoid adding a `/`
                    continue;
                }
                // Don't continue because we still need to convert this part.
            }
            (_, true, _, true) => {
                // 3. trailing `/**` matches everything inside some directory
                // e.g. `abc/**` matches all files inside `abc` recursively
                regex.push_str(r".*");
                continue;
            }
            (_, _, _, true) => {
                // 4. a slash followed by two consecutive asterisks then a slash matches zero or more directories
                // e.g. `a/**/b` matches `a/b`, `a/x/b` and `a/x/y/b`
                regex.push_str(r".*");
                if !is_trailing {
                    regex.push('/');
                }
                continue;
            }
            // NOTE: A part *can* be leading + trailing if there's only one part!
            _ => {}
        };

        let Some(part_regex) = convert_part(part) else {
            // An invalid part => the entire pattern is invalid
            // => return an empty string so that it matches nothing
            return String::new();
        };
        regex.push_str(&part_regex);
        if !is_trailing {
            regex.push('/');
        }
    }

    // The following rule is naturally handled.
    // > If there is a separator at the end of the pattern then the pattern will only match directories, otherwise the pattern can match both files and directories.

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

            if pattern.starts_with('!') {
                // TODO: Support negative matches `!abc.txt` by maintaining a `include_patterns`
                // current `patterns` should be `exclude_patterns`
                todo!("negative patterns are not supported");
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
        // TODO: Rewrite this
        // It should take in a string instead of a Path object

        // TODO: If path is a directory, it MUST end with a trailing slash!

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
        assert_eq!(convert_pattern(&String::from("abc.txt")), r"abc\.txt");
        // Handle beginning separator
        assert_eq!(convert_pattern(&String::from("/abc")), r"^abc");
        // Handle middle separator
        assert_eq!(convert_pattern(&String::from("dir/a.txt")), r"^dir/a\.txt");
        // Handle ending separator
        assert_eq!(convert_pattern(&String::from("abc/")), r"abc/");
        // Handle trailing double asterisks
        assert_eq!(convert_pattern(&String::from("dir/**")), r"^dir/.*");
        // Handle trailing middle asterisks
        assert_eq!(convert_pattern(&String::from("a/**/b")), r"^a/.*/b");

        // Handle escaped characters
        assert_eq!(
            convert_pattern(&String::from(r"data()\[1\].{txt}")),
            r"data\(\)\[1\]\.\{txt\}"
        );

        // Handle invalid pattern
        // Backslash at the end of a pattern is invalid
        assert_eq!(convert_pattern(&String::from(r"abc\")), "");
        assert_eq!(convert_pattern(&String::from(r"dir/abc\")), "");
    }

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
        assert!(!ignore.matches(&PathBuf::from("target"))); // should not match file
    }

    #[test]
    fn test_regex_escaping() {
        // Test that special regex characters in filenames are escaped
        let gitignore_content = br"data()\[1\].{txt}";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.matches(&PathBuf::from("data()[1].{txt}")));
    }

    #[test]
    fn test_basic_ignore() {
        let gitignore_content = b"abc.txt";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.matches(&PathBuf::from("abc.txt")));
        assert!(!ignore.matches(&PathBuf::from("xyz.txt")));
    }
}
