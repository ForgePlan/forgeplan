use std::collections::HashSet;
use std::env;

use chrono::{NaiveDate, Utc};

use forgeplan_core::artifact::frontmatter::Frontmatter;
use forgeplan_core::artifact::types::{ArtifactKind, Mode};
use forgeplan_core::db::store::LanceStore;
use forgeplan_core::scoring::evidence::collect_evidence_for;
use forgeplan_core::scoring::fgr;
use forgeplan_core::scoring::reff;
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

    // --- Recursive R_eff via AssuranceReport ---
    let mut visited = HashSet::new();
    let report = reff::r_eff_recursive(target_id, &store, &mut visited).await?;

    // --- Evidence list via shared bidirectional lookup ---
    let evidence_items = collect_evidence_for(target_id, &store).await?;

    // --- F-G-R computation ---
    let kind: ArtifactKind = target.kind.parse().unwrap_or(ArtifactKind::Note);
    let depth: Mode = target.depth.parse().unwrap_or(Mode::Standard);
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

    // --- Display ---
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

        println!("  R_eff = {:.2} -- {}", report.r_eff, status);
    }

    // Weakest link
    if let Some(ref wl) = report.weakest_link {
        println!("  Weakest link: {}", wl);
    }

    // Factors
    if !report.factors.is_empty() {
        println!();
        for factor in &report.factors {
            println!("  \u{2022} {}", factor);
        }
    }

    // F-G-R breakdown
    println!();
    println!("  Quality (F-G-R):");
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
    let mut hints: Vec<&str> = Vec::new();
    if fgr_score.formality < 0.4 {
        hints.push("Hint: Fill in required sections to improve formality");
    }
    if fgr_score.granularity < 0.4 {
        hints.push("Hint: Add more FR checkboxes to improve granularity");
    }
    if fgr_score.reliability < 0.3 {
        hints.push("Hint: Add evidence with `forgeplan new evidence`");
    }

    if !hints.is_empty() {
        println!();
        for hint in &hints {
            println!("  {}", hint);
        }
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

