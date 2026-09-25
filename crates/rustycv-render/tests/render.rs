use rustycv_core::CvDocument;
use rustycv_render::{render_pdf, render_pngs, RenderError, TEMPLATES};
use serde_json::{json, Value};

const FIXTURE: &str = include_str!("../../../fixtures/adham.json");

fn fixture() -> CvDocument {
    serde_json::from_str(FIXTURE).expect("fixture deserializes")
}

/// Write renders out so they can be eyeballed: `cargo test -p rustycv-render`
/// then open `target/render-out/*.pdf`.
fn dump(name: &str, pdf: &[u8]) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/render-out");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{name}.pdf")), pdf).unwrap();
}

#[test]
fn renders_the_fixture_to_a_pdf() {
    let pdf = render_pdf(&fixture()).expect("fixture should render");
    assert!(pdf.starts_with(b"%PDF"), "output should be a PDF");
    assert!(pdf.len() > 10_000, "a full CV should be more than a stub");
    dump("adham", &pdf);
}

#[test]
fn every_template_renders_the_fixture() {
    for template in TEMPLATES {
        let mut doc = fixture();
        doc.template = template.id.to_string();
        let pdf = render_pdf(&doc)
            .unwrap_or_else(|e| panic!("template `{}` failed: {e:#?}", template.id));
        assert!(pdf.starts_with(b"%PDF"));
        dump(template.id, &pdf);
    }
}

#[test]
fn every_template_survives_an_empty_document() {
    // A brand new CV renders before the user has typed anything.
    for template in TEMPLATES {
        let mut doc = CvDocument::starter();
        doc.template = template.id.to_string();
        let pdf = render_pdf(&doc)
            .unwrap_or_else(|e| panic!("template `{}` failed on empty: {e:#?}", template.id));
        assert!(pdf.starts_with(b"%PDF"));
    }
}

#[test]
fn an_unknown_template_is_an_error_not_a_panic() {
    let mut doc = fixture();
    doc.template = "does-not-exist".into();
    assert!(matches!(
        render_pdf(&doc),
        Err(RenderError::UnknownTemplate(_))
    ));
}

#[test]
fn page_images_are_produced_for_every_page() {
    let pngs = render_pngs(&fixture(), 130.0).expect("fixture should rasterise");
    assert!(!pngs.is_empty());

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/render-out");
    std::fs::create_dir_all(&dir).unwrap();
    for (i, png) in pngs.iter().enumerate() {
        assert!(png.starts_with(b"\x89PNG"));
        std::fs::write(dir.join(format!("adham-page-{}.png", i + 1)), png).unwrap();
    }
    eprintln!("rendered {} page(s)", pngs.len());
}

/// A document containing one blank entry of every section kind.
///
/// This is the shape the editor produces the moment a user clicks "Add" on a
/// section and types nothing — every template has to survive it, including the
/// kinds the acceptance fixture never exercises.
fn one_blank_item_of_each_kind() -> CvDocument {
    let sections: Vec<Value> = [
        json!({"kind": "experience", "items": [{"role": "", "company": "", "bullets": [""]}]}),
        json!({"kind": "education", "items": [{"degree": "", "institution": ""}]}),
        json!({"kind": "skills", "groups": [{"name": "", "items": []}]}),
        json!({"kind": "projects", "items": [{"name": "", "description": "", "tech": []}]}),
        json!({"kind": "certifications", "items": [{"name": "", "issuer": ""}]}),
        json!({"kind": "languages", "items": [{"name": "", "level": ""}]}),
        json!({"kind": "interests", "items": [{"name": ""}]}),
        json!({"kind": "references", "items": [{"name": "", "title": ""}]}),
    ]
    .into_iter()
    .map(|mut body| {
        let object = body.as_object_mut().unwrap();
        object.insert("id".into(), json!(uuid::Uuid::new_v4().to_string()));
        object.insert("title".into(), json!("Section"));
        object.insert("visible".into(), json!(true));
        body
    })
    .collect();

    serde_json::from_value(json!({ "sections": sections })).expect("document builds")
}

#[test]
fn every_section_kind_renders_when_blank() {
    for template in TEMPLATES {
        let mut doc = one_blank_item_of_each_kind();
        doc.template = template.id.to_string();
        let pdf = render_pdf(&doc)
            .unwrap_or_else(|e| panic!("template `{}` failed on blank items: {e:#?}", template.id));
        assert!(pdf.starts_with(b"%PDF"));
    }
}

#[test]
fn every_section_kind_renders_when_populated() {
    // The acceptance fixture has no certifications, languages or interests, so
    // those template branches would otherwise never run in tests.
    let mut doc: CvDocument = serde_json::from_value(json!({
        "basics": {"fullName": "Test Person", "headline": "Engineer"},
        "sections": [
            {"id": "11111111-1111-4111-8111-111111111111", "title": "Certifications", "visible": true,
             "kind": "certifications",
             "items": [{"name": "CKA", "issuer": "CNCF", "url": "https://example.com",
                        "date": {"year": 2024, "month": 6}}]},
            {"id": "22222222-2222-4222-8222-222222222222", "title": "Languages", "visible": true,
             "kind": "languages",
             "items": [{"name": "Arabic", "level": "Native"}, {"name": "English", "level": "C2"}]},
            {"id": "33333333-3333-4333-8333-333333333333", "title": "Interests", "visible": true,
             "kind": "interests",
             "items": [{"name": "Distributed systems"}, {"name": "Climbing"}]}
        ]
    }))
    .unwrap();

    for template in TEMPLATES {
        doc.template = template.id.to_string();
        let pdf = render_pdf(&doc)
            .unwrap_or_else(|e| panic!("template `{}` failed: {e:#?}", template.id));
        assert!(pdf.starts_with(b"%PDF"));
        dump(&format!("{}-all-kinds", template.id), &pdf);
    }
}

#[test]
fn the_flowcv_template_reproduces_the_reference_resume_on_one_page() {
    // The acceptance fixture is the published FlowCV resume this template was
    // measured against. FlowCV fits it on a single A4 page; if a spacing change
    // pushes it onto a second, the reproduction has drifted.
    let doc = fixture();
    assert_eq!(
        doc.template, "flowcv",
        "the fixture is the flowcv reference"
    );

    let pages = render_pngs(&doc, 130.0).expect("reference resume renders");
    assert_eq!(pages.len(), 1, "the reference resume fits on one page");

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/render-out");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("flowcv-reference.png"), &pages[0]).unwrap();
}

/// Set `visible` on every entry of the named section, returning how many changed.
fn set_entries_visible(doc: &mut serde_json::Value, kind: &str, visible: bool) -> usize {
    let mut changed = 0;
    for section in doc["sections"].as_array_mut().unwrap() {
        if section["kind"] != kind {
            continue;
        }
        let key = if kind == "skills" { "groups" } else { "items" };
        for item in section[key].as_array_mut().unwrap() {
            item["visible"] = json!(visible);
            changed += 1;
        }
    }
    changed
}

fn render_page_one(value: &serde_json::Value) -> Vec<u8> {
    let doc: CvDocument = serde_json::from_value(value.clone()).expect("document parses");
    let pages = render_pngs(&doc, 96.0).expect("renders");
    pages.into_iter().next().expect("at least one page")
}

#[test]
fn hiding_an_entry_changes_the_output() {
    let mut hidden: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    hidden["sections"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["kind"] == "projects")
        .expect("fixture has projects")["items"][0]["visible"] = json!(false);

    let before = render_page_one(&serde_json::from_str(FIXTURE).unwrap());
    let after = render_page_one(&hidden);
    assert_ne!(
        before, after,
        "hiding a project should change what is rendered"
    );
}

#[test]
fn hiding_every_entry_is_equivalent_to_removing_the_section() {
    // The strongest statement of what "hidden" means: a section whose entries
    // are all hidden must leave no trace at all — not even its heading band or
    // the vertical space around it.
    let mut all_hidden: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    let count = set_entries_visible(&mut all_hidden, "projects", false);
    assert!(count > 1, "the fixture should have several projects");

    let mut removed: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    removed["sections"]
        .as_array_mut()
        .unwrap()
        .retain(|s| s["kind"] != "projects");

    assert_eq!(
        render_page_one(&all_hidden),
        render_page_one(&removed),
        "hiding every entry should render exactly like deleting the section",
    );
}

#[test]
fn hidden_entries_do_not_disturb_company_grouping() {
    // Bosta has three consecutive roles that render under one company heading.
    // Hiding the middle one must leave the other two still grouped, which is
    // only true if the filter runs before grouping rather than after.
    let mut hidden: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    hidden["sections"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["kind"] == "experience")
        .unwrap()["items"][1]["visible"] = json!(false);

    let mut removed: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    removed["sections"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["kind"] == "experience")
        .unwrap()["items"]
        .as_array_mut()
        .unwrap()
        .remove(1);

    assert_eq!(
        render_page_one(&hidden),
        render_page_one(&removed),
        "a hidden role should render exactly like an absent one",
    );
}

#[test]
fn every_template_handles_hidden_entries() {
    for template in TEMPLATES {
        let mut doc: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
        doc["template"] = json!(template.id);
        for kind in [
            "experience",
            "education",
            "skills",
            "projects",
            "references",
        ] {
            set_entries_visible(&mut doc, kind, false);
        }
        let parsed: CvDocument = serde_json::from_value(doc).unwrap();
        let pdf = render_pdf(&parsed).unwrap_or_else(|e| {
            panic!(
                "template `{}` failed with all entries hidden: {e:#?}",
                template.id
            )
        });
        assert!(pdf.starts_with(b"%PDF"));
    }
}

#[test]
fn standard_metrics_match_theme_default() {
    // Two sources of truth for the same numbers would drift silently: a user on
    // `classic` would reset and land somewhere other than a fresh CV's spacing.
    let theme = rustycv_core::Theme::default();
    let classic = rustycv_render::templates::get("classic").unwrap().metrics;

    assert_eq!(classic.font_size_pt, theme.font_size_pt);
    assert_eq!(classic.margin_mm, theme.margin_mm);
    assert_eq!(classic.line_height, theme.line_height);
    assert_eq!(classic.section_gap_mm, theme.section_gap_mm);
}

#[test]
fn every_template_declares_metrics_that_survive_sanitizing() {
    // Reset must land on a value the renderer will actually honour, otherwise
    // the sliders would jump somewhere else on the next render.
    for template in TEMPLATES {
        let m = template.metrics;
        let theme = rustycv_core::Theme {
            font_size_pt: m.font_size_pt,
            margin_mm: m.margin_mm,
            line_height: m.line_height,
            section_gap_mm: m.section_gap_mm,
            ..Default::default()
        };
        let clamped = theme.sanitized();
        assert_eq!(clamped.font_size_pt, m.font_size_pt, "{}", template.id);
        assert_eq!(clamped.margin_mm, m.margin_mm, "{}", template.id);
        assert_eq!(clamped.line_height, m.line_height, "{}", template.id);
        assert_eq!(clamped.section_gap_mm, m.section_gap_mm, "{}", template.id);
    }
}

#[test]
fn the_flowcv_reference_uses_the_templates_own_metrics() {
    // The fixture is the reproduction target, so its spacing and the template's
    // declared defaults have to agree — otherwise "reset" would move the
    // reference resume off the layout it was measured from.
    let doc = fixture();
    let m = rustycv_render::templates::get("flowcv").unwrap().metrics;

    assert_eq!(doc.theme.font_size_pt, m.font_size_pt);
    assert_eq!(doc.theme.margin_mm, m.margin_mm);
    assert_eq!(doc.theme.line_height, m.line_height);
    assert_eq!(doc.theme.section_gap_mm, m.section_gap_mm);
}

#[test]
fn resetting_spacing_restores_the_reference_layout() {
    // End-to-end statement of what the editor's reset control promises: whatever
    // the sliders were dragged to, applying the template's metrics gets the
    // original layout back exactly.
    let reference = fixture();
    let metrics = rustycv_render::templates::get(&reference.template)
        .unwrap()
        .metrics;

    let mut dragged = reference.clone();
    dragged.theme.font_size_pt = 13.0;
    dragged.theme.margin_mm = 28.0;
    dragged.theme.line_height = 1.4;
    dragged.theme.section_gap_mm = 11.0;
    assert_ne!(
        render_pdf(&dragged).unwrap(),
        render_pdf(&reference).unwrap(),
        "the dragged sliders should actually change the layout"
    );

    let mut reset = dragged;
    reset.theme.font_size_pt = metrics.font_size_pt;
    reset.theme.margin_mm = metrics.margin_mm;
    reset.theme.line_height = metrics.line_height;
    reset.theme.section_gap_mm = metrics.section_gap_mm;

    assert_eq!(
        render_pdf(&reset).unwrap(),
        render_pdf(&reference).unwrap(),
        "reset should reproduce the reference byte for byte"
    );
}

/// The longest vertical runs of dark pixels in the darkest left-hand column.
///
/// The employer hairline is the only thing drawn in its column — role text sits
/// an indent to its right — so this isolates the rule and reports how many
/// separate segments it was drawn in.
fn hairline_segments(png: &[u8], min_run: u32) -> Vec<u32> {
    let img = image::load_from_memory(png)
        .expect("page decodes")
        .to_luma8();
    let (width, height) = img.dimensions();

    // The rule is #cccccc (luma 204) and antialiasing lifts it further, so the
    // threshold has to sit above it rather than at a "looks like ink" level.
    let dark = |x: u32, y: u32| img.get_pixel(x, y).0[0] < 235;
    let mut best = (0u32, 0u32); // (column, longest run)
    for x in 0..width / 3 {
        let (mut run, mut longest) = (0u32, 0u32);
        for y in 0..height {
            run = if dark(x, y) { run + 1 } else { 0 };
            longest = longest.max(run);
        }
        if longest > best.1 {
            best = (x, longest);
        }
    }

    let mut segments = Vec::new();
    let mut run = 0u32;
    for y in 0..height {
        if dark(best.0, y) {
            run += 1;
        } else {
            if run >= min_run {
                segments.push(run);
            }
            run = 0;
        }
    }
    if run >= min_run {
        segments.push(run);
    }
    segments
}

#[test]
fn the_employer_hairline_is_one_unbroken_line() {
    // FlowCV draws a single rule down a whole run of roles at one employer.
    // Wrapping each role in its own bordered block instead looks almost right
    // but leaves a visible gap at every role boundary, which is easy to
    // reintroduce and impossible to notice in a diff.
    // Rendered larger than the default so a 1pt rule lands on whole pixels.
    let doc: CvDocument = serde_json::from_str(FIXTURE).unwrap();
    let page = render_pngs(&doc, 200.0).unwrap().remove(0);
    let segments = hairline_segments(&page, 40);

    assert_eq!(
        segments.len(),
        1,
        "the hairline should be drawn as one segment, got {segments:?}"
    );
}
