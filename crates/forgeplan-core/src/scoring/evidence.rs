use crate::db::store::{ArtifactFilter, ArtifactRecord, LanceStore};
use crate::scoring::reff::{EvidenceItem, EvidenceType, Verdict};

/// Collect all evidence items linked to an artifact (bidirectional lookup).
///
/// Checks both directions:
/// 1. Outgoing from artifact: artifact → EVID (artifact links to evidence)
/// 2. Incoming to artifact: EVID → artifact (evidence links to artifact)
pub async fn collect_evidence_for(
    artifact_id: &str,
    store: &LanceStore,
) -> anyhow::Result<Vec<EvidenceItem>> {
    let outgoing = store.get_relations(artifact_id).await?;
    let outgoing_ids: std::collections::HashSet<String> = outgoing
        .iter()
        .map(|(target_id, _)| target_id.clone())
        .collect();

    let filter = ArtifactFilter {
        kind: Some("evidence".to_string()),
        status: None,
    };
    let all_evidence = store.list_records(Some(&filter)).await?;

    let mut items = Vec::new();
    for ev_record in &all_evidence {
        // Direction 1: artifact → evidence (outgoing)
        let linked_outgoing = outgoing_ids.contains(&ev_record.id);

        // Direction 2: evidence → artifact (incoming)
        let linked_incoming = if !linked_outgoing {
            let ev_rels = store.get_relations(&ev_record.id).await?;
            ev_rels
                .iter()
                .any(|(target, _)| target.eq_ignore_ascii_case(artifact_id))
        } else {
            false
        };

        if linked_outgoing || linked_incoming {
            items.push(parse_evidence_from_record(ev_record));
        }
    }

    Ok(items)
}

/// Preload all evidence items indexed by linked artifact ID (for recursive engine).
///
/// Returns a map: artifact_id → Vec<EvidenceItem> for all artifacts that have evidence.
pub async fn preload_evidence_map(
    store: &LanceStore,
) -> anyhow::Result<std::collections::HashMap<String, Vec<EvidenceItem>>> {
    let filter = ArtifactFilter {
        kind: Some("evidence".to_string()),
        status: None,
    };
    let all_evidence = store.list_records(Some(&filter)).await?;

    let mut map: std::collections::HashMap<String, Vec<EvidenceItem>> =
        std::collections::HashMap::new();

    for ev_record in &all_evidence {
        let ev_rels = store.get_relations(&ev_record.id).await?;
        let item = parse_evidence_from_record(ev_record);

        for (target_id, _) in &ev_rels {
            map.entry(target_id.clone())
                .or_default()
                .push(item.clone());
        }
    }

    Ok(map)
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

    EvidenceItem {
        id: record.id.clone(),
        evidence_type,
        verdict,
        congruence_level: cl,
        valid_until,
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

    // === PRD-016 Audit: parse_evidence_from_record tests ===

    fn make_record(body: &str, valid_until: Option<&str>) -> ArtifactRecord {
        ArtifactRecord {
            id: "EVID-001".into(),
            kind: "evidence".into(),
            status: "active".into(),
            title: "test evidence".into(),
            body: body.into(),
            depth: "tactical".into(),
            author: None,
            parent_epic: None,
            r_eff_score: 0.0,
            valid_until: valid_until.map(|s| s.to_string()),
            created_at: "2026-01-01T00:00:00".into(),
            updated_at: "2026-01-01T00:00:00".into(),
        }
    }

    #[test]
    fn parse_evidence_all_fields() {
        let body = "verdict: supports\ncongruence_level: 3\nevidence_type: test\n";
        let item = parse_evidence_from_record(&make_record(body, None));
        assert!(matches!(item.verdict, Verdict::Supports));
        assert_eq!(item.congruence_level, 3);
        assert!(matches!(item.evidence_type, EvidenceType::Test));
    }

    #[test]
    fn parse_evidence_type_fallback_to_type_key() {
        let body = "verdict: weakens\ncongruence_level: 2\ntype: benchmark\n";
        let item = parse_evidence_from_record(&make_record(body, None));
        assert!(matches!(item.evidence_type, EvidenceType::Benchmark));
        assert!(matches!(item.verdict, Verdict::Weakens));
    }

    #[test]
    fn parse_evidence_type_unknown_defaults_to_measurement() {
        let body = "verdict: supports\nevidence_type: something_unknown\n";
        let item = parse_evidence_from_record(&make_record(body, None));
        assert!(matches!(item.evidence_type, EvidenceType::Measurement));
    }

    #[test]
    fn parse_evidence_valid_until_date_format() {
        let body = "verdict: supports\n";
        let item = parse_evidence_from_record(&make_record(body, Some("2025-06-15")));
        let vu = item.valid_until.expect("should parse date");
        assert_eq!(vu.date().to_string(), "2025-06-15");
        // Date-only parses to 23:59:59
        assert_eq!(vu.time().to_string(), "23:59:59");
    }

    #[test]
    fn parse_evidence_valid_until_datetime_format() {
        let body = "verdict: supports\n";
        let item = parse_evidence_from_record(&make_record(body, Some("2025-06-15T12:30:00")));
        let vu = item.valid_until.expect("should parse datetime");
        assert_eq!(vu.to_string(), "2025-06-15 12:30:00");
    }

    #[test]
    fn parse_evidence_cl_clamped_at_3() {
        let body = "verdict: supports\ncongruence_level: 99\n";
        let item = parse_evidence_from_record(&make_record(body, None));
        assert_eq!(item.congruence_level, 3, "CL should be clamped to 3");
    }
}
