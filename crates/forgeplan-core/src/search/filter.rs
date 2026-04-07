//! Composable filter expressions for artifact search.
//!
//! Typed alternative to the flat `ArtifactFilter { kind, status }` pattern.
//! Supports AND/OR/NOT composition, date ranges, evidence presence.
//!
//! Pattern source: sources/RuVector/crates/ruvector-filter/src/expression.rs
//! (adapted to domain-specific types — no generic serde_json::Value)

use crate::db::store::ArtifactRecord;
use chrono::NaiveDateTime;

/// Predicate: does `tags` contain a tag matching `filter`?
///
/// Filter syntax:
/// - `"key=value"` → exact match against `"key=value"` entries.
/// - `"key"` (no `=`) → matches a bare `"key"` tag OR any `"key=..."` tag.
///
/// This is the canonical home for tag matching (Sprint 13.3 H1/H3 fix);
/// `artifact::frontmatter::has_tag_in` is a thin re-export.
pub fn has_tag_predicate(tags: &[String], filter: &str) -> bool {
    let (key, value) = match filter.split_once('=') {
        Some((k, v)) => (k.trim(), Some(v.trim())),
        None => (filter.trim(), None),
    };
    for t in tags {
        match value {
            Some(v) => {
                if let Some((k, val)) = t.split_once('=')
                    && k.trim() == key
                    && val.trim() == v
                {
                    return true;
                }
            }
            None => {
                if t == key {
                    return true;
                }
                if let Some((k, _)) = t.split_once('=')
                    && k.trim() == key
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Composable filter expression for artifact queries.
///
/// Typed to avoid generic-Value complexity. Each variant is a
/// domain-specific predicate over ArtifactRecord.
#[derive(Debug, Clone)]
pub enum ArtifactFilter {
    /// Match artifacts by kind (prd, rfc, adr, note, ...)
    Kind(String),
    /// Match artifacts by status (draft, active, superseded, deprecated, stale)
    Status(String),
    /// Match artifacts by depth (tactical, standard, deep, critical)
    Depth(String),
    /// Artifacts with evidence linked (R_eff > 0)
    HasEvidence,
    /// Artifacts without evidence (potential blind spots)
    NoEvidence,
    /// Created after a given date
    CreatedAfter(NaiveDateTime),
    /// Created before a given date
    CreatedBefore(NaiveDateTime),
    /// Title contains substring (case-insensitive)
    TitleContains(String),
    /// Match artifacts that have a specific tag.
    /// Filter is `"key=value"` for exact match or `"key"` for bare/prefix match.
    HasTag(String),
    /// All sub-filters must match
    And(Vec<ArtifactFilter>),
    /// Any sub-filter must match
    Or(Vec<ArtifactFilter>),
    /// Inner filter must NOT match
    Not(Box<ArtifactFilter>),
    /// Match everything (useful as default)
    Any,
}

impl ArtifactFilter {
    /// Evaluate this filter against a single record.
    pub fn matches(&self, record: &ArtifactRecord) -> bool {
        match self {
            Self::Kind(k) => record.kind.eq_ignore_ascii_case(k),
            Self::Status(s) => record.status.eq_ignore_ascii_case(s),
            Self::Depth(d) => record.depth.eq_ignore_ascii_case(d),
            Self::HasEvidence => record.r_eff_score > 0.0,
            Self::NoEvidence => record.r_eff_score == 0.0,
            Self::CreatedAfter(dt) => chrono::DateTime::parse_from_rfc3339(&record.created_at)
                .map(|parsed| parsed.naive_utc() > *dt)
                .unwrap_or(false),
            Self::CreatedBefore(dt) => chrono::DateTime::parse_from_rfc3339(&record.created_at)
                .map(|parsed| parsed.naive_utc() < *dt)
                .unwrap_or(false),
            Self::TitleContains(s) => record.title.to_lowercase().contains(&s.to_lowercase()),
            Self::HasTag(t) => has_tag_predicate(&record.tags, t),
            Self::And(filters) => filters.iter().all(|f| f.matches(record)),
            Self::Or(filters) => filters.iter().any(|f| f.matches(record)),
            Self::Not(filter) => !filter.matches(record),
            Self::Any => true,
        }
    }

    /// Convenience constructors
    pub fn and(filters: Vec<ArtifactFilter>) -> Self {
        Self::And(filters)
    }
    pub fn or(filters: Vec<ArtifactFilter>) -> Self {
        Self::Or(filters)
    }
    pub fn not(filter: ArtifactFilter) -> Self {
        Self::Not(Box::new(filter))
    }
}

impl Default for ArtifactFilter {
    fn default() -> Self {
        Self::Any
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(id: &str, kind: &str, status: &str, title: &str) -> ArtifactRecord {
        ArtifactRecord {
            id: id.to_string(),
            kind: kind.to_string(),
            status: status.to_string(),
            title: title.to_string(),
            body: String::new(),
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
    fn kind_filter() {
        let r = mk("PRD-1", "prd", "draft", "Auth");
        assert!(ArtifactFilter::Kind("prd".to_string()).matches(&r));
        assert!(ArtifactFilter::Kind("PRD".to_string()).matches(&r));
        assert!(!ArtifactFilter::Kind("rfc".to_string()).matches(&r));
    }

    #[test]
    fn status_filter() {
        let r = mk("PRD-1", "prd", "active", "X");
        assert!(ArtifactFilter::Status("active".to_string()).matches(&r));
        assert!(!ArtifactFilter::Status("draft".to_string()).matches(&r));
    }

    #[test]
    fn depth_filter() {
        let r = mk("PRD-1", "prd", "active", "X");
        assert!(ArtifactFilter::Depth("standard".to_string()).matches(&r));
        assert!(!ArtifactFilter::Depth("deep".to_string()).matches(&r));
    }

    #[test]
    fn has_evidence_filter() {
        let mut r = mk("PRD-1", "prd", "active", "X");
        r.r_eff_score = 0.5;
        assert!(ArtifactFilter::HasEvidence.matches(&r));
        assert!(!ArtifactFilter::NoEvidence.matches(&r));
        r.r_eff_score = 0.0;
        assert!(!ArtifactFilter::HasEvidence.matches(&r));
        assert!(ArtifactFilter::NoEvidence.matches(&r));
    }

    #[test]
    fn title_contains() {
        let r = mk("PRD-1", "prd", "draft", "Auth System");
        assert!(ArtifactFilter::TitleContains("auth".to_string()).matches(&r));
        assert!(ArtifactFilter::TitleContains("AUTH".to_string()).matches(&r));
        assert!(!ArtifactFilter::TitleContains("payment".to_string()).matches(&r));
    }

    #[test]
    fn created_after_before() {
        let mut r = mk("PRD-1", "prd", "draft", "X");
        r.created_at = "2026-03-15T12:00:00Z".to_string();
        let before =
            NaiveDateTime::parse_from_str("2026-03-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let after =
            NaiveDateTime::parse_from_str("2026-04-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        assert!(ArtifactFilter::CreatedAfter(before).matches(&r));
        assert!(!ArtifactFilter::CreatedAfter(after).matches(&r));
        assert!(ArtifactFilter::CreatedBefore(after).matches(&r));
        assert!(!ArtifactFilter::CreatedBefore(before).matches(&r));
    }

    #[test]
    fn and_filter() {
        let r = mk("PRD-1", "prd", "active", "Auth");
        let f = ArtifactFilter::and(vec![
            ArtifactFilter::Kind("prd".to_string()),
            ArtifactFilter::Status("active".to_string()),
        ]);
        assert!(f.matches(&r));

        let f_fail = ArtifactFilter::and(vec![
            ArtifactFilter::Kind("prd".to_string()),
            ArtifactFilter::Status("draft".to_string()),
        ]);
        assert!(!f_fail.matches(&r));
    }

    #[test]
    fn or_filter() {
        let r = mk("PRD-1", "prd", "draft", "X");
        let f = ArtifactFilter::or(vec![
            ArtifactFilter::Status("active".to_string()),
            ArtifactFilter::Status("draft".to_string()),
        ]);
        assert!(f.matches(&r));
    }

    #[test]
    fn not_filter() {
        let r = mk("PRD-1", "prd", "draft", "X");
        assert!(ArtifactFilter::not(ArtifactFilter::Status("active".to_string())).matches(&r));
        assert!(!ArtifactFilter::not(ArtifactFilter::Status("draft".to_string())).matches(&r));
    }

    #[test]
    fn any_filter() {
        let r = mk("PRD-1", "prd", "draft", "X");
        assert!(ArtifactFilter::Any.matches(&r));
        assert!(ArtifactFilter::default().matches(&r));
    }

    #[test]
    fn has_tag_filter_exact_and_bare() {
        let mut r = mk("PRD-1", "prd", "active", "X");
        r.tags = vec!["source=code".to_string(), "reviewed".to_string()];
        assert!(ArtifactFilter::HasTag("source=code".to_string()).matches(&r));
        assert!(ArtifactFilter::HasTag("source".to_string()).matches(&r));
        assert!(ArtifactFilter::HasTag("reviewed".to_string()).matches(&r));
        assert!(!ArtifactFilter::HasTag("source=docs".to_string()).matches(&r));
        assert!(!ArtifactFilter::HasTag("missing".to_string()).matches(&r));
    }

    #[test]
    fn test_has_tag_filter_composes_with_status_and_kind() {
        // H1 regression: tag must compose with kind+status via the DSL.
        let mut r = mk("PRD-1", "prd", "active", "Auth");
        r.tags = vec!["source=code".to_string()];
        let f = ArtifactFilter::and(vec![
            ArtifactFilter::HasTag("source=code".to_string()),
            ArtifactFilter::Status("active".to_string()),
            ArtifactFilter::Kind("prd".to_string()),
        ]);
        assert!(f.matches(&r));

        // Wrong kind → no match.
        let mut r2 = r.clone();
        r2.kind = "rfc".to_string();
        assert!(!f.matches(&r2));

        // Wrong tag → no match.
        let mut r3 = r.clone();
        r3.tags = vec!["source=docs".to_string()];
        assert!(!f.matches(&r3));
    }

    #[test]
    fn composable_and_or_not() {
        let r = mk("PRD-1", "prd", "active", "Auth System");
        let f = ArtifactFilter::and(vec![
            ArtifactFilter::or(vec![
                ArtifactFilter::Kind("prd".to_string()),
                ArtifactFilter::Kind("rfc".to_string()),
            ]),
            ArtifactFilter::not(ArtifactFilter::Status("deprecated".to_string())),
            ArtifactFilter::TitleContains("auth".to_string()),
        ]);
        assert!(f.matches(&r));
    }
}
