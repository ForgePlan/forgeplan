/// Generate embeddings for all artifacts (title + body) for semantic search.
#[cfg(feature = "semantic-search")]
pub async fn run() -> anyhow::Result<()> {
    use crate::commands::common;
    use crate::ui;
    use forgeplan_core::embed::Embedder;

    let store = common::store().await?;

    ui::info("Loading embedding model...");
    let mut embedder = Embedder::new()?;

    let records = store.list_records(None).await?;
    if records.is_empty() {
        ui::info("No artifacts to embed.");
        return Ok(());
    }

    println!("Embedding {} artifact(s) (title + body)...\n", records.len());

    let mut ok = 0usize;
    let mut err = 0usize;

    for record in &records {
        let text = record.embedding_text();
        match embedder.embed(&text) {
            Ok(vec) => {
                store.update_embedding(&record.id, &vec).await?;
                println!("  {} [{}] \"{}\"", record.id, record.kind, record.title);
                ok += 1;
            }
            Err(e) => {
                eprintln!("  FAIL {} — {}", record.id, e);
                err += 1;
            }
        }
    }

    println!("\nDone: {} embedded, {} failed.", ok, err);
    Ok(())
}

#[cfg(not(feature = "semantic-search"))]
pub async fn run() -> anyhow::Result<()> {
    anyhow::bail!(
        "Embedding not available. Rebuild with: \
         cargo build --features semantic-search"
    );
}
