use anyhow::{Context, Result};
use regex::RegexBuilder;

use std::collections::HashSet;
use std::io::{BufRead, Read};
use std::path::PathBuf;

fn is_text_file(path: &PathBuf, buffer: &mut [u8]) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let Ok(n) = file.read(buffer) else {
        return false;
    };

    let sample = &buffer[..n];

    !sample.contains(&0) && std::str::from_utf8(sample).is_ok()
}

fn get_file_paths_helper(
    seen_paths: &mut HashSet<PathBuf>,
    file_paths: &mut Vec<PathBuf>,
    probe_buffer: &mut [u8],
    path: PathBuf,
    remaining_depth: u32,
) -> Result<()> {
    if seen_paths.contains(&path) {
        return Ok(());
    }
    seen_paths.insert(path.clone());

    if path.is_file() {
        if is_text_file(&path, probe_buffer) {
            file_paths.push(path);
        }
        return Ok(());
    }

    if !path.is_dir() {
        return Ok(());
    }

    if remaining_depth == 0 {
        return Ok(());
    }

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        get_file_paths_helper(
            seen_paths,
            file_paths,
            probe_buffer,
            child,
            remaining_depth - 1,
        )?;
    }

    Ok(())
}

pub fn get_file_paths(paths: Vec<PathBuf>, max_depth: u32) -> Result<Vec<PathBuf>> {
    let mut probe_buffer = [0u8; 1024];
    let mut seen_paths = HashSet::new();
    let mut file_paths = Vec::new();

    for path in paths {
        get_file_paths_helper(
            &mut seen_paths,
            &mut file_paths,
            &mut probe_buffer,
            path,
            max_depth + 1,
        )?;
    }

    Ok(file_paths)
}

pub struct LazyWriter<W: std::io::Write> {
    writer: W,
    header: String,
    has_printed_header: bool,
}

impl<W: std::io::Write> LazyWriter<W> {
    pub fn new(writer: W, header: String) -> Self {
        Self {
            writer: writer,
            header: header,
            has_printed_header: false,
        }
    }
}

impl<W: std::io::Write> std::io::Write for LazyWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if !self.has_printed_header {
            writeln!(self.writer, "{}", self.header)?;
            self.has_printed_header = true;
        }

        self.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

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
