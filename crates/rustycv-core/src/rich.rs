//! Rich text.
//!
//! A description is a short stack of [`Block`]s — paragraphs and list items —
//! each holding an ordered list of styled [`Run`]s rather than a string of HTML
//! or Markdown. That means nothing ever has to parse untrusted markup: the
//! editor writes blocks, the renderer reads blocks, and there is no
//! representation in between that could carry a script tag or an unbalanced tag
//! into a PDF.
//!
//! # Wire format
//!
//! Three forms, and a value is written in the smallest one that fits. Unstyled
//! single-paragraph text stays a bare JSON string, one styled paragraph stays a
//! flat array of runs, and only text that actually needs blocks becomes one:
//!
//! ```json
//! "Reduced API latency by 50%."
//! [{"text": "Improved accuracy to "}, {"text": "98.5%", "bold": true}]
//! [{"kind": "paragraph", "runs": [{"text": "Led the rewrite."}]},
//!  {"kind": "bullet", "runs": [{"text": "Cut p99 latency by 40%."}]}]
//! ```
//!
//! All three are accepted on the way in, so hand-edited fixtures and JSON
//! exports stay readable and every document written before rich text existed
//! still loads — a bare string is simply one unstyled paragraph.
//!
//! A soft line break *inside* a paragraph (Shift+Enter in the editor) is a
//! newline in a run's text, which keeps the common "two lines, no styling" case
//! a bare string too.

use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

/// A span of text sharing one set of marks.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Run {
    /// A newline here is a soft line break within the block.
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

/// What a block is: a paragraph, or one item of a bulleted or numbered list.
///
/// Consecutive list blocks of the same kind render as one list — the list
/// itself is not a node, which keeps the format flat and unnestable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BlockKind {
    #[default]
    Paragraph,
    Bullet,
    Numbered,
}

/// One paragraph or list item.
///
/// Neither field takes a serde default on purpose: a run object has no `kind`
/// and no `runs`, so requiring both is what keeps a block and a run tellable
/// apart when [`RichText`] decides which array form it is reading.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub kind: BlockKind,
    pub runs: Vec<Run>,
}

impl Block {
    pub fn new(kind: BlockKind, runs: Vec<Run>) -> Self {
        Self {
            kind,
            runs: runs.into_iter().filter(|r| !r.text.is_empty()).collect(),
        }
    }

    pub fn paragraph(runs: Vec<Run>) -> Self {
        Self::new(BlockKind::Paragraph, runs)
    }

    /// True when nothing in this block would render.
    pub fn is_empty(&self) -> bool {
        self.runs.iter().all(|r| r.text.trim().is_empty())
    }

    /// True when this block could be written on a line already in progress —
    /// one paragraph, no list marker, no line break inside it.
    fn is_inline(&self) -> bool {
        self.kind == BlockKind::Paragraph && !self.runs.iter().any(|r| r.text.contains('\n'))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RichText(Vec<Block>);

impl RichText {
    pub fn plain(text: impl Into<String>) -> Self {
        let text = text.into();
        if text.is_empty() {
            Self::default()
        } else {
            Self(vec![Block::paragraph(vec![Run::plain(text)])])
        }
    }

    /// One paragraph of styled runs.
    pub fn from_runs(runs: Vec<Run>) -> Self {
        Self::from_blocks(vec![Block::paragraph(runs)])
    }

    /// Blocks, with anything that would render nothing dropped — an empty block
    /// would otherwise print as a blank line the user cannot see or delete.
    pub fn from_blocks(blocks: Vec<Block>) -> Self {
        Self(
            blocks
                .into_iter()
                .map(|b| Block::new(b.kind, b.runs))
                .filter(|b| !b.runs.is_empty())
                .collect(),
        )
    }

    pub fn blocks(&self) -> &[Block] {
        &self.0
    }

    /// Every run, whichever block it sits in.
    pub fn runs(&self) -> Vec<&Run> {
        self.0.iter().flat_map(|b| b.runs.iter()).collect()
    }

    /// True when nothing would render — no blocks, or nothing but whitespace.
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|b| b.is_empty())
    }

    /// True when the whole value could be written on a line already in progress:
    /// at most one paragraph, no lists, no line breaks. A project description
    /// renders after the project's name when this holds and below it when it
    /// does not.
    pub fn is_inline(&self) -> bool {
        let mut blocks = self.0.iter().filter(|b| !b.is_empty());
        match blocks.next() {
            None => true,
            Some(first) => first.is_inline() && blocks.next().is_none(),
        }
    }

    /// The text with all formatting dropped, for places that need a plain
    /// string (search, filenames, a section summary in the editor). Blocks are
    /// separated by newlines, the same as a line break inside one.
    pub fn plain_text(&self) -> String {
        self.0
            .iter()
            .map(|b| b.runs.iter().map(|r| r.text.as_str()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Read an experience entry's highlights.
///
/// They were once a list of values, one per bullet, and are now a single rich
/// value like every other description — so an old document's
/// `["Cut latency", "Shipped retries"]` loads as two bullet blocks. The two
/// shapes cannot be confused: the old one is an array of *values* (strings or
/// run arrays), and no element of it parses as a rich value on its own.
pub fn highlights<'de, D: Deserializer<'de>>(deserializer: D) -> Result<RichText, D::Error> {
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Repr {
        PerBullet(Vec<RichText>),
        Rich(RichText),
    }

    Ok(match Repr::deserialize(deserializer)? {
        Repr::PerBullet(values) => RichText::from_blocks(
            values
                .into_iter()
                .flat_map(|value| value.0)
                .map(|block| Block::new(BlockKind::Bullet, block.runs))
                .collect(),
        ),
        Repr::Rich(text) => text,
    })
}

impl From<&str> for RichText {
    fn from(text: &str) -> Self {
        Self::plain(text)
    }
}

impl Serialize for RichText {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0.as_slice() {
            // Collapse to the smallest form that loses nothing, so unstyled
            // documents stay readable and diff cleanly.
            [] => serializer.serialize_str(""),
            [only] if only.kind == BlockKind::Paragraph => match only.runs.as_slice() {
                [] => serializer.serialize_str(""),
                [run] if run.is_plain() => serializer.serialize_str(&run.text),
                runs => runs.serialize(serializer),
            },
            blocks => blocks.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for RichText {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Both array forms are arrays of objects, so order matters: blocks are
        // tried first and a run — which carries neither `kind` nor `runs` —
        // cannot satisfy them. Flip these and every block would read as a run
        // with no text.
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Plain(String),
            Blocks(Vec<Block>),
            Runs(Vec<Run>),
        }

        Ok(match Repr::deserialize(deserializer)? {
            Repr::Plain(text) => RichText::plain(text),
            Repr::Blocks(blocks) => RichText::from_blocks(blocks),
            Repr::Runs(runs) => RichText::from_runs(runs),
        })
    }
}
