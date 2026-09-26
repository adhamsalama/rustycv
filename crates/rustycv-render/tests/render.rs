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

/// Apply the work-experience switches to the fixture.
fn with_experience_options(order: &str, group: bool) -> CvDocument {
    let mut doc: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    for section in doc["sections"].as_array_mut().unwrap() {
        if section["kind"] == "experience" {
            section["order"] = json!(order);
            section["groupPromotions"] = json!(group);
        }
    }
    serde_json::from_value(doc).unwrap()
}

#[test]
fn the_defaults_match_the_reference_resume() {
    // Role-first with promotions grouped is what the reference was measured
    // from, so an omitted switch must land there or existing CVs would shift.
    let explicit = render_pdf(&with_experience_options("roleFirst", true)).unwrap();
    assert_eq!(
        explicit,
        render_pdf(&fixture()).unwrap(),
        "the fixture omits both switches and must render as if they were default"
    );
}

#[test]
fn entry_order_swaps_role_and_company() {
    let role_first = render_pdf(&with_experience_options("roleFirst", true)).unwrap();
    let company_first = render_pdf(&with_experience_options("companyFirst", true)).unwrap();
    assert_ne!(
        role_first, company_first,
        "switching the title/subtitle order should change the output"
    );
}

#[test]
fn turning_off_group_promotions_removes_the_employer_hairline() {
    // Ungrouped, every role stands alone as "Role, Company" — so the rule that
    // brackets a run of roles at one employer should not be drawn at all.
    let doc = with_experience_options("roleFirst", false);
    let page = render_pngs(&doc, 200.0).unwrap().remove(0);
    assert!(
        hairline_segments(&page, 40).is_empty(),
        "no roles are grouped, so there should be no hairline"
    );

    let grouped = render_pngs(&with_experience_options("roleFirst", true), 200.0)
        .unwrap()
        .remove(0);
    assert_eq!(hairline_segments(&grouped, 40).len(), 1);
}

#[test]
fn every_template_honours_the_experience_switches() {
    // The switches are document data, not flowcv decoration, so every template
    // has to respect them.
    for template in TEMPLATES {
        let mut a = with_experience_options("roleFirst", true);
        let mut b = with_experience_options("companyFirst", false);
        a.template = template.id.to_string();
        b.template = template.id.to_string();

        let rendered_a = render_pdf(&a).unwrap_or_else(|e| panic!("{}: {e:#?}", template.id));
        let rendered_b = render_pdf(&b).unwrap_or_else(|e| panic!("{}: {e:#?}", template.id));
        assert_ne!(
            rendered_a, rendered_b,
            "template `{}` ignores the work-experience switches",
            template.id
        );
    }
}

// ----------------------------------------------------------- rich text

/// Replace the first Bosta bullet with the given rich-text JSON.
fn fixture_with_first_bullet(bullet: serde_json::Value) -> CvDocument {
    let mut doc: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    for section in doc["sections"].as_array_mut().unwrap() {
        if section["kind"] == "experience" {
            section["items"][0]["bullets"] = json!([bullet]);
        }
    }
    serde_json::from_value(doc).unwrap()
}

#[test]
fn marks_change_what_is_drawn() {
    // Same characters, different weight — so any difference in the rendered
    // pixels can only come from the marks being applied.
    let plain = fixture_with_first_bullet(json!([{ "text": "Improved accuracy to 98.5%" }]));
    let bold = fixture_with_first_bullet(json!([
        { "text": "Improved accuracy to " },
        { "text": "98.5%", "bold": true }
    ]));

    assert_ne!(
        render_pngs(&plain, 150.0).unwrap(),
        render_pngs(&bold, 150.0).unwrap(),
        "a bold run should render differently from the same text unstyled"
    );
}

/// A short digest, so a failed comparison prints something readable instead of
/// a megabyte of PNG.
fn digest(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn each_visible_mark_renders_distinctly() {
    // Guards against a mark being parsed but silently dropped. `link` is not
    // here on purpose: the fixture's accent is black, so a link is visually
    // identical to plain text and is checked through the PDF instead.
    let text = "Improved accuracy";
    let mut seen: Vec<(&str, u64)> = Vec::new();

    for mark in ["plain", "bold", "italic", "underline"] {
        let mut run = json!({ "text": text });
        if mark != "plain" {
            run[mark] = json!(true);
        }
        let page = render_pngs(&fixture_with_first_bullet(json!([run])), 150.0).unwrap();
        let hash = digest(&page.concat());

        for (other, previous) in &seen {
            assert_ne!(
                hash, *previous,
                "`{mark}` renders identically to `{other}` — the mark is being dropped"
            );
        }
        seen.push((mark, hash));
    }
}

#[test]
fn a_link_run_reaches_the_pdf() {
    // A link is an annotation rather than a pixel change — with a black accent
    // it looks exactly like plain text — so this has to inspect the PDF.
    let plain = render_pdf(&fixture_with_first_bullet(json!([{ "text": "Docs" }]))).unwrap();
    let linked = render_pdf(&fixture_with_first_bullet(
        json!([{ "text": "Docs", "link": "https://example.com/handbook" }]),
    ))
    .unwrap();

    assert_ne!(
        digest(&plain),
        digest(&linked),
        "a link run should add an annotation to the PDF"
    );
    assert!(
        linked.len() > plain.len(),
        "the annotation and its URI should make the PDF larger"
    );
}

#[test]
fn a_bare_string_bullet_renders_the_same_as_one_unstyled_run() {
    // The compact wire form and the explicit one must be indistinguishable,
    // otherwise saving a document would shift the layout of existing CVs.
    let compact = fixture_with_first_bullet(json!("Improved accuracy to 98.5%"));
    let explicit = fixture_with_first_bullet(json!([{ "text": "Improved accuracy to 98.5%" }]));

    assert_eq!(
        render_pdf(&compact).unwrap(),
        render_pdf(&explicit).unwrap(),
        "the two wire forms of the same text must render identically"
    );
}

#[test]
fn rich_text_is_never_interpreted_as_typst_markup() {
    // Run text reaches Typst as a string, which is inserted literally. If it
    // were ever spliced into markup instead, this would emit a heading, a bold
    // span and a broken function call rather than the characters typed.
    let hostile = "= not a heading *not bold* #panic() $x^2$";
    let doc = fixture_with_first_bullet(json!([{ "text": hostile }]));

    let pdf = render_pdf(&doc).expect("hostile text must not break the render");
    assert!(pdf.starts_with(b"%PDF"));

    // It must also look like the same literal characters however it was stored.
    assert_eq!(
        render_pdf(&doc).unwrap(),
        render_pdf(&fixture_with_first_bullet(json!(hostile))).unwrap(),
    );
}

#[test]
fn every_template_renders_marks() {
    for template in TEMPLATES {
        let mut doc = fixture_with_first_bullet(json!([
            { "text": "Improved accuracy to " },
            { "text": "98.5%", "bold": true, "italic": true },
            { "text": " overall", "underline": true }
        ]));
        doc.template = template.id.to_string();
        let pdf = render_pdf(&doc).unwrap_or_else(|e| panic!("{}: {e:#?}", template.id));
        assert!(pdf.starts_with(b"%PDF"));
    }
}

// ------------------------------------------------- line height / rhythm

/// Distance from the first inked row to the last, i.e. how much vertical space
/// the content actually occupies.
fn content_height(png: &[u8]) -> u32 {
    let img = image::load_from_memory(png)
        .expect("page decodes")
        .to_luma8();
    let (width, height) = img.dimensions();
    // Strict enough to see only text, not the heading band's light tint.
    let inked = |y: u32| (0..width).any(|x| img.get_pixel(x, y).0[0] < 128);

    let first = (0..height).find(|&y| inked(y));
    let last = (0..height).rev().find(|&y| inked(y));
    match (first, last) {
        (Some(a), Some(b)) => b - a,
        _ => 0,
    }
}

/// A document of short, single-line bullets — nothing wraps, so the only
/// vertical space between them is the list's item spacing.
fn short_bullets_at(line_height: f32) -> CvDocument {
    serde_json::from_value(json!({
        "template": "flowcv",
        "theme": { "lineHeight": line_height, "accent": "#000000", "fontFamily": "Source Sans 3" },
        "basics": { "fullName": "T" },
        "sections": [{
            "id": "11111111-1111-4111-8111-111111111111",
            "title": "Work", "visible": true, "kind": "experience",
            "items": [{ "role": "Dev", "bullets": ["one", "two", "three", "four", "five"] }]
        }]
    }))
    .unwrap()
}

#[test]
fn line_height_spaces_bullets_apart_not_just_wrapped_lines() {
    // The bug this guards: leading scaled with the line-height control but the
    // list's item spacing was a fixed length. A bullet's own wrapped lines flew
    // apart while the gap to the next bullet stayed put — and a document of
    // single-line bullets, like this one, did not move at all.
    let tight = content_height(&render_pngs(&short_bullets_at(1.0), 150.0).unwrap()[0]);
    let loose = content_height(&render_pngs(&short_bullets_at(1.5), 150.0).unwrap()[0]);

    assert!(
        loose > tight,
        "raising line height must space single-line bullets apart \
         (tight={tight}px, loose={loose}px)"
    );
}

/// The top edge of each band of inked rows, i.e. where every text line starts.
///
/// Measuring line positions rather than total ink height keeps descenders out
/// of it — "Second bullet" has none and a wrapping sentence does, which is
/// enough to shift a total-height comparison by several pixels.
fn text_line_starts(png: &[u8]) -> Vec<u32> {
    let img = image::load_from_memory(png)
        .expect("page decodes")
        .to_luma8();
    let (width, height) = img.dimensions();
    let inked = |y: u32| (0..width).any(|x| img.get_pixel(x, y).0[0] < 128);

    let mut starts = Vec::new();
    let mut inside = false;
    for y in 0..height {
        if inked(y) {
            if !inside {
                starts.push(y);
            }
            inside = true;
        } else {
            inside = false;
        }
    }
    starts
}

#[test]
fn a_bullet_boundary_and_a_line_wrap_advance_by_the_same_step() {
    // One render, three list lines: a bullet that wraps onto a second line,
    // then a second bullet. The step from line one to line two crosses a wrap;
    // the step from line two to line three crosses a bullet boundary. If item
    // spacing and line leading share a rhythm the two steps match — when item
    // spacing was a fixed length they drifted apart as line height rose.
    let doc: CvDocument = serde_json::from_value(json!({
        "template": "flowcv",
        "theme": { "lineHeight": 1.5, "accent": "#000000", "fontFamily": "Source Sans 3" },
        "basics": {},
        "sections": [{
            "id": "11111111-1111-4111-8111-111111111111",
            "title": "W", "visible": true, "kind": "experience",
            "items": [{ "bullets": [
                "A single bullet long enough that it certainly wraps onto a second line when \
                 set across the full width of an A4 page at nine point, and no further.",
                "Second bullet"
            ]}]
        }]
    }))
    .unwrap();

    let starts = text_line_starts(&render_pngs(&doc, 150.0).unwrap()[0]);
    let list = &starts[starts.len() - 3..];
    let across_wrap = (list[1] - list[0]) as i64;
    let across_boundary = (list[2] - list[1]) as i64;

    assert!(
        (across_wrap - across_boundary).abs() <= 2,
        "a wrap advances {across_wrap}px but a bullet boundary advances \
         {across_boundary}px — item spacing and leading have drifted apart"
    );
}

#[test]
fn an_entry_heading_sits_one_line_step_above_its_first_bullet() {
    // The gap under a role title was its own fixed length, so the first bullet
    // stayed clamped to the title while every bullet below it spread out as
    // line height rose — and at the default it was already tighter than the
    // bullet step. Every step through an entry should be the same, at any line
    // height.
    for line_height in [1.0f32, 1.5] {
        let doc: CvDocument = serde_json::from_value(json!({
            "template": "flowcv",
            "theme": {
                "lineHeight": line_height,
                "accent": "#000000",
                "fontFamily": "Source Sans 3"
            },
            "basics": {},
            "sections": [{
                "id": "11111111-1111-4111-8111-111111111111",
                "title": "W", "visible": true, "kind": "experience",
                "items": [{
                    "role": "Staff Backend Engineer",
                    "bullets": ["First bullet", "Second bullet", "Third bullet"]
                }]
            }]
        }))
        .unwrap();

        let starts = text_line_starts(&render_pngs(&doc, 150.0).unwrap()[0]);
        let tail = &starts[starts.len() - 4..];
        let steps: Vec<i64> = tail.windows(2).map(|w| (w[1] - w[0]) as i64).collect();
        let (min, max) = (*steps.iter().min().unwrap(), *steps.iter().max().unwrap());

        assert!(
            max - min <= 2,
            "at line height {line_height} the steps through an entry were {steps:?} \
             — heading to first bullet should match bullet to bullet"
        );
    }
}

// --------------------------------------------------- banner header band

/// The darkest and lightest pixel inside `banner`'s header band.
///
/// The band is the run of rows near the top that are inked right across the
/// page. Sampling only the middle of them keeps the rounded corners and the
/// white page margins out of the measurement, so what comes back is the fill
/// and the text sitting on it, nothing else.
fn band_extremes(accent: &str) -> (u8, u8) {
    let mut doc: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    doc["template"] = json!("banner");
    doc["theme"]["accent"] = json!(accent);
    let png = render_page_one(&doc);
    let img = image::load_from_memory(&png)
        .expect("page decodes")
        .to_luma8();
    let (width, height) = img.dimensions();
    let (x0, x1) = (width * 15 / 100, width * 85 / 100);

    // "Mostly covered" rather than "entirely covered": the band's own text can
    // be pure white, and demanding every pixel be non-white would drop exactly
    // the rows this is trying to look at.
    let covered = |y: u32| {
        let filled = (x0..x1)
            .filter(|&x| img.get_pixel(x, y).0[0] != 255)
            .count();
        filled * 10 >= (x1 - x0) as usize * 9
    };
    let rows: Vec<u32> = (0..height / 4).filter(|&y| covered(y)).collect();
    assert!(
        rows.len() > 20,
        "expected a solid header band near the top, found {} row(s)",
        rows.len()
    );
    // The band's first and last rows are antialiased against the white page, so
    // they read far lighter than the fill. Leaving them in would let a band
    // with no light ink on it at all still report a light pixel.
    let rows = &rows[2..rows.len() - 2];

    let (mut min, mut max) = (255u8, 0u8);
    for &y in rows {
        for x in x0..x1 {
            let v = img.get_pixel(x, y).0[0];
            min = min.min(v);
            max = max.max(v);
        }
    }
    (min, max)
}

#[test]
fn the_banner_band_picks_ink_that_survives_the_accent() {
    // `banner` is the one template that puts text *on* the accent, so the
    // accent it is given decides whether that text is readable at all. A fixed
    // white would vanish the moment someone picked a yellow.
    let (fill, ink) = band_extremes("#1f2937");
    assert!(
        fill < 100 && ink > 240,
        "a dark accent should carry near-white text: darkest {fill}, lightest {ink}"
    );

    let (ink, fill) = band_extremes("#f5c518");
    assert!(
        ink < 60 && fill > 150,
        "a light accent should carry near-black text: darkest {ink}, lightest {fill}"
    );
}

// --------------------------------------------------------- theme audit

/// Render the fixture on `template` with one theme field overridden.
fn with_theme(template: &str, field: &str, value: serde_json::Value) -> Vec<u8> {
    let mut doc: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    doc["template"] = json!(template);
    doc["theme"][field] = value;
    render_pdf(&serde_json::from_value(doc).unwrap())
        .unwrap_or_else(|e| panic!("{template} failed with {field}: {e:#?}"))
}

#[test]
fn every_theme_control_does_something_on_every_template() {
    // A control wired to nothing is invisible in review — the section-gap
    // slider did nothing at all on `flowcv` for several commits, because that
    // template spaced its headings with a hardcoded length instead.
    //
    // Values are chosen inside `Theme::sanitized`'s clamps so a no-op here
    // means the template ignores the field, not that the value was rejected.
    let knobs: &[(&str, serde_json::Value, serde_json::Value)] = &[
        ("fontSizePt", json!(8.0), json!(12.0)),
        ("lineHeight", json!(0.9), json!(1.5)),
        ("marginMm", json!(10.0), json!(25.0)),
        ("sectionGapMm", json!(1.0), json!(12.0)),
        ("accent", json!("#000000"), json!("#b42318")),
        ("fontFamily", json!("Source Sans 3"), json!("IBM Plex Sans")),
        ("page", json!("a4"), json!("letter")),
        ("headingStyle", json!("bold"), json!("caps")),
        ("bulletStyle", json!("dot"), json!("dash")),
    ];

    let mut dead = Vec::new();
    for template in TEMPLATES {
        for (field, low, high) in knobs {
            if with_theme(template.id, field, low.clone())
                == with_theme(template.id, field, high.clone())
            {
                dead.push(format!("{}.{field}", template.id));
            }
        }
    }

    assert!(dead.is_empty(), "theme controls with no effect: {dead:?}");
}

#[test]
fn spacing_scales_with_type_size() {
    // Gaps written in absolute pt or mm stay put while the glyphs around them
    // grow, so a CV set larger gets relatively tighter.
    //
    // Every line here is short enough never to wrap, which matters: changing
    // the type size changes where text wraps, and a document that reflows
    // would change height for reasons that have nothing to do with spacing.
    // The section gap is held at zero because it is deliberately a physical
    // measurement and is meant not to scale.
    for template in TEMPLATES {
        let height_at = |size: f64| {
            let doc: CvDocument = serde_json::from_value(json!({
                "template": template.id,
                "theme": {
                    "fontSizePt": size,
                    "sectionGapMm": 0.0,
                    "accent": "#000000",
                    "fontFamily": "Source Sans 3"
                },
                "basics": { "fullName": "Name", "headline": "Role" },
                "sections": [
                    {
                        "id": "11111111-1111-4111-8111-111111111111",
                        "title": "Work", "visible": true, "kind": "experience",
                        "items": [
                            { "role": "Alpha", "company": "Acme", "bullets": ["Alpha", "Alpha"] },
                            { "role": "Beta", "company": "Acme", "bullets": ["Alpha"] },
                            { "role": "Gamma", "company": "Other", "bullets": ["Alpha"] }
                        ]
                    },
                    {
                        "id": "22222222-2222-4222-8222-222222222222",
                        "title": "Education", "visible": true, "kind": "education",
                        // Blocks as well as a plain string: the gaps between
                        // paragraphs and list items have to scale too.
                        "items": [{ "degree": "Alpha", "institution": "Acme",
                                    "description": [
                                      { "kind": "paragraph", "runs": [{ "text": "Alpha" }] },
                                      { "kind": "bullet", "runs": [{ "text": "Alpha" }] },
                                      { "kind": "bullet", "runs": [{ "text": "Alpha" }] }] }]
                    },
                    {
                        "id": "33333333-3333-4333-8333-333333333333",
                        "title": "Skills", "visible": true, "kind": "skills",
                        "groups": [{ "name": "Languages", "items": ["Rust"] }]
                    },
                    {
                        "id": "44444444-4444-4444-8444-444444444444",
                        "title": "Projects", "visible": true, "kind": "projects",
                        "items": [{ "name": "Alpha", "description": "Beta" },
                                  { "name": "Gamma", "description": "Delta" }]
                    },
                    {
                        "id": "55555555-5555-4555-8555-555555555555",
                        "title": "Certifications", "visible": true, "kind": "certifications",
                        "items": [{ "name": "Alpha", "issuer": "Acme" }]
                    },
                    {
                        "id": "66666666-6666-4666-8666-666666666666",
                        "title": "References", "visible": true, "kind": "references",
                        "items": [{ "name": "Alpha", "title": "Beta", "company": "Acme" }]
                    }
                ]
            }))
            .unwrap();
            content_height(&render_pngs(&doc, 200.0).unwrap()[0]) as f64
        };

        let grew = height_at(12.0) / height_at(8.0);
        let expected = 12.0 / 8.0;
        let error = (grew / expected - 1.0).abs();

        assert!(
            error < 0.01,
            "{}: type size went up {expected:.2}x but the content grew {grew:.3}x \
             — some spacing is in absolute units rather than em",
            template.id
        );
    }
}

// ------------------------------------------------------- blocks in rich text

/// A document whose only content is a summary, so every text line below the
/// name belongs to the value under test.
fn doc_with_summary(template: &str, summary: Value) -> CvDocument {
    serde_json::from_value(json!({
        "template": template,
        "theme": { "accent": "#000000", "fontFamily": "Source Sans 3" },
        "basics": { "fullName": "N", "summary": summary },
        "sections": []
    }))
    .expect("document builds")
}

/// Every text line's top row paired with its first and last inked column.
///
/// Lines rather than total ink: a marker or a descender moves the overall
/// height for reasons that have nothing to do with the layout under test.
fn line_extents(png: &[u8]) -> Vec<(u32, u32, u32)> {
    let img = image::load_from_memory(png)
        .expect("page decodes")
        .to_luma8();
    let (width, height) = img.dimensions();
    let inked = |x: u32, y: u32| img.get_pixel(x, y).0[0] < 128;

    let mut lines: Vec<(u32, u32, u32)> = Vec::new();
    let mut inside = false;
    for y in 0..height {
        let columns: Vec<u32> = (0..width).filter(|&x| inked(x, y)).collect();
        match (columns.first(), columns.last()) {
            (Some(&left), Some(&right)) => {
                if inside {
                    let last = lines.last_mut().unwrap();
                    last.1 = last.1.min(left);
                    last.2 = last.2.max(right);
                } else {
                    lines.push((y, left, right));
                }
                inside = true;
            }
            _ => inside = false,
        }
    }
    lines
}

#[test]
fn a_soft_line_break_starts_a_new_line() {
    // Shift+Enter is stored as a newline inside a run. Typst treats a lone
    // newline in markup as ordinary whitespace, so without an explicit
    // linebreak the two halves would run together on one line.
    let one_line = render_pngs(&doc_with_summary("classic", json!("Alpha Beta")), 200.0).unwrap();
    let two_lines = render_pngs(&doc_with_summary("classic", json!("Alpha\nBeta")), 200.0).unwrap();

    assert_eq!(text_line_starts(&one_line[0]).len(), 2, "name + summary");
    assert_eq!(
        text_line_starts(&two_lines[0]).len(),
        3,
        "name + both halves of the summary"
    );
}

#[test]
fn a_paragraph_break_is_a_wider_step_than_a_line_break() {
    let step_between_summary_lines = |summary: Value| {
        let page = render_pngs(&doc_with_summary("classic", summary), 200.0).unwrap();
        let starts = text_line_starts(&page[0]);
        assert_eq!(starts.len(), 3, "name plus two lines of summary");
        starts[2] - starts[1]
    };

    let soft = step_between_summary_lines(json!("Alpha\nBeta"));
    let paragraphs = step_between_summary_lines(json!([
        { "kind": "paragraph", "runs": [{ "text": "Alpha" }] },
        { "kind": "paragraph", "runs": [{ "text": "Beta" }] }
    ]));

    // Two line steps rather than one, so the break cannot be mistaken for a
    // wrapped line. This failed at first: a gap of exactly one step renders
    // identically to a line break.
    assert!(
        paragraphs > soft + soft / 3,
        "a paragraph break ({paragraphs}px) should be clearly wider than a line break ({soft}px)"
    );
}

#[test]
fn a_paragraph_break_scales_with_the_line_height_control() {
    // The gap is the caller's line step, so the line-height slider has to move
    // it. A gap hardcoded in em would hold still while the text spread.
    let step_at = |line_height: f32| {
        let mut doc = doc_with_summary(
            "classic",
            json!([
                { "kind": "paragraph", "runs": [{ "text": "Alpha" }] },
                { "kind": "paragraph", "runs": [{ "text": "Beta" }] }
            ]),
        );
        doc.theme.line_height = line_height;
        let page = render_pngs(&doc, 200.0).unwrap();
        let starts = text_line_starts(&page[0]);
        starts[2] - starts[1]
    };

    let tight = step_at(1.0);
    let loose = step_at(1.6);
    assert!(
        loose > tight + 4,
        "raising line height should widen the paragraph gap: {tight}px then {loose}px"
    );
}

#[test]
fn a_list_in_a_description_sets_its_text_behind_a_marker() {
    let paragraph = render_pngs(&doc_with_summary("classic", json!("Alpha")), 200.0).unwrap();
    let bulleted = render_pngs(
        &doc_with_summary(
            "classic",
            json!([{ "kind": "bullet", "runs": [{ "text": "Alpha" }] }]),
        ),
        200.0,
    )
    .unwrap();

    let (_, plain_left, plain_right) = line_extents(&paragraph[0])[1];
    let (_, bullet_left, bullet_right) = line_extents(&bulleted[0])[1];

    // The marker takes the paragraph's place at the left margin and the word
    // moves right behind it, so the line ends further right than it did.
    assert!(
        bullet_left <= plain_left + 2,
        "the marker should sit at the margin: {bullet_left} vs {plain_left}"
    );
    assert!(
        bullet_right > plain_right + 4,
        "the item's text should be indented behind its marker: {bullet_right} vs {plain_right}"
    );
}

/// The bounding box of a list marker: the first run of inked columns on line
/// `line`, which is the marker alone — the item's body starts after the gap the
/// body indent leaves, so nothing else can join that run.
fn marker_box(png: &[u8], line: usize) -> (u32, u32) {
    let img = image::load_from_memory(png)
        .expect("page decodes")
        .to_luma8();
    let (width, height) = img.dimensions();
    // Looser than the text threshold: an en dash is one thin stroke and its
    // antialiased edges carry most of it.
    let inked = |x: u32, y: u32| img.get_pixel(x, y).0[0] < 200;

    // The rows this line of text occupies.
    let mut bands: Vec<(u32, u32)> = Vec::new();
    for y in 0..height {
        if (0..width).any(|x| inked(x, y)) {
            match bands.last_mut() {
                Some(b) if b.1 == y => b.1 = y + 1,
                _ => bands.push((y, y + 1)),
            }
        }
    }
    let (top, bottom) = bands[line];

    let column = |x: u32| (top..bottom).any(|y| inked(x, y));
    let left = (0..width).find(|&x| column(x)).expect("the line has ink");
    let right = (left..width).take_while(|&x| column(x)).last().unwrap();
    let rows = (top..bottom)
        .filter(|&y| (left..=right).any(|x| inked(x, y)))
        .count() as u32;
    (right - left + 1, rows)
}

#[test]
fn the_dash_bullet_is_a_dash_and_the_dot_is_round() {
    // The audit only proves the control changes *something*. This pins what:
    // an en dash is a wide, flat stroke where the default marker is as tall as
    // it is wide, so swapping one glyph for another would not pass here.
    let render = |bullet_style: &str| {
        let mut doc = doc_with_summary(
            "classic",
            json!([{ "kind": "bullet", "runs": [{ "text": "Alpha" }] }]),
        );
        doc.theme.bullet_style = serde_json::from_value(json!(bullet_style)).unwrap();
        render_pngs(&doc, 300.0).unwrap()
    };

    let (dot_w, dot_h) = marker_box(&render("dot")[0], 1);
    let (dash_w, dash_h) = marker_box(&render("dash")[0], 1);

    assert!(
        dot_w < dot_h * 2,
        "the default marker should be round: {dot_w}x{dot_h}"
    );
    assert!(
        dash_w > dash_h * 2,
        "a dash should be far wider than it is tall: {dash_w}x{dash_h}"
    );
}

#[test]
fn a_numbered_list_is_not_drawn_as_a_bulleted_one() {
    let render = |kind: &str| {
        render_pngs(
            &doc_with_summary(
                "classic",
                json!([
                    { "kind": kind, "runs": [{ "text": "Alpha" }] },
                    { "kind": kind, "runs": [{ "text": "Beta" }] }
                ]),
            ),
            200.0,
        )
        .unwrap()
    };

    assert_ne!(
        digest(&render("bullet").concat()),
        digest(&render("numbered").concat()),
        "a numbered list should carry different markers from a bulleted one"
    );
}

#[test]
fn consecutive_items_share_one_list() {
    // Each item is its own block, so nothing stops them being rendered as a
    // stack of one-item lists — which spaces them apart like paragraphs
    // instead of like a list.
    let items = |n: usize| {
        let blocks: Vec<Value> = (0..n)
            .map(|_| json!({ "kind": "bullet", "runs": [{ "text": "Alpha" }] }))
            .collect();
        let page = render_pngs(&doc_with_summary("classic", json!(blocks)), 200.0).unwrap();
        text_line_starts(&page[0])
    };

    let three = items(3);
    assert_eq!(three.len(), 4, "name plus three items");
    let first_gap = three[2] - three[1];
    let second_gap = three[3] - three[2];
    assert_eq!(
        first_gap, second_gap,
        "items should be evenly spaced, not split across separate lists"
    );

    // And that spacing is the list's own, not the wider paragraph break.
    let paragraphs = render_pngs(
        &doc_with_summary(
            "classic",
            json!([
                { "kind": "paragraph", "runs": [{ "text": "Alpha" }] },
                { "kind": "paragraph", "runs": [{ "text": "Alpha" }] }
            ]),
        ),
        200.0,
    )
    .unwrap();
    let starts = text_line_starts(&paragraphs[0]);
    assert!(
        first_gap < starts[2] - starts[1],
        "list items should sit closer together than paragraphs"
    );
}

#[test]
fn a_project_description_drops_below_the_name_when_it_cannot_share_the_line() {
    let project = |description: Value| {
        let doc: CvDocument = serde_json::from_value(json!({
            "template": "classic",
            "theme": { "accent": "#000000", "fontFamily": "Source Sans 3" },
            "basics": { "fullName": "N" },
            "sections": [{
                "id": "11111111-1111-4111-8111-111111111111",
                "title": "Projects", "visible": true, "kind": "projects",
                "items": [{ "name": "Atlas", "description": description, "tech": ["Rust"] }]
            }]
        }))
        .unwrap();
        let page = render_pngs(&doc, 200.0).unwrap();
        text_line_starts(&page[0]).len()
    };

    // Name, section heading, then the entry itself.
    let inline = project(json!("A renderer"));
    assert_eq!(inline, 3, "a one-line description stays on the name line");

    let blocks = project(json!([
        { "kind": "bullet", "runs": [{ "text": "Renders fast" }] },
        { "kind": "bullet", "runs": [{ "text": "One binary" }] }
    ]));
    assert_eq!(
        blocks, 5,
        "a list needs two lines of its own below the name"
    );
}

#[test]
fn highlights_take_the_same_blocks_a_description_does() {
    // Highlights were once one value per bullet; they are now a rich value like
    // any other, so a numbered list or a paragraph renders here too.
    let entry = |bullets: Value| {
        let doc: CvDocument = serde_json::from_value(json!({
            "template": "classic",
            "theme": { "accent": "#000000", "fontFamily": "Source Sans 3" },
            "basics": { "fullName": "N" },
            "sections": [{
                "id": "11111111-1111-4111-8111-111111111111",
                "title": "Work", "visible": true, "kind": "experience",
                "items": [{ "role": "Dev", "company": "Acme", "bullets": bullets }]
            }]
        }))
        .unwrap();
        render_pngs(&doc, 200.0).unwrap()
    };

    // The older per-bullet form and the block form are the same document.
    assert_eq!(
        digest(&entry(json!(["Alpha", "Beta"])).concat()),
        digest(
            &entry(json!([
                { "kind": "bullet", "runs": [{ "text": "Alpha" }] },
                { "kind": "bullet", "runs": [{ "text": "Beta" }] }
            ]))
            .concat()
        ),
        "highlights written one value per bullet must render as bullet blocks"
    );

    let numbered = entry(json!([
        { "kind": "numbered", "runs": [{ "text": "Alpha" }] },
        { "kind": "numbered", "runs": [{ "text": "Beta" }] }
    ]));
    assert_ne!(
        digest(&numbered.concat()),
        digest(&entry(json!(["Alpha", "Beta"])).concat()),
        "a numbered highlight list should not render as bullets"
    );

    // Name, section heading, entry heading, then the two lines themselves.
    assert_eq!(text_line_starts(&numbered[0]).len(), 5);
}

#[test]
fn every_template_renders_paragraphs_and_both_kinds_of_list() {
    // flowcv carries its own entry geometry, so it renders this through a
    // different path from the five that share `common.typ`.
    let summary = json!([
        { "kind": "paragraph", "runs": [{ "text": "Led the rewrite." }] },
        { "kind": "bullet", "runs": [{ "text": "Cut p99 latency." }] },
        { "kind": "bullet", "runs": [{ "text": "Shipped retries." }] },
        { "kind": "numbered", "runs": [{ "text": "Then the migration." }] }
    ]);

    for template in TEMPLATES {
        let lines = |summary: Value| {
            let doc = doc_with_summary(template.id, summary);
            let page = &render_pngs(&doc, 150.0)
                .unwrap_or_else(|e| panic!("template `{}` failed on blocks: {e:#?}", template.id))
                [0];
            text_line_starts(page).len()
        };
        // Against the same page with no summary at all, because a template's
        // own chrome — modern's rule, banner's block — inks rows of its own.
        assert_eq!(
            lines(summary.clone()) as i64 - lines(json!("")) as i64,
            4,
            "template `{}` should draw all four blocks",
            template.id
        );
    }
}
