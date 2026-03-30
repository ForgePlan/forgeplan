//! Smart search: combines keyword, semantic, and graph signals.
//!
//! Scoring model: text-first + boosters.
//! base_score = max(keyword_score, semantic_score)
//! boost = 1.0 + (r_eff * 0.2) + (is_active * 0.1) + (graph_centrality * 0.1)
//! final_score = base_score * boost
//!
//! If embeddings are unavailable, gracefully degrades to keyword-only.

use crate::db::store::ArtifactRecord;
use crate::graph::knowledge::KnowledgeGraph;

/// A search result with combined score and signal breakdown.
#[derive(Debug, Clone)]
pub struct SmartSearchResult {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub score: f64,
    pub keyword_score: f64,
    pub semantic_score: f64,
    pub r_eff: f64,
    pub graph_centrality: f64,
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
    let title_lower = record.title.to_lowercase();
    let body_lower = record.body.to_lowercase();

    if title_lower == q {
        1.0
    } else if title_lower.contains(&q) {
        0.8
    } else if body_lower.contains(&q) {
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
    let base = keyword.max(semantic);
    if base == 0.0 {
        return 0.0;
    }
    let boost = 1.0
        + (r_eff.clamp(0.0, 1.0) * 0.2)
        + (if is_active { 0.1 } else { 0.0 })
        + (graph_centrality.clamp(0.0, 1.0) * 0.1);
    base * boost
}

/// Run smart search across all records.
///
/// `semantic_scores` is an optional map of artifact_id -> cosine_similarity.
/// If None (embeddings unavailable), only keyword + boosters are used.
pub fn smart_search(
    records: &[ArtifactRecord],
    query: &str,
    semantic_scores: Option<&std::collections::HashMap<String, f64>>,
    graph: Option<&KnowledgeGraph>,
    limit: usize,
) -> Vec<SmartSearchResult> {
    let mut results: Vec<SmartSearchResult> = records
        .iter()
        .filter_map(|record| {
            let kw = keyword_score(record, query);
            let sem = semantic_scores
                .and_then(|m| m.get(&record.id))
                .copied()
                .unwrap_or(0.0);
            let centrality = graph
                .map(|g| g.degree_centrality(&record.id))
                .unwrap_or(0.0);
            let is_active = record.status.eq_ignore_ascii_case("active");
            let score = combined_score(kw, sem, record.r_eff_score, is_active, centrality);

            if score > 0.0 {
                Some(SmartSearchResult {
                    id: record.id.clone(),
                    title: record.title.clone(),
                    kind: record.kind.clone(),
                    status: record.status.clone(),
                    score,
                    keyword_score: kw,
                    semantic_score: sem,
                    r_eff: record.r_eff_score,
                    graph_centrality: centrality,
                })
            } else {
                None
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    results
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
        let r = make_record("PRD-001", "Title", "OAuth2 integration needed", "active", 0.0);
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
        assert!((score - 0.9).abs() < 0.001, "should use semantic (0.9) not keyword (0.5)");
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
        let results = smart_search(&records, "auth", None, None, 10);
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

        let results = smart_search(&records, "nonexistent-keyword", Some(&sem), None, 10);
        // Keyword matches nothing, but semantic finds PRD-002
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "PRD-002", "highest semantic score wins");
    }

    #[test]
    fn smart_search_respects_limit() {
        let records: Vec<_> = (0..20)
            .map(|i| make_record(&format!("PRD-{i:03}"), &format!("Auth variant {i}"), "", "active", 0.0))
            .collect();
        let results = smart_search(&records, "auth", None, None, 5);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn smart_search_active_ranks_higher() {
        let records = vec![
            make_record("PRD-001", "Auth System", "", "draft", 0.0),
            make_record("PRD-002", "Auth System", "", "active", 0.0),
        ];
        let results = smart_search(&records, "auth system", None, None, 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "PRD-002", "active should rank higher");
    }

    #[test]
    fn smart_search_r_eff_boosts() {
        let records = vec![
            make_record("PRD-001", "Auth System", "", "active", 0.0),
            make_record("PRD-002", "Auth System", "", "active", 1.0),
        ];
        let results = smart_search(&records, "auth system", None, None, 10);
        assert_eq!(results[0].id, "PRD-002", "higher R_eff should rank higher");
    }

    #[test]
    fn smart_search_empty_query_returns_nothing() {
        let records = vec![make_record("PRD-001", "Auth", "body", "active", 0.0)];
        let results = smart_search(&records, "zzz-no-match", None, None, 10);
        assert!(results.is_empty());
    }
}
