use std::collections::BTreeMap;

use forgeplan_core::progress::{self, ArtifactProgress, CheckboxCount};

use crate::commands::common;

pub async fn run(id: Option<&str>, json: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    let records = store.list_records(None).await?;

    if records.is_empty() {
        if json { println!("[]"); } else { println!("No artifacts found."); }
        return Ok(());
    }

    // Build progress for each artifact
    let all_progress: Vec<ArtifactProgress> = records
        .iter()
        .map(|r| {
            let count = progress::count_checkboxes(&r.body);
            ArtifactProgress {
                id: r.id.clone(),
                title: r.title.clone(),
                kind: r.kind.clone(),
                count,
            }
        })
        .collect();

    if json {
        let data: Vec<_> = all_progress.iter().filter(|p| p.count.total > 0).map(|p| {
            serde_json::json!({"id": p.id, "title": p.title, "kind": p.kind, "completed": p.count.completed, "total": p.count.total, "percent": p.percent()})
        }).collect();
        println!("{}", serde_json::to_string_pretty(&data)?);
        return Ok(());
    }

    // If specific ID requested, show only that artifact
    if let Some(target_id) = id {
        let upper = target_id.to_uppercase();

        // Find both progress and record in one pass
        let found = records.iter().enumerate().find(|(_, r)| r.id.to_uppercase() == upper);
        let (idx, record) = found
            .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", target_id))?;
        let p = &all_progress[idx];

        println!();
        println!("{} \"{}\"", p.id, p.title);
        println!("{}", "─".repeat(50));

        if p.count.total == 0 {
            println!("  No checkboxes found in this artifact.");
        } else {
            println!(
                "  {}  {}/{}  ({}%)  {}",
                progress::render_bar(p.ratio(), 24),
                p.count.completed,
                p.count.total,
                p.percent(),
                p.status_label()
            );

            // Show individual checkbox lines using shared parser
            println!();
            for line in record.body.lines() {
                if let Some((checked, text)) = progress::checkbox_text(line) {
                    let mark = if checked { "x" } else { " " };
                    println!("  [{}] {}", mark, text);
                }
            }
        }
        println!();
        return Ok(());
    }

    // Filter to artifacts with checkboxes
    let with_checkboxes: Vec<&ArtifactProgress> = all_progress
        .iter()
        .filter(|p| p.count.total > 0)
        .collect();

    if with_checkboxes.is_empty() {
        println!("No artifacts with checkboxes found.");
        return Ok(());
    }

    // Group by kind
    let mut by_kind: BTreeMap<String, Vec<&ArtifactProgress>> = BTreeMap::new();
    for p in &with_checkboxes {
        by_kind.entry(p.kind.clone()).or_default().push(p);
    }

    let id_width = with_checkboxes
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
    let total_items: usize = with_checkboxes.iter().map(|p| p.count.total).sum();
    let total_completed: usize = with_checkboxes.iter().map(|p| p.count.completed).sum();
    let overall = CheckboxCount {
        total: total_items,
        completed: total_completed,
    };
    let overall_ratio = if overall.total > 0 {
        overall.completed as f64 / overall.total as f64
    } else {
        0.0
    };

    println!();
    println!(
        "  {}  {}/{}  ({}%)",
        progress::render_bar(overall_ratio, 24),
        overall.completed,
        overall.total,
        (overall_ratio * 100.0).round() as u32,
    );
    println!(
        "  {} artifact(s) with checkboxes",
        with_checkboxes.len()
    );
    println!();

    Ok(())
}
