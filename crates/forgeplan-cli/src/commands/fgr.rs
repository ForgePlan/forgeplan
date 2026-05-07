use forgeplan_core::artifact::frontmatter;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::scoring::fgr;

use crate::commands::common;

pub async fn run(id: Option<&str>, json: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    let fpf_weights = common::config().ok().and_then(|c| c.fpf.map(|f| f.weights));

    let records = if let Some(id) = id {
        // PROB-060 / SPEC-005 Phase 2.6 (CD-6) — accept slug or display id.
        let canonical = store
            .resolve_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found\nFix: forgeplan list"))?;
        let record = store
            .get_record(&canonical)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Artifact not found: {canonical}"))?;
        vec![record]
    } else {
        store.list_records(None).await?
    };

    if records.is_empty() {
        let hint_list = vec![
            Hint::info("Create your first artifact")
                .with_action("forgeplan new prd \"<title>\"".to_string()),
        ];
        if json {
            let payload = serde_json::json!({
                "results": [],
                "_next_action": hints::primary_action(&hint_list),
                "hints": hint_list,
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!("No artifacts to score.");
            print!("{}", hints::render_next_action_line(&hint_list));
        }
        return Ok(());
    }

    if json {
        let mut results = Vec::new();
        // PROB-060 (W1.B, CD-5) — track the lowest-grade record's ref_form
        // (slug pre-merge / display id post-merge) so the agent's next
        // command stays canonical for commit `Refs:`.
        let mut lowest: Option<(String, String, f64)> = None;
        for record in &records {
            let kind = record
                .kind
                .parse()
                .unwrap_or(forgeplan_core::artifact::types::ArtifactKind::Note);
            let depth = record
                .depth
                .parse()
                .unwrap_or(forgeplan_core::artifact::types::Mode::Standard);
            let fm = frontmatter::parse_frontmatter(&record.body)
                .map(|(fm, _)| fm)
                .unwrap_or_default();
            let relations = store.get_relations(&record.id).await.unwrap_or_default();
            let is_stale = record.valid_until.as_ref().is_some_and(|v| {
                chrono::NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S")
                    .map(|dt| chrono::Utc::now().naive_utc() > dt)
                    .or_else(|_| {
                        chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d")
                            .map(|d| chrono::Utc::now().date_naive() > d)
                    })
                    .unwrap_or(false)
            });
            let score = fgr::compute(
                &record.id,
                &record.body,
                &fm,
                &kind,
                &depth,
                record.r_eff_score,
                relations.len(),
                is_stale,
                fpf_weights.as_ref(),
            );
            let overall = score.overall();
            let ref_form =
                forgeplan_core::artifact::frontmatter::refs_form(&fm, &record.id).to_string();
            results.push(serde_json::json!({
                "id": record.id, "title": record.title,
                "formality": score.formality, "granularity": score.granularity,
                "reliability": score.reliability, "overall": overall, "grade": score.grade(),
            }));
            if lowest.as_ref().is_none_or(|(_, _, v)| overall < *v) {
                lowest = Some((record.id.clone(), ref_form, overall));
            }
        }
        let hint_list = if let Some((_id, target_ref, _)) = lowest {
            vec![
                Hint::info(format!("Improve lowest-grade artifact {}", target_ref))
                    .with_action(format!("forgeplan get {}", target_ref)),
            ]
        } else {
            Vec::new()
        };
        let payload = serde_json::json!({
            "results": results,
            "_next_action": hints::primary_action(&hint_list),
            "hints": hint_list,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!(
        "{:<12} {:<30} {:>6} {:>6} {:>6} {:>5}",
        "ID", "Title", "F", "G", "R", "Grade"
    );
    println!("{}", "-".repeat(70));

    // PROB-060 (W1.B, CD-5) — track ref_form of lowest-grade record so the
    // emitted hint stays canonical (slug pre-merge / display id post-merge).
    let mut lowest: Option<(String, String, f64)> = None;
    for record in &records {
        let kind = record
            .kind
            .parse()
            .unwrap_or(forgeplan_core::artifact::types::ArtifactKind::Note);
        let depth = record
            .depth
            .parse()
            .unwrap_or(forgeplan_core::artifact::types::Mode::Standard);
        let fm = frontmatter::parse_frontmatter(&record.body)
            .map(|(fm, _)| fm)
            .unwrap_or_default();

        let relations = store.get_relations(&record.id).await.unwrap_or_default();
        let is_stale = record.valid_until.as_ref().is_some_and(|v| {
            chrono::NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S")
                .map(|dt| chrono::Utc::now().naive_utc() > dt)
                .unwrap_or(false)
        });

        let score = fgr::compute(
            &record.id,
            &record.body,
            &fm,
            &kind,
            &depth,
            record.r_eff_score,
            relations.len(),
            is_stale,
            fpf_weights.as_ref(),
        );

        let title: String = record.title.chars().take(28).collect();
        println!(
            "{:<12} {:<30} {:>5.2} {:>5.2} {:>5.2} {:>5}",
            record.id,
            title,
            score.formality,
            score.granularity,
            score.reliability,
            score.grade()
        );

        let overall = score.overall();
        let ref_form =
            forgeplan_core::artifact::frontmatter::refs_form(&fm, &record.id).to_string();
        if lowest.as_ref().is_none_or(|(_, _, v)| overall < *v) {
            lowest = Some((record.id.clone(), ref_form, overall));
        }
    }

    let hint_list = if let Some((_id, target_ref, _)) = lowest {
        vec![
            Hint::info(format!("Improve lowest-grade artifact {}", target_ref))
                .with_action(format!("forgeplan get {}", target_ref)),
        ]
    } else {
        Vec::new()
    };
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}
