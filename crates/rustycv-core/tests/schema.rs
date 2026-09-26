use rustycv_core::{CvDocument, PageSize, Section, SectionKind};

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
    let mut doc = CvDocument {
        sections: vec![
            Section::new(SectionKind::Experience),
            Section::new(SectionKind::Education),
            Section::new(SectionKind::Skills),
        ],
        ..Default::default()
    };
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

// ------------------------------------------------------------- rich text

use rustycv_core::{Block, BlockKind, RichText, Run};

#[test]
fn a_bare_string_loads_as_unstyled_rich_text() {
    // Every document written before rich text existed stores plain strings.
    let text: RichText = serde_json::from_str(r#""Reduced API latency by 50%.""#).unwrap();
    assert_eq!(text.plain_text(), "Reduced API latency by 50%.");
    assert_eq!(text.runs().len(), 1);
    assert!(text.runs()[0].is_plain());
}

#[test]
fn unstyled_text_serializes_back_to_a_bare_string() {
    // Keeps exports and hand-edited fixtures readable instead of turning every
    // line into an array of one object.
    let text = RichText::plain("Hello");
    assert_eq!(serde_json::to_string(&text).unwrap(), r#""Hello""#);
    assert_eq!(
        serde_json::to_string(&RichText::default()).unwrap(),
        r#""""#
    );
}

#[test]
fn formatted_text_round_trips_as_runs() {
    let json = r#"[{"text":"Improved accuracy to "},{"text":"98.5%","bold":true}]"#;
    let text: RichText = serde_json::from_str(json).unwrap();

    assert_eq!(text.plain_text(), "Improved accuracy to 98.5%");
    assert!(!text.runs()[0].bold);
    assert!(text.runs()[1].bold);

    // Marks must survive, so this one cannot collapse to a string.
    let round_tripped: RichText =
        serde_json::from_str(&serde_json::to_string(&text).unwrap()).unwrap();
    assert_eq!(round_tripped, text);
}

#[test]
fn rich_text_knows_when_it_would_render_nothing() {
    assert!(RichText::default().is_empty());
    assert!(RichText::plain("   ").is_empty());
    assert!(!RichText::plain("x").is_empty());
    assert!(RichText::from_runs(vec![Run::plain("")]).is_empty());
}

#[test]
fn the_fixture_still_loads_with_rich_text_fields() {
    // The fixture predates the feature and stores plain strings throughout.
    let doc: CvDocument = serde_json::from_str(FIXTURE).unwrap();
    assert!(doc.basics.summary.is_empty());

    let json = serde_json::to_string(&doc).unwrap();
    assert!(
        json.contains("\"Reduced API latency by 50% by caching CORS preflight responses.\""),
        "unstyled bullets should still be written as plain strings"
    );
    let again: CvDocument = serde_json::from_str(&json).unwrap();
    assert_eq!(again, doc);
}

#[test]
fn a_bullet_can_carry_marks() {
    let doc: CvDocument = serde_json::from_str(
        r#"{"sections":[{"title":"Work","kind":"experience","items":[{"role":"Dev","bullets":[
             [{"text":"Improved accuracy to "},{"text":"98.5%","bold":true,"italic":true}]
           ]}]}]}"#,
    )
    .unwrap();

    match &doc.sections[0].body {
        rustycv_core::SectionBody::Experience { items, .. } => {
            let highlights = &items[0].bullets;
            assert_eq!(highlights.blocks()[0].kind, BlockKind::Bullet);
            assert_eq!(highlights.plain_text(), "Improved accuracy to 98.5%");
            assert!(highlights.runs()[1].bold && highlights.runs()[1].italic);
        }
        other => panic!("expected experience, got {other:?}"),
    }
}

#[test]
fn highlights_written_one_value_per_bullet_load_as_bullets() {
    // How every document before this change stored them. Losing the shape here
    // would turn a CV's highlights into one run-on paragraph.
    let doc: CvDocument = serde_json::from_str(
        r#"{"sections":[{"title":"Work","kind":"experience","items":[{"role":"Dev",
             "bullets":["Cut p99 latency.", [{"text":"Shipped ","bold":true},{"text":"retries."}]]}]}]}"#,
    )
    .unwrap();

    let highlights = match &doc.sections[0].body {
        rustycv_core::SectionBody::Experience { items, .. } => &items[0].bullets,
        other => panic!("expected experience, got {other:?}"),
    };
    assert_eq!(highlights.blocks().len(), 2);
    assert!(highlights
        .blocks()
        .iter()
        .all(|b| b.kind == BlockKind::Bullet));
    assert_eq!(
        highlights.plain_text(),
        "Cut p99 latency.\nShipped retries."
    );
    assert!(highlights.runs()[1].bold);
}

#[test]
fn highlights_accept_the_forms_every_other_description_takes() {
    let highlights = |json: &str| {
        let doc: CvDocument = serde_json::from_str(&format!(
            r#"{{"sections":[{{"title":"Work","kind":"experience",
                 "items":[{{"role":"Dev","bullets":{json}}}]}}]}}"#
        ))
        .unwrap();
        match &doc.sections[0].body {
            rustycv_core::SectionBody::Experience { items, .. } => items[0].bullets.clone(),
            other => panic!("expected experience, got {other:?}"),
        }
    };

    // A bare string and a run array are one paragraph, exactly as they are in a
    // description — an array of *values* is the older per-bullet shape instead.
    assert_eq!(
        highlights(r#""Just a note.""#),
        RichText::plain("Just a note.")
    );
    assert_eq!(
        highlights(r#"[{"text":"Bold","bold":true}]"#).blocks()[0].kind,
        BlockKind::Paragraph
    );
    assert_eq!(
        highlights(r#"[{"kind":"numbered","runs":[{"text":"First"}]}]"#).blocks()[0].kind,
        BlockKind::Numbered
    );
    assert!(highlights("[]").is_empty());
}

#[test]
fn blocks_round_trip_and_stay_in_the_block_form() {
    let json = r#"[{"kind":"paragraph","runs":[{"text":"Led the rewrite."}]},
                   {"kind":"bullet","runs":[{"text":"Cut p99 latency."}]},
                   {"kind":"numbered","runs":[{"text":"Then the retries."}]}]"#;
    let text: RichText = serde_json::from_str(json).unwrap();

    let kinds: Vec<_> = text.blocks().iter().map(|b| b.kind).collect();
    assert_eq!(
        kinds,
        [BlockKind::Paragraph, BlockKind::Bullet, BlockKind::Numbered]
    );

    // More than one block, so there is no smaller form to collapse into.
    let written = serde_json::to_string(&text).unwrap();
    assert!(written.starts_with("[{\"kind\":"), "got {written}");
    assert_eq!(
        serde_json::from_str::<RichText>(&written).unwrap(),
        text,
        "blocks must survive the trip"
    );
}

#[test]
fn a_block_object_is_never_mistaken_for_a_run() {
    // Both forms are arrays of objects, so the untagged split rests on each
    // rejecting the other's fields. A bullet read as a run would lose its
    // marker and print as a paragraph.
    let text: RichText =
        serde_json::from_str(r#"[{"kind":"bullet","runs":[{"text":"One"}]}]"#).unwrap();
    assert_eq!(text.blocks()[0].kind, BlockKind::Bullet);

    let text: RichText = serde_json::from_str(r#"[{"text":"One","bold":true}]"#).unwrap();
    assert_eq!(text.blocks().len(), 1);
    assert_eq!(text.blocks()[0].kind, BlockKind::Paragraph);
    assert!(text.runs()[0].bold);
}

#[test]
fn a_single_unstyled_paragraph_still_collapses_to_a_string() {
    // The block form must not leak into documents that do not need it.
    let text = RichText::from_blocks(vec![Block::paragraph(vec![Run::plain("Hello")])]);
    assert_eq!(serde_json::to_string(&text).unwrap(), r#""Hello""#);

    // A soft line break lives inside a run, so this stays a bare string too.
    let text = RichText::plain("One\nTwo");
    assert_eq!(serde_json::to_string(&text).unwrap(), r#""One\nTwo""#);
    assert_eq!(
        serde_json::from_str::<RichText>(r#""One\nTwo""#).unwrap(),
        text
    );
}

#[test]
fn blocks_that_would_render_nothing_are_dropped() {
    // An empty block prints as a blank line the user cannot see or select, so
    // it must not survive a save.
    let text = RichText::from_blocks(vec![
        Block::paragraph(vec![Run::plain("One")]),
        Block::paragraph(vec![Run::plain("")]),
        Block::new(BlockKind::Bullet, vec![]),
    ]);
    assert_eq!(text.blocks().len(), 1);
    assert_eq!(serde_json::to_string(&text).unwrap(), r#""One""#);
}

#[test]
fn rich_text_knows_when_it_can_share_a_line() {
    // A project description sits after the project's name when it can, and
    // drops to its own block when it cannot.
    assert!(RichText::plain("A routing daemon").is_inline());
    assert!(RichText::default().is_inline());
    assert!(!RichText::plain("Two\nLines").is_inline());
    assert!(!RichText::from_blocks(vec![
        Block::paragraph(vec![Run::plain("One")]),
        Block::paragraph(vec![Run::plain("Two")]),
    ])
    .is_inline());
    assert!(
        !RichText::from_blocks(vec![Block::new(BlockKind::Bullet, vec![Run::plain("One")])])
            .is_inline()
    );
}

#[test]
fn a_description_can_hold_blocks() {
    let doc: CvDocument = serde_json::from_str(
        r#"{"sections":[{"title":"Projects","kind":"projects","items":[{"name":"Atlas",
             "description":[{"kind":"bullet","runs":[{"text":"Renders 2M points."}]},
                            {"kind":"bullet","runs":[{"text":"One binary."}]}]}]}]}"#,
    )
    .unwrap();

    match &doc.sections[0].body {
        rustycv_core::SectionBody::Projects { items, .. } => {
            let description = &items[0].description;
            assert_eq!(description.blocks().len(), 2);
            assert!(description
                .blocks()
                .iter()
                .all(|b| b.kind == BlockKind::Bullet));
            assert_eq!(description.plain_text(), "Renders 2M points.\nOne binary.");
        }
        other => panic!("expected projects, got {other:?}"),
    }

    let again: CvDocument = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(again, doc);
}
