use std::collections::BTreeMap;
use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::progress::{self, ArtifactProgress};
use forgeplan_core::workspace;

pub async fn run(id: Option<&str>) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let records = store.list_records(None).await?;

    if records.is_empty() {
        println!("No artifacts found.");
        return Ok(());
    }

    // Build progress for each artifact
    let mut all_progress: Vec<ArtifactProgress> = records
        .iter()
        .map(|r| {
            let (total, completed) = progress::count_checkboxes(&r.body);
            ArtifactProgress {
                id: r.id.clone(),
                title: r.title.clone(),
                kind: r.kind.clone(),
                total,
                completed,
            }
        })
        .collect();

    // If specific ID requested, show only that artifact
    if let Some(target_id) = id {
        let upper = target_id.to_uppercase();
        let filtered: Vec<_> = all_progress
            .into_iter()
            .filter(|p| p.id.to_uppercase() == upper)
            .collect();

        if filtered.is_empty() {
            anyhow::bail!("Artifact '{}' not found", target_id);
        }

        let p = &filtered[0];
        println!();
        println!("{} \"{}\"", p.id, p.title);
        println!("{}", "─".repeat(50));

        if p.total == 0 {
            println!("  No checkboxes found in this artifact.");
        } else {
            println!(
                "  {}  {}/{}  ({}%)  {}",
                progress::render_bar(p.ratio(), 24),
                p.completed,
                p.total,
                p.percent(),
                p.status_icon()
            );

            // Show individual checkbox lines
            let record = records
                .iter()
                .find(|r| r.id.to_uppercase() == upper)
                .unwrap();

            println!();
            for line in record.body.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ")
                    || trimmed.starts_with("* [x] ") || trimmed.starts_with("* [X] ")
                {
                    println!("  [x] {}", trimmed.get(6..).unwrap_or(""));
                } else if trimmed.starts_with("- [ ] ") || trimmed.starts_with("* [ ] ") {
                    println!("  [ ] {}", trimmed.get(6..).unwrap_or(""));
                }
            }
        }
        println!();
        return Ok(());
    }

    // Filter to artifacts with checkboxes
    all_progress.retain(|p| p.total > 0);

    if all_progress.is_empty() {
        println!("No artifacts with checkboxes found.");
        return Ok(());
    }

    // Group by kind
    let mut by_kind: BTreeMap<String, Vec<&ArtifactProgress>> = BTreeMap::new();
    for p in &all_progress {
        by_kind.entry(p.kind.clone()).or_default().push(p);
    }

    let id_width = all_progress
        .iter()
        .map(|p| p.id.len())
        .max()
        .unwrap_or(8)
        .max(4);

    println!();
    println!("Forgeplan Progress");
    println!("==================");

    for (kind, items) in &by_kind {
        println!();
        println!("  {}:", kind.to_uppercase());
        for p in items {
            println!("  {}", progress::format_progress_line(p, id_width));
        }
    }

    // Aggregated totals
    let total_items: usize = all_progress.iter().map(|p| p.total).sum();
    let total_completed: usize = all_progress.iter().map(|p| p.completed).sum();
    let overall_ratio = if total_items > 0 {
        total_completed as f64 / total_items as f64
    } else {
        0.0
    };

    println!();
    println!(
        "  {}  {}/{}  ({}%)",
        progress::render_bar(overall_ratio, 24),
        total_completed,
        total_items,
        (overall_ratio * 100.0).round() as u32,
    );
    println!(
        "  {} artifact(s) with checkboxes",
        all_progress.len()
    );
    println!();

    Ok(())
}
