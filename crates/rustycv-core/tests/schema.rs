use rustycv_core::{CvDocument, PageSize, SectionKind};

const FIXTURE: &str = include_str!("../../../fixtures/adham.json");

#[test]
fn fixture_round_trips() {
    let doc: CvDocument = serde_json::from_str(FIXTURE).expect("fixture should deserialize");

    let json = serde_json::to_string(&doc).unwrap();
    let again: CvDocument = serde_json::from_str(&json).unwrap();
    assert_eq!(
        doc, again,
        "document should survive a serialize/deserialize"
    );
}

#[test]
fn fixture_has_the_expected_shape() {
    let doc: CvDocument = serde_json::from_str(FIXTURE).unwrap();

    assert_eq!(doc.basics.full_name, "Adham Salama Mustafa");
    assert_eq!(doc.basics.links.len(), 3);
    assert_eq!(doc.theme.page, PageSize::A4);

    let kinds: Vec<_> = doc.sections.iter().map(|s| s.kind()).collect();
    assert_eq!(
        kinds,
        vec![
            SectionKind::Experience,
            SectionKind::Education,
            SectionKind::Skills,
            SectionKind::Projects,
            SectionKind::References,
        ]
    );

    // Three consecutive Bosta roles: the case that drives company grouping.
    let exp = &doc.sections[0];
    assert_eq!(exp.len(), 4);
}

#[test]
fn sparse_documents_deserialize() {
    // Everything optional. A document from a future/older version that omits
    // fields must still load rather than erroring the user out of their CV.
    let doc: CvDocument = serde_json::from_str(r#"{"basics":{"fullName":"A"}}"#).unwrap();
    assert_eq!(doc.basics.full_name, "A");
    assert_eq!(doc.template, "classic");
    assert!(doc.sections.is_empty());

    let doc: CvDocument = serde_json::from_str(
        r#"{"sections":[{"title":"Work","kind":"experience","items":[{"role":"Dev"}]}]}"#,
    )
    .unwrap();
    assert_eq!(doc.sections[0].len(), 1);
    assert!(doc.sections[0].visible, "visible should default to true");
}

#[test]
fn unknown_fields_are_ignored() {
    // Forward compatibility: a newer client adding a field must not break an
    // older server that is only reading and re-saving the document.
    let doc: CvDocument =
        serde_json::from_str(r#"{"basics":{"fullName":"A","nickname":"B"},"futureKey":1}"#)
            .unwrap();
    assert_eq!(doc.basics.full_name, "A");
}

#[test]
fn reassign_ids_makes_a_disjoint_copy() {
    let doc: CvDocument = serde_json::from_str(FIXTURE).unwrap();
    let mut copy = doc.clone();
    copy.reassign_ids();

    let original: Vec<_> = doc.sections.iter().map(|s| s.id).collect();
    for section in &copy.sections {
        assert!(!original.contains(&section.id), "section id was reused");
    }
    assert_eq!(copy.sections.len(), doc.sections.len());
}

#[test]
fn theme_clamps_hostile_input() {
    let mut doc = CvDocument::default();
    doc.theme.font_size_pt = 10_000.0;
    doc.theme.margin_mm = -5.0;
    doc.theme.accent = "javascript:alert(1)".into();

    let t = doc.theme.sanitized();
    assert_eq!(t.font_size_pt, 18.0);
    assert_eq!(t.margin_mm, 5.0);
    assert_eq!(t.accent, "#1f2937");
}

#[test]
fn empty_and_hidden_sections_are_not_rendered() {
    let mut doc = CvDocument::starter(); // three empty sections
    assert_eq!(doc.visible_sections().count(), 0, "empty sections skipped");

    doc.sections[0].body =
        serde_json::from_str(r#"{"kind":"experience","items":[{"role":"Dev"}]}"#).unwrap();
    assert_eq!(doc.visible_sections().count(), 1);

    doc.sections[0].visible = false;
    assert_eq!(doc.visible_sections().count(), 0, "hidden sections skipped");
}
