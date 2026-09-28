//! The built-in template registry.
//!
//! Templates are embedded at build time. Users pick one and turn the knobs in
//! [`rustycv_core::Theme`]; they never author Typst, so no untrusted code ever
//! reaches the compiler.

/// Icons templates can `read()` and tint, mounted under `/icons/`.
///
/// Font Awesome Free 6 (icons: CC BY 4.0) — see NOTICE.md. Shipped as SVG
/// rather than an icon font so a template can recolour one by rewriting its
/// `fill` before handing it to `image`.
pub const ICONS: &[(&str, &[u8])] = &[
    (
        "/icons/brain.svg",
        include_bytes!("../../../assets/icons/brain.svg"),
    ),
    (
        "/icons/briefcase.svg",
        include_bytes!("../../../assets/icons/briefcase.svg"),
    ),
    (
        "/icons/certificate.svg",
        include_bytes!("../../../assets/icons/certificate.svg"),
    ),
    (
        "/icons/envelope.svg",
        include_bytes!("../../../assets/icons/envelope.svg"),
    ),
    (
        "/icons/github.svg",
        include_bytes!("../../../assets/icons/github.svg"),
    ),
    (
        "/icons/graduation-cap.svg",
        include_bytes!("../../../assets/icons/graduation-cap.svg"),
    ),
    (
        "/icons/heart.svg",
        include_bytes!("../../../assets/icons/heart.svg"),
    ),
    (
        "/icons/language.svg",
        include_bytes!("../../../assets/icons/language.svg"),
    ),
    (
        "/icons/link.svg",
        include_bytes!("../../../assets/icons/link.svg"),
    ),
    (
        "/icons/linkedin-in.svg",
        include_bytes!("../../../assets/icons/linkedin-in.svg"),
    ),
    (
        "/icons/location-dot.svg",
        include_bytes!("../../../assets/icons/location-dot.svg"),
    ),
    (
        "/icons/phone.svg",
        include_bytes!("../../../assets/icons/phone.svg"),
    ),
    (
        "/icons/rocket.svg",
        include_bytes!("../../../assets/icons/rocket.svg"),
    ),
    (
        "/icons/star.svg",
        include_bytes!("../../../assets/icons/star.svg"),
    ),
    (
        "/icons/user.svg",
        include_bytes!("../../../assets/icons/user.svg"),
    ),
];

/// Helpers every template imports, mounted at `/common.typ`.
pub const COMMON: &str = include_str!("../../../templates/common.typ");

/// The spacing a template is designed around.
///
/// These are per-template rather than app-wide because "reset to defaults"
/// has to mean something different for each: `flowcv` reproduces a layout
/// measured at 9pt with 10mm margins, and resetting it to the generic 10pt
/// would quietly break the reproduction.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub font_size_pt: f32,
    pub margin_mm: f32,
    pub line_height: f32,
    pub section_gap_mm: f32,
}

/// What most templates want, and what `Theme::default()` uses.
/// `standard_metrics_match_theme_default` keeps the two from drifting.
const STANDARD: Metrics = Metrics {
    font_size_pt: 10.0,
    margin_mm: 16.0,
    line_height: 1.0,
    section_gap_mm: 5.0,
};

pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub source: &'static str,
    /// Where "reset design" puts the spacing controls for this template.
    pub metrics: Metrics,
}

pub const TEMPLATES: &[Template] = &[
    Template {
        id: "classic",
        name: "Classic",
        description: "Single column, generous whitespace, ATS-friendly.",
        source: include_str!("../../../templates/classic/main.typ"),
        metrics: STANDARD,
    },
    Template {
        id: "flowcv",
        name: "FlowCV",
        description: "Centred header, tinted section bands, roles behind a hairline.",
        source: include_str!("../../../templates/flowcv/main.typ"),
        // Measured from the published FlowCV resume this template reproduces.
        metrics: Metrics {
            font_size_pt: 9.0,
            margin_mm: 10.0,
            // 3mm is what this template's original fixed 0.95em came to at 9pt.
            section_gap_mm: 3.0,
            ..STANDARD
        },
    },
    Template {
        id: "modern",
        name: "Modern",
        description: "Accent-coloured headings with rules and a bolder name.",
        source: include_str!("../../../templates/modern/main.typ"),
        metrics: STANDARD,
    },
    Template {
        id: "engineer",
        name: "Engineer",
        description: "Dense single column, ruled small-caps headings, no graphics — built to survive a parser.",
        source: include_str!("../../../templates/engineer/main.typ"),
        // Engineers' CVs are bullet-heavy, so this one is designed a little
        // tighter than standard — full-size type, less air around it.
        metrics: Metrics {
            margin_mm: 14.0,
            section_gap_mm: 4.5,
            ..STANDARD
        },
    },
    Template {
        id: "banner",
        name: "Banner",
        description: "Name and contact reversed out of a filled accent block, with accent bars beside headings.",
        source: include_str!("../../../templates/banner/main.typ"),
        metrics: STANDARD,
    },
    Template {
        id: "compact",
        name: "Compact",
        description: "Tighter type and spacing, for CVs that spill onto a second page.",
        source: include_str!("../../../templates/compact/main.typ"),
        // Compact already tightens everything internally, so it starts from the
        // standard numbers rather than pre-shrunk ones.
        metrics: STANDARD,
    },
    Template {
        id: "sidebar",
        name: "Sidebar",
        description: "Two columns: a tinted rail for contact, skills and languages beside the story.",
        source: include_str!("../../../templates/sidebar/main.typ"),
        metrics: STANDARD,
    },
    Template {
        id: "academic",
        name: "Academic",
        description: "A sober serif CV with dates in a left gutter, built to run to several pages.",
        source: include_str!("../../../templates/academic/main.typ"),
        metrics: STANDARD,
    },
    Template {
        id: "executive",
        name: "Executive",
        description: "Wide-tracked capitals, a lead-paragraph summary and a lot of air.",
        source: include_str!("../../../templates/executive/main.typ"),
        metrics: Metrics {
            margin_mm: 20.0,
            section_gap_mm: 6.0,
            ..STANDARD
        },
    },
    Template {
        id: "timeline",
        name: "Timeline",
        description: "Experience as a career path: one rail per employer with a node at every role.",
        source: include_str!("../../../templates/timeline/main.typ"),
        metrics: STANDARD,
    },
    Template {
        id: "mono",
        name: "Mono",
        description: "A README in print: monospace headings and dates, skills as code chips.",
        source: include_str!("../../../templates/mono/main.typ"),
        metrics: STANDARD,
    },
    Template {
        id: "editorial",
        name: "Editorial",
        description: "A large display-serif name, numbered sections and the summary as a pull quote.",
        source: include_str!("../../../templates/editorial/main.typ"),
        metrics: Metrics {
            margin_mm: 18.0,
            ..STANDARD
        },
    },
    Template {
        id: "plain",
        name: "Plain",
        description: "Black text, labelled contact lines, no graphics: for portals that parse badly.",
        source: include_str!("../../../templates/plain/main.typ"),
        metrics: STANDARD,
    },
];

pub fn get(id: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.id == id)
}

pub fn default_template() -> &'static Template {
    &TEMPLATES[0]
}
