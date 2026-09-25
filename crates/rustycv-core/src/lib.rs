//! The CV document model.
//!
//! This is the single source of truth for what a CV *is*. It is what we persist
//! (as JSON, in SQLite) and what we hand to the renderer. PDFs are never stored:
//! they are a pure function of this document plus a template.
//!
//! Every field is `#[serde(default)]` so that documents written by older versions
//! of the app keep deserializing as the schema grows.

mod date;
mod defaults;
mod section;
mod theme;

pub use date::DateSpec;
pub use section::{
    CertificationItem, EducationItem, ExperienceItem, InterestItem, LanguageItem, ProjectItem,
    ReferenceItem, Section, SectionBody, SectionKind, SkillGroup,
};
pub use theme::{HeadingStyle, PageSize, Theme};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The current schema version. Bump when a change needs a migration step.
pub const SCHEMA_VERSION: u32 = 1;

/// A complete CV.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CvDocument {
    #[serde(default = "defaults::schema_version")]
    pub schema_version: u32,
    /// Id of the template to render with, e.g. `"classic"`.
    #[serde(default = "defaults::template")]
    pub template: String,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub basics: Basics,
    /// Order here *is* the order sections render in.
    #[serde(default)]
    pub sections: Vec<Section>,
}

impl Default for CvDocument {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            template: defaults::template(),
            theme: Theme::default(),
            basics: Basics::default(),
            sections: Vec::new(),
        }
    }
}

impl CvDocument {
    /// A new, empty CV with one of each core section, ready to fill in.
    pub fn starter() -> Self {
        Self {
            sections: vec![
                Section::new(SectionKind::Experience),
                Section::new(SectionKind::Education),
                Section::new(SectionKind::Skills),
            ],
            ..Default::default()
        }
    }

    /// Assign fresh ids to the document and every section and item within it.
    ///
    /// Used when duplicating or importing, so the copy never shares ids with the
    /// original (which would break React keys and drag-and-drop).
    pub fn reassign_ids(&mut self) {
        for section in &mut self.sections {
            section.reassign_ids();
        }
    }

    /// Sections that should actually be rendered, in order.
    pub fn visible_sections(&self) -> impl Iterator<Item = &Section> {
        self.sections.iter().filter(|s| s.visible && !s.is_empty())
    }
}

/// Name, contact details and summary — the part every template puts at the top.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Basics {
    pub full_name: String,
    /// The line under the name, e.g. "Staff Backend Engineer".
    pub headline: String,
    pub email: String,
    pub phone: String,
    pub location: String,
    pub links: Vec<Link>,
    pub summary: String,
}

/// A labelled external link shown in the contact line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    /// What to show, e.g. "GitHub".
    pub label: String,
    pub url: String,
}

impl Default for Link {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            label: String::new(),
            url: String::new(),
        }
    }
}
