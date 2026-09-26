//! The renderer, compiled for the browser.
//!
//! This crate contains no rendering of its own. It is a shim over
//! [`rustycv_render::render_pdf`] — the same function the server calls, over
//! the same templates, icons and fonts compiled into it — so a PDF made in the
//! browser is the same bytes as one made on the server. That is the whole
//! point: the option this exists for moves *where* a render runs, never what
//! it produces.
//!
//! The browser is also why [`rustycv_render::world::CvWorld`] serving only
//! memory matters twice over. There is no filesystem to reach for out here,
//! so a `World` that wanted one would not have compiled at all.

use rustycv_core::CvDocument;
use rustycv_render::RenderFailure;
use wasm_bindgen::prelude::*;

/// Render a CV, given as the same JSON the `/api/render` route accepts.
///
/// The error is the JSON body that route would have answered with — see
/// [`RenderFailure`] — so the editor turns a failed local render into the
/// identical message and diagnostics it shows for a failed remote one.
pub fn render_json(document_json: &str) -> Result<Vec<u8>, String> {
    let document: CvDocument = serde_json::from_str(document_json).map_err(|e| {
        // A parse failure is not a `RenderError`, but it is still something the
        // editor has to display, so it goes out in the same envelope.
        failure_json(&RenderFailure {
            error: e.to_string(),
            diagnostics: vec![],
        })
    })?;

    rustycv_render::render_pdf(&document).map_err(|e| failure_json(&RenderFailure::from(&e)))
}

/// Serialize a failure, falling back to its message if even that fails.
fn failure_json(failure: &RenderFailure) -> String {
    serde_json::to_string(failure).unwrap_or_else(|_| {
        // `RenderFailure` is plain strings, so this is unreachable in practice;
        // panicking here would poison the module for a merely broken template.
        format!("{{\"error\":{:?}}}", failure.error)
    })
}

/// Render a CV to PDF bytes.
///
/// Throws an `Error` whose message is the JSON failure body. A *panic* throws
/// too, with a message that is not JSON — the caller treats the difference as
/// the difference between "this document does not render" and "this module is
/// no longer trustworthy", because a panic leaves wasm memory in a state the
/// next call would inherit.
#[wasm_bindgen(js_name = renderPdf)]
pub fn render_pdf(document_json: &str) -> Result<Box<[u8]>, JsError> {
    match render_json(document_json) {
        Ok(pdf) => Ok(pdf.into_boxed_slice()),
        Err(body) => Err(JsError::new(&body)),
    }
}

/// The template ids this module can render, as JSON.
///
/// Not to populate the picker — the editor reads `/api/templates` for that —
/// but so the editor can tell that a module it cached is too old to render the
/// template the document asks for, and go back to the server instead of
/// showing the user a compile error it can do nothing about.
#[wasm_bindgen(js_name = templateIds)]
pub fn template_ids() -> String {
    let ids: Vec<&str> = rustycv_render::TEMPLATES.iter().map(|t| t.id).collect();
    serde_json::to_string(&ids).expect("a list of static strings serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/adham.json"
        ))
        .expect("the reference fixture is checked in")
    }

    /// The shim must not become a second render path: what it answers is what
    /// `render_pdf` answers, byte for byte.
    #[test]
    fn a_browser_render_is_the_same_bytes_as_a_server_render() {
        let json = fixture();
        let document: CvDocument = serde_json::from_str(&json).expect("the fixture parses");

        let direct = rustycv_render::render_pdf(&document).expect("the fixture renders");
        let through_shim = render_json(&json).expect("the fixture renders through the shim");

        assert_eq!(direct, through_shim);
    }

    #[test]
    fn an_unknown_template_comes_back_as_the_servers_error_body() {
        let mut document: serde_json::Value =
            serde_json::from_str(&fixture()).expect("the fixture parses");
        document["template"] = serde_json::json!("no-such-template");

        let body = render_json(&document.to_string()).expect_err("an unknown template fails");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("the body is JSON");

        assert_eq!(parsed["error"], "unknown template: no-such-template");
        // No diagnostics: nothing was compiled, so there is nothing to point at.
        assert!(parsed.get("diagnostics").is_none(), "{parsed}");
    }

    #[test]
    fn unparseable_json_fails_without_panicking() {
        let body = render_json("{ not json").expect_err("garbage does not render");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("the body is JSON");
        assert!(parsed["error"].is_string(), "{parsed}");
    }

    /// The editor decides whether to render locally by comparing the
    /// document's template against this list.
    #[test]
    fn template_ids_lists_every_built_in_template() {
        let ids: Vec<String> = serde_json::from_str(&template_ids()).expect("the id list is JSON");
        assert_eq!(ids.len(), rustycv_render::TEMPLATES.len());
        assert!(ids.contains(&"flowcv".to_string()), "{ids:?}");
    }
}
