use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::date::DateSpec;
use crate::defaults;

/// One section of a CV: a heading plus a typed list of entries.
///
/// The common fields live here and the entries live in [`SectionBody`], which is
/// flattened on the wire. A section therefore serializes flat:
/// `{ "id": ..., "title": "Work Experience", "visible": true,
///    "kind": "experience", "items": [...] }`
/// which keeps the Typst templates simple.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    /// User-editable heading. Renaming "Work Experience" to "Experience" is a
    /// data change, not a template change.
    pub title: String,
    #[serde(default = "defaults::r#true")]
    pub visible: bool,
    #[serde(flatten)]
    pub body: SectionBody,
}

impl Section {
    /// An empty section of the given kind, with that kind's conventional title.
    pub fn new(kind: SectionKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: kind.default_title().to_string(),
            visible: true,
            body: kind.empty_body(),
        }
    }

    pub fn kind(&self) -> SectionKind {
        self.body.kind()
    }

    /// True when nothing would render — no entries, or every entry hidden.
    pub fn is_empty(&self) -> bool {
        self.body.visible_len() == 0
    }

    /// Total entries, hidden ones included. This is the count the editor shows.
    pub fn len(&self) -> usize {
        self.body.len()
    }

    /// Entries that would actually render.
    pub fn visible_len(&self) -> usize {
        self.body.visible_len()
    }

    pub fn reassign_ids(&mut self) {
        self.id = Uuid::new_v4();
        self.body.reassign_ids();
    }
}

/// The entries of a section, tagged by `kind`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SectionBody {
    Experience {
        #[serde(default)]
        items: Vec<ExperienceItem>,
    },
    Education {
        #[serde(default)]
        items: Vec<EducationItem>,
    },
    Skills {
        #[serde(default)]
        groups: Vec<SkillGroup>,
    },
    Projects {
        #[serde(default)]
        items: Vec<ProjectItem>,
    },
    Certifications {
        #[serde(default)]
        items: Vec<CertificationItem>,
    },
    Languages {
        #[serde(default)]
        items: Vec<LanguageItem>,
    },
    Interests {
        #[serde(default)]
        items: Vec<InterestItem>,
    },
    References {
        #[serde(default)]
        items: Vec<ReferenceItem>,
    },
}

impl SectionBody {
    pub fn kind(&self) -> SectionKind {
        match self {
            Self::Experience { .. } => SectionKind::Experience,
            Self::Education { .. } => SectionKind::Education,
            Self::Skills { .. } => SectionKind::Skills,
            Self::Projects { .. } => SectionKind::Projects,
            Self::Certifications { .. } => SectionKind::Certifications,
            Self::Languages { .. } => SectionKind::Languages,
            Self::Interests { .. } => SectionKind::Interests,
            Self::References { .. } => SectionKind::References,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Experience { items } => items.len(),
            Self::Education { items } => items.len(),
            Self::Skills { groups } => groups.len(),
            Self::Projects { items } => items.len(),
            Self::Certifications { items } => items.len(),
            Self::Languages { items } => items.len(),
            Self::Interests { items } => items.len(),
            Self::References { items } => items.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn visible_len(&self) -> usize {
        macro_rules! count {
            ($items:expr) => {
                $items.iter().filter(|it| it.visible).count()
            };
        }
        match self {
            Self::Experience { items } => count!(items),
            Self::Education { items } => count!(items),
            Self::Skills { groups } => count!(groups),
            Self::Projects { items } => count!(items),
            Self::Certifications { items } => count!(items),
            Self::Languages { items } => count!(items),
            Self::Interests { items } => count!(items),
            Self::References { items } => count!(items),
        }
    }

    fn reassign_ids(&mut self) {
        macro_rules! refresh {
            ($items:expr) => {
                for it in $items.iter_mut() {
                    it.id = Uuid::new_v4();
                }
            };
        }
        match self {
            Self::Experience { items } => refresh!(items),
            Self::Education { items } => refresh!(items),
            Self::Skills { groups } => refresh!(groups),
            Self::Projects { items } => refresh!(items),
            Self::Certifications { items } => refresh!(items),
            Self::Languages { items } => refresh!(items),
            Self::Interests { items } => refresh!(items),
            Self::References { items } => refresh!(items),
        }
    }
}

/// The set of section types the app knows how to edit and render.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SectionKind {
    Experience,
    Education,
    Skills,
    Projects,
    Certifications,
    Languages,
    Interests,
    References,
}

impl SectionKind {
    pub const ALL: [SectionKind; 8] = [
        Self::Experience,
        Self::Education,
        Self::Skills,
        Self::Projects,
        Self::Certifications,
        Self::Languages,
        Self::Interests,
        Self::References,
    ];

    pub fn default_title(self) -> &'static str {
        match self {
            Self::Experience => "Work Experience",
            Self::Education => "Education",
            Self::Skills => "Skills",
            Self::Projects => "Projects",
            Self::Certifications => "Certifications",
            Self::Languages => "Languages",
            Self::Interests => "Interests",
            Self::References => "References",
        }
    }

    fn empty_body(self) -> SectionBody {
        match self {
            Self::Experience => SectionBody::Experience { items: vec![] },
            Self::Education => SectionBody::Education { items: vec![] },
            Self::Skills => SectionBody::Skills { groups: vec![] },
            Self::Projects => SectionBody::Projects { items: vec![] },
            Self::Certifications => SectionBody::Certifications { items: vec![] },
            Self::Languages => SectionBody::Languages { items: vec![] },
            Self::Interests => SectionBody::Interests { items: vec![] },
            Self::References => SectionBody::References { items: vec![] },
        }
    }
}

/// Generates an item struct with an `id` that defaults to a fresh UUID and
/// every other field defaulting to empty. Saves eight near-identical blocks.
macro_rules! item {
    (
        $(#[$meta:meta])*
        $name:ident { $( $(#[$fmeta:meta])* $field:ident : $ty:ty ),* $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase", default)]
        pub struct $name {
            pub id: Uuid,
            /// Hidden entries stay in the document but are not rendered, so a CV
            /// can be tailored for one application without deleting anything.
            ///
            /// The container-level `#[serde(default)]` fills this from `Default`,
            /// so documents written before the field existed load as visible.
            pub visible: bool,
            $( $(#[$fmeta])* pub $field: $ty, )*
        }

        impl Default for $name {
            fn default() -> Self {
                Self { id: Uuid::new_v4(), visible: true, $( $field: Default::default(), )* }
            }
        }
    };
}

item! {
    /// One role. Consecutive roles sharing a `company` are grouped under a single
    /// company heading by the templates.
    ExperienceItem {
        role: String,
        company: String,
        company_url: String,
        location: String,
        start: Option<DateSpec>,
        end: Option<DateSpec>,
        /// When true the end date is rendered as "Present" and `end` is ignored.
        current: bool,
        bullets: Vec<String>,
    }
}

item! {
    EducationItem {
        degree: String,
        institution: String,
        location: String,
        start: Option<DateSpec>,
        end: Option<DateSpec>,
        current: bool,
        /// Free text under the entry, e.g. "Overall grade: Very Good with honors."
        description: String,
    }
}

item! {
    /// A named group of skills, e.g. "Languages: Go, Rust".
    SkillGroup {
        name: String,
        items: Vec<String>,
    }
}

item! {
    ProjectItem {
        name: String,
        url: String,
        description: String,
        /// Rendered as a trailing "Go, Kubernetes, React" line when non-empty.
        tech: Vec<String>,
    }
}

item! {
    CertificationItem {
        name: String,
        issuer: String,
        url: String,
        date: Option<DateSpec>,
    }
}

item! {
    LanguageItem {
        name: String,
        /// e.g. "Native", "C1".
        level: String,
    }
}

item! {
    InterestItem {
        name: String,
    }
}

item! {
    ReferenceItem {
        name: String,
        url: String,
        title: String,
        company: String,
        email: String,
        phone: String,
    }
}
