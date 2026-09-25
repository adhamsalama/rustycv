//! Renders a [`CvDocument`] to PDF with an embedded Typst compiler.
//!
//! Nothing is cached to disk and no PDF is ever persisted: a PDF is a pure
//! function of the document, its template and its theme, recomputed on demand.

pub mod fonts;
pub mod templates;
pub mod world;

use rustycv_core::CvDocument;
use typst::diag::Severity;
use typst::WorldExt;
use typst_layout::PagedDocument;

pub use templates::{Template, TEMPLATES};

use world::CvWorld;

/// One message from the Typst compiler, resolved to a file and line.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    /// `"error"` or `"warning"`.
    pub severity: String,
    pub message: String,
    pub file: Option<String>,
    /// 1-based, to match what an editor would show.
    pub line: Option<usize>,
    pub hints: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("unknown template: {0}")]
    UnknownTemplate(String),
    #[error("could not serialize the CV: {0}")]
    Serialize(#[from] serde_json::Error),
    /// The template failed to compile. Surfaced to the client rather than
    /// swallowed, so a broken preview says *why* it is broken.
    #[error("typst failed to compile the document ({} diagnostics)", .0.len())]
    Typst(Vec<Diagnostic>),
}

/// Evict Typst's memoization arena every this many renders.
///
/// `comemo`'s cache is global and grows with each distinct compile. Under a
/// live preview that is a new compile per keystroke-ish, so it needs draining;
/// evicting on every render instead would throw away the incremental reuse that
/// makes those previews fast.
const EVICT_EVERY: u64 = 32;
const EVICT_MAX_AGE: usize = 8;

static RENDER_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Compile a document to a laid-out Typst document.
///
/// Both exporters go through here, so a PDF and a preview image of the same
/// document can never disagree about layout.
fn compile(doc: &CvDocument) -> Result<(CvWorld, PagedDocument), RenderError> {
    let template = templates::get(&doc.template)
        .ok_or_else(|| RenderError::UnknownTemplate(doc.template.clone()))?;

    // Hand the template a document whose theme is already clamped, so a hostile
    // or fat-fingered value can't turn into a pathological layout.
    let mut doc = doc.clone();
    doc.theme = doc.theme.sanitized();
    let data_json = serde_json::to_string(&doc)?;

    let world = CvWorld::new(
        template.source,
        &[("/common.typ", templates::COMMON)],
        templates::ICONS,
        data_json,
        None,
    );

    let compiled = typst::compile::<PagedDocument>(&world);

    // `comemo`'s cache is global and grows with each distinct compile. Under a
    // live preview that is a new compile per edit, so it needs draining;
    // evicting on every render instead would throw away the incremental reuse
    // that makes those previews fast.
    if RENDER_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % EVICT_EVERY
        == EVICT_EVERY - 1
    {
        comemo::evict(EVICT_MAX_AGE);
    }

    match compiled.output {
        Ok(document) => Ok((world, document)),
        Err(errors) => {
            let diagnostics = to_diagnostics(&world, &errors);
            Err(RenderError::Typst(diagnostics))
        }
    }
}

/// Render a document to PDF bytes.
///
/// This blocks and is CPU-bound — callers on an async runtime must put it on a
/// blocking thread.
pub fn render_pdf(doc: &CvDocument) -> Result<Vec<u8>, RenderError> {
    let (world, document) = compile(doc)?;
    typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|errors| RenderError::Typst(to_diagnostics(&world, &errors)))
}

/// Render each page to a PNG at the given resolution.
///
/// Used for gallery thumbnails and for visual regression tests.
pub fn render_pngs(doc: &CvDocument, ppi: f32) -> Result<Vec<Vec<u8>>, RenderError> {
    let (_world, document) = compile(doc)?;
    let options = typst_render::RenderOptions {
        pixel_per_pt: typst::utils::Scalar::new(ppi as f64 / 72.0),
        ..Default::default()
    };
    Ok(document
        .pages()
        .iter()
        .map(|page| {
            typst_render::render(page, &options)
                .encode_png()
                .expect("a rendered pixmap always encodes as PNG")
        })
        .collect())
}

fn to_diagnostics(world: &CvWorld, errors: &[typst::diag::SourceDiagnostic]) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|d| {
            let (file, line) = locate(world, d.span);
            Diagnostic {
                severity: match d.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                }
                .to_string(),
                message: d.message.to_string(),
                file,
                line,
                hints: d.hints.iter().map(|h| h.v.to_string()).collect(),
            }
        })
        .collect()
}

/// Resolve a span to `(file, 1-based line)` for a readable error message.
fn locate(world: &CvWorld, span: typst::syntax::DiagSpan) -> (Option<String>, Option<usize>) {
    let Some(id) = span.id() else {
        return (None, None);
    };
    let path = id.get().vpath().get_with_slash().to_string();
    let line = world
        .range(span)
        .and_then(|range| {
            use typst::World;
            let source = world.source(id).ok()?;
            source.lines().byte_to_line(range.start)
        })
        .map(|l| l + 1);
    (Some(path), line)
}
