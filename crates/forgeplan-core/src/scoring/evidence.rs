use crate::db::store::ArtifactRecord;
use crate::scoring::reff::{EvidenceItem, EvidenceType, Verdict};

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

    // Default CL=3 (same context) — evidence created locally is same-context by default.
    let cl = extract_field(&record.body, "congruence_level")
        .and_then(|s| s.parse::<u8>().ok())
        .map(|v| v.min(3))
        .unwrap_or(3);

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
        };
        let item = parse_evidence_from_record(&record);
        assert_eq!(item.congruence_level, 3, "Should be CL3, not CL0");
        assert!(matches!(item.verdict, Verdict::Supports));
        assert!(matches!(item.evidence_type, EvidenceType::Test));
    }
}
