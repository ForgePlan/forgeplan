use forgeplan_core::search;
use forgeplan_core::workspace;

pub async fn run(query: &str, kind: Option<&str>) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let hits = search::search(&ws, query, kind).await?;

    if hits.is_empty() {
        println!("No results for \"{}\"", query);
        return Ok(());
    }

    println!("Found {} artifact(s) matching \"{}\":\n", hits.len(), query);

    for hit in &hits {
        println!(
            "  {} [{}] \"{}\"",
            hit.artifact.id, hit.artifact.kind, hit.artifact.title
        );
        for m in hit.matches.iter().take(3) {
            let line_info = if m.line_number == 0 {
                String::new()
            } else {
                format!("L{}: ", m.line_number)
            };
            let display = if m.line.len() > 100 {
                format!("{}...", &m.line[..100])
            } else {
                m.line.clone()
            };
            println!("    {}{}", line_info, display.trim());
        }
        if hit.matches.len() > 3 {
            println!("    ... and {} more match(es)", hit.matches.len() - 3);
        }
        println!();
    }

    Ok(())
}
