use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

pub async fn run(output: Option<&str>) -> anyhow::Result<()> {
    let store = common::store().await?;
    let cwd = std::env::current_dir()?;

    let records = store.list_records(None).await?;
    let artifacts: Vec<serde_json::Value> = records
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "kind": r.kind,
                "status": r.status,
                "title": r.title,
                "body": r.body,
                "depth": r.depth,
                "author": r.author,
                "parent_epic": r.parent_epic,
                "r_eff_score": r.r_eff_score,
                "valid_until": r.valid_until,
                "created_at": r.created_at,
                "updated_at": r.updated_at,
                "tags": r.tags,
            })
        })
        .collect();

    let all_relations = store.get_all_relations().await?;
    let relations: Vec<serde_json::Value> = all_relations
        .into_iter()
        .map(|(source, target, relation)| {
            serde_json::json!({
                "source": source,
                "target": target,
                "relation": relation,
            })
        })
        .collect();

    let artifact_count = artifacts.len();
    let relation_count = relations.len();

    let data = serde_json::json!({
        "version": 1,
        "artifacts": artifacts,
        "relations": relations,
    });

    let json = serde_json::to_string_pretty(&data)?;

    let path = output.unwrap_or(".forgeplan/export.json");
    let full_path = if std::path::Path::new(path).is_absolute() {
        std::path::PathBuf::from(path)
    } else {
        cwd.join(path)
    };

    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full_path, &json)?;

    println!(
        "Exported {} artifacts, {} relations to {}",
        artifact_count,
        relation_count,
        full_path.display()
    );

    // [w4 HIGH-2 / CWE-78] sanitize the path before interpolating it into
    // the agent-visible hint. `--output <PATH>` is full attacker-controlled
    // input; without filtering, a payload like `'/tmp/foo;rm -rf .'` lands
    // in `Next: forgeplan import /tmp/foo;rm -rf .` and executes the
    // trailing command on copy-paste. Sibling fix of HIGH-1 (tag.rs).
    let path_str = full_path.display().to_string();
    let safe_path = forgeplan_core::artifact::sanitize::sanitize_path_for_hint(&path_str);
    let hint_list = vec![
        Hint::info("Re-import to verify the snapshot round-trips")
            .with_action(format!("forgeplan import {}", safe_path)),
    ];
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}
