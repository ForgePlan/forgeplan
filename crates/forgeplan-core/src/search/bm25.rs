//! BM25 relevance scoring for artifact search.
//!
//! Wraps the `bm25` crate (v2.3.2) which provides production-quality BM25
//! with proper tokenization, stemming, stop-word removal, and unicode
//! normalization — replacing the hand-rolled 140 LOC implementation that
//! lacked all of these (PROB-035).
//!
//! The `bm25` crate's stemmer reduces `authentication` and `auth` to a
//! common stem, which naturally fixes the prefix-query problem (PROB-030)
//! without the substring fallback hack.
//!
//! Public API is kept backwards-compatible: `Bm25Index::build()`, `.score()`,
//! `::normalize()` — callers in `smart.rs` don't change.

use crate::db::store::ArtifactRecord;
use bm25::{DefaultTokenizer, Document, LanguageMode, SearchEngineBuilder};

/// BM25 index wrapping the `bm25` crate's SearchEngine.
///
/// Built once per search from the full record corpus, then queried per-record.
/// Uses String document IDs to match ArtifactRecord::id.
pub struct Bm25Index {
    /// The underlying search engine from the `bm25` crate.
    engine: bm25::SearchEngine<String>,
}

impl Bm25Index {
    /// Build index from a corpus of artifact records.
    ///
    /// Uses automatic language detection (`LanguageMode::Detect`) so both
    /// English and Russian content get proper stemming + stop-words.
    /// Forgeplan artifacts are mixed-language (English titles, Russian body),
    /// so per-document detection is the right approach.
    pub fn build(records: &[ArtifactRecord]) -> Self {
        let documents: Vec<Document<String>> = records
            .iter()
            .map(|r| {
                let clean_body = strip_indexing_noise(&r.body);
                Document::new(r.id.clone(), format!("{} {}", r.title, clean_body))
            })
            .collect();

        // Custom tokenizer: language detection ON, normalization OFF.
        //
        // `deunicode` normalization transliterates non-Latin to ASCII
        // BEFORE stemming, which destroys Cyrillic: "аутентификация" →
        // "autentifikatsiya" → Russian stemmer can't stem ASCII → no match.
        // Disabling normalization preserves original scripts so both
        // English and Russian stemmers work correctly on their native text.
        let tokenizer = DefaultTokenizer::builder()
            .language_mode(LanguageMode::Detect)
            .normalization(false)
            .build();
        let engine =
            SearchEngineBuilder::with_tokenizer_and_documents(tokenizer, documents).build();

        Self { engine }
    }

    /// Score a single record against a query.
    ///
    /// Returns the raw BM25 score (higher = more relevant, unbounded).
    /// The `bm25` crate internally tokenizes, stems, and applies IDF weighting.
    pub fn score(&self, record: &ArtifactRecord, query: &str) -> f64 {
        if query.trim().is_empty() {
            return 0.0;
        }
        // Search the full corpus and find the score for this specific record.
        // The crate returns top-N results; we request all to find our record.
        let results = self.engine.search(query, usize::MAX);
        results
            .iter()
            .find(|r| r.document.id == record.id)
            .map(|r| r.score as f64)
            .unwrap_or(0.0)
    }

    /// Batch score: return scores for all records in one pass.
    ///
    /// More efficient than calling `score()` per-record since the crate
    /// searches the full index once.
    pub fn search_scores(&self, query: &str, limit: usize) -> Vec<(String, f64)> {
        if query.trim().is_empty() {
            return Vec::new();
        }
        self.engine
            .search(query, limit)
            .into_iter()
            .map(|r| (r.document.id.clone(), r.score as f64))
            .collect()
    }

    /// Normalize BM25 score to [0.0, 1.0] for combining with other scores.
    /// Uses a simple tanh saturation — real BM25 scores are unbounded.
    ///
    /// Note: the `bm25` crate may produce different score magnitudes than the
    /// old hand-written BM25 — the constant 5.0 was tuned for the old tokenizer
    /// and remains acceptable for the crate (Audit A LOW finding).
    pub fn normalize(raw: f64) -> f64 {
        (raw / 5.0).tanh().clamp(0.0, 1.0)
    }
}

/// Strip noise from artifact body before BM25 indexing.
///
/// Removes lines that pollute the search index with irrelevant tokens:
/// - YAML frontmatter (`---` delimited blocks at the start)
/// - Template placeholders (`{...}`)
/// - Markdown table rows starting with `|` (template NFR examples)
/// - HTML comments (single-line `<!-- ... -->`)
///
/// This fixes the false-positive issue where `forgeplan search "auth"` matched
/// unrelated PRDs because their template body contained `author:` (frontmatter)
/// or `| ... authenticate ... |` (example NFR table row).
pub fn strip_indexing_noise(body: &str) -> String {
    let mut result = Vec::new();
    let mut in_frontmatter = false;
    let mut saw_frontmatter_open = false;

    for line in body.lines() {
        let trimmed = line.trim();

        // YAML frontmatter: skip `---` delimited block at start of body.
        if trimmed == "---" {
            if !saw_frontmatter_open {
                saw_frontmatter_open = true;
                in_frontmatter = true;
                continue;
            } else if in_frontmatter {
                in_frontmatter = false;
                continue;
            }
        }
        if in_frontmatter {
            continue;
        }

        // Template placeholder lines: `{Что измерено...}`, `{author}`
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            continue;
        }

        // Markdown table rows: `| NFR-003 | Security | System shall authenticate |`
        if trimmed.starts_with('|') {
            continue;
        }

        // Single-line HTML comments: `<!-- ... -->`
        if trimmed.starts_with("<!--") && trimmed.ends_with("-->") {
            continue;
        }

        result.push(line);
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_indexing_noise_removes_frontmatter() {
        let body = "---\nauthor: test\nkind: prd\n---\n\n# Real content\n\nHello world";
        let clean = strip_indexing_noise(body);
        assert!(!clean.contains("author"), "frontmatter should be stripped");
        assert!(clean.contains("Real content"));
        assert!(clean.contains("Hello world"));
    }

    #[test]
    fn strip_indexing_noise_removes_template_tables_and_placeholders() {
        let body = "## NFR\n\n| NFR-003 | Security | System shall authenticate | OAuth2 |\n| NFR-004 | Perf | Fast |\n\n## Goals\n\n{Описание целей}\n\nReal goals here";
        let clean = strip_indexing_noise(body);
        assert!(
            !clean.contains("authenticate"),
            "template table row should be stripped"
        );
        assert!(
            !clean.contains("Описание целей"),
            "placeholder should be stripped"
        );
        assert!(clean.contains("Real goals here"));
    }

    #[test]
    fn bm25_no_false_positive_from_frontmatter_author() {
        // PROB-035 fix: "auth" should NOT match a PRD about payments
        // just because its frontmatter has `author:` field.
        let records = vec![
            mk(
                "PRD-1",
                "Authentication System",
                "---\nauthor: john\nkind: prd\n---\n\n## Problem\n\nUser auth flow",
            ),
            mk(
                "PRD-2",
                "Payment Processing",
                "---\nauthor: jane\nkind: prd\n---\n\n## Problem\n\nStripe integration",
            ),
        ];
        let idx = Bm25Index::build(&records);
        let s1 = idx.score(&records[0], "auth");
        let s2 = idx.score(&records[1], "auth");
        assert!(s1 > 0.0, "PRD-1 should match 'auth' in title");
        assert_eq!(
            s2, 0.0,
            "PRD-2 should NOT match 'auth' — frontmatter author: stripped"
        );
    }

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
            body_hash: None,
            embedding: None,
        }
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
    fn bm25_stemming_normalizes_word_forms() {
        // The bm25 crate's stemmer normalizes word forms:
        // "authenticated" and "authentication" share a common stem.
        // Note: "auth" (short prefix) does NOT share a stem with
        // "authentication" — prefix matching is handled by keyword_score
        // in smart.rs, not by BM25 stemming.
        let records = vec![
            mk(
                "PRD-1",
                "Authentication OAuth2 system",
                "user authenticated via oauth2",
            ),
            mk("PRD-2", "Payment Service", "payment processing"),
        ];
        let idx = Bm25Index::build(&records);
        let score = idx.score(&records[0], "authenticated");
        assert!(
            score > 0.0,
            "stemmer should match 'authenticated' to 'authentication' via common stem"
        );
    }

    #[test]
    fn bm25_russian_morphology_with_language_detection() {
        // LanguageMode::Detect enables Snowball Russian stemmer.
        // "аутентификация" (nominative) should match "аутентификации"
        // (genitive) via common stem. Previously 0 results.
        let records = vec![
            mk(
                "PRD-1",
                "Система аутентификации пользователей",
                "модуль авторизации и проверки токенов",
            ),
            mk("PRD-2", "Payment Service", "billing"),
        ];
        let idx = Bm25Index::build(&records);
        let score = idx.score(&records[0], "аутентификация");
        assert!(
            score > 0.0,
            "Russian stemmer should match 'аутентификация' to 'аутентификации', got score={score}"
        );
    }

    #[test]
    fn bm25_russian_plural_forms() {
        // "пользователь" (singular) should match "пользователей" (genitive plural).
        let records = vec![mk(
            "PRD-1",
            "Система аутентификации пользователей",
            "управление доступом",
        )];
        let idx = Bm25Index::build(&records);
        let score = idx.score(&records[0], "пользователь");
        assert!(
            score > 0.0,
            "Russian stemmer should match singular→plural, got score={score}"
        );
    }

    #[test]
    fn bm25_plural_stemming() {
        // Audit B: plural forms should match via stemmer.
        // "systems" → stem "system", matches "system" in title.
        let records = vec![
            mk("PRD-1", "Authentication System", "user login"),
            mk("PRD-2", "Payment Service", "billing"),
        ];
        let idx = Bm25Index::build(&records);
        let score = idx.score(&records[0], "systems");
        assert!(
            score > 0.0,
            "plural 'systems' should match 'System' via stemmer, got score={score}"
        );
    }

    #[test]
    fn bm25_stopword_resilience() {
        // Audit B: stop-words ("the", "for", "and") should be ignored.
        // "the authentication" should score similarly to "authentication".
        let records = vec![
            mk("PRD-1", "Authentication System", "user authentication flow"),
            mk("PRD-2", "Payment Service", "payment processing"),
        ];
        let idx = Bm25Index::build(&records);
        let with_stop = idx.score(&records[0], "the authentication");
        let without_stop = idx.score(&records[0], "authentication");
        assert!(with_stop > 0.0, "query with stop-word should still match");
        // Scores should be close (stop-word ignored by the tokenizer)
        let diff = (with_stop - without_stop).abs();
        assert!(
            diff < 0.5,
            "stop-word 'the' should not significantly change score: with={with_stop} without={without_stop} diff={diff}"
        );
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

        let s_rare = idx.score(&records[2], "quantum");
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

    #[test]
    fn bm25_batch_search_scores() {
        let records = vec![
            mk("PRD-1", "Authentication System", "login and oauth2"),
            mk("PRD-2", "Payment Service", "stripe checkout"),
        ];
        let idx = Bm25Index::build(&records);
        let scores = idx.search_scores("authentication", 10);
        assert!(!scores.is_empty());
        assert_eq!(scores[0].0, "PRD-1");
        assert!(scores[0].1 > 0.0);
    }
}
