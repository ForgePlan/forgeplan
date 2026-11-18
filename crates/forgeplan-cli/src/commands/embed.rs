/// Generate embeddings for all indexable rows — artifacts (title + body
/// prefix) and their sections (title + heading + content prefix).
///
/// Explicit `embed` is one of the three permitted model obtainers: it is the
/// user's own command, so it may download. What it may **not** do is decide
/// what to publish onto — that is admission's job, under the owner — which is
/// why this command drives
/// [`facade::prepare_embedding_batch`] → [`Embedder`] →
/// [`facade::complete_embedding_batch`] rather than computing a map of
/// vectors and handing it to a writer.
#[cfg(feature = "semantic-search")]
pub async fn run() -> anyhow::Result<()> {
    use crate::ui;
    use forgeplan_core::artifact::sanitize::sanitize_for_hint;
    use forgeplan_core::constellation::ConstellationError;
    use forgeplan_core::constellation::capture::CaptureLimits;
    use forgeplan_core::constellation::facade::{self, MaintainCapability};
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
    let capability = MaintainCapability::for_strict_maintenance(ws.clone());
    let deadline = || std::time::Instant::now() + std::time::Duration::from_secs(120);

    // A workspace with no published generation cannot hold a vector — the
    // table the binding predicate would match does not exist, and paying for
    // the model before discovering that is the failure this step removes.
    if forgeplan_core::constellation::storage::read_current(capability.index_root())?.is_none() {
        let (view, _) = facade::maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .map_err(|e| anyhow::anyhow!("index bootstrap failed: {e}"))?;
        ui::info(&format!(
            "Index bootstrap: generation {}",
            view.manifest.generation_id
        ));
    }

    // The profile comes from the **cached** snapshot. An explicit `embed` may
    // download the weights below, but the profile it publishes against is
    // only ever the one whose checksums were read off disk — never a repo name
    // or a caller-supplied string.
    let cache_dir = forgeplan_core::embed::resolve_cache_dir();
    // The profile is resolved once, from one snapshot, with this workspace's
    // chunk size. Reading the checksums here and letting the caller re-resolve
    // them inside `open` is what let two runs publish the same repo under
    // different `refs/main` bytes.
    let body_prefix_chars = forgeplan_core::workspace::load_config(&ws)
        .ok()
        .and_then(|config| config.embedding.map(|embedding| embedding.chunk_size))
        .unwrap_or(forgeplan_core::constellation::jobs::DEFAULT_BODY_PREFIX_CHARS);
    let profile = forgeplan_core::constellation::jobs::ResolvedProfile::open(
        &cache_dir,
        forgeplan_core::constellation::jobs::DEFAULT_MODEL_REPO,
        facade::ENGINE_VERSION,
        body_prefix_chars,
    )
    .map_err(|e| match e {
        ConstellationError::ModelUnavailable { detail } => {
            anyhow::anyhow!("{detail}\nFix: forgeplan setup --warm-model")
        }
        other => anyhow::anyhow!("{}", other),
    })?
    .profile;

    // "Nothing to do" is a stored-binding answer. The precheck costs a scalar
    // scan and no weights; when it abstains (no cached snapshot, a busy owner)
    // the model path proceeds and admission rejudges the same eligibility.
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

    // `embed` is an explicit, operator-invoked command and it resolved the
    // profile from the cached snapshot itself, one line above — so it is
    // allowed to say which space the index serves. Admission admits only in
    // the space `CURRENT` names, and without this the command could not
    // effect a model change at all: the drain would be refused as a stale
    // caller holding the previous space.
    facade::adopt_embedding_profile(&capability, &profile, deadline())
        .await
        .map_err(|e| anyhow::anyhow!("could not point the index at the resolved profile: {e}"))?;

    ui::info("Loading embedding model...");
    let mut embedder = Embedder::new()?;

    let mut embedded = 0usize;
    let mut stale = 0usize;
    let mut failed = 0usize;
    let mut recorded_failed_total = 0usize;
    let mut cannot_judge: Vec<(String, String)> = Vec::new();

    // Each iteration is one admission → compute → completion cycle. A cycle
    // that admits nothing ends the loop; the durable pending set is what makes
    // the next `forgeplan embed` or queue drain pick up what did not fit.
    loop {
        let prepared = facade::prepare_embedding_batch(&capability, &profile, deadline())
            .await
            .map_err(|e| anyhow::anyhow!("embedding admission failed: {e}"))?;
        if prepared.inputs.is_empty() {
            break;
        }
        println!(
            "Embedding {} row(s) — artifacts and their sections...\n",
            prepared.inputs.len()
        );

        // The caller **computes** — explicit `embed` is a permitted model
        // obtainer — and does not write. Every vector leaves through the
        // guarded completion, carrying the job it was computed for.
        let mut results = Vec::with_capacity(prepared.inputs.len());
        let mut encode_failures: Vec<forgeplan_core::constellation::jobs::EmbeddingJob> =
            Vec::new();
        let mut failure_detail: Option<String> = None;
        for input in &prepared.inputs {
            let key = input.job.canonical_key.as_str();
            match embedder.embed(&input.input_text) {
                Ok(vector) => {
                    // SEC-H1 (CWE-117 / CWE-150): ids come from frontmatter and
                    // are attacker-controllable; sanitise before any emission.
                    println!("  {}", sanitize_for_hint(key));
                    results.push(forgeplan_core::constellation::jobs::ComputedEmbedding {
                        job: input.job.clone(),
                        vector,
                    });
                }
                Err(e) => {
                    eprintln!("  FAIL {} — {}", sanitize_for_hint(key), e);
                    failed += 1;
                    encode_failures.push(input.job.clone());
                    failure_detail.get_or_insert_with(|| e.to_string());
                }
            }
        }

        // A failed encode is recorded, not merely counted: the row goes to
        // `failed` with an advanced attempt and a retry delay, so the next run
        // backs off instead of re-spending the same failing inference — and so
        // "this key does not embed" is answerable from the index rather than
        // from this command's output.
        let mut recorded_failed = 0usize;
        if !encode_failures.is_empty() {
            let record_deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            let outcome = facade::fail_embedding_batch(
                &capability,
                &prepared,
                &encode_failures,
                failure_detail
                    .as_deref()
                    .unwrap_or("the model could not embed this input"),
                record_deadline,
            )
            .await
            .map_err(|e| anyhow::anyhow!("embedding failure record failed: {e}"))?;
            recorded_failed = outcome.failed.len();
        }
        recorded_failed_total += recorded_failed;

        if results.is_empty() {
            // Every input failed to encode. Their jobs are recorded failed and
            // stay pending behind a retry delay; looping again in this run would
            // re-run the same failing inference immediately.
            break;
        }

        // Publish, conditionally. A result whose job is not this batch's is
        // refused before the first write, and a stale one is reported as
        // stale — which is an answer, not a failure.
        let outcome =
            facade::complete_embedding_batch(&capability, &prepared, &results, deadline())
                .await
                .map_err(|e| anyhow::anyhow!("embedding completion failed: {e}"))?;
        embedded += outcome.published.len();
        stale += outcome.stale.len();
        for (key, reason) in outcome.cannot_judge {
            eprintln!("  cannot judge {} — {}", sanitize_for_hint(&key), reason);
            cannot_judge.push((key, reason));
        }
        for key in &outcome.stale {
            // Stale covers two facts: the row moved, or a newer admission
            // re-pointed this key. Neither wrote anything, and neither is an
            // error — the next pass picks the key up under its current job.
            eprintln!(
                "  stale {} — this vector no longer matches the row; nothing written",
                sanitize_for_hint(key)
            );
        }
    }

    if embedded == 0 && stale == 0 && cannot_judge.is_empty() && failed == 0 {
        ui::info("No artifacts or sections to embed.");
        let hint_list = vec![
            Hint::info("Create your first artifact")
                .with_action("forgeplan new prd \"<title>\"".to_string()),
        ];
        print!("{}", hints::render_next_action_line(&hint_list));
        return Ok(());
    }

    println!(
        "\nDone: {} embedded, {} stale, {} could not be judged, {} failed ({} recorded for retry).",
        embedded,
        stale,
        cannot_judge.len(),
        failed,
        recorded_failed_total
    );
    if !cannot_judge.is_empty() {
        println!(
            "Partial: {} row(s) could not be judged, so this run's coverage is incomplete.",
            cannot_judge.len()
        );
    }
    let hint_list = if failed > 0 || !cannot_judge.is_empty() {
        vec![
            Hint::warning(format!(
                "{} failed and/or could not be judged",
                failed + cannot_judge.len()
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
