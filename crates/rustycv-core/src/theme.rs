use serde::{Deserialize, Serialize};

/// The styling knobs a user can turn. Templates read these; users never write Typst.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Theme {
    /// Hex colour, `#rrggbb`. Used for headings, rules and links.
    pub accent: String,
    /// A family name the renderer has a font for, e.g. "Inter".
    pub font_family: String,
    pub font_size_pt: f32,
    pub page: PageSize,
    pub margin_mm: f32,
    /// Multiplier on the default leading.
    pub line_height: f32,
    /// Vertical gap between sections, in mm.
    pub section_gap_mm: f32,
    pub heading_style: HeadingStyle,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: "#1f2937".to_string(),
            font_family: "Inter".to_string(),
            font_size_pt: 10.0,
            page: PageSize::A4,
            margin_mm: 16.0,
            line_height: 1.0,
            section_gap_mm: 5.0,
            heading_style: HeadingStyle::Bold,
        }
    }
}

impl Theme {
    /// Clamp every numeric knob into a range that still produces a sane page.
    ///
    /// The API accepts arbitrary JSON, so this is what stops a `fontSizePt` of
    /// 10000 from turning a render into an OOM.
    pub fn sanitized(&self) -> Self {
        let mut t = self.clone();
        t.font_size_pt = t.font_size_pt.clamp(6.0, 18.0);
        t.margin_mm = t.margin_mm.clamp(5.0, 40.0);
        t.line_height = t.line_height.clamp(0.7, 2.5);
        t.section_gap_mm = t.section_gap_mm.clamp(0.0, 20.0);
        if !is_hex_color(&t.accent) {
            t.accent = Theme::default().accent;
        }
        if t.font_family.trim().is_empty() {
            t.font_family = Theme::default().font_family;
        }
        t
    }
}

fn is_hex_color(s: &str) -> bool {
    let s = s.strip_prefix('#').unwrap_or(s);
    matches!(s.len(), 3 | 6) && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PageSize {
    #[default]
    A4,
    Letter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HeadingStyle {
    /// Bold text, no rule. What FlowCV's default does.
    #[default]
    Bold,
    /// Bold text with a hairline rule underneath.
    Underline,
    /// Uppercase, letter-spaced.
    Caps,
}
