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

pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub source: &'static str,
}

pub const TEMPLATES: &[Template] = &[
    Template {
        id: "classic",
        name: "Classic",
        description: "Single column, generous whitespace, ATS-friendly.",
        source: include_str!("../../../templates/classic/main.typ"),
    },
    Template {
        id: "flowcv",
        name: "FlowCV",
        description: "Centred header, tinted section bands, roles behind a hairline.",
        source: include_str!("../../../templates/flowcv/main.typ"),
    },
    Template {
        id: "modern",
        name: "Modern",
        description: "Accent-coloured headings with rules and a bolder name.",
        source: include_str!("../../../templates/modern/main.typ"),
    },
    Template {
        id: "compact",
        name: "Compact",
        description: "Tighter type and spacing, for CVs that spill onto a second page.",
        source: include_str!("../../../templates/compact/main.typ"),
    },
];

pub fn get(id: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.id == id)
}

pub fn default_template() -> &'static Template {
    &TEMPLATES[0]
}
