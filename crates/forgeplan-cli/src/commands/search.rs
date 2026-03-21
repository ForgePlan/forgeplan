use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::workspace;

pub async fn run(query: &str, kind: Option<&str>) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let hits = store.search_body(query, kind).await?;

    if hits.is_empty() {
        println!("No results for \"{}\"", query);
        return Ok(());
    }

    println!("Found {} artifact(s) matching \"{}\":\n", hits.len(), query);

    for record in &hits {
        println!("  {} [{}] \"{}\"", record.id, record.kind, record.title);

        // Show matching lines from body
        let query_lower = query.to_lowercase();
        let mut match_count = 0;
        for (i, line) in record.body.lines().enumerate() {
            if line.to_lowercase().contains(&query_lower) {
                let display = if line.chars().count() > 100 {
                    format!("{}...", line.chars().take(100).collect::<String>())
                } else {
                    line.to_string()
                };
                println!("    L{}: {}", i + 1, display.trim());
                match_count += 1;
                if match_count >= 3 {
                    // Count remaining matches
                    let remaining: usize = record
                        .body
                        .lines()
                        .skip(i + 1)
                        .filter(|l| l.to_lowercase().contains(&query_lower))
                        .count();
                    if remaining > 0 {
                        println!("    ... and {} more match(es)", remaining);
                    }
                    break;
                }
            }
        }
        println!();
    }

    Ok(())
}
