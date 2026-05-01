use forgeplan_core::hints::{self, Hint};
use forgeplan_core::link;

use crate::commands::common;

pub async fn run(source_id: &str, target_id: &str, relation: &str) -> anyhow::Result<()> {
    // Normalize relation
    let relation = link::normalize_relation(relation)?;

    let (ws, store) = common::open_store().await?;

    // Verify source exists
    let source = store.get_artifact(source_id).await?;
    if source.is_none() {
        anyhow::bail!(
            "Source artifact '{}' not found
Fix: forgeplan list",
            source_id
        );
    }

    // Verify target exists (warning only)
    let target = store.get_artifact(target_id).await?;
    if target.is_none() {
        eprintln!(
            "Warning: Target artifact '{}' not found in workspace (creating link anyway)",
            target_id
        );
    }

    // PRD-073 FR-005: helper handles sync→add_relation→render for BOTH sides
    // so target file's frontmatter stays in lockstep with LanceDB.
    forgeplan_core::projection::add_link_with_projection(
        &ws, &store, source_id, target_id, &relation,
    )
    .await?;

    common::log_change_field(
        &store,
        source_id,
        "link",
        "relation",
        None,
        Some(&format!("{}:{}", target_id, relation)),
        "cli",
    )
    .await;

    println!("Linked: {} --{}--> {}", source_id, relation, target_id);

    // PRD-071 contract: after linking evidence/refines/based_on, the natural
    // next step is to re-score the source so R_eff updates.
    let hints_vec = vec![
        Hint::info("Recompute R_eff after linking")
            .with_action(format!("forgeplan score {}", source_id)),
    ];
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}

pub async fn run_unlink(source_id: &str, target_id: &str, relation: &str) -> anyhow::Result<()> {
    let relation = link::normalize_relation(relation)?;
    let (ws, store) = common::open_store().await?;

    // Check relation exists before deleting.
    // Use get_all_relations for resilient lookup (works even if source artifact is deleted).
    let all_rels = store.get_all_relations().await?;
    let found = all_rels.iter().any(|(s, t, r)| {
        s.eq_ignore_ascii_case(source_id) && t.eq_ignore_ascii_case(target_id) && r == &relation
    });
    if !found {
        anyhow::bail!(
            "Relation '{}' from {} to {} not found",
            relation,
            source_id,
            target_id
        );
    }

    // PRD-073 FR-005: bidirectional render via helper.
    forgeplan_core::projection::delete_link_with_projection(
        &ws, &store, source_id, target_id, &relation,
    )
    .await?;

    common::log_change_field(
        &store,
        source_id,
        "unlink",
        "relation",
        Some(&format!("{}:{}", target_id, relation)),
        None,
        "cli",
    )
    .await;

    println!("Unlinked: {} --{}--> {}", source_id, relation, target_id);

    // PRD-071 contract: rescoring is the right follow-up after unlinking too.
    let hints_vec = vec![
        Hint::info("Recompute R_eff after unlink")
            .with_action(format!("forgeplan score {}", source_id)),
    ];
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}
