use std::borrow::Cow;

use forgeplan_core::db::store::ArtifactRecord;
use forgeplan_core::depth;
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

pub async fn run(id: Option<&str>) -> anyhow::Result<()> {
    let (_, source) = common::open_read_source().await?;
    let records: Cow<'_, [ArtifactRecord]> = match &source {
        common::ReadSource::Legacy(store) => Cow::Owned(store.list_records(None).await?),
        common::ReadSource::Captured(read) => Cow::Borrowed(read.records()),
    };

    if records.is_empty() {
        println!("No artifacts found.");
        return Ok(());
    }

    let to_check: Vec<_> = if let Some(target_id) = id {
        let upper = target_id.to_uppercase();
        records
            .iter()
            .filter(|r| r.id.to_uppercase() == upper)
            .collect()
    } else {
        records.iter().collect()
    };

    if to_check.is_empty()
        && let Some(target_id) = id
    {
        anyhow::bail!(
            "Artifact '{}' not found
Fix: forgeplan list",
            target_id
        );
    }

    let mut escalations = 0;
    let mut first_escalation_id: Option<String> = None;

    for record in &to_check {
        let link_count = match &source {
            common::ReadSource::Legacy(store) => store
                .get_relations(&record.id)
                .await
                .unwrap_or_default()
                .len(),
            common::ReadSource::Captured(read) => read.get_relations(&record.id).len(),
        };
        let result = depth::suggest_depth(record, link_count);

        if id.is_some() || result.escalation_needed {
            println!();
            println!("{} \"{}\"", result.artifact_id, result.artifact_title);
            println!("{}", "─".repeat(50));
            println!("  Current:   {:?}", result.current_depth);
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
                    println!("    {:?} → {} ({})", s.minimum_depth, s.name, s.value);
                }
            }

            if result.escalation_needed {
                escalations += 1;
                if first_escalation_id.is_none() {
                    first_escalation_id = Some(result.artifact_id.clone());
                }
            }
        }
    }

    if id.is_none() {
        println!();
        if escalations > 0 {
            println!("{} artifact(s) need depth escalation.", escalations);
        } else {
            println!("All artifacts are at appropriate depth levels.");
        }
    }

    let mut hint_list: Vec<Hint> = Vec::new();
    if let Some(target) = first_escalation_id {
        hint_list.push(
            Hint::warning(format!("Escalate depth for {}", target))
                .with_action(format!("forgeplan get {}", target)),
        );
    } else if let Some(target_id) = id {
        hint_list.push(
            Hint::info("Depth looks fine — verify R_eff next")
                .with_action(format!("forgeplan score {}", target_id)),
        );
    }

    println!();
    print!("{}", hints::render_next_action_line(&hint_list));
    Ok(())
}
