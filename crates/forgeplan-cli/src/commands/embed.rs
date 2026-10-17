/// Generate embeddings for all artifacts (title + body) for semantic search.
#[cfg(feature = "semantic-search")]
pub async fn run() -> anyhow::Result<()> {
    use crate::ui;
    use forgeplan_core::artifact::sanitize::sanitize_for_hint;
    use forgeplan_core::constellation::ConstellationError;
    use forgeplan_core::constellation::capture::CaptureLimits;
    use forgeplan_core::constellation::facade::{
        self, MaintainCapability, eligible_inputs, embed_with_publisher,
    };
    use forgeplan_core::embed::Embedder;
    use forgeplan_core::hints::{self, Hint};

    // The workspace path, resolved the way `common::open_store` resolves it
    // (`common.rs:12-13`) but **without opening the store**. `open_store` would
    // hand back the path only by opening a `LanceStore`, and a store open runs
    // schema migrations — which is exactly what this command must not do: it
    // writes through the publisher, under the publisher's owner.
    let cwd = std::env::current_dir()?;
    let ws = forgeplan_core::workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;
    let ws = ws.canonicalize().unwrap_or_else(|_| ws.clone());

    // 1. The work list, and the text for each key, from the **capture** —
    //    not from the store. `body_hash` is deliberately not consulted: it is a
    //    change-detection stamp on a derived row, it cannot see a profile
    //    change, and as an "already embedded" test it would let a row whose
    //    vector belongs to a different space claim to be current.
    //
    //    A key that cannot be judged does not appear here; `embed_with_publisher`
    //    reports those separately, with reasons.
    let inputs = eligible_inputs(&ws).await.map_err(|e| match e {
        ConstellationError::ModelUnavailable { detail } => {
            anyhow::anyhow!("{detail}\nFix: forgeplan setup --warm-model")
        }
        other => anyhow::anyhow!("{}", other),
    })?;

    if inputs.is_empty() {
        ui::info("No artifacts to embed.");
        let hint_list = vec![
            Hint::info("Create your first artifact")
                .with_action("forgeplan new prd \"<title>\"".to_string()),
        ];
        print!("{}", hints::render_next_action_line(&hint_list));
        return Ok(());
    }

    // A workspace with no published generation cannot hold a vector — the
    // table the binding predicate would match does not exist, and paying for
    // the model before discovering that is the failure this step removes.
    // Bootstrap the index the same way `forgeplan index` does; the rows it
    // publishes are the rows the vectors below bind to.
    let capability = MaintainCapability::for_strict_maintenance(ws.clone());
    if forgeplan_core::constellation::storage::read_current(capability.index_root())?.is_none() {
        let (view, _) = facade::maintain_index(
            &capability,
            CaptureLimits::default(),
            std::time::Instant::now() + std::time::Duration::from_secs(120),
        )
        .await
        .map_err(|e| anyhow::anyhow!("index bootstrap failed: {e}"))?;
        ui::info(&format!(
            "Index bootstrap: generation {}",
            view.manifest.generation_id
        ));
    }

    // "Nothing to do" is a stored-binding answer, not a capture answer —
    // `inputs` lists what exists. The precheck costs a scalar scan and no
    // weights; when it abstains (no cached snapshot, a busy owner) the model
    // path proceeds and the real publish rejudges the same eligibility.
    if let Some(precheck) = facade::embedding_precheck(&ws).await?
        && precheck.eligible == 0
    {
        println!(
            "Done: 0 embedded, 0 stale, {} could not be judged, 0 failed.",
            precheck.cannot_judge
        );
        println!("{} already current.", precheck.current);
        if precheck.cannot_judge > 0 {
            println!(
                "Partial: {} row(s) could not be judged, so this run's coverage is incomplete.",
                precheck.cannot_judge
            );
        }
        let hint_list = if precheck.cannot_judge > 0 {
            vec![
                Hint::warning(format!("{} could not be judged", precheck.cannot_judge))
                    .with_action("forgeplan health".to_string()),
            ]
        } else {
            vec![
                Hint::info("Run a semantic search")
                    .with_action("forgeplan search \"<query>\"".to_string()),
            ]
        };
        print!("{}", hints::render_next_action_line(&hint_list));
        return Ok(());
    }

    if let Some(notice) = forgeplan_core::embed::first_run_notice() {
        ui::info(&notice);
    }

    ui::info("Loading embedding model...");
    let mut embedder = Embedder::new()?;

    println!("Embedding {} artifact(s)...\n", inputs.len());

    // 2. The caller **computes** — explicit `embed` is a permitted model
    //    obtainer — and does not write. Every vector leaves through the
    //    publisher, which applies the conditional `Binding` guard.
    let mut vectors = std::collections::BTreeMap::new();
    let mut ok = 0usize;
    let mut err = 0usize;
    for (key, text) in &inputs {
        match embedder.embed(text) {
            Ok(vector) => {
                // SEC-H1 (CWE-117 / CWE-150): ids come from frontmatter and are
                // attacker-controllable; sanitise before any TTY emission.
                println!(
                    "  {} \"{}\"",
                    sanitize_for_hint(key),
                    sanitize_for_hint(key)
                );
                vectors.insert(key.clone(), vector);
                ok += 1;
            }
            Err(e) => {
                eprintln!("  FAIL {} — {}", sanitize_for_hint(key), e);
                err += 1;
            }
        }
    }

    // 3. Publish, conditionally. A key absent from the capture is refused
    //    inside `publish_embeddings`, and a stale one is reported as stale —
    //    which is an answer, not a failure.
    let outcome = embed_with_publisher(&ws, &vectors)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    for (key, reason) in &outcome.cannot_judge {
        eprintln!("  cannot judge {} — {}", sanitize_for_hint(key), reason);
    }
    for key in &outcome.stale {
        eprintln!(
            "  stale {} — the row moved; nothing written",
            sanitize_for_hint(key)
        );
    }

    println!(
        "\nDone: {} embedded, {} stale, {} could not be judged, {} failed.",
        outcome.published.len(),
        outcome.stale.len(),
        outcome.cannot_judge.len(),
        err
    );
    if !outcome.is_complete() {
        println!(
            "Partial: {} row(s) could not be judged, so this run's coverage is incomplete.",
            outcome.cannot_judge.len()
        );
    }
    let _ = ok;
    let hint_list = if err > 0 || !outcome.is_complete() {
        vec![
            Hint::warning(format!(
                "{} failed and/or could not be judged",
                err + outcome.cannot_judge.len()
            ))
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
