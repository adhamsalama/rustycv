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
