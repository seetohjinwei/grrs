use std::io::BufRead;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use log::warn;
use regex::RegexSet;

// Check why a file is ignored.
// git check-ignore -v <FILE> [FILE...]

const DIR_SEP: char = '/';

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
            '*' => regex.push_str(r".*"),
            '?' => regex.push_str(r"."),
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
/// Invalid patterns will return None.
///
/// Reference link: https://git-scm.com/docs/gitignore
fn convert_pattern(pattern: &str) -> Option<String> {
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

        if is_leading {
            if (has_multiple_separators || has_non_ending_separator) && !is_double_asterisk {
                // 1. If there is a separator at the beginning or middle (or both) of the pattern, then the pattern is relative to the directory level of the particular .gitignore file itself. Otherwise the pattern may also match at any level below the .gitignore level.
                // e.g. `dir/a.txt` matches `dir/a.txt` but not `dir2/dir/a.txt`.
                // e.g. `dir/`      matches `dir/`      and     `dir2/dir/`.
                // 2. leading `**/` overrides rule 1
                regex.push('^');
                if part.is_empty() {
                    // Avoid adding a `/`
                    continue;
                }
            } else {
                regex.push_str(r"(?:^|/)");
            }
        }

        if is_double_asterisk {
            if is_trailing {
                // 3. trailing `/**` matches everything inside some directory
                // e.g. `abc/**` matches all files inside `abc` recursively
                regex.push_str(r".*");
            } else {
                // 4. a slash followed by two consecutive asterisks then a slash matches zero or more directories
                // e.g. `a/**/b` matches `a/b`, `a/x/b` and `a/x/y/b`
                regex.push_str(r"(?:.*/)?");
            }

            // Don't parse the pattern
            continue;
        }

        let Some(part_regex) = convert_part(part) else {
            // An invalid part => the entire pattern is invalid
            // => return an empty string so that it matches nothing
            return None;
        };
        regex.push_str(&part_regex);
        if !is_trailing {
            regex.push('/');
        }
    }

    // The following rule is naturally handled.
    // > If there is a separator at the end of the pattern then the pattern will only match directories, otherwise the pattern can match both files and directories.

    if !regex.ends_with(".*") && !regex.ends_with('/') {
        regex.push_str(r"(?:/|$)");
    }

    Some(regex)
}

struct GitIgnore {
    root_path: PathBuf,
    include_patterns: RegexSet,
    exclude_patterns: RegexSet,
}

impl GitIgnore {
    pub fn empty() -> Self {
        Self {
            root_path: PathBuf::new(),
            include_patterns: RegexSet::empty(),
            exclude_patterns: RegexSet::empty(),
        }
    }

    pub fn from<R: BufRead>(ignore_path: PathBuf, reader: R) -> Result<Self> {
        let mut include_patterns = Vec::new();
        let mut exclude_patterns = Vec::new();

        for pattern in reader.lines() {
            let pattern = pattern?;
            let pattern = clean_pattern(pattern);

            // A blank line matches no files
            if pattern.is_empty() {
                continue;
            }

            if pattern.starts_with('!') {
                let Some(pattern) = convert_pattern(&pattern[1..]) else {
                    continue;
                };
                exclude_patterns.push(pattern);
            } else {
                let Some(pattern) = convert_pattern(&pattern) else {
                    continue;
                };
                include_patterns.push(pattern);
            }
        }

        Ok(Self {
            root_path: ignore_path,
            include_patterns: RegexSet::new(include_patterns)?,
            exclude_patterns: RegexSet::new(exclude_patterns)?,
        })
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

        Self::from(
            ignore_path
                .parent()
                .and_then(|p| Option::Some(p.to_path_buf()))
                .unwrap_or(PathBuf::new()),
            reader,
        )
    }

    pub fn from_dir(dir_path: &PathBuf) -> Result<Option<Self>> {
        let gitignore_path = dir_path.join(".gitignore");

        // Fetch the metadata once because it requires a syscall
        let metadata = gitignore_path.metadata()?;

        if metadata.is_file() {
            let gitignore = GitIgnore::new(&gitignore_path)?;

            Ok(Some(gitignore))
        } else {
            Ok(None)
        }
    }

    pub fn is_match(&self, path: &Path, is_dir: bool) -> bool {
        let path = path.strip_prefix("./").unwrap_or(path);
        let path = path.strip_prefix(&self.root_path).unwrap_or(path);

        let mut path = path.to_string_lossy();
        if is_dir {
            if path == ".git" {
                // We should always ignore .git directory!
                return true;
            }
            path.to_mut().push('/');
        }

        if self.exclude_patterns.is_match(&path) {
            return false;
        }

        self.include_patterns.is_match(&path)
    }
}

struct GitIgnoreStack {
    stack: Vec<GitIgnore>,
}

impl GitIgnoreStack {
    fn new() -> Self {
        Self { stack: Vec::new() }
    }

    fn push(&mut self, gitignore: GitIgnore) -> () {
        self.stack.push(gitignore)
    }

    fn pop(&mut self) -> Option<GitIgnore> {
        self.stack.pop()
    }

    fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    fn is_match(&self, path: &Path, is_dir: bool) -> bool {
        for gitignore in self.stack.iter().rev() {
            if gitignore.is_match(path, is_dir) {
                return true;
            }
        }

        false
    }
}

/// Checks if `path` is a valid text file.
/// Uses a re-usable `probe_buffer`.
fn is_text_file(probe_buffer: &mut [u8; 1024], path: &PathBuf) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let Ok(n) = file.read(probe_buffer) else {
        return false;
    };

    let sample = &probe_buffer[..n];

    !sample.contains(&0) && std::str::from_utf8(sample).is_ok()
}

struct Walker {
    max_depth: u32,

    file_paths: Vec<PathBuf>,
    probe_buffer: [u8; 1024],
    gitignore_stack: GitIgnoreStack,
}


/// Walks the path using DFS.
fn walk_dfs(walker: &mut Walker, path: PathBuf, current_depth: u32) -> Result<()> {
    if current_depth >= walker.max_depth {
        return Ok(());
    }

    // Fetch the metadata once because it requires a syscall
    let metadata = path.symlink_metadata()?;

    if metadata.is_symlink() {
        // Don't follow symlinks to guarantee that it is a tree
        return Ok(());
    } else if metadata.is_file() {
        if walker.gitignore_stack.is_match(&path, false) {
            return Ok(());
        }

        if !is_text_file(&mut walker.probe_buffer, &path) {
            return Ok(());
        }
        walker.file_paths.push(path);
    } else if metadata.is_dir() {
        // gitignore cannot ignore its own directory, (it can only do stuff like `*` to ignore children)
        // so it is safe to do this before checking if it exists in current directory.
        if walker.gitignore_stack.is_match(&path, true) {
            return Ok(());
        }

        // If gitignore exists in this directory, add it to the stack
        let gitignore = GitIgnore::from_dir(&path).unwrap_or(None);
        let has_gitignore = gitignore.is_some();
        if let Some(gitignore) = gitignore {
            walker.gitignore_stack.push(gitignore);
        }

        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let child = entry.path();
            walk_dfs(walker, child, current_depth + 1)?;
        }

        // Clean it up from the stack
        if has_gitignore {
            let _ = walker.gitignore_stack.pop();
        }
    } else {
        assert!(false, "path {:?} is not any of symlink, file, dir...", path);
    }

    Ok(())
}

/// Walks the file tree rooted at `initial_path` (up to `max_depth`), collecting all files into the result.
pub fn walk(initial_path: PathBuf, max_depth: u32) -> Result<Vec<PathBuf>> {
    let mut walker = Walker {
        max_depth,
        file_paths: Vec::new(),
        probe_buffer: [0u8; 1024],
        gitignore_stack: GitIgnoreStack::new(),
    };

    walk_dfs(&mut walker, initial_path, 0)?;

    assert!(walker.gitignore_stack.is_empty(), "walk_dfs should have cleaned up all gitignores");

    Ok(walker.file_paths)
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
        assert_eq!(
            convert_pattern(&String::from("")),
            Some(String::from(r"(?:^|/)(?:/|$)"))
        );
        // Basic file
        assert_eq!(
            convert_pattern(&String::from("abc.txt")),
            Some(String::from(r"(?:^|/)abc\.txt(?:/|$)"))
        );
        // Handle beginning separator
        assert_eq!(
            convert_pattern(&String::from("/abc")),
            Some(String::from(r"^abc(?:/|$)"))
        );
        // Handle middle separator
        assert_eq!(
            convert_pattern(&String::from("dir/a.txt")),
            Some(String::from(r"^dir/a\.txt(?:/|$)"))
        );
        // Handle ending separator
        assert_eq!(
            convert_pattern(&String::from("abc/")),
            Some(String::from(r"(?:^|/)abc/"))
        );
        // Handle trailing double asterisks
        assert_eq!(
            convert_pattern(&String::from("dir/**")),
            Some(String::from(r"^dir/.*"))
        );
        // Handle middle double asterisks
        assert_eq!(
            convert_pattern(&String::from("a/**/b")),
            Some(String::from(r"^a/(?:.*/)?b(?:/|$)"))
        );

        // Handle escaped characters
        assert_eq!(
            convert_pattern(&String::from(r"data()\[1\].{txt}")),
            Some(String::from(r"(?:^|/)data\(\)\[1\]\.\{txt\}(?:/|$)"))
        );

        // Handle invalid pattern
        // Backslash at the end of a pattern is invalid
        assert_eq!(convert_pattern(&String::from(r"abc\")), None);
        assert_eq!(convert_pattern(&String::from(r"dir/abc\")), None);
    }

    #[test]
    fn test_basic_file_ignore() {
        // Unanchored: should match anywhere
        let gitignore_content = b"abc.txt";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.is_match(&PathBuf::from("abc.txt"), false));
        assert!(ignore.is_match(&PathBuf::from("src/abc.txt"), false));
        assert!(ignore.is_match(&PathBuf::from("debug/logs/abc.txt"), false));
        assert!(!ignore.is_match(&PathBuf::from("xyz.txt"), false));
    }

    #[test]
    fn test_partial_match() {
        // Unanchored: should match anywhere
        let gitignore_content = b"def";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.is_match(&PathBuf::from("def"), false));
        assert!(!ignore.is_match(&PathBuf::from("abcdef"), false));
        assert!(!ignore.is_match(&PathBuf::from("defghi"), false));
    }

    #[test]
    fn test_double_asterisk() {
        // Unanchored: should match anywhere
        let gitignore_content = b"a/**/b";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.is_match(&PathBuf::from("a/b"), false));
        assert!(ignore.is_match(&PathBuf::from("a/x/b"), false));
        assert!(ignore.is_match(&PathBuf::from("a/x/y/b"), false));
    }

    #[test]
    fn test_anchored_ignore() {
        // Anchored with leading slash: should only match root
        let gitignore_content = b"/root_only.txt";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.is_match(&PathBuf::from("root_only.txt"), false));
        assert!(!ignore.is_match(&PathBuf::from("subdir/root_only.txt"), false));
    }

    #[test]
    fn test_extension_wildcard() {
        let gitignore_content = b"*.log";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.is_match(&PathBuf::from("error.log"), false));
        assert!(ignore.is_match(&PathBuf::from("build/output.log"), false));
        assert!(!ignore.is_match(&PathBuf::from("log.txt"), false));
    }

    #[test]
    fn test_directory_ignore() {
        // Trailing slash: matches everything inside the folder
        let gitignore_content = b"target/";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.is_match(&PathBuf::from("target/debug/app"), false));
        assert!(ignore.is_match(&PathBuf::from("target/debug/app"), true));
        assert!(ignore.is_match(&PathBuf::from("src/target/old_build"), false));
        assert!(ignore.is_match(&PathBuf::from("src/target/old_build"), true));
        assert!(!ignore.is_match(&PathBuf::from("target"), false)); // should not match file
        assert!(ignore.is_match(&PathBuf::from("target/"), true)); // should match directory
    }

    #[test]
    fn test_regex_escaping() {
        // Test that special regex characters in filenames are escaped
        let gitignore_content = br"data()\[1\].{txt}";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.is_match(&PathBuf::from("data()[1].{txt}"), false));
    }

    #[test]
    fn test_backslash() {
        // Test backslashes in file names
        let gitignore_content = br"file\\name";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.is_match(&PathBuf::from(r"file\name"), false));
    }

    #[test]
    fn test_backslash_with_question_mark() {
        // Test backslashes in file names when pattern has asterisk
        let gitignore_content = br"file?name";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.is_match(&PathBuf::from(r"file\name"), false));
    }

    #[test]
    fn test_backslash_with_asterisk() {
        // Test backslashes in file names when pattern has asterisk
        let gitignore_content = br"file*name";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.is_match(&PathBuf::from(r"file\name"), false));
    }

    #[test]
    fn test_invalid_backslash() {
        // Test backslashes at end of pattern should not match anything
        let gitignore_content = br"file\";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(!ignore.is_match(&PathBuf::from(r""), false));
        assert!(!ignore.is_match(&PathBuf::from(r"file"), false));
    }

    #[test]
    fn test_ranges() {
        // Test backslashes in file names when pattern has asterisk
        let gitignore_content = br"file-[a-z]";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.is_match(&PathBuf::from(r"file-a"), false));
        assert!(ignore.is_match(&PathBuf::from(r"file-z"), false));
        assert!(!ignore.is_match(&PathBuf::from(r"file-3"), false));
        assert!(!ignore.is_match(&PathBuf::from(r"file-B"), false));
        assert!(!ignore.is_match(&PathBuf::from(r"file-[a-z]"), false));
    }

    #[test]
    fn test_excluded_matches() {
        // Test exclude patterns
        let gitignore_content = b"*\n!file*.txt";
        let ignore = GitIgnore::from(PathBuf::new(), &gitignore_content[..]).unwrap();

        assert!(ignore.is_match(&PathBuf::from(r"abc.txt"), false));
        assert!(!ignore.is_match(&PathBuf::from(r"file.txt"), false));
        assert!(!ignore.is_match(&PathBuf::from(r"file2.txt"), false));
    }
}
