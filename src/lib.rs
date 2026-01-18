use anyhow::{Context, Result};

use std::io::BufRead;

pub fn find_matches<R: BufRead, W: std::io::Write>(
    reader: R,
    mut writer: W,
    pattern: &String,
) -> Result<()> {
    for line in reader.lines() {
        let message = line.with_context(|| format!("could not read line"))?;
        if message.contains(pattern) {
            writeln!(writer, "{}", message)?;
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

        find_matches(&input[..], &mut result, &"dolor".to_string()).unwrap();

        assert_eq!(result, b"dolor sit amet\n");
    }
}
