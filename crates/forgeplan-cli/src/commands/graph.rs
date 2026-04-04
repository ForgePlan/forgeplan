use forgeplan_core::graph;

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
        if let Some(parent) = &record.parent_epic {
            if !parent.is_empty() {
                all_edges.push(graph::Edge {
                    from: record.id.clone(),
                    to: parent.clone(),
                    relation: "belongs_to".to_string(),
                });
            }
        }
    }

    all_edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));

    if json {
        let data: Vec<_> = all_edges
            .iter()
            .map(|e| serde_json::json!({"from": e.from, "to": e.to, "relation": e.relation}))
            .collect();
        println!("{}", serde_json::to_string_pretty(&data)?);
        return Ok(());
    }

    let mermaid = graph::render_mermaid(&all_edges);
    println!("{}", mermaid);
    Ok(())
}
