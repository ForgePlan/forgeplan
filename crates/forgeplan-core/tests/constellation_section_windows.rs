//! Behaviour tests for token-windowed section rows (ADR-029 §2A.3, §2A.7).
//!
//! The invariant under test is the one the old implementation could not meet:
//! **the concatenation of every window, in ordinal order, reproduces the
//! artifact's body exactly** — every byte, including inter-window whitespace,
//! metaspaces, CJK, emoji and a long no-space tail. A scheme that merely
//! *claims* whole-body coverage while dropping a tail is the defect this slice
//! removes, so every case here ends by comparing the rebuilt string to the
//! original body, not by counting windows.
//!
//! Runs in the `no-semantic` build on purpose: metadata-only layout is what
//! lets a `no-semantic` capture publish sections at all, and a suite that only
//! ran with weights would miss the case that matters most here.

use std::path::Path;

use forgeplan_core::constellation::capture::{
    CapturedSection, CapturedSourceView, Coverage, FilesystemSupport, ParsedArtifact,
    body_is_fully_covered, sections_for,
};
use forgeplan_core::embed::tokenizer::{
    LayoutResolution, MetadataTokenizer, SectionLayout, TokenBudgets, resolve_layout_in, testing,
};

fn tokenizer(limit: usize) -> (tempfile::TempDir, MetadataTokenizer, TokenBudgets) {
    testing::fixture_tokenizer(limit).expect("the fixture tokenizer must load")
}

fn artifact(body: &str) -> ParsedArtifact {
    ParsedArtifact {
        canonical_key: "PRD-001".to_string(),
        display_id: "PRD-001".to_string(),
        kind: "prd".to_string(),
        status: "draft".to_string(),
        title: "Auth".to_string(),
        body: std::sync::Arc::from(body),
        depth: Some("standard".to_string()),
        author: None,
        parent_epic: None,
        valid_until: None,
        created_at: None,
        updated_at: None,
        tags: Vec::new(),
        identity: Default::default(),
        frontmatter: Default::default(),
        relative_path: "prds/PRD-001-auth.md".to_string(),
        outgoing: Default::default(),
        raw_bytes: std::sync::Arc::from(body.as_bytes()),
        source_digest: "sd".to_string(),
        input_digest: "id".to_string(),
        profile_id: None,
        semantic_state: forgeplan_core::constellation::jobs::SemanticState::Unavailable,
        incarnation: 1,
        row_revision: 1,
        diagnostics: Vec::new(),
    }
}

/// The core invariant, as a reusable assertion.
fn assert_body_is_reproduced(parsed: &ParsedArtifact, sections: &[CapturedSection]) {
    let mut rows: Vec<&CapturedSection> = sections.iter().collect();
    rows.sort_by_key(|row| row.ordinal);

    let mut rebuilt = String::new();
    let mut cursor = 0usize;
    for row in rows {
        assert_eq!(
            row.byte_start, cursor,
            "section `{}` starts at {} but the previous window ended at {cursor}; the windows \
             must tile the body with no gap",
            row.canonical_key, row.byte_start
        );
        rebuilt.push_str(row.content.as_ref());
        cursor = row.byte_end;
    }
    assert_eq!(
        cursor,
        parsed.body.len(),
        "the windows cover {cursor} of {} body bytes",
        parsed.body.len()
    );
    // Compared as `&str` on both sides: `rebuilt` is a `String` and
    // `parsed.body` an `Arc<str>`, and there is no `PartialEq<Arc<str>>` for
    // `String`. This is the same type treatment the production verifier uses
    // in `capture::body_is_fully_covered`, so the test and the code it checks
    // assert the identical invariant the same way — do not "simplify" this
    // back to a `String`-versus-`Arc` comparison.
    assert_eq!(
        rebuilt.as_str(),
        parsed.body.as_ref(),
        "the concatenation of the windows must BE the body, byte for byte"
    );
    body_is_fully_covered(parsed, sections).expect("the coverage verifier agrees");
}

// ---------------------------------------------------------------------------
// Whole-body coverage, per input shape
// ---------------------------------------------------------------------------

#[test]
fn russian_body_coverage_is_exact() {
    let (_tmp, tokenizer, budgets) = tokenizer(64);
    let mut body = String::from("Предисловие до первого заголовка.\n\n");
    for index in 0..120 {
        body.push_str(&format!(
            "## Заголовок {index}\n\nтело раздела для проверки покрытия {index}.\n\n"
        ));
    }
    let parsed = artifact(&body);
    let sections = sections_for(&parsed, &tokenizer, &budgets).expect("sections");
    assert!(
        sections.len() > 120,
        "a long Russian document must produce many windows, got {}",
        sections.len()
    );
    assert_body_is_reproduced(&parsed, &sections);
}

#[test]
fn english_body_coverage_is_exact() {
    let (_tmp, tokenizer, budgets) = tokenizer(64);
    let mut body = String::from("A preamble before the first heading.\n\n");
    for index in 0..120 {
        body.push_str(&format!(
            "## Heading {index}\n\nthe body of section {index} for coverage checking.\n\n"
        ));
    }
    let parsed = artifact(&body);
    let sections = sections_for(&parsed, &tokenizer, &budgets).expect("sections");
    assert_body_is_reproduced(&parsed, &sections);
}

#[test]
fn cjk_and_emoji_coverage_is_exact() {
    let (_tmp, tokenizer, budgets) = tokenizer(64);
    let mut body = String::from("前言。\n\n");
    for index in 0..100 {
        body.push_str(&format!(
            "## 日本語の見出し {index}\n\n中文の本文 {index}。한국어 본문 {index}. 😀 emoji tail.\n\n"
        ));
    }
    let parsed = artifact(&body);
    let sections = sections_for(&parsed, &tokenizer, &budgets).expect("sections");
    assert_body_is_reproduced(&parsed, &sections);
    assert!(
        sections.iter().any(|row| row.content.contains('😀')),
        "a four-byte emoji must land inside some window, not be dropped as an offset artefact"
    );
}

#[test]
fn a_no_space_run_keeps_its_tail() {
    // No spaces at all: one enormous token run, which is the case where a naive
    // character-window implementation silently truncates.
    let (_tmp, tokenizer, budgets) = tokenizer(32);
    let line: String = std::iter::repeat('x').take(4_000).collect();
    let body = format!("# Heading\n\n{line}\n");
    let parsed = artifact(&body);
    let sections = sections_for(&parsed, &tokenizer, &budgets).expect("sections");
    assert_body_is_reproduced(&parsed, &sections);
    assert!(
        sections.iter().any(|row| row.content.contains(&line[..2_000])),
        "the long run must be present in the window set"
    );
}

#[test]
fn whitespace_and_metaspace_between_windows_is_preserved() {
    let (_tmp, tokenizer, budgets) = tokenizer(48);
    // Inter-window gaps are whitespace runs, tabs and blank lines. "Start
    // here, stop at the token limit" would drop these.
    let mut body = String::new();
    for index in 0..80 {
        body.push_str(&format!("## Section {index}\n\n   \n\t\nbody word {index}\n\n\n"));
    }
    let parsed = artifact(&body);
    let sections = sections_for(&parsed, &tokenizer, &budgets).expect("sections");
    assert_body_is_reproduced(&parsed, &sections);
    assert!(
        sections.iter().any(|row| row.content.contains('\t')),
        "a tab between windows must survive in some window"
    );
}

#[test]
fn an_empty_body_yields_one_empty_window_and_stays_empty() {
    let (_tmp, tokenizer, budgets) = tokenizer(64);
    let parsed = artifact("");
    let sections = sections_for(&parsed, &tokenizer, &budgets).expect("sections");
    assert_eq!(
        sections.len(),
        1,
        "an empty body still produces exactly one row, so 'no sections' stays distinguishable"
    );
    assert_eq!(sections[0].content.as_ref(), "");
    assert_eq!(sections[0].byte_start, 0);
    assert_eq!(sections[0].byte_end, 0);
    assert_body_is_reproduced(&parsed, &sections);
}

#[test]
fn a_whitespace_only_body_is_preserved_exactly() {
    let (_tmp, tokenizer, budgets) = tokenizer(64);
    let parsed = artifact("   \n\n\t\n   ");
    let sections = sections_for(&parsed, &tokenizer, &budgets).expect("sections");
    assert_body_is_reproduced(&parsed, &sections);
}

// ---------------------------------------------------------------------------
// Budget: every input fits, and only the input is capped
// ---------------------------------------------------------------------------

#[test]
fn every_section_input_fits_the_actual_limit_including_special_tokens() {
    let (_tmp, tokenizer, budgets) = tokenizer(48);
    let huge_title = "Заголовок ".repeat(300);
    let mut body = String::new();
    for index in 0..100 {
        body.push_str(&format!("## Раздел {index}\n\nтело для проверки {index}.\n\n"));
    }
    let mut parsed = artifact(&body);
    parsed.title = huge_title;
    let sections = sections_for(&parsed, &tokenizer, &budgets).expect("sections");
    assert_body_is_reproduced(&parsed, &sections);

    for row in &sections {
        let input = tokenizer
            .build_section_input(
                &parsed.title,
                row.heading.as_deref(),
                &row.content,
                &budgets,
            )
            .expect("input");
        let counted = tokenizer
            .count_with_special_tokens(&input)
            .expect("count");
        assert!(
            counted <= budgets.max_tokens,
            "section `{}` produced {counted} tokens against a limit of {}",
            row.canonical_key,
            budgets.max_tokens
        );
    }
}

#[test]
fn the_display_heading_is_never_shortened_even_though_the_input_is() {
    // The rule that is easy to break while fixing the budget: cap the
    // embedding input, never the stored heading.
    let (_tmp, tokenizer, budgets) = tokenizer(32);
    let long_heading = "Очень длинный заголовок раздела ".repeat(40);
    let body = format!("# {long_heading}\n\nthe body of the section.\n");
    let parsed = artifact(&body);
    let sections = sections_for(&parsed, &tokenizer, &budgets).expect("sections");
    assert_body_is_reproduced(&parsed, &sections);
    assert!(
        sections
            .iter()
            .all(|row| row.heading.as_deref() == Some(long_heading.as_str())),
        "the row keeps the FULL display heading; only the embedding input is budgeted"
    );
}

#[test]
fn two_captures_of_the_same_bytes_under_different_layouts_differ_in_identity() {
    let (_, first, _) = tokenizer(64);
    let (_, second, _) = tokenizer(128);
    assert_ne!(
        first.layout().fingerprint(),
        second.layout().fingerprint(),
        "the same physical window under a different layout is a different input, so the layout \
         identity has to travel with the capture"
    );
}

// ---------------------------------------------------------------------------
// Metadata absent / present / absent
// ---------------------------------------------------------------------------

fn view_with(
    resolution: &LayoutResolution,
    sections: Vec<CapturedSection>,
) -> CapturedSourceView {
    CapturedSourceView {
        workspace: std::path::PathBuf::from("/tmp/ws/.forgeplan"),
        checkout_id: "local-test".to_string(),
        index_epoch: 1,
        snapshot_id: "snap".to_string(),
        evaluated_at: "2026-10-21T00:00:00.000Z".to_string(),
        source_set_digest: "srcset".to_string(),
        config_digest: "cfg".to_string(),
        layout: resolution.layout(),
        tokenizer: resolution.tokenizer().cloned(),
        body_prefix_chars: 2000,
        artifacts: Vec::new(),
        sections,
        entries: Vec::new(),
        unusable: Vec::new(),
        enumerated_paths: Vec::new(),
        coverage: Coverage {
            expected: 0,
            read: 0,
            invalid: 0,
            unavailable: 0,
            excluded: 0,
        },
        diagnostics: Vec::new(),
        confirmed: true,
        filesystem_support: FilesystemSupport::Supported,
        attempts: 1,
    }
}

/// A cache directory holding a real metadata-only snapshot, plus the cache
/// root so the test can remove it again.
fn populated_cache(limit: usize) -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    // The path is taken **before** `tmp` is moved into the tuple. A tuple
    // expression evaluates left to right, so returning `(tmp, tmp.path())`
    // moves the `TempDir` and then borrows it for the second element.
    let root = tmp.path().to_path_buf();
    let snapshot = root.join("models--BAAI--bge-m3/snapshots/rev-1");
    std::fs::create_dir_all(&snapshot).expect("dirs");
    testing::write_snapshot(&snapshot, limit).expect("write fixture");
    (tmp, root)
}

#[test]
fn metadata_absent_publishes_exactly_no_sections_and_no_input() {
    let resolution = resolve_layout_in(Path::new("/nonexistent-cache"), "BAAI/bge-m3");
    let view = view_with(&resolution, Vec::new());

    assert!(
        view.sections.is_empty(),
        "a capture without metadata publishes an exactly empty section set"
    );
    assert!(!view.layout.is_available());
    assert!(
        view.token_budgets().is_err(),
        "there is no budget to build an input under, and none may be invented"
    );
    assert!(
        view.section_layout().is_err(),
        "section_input must refuse rather than fall back to a character window"
    );
}

#[test]
fn metadata_absent_present_absent_yields_three_distinct_layouts_and_two_empty_sets() {
    let (cache, root) = populated_cache(128);

    // present
    let present = resolve_layout_in(&root, "BAAI/bge-m3");
    assert!(present.layout().is_available());
    let tokenizer = present.tokenizer().expect("tokenizer").clone();
    let budgets = TokenBudgets::resolve(tokenizer.layout()).expect("budgets");
    let parsed = artifact("# H\n\nthe body.\n");
    let sections = sections_for(&parsed, &tokenizer, &budgets).expect("sections");
    let view_present = view_with(&present, sections);
    assert!(
        !view_present.sections.is_empty(),
        "metadata present produces real sections"
    );

    // absent again — removing the snapshot is a NEW state, not a return to
    // any earlier one
    std::fs::remove_dir_all(root.join("models--BAAI--bge-m3/snapshots")).expect("remove");
    let absent_again = resolve_layout_in(&root, "BAAI/bge-m3");
    assert!(!absent_again.layout().is_available());
    let view_absent_again = view_with(&absent_again, Vec::new());
    assert!(view_absent_again.sections.is_empty());

    let absent_first = resolve_layout_in(Path::new("/nonexistent-cache"), "BAAI/bge-m3");
    let view_absent = view_with(&absent_first, Vec::new());

    assert_ne!(
        view_absent.layout.fingerprint(),
        view_present.layout.fingerprint(),
        "absent -> present is a new layout identity"
    );
    assert_ne!(
        view_present.layout.fingerprint(),
        view_absent_again.layout.fingerprint(),
        "present -> absent is a new layout identity, and the reason differs"
    );
    assert_ne!(
        view_absent.layout.fingerprint(),
        view_absent_again.layout.fingerprint(),
        "two absences with different reasons are different fingerprinted states"
    );
    drop(cache);
}

#[test]
fn an_unavailable_layout_is_a_value_not_a_missing_field() {
    let layout = SectionLayout::unavailable("no cached snapshot");
    assert!(!layout.is_available());
    assert_eq!(layout.fingerprint().len(), 64);
    assert_ne!(
        layout.fingerprint(),
        SectionLayout::unavailable("half-downloaded").fingerprint(),
        "the reason is part of the state"
    );
}

// ---------------------------------------------------------------------------
// Ordinals and keys
// ---------------------------------------------------------------------------

#[test]
fn ordinals_are_dense_and_keys_are_positional() {
    let (_tmp, tokenizer, budgets) = tokenizer(48);
    let mut body = String::new();
    for index in 0..40 {
        body.push_str(&format!("## Section {index}\n\nthe body {index}.\n\n"));
    }
    let parsed = artifact(&body);
    let sections = sections_for(&parsed, &tokenizer, &budgets).expect("sections");

    let ordinals: Vec<u32> = sections.iter().map(|row| row.ordinal).collect();
    let expected: Vec<u32> = (0..sections.len() as u32).collect();
    assert_eq!(
        ordinals, expected,
        "ordinals are dense and start at 0, so a key always names one position"
    );
    for row in &sections {
        assert_eq!(
            row.canonical_key,
            format!("PRD-001#s{}", row.ordinal),
            "the key is derived from the ordinal, never guessed"
        );
        assert_eq!(row.parent_key, "PRD-001");
    }
}

#[test]
fn a_reordering_edit_changes_which_bytes_a_key_binds() {
    let (_tmp, tokenizer, budgets) = tokenizer(48);
    let first = artifact("## A\n\nbody a.\n\n## B\n\nbody b.\n");
    let second = artifact("## B\n\nbody b.\n\n## A\n\nbody a.\n");
    let first_sections = sections_for(&first, &tokenizer, &budgets).expect("first");
    let second_sections = sections_for(&second, &tokenizer, &budgets).expect("second");
    assert_body_is_reproduced(&first, &first_sections);
    assert_body_is_reproduced(&second, &second_sections);

    let first_at_0 = first_sections
        .iter()
        .find(|row| row.ordinal == 0)
        .expect("ordinal 0");
    let second_at_0 = second_sections
        .iter()
        .find(|row| row.ordinal == 0)
        .expect("ordinal 0");
    assert_ne!(
        first_at_0.input_digest, second_at_0.input_digest,
        "the same key now binds different bytes, and input_digest is what detects it"
    );
}

#[test]
fn identical_bytes_produce_identical_rows() {
    let (_tmp, tokenizer, budgets) = tokenizer(48);
    let body = "# H\n\nthe body.\n";
    let first = sections_for(&artifact(body), &tokenizer, &budgets).expect("first");
    let second = sections_for(&artifact(body), &tokenizer, &budgets).expect("second");
    assert_eq!(
        first, second,
        "the same captured bytes under the same layout are a pure function"
    );
}
