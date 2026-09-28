//! Fonts bundled into the binary.
//!
//! Embedding rather than reading from disk keeps the server self-contained and,
//! more importantly, makes a render reproducible: the same document always
//! produces the same PDF regardless of what is installed on the host.

use std::sync::OnceLock;

use typst::foundations::Bytes;
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;

/// `(family label, embedded bytes)`. The label is only for [`families`]; Typst
/// matches on the family name it reads out of the font file itself.
const EMBEDDED: &[(&str, &[u8])] = &[
    (
        "Inter",
        include_bytes!("../../../assets/fonts/Inter-Regular.ttf"),
    ),
    (
        "Inter",
        include_bytes!("../../../assets/fonts/Inter-Italic.ttf"),
    ),
    (
        "Inter",
        include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf"),
    ),
    (
        "Inter",
        include_bytes!("../../../assets/fonts/Inter-Bold.ttf"),
    ),
    (
        "Inter",
        include_bytes!("../../../assets/fonts/Inter-BoldItalic.ttf"),
    ),
    (
        "IBM Plex Sans",
        include_bytes!("../../../assets/fonts/IBMPlexSans-Regular.ttf"),
    ),
    (
        "IBM Plex Sans",
        include_bytes!("../../../assets/fonts/IBMPlexSans-Italic.ttf"),
    ),
    (
        "IBM Plex Sans",
        include_bytes!("../../../assets/fonts/IBMPlexSans-SemiBold.ttf"),
    ),
    (
        "IBM Plex Sans",
        include_bytes!("../../../assets/fonts/IBMPlexSans-Bold.ttf"),
    ),
    (
        "IBM Plex Sans",
        include_bytes!("../../../assets/fonts/IBMPlexSans-BoldItalic.ttf"),
    ),
    (
        "Source Sans 3",
        include_bytes!("../../../assets/fonts/SourceSans3-Regular.ttf"),
    ),
    (
        "Source Sans 3",
        include_bytes!("../../../assets/fonts/SourceSans3-It.ttf"),
    ),
    (
        "Source Sans 3",
        include_bytes!("../../../assets/fonts/SourceSans3-Semibold.ttf"),
    ),
    (
        "Source Sans 3",
        include_bytes!("../../../assets/fonts/SourceSans3-Bold.ttf"),
    ),
    (
        "Source Sans 3",
        include_bytes!("../../../assets/fonts/SourceSans3-BoldIt.ttf"),
    ),
    (
        "Source Serif 4",
        include_bytes!("../../../assets/fonts/SourceSerif4-Regular.ttf"),
    ),
    (
        "Source Serif 4",
        include_bytes!("../../../assets/fonts/SourceSerif4-It.ttf"),
    ),
    (
        "Source Serif 4",
        include_bytes!("../../../assets/fonts/SourceSerif4-Semibold.ttf"),
    ),
    (
        "Source Serif 4",
        include_bytes!("../../../assets/fonts/SourceSerif4-Bold.ttf"),
    ),
    (
        "Source Serif 4",
        include_bytes!("../../../assets/fonts/SourceSerif4-BoldIt.ttf"),
    ),
    (
        "IBM Plex Mono",
        include_bytes!("../../../assets/fonts/IBMPlexMono-Regular.ttf"),
    ),
    (
        "IBM Plex Mono",
        include_bytes!("../../../assets/fonts/IBMPlexMono-Italic.ttf"),
    ),
    (
        "IBM Plex Mono",
        include_bytes!("../../../assets/fonts/IBMPlexMono-SemiBold.ttf"),
    ),
    (
        "IBM Plex Mono",
        include_bytes!("../../../assets/fonts/IBMPlexMono-Bold.ttf"),
    ),
];

/// The font families a user may choose in the theme picker, deduplicated and
/// in the order they are declared above.
pub fn families() -> Vec<&'static str> {
    let mut seen = Vec::new();
    for (label, _) in EMBEDDED {
        if !seen.contains(label) {
            seen.push(*label);
        }
    }
    seen
}

pub struct Fonts {
    pub book: LazyHash<FontBook>,
    pub fonts: Vec<Font>,
}

/// Parse every embedded font once, for the life of the process.
///
/// `Font` is reference-counted, so cloning out of here is cheap and every
/// `CvWorld` shares the same parsed faces.
pub fn shared() -> &'static Fonts {
    static FONTS: OnceLock<Fonts> = OnceLock::new();
    FONTS.get_or_init(|| {
        let fonts: Vec<Font> = EMBEDDED
            .iter()
            .flat_map(|(_, data)| Font::iter(Bytes::new(*data)))
            .collect();
        assert!(!fonts.is_empty(), "no embedded font could be parsed");
        let book = FontBook::from_fonts(&fonts);
        Fonts {
            book: LazyHash::new(book),
            fonts,
        }
    })
}
