//! BM25 relevance scoring for artifact search.
//!
//! Replaces the primitive substring-match keyword_score with proper
//! term-frequency × inverse-document-frequency scoring.
//!
//! Parameters (standard defaults):
//! - k1 = 1.5 (term frequency saturation)
//! - b = 0.75 (length normalization)
//!
//! Pattern source: sources/RuVector/crates/ruvector-core/src/advanced_features/hybrid_search.rs

use crate::db::store::ArtifactRecord;
use std::collections::{HashMap, HashSet};

/// BM25 index with IDF scores and inverted index.
#[derive(Debug, Clone, Default)]
pub struct Bm25Index {
    /// IDF scores per term
    idf: HashMap<String, f64>,
    /// Document lengths (token counts) per artifact ID
    doc_lengths: HashMap<String, usize>,
    /// Average document length across corpus
    avg_doc_len: f64,
    /// Inverted index: term -> set of artifact IDs containing it
    inverted_index: HashMap<String, HashSet<String>>,
    /// BM25 k1 parameter (default 1.5)
    k1: f64,
    /// BM25 b parameter (default 0.75)
    b: f64,
}

impl Bm25Index {
    /// Create empty index with standard BM25 parameters.
    pub fn new() -> Self {
        Self {
            k1: 1.5,
            b: 0.75,
            ..Default::default()
        }
    }

    /// Build index from a corpus of artifact records.
    /// Call this once before any scoring calls.
    pub fn build(records: &[ArtifactRecord]) -> Self {
        let mut idx = Self::new();
        for record in records {
            idx.index_document(&record.id, &Self::doc_text(record));
        }
        idx.compute_idf();
        idx
    }

    /// Extract searchable text from a record (title + body).
    fn doc_text(record: &ArtifactRecord) -> String {
        format!("{} {}", record.title, record.body)
    }

    /// Index a single document.
    fn index_document(&mut self, doc_id: &str, text: &str) {
        let terms = tokenize(text);
        self.doc_lengths.insert(doc_id.to_string(), terms.len());
        for term in &terms {
            self.inverted_index
                .entry(term.clone())
                .or_default()
                .insert(doc_id.to_string());
        }
    }

    /// Compute IDF scores after all documents indexed.
    fn compute_idf(&mut self) {
        let num_docs = self.doc_lengths.len() as f64;
        if num_docs == 0.0 {
            self.avg_doc_len = 0.0;
            return;
        }
        self.avg_doc_len = self.doc_lengths.values().sum::<usize>() as f64 / num_docs;

        for (term, doc_set) in &self.inverted_index {
            let doc_freq = doc_set.len() as f64;
            // BM25 IDF formula (with +1 smoothing)
            let idf = ((num_docs - doc_freq + 0.5) / (doc_freq + 0.5) + 1.0).ln();
            self.idf.insert(term.clone(), idf);
        }
    }

    /// Score a record against a query.
    /// Returns BM25 score (higher = more relevant).
    pub fn score(&self, record: &ArtifactRecord, query: &str) -> f64 {
        let query_terms = tokenize(query);
        if query_terms.is_empty() {
            return 0.0;
        }
        let doc_text = Self::doc_text(record);
        let doc_terms = tokenize(&doc_text);
        let doc_len = self
            .doc_lengths
            .get(&record.id)
            .copied()
            .unwrap_or(doc_terms.len()) as f64;

        // Count term frequencies in document
        let mut term_freq: HashMap<String, f64> = HashMap::new();
        for term in doc_terms {
            *term_freq.entry(term).or_insert(0.0) += 1.0;
        }

        // BM25 score: sum over query terms
        let mut score = 0.0;
        for term in query_terms {
            let idf = self.idf.get(&term).copied().unwrap_or(0.0);
            let tf = term_freq.get(&term).copied().unwrap_or(0.0);

            let numerator = tf * (self.k1 + 1.0);
            let denominator =
                tf + self.k1 * (1.0 - self.b + self.b * (doc_len / self.avg_doc_len.max(1.0)));

            score += idf * (numerator / denominator.max(1e-9));
        }

        score
    }

    /// Normalize BM25 score to [0.0, 1.0] for combining with other scores.
    /// Uses a simple tanh saturation — real BM25 scores are unbounded.
    pub fn normalize(raw: f64) -> f64 {
        // tanh saturates around 5.0, giving nice [0, 1) output
        (raw / 5.0).tanh().clamp(0.0, 1.0)
    }
}

/// Tokenize text into lowercase words (min length 3, alphanumeric).
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split_whitespace()
        .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|s| s.len() >= 3)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(id: &str, title: &str, body: &str) -> ArtifactRecord {
        ArtifactRecord {
            id: id.to_string(),
            kind: "prd".to_string(),
            status: "active".to_string(),
            title: title.to_string(),
            body: body.to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            r_eff_score: 0.0,
            valid_until: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn tokenize_basic() {
        let tokens = tokenize("The quick brown fox!");
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"brown".to_string()));
        assert!(!tokens.iter().any(|t| t.len() < 3));
    }

    #[test]
    fn tokenize_strips_punctuation() {
        let tokens = tokenize("hello, world! foo.bar");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
    }

    #[test]
    fn bm25_empty_index() {
        let idx = Bm25Index::build(&[]);
        let rec = mk("PRD-1", "Auth", "content");
        assert_eq!(idx.score(&rec, "auth"), 0.0);
    }

    #[test]
    fn bm25_empty_query() {
        let records = vec![mk("PRD-1", "Auth", "body")];
        let idx = Bm25Index::build(&records);
        assert_eq!(idx.score(&records[0], ""), 0.0);
    }

    #[test]
    fn bm25_exact_match() {
        let records = vec![
            mk("PRD-1", "Authentication System", "user authentication flow"),
            mk("PRD-2", "Payment Service", "payment processing"),
        ];
        let idx = Bm25Index::build(&records);

        let s1 = idx.score(&records[0], "authentication");
        let s2 = idx.score(&records[1], "authentication");

        assert!(s1 > s2, "PRD-1 should score higher for 'authentication'");
        assert!(s1 > 0.0);
    }

    #[test]
    fn bm25_rare_term_higher_idf() {
        let records = vec![
            mk(
                "PRD-1",
                "Common authentication flow",
                "authentication everywhere",
            ),
            mk("PRD-2", "Authentication module", "authentication standard"),
            mk(
                "PRD-3",
                "Authentication api",
                "authentication and quantum entanglement",
            ),
        ];
        let idx = Bm25Index::build(&records);

        // "quantum" appears in only 1 doc → high IDF
        let s_rare = idx.score(&records[2], "quantum");
        // "authentication" appears in all 3 docs → low IDF
        let s_common = idx.score(&records[0], "authentication");

        assert!(s_rare > 0.0);
        assert!(s_rare > s_common, "rare term should outscore common term");
    }

    #[test]
    fn bm25_normalize_range() {
        assert_eq!(Bm25Index::normalize(0.0), 0.0);
        assert!(Bm25Index::normalize(10.0) < 1.0);
        assert!(Bm25Index::normalize(10.0) > 0.9);
        assert!(Bm25Index::normalize(1.0) > 0.0);
    }

    #[test]
    fn bm25_unknown_term_returns_zero() {
        let records = vec![mk("PRD-1", "Auth", "body content")];
        let idx = Bm25Index::build(&records);
        assert_eq!(idx.score(&records[0], "nonexistentterm"), 0.0);
    }
}
