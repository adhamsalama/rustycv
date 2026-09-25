//! Inline rich text.
//!
//! A description is an ordered list of styled [`Run`]s rather than a string of
//! HTML or Markdown. That means nothing ever has to parse untrusted markup: the
//! editor writes runs, the renderer reads runs, and there is no representation
//! in between that could carry a script tag or an unbalanced tag into a PDF.
//!
//! # Wire format
//!
//! Unstyled text serializes as a plain JSON string, and only text that actually
//! carries marks becomes an array of runs:
//!
//! ```json
//! "Reduced API latency by 50%."
//! [{"text": "Improved accuracy to "}, {"text": "98.5%", "bold": true}]
//! ```
//!
//! Both forms are accepted on the way in. That keeps hand-edited fixtures and
//! JSON exports readable, and means every document written before rich text
//! existed still loads — a bare string is simply one unstyled run.

use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

/// A span of text sharing one set of marks.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Run {
    pub text: String,
    #[serde(skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub italic: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub underline: bool,
    /// Empty when the run is not a link.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub link: String,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Run {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Default::default()
        }
    }

    /// True when this run carries no formatting and could be written as a bare
    /// string.
    pub fn is_plain(&self) -> bool {
        !self.bold && !self.italic && !self.underline && self.link.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RichText(Vec<Run>);

impl RichText {
    pub fn plain(text: impl Into<String>) -> Self {
        let text = text.into();
        if text.is_empty() {
            Self::default()
        } else {
            Self(vec![Run::plain(text)])
        }
    }

    pub fn from_runs(runs: Vec<Run>) -> Self {
        Self(runs.into_iter().filter(|r| !r.text.is_empty()).collect())
    }

    pub fn runs(&self) -> &[Run] {
        &self.0
    }

    /// True when nothing would render — no runs, or nothing but whitespace.
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|r| r.text.trim().is_empty())
    }

    /// The text with all formatting dropped, for places that need a plain
    /// string (search, filenames, a section summary in the editor).
    pub fn plain_text(&self) -> String {
        self.0.iter().map(|r| r.text.as_str()).collect()
    }
}

impl From<&str> for RichText {
    fn from(text: &str) -> Self {
        Self::plain(text)
    }
}

impl Serialize for RichText {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0.as_slice() {
            // Collapse to a bare string whenever no formatting would be lost,
            // so unstyled documents stay readable and diff cleanly.
            [] => serializer.serialize_str(""),
            [only] if only.is_plain() => serializer.serialize_str(&only.text),
            runs => runs.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for RichText {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Plain(String),
            Runs(Vec<Run>),
        }

        Ok(match Repr::deserialize(deserializer)? {
            Repr::Plain(text) => RichText::plain(text),
            Repr::Runs(runs) => RichText::from_runs(runs),
        })
    }
}
