use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::graph;
use forgeplan_core::workspace;

pub async fn run() -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;

    // Build edges from LanceDB relations
    let relations = store.get_all_relations().await?;
    let edges: Vec<graph::Edge> = relations
        .into_iter()
        .map(|(from, to, relation)| graph::Edge {
            from,
            to,
            relation,
        })
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
    let mermaid = graph::render_mermaid(&all_edges);
    println!("{}", mermaid);
    Ok(())
}
