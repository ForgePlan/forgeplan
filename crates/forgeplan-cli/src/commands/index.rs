//! `forgeplan index` — run one Constellation maintenance pass.
//!
//! The Constellation index is the corrected 0A/0B index: capture → reconcile →
//! staged writes → pinned verification → atomic `CURRENT` publish, with a
//! durability barrier before any durability claim. This command runs one pass
//! explicitly; it changes no local command and reads the same `.forgeplan/`
//! markdown every other command does.
//!
//! `--embed-profile <id>` additionally drains the embedding queue for that
//! profile — the 0B "queue" half. Requires the `semantic-search` feature: the
//! core takes the embedder as an injected callback so the queue machinery is
//! model-agnostic.

use std::path::PathBuf;

use anyhow::Result;

use forgeplan_core::constellation::capture::CaptureLimits;
use forgeplan_core::constellation::facade::{self, MaintainCapability};
use forgeplan_core::constellation::runtime::{MaintenanceHost, MaintenanceRunner, ReportSink};
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::workspace;

use crate::ui;

/// Display one page from the published projection without running maintenance.
pub async fn list(offset: usize) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;
    let canonical = ws.canonicalize()?;
    let capability = MaintainCapability::for_strict_maintenance(canonical);
    let page = facade::list_pinned_documents(capability.index_root(), offset, 100).await?;
    println!(
        "Published generation {} (artifacts@v{})",
        page.generation_id, page.pinned_artifacts_version
    );
    for document in &page.documents {
        println!(
            "{:<24} {:<12} {}",
            document.canonical_key,
            document.semantic_state.as_deref().unwrap_or("unknown"),
            document.relative_path
        );
    }
    if page.has_more {
        println!("Next: forgeplan index --list --offset {}", offset + 100);
    } else {
        println!("Done.");
    }
    Ok(())
}

/// The generation index: every published generation with its size and
/// protection state — current, pinned by a live reader, keepable. Read-only.
pub fn generations() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;
    let canonical = ws.canonicalize()?;
    let capability = MaintainCapability::for_strict_maintenance(canonical);
    let index =
        forgeplan_core::constellation::generations::generation_index(capability.index_root())
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("generations under {}:", capability.index_root().display());
    for g in &index.generations {
        let mut flags = Vec::new();
        if g.is_current {
            flags.push("current".to_string());
        }
        if !g.leased_by.is_empty() {
            flags.push(format!("pinned:{}", g.leased_by.join(",")));
        }
        if !g.manifest_ok {
            flags.push("manifest-unreadable".to_string());
        }
        println!(
            "  {:<40} {:>8}B  v{}/{}/{}  {}",
            g.generation_id,
            g.manifest_bytes,
            g.artifacts_version
                .map(|v| v.to_string())
                .unwrap_or_else(|| "?".into()),
            g.relations_version
                .map(|v| v.to_string())
                .unwrap_or_else(|| "?".into()),
            g.sections_version
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".into()),
            flags.join(" ")
        );
    }
    println!(
        "total: {}B over {} generation(s)",
        index.lance_bytes,
        index.generations.len()
    );
    Ok(())
}

/// Manual GC — the only deletion path, and it is a verb the operator
/// runs. Removes generations that are neither CURRENT, pinned by a live
/// reader, nor among the newest `keep` deletable ones. Every verdict is
/// printed.
pub async fn gc(keep: usize) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;
    let canonical = ws.canonicalize()?;
    let capability = MaintainCapability::for_strict_maintenance(canonical);
    let report =
        forgeplan_core::constellation::generations::gc_generations(capability.index_root(), keep)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    for (id, why) in &report.protected {
        println!("kept   {id} ({why})");
    }
    for id in &report.removed_manifests {
        println!("removed {id} (manifest)");
    }
    for prune in &report.pruned {
        match &prune.skipped {
            Some(why) => println!("prune  {}: skipped — {why}", prune.table),
            None => println!(
                "prune  {}: {} version(s), {}B freed (kept after {})",
                prune.table, prune.versions_removed, prune.bytes_removed, prune.kept_after
            ),
        }
    }
    if let Some(why) = &report.physical_prune_skipped {
        println!("prune  (all tables): skipped — {why}");
    }
    println!(
        "{} manifest(s) removed, {} protected (keep={})",
        report.removed_manifests.len(),
        report.protected.len(),
        report.keep
    );
    Ok(())
}

/// One maintenance pass plus an optional queue drain.
pub async fn run(embed_profile: Option<String>, deadline_secs: u64) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;
    // Config is optional for a maintenance pass — a workspace restored
    // from markdown alone (no config.yaml) must still index. The drain
    // path below falls back to defaults the same way.
    let _config = workspace::load_config(&ws).unwrap_or_default();

    let canonical = ws.canonicalize().unwrap_or_else(|_| ws.clone());
    let capability = MaintainCapability::for_strict_maintenance(canonical);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(deadline_secs);

    let (view, outcome) = facade::maintain_index(&capability, CaptureLimits::default(), deadline)
        .await
        .map_err(|e| anyhow::anyhow!("index maintenance failed: {e}"))?;

    println!("Index maintenance ({})", ws.display());
    println!("  generation:  {}", view.manifest.generation_id);
    println!("  progress:    {:?}", outcome.progress);
    println!("  commits:     {}", outcome.commits.len());
    for commit in &outcome.commits {
        println!(
            "    {:<18} version={} rows={}",
            commit.kind, commit.version, commit.rows_affected
        );
    }
    println!(
        "  tables:      artifacts@v{} relations@v{} sections@v{}",
        outcome.table_versions.artifacts,
        outcome.table_versions.relations,
        outcome
            .table_versions
            .sections
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".into())
    );
    match &outcome.verification {
        Some(report) => {
            println!(
                "  verified:    {}/{} keys, {}/{} edges, {}/{} sections (pinned v{}, relations v{}, sections v{})",
                report.verified_keys,
                report.expected_keys,
                report.verified_edges,
                report.expected_edges,
                report.verified_sections,
                report.expected_sections,
                report.reader_version,
                report.relations_version,
                report
                    .sections_version
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".into())
            );
        }
        None => println!("  verified:    nothing staged (no-op pass)"),
    }
    println!("  entries:     {}", view.manifest.source_entries.len());

    if let Some(profile_id) = embed_profile {
        drain(&capability, &profile_id, deadline).await?;
    } else {
        let hint_list = vec![
            Hint::info("Drain the embedding queue for a profile")
                .with_action("forgeplan index --embed-profile <profile-id>".to_string()),
        ];
        print!("{}", hints::render_next_action_line(&hint_list));
    }

    Ok(())
}

#[cfg(feature = "semantic-search")]
async fn drain(
    capability: &MaintainCapability,
    profile_id: &str,
    deadline: std::time::Instant,
) -> Result<()> {
    // `ShutdownOutcome` is named below; `InferenceWorker::shutdown` returns
    // it, so both come from the same feature-gated module.
    #[cfg(feature = "semantic-search")]
    use forgeplan_core::constellation::runtime::{InferenceWorker, ShutdownOutcome};
    use forgeplan_core::constellation::{jobs, storage};

    // The `--embed-profile` argument is checked against the profile the
    // **cached snapshot** actually resolves to, not used as a label. Labelling
    // vectors with an arbitrary string is what made `--embed-profile` able to
    // stamp BGE-M3 vectors into a space that was never BGE-M3's.
    let expected = profile_id.to_string();
    let cache_dir = forgeplan_core::embed::resolve_cache_dir();
    let body_prefix_chars = forgeplan_core::workspace::load_config(capability.workspace())
        .ok()
        .and_then(|config| config.embedding.map(|embedding| embedding.chunk_size))
        .unwrap_or(jobs::DEFAULT_BODY_PREFIX_CHARS);
    let resolved = jobs::ResolvedProfile::open(
        &cache_dir,
        jobs::DEFAULT_MODEL_REPO,
        facade::ENGINE_VERSION,
        body_prefix_chars,
    )?;
    if resolved.profile.profile_id() != expected {
        anyhow::bail!(
            "--embed-profile {expected} does not match the profile the cached snapshot resolves \
             to ({}). Re-run with that id, or point FORGEPLAN_MODEL_CACHE at the snapshot you \
             mean.",
            resolved.profile.profile_id()
        );
    }
    let profile = resolved.profile.clone();

    // No published generation means nothing to drain onto; a maintain pass is
    // what creates the rows, and `forgeplan index` has just run one.
    if storage::read_current(capability.index_root())?.is_none() {
        ui::warning("no published generation — nothing to drain");
        return Ok(());
    }

    ui::info(&format!("Draining embedding queue (profile {profile_id})…"));
    // This command is the migration. Admission only ever works in the space
    // `CURRENT` already serves, so pointing the index at the verified profile
    // is what makes a model change take effect — and it is done here, by the
    // one caller that resolved the profile from the cache on purpose. A
    // background drain has no such authority and will be refused if the
    // cached profile and the served profile disagree.
    match facade::adopt_embedding_profile(capability, &profile, deadline).await {
        Ok(true) => ui::info(&format!("Index now serves embedding space {profile_id}.")),
        Ok(false) => {}
        Err(e) => anyhow::bail!("could not point the index at profile {profile_id}: {e}"),
    }

    // One inference worker for this drain, opened from the **same snapshot the
    // profile was resolved from**. `ResolvedProfile::open` already refused to
    // continue without a cached snapshot, so `spawn()` here could only ever
    // reload the same 2 GB a second time — and if `refs/main` moved between
    // the two, the profile would be advertised against weights that were
    // never embedded. The core holds the worker only between admission and
    // completion — never across the model call.
    let mut worker = InferenceWorker::spawn_resolved(jobs::DEFAULT_MODEL_NAME, &resolved)?;
    let outcome = facade::drain_embedding_queue(
        capability,
        &profile,
        &mut worker,
        deadline,
        // `forgeplan index` is a one-shot command with no host loop, so no
        // stop or control scan can interrupt it early; `deadline` is the bound.
        &forgeplan_core::constellation::runtime::DrainControl::never(),
    )
    .await;
    // Stop admission without waiting indefinitely for a native model call.
    let shutdown = worker.shutdown();
    if matches!(&shutdown, Ok(ShutdownOutcome::Detached)) {
        eprintln!("WARN: in-flight inference thread detached; native call has not finished");
    }
    let outcome = outcome.map_err(|e| anyhow::anyhow!("embedding queue drain failed: {e}"))?;
    shutdown.map_err(|e| anyhow::anyhow!("inference worker shutdown failed: {e}"))?;

    let before_eligible = outcome.before.eligible_keys().len();
    let after_eligible = outcome.after.eligible_keys().len();
    let cannot_judge = outcome.after.cannot_judge().len();

    println!(
        "  queue:       {} eligible before, {} embedded, {} stale completions, {} recorded failed",
        before_eligible, outcome.embedded, outcome.stale_completions, outcome.failed
    );
    println!(
        "  coverage:    {} still eligible after, {} cannot-judge",
        after_eligible, cannot_judge
    );
    if after_eligible > 0 {
        println!("  note:        run again — the queue is not empty");
    }
    Ok(())
}

#[cfg(not(feature = "semantic-search"))]
async fn drain(
    _capability: &MaintainCapability,
    _profile_id: &str,
    _deadline: std::time::Instant,
) -> Result<()> {
    ui::warning("this build has no `semantic-search` feature — the embedder is absent");
    anyhow::bail!("rebuild with --features semantic-search to drain the embedding queue")
}

/// Maintenance for a long-running host (`forgeplan serve --auto-index`).
///
/// Returns a **held** [`MaintenanceHost`], not a detached task. The old shape
/// spawned a `tokio::spawn` that ran one pass and was then dropped: nothing
/// could tell whether it was still running, nothing stopped it, and a save
/// after that single pass stayed unindexed until the next process started. The
/// host keeps watching, keeps the control scan running and can be shut down
/// explicitly when the server stops.
///
/// # Which workspace
///
/// `explicit` is the `--workspace` argument, and it **wins over `cwd`**. That
/// is the whole point of the parameter: a server started in X that is asked to
/// maintain Y must maintain Y, and silently maintaining X would leave the
/// request's own workspace stale while reporting that maintenance was running.
///
/// # Authority
///
/// A host is a publisher. It is started only for a workspace that is *supposed*
/// to be maintained:
///
/// * an explicit `--auto-index` — the operator asked for it by name; or
/// * a workspace that is already migrated to the constellation protocol, whose
///   index therefore has no legacy writer to keep it current.
///
/// A workspace that is neither gets no host at all. That is not politeness: a
/// legacy workspace still has `LanceStore` as its writer, and a constellation
/// host racing it is precisely the split-brain the migration exists to end.
/// A read-only or uninitialised `workspace=` target is likewise skipped rather
/// than bootstrapped.
///
/// Returns `Ok(None)` when no workspace qualifies, which is a skip and not an
/// error — stated rather than implied by a printed line.
pub async fn spawn_background(
    cwd: PathBuf,
    explicit: Option<&std::path::Path>,
) -> anyhow::Result<Option<MaintenanceRunner>> {
    let start_from = explicit.map(std::path::Path::to_path_buf).unwrap_or(cwd);
    let Some(ws) = workspace::find_workspace(&start_from) else {
        eprintln!(
            "auto-index: no .forgeplan/ under {} — skipped",
            start_from.display()
        );
        return Ok(None);
    };
    if explicit.is_none() && !forgeplan_core::constellation::facade::workspace_is_migrated(&ws) {
        eprintln!(
            "auto-index: {} is not migrated and no explicit --auto-index scope was given — skipped",
            ws.display()
        );
        return Ok(None);
    }
    let canonical = ws.canonicalize().unwrap_or(ws);
    let host = MaintenanceHost::start(
        MaintainCapability::for_strict_maintenance(canonical),
        ReportSink::Background {
            label: "auto-index",
        },
    )
    .await?;
    Ok(Some(host.drive()))
}
