use std::collections::BTreeMap;

use forgeplan_core::artifact::frontmatter::parse_frontmatter;
use forgeplan_core::graph;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::hypothesis::{HypothesisState, current_state_from_frontmatter};

use crate::commands::common;

pub async fn run(json: bool, brownfield_only: bool) -> anyhow::Result<()> {
    let store = common::store().await?;

    // Build edges from LanceDB relations
    let relations = store.get_all_relations().await?;
    let edges: Vec<graph::Edge> = relations
        .into_iter()
        .map(|(from, to, relation)| graph::Edge { from, to, relation })
        .collect();

    // Also add parent_epic edges from artifacts
    let records = store.list_records(None).await?;
    let mut all_edges = edges;
    for record in &records {
        if let Some(parent) = &record.parent_epic
            && !parent.is_empty()
        {
            all_edges.push(graph::Edge {
                from: record.id.clone(),
                to: parent.clone(),
                relation: "belongs_to".to_string(),
            });
        }
    }

    all_edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));

    // Issue #287 Phase F (W4): collect brownfield render data — per-HYP
    // verification state and per-DM composition list. We always pre-load
    // these; the render call inspects `brownfield_only` to decide whether
    // to filter, but the data itself is read-only and small, and the
    // colour palette improves the default render too.
    let mut hypothesis_states: BTreeMap<String, HypothesisState> = BTreeMap::new();
    let mut dm_compositions: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for record in &records {
        match record.kind.as_str() {
            "hypothesis" => {
                // Parse the frontmatter to extract the canonical
                // `hypothesis_state:` field. Failures default to `parked`
                // via `current_state_from_frontmatter` — the renderer
                // then falls back further to `inferred` (amber) when the
                // id is absent from the map, which matches the contract.
                match parse_frontmatter(&record.body) {
                    Ok((fm, _)) => {
                        let state = current_state_from_frontmatter(&fm);
                        hypothesis_states.insert(record.id.clone(), state);
                    }
                    Err(err) => {
                        // Body without parseable frontmatter is a recovery
                        // case (corrupted artifact). Surface a warning on
                        // stderr and continue — the renderer falls back to
                        // the default state for the missing entry. We
                        // print to stderr so stdout stays a clean mermaid
                        // stream for `mmdc` pipelines.
                        eprintln!(
                            "warning: graph: failed to parse hypothesis frontmatter for {}: {} (using default state)",
                            record.id, err
                        );
                    }
                }
            }
            "domain_model" => {
                // `composition_for_dm` is total — empty section yields an
                // empty vec, which still emits the labelled cluster.
                let composed = graph::composition_for_dm(&record.body);
                dm_compositions.insert(record.id.clone(), composed);
            }
            _ => {}
        }
    }

    let opts = graph::BrownfieldRenderOpts {
        hypothesis_states,
        dm_compositions,
        brownfield_only,
    };

    // PRD-071 contract: graph output is a pipe-friendly artifact. Suggest the
    // canonical render command. Empty graph → suggest linking artifacts.
    let mut hints_vec: Vec<Hint> = Vec::new();
    if all_edges.is_empty() {
        hints_vec.push(
            Hint::suggestion("No relations yet — link two artifacts").with_action(
                "forgeplan link <source-id> <target-id> --relation refines".to_string(),
            ),
        );
    } else {
        hints_vec.push(
            Hint::info("Render graph to PNG with mmdc")
                .with_action("forgeplan graph | mmdc -o graph.png".to_string()),
        );
    }

    if json {
        let data: Vec<_> = all_edges
            .iter()
            .map(|e| serde_json::json!({"from": e.from, "to": e.to, "relation": e.relation}))
            .collect();
        let payload = serde_json::json!({
            "edges": data,
            "brownfield_only": brownfield_only,
            "_next_action": hints::primary_action(&hints_vec),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    let mermaid = graph::render_mermaid_with_opts(&all_edges, &opts);
    println!("{}", mermaid);
    // Mermaid output is consumed by `mmdc`; emit hint as a comment so that
    // pipe consumers don't choke on it. Plain `Next:` line is human-friendly
    // when the user runs the command interactively.
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}
