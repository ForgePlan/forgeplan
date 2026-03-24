use std::env;

use forgeplan_core::artifact::frontmatter;
use forgeplan_core::db::store::LanceStore;
use forgeplan_core::scoring::fgr;
use forgeplan_core::workspace;

pub async fn run(id: Option<&str>, json: bool) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;

    let records = if let Some(id) = id {
        let record = store
            .get_record(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Artifact not found: {id}"))?;
        vec![record]
    } else {
        store.list_records(None).await?
    };

    if records.is_empty() {
        if json { println!("[]"); } else { println!("No artifacts to score."); }
        return Ok(());
    }

    if json {
        let mut results = Vec::new();
        for record in &records {
            let kind = record.kind.parse().unwrap_or(forgeplan_core::artifact::types::ArtifactKind::Note);
            let depth = record.depth.parse().unwrap_or(forgeplan_core::artifact::types::Mode::Standard);
            let fm = frontmatter::parse_frontmatter(&record.body).map(|(fm, _)| fm).unwrap_or_default();
            let relations = store.get_relations(&record.id).await.unwrap_or_default();
            let is_stale = record.valid_until.as_ref().is_some_and(|v| {
                chrono::NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S")
                    .map(|dt| chrono::Utc::now().naive_utc() > dt).unwrap_or(false)
            });
            let score = fgr::compute(&record.id, &record.body, &fm, &kind, &depth, record.r_eff_score, relations.len(), is_stale);
            results.push(serde_json::json!({
                "id": record.id, "title": record.title,
                "formality": score.formality, "granularity": score.granularity,
                "reliability": score.reliability, "overall": score.overall(), "grade": score.grade(),
            }));
        }
        println!("{}", serde_json::to_string_pretty(&results)?);
        return Ok(());
    }

    println!(
        "{:<12} {:<30} {:>6} {:>6} {:>6} {:>5}",
        "ID", "Title", "F", "G", "R", "Grade"
    );
    println!("{}", "-".repeat(70));

    for record in &records {
        let kind = record.kind.parse().unwrap_or(forgeplan_core::artifact::types::ArtifactKind::Note);
        let depth = record.depth.parse().unwrap_or(forgeplan_core::artifact::types::Mode::Standard);
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
        );

        let title: String = record.title.chars().take(28).collect();
        println!(
            "{:<12} {:<30} {:>5.2} {:>5.2} {:>5.2} {:>5}",
            record.id, title, score.formality, score.granularity, score.reliability, score.grade()
        );
    }

    Ok(())
}
