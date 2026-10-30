use std::collections::BTreeMap;

use forgeplan_core::artifact::canonical_key::ArtifactCandidate;
use forgeplan_core::artifact::frontmatter::parse_frontmatter;
use forgeplan_core::graph;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::hypothesis::{HypothesisState, current_state_from_frontmatter};

use crate::commands::common;

pub async fn run(json: bool, brownfield_only: bool) -> anyhow::Result<()> {
    let (_, source) = common::open_read_source().await?;
    let relations = match &source {
        common::ReadSource::Legacy(store) => store.get_all_relations().await?,
        common::ReadSource::Captured(read) => read.get_all_relations(),
    };
    let legacy_records;
    let records = match &source {
        common::ReadSource::Legacy(store) => {
            legacy_records = store.list_records(None).await?;
            legacy_records.as_slice()
        }
        common::ReadSource::Captured(read) => read.records(),
    };
    let edges: Vec<graph::Edge> = relations
        .into_iter()
        .map(|(from, to, relation)| graph::Edge { from, to, relation })
        .collect();

    // Parent-epic edges use the same records as the explicit relations.
    let mut all_edges = edges;
    for record in records {
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

    // The document set the graph is read against: every document, named by
    // its canonical key, its display id and its title. The node id of the
    // graph is the canonical key, so two documents that share a display id
    // stay two vertices.
    //
    // It is the SAME set the MCP graph reads — one builder in core — so the
    // two surfaces render the same vertices and the same labels.
    let documents: Vec<ArtifactCandidate> =
        forgeplan_core::artifact::documents::candidates_from_records(records);

    // Every endpoint is addressed by the canonical key of the one document
    // it names. A forward reference — or an alias two documents answer to —
    // keeps its declared text.
    let mut all_edges = graph::canonical_edges(&all_edges, &documents);

    all_edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));

    // Issue #287 Phase F (W4): collect brownfield render data — per-HYP
    // verification state and per-DM composition list. We always pre-load
    // these; the render call inspects `brownfield_only` to decide whether
    // to filter, but the data itself is read-only and small, and the
    // colour palette improves the default render too.
    let mut hypothesis_states: BTreeMap<String, HypothesisState> = BTreeMap::new();
    let mut dm_compositions: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for record in records {
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
                        // Keyed by the canonical key: the key IS the node id
                        // of the graph, and two documents may share a
                        // display id.
                        hypothesis_states.insert(record.canonical_key.clone(), state);
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
                // Keyed by the canonical key, like every other node id.
                let composed = graph::composition_for_dm(&record.body);
                dm_compositions.insert(record.canonical_key.clone(), composed);
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

    // The identity graph: every node id is a canonical key and every node
    // is labelled with its display id and its title.
    let mermaid = graph::render_identity_graph(&all_edges, &documents, &opts);
    println!("{}", mermaid);
    // Mermaid output is consumed by `mmdc`; emit hint as a comment so that
    // pipe consumers don't choke on it. Plain `Next:` line is human-friendly
    // when the user runs the command interactively.
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}
