use anyhow::{Result};

use std::collections::HashSet;
use std::io::{Read};
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
