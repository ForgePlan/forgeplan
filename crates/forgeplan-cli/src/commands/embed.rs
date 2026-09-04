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

    // PROB-093: only encode what actually moved. A record is current when it
    // already carries a vector AND its content hash still matches. Before
    // that fix, `embed` recomputed all 400+ records to index one new artifact
    // — 13m18s on this workspace, which is why the step people were supposed
    // to run manually did not get run.
    //
    // Deciding this BEFORE the model loads is the other half, and it was
    // missing: the incremental skip removed the encoding work but left the
    // load unconditional, so a fully-current workspace still paid 8.22s to
    // report "0 embedded, 427 already current". The model was read off disk
    // and never used. Work first, model only if there is work.
    let mut work: Vec<(&forgeplan_core::db::store::ArtifactRecord, String)> = Vec::new();
    let mut skipped = 0usize;
    for record in &records {
        let current_hash =
            forgeplan_core::db::store::compute_content_hash(&record.title, &record.body);
        if record.embedding.is_some() && record.body_hash.as_deref() == Some(current_hash.as_str())
        {
            skipped += 1;
            continue;
        }
        work.push((record, current_hash));
    }

    if work.is_empty() {
        println!("Done: 0 embedded, {skipped} already current, 0 failed.");
        let hint_list = vec![
            Hint::info("Run a semantic search")
                .with_action("forgeplan search \"<query>\"".to_string()),
        ];
        print!("{}", hints::render_next_action_line(&hint_list));
        return Ok(());
    }

    // Tell the user about a multi-gigabyte download BEFORE it starts, not
    // after they notice the process sitting there. Silent when the model is
    // already cached — and now silent as well when nothing needs encoding,
    // which is the common case.
    if let Some(notice) = forgeplan_core::embed::first_run_notice() {
        ui::info(&notice);
    }

    ui::info("Loading embedding model...");
    let mut embedder = Embedder::new()?;

    println!(
        "Embedding {} of {} artifact(s) (title + body, chunk_size={})...\n",
        work.len(),
        records.len(),
        chunk_size
    );

    let mut ok = 0usize;
    let mut err = 0usize;

    for (record, current_hash) in &work {
        let text = record.embedding_text(chunk_size);
        match embedder.embed(&text) {
            Ok(vec) => {
                store.update_embedding(&record.id, &vec).await?;
                store
                    .update_body_hash(&record.id, current_hash)
                    .await
                    // The vector is written; a failed hash stamp only costs a
                    // redundant re-encode next run, so it must not fail the
                    // command and lose the work already done.
                    .unwrap_or_else(|e| {
                        eprintln!("  warn {} — hash not stamped: {}", record.id, e)
                    });
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

    println!(
        "\nDone: {} embedded, {} already current, {} failed.",
        ok, skipped, err
    );
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

/// Refusal path for builds without the `semantic-search` feature.
///
/// The remediation MUST be runnable by the audience that actually sees this:
/// someone who installed a prebuilt binary (brew / install.sh / GitHub
/// Releases) and has no checkout on disk. `cargo build` was the previous
/// advice and it is inert for them — it needs a source tree they do not have.
/// `cargo install --git` fetches the source itself, so it works from an empty
/// directory. PRD-071 requires `Fix:` to be runnable as-is; PROB-088 M2
/// recorded the violation.
#[cfg(not(feature = "semantic-search"))]
pub async fn run() -> anyhow::Result<()> {
    anyhow::bail!(
        "Embedding not available — this build was compiled without the \
         semantic-search feature.\n\
         Install a build that includes it (downloads the model on first use):\n\
         Fix: cargo install --git https://github.com/ForgePlan/forgeplan --features semantic-search"
    );
}
