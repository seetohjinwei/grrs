/// Utility functions for string operations that support escaping.

// We do not always support using escape char as a target.
const ESCAPE_CHAR: char = '\\';

/// Finds a character in a string if it is not escaped.
pub fn find_char(string: &str, target_char: char) -> Option<usize> {
    let mut escaped = false;
    for (idx, c) in string.char_indices() {
        if c == target_char && !escaped {
            return Some(idx);
        }

        if c == ESCAPE_CHAR && !escaped {
            escaped = true;
        } else {
            escaped = false;
        }
    }

    None
}

/// Returns a string slice with trailing whitespace removed.
pub fn trim_end(mut string: String) -> String {
    // We can treat the UTF-8 string as bytes because
    // we only compare against ' ' (SPACE) and ESCAPE_CHAR which are both single-byte characters.
    let bytes = string.as_bytes();

    let mut last_valid_idx = string.len();

    for i in (1..=bytes.len()).rev() {
        if bytes[i - 1] == b' ' {
            // Check for preceding backslashes, flipping `is_escaped` for each
            let mut is_escaped = false;
            let mut j = i - 1;
            while j > 0 && bytes[j - 1] == b'\\' {
                is_escaped = !is_escaped;
                j -= 1;
            }

            if is_escaped {
                // Non-space character
                break;
            }
            last_valid_idx = i - 1;
        } else {
            // Non-space character
            break;
        }
    }

    string.truncate(last_valid_idx);
    string
}

pub struct Split<'a> {
    chars: core::str::Chars<'a>,
    split_by: char,
    is_escaped: bool,
    done: bool,
}

impl<'a> Iterator for Split<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let mut substring = String::new();

        loop {
            let Some(c) = self.chars.next() else {
                self.done = true;
                break;
            };

            if self.is_escaped {
                substring.push(c);
                self.is_escaped = false;
                continue;
            }

            if c == ESCAPE_CHAR {
                substring.push(c);
                self.is_escaped = !self.is_escaped;
            } else if c == self.split_by {
                break;
            } else {
                substring.push(c);
            }
        }

        Some(substring)
    }
}

/// Returns an iterator of substrings of this string slice, separated by the specified character.
// We implemented it to return an iterator to learn about iterators :)
pub fn split(string: &str, split_by: char) -> Split<'_> {
    // We do not support this!
    assert_ne!(split_by, ESCAPE_CHAR);

    Split {
        chars: string.chars(),
        split_by: split_by,
        is_escaped: false,
        done: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_char() {
        // Empty pattern has no comment
        assert_eq!(find_char(&String::from(""), '#'), None);
        // Empty char at start of line
        assert_eq!(find_char(&String::from("#"), '#'), Some(0));
        // char at start of line
        assert_eq!(find_char(&String::from("# ABC"), '#'), Some(0));
        // Empty char after some pattern
        assert_eq!(
            find_char(&String::from("/build/  # Build files!"), '#'),
            Some(9)
        );
        // char after some pattern
        assert_eq!(find_char(&String::from("/build/  #"), '#'), Some(9));
        // Multiple hashtags
        assert_eq!(
            find_char(&String::from("/build/  # COMMENT! #"), '#'),
            Some(9)
        );
        // Escaped hashtags without a char
        assert_eq!(find_char(&String::from(r"/\#hashtag\#/"), '#'), None);
        // Escaped hashtags with char
        assert_eq!(
            find_char(&String::from(r"/\#hashtag\#/  # COMMENT! #"), '#'),
            Some(15)
        );

        // Handling double escape
        assert_eq!(find_char(&String::from(r"\\?"), '?'), Some(2));
        // Handling triple escape
        assert_eq!(find_char(&String::from(r"\\\?"), '?'), None);
        // Finding escape character (behavior is a bit undefined)
        assert_eq!(find_char(&String::from(r"\"), '\\'), Some(0));
        assert_eq!(find_char(&String::from(r"\\"), '\\'), Some(0));
        assert_eq!(find_char(&String::from(r"\\\\"), '\\'), Some(0));

        // Handling unicode characters
        assert_eq!(find_char(&String::from(r"🦀 CRAB"), 'C'), Some(5));
        assert_eq!(find_char(&String::from(r"게 CRAB"), 'C'), Some(4));
    }

    #[test]
    fn test_trim_end() {
        // Empty string
        assert_eq!(trim_end(String::from("")), "");
        // Only spaces
        assert_eq!(trim_end(String::from("   ")), "");
        // No trailing spaces
        assert_eq!(trim_end(String::from("abc")), "abc");
        // Trailing spaces
        assert_eq!(trim_end(String::from("abc  ")), "abc");
        // Non-trailing spaces
        assert_eq!(trim_end(String::from(" a b c")), " a b c");
        // Trailing escaped spaces
        assert_eq!(trim_end(String::from(r"abc\ ")), r"abc\ ");
        // Trailing spaces with escaped spaces
        assert_eq!(trim_end(String::from(r"abc\  ")), r"abc\ ");
        // Trailing spaces with escaped spaces with non-trailing spaces
        assert_eq!(trim_end(String::from(r" a bc\  ")), r" a bc\ ");

        // Handling double escape
        assert_eq!(trim_end(String::from(r"\\ ")), r"\\");
        assert_eq!(trim_end(String::from(r"\ \ ")), r"\ \ ");

        // Handling unicode characters
        assert_eq!(trim_end(String::from(" 게")), " 게");
        assert_eq!(trim_end(String::from("게 ")), "게");
    }

    #[test]
    fn test_split() {
        // Empty string
        assert_eq!("".split('!').collect::<Vec<_>>(), vec![""]);
        assert_eq!(split("", '!').collect::<Vec<String>>(), vec![""]);
        // No matches
        assert_eq!(split("abc", '!').collect::<Vec<String>>(), vec!["abc"]);
        // Split at start
        assert_eq!(split("/abc", '/').collect::<Vec<String>>(), vec!["", "abc"]);
        // Split at end
        assert_eq!(split("abc/", '/').collect::<Vec<String>>(), vec!["abc", ""]);
        // Simple match
        assert_eq!(
            split("abc,def,ghi", ',').collect::<Vec<String>>(),
            vec!["abc", "def", "ghi"]
        );
        // Escaped split character
        assert_eq!(
            split(r"abc\,def\,ghi", ',').collect::<Vec<String>>(),
            vec![r"abc\,def\,ghi"]
        );
        // Mixed escaped and un-escaped split characters
        assert_eq!(
            split(r"abc\,def,ghi", ',').collect::<Vec<String>>(),
            vec![r"abc\,def", "ghi"]
        );
    }
}
