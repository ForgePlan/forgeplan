use std::collections::BTreeMap;

use forgeplan_core::hints::{self, Hint};
use forgeplan_core::progress::{self, ArtifactProgress, CheckboxCount};

use crate::commands::common;

pub async fn run(id: Option<&str>, json: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    let records = store.list_records(None).await?;

    if records.is_empty() {
        let hints_vec = vec![
            Hint::suggestion("Workspace is empty — create your first PRD")
                .with_action("forgeplan new prd \"<title>\"".to_string()),
        ];
        if json {
            let payload = serde_json::json!({
                "progress": [],
                "_next_action": hints::primary_action(&hints_vec),
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!("No artifacts found.");
            print!("{}", hints::render_next_action_line(&hints_vec));
        }
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

    // PRD-071 contract: pick the artifact with the lowest percent (most work
    // remaining) and suggest viewing it. Skip artifacts with no checkboxes
    // and those that are 100% done.
    let mut hints_vec: Vec<Hint> = Vec::new();
    let candidate = all_progress
        .iter()
        .filter(|p| p.count.total > 0 && p.count.completed < p.count.total)
        .min_by_key(|p| p.percent());
    if let Some(p) = candidate {
        hints_vec.push(
            Hint::info(format!(
                "{} is {}% complete — focus there",
                p.id,
                p.percent()
            ))
            .with_action(format!("forgeplan get {}", p.id)),
        );
    }

    if json {
        let data: Vec<_> = all_progress.iter().filter(|p| p.count.total > 0).map(|p| {
            serde_json::json!({"id": p.id, "title": p.title, "kind": p.kind, "completed": p.count.completed, "total": p.count.total, "percent": p.percent()})
        }).collect();
        let payload = serde_json::json!({
            "progress": data,
            "_next_action": hints::primary_action(&hints_vec),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    // If specific ID requested, show only that artifact.
    // Phase 2.5 (PROB-060) — accept slug or display id form via resolver.
    if let Some(target_id) = id {
        let canonical = store.resolve_id(target_id).await?.ok_or_else(|| {
            anyhow::anyhow!("Artifact '{}' not found\nFix: forgeplan list", target_id)
        })?;
        let canonical_upper = canonical.to_uppercase();

        // Find both progress and record in one pass
        let found = records
            .iter()
            .enumerate()
            .find(|(_, r)| r.id.to_uppercase() == canonical_upper);
        let (idx, record) = found.ok_or_else(|| {
            anyhow::anyhow!(
                "Artifact '{}' not found
Fix: forgeplan list",
                target_id
            )
        })?;
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
        // PRD-071 contract: focused view — suggest activating when 100%, else
        // jump to the artifact body.
        let single_hints = if p.count.total > 0 && p.count.completed == p.count.total {
            vec![
                Hint::suggestion(format!("All checkboxes complete — activate {}", p.id))
                    .with_action(format!("forgeplan activate {}", p.id)),
            ]
        } else {
            vec![
                Hint::info(format!("Open {} to mark next checkbox", p.id))
                    .with_action(format!("forgeplan get {}", p.id)),
            ]
        };
        print!("{}", hints::render_next_action_line(&single_hints));
        return Ok(());
    }

    // Filter to artifacts with checkboxes
    let with_checkboxes: Vec<&ArtifactProgress> =
        all_progress.iter().filter(|p| p.count.total > 0).collect();

    if with_checkboxes.is_empty() {
        println!("No artifacts with checkboxes found.");
        let bootstrap = vec![
            Hint::suggestion("Add FR checkboxes to a PRD or RFC")
                .with_action("forgeplan list --kind prd".to_string()),
        ];
        print!("{}", hints::render_next_action_line(&bootstrap));
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
    println!("  {} artifact(s) with checkboxes", with_checkboxes.len());
    println!();

    print!("{}", hints::render_next_action_line(&hints_vec));

    Ok(())
}
