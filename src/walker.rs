use std::collections::HashSet;
use std::io::Read;
use std::path::PathBuf;

use crate::ignore;

use anyhow::Result;

pub struct Walker {
    gitignore: ignore::GitIgnore,

    seen_paths: HashSet<PathBuf>,
    file_paths: Vec<PathBuf>,
    probe_buffer: [u8; 1024],
}

// TODO: Support natural discovery of .gitignore files
// Also, we should probably move this into crate::ignore

impl Walker {
    pub fn new(gitignore: ignore::GitIgnore) -> Self {
        let seen_paths = HashSet::new();
        let file_paths = Vec::new();
        let probe_buffer = [0u8; 1024];

        Self {
            gitignore,
            seen_paths,
            file_paths,
            probe_buffer,
        }
    }

    // TODO: Return an iterator so that we can start processing immediately...
    pub fn collect_file_paths(
        mut self,
        paths: Vec<PathBuf>,
        max_depth: u32,
    ) -> Result<Vec<PathBuf>> {
        let max_depth = max_depth.saturating_add(1);

        for path in paths {
            self.search_path(path, max_depth)?;
        }

        Ok(self.file_paths)
    }

    fn is_text_file(&mut self, path: &PathBuf) -> bool {
        let Ok(mut file) = std::fs::File::open(path) else {
            return false;
        };
        let Ok(n) = file.read(&mut self.probe_buffer) else {
            return false;
        };

        let sample = &self.probe_buffer[..n];

        !sample.contains(&0) && std::str::from_utf8(sample).is_ok()
    }

    fn search_path(&mut self, path: PathBuf, remaining_depth: u32) -> Result<()> {
        if !self.seen_paths.insert(path.clone()) {
            // insert returns False if it was already in the Set
            return Ok(());
        }

        // Don't follow symlinks
        // Get the metadata once (it requires a syscall)
        let metadata = path.symlink_metadata()?;

        if metadata.is_symlink() {
            return Ok(());
        }

        if metadata.is_file() {
            if self.gitignore.is_match(&path, false) {
                return Ok(());
            }
            if self.is_text_file(&path) {
                self.file_paths.push(path);
            }
            return Ok(());
        }

        if !metadata.is_dir() {
            return Ok(());
        }

        if self.gitignore.is_match(&path, true) {
            return Ok(());
        }

        if remaining_depth == 0 {
            return Ok(());
        }

        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let child = entry.path();
            self.search_path(child, remaining_depth - 1)?;
        }

        Ok(())
    }
}
