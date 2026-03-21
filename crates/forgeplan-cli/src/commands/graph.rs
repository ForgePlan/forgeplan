use forgeplan_core::graph;
use forgeplan_core::workspace;

pub async fn run() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let edges = graph::build_edges(&ws).await?;
    let mermaid = graph::render_mermaid(&edges);

    println!("{}", mermaid);
    Ok(())
}
