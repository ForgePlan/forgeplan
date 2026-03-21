use std::env;

use forgeplan_core::db::store::{ArtifactFilter, LanceStore};
use forgeplan_core::scoring::reff::{self, EvidenceItem, EvidenceType, Verdict};
use forgeplan_core::workspace;

pub async fn run(id: Option<&str>) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let target_id = id.ok_or_else(|| anyhow::anyhow!("Usage: forgeplan score <ID>"))?;

    let store = LanceStore::open(&ws).await?;

    // Get the target artifact
    let target = store
        .get_record(target_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", target_id))?;

    // Get relations FROM target (outgoing links)
    let outgoing = store.get_relations(target_id).await?;
    let evidence_targets: Vec<String> = outgoing
        .iter()
        .filter(|(_, rel)| rel == "informs" || rel == "based_on" || rel == "refines")
        .map(|(t, _)| t.clone())
        .collect();

    // Find all EvidencePack artifacts
    let filter = ArtifactFilter {
        kind: Some("evidence".to_string()),
        status: None,
    };
    let evidence_records = store.list_records(Some(&filter)).await?;

    let mut evidence_items: Vec<EvidenceItem> = Vec::new();

    for ev_record in &evidence_records {
        // Check if this evidence is linked from target
        let is_linked = evidence_targets
            .iter()
            .any(|eid| eid.eq_ignore_ascii_case(&ev_record.id));

        if !is_linked {
            // Check if evidence links TO target
            let ev_relations = store.get_relations(&ev_record.id).await?;
            let links_to_target = ev_relations
                .iter()
                .any(|(t, _)| t.eq_ignore_ascii_case(target_id));
            if !links_to_target {
                continue;
            }
        }

        let item = parse_evidence_from_record(ev_record);
        evidence_items.push(item);
    }

    // Compute R_eff
    let score = reff::r_eff(&evidence_items);

    println!();
    println!("{} \"{}\"", target.id, target.title);
    println!("{}", "-".repeat(50));

    if evidence_items.is_empty() {
        println!("  No evidence linked. R_eff = 0.0");
        println!();
        println!("  Hint: Create an EvidencePack and link it:");
        println!("    forgeplan new evidence \"Benchmark for {}\"", target.id);
        println!("    forgeplan link EVID-001 {} --relation informs", target.id);
    } else {
        println!("  Evidence breakdown:");
        for item in &evidence_items {
            let expired = item
                .valid_until
                .map(|dt| chrono::Utc::now().naive_utc() > dt)
                .unwrap_or(false);
            let item_score = reff::r_eff(&[item.clone()]);
            println!(
                "    {} [{:?}] CL{} = {:.1}{}",
                item.id,
                item.verdict,
                item.congruence_level,
                item_score,
                if expired { " (EXPIRED)" } else { "" }
            );
        }
        println!();

        let status = if score >= 0.5 {
            "Adequate"
        } else if score >= 0.3 {
            "Needs Review"
        } else {
            "AT RISK"
        };

        println!("  R_eff = {:.2} -- {}", score, status);
    }
    println!();

    Ok(())
}

/// Parse evidence fields from an ArtifactRecord's body.
/// Evidence metadata is stored in the body as YAML-like fields.
fn parse_evidence_from_record(
    record: &forgeplan_core::db::store::ArtifactRecord,
) -> EvidenceItem {
    // Parse verdict from body (look for "verdict:" line)
    let verdict = extract_field(&record.body, "verdict")
        .map(|s| match s.to_lowercase().as_str() {
            "supports" => Verdict::Supports,
            "weakens" => Verdict::Weakens,
            "refutes" => Verdict::Refutes,
            _ => Verdict::Supports,
        })
        .unwrap_or(Verdict::Supports);

    // Parse congruence_level
    let cl = extract_field(&record.body, "congruence_level")
        .and_then(|s| s.parse::<u8>().ok())
        .map(|v| v.min(3))
        .unwrap_or(0);

    let valid_until = record
        .valid_until
        .as_deref()
        .and_then(|s| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                .ok()
                .or_else(|| {
                    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .ok()
                        .and_then(|d| d.and_hms_opt(23, 59, 59))
                })
        });

    EvidenceItem {
        id: record.id.clone(),
        evidence_type: EvidenceType::Measurement,
        verdict,
        congruence_level: cl,
        valid_until,
    }
}

/// Extract a simple "key: value" from body text.
fn extract_field(body: &str, key: &str) -> Option<String> {
    let prefix = format!("{}:", key);
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
