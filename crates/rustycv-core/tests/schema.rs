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

#[test]
fn entries_are_visible_unless_said_otherwise() {
    // Documents written before `visible` existed must load as visible, or
    // upgrading the app would silently blank out everyone's CV.
    let doc: CvDocument = serde_json::from_str(
        r#"{"sections":[{"title":"Projects","kind":"projects",
             "items":[{"name":"A"},{"name":"B","visible":false}]}]}"#,
    )
    .unwrap();

    assert_eq!(doc.sections[0].len(), 2, "both entries are kept");
    assert_eq!(doc.sections[0].visible_len(), 1, "only one would render");
}

#[test]
fn a_section_whose_entries_are_all_hidden_counts_as_empty() {
    let doc: CvDocument = serde_json::from_str(
        r#"{"sections":[{"title":"Projects","kind":"projects",
             "items":[{"name":"A","visible":false}]}]}"#,
    )
    .unwrap();

    assert!(doc.sections[0].is_empty(), "nothing would render");
    assert_eq!(
        doc.visible_sections().count(),
        0,
        "so the section should not render its heading either"
    );
}

#[test]
fn hiding_an_entry_survives_a_round_trip() {
    let mut doc: CvDocument = serde_json::from_str(FIXTURE).unwrap();
    let section = doc
        .sections
        .iter_mut()
        .find(|s| s.kind() == SectionKind::Projects)
        .unwrap();

    let json = serde_json::to_string(&section).unwrap();
    assert!(
        json.contains("\"visible\""),
        "the flag is serialized per entry"
    );

    let before = section.visible_len();
    let parsed: CvDocument = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(
        parsed
            .sections
            .iter()
            .find(|s| s.kind() == SectionKind::Projects)
            .unwrap()
            .visible_len(),
        before
    );
}

#[test]
fn experience_switches_default_to_the_reference_behaviour() {
    // A document that predates the switches must render exactly as it used to:
    // role first, promotions grouped.
    let doc: CvDocument = serde_json::from_str(
        r#"{"sections":[{"title":"Work","kind":"experience","items":[{"role":"Dev"}]}]}"#,
    )
    .unwrap();

    match &doc.sections[0].body {
        rustycv_core::SectionBody::Experience {
            order,
            group_promotions,
            ..
        } => {
            assert_eq!(*order, rustycv_core::EntryOrder::RoleFirst);
            assert!(*group_promotions);
        }
        other => panic!("expected an experience section, got {other:?}"),
    }
}

#[test]
fn every_serialized_key_is_camel_case() {
    // `#[serde(rename_all)]` on an enum renames its *variants*, not the fields
    // inside them — so `group_promotions` shipped as snake_case while the
    // templates looked for `groupPromotions`, and the switch silently did
    // nothing. Assert the whole document rather than that one field.
    let doc: CvDocument = serde_json::from_str(FIXTURE).unwrap();
    let value = serde_json::to_value(&doc).unwrap();

    fn walk(value: &serde_json::Value, path: &str, bad: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, child) in map {
                    if key.contains('_') {
                        bad.push(format!("{path}.{key}"));
                    }
                    walk(child, &format!("{path}.{key}"), bad);
                }
            }
            serde_json::Value::Array(items) => {
                for (i, child) in items.iter().enumerate() {
                    walk(child, &format!("{path}[{i}]"), bad);
                }
            }
            _ => {}
        }
    }

    let mut bad = Vec::new();
    walk(&value, "", &mut bad);
    assert!(
        bad.is_empty(),
        "snake_case keys leaked into the wire format: {bad:?}"
    );
}
