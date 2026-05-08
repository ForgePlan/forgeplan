//! PROB-060 Phase 2.3 (T3) — frontmatter edge case test suite.
//!
//! Hardens the parser/validator against the "weird shapes" we've actually
//! observed in the wild на legacy artifacts:
//!
//! - Missing identity fields (no slug / predicted_number / assigned_number)
//! - **Двойной frontmatter** (e.g. PRD-076 — two consecutive `---...---`
//!   blocks before body). Parser must read only the FIRST block (matches
//!   `forgeplan-core/src/artifact/frontmatter.rs::parse_frontmatter`
//!   behaviour) so the canonical identity fields in the outer frontmatter
//!   are honoured even when a stale BMAD-style inner block follows.
//! - Malformed YAML — fail with clear error, no panic.
//! - Very long titles (>200 chars) — slug generator truncates per SPEC-005.
//! - Unicode titles (Cyrillic, emoji, RTL Arabic, CJK) — `slugify` strips
//!   non-ASCII; if everything is non-ASCII the slug builder errors cleanly.
//! - Invalid slug shape (UPPERCASE / underscore / spaces) — `validate_slug`
//!   rejects.
//! - `assigned_number` null variants (`null`, `~`, empty, missing) — all
//!   normalise to `None`.
//! - `predicted_number` boundary values (0, very large) — out-of-range
//!   returns `None`.
//! - Empty body (frontmatter only) — must not panic.
//!
//! ## Why integration crate
//!
//! Most edge-case logic lives in `forgeplan-core::artifact::frontmatter`
//! and `forgeplan-core::artifact::types`. We could've put these in
//! the core crate's unit tests, but Phase 2.3 brief explicitly asks for
//! `crates/forgeplan-cli/tests/edge_cases_frontmatter.rs` so the suite
//! can also call into CLI-side helpers if needed in the future. Today
//! the tests exercise the public API of `forgeplan-core` end-to-end —
//! treating the core crate как black-box from the CLI's perspective.
//!
//! Reference: PROB-060, SPEC-005, ADR-012, RFC-009.

use forgeplan_core::artifact::frontmatter::{
    MAX_ARTIFACT_NUMBER, assigned_number_from_frontmatter, augment_frontmatter_with_id_fields,
    is_pre_merge, parse_frontmatter, predicted_number_from_frontmatter, refs_form_from_body,
    set_assigned_number, slug_from_frontmatter,
};
use forgeplan_core::artifact::types::{
    ArtifactKind, MAX_SLUG_LEN, MIN_SLUG_LEN, slug_from_kind_title, validate_slug,
};

// ---------------------------------------------------------------------------
// Group A — Missing fields
// ---------------------------------------------------------------------------

/// Legacy pre-PROB-060 artifact: `id`/`status`/`title` only, no slug,
/// no predicted_number, no assigned_number. Parser must succeed; helpers
/// must return `None`. Foundation case for backward-compat.
#[test]
fn missing_all_identity_fields_parses_returns_none() {
    let content = "---\nid: PRD-018\nstatus: active\ntitle: Legacy Artifact\n---\n\nBody.\n";
    let (fm, body) = parse_frontmatter(content).expect("must parse");
    assert_eq!(slug_from_frontmatter(&fm), None);
    assert_eq!(predicted_number_from_frontmatter(&fm), None);
    assert_eq!(assigned_number_from_frontmatter(&fm), None);
    assert!(is_pre_merge(&fm), "no assigned_number → pre-merge");
    assert!(body.contains("Body."));
}

/// Mid-migration artifact: slug present but predicted_number missing.
/// Helpers must each return their own state independently.
#[test]
fn missing_predicted_number_only() {
    let content = "---\nid: PRD-074\nstatus: draft\nslug: prd-auth-system\n---\n\nBody.\n";
    let (fm, _body) = parse_frontmatter(content).unwrap();
    assert_eq!(slug_from_frontmatter(&fm), Some("prd-auth-system"));
    assert_eq!(predicted_number_from_frontmatter(&fm), None);
    assert_eq!(assigned_number_from_frontmatter(&fm), None);
}

/// Mid-migration artifact: predicted_number present but slug missing.
/// `is_pre_merge` should still be true (assigned_number absent).
#[test]
fn missing_slug_only() {
    let content = "---\nid: PRD-074\nstatus: draft\npredicted_number: 74\n---\n\nBody.\n";
    let (fm, _body) = parse_frontmatter(content).unwrap();
    assert_eq!(slug_from_frontmatter(&fm), None);
    assert_eq!(predicted_number_from_frontmatter(&fm), Some(74));
    assert!(is_pre_merge(&fm));
}

// ---------------------------------------------------------------------------
// Group B — Двойной frontmatter (the PRD-076 case)
// ---------------------------------------------------------------------------

/// Real workspace pattern (e.g. PRD-076): outer frontmatter from
/// `forgeplan_new` template + inner BMAD-style block manually pasted
/// before the body. Parser MUST read only the first block — the inner
/// block becomes part of the body text. This is the canonical
/// behaviour of `parse_frontmatter` (it splits on the FIRST `\n---`
/// after the opening `---`).
#[test]
fn double_frontmatter_first_block_wins() {
    let content = "\
---
id: PRD-076
slug: prd-lazy-id
predicted_number: 76
assigned_number: null
status: draft
---

---
id: PRD-076
title: \"Lazy artifact ID assignment\"
status: Draft
priority: P0
---

# PRD-076: Body content here.
";
    let (fm, body) = parse_frontmatter(content).expect("must parse double FM");
    // First block determined identity; inner block is body text.
    assert_eq!(slug_from_frontmatter(&fm), Some("prd-lazy-id"));
    assert_eq!(predicted_number_from_frontmatter(&fm), Some(76));
    assert!(is_pre_merge(&fm));
    // Inner block is preserved verbatim in the body.
    assert!(
        body.contains("priority: P0"),
        "inner FM became body, got: {body}"
    );
    assert!(body.contains("# PRD-076: Body content here."));
}

/// Even on double frontmatter, `refs_form_from_body` must surface the
/// canonical slug from the FIRST block — not anything from the inner
/// block.
#[test]
fn double_frontmatter_refs_form_uses_first_slug() {
    let content = "\
---
slug: prd-canonical
assigned_number: null
---

---
slug: prd-stale-do-not-use
assigned_number: 999
---

Body.
";
    assert_eq!(refs_form_from_body(content, "PRD-074"), "prd-canonical");
}

// ---------------------------------------------------------------------------
// Group C — Malformed YAML
// ---------------------------------------------------------------------------

/// Unclosed string triggers a clear `serde_yaml` error instead of a
/// panic. Caller treats the artifact as broken и surfaces the error.
#[test]
fn malformed_yaml_unclosed_string_fails_gracefully() {
    let content = "---\nid: PRD-001\ntitle: \"unterminated\n---\n\nBody.\n";
    let result = parse_frontmatter(content);
    assert!(result.is_err(), "unterminated string must fail to parse");
    let err = result.unwrap_err();
    let msg = err.to_string();
    // We don't pin the exact error message (serde_yaml internal), но он
    // должен mention YAML/parse so a debugger can grep it.
    assert!(
        !msg.is_empty(),
        "error must carry a message, got empty string"
    );
}

/// No closing `---` marker at all — clear error.
#[test]
fn malformed_yaml_no_closing_marker() {
    let content = "---\nid: PRD-001\nstatus: draft\nBody never separated by a closing marker.\n";
    let result = parse_frontmatter(content);
    assert!(result.is_err(), "no closing --- must fail");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("closing"),
        "error should mention closing marker"
    );
}

/// Missing opening `---` (e.g. file starts with raw markdown). Clear error.
#[test]
fn malformed_yaml_no_opening_marker() {
    let content = "# PRD-001\n\nNo frontmatter at all.\n";
    let result = parse_frontmatter(content);
    assert!(result.is_err(), "no opening --- must fail");
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        msg.contains("frontmatter") || msg.contains("opening"),
        "error should mention frontmatter or opening, got: {msg}"
    );
}

/// Mixed indent (tabs vs spaces) in YAML — serde_yaml is strict about
/// this, must error rather than silently mis-parse.
#[test]
fn malformed_yaml_mixed_indent_fails() {
    // YAML rule: tabs не allowed for indentation внутри mappings.
    // Construct a value that *requires* indentation (nested mapping)
    // и use a tab inside — serde_yaml rejects.
    let content = "---\nlinks:\n\t- target: PRD-002\n---\n\nBody.\n";
    let result = parse_frontmatter(content);
    assert!(result.is_err(), "tab indent in YAML must fail");
}

// ---------------------------------------------------------------------------
// Group D — Long titles + slug truncation
// ---------------------------------------------------------------------------

/// Title >200 chars must produce a slug ≤ MAX_SLUG_LEN (80). SPEC-005
/// truncation rule: cut at the last hyphen before max_title_len.
#[test]
fn very_long_title_truncates_to_max_slug_len() {
    let long_title = "Very long title ".repeat(20); // > 200 chars
    assert!(long_title.len() > 200);
    let slug =
        slug_from_kind_title(&ArtifactKind::Prd, &long_title).expect("must produce valid slug");
    assert!(
        slug.len() <= MAX_SLUG_LEN,
        "slug {} chars (max {})",
        slug.len(),
        MAX_SLUG_LEN
    );
    assert!(
        slug.starts_with("prd-"),
        "slug must keep kind prefix, got {slug}"
    );
    // Truncated slug must still validate.
    validate_slug(&slug).expect("truncated slug must validate");
}

/// Edge: title at exactly MAX_SLUG_LEN bytes. Should produce valid slug
/// without truncation pain.
#[test]
fn title_at_max_slug_boundary() {
    // Suffix "-a" repeated to fill, kind prefix `prd-` is 4 chars,
    // so title length budget is MAX_SLUG_LEN - 4 = 76 chars.
    let title = "x".repeat(MAX_SLUG_LEN); // way bigger than allowed; will truncate
    let slug = slug_from_kind_title(&ArtifactKind::Prd, &title).unwrap();
    assert!(slug.len() <= MAX_SLUG_LEN);
    validate_slug(&slug).expect("boundary slug must validate");
}

// ---------------------------------------------------------------------------
// Group E — Unicode titles
// ---------------------------------------------------------------------------

/// Cyrillic title — `slugify` strips non-ASCII, so the all-Cyrillic title
/// produces an empty slug suffix → error from `slug_from_kind_title`.
/// This is correct behaviour: caller must supply a transliteration или
/// fall back на manual slug.
#[test]
fn cyrillic_only_title_errors_with_clear_message() {
    let title = "Авторизация системы";
    let result = slug_from_kind_title(&ArtifactKind::Prd, title);
    assert!(result.is_err(), "all-Cyrillic title must fail");
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        msg.contains("empty") || msg.contains("non-ascii") || msg.contains("non ascii"),
        "error must explain why, got: {msg}"
    );
}

/// Mixed Cyrillic + ASCII produces a slug from the ASCII portion.
#[test]
fn mixed_cyrillic_ascii_keeps_ascii() {
    let title = "Auth Авторизация System";
    let slug = slug_from_kind_title(&ArtifactKind::Prd, title).unwrap();
    assert_eq!(slug, "prd-auth-system");
}

/// Emoji title — emoji are non-ASCII, stripped to dashes. If only emoji,
/// slug is empty → error.
#[test]
fn emoji_only_title_errors() {
    let title = "🎉🚀✨";
    let result = slug_from_kind_title(&ArtifactKind::Prd, title);
    assert!(result.is_err(), "emoji-only title must fail");
}

/// Emoji + ASCII keeps the ASCII portion.
#[test]
fn emoji_with_ascii_strips_emoji() {
    let title = "Launch 🚀 system";
    let slug = slug_from_kind_title(&ArtifactKind::Prd, title).unwrap();
    assert_eq!(slug, "prd-launch-system");
}

/// Arabic (RTL) title — same non-ASCII behaviour, errors на all-RTL.
#[test]
fn rtl_arabic_only_title_errors() {
    let title = "نظام المصادقة";
    let result = slug_from_kind_title(&ArtifactKind::Prd, title);
    assert!(result.is_err(), "all-Arabic title must fail");
}

/// CJK title — same as other non-ASCII.
#[test]
fn cjk_only_title_errors() {
    let title = "認証システム";
    let result = slug_from_kind_title(&ArtifactKind::Prd, title);
    assert!(result.is_err(), "all-CJK title must fail");
}

// ---------------------------------------------------------------------------
// Group F — Invalid slug formats (Rule 3 territory)
// ---------------------------------------------------------------------------

/// Uppercase slug rejected (validate_slug). Validator script categorises
/// this as a Rule 3 warning, not an error — но `validate_slug` itself
/// returns Err, so callers wishing to enforce strict can.
#[test]
fn uppercase_slug_rejected() {
    assert!(validate_slug("PRD-Auth-System").is_err());
    assert!(validate_slug("prd-Auth").is_err());
}

/// Underscores not allowed (only `-` is a separator).
#[test]
fn underscore_slug_rejected() {
    assert!(validate_slug("prd_auth_system").is_err());
    assert!(validate_slug("prd-auth_system").is_err());
}

/// Spaces not allowed.
#[test]
fn space_slug_rejected() {
    assert!(validate_slug("prd auth system").is_err());
    assert!(validate_slug("prd-auth system").is_err());
}

/// Slash not allowed (path traversal defence).
#[test]
fn slash_slug_rejected() {
    assert!(validate_slug("prd-../etc/passwd").is_err());
    assert!(validate_slug("prd-foo/bar").is_err());
}

/// Below MIN_SLUG_LEN rejected.
#[test]
fn too_short_slug_rejected() {
    let _ = MIN_SLUG_LEN; // sanity reference
    assert!(validate_slug("p").is_err());
    assert!(validate_slug("ab").is_err());
}

// ---------------------------------------------------------------------------
// Group G — assigned_number null variants
// ---------------------------------------------------------------------------

/// `assigned_number: null` (explicit) → None.
#[test]
fn assigned_number_explicit_null() {
    let content = "---\nslug: prd-foo\nassigned_number: null\n---\n\nBody.\n";
    let (fm, _) = parse_frontmatter(content).unwrap();
    assert_eq!(assigned_number_from_frontmatter(&fm), None);
    assert!(is_pre_merge(&fm));
}

/// `assigned_number: ~` (YAML null tilde shorthand) → None.
#[test]
fn assigned_number_tilde_null() {
    let content = "---\nslug: prd-foo\nassigned_number: ~\n---\n\nBody.\n";
    let (fm, _) = parse_frontmatter(content).unwrap();
    assert_eq!(assigned_number_from_frontmatter(&fm), None);
    assert!(is_pre_merge(&fm));
}

/// `assigned_number:` (empty value) → null in YAML → None.
#[test]
fn assigned_number_empty_value_treated_as_null() {
    let content = "---\nslug: prd-foo\nassigned_number:\n---\n\nBody.\n";
    let (fm, _) = parse_frontmatter(content).unwrap();
    assert_eq!(assigned_number_from_frontmatter(&fm), None);
    assert!(is_pre_merge(&fm));
}

/// Field absent entirely → None.
#[test]
fn assigned_number_field_absent() {
    let content = "---\nslug: prd-foo\n---\n\nBody.\n";
    let (fm, _) = parse_frontmatter(content).unwrap();
    assert_eq!(assigned_number_from_frontmatter(&fm), None);
    assert!(is_pre_merge(&fm));
}

// ---------------------------------------------------------------------------
// Group H — predicted_number / assigned_number boundary values
// ---------------------------------------------------------------------------

/// `predicted_number: 0` is below the schema floor (1+) — the validator
/// script catches this в Rule 1, но `predicted_number_from_frontmatter`
/// still returns Some(0). We simply document the contract: the parser is
/// permissive, the validator script is the gate. Add explicit assertion.
#[test]
fn predicted_number_zero_parses_as_zero() {
    let content = "---\nslug: prd-foo\npredicted_number: 0\n---\n\nBody.\n";
    let (fm, _) = parse_frontmatter(content).unwrap();
    // Parser passes through — bounds checked by validator script Rule 1.
    assert_eq!(predicted_number_from_frontmatter(&fm), Some(0));
}

/// Negative `predicted_number` → not a u64 → None (parser rejects).
#[test]
fn predicted_number_negative_returns_none() {
    let content = "---\nslug: prd-foo\npredicted_number: -1\n---\n\nBody.\n";
    let (fm, _) = parse_frontmatter(content).unwrap();
    assert_eq!(predicted_number_from_frontmatter(&fm), None);
}

/// Above MAX_ARTIFACT_NUMBER → None (CWE-1284 defence).
#[test]
fn predicted_number_above_max_returns_none() {
    let content = format!(
        "---\nslug: prd-foo\npredicted_number: {}\n---\n\nBody.\n",
        MAX_ARTIFACT_NUMBER + 1
    );
    let (fm, _) = parse_frontmatter(&content).unwrap();
    assert_eq!(predicted_number_from_frontmatter(&fm), None);
}

/// `u32::MAX` for assigned_number must be rejected (CWE-1284). Display
/// id `PRD-4294967295` is non-sensical.
#[test]
fn assigned_number_u32_max_returns_none() {
    let content = "---\nslug: prd-foo\nassigned_number: 4294967295\n---\n\nBody.\n";
    let (fm, _) = parse_frontmatter(content).unwrap();
    assert_eq!(assigned_number_from_frontmatter(&fm), None);
}

// ---------------------------------------------------------------------------
// Group I — Empty body
// ---------------------------------------------------------------------------

/// Frontmatter only, no body content. Must not panic; body comes back
/// empty.
#[test]
fn empty_body_parses_cleanly() {
    let content = "---\nslug: prd-foo\nassigned_number: null\n---\n";
    let (fm, body) = parse_frontmatter(content).expect("must parse");
    assert_eq!(slug_from_frontmatter(&fm), Some("prd-foo"));
    assert!(body.is_empty(), "body must be empty, got: {body:?}");
}

/// Frontmatter + trailing newline only.
#[test]
fn empty_body_with_trailing_newline_only() {
    let content = "---\nslug: prd-foo\n---\n\n";
    let (_fm, body) = parse_frontmatter(content).expect("must parse");
    assert!(
        body.trim().is_empty(),
        "body must be effectively empty, got: {body:?}"
    );
}

// ---------------------------------------------------------------------------
// Group J — Augment + set_assigned_number on edge cases
// ---------------------------------------------------------------------------

/// Augment must work even on a minimal frontmatter (just `id:`). Field
/// set is added; existing fields preserved.
#[test]
fn augment_works_on_minimal_frontmatter() {
    let content = "---\nid: PRD-074\n---\n\nBody.\n";
    let augmented =
        augment_frontmatter_with_id_fields(content, "prd-foo", 74).expect("augment must succeed");
    let (fm, body) = parse_frontmatter(&augmented).unwrap();
    assert_eq!(slug_from_frontmatter(&fm), Some("prd-foo"));
    assert_eq!(predicted_number_from_frontmatter(&fm), Some(74));
    assert_eq!(assigned_number_from_frontmatter(&fm), Some(74));
    // Existing field preserved.
    assert_eq!(fm.get("id").and_then(|v| v.as_str()), Some("PRD-074"));
    assert!(body.contains("Body."));
}

/// `set_assigned_number` must reject non-null I-2 violation regardless of
/// whether the field was a number or string.
#[test]
fn set_assigned_number_rejects_string_non_null() {
    // Even if frontmatter has a string-typed assigned_number (corruption),
    // I-2 must reject.
    let content = "---\nslug: prd-foo\nassigned_number: \"73\"\n---\n\nBody.\n";
    let result = set_assigned_number(content, 74);
    assert!(
        result.is_err(),
        "non-null assigned_number must trigger I-2 even when string-typed"
    );
}

/// Empty input — parser fails cleanly, no panic.
#[test]
fn empty_input_fails_cleanly() {
    let result = parse_frontmatter("");
    assert!(result.is_err());
    let result_set = set_assigned_number("", 1);
    assert!(result_set.is_err());
}

/// Whitespace-only input — fails cleanly.
#[test]
fn whitespace_only_input_fails_cleanly() {
    let result = parse_frontmatter("   \n\n  \t\n");
    assert!(result.is_err());
}
