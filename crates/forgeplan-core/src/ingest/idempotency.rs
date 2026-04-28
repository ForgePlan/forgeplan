//! Idempotency support for ingest re-runs (PRD-066 FR-5, AC-3).
//!
//! Each generated artifact carries a stable `source_hash` derived from
//! `(rule_id, full_source_text)`. On re-run the engine compares the new hash
//! against the existing one extracted from the artifact body to decide whether
//! to skip, create, or update.
//!
//! The hash is embedded in the artifact body as an HTML comment so it survives
//! markdown round-trips without being rendered:
//!
//! ```text
//! <!-- source_hash: 6c1eaab… -->
//! ```
//!
//! See [SPEC-004 §`source_hash`](../../../../.forgeplan/specs/SPEC-004-mapping-yaml-schema.md)
//! for the contract.

use sha2::{Digest, Sha256};

use super::sources::ParsedSource;

/// Decision returned by [`artifact_needs_update`].
///
/// `#[non_exhaustive]` so future idempotency policies (`Merge`, `Diff`)
/// can be added without breaking downstream `match` arms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UpdateDecision {
    /// No existing artifact — create it.
    Create,
    /// Existing artifact's hash differs — update it.
    Update,
    /// Existing artifact's hash matches — no-op.
    Skip,
}

/// Compute the canonical idempotency hash for a `(rule, source)` pair.
///
/// Uses sha256 over `"{rule_id}\n{full_text}"` and returns the hex digest. The
/// rule id is included so two different rules pointing at the same source
/// document still produce distinct artifacts.
///
/// The hash is deliberately **not** computed over rendered output: that would
/// make every template tweak look like a content change. Source-level hashing
/// keeps re-runs stable across cosmetic mapping edits.
pub fn compute_source_hash(parsed: &ParsedSource, rule_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(rule_id.as_bytes());
    hasher.update(b"\n");
    hasher.update(parsed.full_text.as_bytes());
    hex_encode(&hasher.finalize())
}

/// Compare two hashes and decide what to do on apply.
///
/// `existing_hash` should be the value extracted from the existing artifact via
/// [`extract_existing_source_hash`]; `None` means no artifact yet.
pub fn artifact_needs_update(existing_hash: Option<&str>, new_hash: &str) -> UpdateDecision {
    match existing_hash {
        None => UpdateDecision::Create,
        Some(prev) if prev == new_hash => UpdateDecision::Skip,
        Some(_) => UpdateDecision::Update,
    }
}

/// Extract the previously-stored source hash from an artifact body.
///
/// Looks for the marker `<!-- source_hash: <hex> -->`. Whitespace inside the
/// comment is tolerated; if the marker is missing or malformed the function
/// returns `None` (callers treat that as "create new artifact").
pub fn extract_existing_source_hash(artifact_body: &str) -> Option<String> {
    for line in artifact_body.lines() {
        let trimmed = line.trim();
        // Cheap filter before more expensive parsing.
        if !trimmed.starts_with("<!--") {
            continue;
        }
        // Strip the comment delimiters.
        let inner = trimmed
            .trim_start_matches("<!--")
            .trim_end_matches("-->")
            .trim();
        // Expect "source_hash: <hex>".
        let rest = match inner.strip_prefix("source_hash:") {
            Some(r) => r.trim(),
            None => continue,
        };
        if rest.is_empty() {
            continue;
        }
        // Defensive: keep only the leading hex token, drop any trailing junk.
        let hex_token: String = rest.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        if !hex_token.is_empty() {
            return Some(hex_token);
        }
    }
    None
}

/// Render the source-hash marker that should be embedded in artifact bodies.
///
/// Convenience helper used by the engine when assembling drafts.
pub fn render_source_hash_marker(hash: &str) -> String {
    format!("<!-- source_hash: {hash} -->")
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::sources::ParsedSource;
    use serde_json::json;
    use std::collections::HashMap;

    fn make_parsed(text: &str) -> ParsedSource {
        ParsedSource {
            path: "test.md".to_owned(),
            front_matter: json!({}),
            sections: HashMap::new(),
            full_text: text.to_owned(),
            line_count: text.lines().count(),
        }
    }

    #[test]
    fn same_input_same_hash() {
        let p = make_parsed("hello world");
        let a = compute_source_hash(&p, "rule-1");
        let b = compute_source_hash(&p, "rule-1");
        assert_eq!(a, b);
        // sha256 hex digest is 64 chars.
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn different_rule_id_changes_hash() {
        let p = make_parsed("hello world");
        let a = compute_source_hash(&p, "rule-1");
        let b = compute_source_hash(&p, "rule-2");
        assert_ne!(a, b);
    }

    #[test]
    fn different_text_changes_hash() {
        let a = compute_source_hash(&make_parsed("hello world"), "r");
        let b = compute_source_hash(&make_parsed("hello world!"), "r");
        assert_ne!(a, b);
    }

    #[test]
    fn update_decision_matrix() {
        assert_eq!(artifact_needs_update(None, "abc"), UpdateDecision::Create);
        assert_eq!(
            artifact_needs_update(Some("abc"), "abc"),
            UpdateDecision::Skip
        );
        assert_eq!(
            artifact_needs_update(Some("abc"), "xyz"),
            UpdateDecision::Update
        );
    }

    #[test]
    fn extract_marker_round_trips() {
        let marker = render_source_hash_marker("deadbeef");
        let body = format!("# Title\n\n{marker}\n\nbody text\n");
        assert_eq!(
            extract_existing_source_hash(&body).as_deref(),
            Some("deadbeef")
        );
    }

    #[test]
    fn extract_marker_missing_returns_none() {
        assert!(extract_existing_source_hash("# Title\n\nno hash here\n").is_none());
        assert!(extract_existing_source_hash("").is_none());
    }

    #[test]
    fn extract_marker_handles_extra_whitespace() {
        let body = "<!--   source_hash:    cafe1234   -->\n";
        assert_eq!(
            extract_existing_source_hash(body).as_deref(),
            Some("cafe1234")
        );
    }

    #[test]
    fn extract_marker_rejects_non_hex_payload() {
        let body = "<!-- source_hash: not-a-hash -->\n";
        // "not" starts with non-hex chars, so leading hex token is empty → None.
        assert!(extract_existing_source_hash(body).is_none());
    }
}
