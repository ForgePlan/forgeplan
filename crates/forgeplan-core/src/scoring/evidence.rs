use crate::db::store::ArtifactRecord;
use crate::scoring::reff::{EvidenceItem, EvidenceType, Verdict};
use serde::{Deserialize, Serialize};

/// Source tier for discovery findings.
/// Maps to congruence level when evidence is created from tiered sources.
///
/// - T1 (authoritative): code files, git log, package manifests → CL3 (no penalty)
/// - T2 (extracted): tests, JSDoc, CI configs → CL2 (0.1 penalty)
/// - T3 (supplementary): docs/, README, legacy documentation → CL1 (0.4 penalty)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTier {
    /// Tier 1: authoritative — code files, git log, package manifests.
    T1,
    /// Tier 2: extracted — tests, JSDoc, CI configs.
    T2,
    /// Tier 3: supplementary — docs/, README, legacy (may be stale).
    T3,
}

impl SourceTier {
    /// Map tier to congruence level for R_eff computation.
    /// T1 → CL3 (no penalty), T2 → CL2 (0.1 penalty), T3 → CL1 (0.4 penalty).
    pub fn to_congruence_level(&self) -> u8 {
        match self {
            Self::T1 => 3,
            Self::T2 => 2,
            Self::T3 => 1,
        }
    }

    /// Parse from string — accepts "t1"/"tier1"/"tier-1"/"1" and case variants.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "t1" | "tier1" | "tier-1" | "1" => Some(Self::T1),
            "t2" | "tier2" | "tier-2" | "2" => Some(Self::T2),
            "t3" | "tier3" | "tier-3" | "3" => Some(Self::T3),
            _ => None,
        }
    }
}

/// Parse evidence metadata from an ArtifactRecord's body fields.
pub fn parse_evidence_from_record(record: &ArtifactRecord) -> EvidenceItem {
    let verdict = extract_field(&record.body, "verdict")
        .map(|s| match s.to_lowercase().as_str() {
            "supports" => Verdict::Supports,
            "weakens" => Verdict::Weakens,
            "refutes" => Verdict::Refutes,
            _ => Verdict::Supports,
        })
        .unwrap_or(Verdict::Supports);

    // source_tier (if present) takes precedence over congruence_level.
    // This enables discovery findings to auto-map tier → CL for R_eff.
    let tier_cl = extract_field(&record.body, "source_tier")
        .and_then(|s| SourceTier::parse(&s))
        .map(|t| t.to_congruence_level());

    let explicit_cl = extract_field(&record.body, "congruence_level")
        .and_then(|s| s.parse::<u8>().ok())
        .map(|v| v.min(3));

    if tier_cl.is_some() && explicit_cl.is_some() {
        eprintln!(
            "warn: evidence {} has both source_tier and congruence_level; source_tier takes precedence",
            record.id
        );
    }

    // Default CL=3 (same context) — evidence created locally is same-context by default.
    let cl = tier_cl.or(explicit_cl).unwrap_or(3);

    let valid_until = record.valid_until.as_deref().and_then(|s| {
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
            .ok()
            .or_else(|| {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(23, 59, 59))
            })
    });

    let evidence_type = extract_field(&record.body, "evidence_type")
        .or_else(|| extract_field(&record.body, "type"))
        .map(|s| match s.to_lowercase().as_str() {
            "test" => EvidenceType::Test,
            "measurement" => EvidenceType::Measurement,
            "benchmark" => EvidenceType::Benchmark,
            "audit" => EvidenceType::Audit,
            _ => EvidenceType::Measurement,
        })
        .unwrap_or(EvidenceType::Measurement);

    EvidenceItem {
        id: record.id.clone(),
        evidence_type,
        verdict,
        congruence_level: cl,
        valid_until,
    }
}

/// Extract a simple "key: value" from body text.
///
/// Skips markdown table rows (lines starting with `|`) and HTML comments
/// to avoid matching template placeholder values like `| CL | 0 / 1 / 2 / 3 |`.
/// Only matches standalone `key: value` lines (structured fields).
pub fn extract_field(body: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    for line in body.lines() {
        let trimmed = line.trim();
        // Skip markdown table rows and HTML comments — these are template placeholders
        if trimmed.starts_with('|') || trimmed.starts_with("<!--") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let val = rest.trim();
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_field_basic() {
        let body = "verdict: supports\ncongruence_level: 2\n";
        assert_eq!(extract_field(body, "verdict"), Some("supports".into()));
        assert_eq!(extract_field(body, "congruence_level"), Some("2".into()));
        assert_eq!(extract_field(body, "missing"), None);
    }

    #[test]
    fn extract_field_with_whitespace() {
        let body = "  verdict:   Weakens  \n";
        assert_eq!(extract_field(body, "verdict"), Some("Weakens".into()));
    }

    #[test]
    fn extract_field_ignores_table_rows() {
        let body = "| Type | measurement / test / benchmark / audit |\n| Verdict | supports / weakens / refutes |\n| CL | 0 / 1 / 2 / 3 |\n";
        // Table rows should NOT match any field
        assert_eq!(extract_field(body, "type"), None);
        assert_eq!(extract_field(body, "verdict"), None);
        assert_eq!(extract_field(body, "congruence_level"), None);
    }

    #[test]
    fn extract_field_ignores_html_comments() {
        let body = "<!-- congruence_level: 0 | 1 | 2 | 3 -->\ncongruence_level: 3\n";
        assert_eq!(extract_field(body, "congruence_level"), Some("3".into()));
    }

    #[test]
    fn extract_field_structured_field_wins_over_template_placeholders() {
        // Simulates body with template table AND user-added structured fields
        let body = r#"| Field | Value |
|-------|-------|
| Status | Draft |
| Type | measurement / test / benchmark / audit |
| Verdict | supports / weakens / refutes |
| CL | 0 / 1 / 2 / 3 |

## Structured Fields

evidence_type: test
verdict: supports
congruence_level: 3
"#;
        assert_eq!(extract_field(body, "verdict"), Some("supports".into()));
        assert_eq!(extract_field(body, "congruence_level"), Some("3".into()));
        assert_eq!(extract_field(body, "evidence_type"), Some("test".into()));
    }

    fn mk_record(body: &str) -> ArtifactRecord {
        ArtifactRecord {
            id: "EVID-999".into(),
            kind: "evidence".into(),
            status: "draft".into(),
            title: "t".into(),
            body: body.into(),
            depth: "tactical".into(),
            author: None,
            parent_epic: None,
            r_eff_score: 0.0,
            valid_until: None,
            created_at: String::new(),
            updated_at: String::new(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn source_tier_to_cl_mapping() {
        assert_eq!(SourceTier::T1.to_congruence_level(), 3);
        assert_eq!(SourceTier::T2.to_congruence_level(), 2);
        assert_eq!(SourceTier::T3.to_congruence_level(), 1);
    }

    #[test]
    fn source_tier_parse_variants() {
        assert_eq!(SourceTier::parse("t1"), Some(SourceTier::T1));
        assert_eq!(SourceTier::parse("T1"), Some(SourceTier::T1));
        assert_eq!(SourceTier::parse("tier1"), Some(SourceTier::T1));
        assert_eq!(SourceTier::parse("TIER-2"), Some(SourceTier::T2));
        assert_eq!(SourceTier::parse("  tier-3 "), Some(SourceTier::T3));
        assert_eq!(SourceTier::parse("3"), Some(SourceTier::T3));
        assert_eq!(SourceTier::parse("bogus"), None);
        assert_eq!(SourceTier::parse("t4"), None);
    }

    #[test]
    fn evidence_body_with_source_tier_maps_to_cl() {
        let body = "source_tier: t2\nevidence_type: test\n";
        let item = parse_evidence_from_record(&mk_record(body));
        assert_eq!(item.congruence_level, 2);
    }

    #[test]
    fn evidence_body_source_tier_t1_maps_to_cl3() {
        let body = "source_tier: tier1\n";
        let item = parse_evidence_from_record(&mk_record(body));
        assert_eq!(item.congruence_level, 3);
    }

    #[test]
    fn evidence_body_source_tier_t3_maps_to_cl1() {
        let body = "source_tier: 3\n";
        let item = parse_evidence_from_record(&mk_record(body));
        assert_eq!(item.congruence_level, 1);
    }

    #[test]
    fn evidence_body_source_tier_takes_precedence_over_cl() {
        let body = "source_tier: t1\ncongruence_level: 0\n";
        let item = parse_evidence_from_record(&mk_record(body));
        assert_eq!(
            item.congruence_level, 3,
            "source_tier must win over explicit congruence_level"
        );
    }

    #[test]
    fn evidence_body_invalid_source_tier_falls_back_to_cl() {
        let body = "source_tier: bogus\ncongruence_level: 2\n";
        let item = parse_evidence_from_record(&mk_record(body));
        assert_eq!(item.congruence_level, 2);
    }

    #[test]
    fn parse_evidence_with_template_placeholders_returns_cl3() {
        // Body has both template table placeholders AND structured fields
        let body = r#"| Type | measurement / test / benchmark / audit |
| Verdict | supports / weakens / refutes |
| CL | 0 / 1 / 2 / 3 |

## Structured Fields

evidence_type: test
verdict: supports
congruence_level: 3
"#;
        let record = ArtifactRecord {
            id: "EVID-001".into(),
            kind: "evidence".into(),
            status: "draft".into(),
            title: "Test evidence".into(),
            body: body.into(),
            depth: "tactical".into(),
            author: None,
            parent_epic: None,
            r_eff_score: 0.0,
            valid_until: None,
            created_at: String::new(),
            updated_at: String::new(),
            tags: Vec::new(),
        };
        let item = parse_evidence_from_record(&record);
        assert_eq!(item.congruence_level, 3, "Should be CL3, not CL0");
        assert!(matches!(item.verdict, Verdict::Supports));
        assert!(matches!(item.evidence_type, EvidenceType::Test));
    }
}
