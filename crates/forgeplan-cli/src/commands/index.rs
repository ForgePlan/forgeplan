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

/// One maintenance pass plus an optional queue drain.
pub async fn run(embed_profile: Option<String>, deadline_secs: u64) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;
    let _config = workspace::load_config(&ws)?;

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
        "  tables:      artifacts@v{} relations@v{}",
        outcome.table_versions.artifacts, outcome.table_versions.relations
    );
    match &outcome.verification {
        Some(report) => {
            println!(
                "  verified:    {}/{} keys, {}/{} edges (pinned v{}, relations v{})",
                report.verified_keys,
                report.expected_keys,
                report.verified_edges,
                report.expected_edges,
                report.reader_version,
                report.relations_version
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
    use forgeplan_core::embed::Embedder;

    let config = crate::commands::common::config().unwrap_or_default();
    let prefix = config
        .embedding
        .as_ref()
        .map(|e| e.chunk_size)
        .unwrap_or(2000);

    let mut embedder = Embedder::new()?;
    ui::info(&format!("Draining embedding queue (profile {profile_id})…"));

    let outcome = facade::drain_embedding_queue(
        capability,
        profile_id,
        prefix,
        |inputs| {
            let texts: Vec<&str> = inputs.iter().map(|(_, t)| t.as_str()).collect();
            embedder.embed_batch(&texts).map_err(|e| {
                forgeplan_core::constellation::ConstellationError::ModelUnavailable {
                    detail: format!("embedding failed: {e}"),
                }
            })
        },
        deadline,
    )
    .await
    .map_err(|e| anyhow::anyhow!("embedding queue drain failed: {e}"))?;

    let before_eligible = outcome.before.eligible_keys().len();
    let after_eligible = outcome.after.eligible_keys().len();
    let cannot_judge = outcome.after.cannot_judge().len();

    println!(
        "  queue:       {} eligible before, {} embedded, {} stale completions",
        before_eligible, outcome.embedded, outcome.stale_completions
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

/// Fire-and-forget maintenance for a long-running host (`forgeplan serve
/// --auto-index`). Runs one `maintain_index` pass on a detached task, then —
/// when a `semantic-search` build and a configured model exist — drains the
/// embedding queue for that profile.
///
/// It never blocks the server: a failure is logged to stderr and the MCP
/// surface keeps answering reads from whatever generation is published.
/// Watch-style continuous upkeep stays with `forgeplan watch
/// --constellation`; this is the "queue in serve" half of 0B.
pub fn spawn_background(cwd: PathBuf) {
    tokio::spawn(async move {
        let Some(ws) = workspace::find_workspace(&cwd) else {
            eprintln!(
                "auto-index: no .forgeplan/ under {} — skipped",
                cwd.display()
            );
            return;
        };
        let canonical = ws.canonicalize().unwrap_or_else(|_| ws.clone());
        let capability = MaintainCapability::for_strict_maintenance(canonical);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
        match facade::maintain_index(&capability, CaptureLimits::default(), deadline).await {
            Ok((view, outcome)) => eprintln!(
                "auto-index: {} — {} commit(s)",
                view.manifest.generation_id,
                outcome.commits.len()
            ),
            Err(e) => eprintln!("auto-index: maintenance failed: {e}"),
        }

        drain_queue_if_configured(&capability, deadline, "auto-index").await;
    });
}

/// Drain the pending embedding queue **only when** a model is configured and
/// resolvable — the 0B "no manual `embed`" half for long-running hosts.
///
/// Feature-off or unconfigured is a silent no-op: the queue drains through
/// binding-predicated publish, so a skipped drain means pending rows stay
/// pending — visible in `constellation_status` coverage — never corrupted.
/// `label` prefixes the stderr report so `watch` and `serve` log lines are
/// distinguishable.
#[cfg(feature = "semantic-search")]
pub(crate) async fn drain_queue_if_configured(
    capability: &MaintainCapability,
    deadline: std::time::Instant,
    label: &str,
) {
    drain_queue_with_config(capability, deadline, label, false).await;
}

/// Hook path: never download a model while Git is waiting for the hook.
#[cfg(feature = "semantic-search")]
pub(crate) async fn drain_queue_if_cached(
    capability: &MaintainCapability,
    deadline: std::time::Instant,
    label: &str,
) {
    drain_queue_with_config(capability, deadline, label, true).await;
}

#[cfg(feature = "semantic-search")]
async fn drain_queue_with_config(
    capability: &MaintainCapability,
    deadline: std::time::Instant,
    label: &str,
    cached_only: bool,
) {
    use forgeplan_core::embed::Embedder;

    let config = crate::commands::common::config().unwrap_or_default();
    let Some(model) = config.embedding.as_ref().map(|e| e.model.clone()) else {
        return;
    };
    let prefix = config
        .embedding
        .as_ref()
        .map(|e| e.chunk_size)
        .unwrap_or(2000);
    let Some(repo) = forgeplan_core::embed::resolve_repo(&model) else {
        eprintln!("{label}: embedding model `{model}` unsupported — queue skipped");
        return;
    };
    let cache_dir = forgeplan_core::embed::resolve_cache_dir();
    let profile_id = match forgeplan_core::constellation::jobs::snapshot_checksums(&cache_dir, repo)
        .and_then(|checksums| {
            forgeplan_core::constellation::jobs::ResolvedProfile::open(
                &cache_dir,
                repo,
                &checksums,
                forgeplan_core::constellation::facade::ENGINE_VERSION,
            )
            .map(|resolved| resolved.profile.profile_id())
        }) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{label}: profile for `{model}` unresolved — queue skipped: {e}");
            return;
        }
    };
    let opened = if cached_only {
        Embedder::with_cached_model(&model)
    } else {
        Embedder::with_model(&model)
    };
    let Ok(mut embedder) = opened else {
        eprintln!("{label}: embedding model `{model}` unavailable — queue skipped");
        return;
    };
    match facade::drain_embedding_queue(
        capability,
        &profile_id,
        prefix,
        |inputs| {
            let texts: Vec<&str> = inputs.iter().map(|(_, t)| t.as_str()).collect();
            embedder.embed_batch(&texts).map_err(|e| {
                forgeplan_core::constellation::ConstellationError::ModelUnavailable {
                    detail: format!("embedding failed: {e}"),
                }
            })
        },
        deadline,
    )
    .await
    {
        Ok(outcome) => eprintln!(
            "{label}: queue drained — {} embedded, {} stale, {} still eligible",
            outcome.embedded,
            outcome.stale_completions,
            outcome.after.eligible_keys().len()
        ),
        Err(e) => eprintln!("{label}: queue drain failed: {e}"),
    }
}

/// No-op without the `semantic-search` feature — same call sites compile.
#[cfg(not(feature = "semantic-search"))]
pub(crate) async fn drain_queue_if_configured(
    _capability: &MaintainCapability,
    _deadline: std::time::Instant,
    _label: &str,
) {
}

#[cfg(not(feature = "semantic-search"))]
pub(crate) async fn drain_queue_if_cached(
    _capability: &MaintainCapability,
    _deadline: std::time::Instant,
    _label: &str,
) {
}
