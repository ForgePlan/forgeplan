//! Smart search: combines BM25 keyword, semantic, and graph signals.
//!
//! Scoring model: text-first + boosters.
//! base_score = max(bm25_normalized, semantic_score)
//! boost = 1.0 + (r_eff * 0.2) + (is_active * 0.1) + (graph_centrality * 0.1)
//! final_score = base_score * boost
//!
//! After ranking, top results are optionally expanded with 1-hop graph
//! neighbors (FR-003, PRD-039) using a configurable decay factor.
//!
//! If embeddings are unavailable, gracefully degrades to keyword-only.

use crate::db::store::ArtifactRecord;
use crate::graph::knowledge::KnowledgeGraph;
use crate::search::bm25::{Bm25Index, strip_indexing_noise};
use crate::search::filter::ArtifactFilter;
use std::collections::{HashMap, HashSet};

/// Default decay factor applied to graph-expanded neighbors.
pub const GRAPH_EXPANSION_DECAY: f64 = 0.7;
/// Default maximum neighbors to expand per top result.
pub const GRAPH_EXPANSION_MAX_PER_RESULT: usize = 3;

/// A search result with combined score and signal breakdown.
#[derive(Debug, Clone)]
pub struct SmartSearchResult {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub score: f64,
    /// Backwards-compat substring/keyword signal (kept for callers that
    /// haven't migrated to BM25 inspection).
    pub keyword_score: f64,
    /// BM25 normalized score in [0.0, 1.0].
    pub bm25_score: f64,
    pub semantic_score: f64,
    pub r_eff: f64,
    pub graph_centrality: f64,
    /// If this result was added via graph expansion, the parent ID it was
    /// expanded from. `None` for direct text/semantic matches.
    pub expanded_from: Option<String>,
}

/// Compute keyword relevance score for a record against a query.
///
/// Returns a score in [0.0, 1.0] based on:
/// - Title exact match: 1.0
/// - Title contains query: 0.8
/// - Body contains query: 0.5
/// - No match: 0.0
///
/// Case-insensitive matching.
pub fn keyword_score(record: &ArtifactRecord, query: &str) -> f64 {
    let q = query.to_lowercase();
    if q.trim().is_empty() {
        return 0.0;
    }
    let title_lower = record.title.to_lowercase();

    if title_lower == q {
        1.0
    } else if title_lower.contains(&q) {
        0.8
    } else if strip_indexing_noise(&record.body)
        .to_lowercase()
        .contains(&q)
    {
        0.5
    } else {
        0.0
    }
}

/// Combine keyword score, semantic score, and boosters into a final score.
///
/// Formula: max(keyword, semantic) * (1.0 + r_eff_boost + status_boost + graph_boost)
/// - r_eff_boost: r_eff * 0.2 (quality artifacts rank higher)
/// - status_boost: 0.1 if active, 0.0 otherwise
/// - graph_boost: degree_centrality * 0.1 (well-connected artifacts rank higher)
pub fn combined_score(
    keyword: f64,
    semantic: f64,
    r_eff: f64,
    is_active: bool,
    graph_centrality: f64,
) -> f64 {
    let safe = |v: f64| {
        if v.is_finite() {
            v.clamp(0.0, 1.0)
        } else {
            0.0
        }
    };
    let base = safe(keyword).max(safe(semantic));
    if base == 0.0 {
        return 0.0;
    }
    let boost = 1.0
        + (safe(r_eff) * 0.2)
        + (if is_active { 0.1 } else { 0.0 })
        + (safe(graph_centrality) * 0.1);
    base * boost
}

/// Run smart search across all records.
///
/// Pipeline:
/// 1. Apply optional `filter` to exclude records.
/// 2. Score each remaining record with BM25 (built fresh from corpus) +
///    optional semantic similarity, then boost with R_eff/active/centrality.
/// 3. Sort, truncate to `limit`.
/// 4. If `graph` is `Some`, expand top results with 1-hop neighbors
///    (decayed score), dedupe, re-sort, truncate to `limit`.
///
/// `semantic_scores` is an optional map of artifact_id -> cosine_similarity.
/// If None (embeddings unavailable), only BM25 + boosters are used.
pub fn smart_search(
    records: &[ArtifactRecord],
    query: &str,
    graph: Option<&KnowledgeGraph>,
    semantic_scores: Option<&HashMap<String, f64>>,
    filter: Option<&ArtifactFilter>,
    limit: usize,
) -> Vec<SmartSearchResult> {
    if limit == 0 {
        return Vec::new();
    }

    // Build BM25 index over the (already filter-respecting) corpus. We index
    // the entire record set so IDF stats reflect the real corpus, not the
    // filtered subset; filtering only excludes from the result set.
    let bm25 = Bm25Index::build(records);
    // Batch search: single pass over the corpus → O(N), not O(N²).
    // Audit A (PROB-035): the per-record .score() called engine.search()
    // for each record, making smart_search O(N²). Use search_scores()
    // to get all BM25 scores in one pass, then look up per-record in O(1).
    let bm25_scores: std::collections::HashMap<String, f64> =
        bm25.search_scores(query, usize::MAX).into_iter().collect();

    let mut results: Vec<SmartSearchResult> = records
        .iter()
        .filter(|r| filter.map(|f| f.matches(r)).unwrap_or(true))
        .filter_map(|record| {
            let raw_bm25 = bm25_scores.get(&record.id).copied().unwrap_or(0.0);
            let bm25_norm = Bm25Index::normalize(raw_bm25);
            // PROB-030 fix: BM25 is token-based — queries like "auth" don't
            // match the "authentication" token. Users expect grep-like prefix
            // behavior. We take max(BM25, substring) so BM25 still wins on
            // exact-token matches (richer signal) but falls back to substring
            // when BM25 returns 0 for partial/prefix queries.
            let kw = keyword_score(record, query);
            let keyword_channel = bm25_norm.max(kw);
            let sem = semantic_scores
                .and_then(|m| m.get(&record.id))
                .copied()
                .unwrap_or(0.0);
            let centrality = graph
                .map(|g| g.degree_centrality(&record.id))
                .unwrap_or(0.0);
            let is_active = record.status.eq_ignore_ascii_case("active");
            let score = combined_score(
                keyword_channel,
                sem,
                record.r_eff_score,
                is_active,
                centrality,
            );

            if score > 0.0 {
                Some(SmartSearchResult {
                    id: record.id.clone(),
                    title: record.title.clone(),
                    kind: record.kind.clone(),
                    status: record.status.clone(),
                    score,
                    keyword_score: kw,
                    bm25_score: bm25_norm,
                    semantic_score: sem,
                    r_eff: record.r_eff_score,
                    graph_centrality: centrality,
                    expanded_from: None,
                })
            } else {
                None
            }
        })
        .collect();

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.id.cmp(&b.id))
    });
    results.truncate(limit);

    // Graph expansion (FR-003): add 1-hop neighbors of top results.
    if let Some(g) = graph {
        results = expand_with_graph_neighbors(
            &results,
            g,
            records,
            GRAPH_EXPANSION_DECAY,
            GRAPH_EXPANSION_MAX_PER_RESULT,
            limit,
        );
    }

    results
}

/// Expand top results with 1-hop graph neighbors.
///
/// For each top result we look up its neighbors in `graph`, attach decayed
/// scores, dedupe against direct hits, and merge. Direct hits always win
/// over expansion hits with the same ID.
pub fn expand_with_graph_neighbors(
    top_results: &[SmartSearchResult],
    graph: &KnowledgeGraph,
    all_records: &[ArtifactRecord],
    decay_factor: f64,
    max_expansions_per_result: usize,
    limit: usize,
) -> Vec<SmartSearchResult> {
    let mut by_id: HashMap<String, SmartSearchResult> = HashMap::new();
    for r in top_results {
        by_id.insert(r.id.clone(), r.clone());
    }
    let direct_ids: HashSet<String> = top_results.iter().map(|r| r.id.clone()).collect();

    // Index records by id for fast lookup.
    let record_by_id: HashMap<&str, &ArtifactRecord> =
        all_records.iter().map(|r| (r.id.as_str(), r)).collect();

    for parent in top_results {
        let neighbors = graph.neighbors(&parent.id);
        let mut added = 0usize;
        for n in neighbors {
            if added >= max_expansions_per_result {
                break;
            }
            if direct_ids.contains(&n.id) {
                continue; // direct hit wins
            }
            let neighbor_score = parent.score * decay_factor;
            // Skip if we already have this neighbor with a higher score.
            if let Some(existing) = by_id.get(&n.id)
                && existing.score >= neighbor_score
            {
                continue;
            }
            // Pull metadata from records when available; fall back to graph node.
            let (title, status, r_eff) = match record_by_id.get(n.id.as_str()) {
                Some(rec) => (rec.title.clone(), rec.status.clone(), rec.r_eff_score),
                None => (n.id.clone(), n.status.clone(), 0.0),
            };
            let entry = SmartSearchResult {
                id: n.id.clone(),
                title,
                kind: n.kind.clone(),
                status,
                score: neighbor_score,
                keyword_score: 0.0,
                bm25_score: 0.0,
                semantic_score: 0.0,
                r_eff,
                graph_centrality: graph.degree_centrality(&n.id),
                expanded_from: Some(parent.id.clone()),
            };
            by_id.insert(n.id.clone(), entry);
            added += 1;
        }
    }

    let mut merged: Vec<SmartSearchResult> = by_id.into_values().collect();
    merged.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.id.cmp(&b.id))
    });
    merged.truncate(limit);
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_record(id: &str, title: &str, body: &str, status: &str, r_eff: f64) -> ArtifactRecord {
        ArtifactRecord {
            id: id.to_string(),
            kind: "prd".to_string(),
            status: status.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            r_eff_score: r_eff,
            valid_until: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            tags: Vec::new(),
            body_hash: None,
            embedding: None,
        }
    }

    // ── keyword_score ───────────────────────────────────────────────

    #[test]
    fn keyword_exact_title_match() {
        let r = make_record("PRD-001", "Auth System", "", "active", 0.0);
        assert_eq!(keyword_score(&r, "auth system"), 1.0);
    }

    #[test]
    fn keyword_title_contains() {
        let r = make_record("PRD-001", "Auth System Design", "", "active", 0.0);
        assert!((keyword_score(&r, "auth system") - 0.8).abs() < 0.001);
    }

    #[test]
    fn keyword_body_match() {
        let r = make_record(
            "PRD-001",
            "Title",
            "OAuth2 integration needed",
            "active",
            0.0,
        );
        assert!((keyword_score(&r, "oauth2") - 0.5).abs() < 0.001);
    }

    #[test]
    fn keyword_no_match() {
        let r = make_record("PRD-001", "Title", "Body text", "active", 0.0);
        assert_eq!(keyword_score(&r, "nonexistent"), 0.0);
    }

    #[test]
    fn keyword_case_insensitive() {
        let r = make_record("PRD-001", "Authentication", "", "active", 0.0);
        assert_eq!(keyword_score(&r, "AUTHENTICATION"), 1.0);
    }

    // ── combined_score ──────────────────────────────────────────────

    #[test]
    fn combined_zero_when_no_text_match() {
        assert_eq!(combined_score(0.0, 0.0, 1.0, true, 1.0), 0.0);
    }

    #[test]
    fn combined_boosters_increase_score() {
        let base_only = combined_score(0.8, 0.0, 0.0, false, 0.0);
        let with_boosters = combined_score(0.8, 0.0, 1.0, true, 0.5);
        assert!(with_boosters > base_only);
    }

    #[test]
    fn combined_semantic_can_be_base() {
        let score = combined_score(0.0, 0.9, 0.0, false, 0.0);
        assert!((score - 0.9).abs() < 0.001);
    }

    #[test]
    fn combined_uses_max_of_keyword_semantic() {
        let score = combined_score(0.5, 0.9, 0.0, false, 0.0);
        assert!(
            (score - 0.9).abs() < 0.001,
            "should use semantic (0.9) not keyword (0.5)"
        );
    }

    #[test]
    fn combined_full_boosters() {
        // base=1.0, r_eff=1.0 (+0.2), active (+0.1), centrality=1.0 (+0.1) = 1.0 * 1.4 = 1.4
        let score = combined_score(1.0, 0.0, 1.0, true, 1.0);
        assert!((score - 1.4).abs() < 0.001);
    }

    #[test]
    fn combined_clamps_inputs() {
        // r_eff and centrality clamped to [0, 1]
        let score = combined_score(1.0, 0.0, 5.0, true, 5.0);
        assert!((score - 1.4).abs() < 0.001, "should clamp to max 1.0");
    }

    // ── smart_search ────────────────────────────────────────────────

    #[test]
    fn smart_search_keyword_only() {
        let records = vec![
            make_record("PRD-001", "Auth System", "OAuth2 login", "active", 0.8),
            make_record("PRD-002", "Performance", "Load testing", "draft", 0.0),
        ];
        let results = smart_search(&records, "auth", None, None, None, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "PRD-001");
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn smart_search_with_semantic() {
        let records = vec![
            make_record("PRD-001", "Auth", "login", "active", 0.0),
            make_record("PRD-002", "Perf", "speed", "active", 0.0),
        ];
        let mut sem = HashMap::new();
        sem.insert("PRD-001".to_string(), 0.3);
        sem.insert("PRD-002".to_string(), 0.95);

        let results = smart_search(&records, "nonexistent-keyword", None, Some(&sem), None, 10);
        // Keyword matches nothing, but semantic finds PRD-002
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "PRD-002", "highest semantic score wins");
    }

    #[test]
    fn smart_search_respects_limit() {
        let records: Vec<_> = (0..20)
            .map(|i| {
                make_record(
                    &format!("PRD-{i:03}"),
                    &format!("Auth variant {i}"),
                    "",
                    "active",
                    0.0,
                )
            })
            .collect();
        let results = smart_search(&records, "auth", None, None, None, 5);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn smart_search_active_ranks_higher() {
        let records = vec![
            make_record("PRD-001", "Auth System", "", "draft", 0.0),
            make_record("PRD-002", "Auth System", "", "active", 0.0),
        ];
        let results = smart_search(&records, "auth system", None, None, None, 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "PRD-002", "active should rank higher");
    }

    #[test]
    fn smart_search_r_eff_boosts() {
        let records = vec![
            make_record("PRD-001", "Auth System", "", "active", 0.0),
            make_record("PRD-002", "Auth System", "", "active", 1.0),
        ];
        let results = smart_search(&records, "auth system", None, None, None, 10);
        assert_eq!(results[0].id, "PRD-002", "higher R_eff should rank higher");
    }

    #[test]
    fn keyword_no_match_returns_nothing() {
        let records = vec![make_record("PRD-001", "Auth", "body", "active", 0.0)];
        let results = smart_search(&records, "zzz-no-match", None, None, None, 10);
        assert!(results.is_empty());
    }

    // ── audit findings: edge cases ──────────────────────────────────

    #[test]
    fn keyword_empty_query_returns_zero() {
        let r = make_record("PRD-001", "Auth", "body", "active", 0.0);
        assert_eq!(keyword_score(&r, ""), 0.0);
        assert_eq!(keyword_score(&r, "  "), 0.0);
    }

    #[test]
    fn combined_nan_inputs_return_finite() {
        let score = combined_score(f64::NAN, 0.5, 0.5, true, 0.5);
        assert!(score.is_finite(), "NaN keyword should be sanitized");

        let score2 = combined_score(0.5, f64::NAN, 0.5, true, 0.5);
        assert!(score2.is_finite(), "NaN semantic should be sanitized");

        let score3 = combined_score(0.5, 0.5, f64::NAN, true, 0.5);
        assert!(score3.is_finite(), "NaN r_eff should be sanitized");

        let score4 = combined_score(0.5, 0.5, 0.5, true, f64::NAN);
        assert!(score4.is_finite(), "NaN centrality should be sanitized");
    }

    #[test]
    fn combined_infinity_inputs_sanitized() {
        // INFINITY is not finite → safe() returns 0.0 → base = 0.0 → early return 0.0
        let score = combined_score(f64::INFINITY, 0.0, 0.0, false, 0.0);
        assert!(score.is_finite());
        assert_eq!(score, 0.0, "infinite keyword treated as no match");

        // With a valid semantic, infinity keyword is ignored, semantic used as base
        let score2 = combined_score(f64::INFINITY, 0.8, 0.0, false, 0.0);
        assert!(score2.is_finite());
        assert!((score2 - 0.8).abs() < 0.001, "valid semantic used as base");
    }

    #[test]
    fn combined_negative_inputs_clamped() {
        let score = combined_score(0.5, 0.0, -1.0, false, -1.0);
        // negative r_eff and centrality clamped to 0.0, boost = 1.0
        assert!((score - 0.5).abs() < 0.001);
    }

    #[test]
    fn smart_search_empty_records() {
        let results = smart_search(&[], "auth", None, None, None, 10);
        assert!(results.is_empty());
    }

    #[test]
    fn smart_search_empty_query() {
        let records = vec![make_record("PRD-001", "Auth", "body", "active", 0.0)];
        let results = smart_search(&records, "", None, None, None, 10);
        assert!(results.is_empty(), "empty query should return nothing");
    }

    #[test]
    fn smart_search_limit_zero() {
        let records = vec![make_record("PRD-001", "Auth", "", "active", 0.0)];
        let results = smart_search(&records, "auth", None, None, None, 0);
        assert!(results.is_empty(), "limit=0 returns nothing");
    }

    #[test]
    fn smart_search_nan_r_eff_handled() {
        let records = vec![make_record("PRD-001", "Auth", "", "active", f64::NAN)];
        let results = smart_search(&records, "auth", None, None, None, 10);
        assert_eq!(results.len(), 1);
        assert!(
            results[0].score.is_finite(),
            "NaN r_eff must not corrupt score"
        );
    }

    #[test]
    fn smart_search_all_three_signals() {
        use crate::graph::knowledge::{ArtifactNode, KnowledgeGraph};

        let records = vec![
            make_record("PRD-001", "Auth System", "", "active", 0.0),
            make_record("PRD-002", "Perf System", "", "active", 1.0),
        ];

        // PRD-002 has high semantic, high r_eff, high centrality but no keyword match
        // PRD-001 has keyword match but no semantic, no r_eff, no centrality
        let mut sem = HashMap::new();
        sem.insert("PRD-002".to_string(), 0.95);

        let nodes = vec![
            ArtifactNode {
                id: "PRD-001".into(),
                kind: "prd".into(),
                status: "active".into(),
            },
            ArtifactNode {
                id: "PRD-002".into(),
                kind: "prd".into(),
                status: "active".into(),
            },
            ArtifactNode {
                id: "RFC-001".into(),
                kind: "rfc".into(),
                status: "active".into(),
            },
        ];
        let edges = vec![("RFC-001".into(), "PRD-002".into(), "based_on".into())];
        let graph = KnowledgeGraph::from_parts(nodes, edges);

        let results = smart_search(&records, "auth", Some(&graph), Some(&sem), None, 10);
        // With graph expansion enabled, RFC-001 (neighbor of PRD-002) may be added.
        assert!(results.len() >= 2);
        // PRD-002: sem=0.95 * boost(r_eff=1.0, active, centrality=0.5)
        // PRD-001: kw=0.8 * boost(r_eff=0.0, active, centrality=0.0)
        assert_eq!(
            results[0].id, "PRD-002",
            "semantic+boosters should outrank keyword-only"
        );
    }

    #[test]
    fn smart_search_sort_stability_tiebreaker() {
        let records = vec![
            make_record("PRD-002", "Auth", "", "active", 0.0),
            make_record("PRD-001", "Auth", "", "active", 0.0),
        ];
        let results = smart_search(&records, "auth", None, None, None, 10);
        assert_eq!(results.len(), 2);
        // Same score — tiebreaker by id ascending
        assert_eq!(results[0].id, "PRD-001");
        assert_eq!(results[1].id, "PRD-002");
    }

    #[test]
    fn keyword_unicode_cyrillic() {
        let r = make_record(
            "PRD-001",
            "Аутентификация",
            "Логин через OAuth",
            "active",
            0.0,
        );
        assert_eq!(keyword_score(&r, "аутентификация"), 1.0);
        assert!((keyword_score(&r, "логин") - 0.5).abs() < 0.001);
    }

    // ── BM25 / filter / graph expansion (PRD-039 W2) ───────────────

    #[test]
    fn smart_search_with_bm25_scoring_higher_tf_wins() {
        // PRD-001 mentions "auth" multiple times → higher BM25 TF.
        let records = vec![
            make_record(
                "PRD-001",
                "Authentication",
                "auth auth auth login flow",
                "active",
                0.0,
            ),
            make_record("PRD-002", "Authentication", "auth once", "active", 0.0),
        ];
        let results = smart_search(&records, "auth", None, None, None, 10);
        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0].id, "PRD-001",
            "higher TF should rank higher with BM25"
        );
        assert!(results[0].bm25_score > 0.0);
    }

    #[test]
    fn smart_search_with_filter_excludes_non_matching() {
        let records = vec![
            make_record("PRD-001", "Auth System", "", "active", 0.0),
            make_record("PRD-002", "Auth Module", "", "draft", 0.0),
        ];
        let filter = ArtifactFilter::Status("active".to_string());
        let results = smart_search(&records, "auth", None, None, Some(&filter), 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "PRD-001");
    }

    #[test]
    fn smart_search_filter_none_includes_all_backward_compat() {
        let records = vec![
            make_record("PRD-001", "Auth", "", "active", 0.0),
            make_record("PRD-002", "Auth", "", "draft", 0.0),
        ];
        let results = smart_search(&records, "auth", None, None, None, 10);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn smart_search_graph_expansion_adds_neighbors() {
        use crate::graph::knowledge::{ArtifactNode, KnowledgeGraph};
        let records = vec![
            make_record("PRD-001", "Auth System", "", "active", 0.0),
            make_record("RFC-001", "Auth Architecture", "", "active", 0.0),
        ];
        // RFC-001 doesn't match query, but is linked from PRD-001 → expansion adds it.
        let nodes = vec![
            ArtifactNode {
                id: "PRD-001".into(),
                kind: "prd".into(),
                status: "active".into(),
            },
            ArtifactNode {
                id: "RFC-001".into(),
                kind: "rfc".into(),
                status: "active".into(),
            },
        ];
        let edges = vec![("PRD-001".into(), "RFC-001".into(), "informs".into())];
        let graph = KnowledgeGraph::from_parts(nodes, edges);

        // Limit=2 so both fit; query "system" matches only PRD-001 directly.
        let results = smart_search(&records, "system", Some(&graph), None, None, 2);
        let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"PRD-001"));
        assert!(
            ids.contains(&"RFC-001"),
            "neighbor should be added via graph expansion"
        );
        let rfc = results.iter().find(|r| r.id == "RFC-001").unwrap();
        assert_eq!(rfc.expanded_from.as_deref(), Some("PRD-001"));
        // Decayed score = parent.score * 0.7
        let prd = results.iter().find(|r| r.id == "PRD-001").unwrap();
        assert!((rfc.score - prd.score * GRAPH_EXPANSION_DECAY).abs() < 1e-6);
    }

    #[test]
    fn smart_search_graph_expansion_dedupe_direct_wins() {
        use crate::graph::knowledge::{ArtifactNode, KnowledgeGraph};
        let records = vec![
            make_record("PRD-001", "Auth", "", "active", 0.0),
            make_record("PRD-002", "Auth", "", "active", 0.0),
        ];
        let nodes = vec![
            ArtifactNode {
                id: "PRD-001".into(),
                kind: "prd".into(),
                status: "active".into(),
            },
            ArtifactNode {
                id: "PRD-002".into(),
                kind: "prd".into(),
                status: "active".into(),
            },
        ];
        let edges = vec![("PRD-001".into(), "PRD-002".into(), "informs".into())];
        let graph = KnowledgeGraph::from_parts(nodes, edges);

        let results = smart_search(&records, "auth", Some(&graph), None, None, 10);
        // No duplicates: each ID appears at most once.
        let mut ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        ids.sort();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "expanded results must be unique");
        // Both should be present, both as direct hits (expanded_from = None)
        for r in &results {
            assert!(
                r.expanded_from.is_none(),
                "{} should be direct, not expansion",
                r.id
            );
        }
    }

    #[test]
    fn smart_search_backward_compat_no_filter_no_graph() {
        let records = vec![
            make_record("PRD-001", "Auth", "login", "active", 0.5),
            make_record("PRD-002", "Perf", "speed", "draft", 0.0),
        ];
        let results = smart_search(&records, "auth", None, None, None, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "PRD-001");
        assert!(results[0].expanded_from.is_none());
    }

    // ── PROB-030 regression: BM25 prefix fallback ───────────────────

    #[test]
    fn smart_search_prefix_query_falls_back_to_substring() {
        // PROB-030: BM25 is token-based — the prefix "auth" does not match
        // the token "authentication", so BM25 alone returns 0. Users expect
        // grep-like prefix behavior. The fix: combined keyword channel =
        // max(bm25_norm, substring_keyword_score), so substring match wins
        // when BM25 is silent.
        let records = vec![
            make_record(
                "PRD-001",
                "Authentication OAuth2 system",
                "user login flow",
                "active",
                0.0,
            ),
            make_record(
                "PRD-002",
                "Unrelated topic",
                "nothing about logins",
                "active",
                0.0,
            ),
        ];
        let results = smart_search(&records, "auth", None, None, None, 10);
        assert_eq!(
            results.len(),
            1,
            "prefix query 'auth' must find 'Authentication' via substring fallback"
        );
        assert_eq!(results[0].id, "PRD-001");
        assert!(
            results[0].score > 0.0,
            "score must be non-zero for a matching prefix query"
        );
    }

    #[test]
    fn smart_search_prefix_unicode_cyrillic() {
        // PROB-030 follow-up (audit B): substring fallback must work
        // for non-ASCII prefixes like `"аут"` matching `"аутентификация"`.
        let records = vec![
            make_record(
                "PRD-001",
                "Система аутентификации OAuth2",
                "логин и сессии",
                "active",
                0.0,
            ),
            make_record("PRD-002", "Платежи", "обработка карт", "active", 0.0),
        ];
        let results = smart_search(&records, "аут", None, None, None, 10);
        assert!(!results.is_empty(), "cyrillic prefix must find record");
        assert_eq!(results[0].id, "PRD-001");
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn smart_search_exact_token_still_wins_over_prefix() {
        // PROB-030 regression guard: substring fallback must NOT suppress
        // BM25's richer signal for exact-token matches. Docs with more TF
        // of the exact query term should still rank higher than docs that
        // only match via substring.
        let records = vec![
            make_record("PRD-001", "Auth", "auth auth auth auth", "active", 0.0),
            make_record("PRD-002", "Authentication", "login only", "active", 0.0),
        ];
        let results = smart_search(&records, "auth", None, None, None, 10);
        assert!(results.len() >= 2);
        assert_eq!(
            results[0].id, "PRD-001",
            "exact-token doc with higher TF must still beat substring-only match"
        );
    }
}
