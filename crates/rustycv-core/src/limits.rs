//! Ceilings on how much a document may hold.
//!
//! A CV is a page or two of text, so every bound here is far above anything a
//! real one reaches, and exists only so that a hostile document cannot make the
//! renderer lay out a novel or the database store one.
//!
//! The walk is over the *serialized* document rather than field by field, so a
//! field added later is covered without anyone remembering to list it.

use serde_json::Value;

/// The longest any single string may be, in characters. A long paragraph is a
/// few hundred; a run of rich text is at most one paragraph.
pub const MAX_STRING_CHARS: usize = 5_000;
/// The most text a whole document may carry, in characters — around thirty
/// dense pages, three times the page limit.
pub const MAX_TEXT_CHARS: usize = 100_000;
/// The most elements any one list may have: sections, entries, bullets, runs.
pub const MAX_LIST_LEN: usize = 200;
/// The most pages a render may produce.
pub const MAX_PAGES: usize = 10;
/// The longest a CV's or an application's title may be.
pub const MAX_TITLE_CHARS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LimitError {
    #[error("a text field is {0} characters long; the most allowed is {MAX_STRING_CHARS}")]
    StringTooLong(usize),
    #[error("the CV holds more than {MAX_TEXT_CHARS} characters of text")]
    TooMuchText,
    #[error("a list has {0} items; the most allowed is {MAX_LIST_LEN}")]
    ListTooLong(usize),
}

/// Check a serialized document against every limit.
pub fn check(value: &Value) -> Result<(), LimitError> {
    let mut total = 0;
    walk(value, &mut total)
}

fn walk(value: &Value, total: &mut usize) -> Result<(), LimitError> {
    match value {
        Value::String(s) => {
            let chars = s.chars().count();
            if chars > MAX_STRING_CHARS {
                return Err(LimitError::StringTooLong(chars));
            }
            *total += chars;
            if *total > MAX_TEXT_CHARS {
                return Err(LimitError::TooMuchText);
            }
        }
        Value::Array(items) => {
            if items.len() > MAX_LIST_LEN {
                return Err(LimitError::ListTooLong(items.len()));
            }
            for item in items {
                walk(item, total)?;
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                walk(item, total)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// A title, checked on its own because it lives beside the document.
pub fn check_title(title: &str) -> Result<(), String> {
    let chars = title.chars().count();
    if chars > MAX_TITLE_CHARS {
        Err(format!(
            "a title is {chars} characters long; the most allowed is {MAX_TITLE_CHARS}"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_long_string_is_refused() {
        let ok = "a".repeat(MAX_STRING_CHARS);
        assert_eq!(check(&json!({ "x": ok })), Ok(()));
        let long = "a".repeat(MAX_STRING_CHARS + 1);
        assert_eq!(
            check(&json!({ "x": long })),
            Err(LimitError::StringTooLong(MAX_STRING_CHARS + 1))
        );
    }

    #[test]
    fn many_short_strings_are_refused_in_total() {
        let chunk = "a".repeat(MAX_STRING_CHARS);
        let many: Vec<_> = (0..MAX_TEXT_CHARS / MAX_STRING_CHARS + 1)
            .map(|_| chunk.clone())
            .collect();
        assert_eq!(check(&json!(many)), Err(LimitError::TooMuchText));
    }

    #[test]
    fn a_long_list_is_refused() {
        let list: Vec<u8> = vec![0; MAX_LIST_LEN + 1];
        assert_eq!(
            check(&json!({ "deep": [{ "items": list }] })),
            Err(LimitError::ListTooLong(MAX_LIST_LEN + 1))
        );
    }
}
