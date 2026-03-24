use std::collections::HashSet;
use std::env;

use chrono::{NaiveDate, Utc};

use forgeplan_core::artifact::frontmatter::Frontmatter;
use forgeplan_core::artifact::types::{ArtifactKind, Mode};
use forgeplan_core::db::store::{ArtifactFilter, LanceStore};
use forgeplan_core::scoring::fgr;
use forgeplan_core::scoring::reff::{self, EvidenceItem, EvidenceType, Verdict};
use forgeplan_core::workspace;

use crate::ui;

pub async fn run(id: Option<&str>, json: bool) -> anyhow::Result<()> {
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

    // --- Recursive R_eff via AssuranceReport ---
    let mut visited = HashSet::new();
    let report = reff::r_eff_recursive(target_id, &store, &mut visited).await?;

    // --- Flat evidence list for display (backward-compat) ---
    let outgoing = store.get_relations(target_id).await?;
    let evidence_targets: Vec<String> = outgoing
        .iter()
        .filter(|(_, rel)| rel == "informs" || rel == "based_on" || rel == "refines")
        .map(|(t, _)| t.clone())
        .collect();

    let filter = ArtifactFilter {
        kind: Some("evidence".to_string()),
        status: None,
    };
    let evidence_records = store.list_records(Some(&filter)).await?;

    let mut evidence_items: Vec<EvidenceItem> = Vec::new();

    for ev_record in &evidence_records {
        let is_linked = evidence_targets
            .iter()
            .any(|eid| eid.eq_ignore_ascii_case(&ev_record.id));

        if !is_linked {
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

    // --- F-G-R computation ---
    let kind: ArtifactKind = target.kind.parse().unwrap_or_else(|_| {
        eprintln!("  Warning: unknown kind '{}', defaulting to Note", target.kind);
        ArtifactKind::Note
    });
    let depth: Mode = target.depth.parse().unwrap_or_else(|_| {
        eprintln!("  Warning: unknown depth '{}', defaulting to Standard", target.depth);
        Mode::Standard
    });
    let frontmatter: Frontmatter = Frontmatter::new();

    // Determine staleness from valid_until
    let is_stale = target
        .valid_until
        .as_deref()
        .and_then(|s| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .ok()
                .or_else(|| {
                    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                        .ok()
                        .map(|dt| dt.date())
                })
        })
        .map(|d| Utc::now().date_naive() > d)
        .unwrap_or(false);

    // Link count for reliability
    let all_relations = store.get_all_relations().await?;
    let link_count = all_relations
        .iter()
        .filter(|(src, tgt, _)| src == target_id || tgt == target_id)
        .count();

    let fgr_score = fgr::compute(
        target_id,
        &target.body,
        &frontmatter,
        &kind,
        &depth,
        report.r_eff,
        link_count,
        is_stale,
    );

    // --- JSON output ---
    if json {
        let evidence_json: Vec<_> = evidence_items
            .iter()
            .map(|item| {
                let item_score = reff::r_eff(&[item.clone()]);
                serde_json::json!({
                    "id": item.id,
                    "verdict": format!("{:?}", item.verdict),
                    "congruence_level": item.congruence_level,
                    "score": item_score,
                    "expired": item.valid_until.map(|dt| Utc::now().naive_utc() > dt).unwrap_or(false),
                })
            })
            .collect();

        let json_data = serde_json::json!({
            "id": target.id,
            "title": target.title,
            "r_eff": report.r_eff,
            "weakest_link": report.weakest_link,
            "factors": report.factors,
            "evidence": evidence_json,
            "fgr": {
                "formality": fgr_score.formality,
                "granularity": fgr_score.granularity,
                "reliability": fgr_score.reliability,
                "overall": fgr_score.overall(),
                "grade": fgr_score.grade(),
            },
        });
        println!("{}", serde_json::to_string_pretty(&json_data)?);
        return Ok(());
    }

    // --- Styled display ---
    ui::header(&target.id, &target.title);

    if evidence_items.is_empty() {
        ui::info("No evidence linked. R_eff = 0.0");
        println!();
        ui::error_hint(
            "No evidence found",
            &format!(
                "forgeplan new evidence \"Benchmark for {}\" && forgeplan link EVID-XXX {} --relation informs",
                target.id, target.id
            ),
        );
    } else {
        ui::section("Evidence breakdown");
        for item in &evidence_items {
            let expired = item
                .valid_until
                .map(|dt| Utc::now().naive_utc() > dt)
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

        let status = if report.r_eff >= 0.5 {
            "Adequate"
        } else if report.r_eff >= 0.3 {
            "Needs Review"
        } else {
            "AT RISK"
        };

        ui::kv("R_eff", &format!("{} -- {}", ui::styled_reff(report.r_eff), status));
    }

    if let Some(ref wl) = report.weakest_link {
        ui::kv("Weakest link", wl);
    }

    if !report.factors.is_empty() {
        println!();
        for factor in &report.factors {
            println!("  \u{2022} {}", factor);
        }
    }

    ui::section("Quality (F-G-R)");
    println!(
        "    Formality:    {:.2} ({})",
        fgr_score.formality,
        score_grade(fgr_score.formality)
    );
    println!(
        "    Granularity:  {:.2} ({})",
        fgr_score.granularity,
        score_grade(fgr_score.granularity)
    );
    println!(
        "    Reliability:  {:.2} ({})",
        fgr_score.reliability,
        score_grade(fgr_score.reliability)
    );
    println!(
        "    Overall:      {:.2} ({})",
        fgr_score.overall(),
        fgr_score.grade()
    );

    // Hints for low scores
    if fgr_score.formality < 0.4 {
        ui::warning("Fill in required sections to improve formality");
    }
    if fgr_score.granularity < 0.4 {
        ui::warning("Add more FR checkboxes to improve granularity");
    }
    if fgr_score.reliability < 0.3 {
        ui::warning("Add evidence with `forgeplan new evidence`");
    }

    println!();

    Ok(())
}

/// Per-dimension grade (same thresholds as FgrScore::grade but for a single value).
fn score_grade(v: f64) -> &'static str {
    if v > 0.8 {
        "A"
    } else if v > 0.6 {
        "B"
    } else if v > 0.4 {
        "C"
    } else if v > 0.2 {
        "D"
    } else {
        "F"
    }
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
                    NaiveDate::parse_from_str(s, "%Y-%m-%d")
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
