use anyhow::{Context, Result};
use regex::RegexBuilder;

use std::io::BufRead;

pub struct MatchOptions {
    pub show_line_numbers: bool,
    pub case_insensitive: bool,
}

impl Default for MatchOptions {
    fn default() -> Self {
        Self {
            show_line_numbers: false,
            case_insensitive: false,
        }
    }
}

pub fn find_matches<R: BufRead, W: std::io::Write>(
    reader: R,
    mut writer: W,
    pattern: &String,
    options: &MatchOptions,
) -> Result<()> {
    let pattern_regex = RegexBuilder::new(pattern)
        .case_insensitive(options.case_insensitive)
        .build()
        .context("invalid search pattern")?;

    for (line_num, line) in reader.lines().enumerate() {
        let message = line.with_context(|| format!("could not read line"))?;
        if pattern_regex.is_match(&message) {
            if options.show_line_numbers {
                writeln!(writer, "{}: {}", line_num + 1, message)?;
            } else {
                writeln!(writer, "{}", message)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_matches() {
        let input = b"lorem ipsum\ndolor sit amet\nquick brown fox";
        let mut result = Vec::new();

        find_matches(
            &input[..],
            &mut result,
            &"dolor".to_string(),
            &MatchOptions::default(),
        )
        .unwrap();

        assert_eq!(result, b"dolor sit amet\n");
    }
}
