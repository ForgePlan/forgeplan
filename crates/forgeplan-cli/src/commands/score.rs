use forgeplan_core::artifact::frontmatter;
use forgeplan_core::artifact::store;
use forgeplan_core::link;
use forgeplan_core::scoring::reff::{self, EvidenceItem, EvidenceType, Verdict};
use forgeplan_core::workspace;

pub async fn run(id: Option<&str>) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let target_id = id.ok_or_else(|| anyhow::anyhow!("Usage: forgeplan score <ID>"))?;

    let artifacts = store::list_artifacts(&ws).await?;

    // Find the target artifact
    let target = artifacts
        .iter()
        .find(|a| a.id.eq_ignore_ascii_case(target_id))
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", target_id))?;

    let content = tokio::fs::read_to_string(&target.path).await?;
    let (fm, _) = frontmatter::parse_frontmatter(&content)?;

    // Collect evidence from linked EvidencePack artifacts
    let links = link::list_links(&fm);
    let evidence_ids: Vec<String> = links
        .iter()
        .filter(|(_, rel)| rel == "informs" || rel == "based_on" || rel == "refines")
        .map(|(target, _)| target.clone())
        .collect();

    // Also scan for EvidencePack artifacts that link TO this artifact
    let mut evidence_items: Vec<EvidenceItem> = Vec::new();

    for artifact in &artifacts {
        if artifact.kind.to_lowercase() != "evidence" && artifact.kind.to_lowercase() != "evidencepack" {
            continue;
        }

        let ev_content = tokio::fs::read_to_string(&artifact.path).await?;
        let ev_fm = match frontmatter::parse_frontmatter(&ev_content) {
            Ok((fm, _)) => fm,
            Err(_) => continue,
        };

        // Check if this evidence is linked from target or links to target
        let is_linked = evidence_ids.iter().any(|eid| eid.eq_ignore_ascii_case(&artifact.id));

        if !is_linked {
            let ev_links = link::list_links(&ev_fm);
            let links_to_target = ev_links
                .iter()
                .any(|(t, _)| t.eq_ignore_ascii_case(target_id));
            if !links_to_target {
                continue;
            }
        }

        let item = parse_evidence_item(&artifact.id, &ev_fm);
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
        println!("    forgeplan link EVID-001 --informs {}", target.id);
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

fn parse_evidence_item(id: &str, fm: &frontmatter::Frontmatter) -> EvidenceItem {
    let verdict = fm
        .get("verdict")
        .and_then(|v| v.as_str())
        .map(|s| match s.to_lowercase().as_str() {
            "supports" => Verdict::Supports,
            "weakens" => Verdict::Weakens,
            "refutes" => Verdict::Refutes,
            _ => Verdict::Supports,
        })
        .unwrap_or(Verdict::Supports);

    let cl = fm
        .get("congruence_level")
        .and_then(|v| v.as_u64())
        .map(|v| v.min(3) as u8)
        .unwrap_or(0); // conservative default: absent CL = highest penalty

    let valid_until = fm
        .get("valid_until")
        .and_then(|v| v.as_str())
        .and_then(|s| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok()
                .or_else(|| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
                    .map(|d| d.and_hms_opt(23, 59, 59).unwrap()))
        });

    EvidenceItem {
        id: id.to_string(),
        evidence_type: EvidenceType::Measurement,
        verdict,
        congruence_level: cl,
        valid_until,
    }
}
