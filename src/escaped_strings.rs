/// Utility functions for string operations that support escaping.

const ESCAPE_CHAR: char = '\\';

/// Finds a character in a string if it is not escaped.
pub fn find_char(string: &String, target_char: char) -> Option<usize> {
    let mut prev_c: Option<char> = None;
    for (i, c) in string.chars().enumerate() {
        if c == target_char && (prev_c.map_or(true, |prev_c| prev_c != ESCAPE_CHAR)) {
            return Some(i);
        }

        prev_c = Some(c);
    }

    None
}

pub fn trim_end(mut string: String) -> String {
    // Represents the start of the trailing spaces.
    // Escaped spaces are not considered spaces.
    let mut truncate_len: Option<usize> = None;
    let mut prev_truncate_len: Option<usize> = None;

    for (i, c) in string.chars().rev().enumerate() {
        match c {
            ' ' => {
                // Save the previous truncate index.
                // See comment below.
                prev_truncate_len = truncate_len;

                let index = string.len() - i - 1;
                truncate_len = Some(index);
            }
            ESCAPE_CHAR => {
                // The current truncate index is for an escaped space,
                // so, we have to revert to the previous truncate index.
                truncate_len = prev_truncate_len;
                break;
            }
            _ => break,
        }
    }

    let Some(truncate_index) = truncate_len else {
        return string;
    };

    string.truncate(truncate_index);
    string
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_comment() {
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
    }

    #[test]
    fn test_trim_end() {
        // Empty string
        assert_eq!(trim_end(String::from("")), "");
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
    }
}
