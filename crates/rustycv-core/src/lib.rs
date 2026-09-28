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
pub mod limits;
mod rich;
mod section;
mod theme;

pub use date::DateSpec;
pub use limits::LimitError;
pub use rich::{Block, BlockKind, RichText, Run};
pub use section::{
    CertificationItem, CustomItem, EducationItem, EntryOrder, ExperienceItem, InterestItem,
    LanguageItem, ProjectItem, ReferenceItem, Section, SectionBody, SectionKind, SkillGroup,
};
pub use theme::{BulletStyle, HeadingStyle, PageSize, Theme};

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
    /// Check the document against [`limits`], so a hostile one is refused
    /// before it is stored or laid out.
    pub fn check_limits(&self) -> Result<(), LimitError> {
        let value = serde_json::to_value(self).expect("a CvDocument always serializes");
        limits::check(&value)
    }

    /// A new CV with one of each core section, filled with placeholder
    /// content so a first-time user sees what a finished CV looks like
    /// instead of a blank page, and edits the sample rather than starting
    /// from nothing.
    pub fn starter() -> Self {
        Self {
            basics: Basics {
                full_name: "Jordan Rivera".to_string(),
                headline: "Senior Software Engineer".to_string(),
                email: "jordan.rivera@example.com".to_string(),
                phone: "+1 555 123 4567".to_string(),
                location: "Austin, TX".to_string(),
                links: vec![
                    Link {
                        id: Uuid::new_v4(),
                        label: "GitHub".to_string(),
                        url: "https://github.com/jordanrivera".to_string(),
                    },
                    Link {
                        id: Uuid::new_v4(),
                        label: "LinkedIn".to_string(),
                        url: "https://linkedin.com/in/jordanrivera".to_string(),
                    },
                ],
                summary: "Backend-leaning engineer who enjoys turning ambiguous \
                    problems into reliable, well-tested systems. Comfortable owning \
                    a service end to end, from design through on-call."
                    .into(),
            },
            sections: vec![
                Section {
                    id: Uuid::new_v4(),
                    title: SectionKind::Experience.default_title().to_string(),
                    visible: true,
                    body: SectionBody::Experience {
                        items: vec![ExperienceItem {
                            role: "Software Engineer".to_string(),
                            company: "Acme Corp".to_string(),
                            location: "Austin, TX".to_string(),
                            start: Some(DateSpec::new(2022, 6)),
                            current: true,
                            bullets: RichText::from_blocks(vec![
                                Block::new(
                                    BlockKind::Bullet,
                                    vec![Run::plain(
                                        "Designed and shipped a service handling \
                                        2M+ requests a day, cutting p99 latency by 35%.",
                                    )],
                                ),
                                Block::new(
                                    BlockKind::Bullet,
                                    vec![Run::plain(
                                        "Led the migration to a new queueing system, \
                                        reducing failed job retries by half.",
                                    )],
                                ),
                                Block::new(
                                    BlockKind::Bullet,
                                    vec![Run::plain(
                                        "Mentored two junior engineers and ran the \
                                        team's weekly code review sessions.",
                                    )],
                                ),
                            ]),
                            ..Default::default()
                        }],
                        order: EntryOrder::default(),
                        group_promotions: true,
                    },
                },
                Section {
                    id: Uuid::new_v4(),
                    title: SectionKind::Education.default_title().to_string(),
                    visible: true,
                    body: SectionBody::Education {
                        items: vec![EducationItem {
                            degree: "B.S. in Computer Science".to_string(),
                            institution: "University of Texas at Austin".to_string(),
                            start: Some(DateSpec::year_only(2016)),
                            end: Some(DateSpec::year_only(2020)),
                            ..Default::default()
                        }],
                    },
                },
                Section {
                    id: Uuid::new_v4(),
                    title: SectionKind::Skills.default_title().to_string(),
                    visible: true,
                    body: SectionBody::Skills {
                        groups: vec![
                            SkillGroup {
                                name: "Languages".to_string(),
                                items: vec![
                                    "Rust".to_string(),
                                    "Go".to_string(),
                                    "TypeScript".to_string(),
                                ],
                                ..Default::default()
                            },
                            SkillGroup {
                                name: "Tools".to_string(),
                                items: vec![
                                    "PostgreSQL".to_string(),
                                    "Docker".to_string(),
                                    "AWS".to_string(),
                                ],
                                ..Default::default()
                            },
                        ],
                    },
                },
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
    pub summary: RichText,
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
