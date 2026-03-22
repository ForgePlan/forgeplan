use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::depth;
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

    let to_check: Vec<_> = if let Some(target_id) = id {
        let upper = target_id.to_uppercase();
        records
            .into_iter()
            .filter(|r| r.id.to_uppercase() == upper)
            .collect()
    } else {
        records
    };

    if to_check.is_empty() {
        if let Some(target_id) = id {
            anyhow::bail!("Artifact '{}' not found", target_id);
        }
    }

    let mut escalations = 0;

    for record in &to_check {
        let link_count = store.get_relations(&record.id).await.unwrap_or_default().len();
        let result = depth::suggest_depth(record, link_count);

        if id.is_some() || result.escalation_needed {
            println!();
            println!(
                "{} \"{}\"",
                result.artifact_id, result.artifact_title
            );
            println!("{}", "─".repeat(50));
            println!(
                "  Current:   {:?}",
                result.current_depth
            );
            println!(
                "  Suggested: {:?}{}",
                result.suggested_depth,
                if result.escalation_needed {
                    " ⬆ ESCALATION"
                } else {
                    ""
                }
            );

            if !result.signals.is_empty() {
                println!("  Signals:");
                for s in &result.signals {
                    println!(
                        "    {:?} → {} ({})",
                        s.minimum_depth, s.name, s.value
                    );
                }
            }

            if result.escalation_needed {
                escalations += 1;
            }
        }
    }

    if id.is_none() {
        println!();
        if escalations > 0 {
            println!(
                "{} artifact(s) need depth escalation.",
                escalations
            );
        } else {
            println!("All artifacts are at appropriate depth levels.");
        }
    }

    println!();
    Ok(())
}
