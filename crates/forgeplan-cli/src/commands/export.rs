use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

pub async fn run(output: Option<&str>, json_flag: bool) -> anyhow::Result<()> {
    let (_ws, source, _read_guard) = common::open_read_source().await?;
    let cwd = std::env::current_dir()?;

    let legacy_records;
    let records = match &source {
        common::ReadSource::Legacy(store) => {
            legacy_records = store.list_records(&_read_guard, None).await?;
            legacy_records.as_slice()
        }
        common::ReadSource::Captured(read) => read.records(),
    };
    let artifacts: Vec<serde_json::Value> = records
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "canonical_key": r.canonical_key,
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

    let all_relations = match &source {
        common::ReadSource::Legacy(store) => store.get_all_relations(&_read_guard).await?,
        common::ReadSource::Captured(read) => read.get_all_relations(),
    };
    // The stored endpoints are canonical keys. The bundle keeps the ids it
    // always carried — the display ids a consumer reads — and carries the
    // key beside them, so a document that shares its display id with another
    // is still named exactly.
    let display_by_key: std::collections::HashMap<&str, &str> = records
        .iter()
        .map(|r| (r.canonical_key.as_str(), r.id.as_str()))
        .collect();
    let key_by_display: std::collections::HashMap<&str, &str> = records
        .iter()
        .map(|r| (r.id.as_str(), r.canonical_key.as_str()))
        .collect();
    let describe = |endpoint: &str| -> (String, String) {
        if let Some(display) = display_by_key.get(endpoint) {
            return ((*display).to_string(), endpoint.to_string());
        }
        if let Some(key) = key_by_display.get(endpoint) {
            return (endpoint.to_string(), (*key).to_string());
        }
        (endpoint.to_string(), endpoint.to_string())
    };

    let relations: Vec<serde_json::Value> = all_relations
        .into_iter()
        .map(|(source, target, relation)| {
            let (source_id, source_key) = describe(&source);
            let (target_id, target_key) = describe(&target);
            serde_json::json!({
                "source": source_id,
                "target": target_id,
                "relation": relation,
                "source_key": source_key,
                "target_key": target_key,
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

    if json_flag {
        // Mirrors `forgeplan_export`: no --output → the bundle itself plus
        // `_next_action`; with --output → the write-confirmation object.
        // --json never silently discards --output: both are honoured.
        if let Some(output_path) = &output {
            let full_path = if std::path::Path::new(*output_path).is_absolute() {
                std::path::PathBuf::from(*output_path)
            } else {
                cwd.join(*output_path)
            };
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&full_path, serde_json::to_string_pretty(&data)?)?;
            let safe_path = forgeplan_core::artifact::sanitize::sanitize_path_for_hint(
                &full_path.display().to_string(),
            );
            let confirmation = serde_json::json!({
                "artifacts": artifact_count,
                "relations": relation_count,
                "written_to": full_path.display().to_string(),
                "_next_action": format!(
                    "Exported to {safe_path}. Restore via `forgeplan import {safe_path}`."
                ),
            });
            println!("{}", serde_json::to_string_pretty(&confirmation)?);
        } else {
            let mut payload = data;
            payload
                .as_object_mut()
                .expect("export bundle is an object")
                .insert(
                    "_next_action".to_string(),
                    serde_json::Value::String(format!(
                        "Exported {artifact_count} artifacts + {relation_count} relations. \
                     Save to disk: `forgeplan export --output backup.json`."
                    )),
                );
            println!("{}", serde_json::to_string_pretty(&payload)?);
        }
        return Ok(());
    }

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
