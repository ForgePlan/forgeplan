use std::env;
use std::path::PathBuf;

use forgeplan_core::db::store::{FpfChunk, LanceStore};
use forgeplan_core::fpf;
use forgeplan_core::fpf::knowledge;
use forgeplan_core::workspace;

/// FPF Dashboard (original command, now `forgeplan fpf dashboard`)
pub async fn run_dashboard() -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let dashboard = fpf::dashboard(&store).await?;
    print!("{dashboard}");

    Ok(())
}

/// `forgeplan fpf ingest [--path <dir>]`
pub async fn run_ingest(path: Option<&str>) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let fpf_path = match path {
        Some(p) => PathBuf::from(p),
        None => knowledge::default_fpf_path()
            .ok_or_else(|| anyhow::anyhow!("FPF spec not found. Use --path to specify location"))?,
    };

    println!("  Ingesting FPF spec from {}...", fpf_path.display());

    let chunks = knowledge::ingest_fpf_directory(&fpf_path).await?;
    println!("  Parsed {} sections", chunks.len());

    // Use init() to ensure fpf_spec table exists
    let store = LanceStore::init(&ws).await?;

    // Clear existing FPF data and re-ingest
    if store.has_fpf() {
        store.clear_fpf().await?;
    }

    // Convert IngestChunk to FpfChunk
    let now = chrono::Utc::now().to_rfc3339();
    let fpf_chunks: Vec<FpfChunk> = chunks
        .iter()
        .map(|c| FpfChunk {
            id: c.id.clone(),
            section_id: c.section_id.clone(),
            parent_section: c.parent_section.clone(),
            title: c.title.clone(),
            body: c.body.clone(),
            line_count: c.line_count,
            file_path: c.file_path.clone(),
            created_at: now.clone(),
        })
        .collect();

    let count = store.insert_fpf_chunks(&fpf_chunks).await?;
    println!("  Ingested {} FPF sections into LanceDB", count);
    Ok(())
}

/// `forgeplan fpf search <query> [--limit N]`
pub async fn run_search(query: &str, limit: usize) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let results = store.search_fpf(query, limit).await?;

    if results.is_empty() {
        println!("  No FPF sections match '{}'", query);
        println!("  Hint: Run `forgeplan fpf ingest` first");
        return Ok(());
    }

    println!();
    for (i, chunk) in results.iter().enumerate() {
        let snippet: String = chunk
            .body
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(200)
            .collect();
        println!("  {}. [{}] {}", i + 1, chunk.section_id, chunk.title);
        println!("     {} ({} lines)", snippet, chunk.line_count);
        println!();
    }
    Ok(())
}

/// `forgeplan fpf section <id> [--summary]`
pub async fn run_section(id: &str, summary: bool) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let chunk = store
        .get_fpf_section(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("FPF section '{}' not found", id))?;

    println!();
    println!("## {} — {}", chunk.section_id, chunk.title);
    println!();
    if summary {
        let preview: String = chunk.body.chars().take(500).collect();
        println!("{}", preview);
        if chunk.body.len() > 500 {
            println!(
                "\n  ... ({} more chars. Use without --summary for full text)",
                chunk.body.len() - 500
            );
        }
    } else {
        println!("{}", chunk.body);
    }
    Ok(())
}

/// `forgeplan fpf status`
pub async fn run_status() -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    println!();
    println!("FPF Knowledge Base Status");
    println!("{}", "=".repeat(40));

    // Check source
    let source_path = knowledge::default_fpf_path();
    let source_count = match &source_path {
        Some(p) => {
            println!("  Source:    {} (exists)", p.display());
            count_md_files(p).await
        }
        None => {
            println!("  Source:    not found (set fpf.path in config or install fpf-simple skill)");
            0
        }
    };
    if source_count > 0 {
        println!("  Files:     {} markdown files", source_count);
    }

    // Check ingested
    let store = LanceStore::open(&ws).await?;
    if store.has_fpf() {
        let sections = store.list_fpf_sections().await?;
        if sections.is_empty() {
            println!("  Ingested:  empty (run `forgeplan fpf ingest`)");
        } else {
            let total_lines: i32 = sections.iter().map(|s| s.line_count).sum();
            println!(
                "  Ingested:  {} sections, {} total lines",
                sections.len(),
                total_lines
            );

            // Staleness check
            if source_count > 0 && source_count != sections.len() {
                println!(
                    "  Status:    STALE — source has {} files, ingested has {} sections",
                    source_count,
                    sections.len()
                );
                println!("  Action:    Run `forgeplan fpf ingest` to re-sync");
            } else if source_count > 0 {
                println!("  Status:    UP TO DATE");
            }
        }
    } else {
        println!("  Ingested:  not initialized");
        println!("  Action:    Run `forgeplan fpf ingest` to load FPF spec");
    }

    println!();
    Ok(())
}

async fn count_md_files(dir: &std::path::Path) -> usize {
    let mut count = 0;
    if let Ok(mut rd) = tokio::fs::read_dir(dir).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let p = entry.path();
            if p.is_dir()
                && let Ok(mut sub) = tokio::fs::read_dir(&p).await
            {
                while let Ok(Some(sub_entry)) = sub.next_entry().await {
                    let sp = sub_entry.path();
                    if sp.extension().is_some_and(|e| e == "md")
                        && sp.file_name().is_some_and(|n| n != "_index.md")
                    {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

/// `forgeplan fpf list`
pub async fn run_list() -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let sections = store.list_fpf_sections().await?;

    if sections.is_empty() {
        println!("  No FPF sections loaded. Run `forgeplan fpf ingest` first.");
        return Ok(());
    }

    println!();
    println!("  {:10}  {:5}  Title", "Section", "Lines");
    println!("  {}", "-".repeat(60));
    for s in &sections {
        println!("  {:10}  {:5}  {}", s.section_id, s.line_count, s.title);
    }
    println!();
    println!("  {} sections total", sections.len());
    Ok(())
}
