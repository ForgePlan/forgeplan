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

    let cl = extract_field(&record.body, "congruence_level")
        .and_then(|s| s.parse::<u8>().ok())
        .map(|v| v.min(3))
        .unwrap_or(0);

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

    let formality_level = extract_field(&record.body, "formality_level")
        .or_else(|| extract_field(&record.body, "formality"))
        .and_then(|s| s.parse::<u8>().ok())
        .map(|v| v.min(9))
        .unwrap_or(5);

    EvidenceItem {
        id: record.id.clone(),
        evidence_type,
        verdict,
        congruence_level: cl,
        valid_until,
        formality_level,
    }
}

/// Extract a simple "key: value" from body text.
pub fn extract_field(body: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    for line in body.lines() {
        let trimmed = line.trim();
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
}
