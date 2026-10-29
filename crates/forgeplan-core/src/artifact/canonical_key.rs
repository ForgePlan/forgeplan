//! Canonical artifact identity: the permanent key, the display alias, and the
//! single resolution algorithm both are read through.
//!
//! ADR-030 makes the **canonical key** the authoritative identity of a
//! document and the display id (`PRD-074`) a repeatable alias. SPEC-009
//! freezes the key format and the resolution order:
//!
//! 1. an empty or unusable selector is rejected;
//! 2. an exact canonical-key match wins — and several files claiming one key
//!    are a conflict, never a winner;
//! 3. otherwise the display id is normalised and **all** matches are
//!    collected: one is the result, several are an ambiguity, none is "not
//!    found";
//! 4. a numeric-shaped canonical key does not take priority over a competing
//!    display alias of the same shape.
//!
//! Nothing here selects by filename prefix, by array order, by the most
//! recent file, by a database row, or by title similarity. Those are exactly
//! the rules that let two documents answer to one name.
//!
//! The key itself is minted from the operating-system random source only
//! (SPEC-009 "Key independence from mutable and ambient data"): never from
//! the timestamp, the display number, the process id, the path, or a hash of
//! the content. Uniqueness is therefore probabilistic at 128 bits, which the
//! contract accepts.

use std::path::PathBuf;

use crate::artifact::frontmatter::{parse_frontmatter, slug_from_frontmatter};
use crate::artifact::types::{ArtifactKind, MAX_SLUG_LEN, slugify, validate_slug};

/// Random bytes behind the key's hexadecimal suffix.
const KEY_ENTROPY_BYTES: usize = 16;

/// Hexadecimal characters in the rendered suffix — two per random byte.
const KEY_HEX_LEN: usize = KEY_ENTROPY_BYTES * 2;

/// Fixed characters a minted key spends on separators and the `k` marker:
/// one dash after the kind, `-k` before the suffix.
const KEY_FIXED_OVERHEAD: usize = 1 + 2;

/// Title part used when the title slugifies to nothing — an all-Cyrillic or
/// all-CJK title produces an empty ASCII slug.
const EMPTY_TITLE_FALLBACK: &str = "document";

/// The permanent identity of a document: its canonical key and its display
/// alias.
///
/// The key is the identity; the display id is an alias that may repeat inside
/// a workspace and across projects. Callers that must name exactly one
/// document use the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactIdentity {
    /// The permanent key: `<kind>-<title-part>-k<32 lowercase hex>` for a
    /// minted document, the ADR-012 slug for a preserved one, the display id
    /// for a legacy document that carries no slug.
    pub canonical_key: String,
    /// The human-facing alias: `PRD-074`.
    pub display_id: String,
}

/// One document that a selector resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedArtifact {
    /// The identity of the resolved document.
    pub identity: ArtifactIdentity,
    /// Workspace-relative path of the resolved document. Never absolute.
    pub relative_path: PathBuf,
}

/// One document that a selector may have matched.
///
/// Candidates are what an ambiguity or a conflict names, so a caller can show
/// the operator *which* documents were in play without exposing absolute
/// paths or another source's data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactCandidate {
    /// The document's canonical key.
    pub canonical_key: String,
    /// The document's display alias.
    pub display_id: String,
    /// The document's title.
    pub title: String,
    /// Workspace-relative path of the document.
    pub relative_path: PathBuf,
}

/// Why a selector did not name exactly one document.
///
/// The four outcomes are deliberately distinct: "not found", "ambiguous",
/// "conflicting", and "the selector is not a selector" are different facts,
/// and a caller must be able to tell them apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// The selector matched more than one document by display alias.
    Ambiguous {
        /// The selector as the caller wrote it.
        input: String,
        /// Every document that matched, sorted by canonical key then path.
        candidates: Vec<ArtifactCandidate>,
    },
    /// More than one document claims the same canonical key.
    ConflictingCanonicalKey {
        /// The contested key.
        key: String,
        /// Every document claiming it, sorted by canonical key then path.
        candidates: Vec<ArtifactCandidate>,
    },
    /// The selector is empty or is not a usable reference.
    InvalidSelector {
        /// The selector as the caller wrote it.
        input: String,
    },
    /// The document set could not be fully parsed, so any answer computed over
    /// it would be partial.
    IncompleteCapture {
        /// What was missing from the document set.
        message: String,
    },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ambiguous { input, candidates } => write!(
                f,
                "selector {input:?} matches {} documents; address one by its canonical key",
                candidates.len()
            ),
            Self::ConflictingCanonicalKey { key, candidates } => write!(
                f,
                "canonical key {key:?} is claimed by {} documents",
                candidates.len()
            ),
            Self::InvalidSelector { input } => {
                write!(f, "selector {input:?} is not a valid artifact reference")
            }
            Self::IncompleteCapture { message } => {
                write!(f, "document set is incomplete: {message}")
            }
        }
    }
}

impl std::error::Error for ResolveError {}

/// Mint the canonical key of a new document.
///
/// The key is `<kind>-<title-part>-k<32 lowercase hex>`:
///
/// * `<kind>` is the artifact kind's slug prefix (`prd`, `evid`, `note`, …) —
///   the same prefix the slug grammar accepts, so the key is a valid slug and
///   can live in the `slug` frontmatter field;
/// * `<title-part>` is the slugified title, truncated at a hyphen boundary so
///   the whole key stays within [`MAX_SLUG_LEN`]; an all-Cyrillic title
///   slugifies to nothing and becomes `document`;
/// * `k<32 lowercase hex>` is 16 bytes from the operating-system random
///   source. The `k` marker keeps the key from ever being numeric-only.
///
/// The key never derives from the timestamp, the display number, the process
/// id, the path, or a content hash, so editing a document never changes its
/// identity. A failure of the entropy source fails the whole call: no key is
/// returned, so no file, index row, relation, or claim can be written under a
/// half-minted identity.
pub fn mint_artifact_key(kind: &ArtifactKind, title: &str) -> anyhow::Result<String> {
    mint_artifact_key_with_entropy(kind, title, fill_from_operating_system)
}

/// The canonical key of a document that carries no minted key.
///
/// ADR-030's legacy rule: a document keeps the canonical key it already has
/// — the ADR-012 `slug:` when the frontmatter declares a valid one, and its
/// display id when it declares none. The constellation capture derives its
/// canonical key by exactly this rule, so a record built here and a document
/// read from the same workspace agree on one identity.
///
/// `document` is the document's markdown text — the whole file, frontmatter
/// included, which is how the store keeps it. A document without a parseable
/// frontmatter keeps its display id: it declares no slug, so it has no other
/// identity.
pub fn canonical_key_from_document(display_id: &str, document: &str) -> String {
    // The cheap test first: a body without a `slug:` key cannot name one, and
    // most documents carry no slug at all. Skipping the YAML parse there keeps
    // the per-row cost of reading an index proportional to the bytes, not to
    // the number of documents.
    if !document.contains("slug:") {
        return display_id.to_string();
    }
    match parse_frontmatter(document) {
        Ok((frontmatter, _body)) => match slug_from_frontmatter(&frontmatter) {
            Some(slug) if validate_slug(slug).is_ok() => slug.to_string(),
            _ => display_id.to_string(),
        },
        Err(_) => display_id.to_string(),
    }
}

/// The minting rule with the entropy source injected.
///
/// The seam exists so the failure path is testable without mocking the
/// operating system: a caller that cannot produce random bytes gets an error
/// instead of a key built from a predictable source.
fn mint_artifact_key_with_entropy<F>(
    kind: &ArtifactKind,
    title: &str,
    fill: F,
) -> anyhow::Result<String>
where
    F: FnOnce(&mut [u8]) -> anyhow::Result<()>,
{
    let kind_prefix = kind.prefix().trim_end_matches('-');

    let mut title_part = slugify(title);
    if title_part.is_empty() {
        title_part = EMPTY_TITLE_FALLBACK.to_string();
    }

    let mut entropy = [0u8; KEY_ENTROPY_BYTES];
    fill(&mut entropy)?;
    let suffix = render_hex_lower(&entropy);

    // Build the key, then let the slug grammar have the last word: a key the
    // grammar rejects is not a key. The only grammar rule a title can break
    // is the reserved-prefix rule (a title starting with `draft-`, `pending-`
    // or `tmp-`), and the established escape for it is the `x-` marker
    // `slug_from_kind_title` uses.
    let mut key = assemble_key(kind_prefix, &title_part, &suffix);
    if let Err(error) = validate_slug(&key) {
        let escaped = format!("x-{title_part}");
        key = assemble_key(kind_prefix, &escaped, &suffix);
        validate_slug(&key).map_err(|second| {
            anyhow::anyhow!(
                "cannot build a canonical key for {kind_prefix:?}: {second} (first attempt: {error})"
            )
        })?;
    }

    Ok(key)
}

/// Compose `<kind>-<title>-k<suffix>`, truncating only the title part.
fn assemble_key(kind_prefix: &str, title_part: &str, suffix: &str) -> String {
    let budget = MAX_SLUG_LEN
        .saturating_sub(kind_prefix.len())
        .saturating_sub(KEY_FIXED_OVERHEAD)
        .saturating_sub(KEY_HEX_LEN);
    let title_part = truncate_at_hyphen(title_part, budget);
    format!("{kind_prefix}-{title_part}-k{suffix}")
}

/// Truncate a title part to `budget` characters, preferring a hyphen
/// boundary.
///
/// `slugify` output is ASCII, so a byte index and a character index are the
/// same number here. A title part with no hyphen inside the window is cut at
/// the window edge: the length limit is a hard rule of the key format, and a
/// single long word has no boundary to cut at.
fn truncate_at_hyphen(part: &str, budget: usize) -> String {
    if part.len() <= budget {
        return part.to_string();
    }
    let window = &part[..budget];
    match window.rfind('-') {
        Some(index) => part[..index].to_string(),
        None => part[..budget].to_string(),
    }
}

/// Render bytes as lowercase hexadecimal, two characters per byte.
fn render_hex_lower(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[usize::from(byte >> 4)] as char);
        out.push(DIGITS[usize::from(byte & 0x0f)] as char);
    }
    out
}

/// Fill a buffer from the operating-system random source.
fn fill_from_operating_system(buffer: &mut [u8]) -> anyhow::Result<()> {
    use rand::TryRngCore;

    let mut source = rand::rngs::OsRng;
    source
        .try_fill_bytes(buffer)
        .map_err(|error| anyhow::anyhow!("operating-system entropy source failed: {error:?}"))
}

/// Resolve a selector against a fully parsed document set.
///
/// This is **the** resolution algorithm (SPEC-009 "One resolution algorithm").
/// The document set is the whole corpus, not a filtered slice: the resolution
/// order depends on knowing every document that carries the name.
///
/// 1. an empty or unusable selector is [`ResolveError::InvalidSelector`];
/// 2. an exact canonical-key match wins. Several files claiming that key are
///    [`ResolveError::ConflictingCanonicalKey`], naming every claimant;
/// 3. otherwise the display id is normalised and **all** matches are
///    collected: exactly one is the result, several are
///    [`ResolveError::Ambiguous`], none is `Ok(None)` — "not found";
/// 4. a numeric-shaped canonical key does not take priority over a competing
///    display alias of the same shape, so a legacy `PRD-001` and another
///    document aliasing `PRD-001` are an ambiguity, not a silent winner.
///
/// Candidates are sorted by canonical key, then by relative path, so the same
/// input always reports the same candidates in the same order.
pub fn resolve_artifact(
    input: &str,
    documents: &[ArtifactCandidate],
) -> Result<Option<ResolvedArtifact>, ResolveError> {
    let selector = input.trim();
    if !is_valid_selector(selector) {
        return Err(ResolveError::InvalidSelector {
            input: input.to_string(),
        });
    }

    let key_matches: Vec<&ArtifactCandidate> = documents
        .iter()
        .filter(|document| document.canonical_key == selector)
        .collect();

    if key_matches.len() > 1 {
        return Err(ResolveError::ConflictingCanonicalKey {
            key: selector.to_string(),
            candidates: sort_candidates(key_matches),
        });
    }

    let alias_matches: Vec<&ArtifactCandidate> = documents
        .iter()
        .filter(|document| display_id_matches(&document.display_id, selector))
        .collect();

    if let Some(matched) = key_matches.first() {
        // Rule 4: a numeric-shaped canonical key — a legacy document whose key
        // *is* its number — does not outrank a document that carries the same
        // shape as its display alias. Both are named; neither wins.
        if is_numeric_shaped(&matched.canonical_key) {
            let competing: Vec<&ArtifactCandidate> = alias_matches
                .iter()
                .copied()
                .filter(|candidate| candidate.canonical_key != matched.canonical_key)
                .collect();
            if !competing.is_empty() {
                let mut all = key_matches.clone();
                all.extend(competing);
                return Err(ResolveError::Ambiguous {
                    input: selector.to_string(),
                    candidates: sort_candidates(all),
                });
            }
        }
        return Ok(Some(resolved(matched)));
    }

    match alias_matches.len() {
        0 => Ok(None),
        1 => Ok(Some(resolved(alias_matches[0]))),
        _ => Err(ResolveError::Ambiguous {
            input: selector.to_string(),
            candidates: sort_candidates(alias_matches),
        }),
    }
}

/// Whether a selector can name a document at all.
///
/// A selector is a display id (`PRD-074`), a canonical key
/// (`prd-weekly-review-k<32 hex>`), or a legacy slug (`prd-auth-system`) —
/// all of them ASCII alphanumerics and hyphens. A qualified reference
/// (`KEY@qualifier`) names a document in *another* source and is therefore
/// not a selector this resolver can honour.
fn is_valid_selector(selector: &str) -> bool {
    if selector.is_empty() {
        return false;
    }
    if selector.contains('@') {
        return false;
    }
    selector
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

/// Whether a document's display id answers to the selector.
///
/// Exact equality always matches. A display-shaped selector (`prd-74`) also
/// matches the normalised form of a display id, so the same document answers
/// to `PRD-074`, `prd-74` and `PRD-74`.
fn display_id_matches(display_id: &str, selector: &str) -> bool {
    if display_id == selector {
        return true;
    }
    match (normalize_display_id(display_id), normalize_display_id(selector)) {
        (Some(display), Some(selector)) => display == selector,
        _ => false,
    }
}

/// Normalise a display id to its canonical `PREFIX-NNN` form.
///
/// Returns `None` for anything that is not a display id — a slug, a literal
/// id (`mem-my-note`), a predicted id (`PRD-74?`) — because those are
/// compared verbatim instead.
fn normalize_display_id(value: &str) -> Option<String> {
    let (prefix, number) = value.split_once('-')?;
    if prefix.is_empty()
        || number.is_empty()
        || !prefix.chars().all(|character| character.is_ascii_alphabetic())
        || !number.chars().all(|character| character.is_ascii_digit())
    {
        return None;
    }
    let kind = ArtifactKind::from_slug_prefix(&prefix.to_ascii_lowercase())?;
    let number: u32 = number.parse().ok()?;
    Some(format!(
        "{}-{:03}",
        kind.prefix().trim_end_matches('-').to_ascii_uppercase(),
        number
    ))
}

/// Whether a key has the shape of a display id — `PRD-074`, `prd-074`.
///
/// Such a key is a legacy document's number standing in for its identity, so
/// it must not outrank a document that carries the same shape as an alias.
fn is_numeric_shaped(key: &str) -> bool {
    normalize_display_id(key).is_some()
}

/// The resolved shape of one matched document.
fn resolved(candidate: &ArtifactCandidate) -> ResolvedArtifact {
    ResolvedArtifact {
        identity: ArtifactIdentity {
            canonical_key: candidate.canonical_key.clone(),
            display_id: candidate.display_id.clone(),
        },
        relative_path: candidate.relative_path.clone(),
    }
}

/// Sort candidates by canonical key, then by relative path.
fn sort_candidates(candidates: Vec<&ArtifactCandidate>) -> Vec<ArtifactCandidate> {
    let mut owned: Vec<ArtifactCandidate> = candidates.into_iter().cloned().collect();
    owned.sort_by(|left, right| {
        left.canonical_key
            .cmp(&right.canonical_key)
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    owned
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn candidate(key: &str, display: &str, title: &str, path: &str) -> ArtifactCandidate {
        ArtifactCandidate {
            canonical_key: key.to_string(),
            display_id: display.to_string(),
            title: title.to_string(),
            relative_path: PathBuf::from(path),
        }
    }

    /// A document with a minted key.
    fn minted(title: &str, path: &str) -> ArtifactCandidate {
        candidate(
            &mint_artifact_key(&ArtifactKind::Prd, title).unwrap(),
            "PRD-001",
            title,
            path,
        )
    }

    #[test]
    fn two_documents_with_the_same_title_get_different_keys() {
        let first = mint_artifact_key(&ArtifactKind::Prd, "Weekly review").unwrap();
        let second = mint_artifact_key(&ArtifactKind::Prd, "Weekly review").unwrap();
        assert_ne!(
            first, second,
            "the key must come from the random source, not from the title"
        );
    }

    #[test]
    fn a_cyrillic_title_falls_back_to_the_word_document() {
        let key = mint_artifact_key(&ArtifactKind::Prd, "Еженедельный обзор").unwrap();
        assert!(
            key.starts_with("prd-document-k"),
            "an all-Cyrillic title must fall back to `document`, got {key:?}"
        );
        assert_eq!(key.len(), "prd-document-k".len() + 32);
    }

    #[test]
    fn a_minted_key_stays_within_the_length_limit() {
        let long_title = "weekly ".repeat(60);
        let key = mint_artifact_key(&ArtifactKind::Prd, &long_title).unwrap();
        assert!(
            key.len() <= MAX_SLUG_LEN,
            "key must stay within {MAX_SLUG_LEN} characters, got {} ({key:?})",
            key.len()
        );
        assert!(
            validate_slug(&key).is_ok(),
            "a minted key must satisfy the slug grammar: {key:?}"
        );
    }

    #[test]
    fn a_minted_key_has_the_frozen_shape() {
        let key = mint_artifact_key(&ArtifactKind::Rfc, "mTLS rollout").unwrap();
        let (kind, rest) = key.split_once('-').unwrap();
        assert_eq!(kind, "rfc");
        let (title, suffix) = rest.rsplit_once("-k").unwrap();
        assert_eq!(title, "mtls-rollout");
        assert_eq!(suffix.len(), 32, "32 hexadecimal characters: {suffix:?}");
        assert!(
            suffix
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "the suffix must be lowercase hexadecimal: {suffix:?}"
        );
        assert!(
            !suffix.chars().all(|c| c.is_ascii_digit()),
            "the `k` marker keeps the key from being numeric-only"
        );
    }

    #[test]
    fn a_title_that_would_use_a_reserved_prefix_still_yields_a_valid_key() {
        let key = mint_artifact_key(&ArtifactKind::Prd, "Draft alternative").unwrap();
        assert!(
            validate_slug(&key).is_ok(),
            "the reserved-prefix rule must not make a document unmintable: {key:?}"
        );
        assert!(key.starts_with("prd-"));
    }

    #[test]
    fn a_failing_entropy_source_fails_the_whole_mint() {
        let error = mint_artifact_key_with_entropy(&ArtifactKind::Prd, "Weekly review", |_| {
            anyhow::bail!("entropy source unavailable")
        })
        .expect_err("a failing entropy source must not produce a key");
        assert!(
            error.to_string().contains("entropy source unavailable"),
            "the failure must name the entropy source, got {error}"
        );
    }

    #[test]
    fn an_exact_key_match_resolves() {
        let document = minted("Weekly review", "prds/prd-weekly-review.md");
        let key = document.canonical_key.clone();
        let resolved = resolve_artifact(&key, std::slice::from_ref(&document))
            .unwrap()
            .expect("the key must resolve");
        assert_eq!(resolved.identity.canonical_key, key);
        assert_eq!(resolved.identity.display_id, "PRD-001");
        assert_eq!(
            resolved.relative_path,
            Path::new("prds/prd-weekly-review.md")
        );
    }

    #[test]
    fn a_lowercase_display_id_resolves_to_the_same_document() {
        let document = candidate(
            "prd-weekly-review-k0123456789abcdef0123456789abcdef",
            "PRD-074",
            "Weekly review",
            "prds/a.md",
        );
        let resolved = resolve_artifact("prd-74", std::slice::from_ref(&document))
            .unwrap()
            .expect("a lowercase display id must resolve");
        assert_eq!(
            resolved.identity.canonical_key,
            "prd-weekly-review-k0123456789abcdef0123456789abcdef"
        );
        assert_eq!(resolved.identity.display_id, "PRD-074");
    }

    #[test]
    fn a_reference_that_names_no_document_is_not_found() {
        let document = minted("Weekly review", "prds/a.md");
        assert!(
            resolve_artifact("PRD-999", std::slice::from_ref(&document))
                .unwrap()
                .is_none(),
            "an absent document is `Ok(None)`, not an error"
        );
    }

    #[test]
    fn an_empty_or_unusable_selector_is_rejected() {
        let document = minted("Weekly review", "prds/a.md");
        for input in ["", "   ", "prd 001", "PRD-001@other-source", "../etc/passwd"] {
            let error = resolve_artifact(input, std::slice::from_ref(&document))
                .expect_err("an unusable selector must be rejected");
            assert!(
                matches!(error, ResolveError::InvalidSelector { .. }),
                "expected InvalidSelector for {input:?}, got {error:?}"
            );
        }
    }

    #[test]
    fn two_documents_sharing_a_display_id_are_ambiguous() {
        let first = candidate(
            "prd-first-k0123456789abcdef0123456789abcdef",
            "PRD-074",
            "First",
            "prds/first.md",
        );
        let second = candidate(
            "prd-second-k0123456789abcdef0123456789abcdef",
            "PRD-074",
            "Second",
            "prds/second.md",
        );
        let error = resolve_artifact("PRD-074", &[first.clone(), second.clone()])
            .expect_err("a shared display id must not pick a winner");
        match error {
            ResolveError::Ambiguous { input, candidates } => {
                assert_eq!(input, "PRD-074");
                assert_eq!(candidates.len(), 2, "both documents must be named");
                assert_eq!(candidates[0].canonical_key, first.canonical_key);
                assert_eq!(candidates[1].canonical_key, second.canonical_key);
                assert_eq!(candidates[0].title, "First");
                assert_eq!(candidates[1].title, "Second");
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
    }

    #[test]
    fn two_files_claiming_one_canonical_key_are_a_conflict() {
        let first = candidate(
            "prd-shared-k0123456789abcdef0123456789abcdef",
            "PRD-001",
            "First",
            "prds/first.md",
        );
        let second = candidate(
            "prd-shared-k0123456789abcdef0123456789abcdef",
            "PRD-002",
            "Second",
            "prds/second.md",
        );
        let key = "prd-shared-k0123456789abcdef0123456789abcdef";
        let error = resolve_artifact(key, &[first.clone(), second.clone()])
            .expect_err("two claimants of one key must not produce a winner");
        match error {
            ResolveError::ConflictingCanonicalKey { key: contested, candidates } => {
                assert_eq!(contested, key);
                assert_eq!(candidates.len(), 2, "every claimant must be named");
                assert_eq!(candidates[0].relative_path, Path::new("prds/first.md"));
                assert_eq!(candidates[1].relative_path, Path::new("prds/second.md"));
            }
            other => panic!("expected ConflictingCanonicalKey, got {other:?}"),
        }
    }

    #[test]
    fn a_numeric_legacy_key_does_not_shadow_a_display_alias() {
        // A legacy document without a slug keeps its number as its key; a
        // second document carries the same number as its display alias.
        let legacy = candidate("PRD-001", "PRD-001", "Legacy", "prds/legacy.md");
        let alias_holder = candidate(
            "prd-holder-k0123456789abcdef0123456789abcdef",
            "PRD-001",
            "Holder",
            "prds/holder.md",
        );
        let error = resolve_artifact("PRD-001", &[legacy.clone(), alias_holder.clone()])
            .expect_err("a numeric key must not outrank a competing display alias");
        match error {
            ResolveError::Ambiguous { candidates, .. } => {
                assert_eq!(candidates.len(), 2, "both documents must be named");
                let keys: Vec<&str> = candidates
                    .iter()
                    .map(|candidate| candidate.canonical_key.as_str())
                    .collect();
                assert!(keys.contains(&"PRD-001"));
                assert!(keys.contains(&"prd-holder-k0123456789abcdef0123456789abcdef"));
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
    }

    #[test]
    fn candidates_are_reported_in_a_stable_order() {
        // Both documents carry the SAME display alias, so the alias names more
        // than one document and the resolver must report both — sorted by
        // canonical key, regardless of the order they arrived in.
        let second = candidate(
            "prd-b-k0123456789abcdef0123456789abcdef",
            "PRD-001",
            "B",
            "prds/b.md",
        );
        let first = candidate(
            "prd-a-k0123456789abcdef0123456789abcdef",
            "PRD-001",
            "A",
            "prds/a.md",
        );
        let error = resolve_artifact("PRD-001", &[second, first]).expect_err("shared alias");
        match error {
            ResolveError::Ambiguous { candidates, .. } => {
                assert_eq!(
                    candidates[0].canonical_key,
                    "prd-a-k0123456789abcdef0123456789abcdef"
                );
                assert_eq!(
                    candidates[1].canonical_key,
                    "prd-b-k0123456789abcdef0123456789abcdef"
                );
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
    }

    #[test]
    fn a_document_with_a_slug_keeps_it_as_its_canonical_key() {
        let document = "---\nid: PRD-001\nkind: prd\nslug: prd-auth-system\n---\n\nBody.\n";
        assert_eq!(
            canonical_key_from_document("PRD-001", document),
            "prd-auth-system"
        );
    }

    #[test]
    fn a_legacy_document_without_a_slug_keeps_its_display_id() {
        let document = "---\nid: PRD-002\nkind: prd\n---\n\nBody.\n";
        assert_eq!(canonical_key_from_document("PRD-002", document), "PRD-002");
    }

    #[test]
    fn a_document_with_an_unparsable_frontmatter_keeps_its_display_id() {
        assert_eq!(
            canonical_key_from_document("PRD-003", "no frontmatter here"),
            "PRD-003"
        );
    }
}
