/// Generate embeddings for all artifacts (title + body) for semantic search.
#[cfg(feature = "semantic-search")]
pub async fn run() -> anyhow::Result<()> {
    use crate::commands::common;
    use crate::ui;
    use forgeplan_core::artifact::sanitize::sanitize_for_hint;
    use forgeplan_core::embed::Embedder;
    use forgeplan_core::hints::{self, Hint};

    let store = common::store().await?;
    let config = common::config().unwrap_or_default();
    let chunk_size = config
        .embedding
        .as_ref()
        .map(|e| e.chunk_size)
        .unwrap_or(2000);

    ui::info("Loading embedding model...");
    let mut embedder = Embedder::new()?;

    let records = store.list_records(None).await?;
    if records.is_empty() {
        ui::info("No artifacts to embed.");
        let hint_list = vec![
            Hint::info("Create your first artifact")
                .with_action("forgeplan new prd \"<title>\"".to_string()),
        ];
        print!("{}", hints::render_next_action_line(&hint_list));
        return Ok(());
    }

    println!(
        "Embedding {} artifact(s) (title + body, chunk_size={})...\n",
        records.len(),
        chunk_size
    );

    let mut ok = 0usize;
    let mut err = 0usize;

    for record in &records {
        let text = record.embedding_text(chunk_size);
        match embedder.embed(&text) {
            Ok(vec) => {
                store.update_embedding(&record.id, &vec).await?;
                // SEC-H1 (CWE-117 / CWE-150): titles are attacker-
                // controllable via frontmatter; sanitize before TTY
                // emission to neutralise ANSI/bidi/control bytes.
                println!(
                    "  {} [{}] \"{}\"",
                    record.id,
                    record.kind,
                    sanitize_for_hint(&record.title)
                );
                ok += 1;
            }
            Err(e) => {
                eprintln!("  FAIL {} — {}", record.id, e);
                err += 1;
            }
        }
    }

    println!("\nDone: {} embedded, {} failed.", ok, err);
    let hint_list = if err > 0 {
        vec![
            Hint::warning(format!("{} artifact(s) failed to embed", err))
                .with_action("forgeplan health".to_string()),
        ]
    } else {
        vec![
            Hint::info("Run a semantic search")
                .with_action("forgeplan search \"<query>\"".to_string()),
        ]
    };
    print!("{}", hints::render_next_action_line(&hint_list));
    Ok(())
}

#[cfg(not(feature = "semantic-search"))]
pub async fn run() -> anyhow::Result<()> {
    anyhow::bail!(
        "Embedding not available. Rebuild with: cargo build --features semantic-search\n\
         Fix: cargo build --features semantic-search"
    );
}
