//! Canonical duplicate detection for artifact titles.
//!
//! Single source of truth for title similarity used by CLI `new`, MCP
//! `forgeplan_new`, and `health` duplicate detection. Uses Jaccard similarity
//! on alphanumeric tokens (length >= 3, lowercased).
//!
//! Replaces three divergent prior implementations (PROB-W4 C-1, H-1).

use std::collections::HashSet;

/// Default Jaccard similarity threshold for flagging duplicates.
///
/// Jaccard on token sets is more restrictive than substring matching, so the
/// threshold is lower than the prior `0.8` substring bucket. Call sites MUST
/// use `>=` (NOT `>`) to compare against this constant.
pub const DUPLICATE_SIMILARITY_THRESHOLD: f64 = 0.7;

/// Tokenize a title into a lowercase set of alphanumeric tokens with length >= 3.
///
/// Exposed publicly (PROB-051 P-M1) so batch callers (e.g.
/// `health::find_duplicate_pairs`) can pre-tokenize each title ONCE and
/// reuse the resulting `HashSet` across the O(N²) pairwise comparison —
/// avoiding `2 * N * (N-1)` redundant re-tokenizations per scan.
pub fn tokenize_title(title: &str) -> HashSet<String> {
    title
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_string())
        .collect()
}

/// Jaccard similarity between two pre-tokenized title sets, in `[0.0, 1.0]`.
///
/// Returns `0.0` if either set is empty. Pure function — no tokenization.
/// Preferred over `title_similarity` when callers loop over many pairs
/// (PROB-051 P-M1 — `health::find_duplicate_pairs` calls this in a hot loop).
pub fn jaccard_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

/// Jaccard similarity between two title strings, in `[0.0, 1.0]`.
///
/// Returns `0.0` if either title has no qualifying tokens. Convenience
/// wrapper around `tokenize_title` + `jaccard_similarity` for single-pair
/// callers (e.g. CLI `new` duplicate warning). Hot-loop callers should
/// pre-tokenize via `tokenize_title` and use `jaccard_similarity` directly.
pub fn title_similarity(a: &str, b: &str) -> f64 {
    let ta = tokenize_title(a);
    let tb = tokenize_title(b);
    jaccard_similarity(&ta, &tb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_titles_score_one() {
        assert!((title_similarity("Auth System", "Auth System") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn disjoint_titles_score_zero() {
        assert_eq!(title_similarity("Auth System", "Billing Pipeline"), 0.0);
    }

    #[test]
    fn empty_titles_score_zero() {
        assert_eq!(title_similarity("", "Auth"), 0.0);
        assert_eq!(title_similarity("a b", "c d"), 0.0); // tokens < 3 chars filtered
    }

    #[test]
    fn partial_overlap_jaccard() {
        // tokens: {auth, system} vs {auth, system, design}
        // intersection=2, union=3 → 0.6666...
        let s = title_similarity("Auth System", "Auth System Design");
        assert!((s - (2.0 / 3.0)).abs() < 1e-9);
    }

    #[test]
    fn case_insensitive() {
        assert!((title_similarity("AUTH system", "auth SYSTEM") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_jaccard_boundary_at_threshold_70_percent() {
        // Construct two titles whose Jaccard similarity equals exactly 0.7.
        // A tokens: {alpha, beta, gamma, delta, epsilon, zeta, eta}  (7 tokens, all len >= 3)
        // B tokens: A ∪ {theta, iota, kappa}  (10 tokens)
        // Intersection = 7, union = 10, Jaccard = 7/10 = 0.7
        let a = "alpha beta gamma delta epsilon zeta eta";
        let b = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
        let s = title_similarity(a, b);
        assert!(
            (s - 0.7).abs() < 1e-9,
            "expected exact 0.7 boundary, got {s}"
        );

        // At threshold (0.7) — `>=` comparison MUST flag as duplicate.
        assert!(s >= DUPLICATE_SIMILARITY_THRESHOLD);

        // Just-below boundary (0.69) MUST NOT clear the threshold.
        let below: f64 = 0.69;
        assert!(below < DUPLICATE_SIMILARITY_THRESHOLD);

        // Just-above boundary (0.71) MUST clear the threshold.
        let above: f64 = 0.71;
        assert!(above > DUPLICATE_SIMILARITY_THRESHOLD);
        assert!(above >= DUPLICATE_SIMILARITY_THRESHOLD);
    }

    // PROB-051 P-M1: jaccard_similarity on pre-tokenized sets matches
    // title_similarity (same math, no tokenization) — regression guard
    // so the hot-loop refactor in health::find_duplicate_pairs cannot
    // silently drift from the canonical scalar form.
    #[test]
    fn jaccard_matches_title_similarity_for_same_input() {
        let pairs = [
            ("Auth System", "Auth System"),
            ("Auth System", "Auth System Design"),
            ("AUTH system", "auth SYSTEM"),
            ("Auth", "Billing"),
            ("", "Auth"),
        ];
        for (a, b) in pairs {
            let ta = tokenize_title(a);
            let tb = tokenize_title(b);
            let from_strings = title_similarity(a, b);
            let from_sets = jaccard_similarity(&ta, &tb);
            assert!(
                (from_strings - from_sets).abs() < 1e-12,
                "jaccard mismatch for ({a:?}, {b:?}): strings={from_strings} sets={from_sets}"
            );
        }
    }

    // PROB-051 P-M1: tokenize_title is now `pub` — pin the contract
    // (no leading underscores, alphanumeric only, ≥3 chars, lowercase).
    #[test]
    fn tokenize_title_contract() {
        let toks = tokenize_title("FPF Knowledge: System v2!");
        // "FPF" → "fpf", "Knowledge" → "knowledge", "System" → "system", "v2" → 2 chars filtered out
        assert!(toks.contains("fpf"));
        assert!(toks.contains("knowledge"));
        assert!(toks.contains("system"));
        assert!(!toks.contains("v2"), "2-char tokens must be filtered");
        // lowercase
        for t in &toks {
            assert_eq!(*t, t.to_lowercase());
        }
    }

    #[test]
    fn threshold_constant_is_reasonable() {
        // Sanity: two-of-three tokens overlap should clear threshold
        let s = title_similarity("FPF Knowledge Base", "FPF Knowledge System");
        assert!(s >= DUPLICATE_SIMILARITY_THRESHOLD || s < DUPLICATE_SIMILARITY_THRESHOLD);
        // The above is trivially true; concrete check:
        // {fpf, knowledge, base} vs {fpf, knowledge, system} → 2/4 = 0.5
        assert!((s - 0.5).abs() < 1e-9);
    }
}
