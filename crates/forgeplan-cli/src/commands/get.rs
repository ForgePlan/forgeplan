use forgeplan_core::hints::{self, Hint};

use crate::commands::common;
use crate::ui;

pub async fn run(id: &str, json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let workspace = forgeplan_core::workspace::find_workspace(&cwd).ok_or_else(|| {
        anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first.")
    })?;
    let (record, relations, incoming, view_metadata) = if common::index_migrated(&workspace) {
        let read = common::workspace_read(&workspace).await?;
        // The reference is resolved through the ONE shared resolver, over
        // the documents the capture holds. A name two documents answer to
        // names NEITHER of them: the command says which two, and exits
        // non-zero. It never picks one.
        let resolution = common::resolve_against_capture(&read, id);
        let canonical = match &resolution.outcome {
            common::Resolution::Found(found) => found.identity.canonical_key.clone(),
            other => {
                let failure = common::resolution_failure(other, id, &resolution.candidates);
                common::print_resolution_failure(&failure);
                return Err(common::resolution_failure_error(&failure));
            }
        };
        let record = read
            .get_record_by_key(&canonical)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Artifact '{id}' is not in the captured view\nFix: forgeplan index"
                )
            })?
            .clone();
        // The edges are held under the canonical key. The report names each
        // endpoint by the display id a reader knows it by, and carries the
        // key beside it — so a document that shares its display id with
        // another is still named exactly.
        let display_by_key: std::collections::HashMap<String, String> = read
            .records()
            .iter()
            .map(|record| (record.canonical_key.clone(), record.id.clone()))
            .collect();
        let name = |endpoint: &str| -> String {
            display_by_key
                .get(endpoint)
                .cloned()
                .unwrap_or_else(|| endpoint.to_string())
        };
        let relations = read
            .get_relations_by_key(&canonical)
            .into_iter()
            .map(|(target, relation)| (name(&target), target, relation))
            .collect::<Vec<_>>();
        let incoming = read
            .get_incoming_relations(&canonical)
            .into_iter()
            .map(|(source, relation)| (name(&source), source, relation))
            .collect::<Vec<_>>();
        let metadata = serde_json::json!({
            "index": "constellation",
            "generation_id": read.view().descriptor().map(|descriptor| &descriptor.generation_id),
            "snapshot_id": read.snapshot_id(),
            "source_set_digest": read.source_set_digest(),
            "persisted": read.is_persisted(),
        });
        (record, relations, incoming, Some(metadata))
    } else {
        // The legacy layout resolves against the index rows: the same shared
        // algorithm, over the same document set the MCP server reads. A name
        // two rows answer to names neither of them — the command says which
        // two, and exits non-zero.
        let store = common::store().await?;
        let resolution = common::resolve_against_legacy_index(&store, &workspace, id).await?;
        let canonical = match &resolution.outcome {
            common::Resolution::Found(found) => found.identity.canonical_key.clone(),
            other => {
                let failure = common::resolution_failure(other, id, &resolution.candidates);
                common::print_resolution_failure(&failure);
                return Err(common::resolution_failure_error(&failure));
            }
        };
        let record = store
            .get_record(&canonical)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' missing after resolution"))?;
        // The edges are held under the canonical key. The report names each
        // endpoint by the display id a reader knows it by, and carries the
        // key beside it.
        let mut relations: Vec<(String, String, String)> = Vec::new();
        for (target, relation) in store.get_relations(&record.id).await? {
            let display = store
                .resolve_display_id(&target)
                .await?
                .unwrap_or_else(|| target.clone());
            relations.push((display, target, relation));
        }
        let mut incoming: Vec<(String, String, String)> = Vec::new();
        for (source, relation) in store.get_incoming_relations(&record.id).await? {
            let display = store
                .resolve_display_id(&source)
                .await?
                .unwrap_or_else(|| source.clone());
            incoming.push((display, source, relation));
        }
        (record, relations, incoming, None)
    };
    let has_links = !relations.is_empty() || !incoming.is_empty();

    // #447 — both edge sets were already fetched above and then collapsed
    // into `has_links` purely to pick a hint. The data was one line from the
    // output and thrown away, so a linked artifact and an orphan rendered
    // identically. Emitted unconditionally: a caller must be able to tell
    // "this has no links" from "this tool does not report links", and an
    // absent field says the same thing as an empty one.
    let links_json = serde_json::json!({
        "outbound": relations
            .iter()
            .map(|(target, target_key, relation)| serde_json::json!({
                "target": target, "target_key": target_key, "relation": relation
            }))
            .collect::<Vec<_>>(),
        "inbound": incoming
            .iter()
            .map(|(source, source_key, relation)| serde_json::json!({
                "source": source, "source_key": source_key, "relation": relation
            }))
            .collect::<Vec<_>>(),
    });
    let kind: forgeplan_core::artifact::types::ArtifactKind = record
        .kind
        .parse()
        .unwrap_or(forgeplan_core::artifact::types::ArtifactKind::Note);
    let depth: forgeplan_core::artifact::types::Mode = record
        .depth
        .parse()
        .unwrap_or(forgeplan_core::artifact::types::Mode::Standard);

    // PROB-060 / SPEC-005 Phase 1.4 — slug lives in the body's YAML
    // frontmatter (the template-rendered block, not the DB-derived
    // synthetic one). Parse the body to extract it; non-fatal on failure.
    let parsed_fm = forgeplan_core::artifact::frontmatter::parse_frontmatter(&record.body).ok();
    let slug_for_json = parsed_fm.as_ref().and_then(|(fm, _)| {
        forgeplan_core::artifact::frontmatter::slug_from_frontmatter(fm).map(|s| s.to_string())
    });

    // PROB-060 / SPEC-005 / ADR-012 (W1.B, CD-5) — pick the canonical
    // reference form for hints: slug pre-merge, display id post-merge.
    // Falls back to `record.id` for legacy artifacts without slug.
    let ref_form: String = parsed_fm
        .as_ref()
        .map(|(fm, _)| forgeplan_core::artifact::frontmatter::refs_form(fm, &record.id).to_string())
        .unwrap_or_else(|| record.id.clone());

    let mut hints_vec: Vec<Hint> =
        hints::get_hints(&ref_form, &record.status, &kind, has_links, &depth);

    // Top-level Next: hint per status — full command using slug pre-merge
    // or display id post-merge so commit `Refs:` lines stay canonical.
    let primary = match record.status.as_str() {
        "draft" => Some(
            Hint::suggestion("Validate after filling MUST sections")
                .with_action(format!("forgeplan validate {}", ref_form)),
        ),
        "active" if record.r_eff_score < 0.5 => Some(
            Hint::warning("R_eff below 0.5 — score and add evidence")
                .with_action(format!("forgeplan score {}", ref_form)),
        ),
        _ => None,
    };
    if let Some(h) = primary {
        hints_vec.insert(0, h);
    }

    if json {
        let mut json_data = serde_json::json!({
            "id": record.id,
            // ADR-030 / SPEC-009: the response keeps the display id AND
            // carries the canonical key — the identity an address must use
            // when the display id is not unique.
            "canonical_key": record.canonical_key,
            "slug": slug_for_json,
            "kind": record.kind,
            "status": record.status,
            "title": record.title,
            "depth": record.depth,
            "author": record.author,
            "parent_epic": record.parent_epic,
            "valid_until": record.valid_until,
            "tags": record.tags,
            "r_eff": record.r_eff_score,
            "created_at": record.created_at,
            "updated_at": record.updated_at,
            "body": record.body,
            "links": links_json,
            "_next_action": hints::primary_action(&hints_vec),
        });
        if let Some(serde_json::Value::Object(metadata)) = view_metadata {
            json_data
                .as_object_mut()
                .expect("get payload is an object")
                .extend(metadata);
        }
        println!("{}", serde_json::to_string_pretty(&json_data)?);
        return Ok(());
    }

    // PROB-060 / SPEC-005 Phase 1.4 — show slug in CLI output when present.
    // Reuse the same body-parse result computed above for JSON.
    ui::header(&record.id, &record.title);
    ui::kv("Kind", &record.kind);
    if let Some(s) = &slug_for_json {
        ui::kv("Slug", s);
    }
    ui::kv("Status", &ui::styled_status(&record.status));
    ui::kv("Depth", &ui::styled_depth(&record.depth));
    if let Some(ref author) = record.author {
        ui::kv("Author", author);
    }
    if let Some(ref epic) = record.parent_epic
        && !epic.is_empty()
    {
        ui::kv("Parent Epic", epic);
    }
    if let Some(ref vu) = record.valid_until {
        ui::kv("Valid Until", vu);
    }
    ui::kv("R_eff", &ui::styled_reff(record.r_eff_score));
    ui::kv("Created", &record.created_at);
    ui::kv("Updated", &record.updated_at);
    // #447 — the same edges the JSON path reports. Printed even when there
    // are none, for the same reason.
    if relations.is_empty() && incoming.is_empty() {
        ui::kv("Links", "none");
    } else {
        if !relations.is_empty() {
            let out: Vec<String> = relations
                .iter()
                .map(|(t, _key, r)| format!("{t} ({r})"))
                .collect();
            ui::kv("Links out", &out.join(", "));
        }
        if !incoming.is_empty() {
            let inc: Vec<String> = incoming
                .iter()
                .map(|(s, _key, r)| format!("{s} ({r})"))
                .collect();
            ui::kv("Links in", &inc.join(", "));
        }
    }
    println!();
    println!("{}", record.body);

    if !hints_vec.is_empty() {
        print!("{}", hints::format_hints(&hints_vec));
    }
    print!("{}", hints::render_next_action_line(&hints_vec));

    Ok(())
}
