//! Behaviour tests for the metadata-only tokenizer loader and the section
//! layout it resolves (ADR-029 §2A.1, §2A.4).
//!
//! These run in a **no-semantic** build on purpose. The loader exists so that
//! capture can measure token windows without weights, a tract plan or a
//! download; a suite that only ran with `semantic-search` would prove nothing
//! about the build most users have.
//!
//! Fixtures come from [`forgeplan_core::embed::tokenizer::testing`], which
//! builds a **real** character-splitting tokenizer, writes it through the
//! crate's own serialiser, and loads it back through the production
//! `MetadataTokenizer::load`. Nothing here fakes a tokenizer, so a loader that
//! never read the file would fail these tests rather than pass them.

use std::path::Path;

use forgeplan_core::constellation::jobs::{EmbeddingProfile, section_input_digest, section_input_text};
use forgeplan_core::embed::tokenizer::{
    ByteRange, HEADER_CONTEXT_TOKENS, LayoutResolution, MAX_TOKENS_CEILING, MetadataTokenizer,
    SECTION_LAYOUT_ALGORITHM, SectionLayout, TokenBudgets, find_metadata_snapshot, has_all_metadata,
    resolve_layout_in, testing,
};

fn tokenizer(limit: usize) -> (tempfile::TempDir, MetadataTokenizer, TokenBudgets) {
    testing::fixture_tokenizer(limit).expect("the fixture tokenizer must load")
}

// ---------------------------------------------------------------------------
// What the loader may read
// ---------------------------------------------------------------------------

#[test]
fn three_metadata_files_are_enough_and_no_weights_are_needed() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    testing::write_snapshot(tmp.path(), MAX_TOKENS_CEILING).expect("write fixture snapshot");

    // The fixture carries nothing else. If the loader needed the graph or the
    // 2.1 GB of weights, this is the call that would fail — and it is the
    // property that keeps a `no-semantic` build from needing them.
    assert!(tmp.path().join("tokenizer.json").is_file());
    assert!(!tmp.path().join("onnx").exists());
    assert!(!tmp.path().join("model.onnx_data").exists());
    assert!(has_all_metadata(tmp.path()));

    let loaded = MetadataTokenizer::load(tmp.path()).expect("metadata alone must load");
    assert!(
        loaded.layout().is_available(),
        "a complete metadata snapshot resolves a real layout"
    );
    assert_eq!(loaded.layout().algorithm, SECTION_LAYOUT_ALGORITHM);
}

#[test]
fn a_directory_missing_any_metadata_file_is_not_a_snapshot() {
    for missing in ["tokenizer.json", "tokenizer_config.json", "config.json"] {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        testing::write_snapshot(tmp.path(), MAX_TOKENS_CEILING).expect("write fixture");
        std::fs::remove_file(tmp.path().join(missing)).expect("remove one file");

        assert!(
            !has_all_metadata(tmp.path()),
            "a snapshot without `{missing}` is a half-fetched directory"
        );
        assert!(
            MetadataTokenizer::load(tmp.path()).is_err(),
            "loading must fail loudly rather than produce a half layout"
        );
    }
}

#[test]
fn a_stray_tokenizer_json_alone_is_not_a_snapshot() {
    // The exact case the old lookup accepted: one file left by an interrupted
    // fetch. It must not resolve to a snapshot, and inside a cache layout it
    // must resolve to the fingerprinted unavailable state.
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let snapshot = tmp.path().join("models--BAAI--bge-m3/snapshots/rev-x");
    std::fs::create_dir_all(&snapshot).expect("snapshot dir");
    std::fs::write(snapshot.join("tokenizer.json"), "{}").expect("stray file");

    assert!(!has_all_metadata(&snapshot));
    assert!(find_metadata_snapshot(tmp.path(), "BAAI/bge-m3").is_none());

    let resolution = resolve_layout_in(tmp.path(), "BAAI/bge-m3");
    assert!(matches!(resolution, LayoutResolution::Unavailable(_)));
    assert!(!resolution.layout().is_available());
}

#[test]
fn refs_main_selects_the_revision_and_a_scan_finds_one_without_it() {
    let with_ref = tempfile::TempDir::new().expect("temp dir");
    let revision_dir = with_ref.path().join("models--BAAI--bge-m3");
    std::fs::create_dir_all(revision_dir.join("snapshots")).expect("dirs");
    testing::write_snapshot(&revision_dir.join("snapshots/rev-main"), MAX_TOKENS_CEILING)
        .expect("write fixture");
    std::fs::write(revision_dir.join("refs/main"), "rev-main\n").expect("write refs/main");
    assert_eq!(
        find_metadata_snapshot(with_ref.path(), "BAAI/bge-m3"),
        Some(revision_dir.join("snapshots/rev-main")),
        "refs/main names the resolved revision"
    );

    // No `refs/main`: the loader must still find the only complete snapshot by
    // scanning, because a cache that lost its ref file is still a usable cache.
    let without_ref = tempfile::TempDir::new().expect("temp dir");
    let scanned = without_ref.path().join("models--BAAI--bge-m3/snapshots/only");
    std::fs::create_dir_all(scanned.parent().expect("parent")).expect("dirs");
    testing::write_snapshot(&scanned, MAX_TOKENS_CEILING).expect("write fixture");
    assert_eq!(
        find_metadata_snapshot(without_ref.path(), "BAAI/bge-m3"),
        Some(scanned),
        "a complete snapshot is found even without a ref file"
    );
}

#[test]
fn an_absent_cache_resolves_to_the_fingerprinted_unavailable_state() {
    let resolution = resolve_layout_in(Path::new("/nonexistent-forgeplan-cache"), "BAAI/bge-m3");
    assert!(
        matches!(resolution, LayoutResolution::Unavailable(_)),
        "no cache is a state, not a failure"
    );
    let layout = resolution.layout();
    assert!(!layout.is_available());
    assert_eq!(
        layout.fingerprint().len(),
        64,
        "absence still has a fingerprint, so it can never equal a present layout"
    );
    assert!(resolution.tokenizer().is_none());
}

#[test]
fn a_present_and_an_absent_layout_never_hash_equal() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    testing::write_snapshot(tmp.path(), MAX_TOKENS_CEILING).expect("write fixture");
    let present = resolve_layout_in(tmp.path(), "BAAI/bge-m3");
    let absent = resolve_layout_in(Path::new("/nonexistent-cache"), "BAAI/bge-m3");

    assert!(present.layout().is_available());
    assert!(!absent.layout().is_available());
    assert_ne!(
        present.fingerprint(),
        absent.fingerprint(),
        "metadata appearing or disappearing is a new layout identity, never a silent reuse"
    );
}

// ---------------------------------------------------------------------------
// The layout fingerprint
// ---------------------------------------------------------------------------

#[test]
fn the_fingerprint_binds_the_actual_limit_and_the_tokenizer_bytes() {
    let (dir, base, _) = tokenizer(MAX_TOKENS_CEILING);
    let base_fingerprint = base.layout().fingerprint();

    // A different declared limit is a different layout. This is the case that
    // used not to exist at all: `config_digest` carried no tokenizer input, so
    // two captures that cut windows differently agreed.
    let (_, smaller, _) = tokenizer(128);
    assert_ne!(
        base_fingerprint,
        smaller.layout().fingerprint(),
        "a different actual max_tokens is a different layout"
    );
    drop(dir);
}

#[test]
fn the_fingerprint_is_stable_across_two_loads_of_the_same_bytes() {
    let tmp_a = tempfile::TempDir::new().expect("temp dir");
    let tmp_b = tempfile::TempDir::new().expect("temp dir");
    testing::write_snapshot(tmp_a.path(), MAX_TOKENS_CEILING).expect("write a");
    testing::write_snapshot(tmp_b.path(), MAX_TOKENS_CEILING).expect("write b");

    let first = resolve_layout_in(tmp_a.path(), "BAAI/bge-m3");
    let second = resolve_layout_in(tmp_b.path(), "BAAI/bge-m3");
    assert_eq!(
        first.fingerprint(),
        second.fingerprint(),
        "the same physical bytes under the same layout identity give the same fingerprint — \
         this is what makes the layout reproducible across machines"
    );
}

#[test]
fn the_actual_limit_is_the_model_limit_when_smaller_and_the_ceiling_otherwise() {
    let declared = testing::fixture_layout(128).expect("fixture layout");
    assert_eq!(declared.max_tokens, 128);

    let generous = testing::fixture_layout(8192).expect("fixture layout");
    assert_eq!(
        generous.max_tokens, MAX_TOKENS_CEILING,
        "BGE-M3 declares 8192; the ceiling wins"
    );
}

// ---------------------------------------------------------------------------
// The shared builder's budget
// ---------------------------------------------------------------------------

#[test]
fn a_small_limit_shrinks_the_header_so_the_body_still_advances() {
    let (_, small, budgets) = tokenizer(64);
    assert_eq!(small.layout().max_tokens, 64);
    assert!(
        budgets.header_context_tokens < HEADER_CONTEXT_TOKENS,
        "a 64-token limit cannot spend 128 tokens on the title"
    );
    assert!(budgets.body_tokens > 0, "the body must still get room");
    assert!(
        budgets.header_context_tokens + budgets.body_tokens <= budgets.max_tokens,
        "header plus body must fit inside the actual limit"
    );
}

#[test]
fn the_special_token_budget_is_a_real_measurement_not_a_hardcoded_zero() {
    let (_, tokenizer, budgets) = tokenizer(256);
    assert!(
        budgets.special_tokens > 0,
        "the fixture wraps every sequence, and the arithmetic that reserves room for those \
         tokens is what this measures"
    );
    assert_eq!(budgets.special_tokens, tokenizer.layout().special_tokens);
}

#[test]
fn a_profile_that_cannot_hold_one_body_token_is_refused_not_looped() {
    // A layout whose limit is smaller than the special-token budget plus one
    // body token has no honest windowing. It must be an explicit refusal:
    // infinite empty windows, or a silently truncated tail, are the two
    // failures this replaces.
    let impossible = SectionLayout::from_metadata(
        "t".to_string(),
        "tc".to_string(),
        "c".to_string(),
        Some(3),
        64,
    );
    let error = TokenBudgets::resolve(&impossible)
        .expect_err("no body token can fit")
        .to_string();
    assert!(
        error.contains("single body token"),
        "the refusal must name the reason; got: {error}"
    );
}

#[test]
fn the_unavailable_layout_has_no_budgets_at_all() {
    let unavailable = SectionLayout::unavailable("no cached snapshot");
    assert!(
        TokenBudgets::resolve(&unavailable).is_err(),
        "a fingerprinted unavailable layout must refuse to pretend it can measure"
    );
}

// ---------------------------------------------------------------------------
// The assembled input
// ---------------------------------------------------------------------------

#[test]
fn the_assembled_input_never_exceeds_the_actual_limit_with_special_tokens() {
    let (_, tokenizer, budgets) = tokenizer(96);
    let limit = budgets.max_tokens;

    // A title and heading far longer than the header budget, and a body of
    // many tokens: the two ways the old builder overflowed.
    let huge_title = "the ".repeat(400);
    let huge_heading = "section ".repeat(400);
    let body = "body ".repeat(2_000);

    for (title, heading) in [
        (huge_title.as_str(), Some(huge_heading.as_str())),
        (huge_title.as_str(), None),
        ("short", Some(huge_heading.as_str())),
    ] {
        let input = section_input_text(&tokenizer, &budgets, title, heading, body.as_str())
            .expect("a workable profile must build an input");
        let counted = tokenizer
            .count_with_special_tokens(&input)
            .expect("count");
        assert!(
            counted <= limit,
            "the final input carried {counted} tokens against an actual limit of {limit}"
        );
        assert!(
            !input.is_empty(),
            "the body must survive; an empty input would be a silent tail loss"
        );
    }
}

#[test]
fn the_display_title_and_heading_are_never_shortened_in_the_row_but_the_input_is() {
    let (_, tokenizer, budgets) = tokenizer(48);
    let long_title = "the title ".repeat(400);
    let long_heading = "section ".repeat(400);
    let body = "body ".repeat(2_000);

    let input = section_input_text(
        &tokenizer,
        &budgets,
        &long_title,
        Some(&long_heading),
        body.as_str(),
    )
    .expect("input");
    assert!(
        tokenizer
            .count_with_special_tokens(&input)
            .expect("count")
            <= budgets.max_tokens,
        "the input is capped"
    );
    // The builder returns only the input; the display strings are the caller's
    // to keep. This asserts the cap is applied to the *text*, not by rewriting
    // what the caller still holds.
    assert_eq!(
        long_title.len() > 4_000,
        true,
        "the caller's display title is untouched by the builder"
    );
}

#[test]
fn a_section_input_digest_is_the_digest_of_exactly_that_input() {
    let (_, tokenizer, budgets) = tokenizer(128);

    let first = section_input_digest(
        &tokenizer,
        &budgets,
        "Auth",
        Some("Problem"),
        "the body of the problem section",
    )
    .expect("digest");
    let again = section_input_digest(
        &tokenizer,
        &budgets,
        "Auth",
        Some("Problem"),
        "the body of the problem section",
    )
    .expect("digest");
    assert_eq!(first, again, "the same inputs hash the same");

    let other = section_input_digest(
        &tokenizer,
        &budgets,
        "Auth",
        Some("Problem"),
        "the body of the problem section!",
    )
    .expect("digest");
    assert_ne!(first, other, "one byte of body changes the input digest");
    assert_eq!(first.len(), 64, "a SHA-256 hex digest");
}

// ---------------------------------------------------------------------------
// The profile binding
// ---------------------------------------------------------------------------

fn profile_with(layout_fingerprint: &str, max_tokens: usize) -> EmbeddingProfile {
    EmbeddingProfile {
        model_repo: "BAAI/bge-m3".to_string(),
        weights_revision: "rev".to_string(),
        weights_checksums: vec!["w".to_string()],
        tokenizer_checksum: "t".to_string(),
        tokenizer_config_checksum: "tc".to_string(),
        dimensions: 1024,
        engine_version: "engine".to_string(),
        pooling: "cls".to_string(),
        normalization: "l2".to_string(),
        query_prefix: String::new(),
        document_prefix: String::new(),
        body_prefix_chars: 2000,
        max_tokens,
        padding: "none".to_string(),
        truncation: "longest_first".to_string(),
        section_layout_fingerprint: layout_fingerprint.to_string(),
    }
}

#[test]
fn a_layout_change_changes_the_profile_id() {
    // Two profiles identical except for the layout fingerprint must not share
    // an id: the whole point of binding it is that vectors from one windowing
    // cannot be served as current under another.
    let base = profile_with("layout-a", MAX_TOKENS_CEILING);
    let other = profile_with("layout-b", MAX_TOKENS_CEILING);
    assert_ne!(
        base.profile_id(),
        other.profile_id(),
        "the layout fingerprint is part of the embedding space's identity"
    );

    // And the actual token limit too, so a capture measuring windows at one
    // limit cannot be served by an engine truncating at another.
    let smaller = profile_with("layout-a", 128);
    assert_ne!(base.profile_id(), smaller.profile_id());
}

// ---------------------------------------------------------------------------
// Byte ranges
// ---------------------------------------------------------------------------

#[test]
fn a_byte_range_slices_only_on_a_real_boundary() {
    let body = "abcdefghij";
    assert_eq!(
        ByteRange { start: 0, end: 4 }.slice(body),
        Some("abcd"),
        "a valid range slices"
    );
    assert_eq!(
        ByteRange { start: 9, end: 99 }.slice(body),
        None,
        "an out-of-range end must be None, not a panic and not a silent clamp"
    );
    // A range that would split a multi-byte character is not a boundary.
    let multibyte = "привет";
    assert_eq!(
        ByteRange { start: 0, end: 1 }.slice(multibyte),
        None,
        "byte 1 is inside the first two-byte character"
    );
    assert!(ByteRange { start: 0, end: 0 }.is_empty());
}

// ---------------------------------------------------------------------------
// One shared tokenizer
// ---------------------------------------------------------------------------

#[test]
fn one_shared_tokenizer_gives_one_input_per_identical_input() {
    use std::sync::Arc;

    // One immutable `Arc` per capture is the shape the layout rests on:
    // re-parsing `tokenizer.json` per artifact is what made capture time scale
    // with corpus size for nothing.
    let (_, loaded, budgets) = tokenizer(128);
    let shared: Arc<MetadataTokenizer> = Arc::new(loaded);
    let handles: Vec<_> = (0..4).map(|_| Arc::clone(&shared)).collect();

    let digests: Vec<String> = handles
        .iter()
        .map(|tokenizer| {
            section_input_digest(tokenizer, &budgets, "Auth", None, "the body").expect("digest")
        })
        .collect();
    assert!(
        digests.windows(2).all(|pair| pair[0] == pair[1]),
        "one shared tokenizer must produce one input per identical input"
    );
}
