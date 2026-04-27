use forgeplan_core::graph;
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

pub async fn run(json: bool) -> anyhow::Result<()> {
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
            "_next_action": hints::primary_action(&hints_vec),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    let mermaid = graph::render_mermaid(&all_edges);
    println!("{}", mermaid);
    // Mermaid output is consumed by `mmdc`; emit hint as a comment so that
    // pipe consumers don't choke on it. Plain `Next:` line is human-friendly
    // when the user runs the command interactively.
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}
