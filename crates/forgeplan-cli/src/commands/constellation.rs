//! `forgeplan constellation` — the source-registry commands (stage 2).
//!
//! The registry is a per-user file, `~/.forgeplan/constellations.yaml`, so
//! these verbs work from any directory and from any ForgePlan build — a
//! constellation made by one CLI instance is visible to every other.
//!
//! `search` and `get` fan out over the active constellation's sources
//! through the federation facade: a source with a published index answers
//! from its pinned generation, one without is read from canonical
//! markdown, and a source that cannot answer is a named failure — never
//! zero silent matches.

use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;

use forgeplan_core::constellation::federation::{self, FederatedGet};
use forgeplan_core::constellation::registry;
use forgeplan_core::constellation::snapshot as snap;

#[derive(Subcommand)]
pub enum ConstellationAction {
    /// Create a named constellation. The first one created becomes active.
    Create { name: String },
    /// Add a `.forgeplan` directory as a source. Validates the path at
    /// registration time — a symlink or a non-workspace fails here, not at
    /// every later read.
    Add {
        /// Path to the source's `.forgeplan` directory
        path: PathBuf,
        /// Constellation to add to (default: the active one)
        #[arg(long)]
        to: Option<String>,
    },
    /// Remove a source from a constellation.
    Remove {
        /// Path as it was added (canonicalized before matching)
        path: PathBuf,
        /// Constellation to remove from (default: the active one)
        #[arg(long)]
        from: Option<String>,
    },
    /// List all constellations and which is active.
    List,
    /// Select the active constellation for `search`/`get`.
    Use { name: String },
    /// List the sources of a constellation (default: active).
    Sources {
        /// Constellation to inspect (default: the active one)
        #[arg(long)]
        of: Option<String>,
    },
    /// Health-check every registered source: reachable, looks like a
    /// workspace, has a published index.
    Doctor,
    /// Pin every source of the active constellation at its current Git
    /// HEAD and write the lockfile to `~/.forgeplan/snapshots/`. Sources
    /// that are not git checkouts are reported, not pinned.
    Snapshot,
    /// Search across the active constellation's sources.
    Search {
        query: String,
        /// Maximum hits (default: 10)
        #[arg(long, short = 'n', default_value = "10")]
        limit: usize,
        /// Read at a recorded snapshot instead of the live sources
        #[arg(long)]
        snapshot: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Fetch one artifact by id across the active constellation. When two
    /// sources claim the same id the answer is `ambiguous` and names them.
    Get {
        id: String,
        /// Read at a recorded snapshot instead of the live sources
        #[arg(long)]
        snapshot: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

fn load_registry() -> Result<(PathBuf, registry::ConstellationRegistry)> {
    let path = registry::default_registry_path().map_err(|e| anyhow::anyhow!("{e}"))?;
    let registry = registry::load(&path).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok((path, registry))
}

fn save_registry(path: &std::path::Path, registry: &registry::ConstellationRegistry) -> Result<()> {
    registry::save(path, registry).map_err(|e| anyhow::anyhow!("{e}"))
}

/// The constellation `--to`/`--from`/`--of` names, or the active one.
fn resolve_name(
    registry: &registry::ConstellationRegistry,
    explicit: Option<&str>,
) -> Result<String> {
    if let Some(name) = explicit {
        return Ok(name.to_string());
    }
    registry.current.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "no active constellation — create one with `forgeplan constellation create <name>`"
        )
    })
}

pub async fn run(action: ConstellationAction) -> Result<()> {
    match action {
        ConstellationAction::Create { name } => {
            let (path, mut registry) = load_registry()?;
            registry::create(&mut registry, &name).map_err(|e| anyhow::anyhow!("{e}"))?;
            save_registry(&path, &registry)?;
            println!("Created constellation '{name}' (active)");
            Ok(())
        }
        ConstellationAction::Add { path, to } => {
            let (reg_path, mut registry) = load_registry()?;
            let name = resolve_name(&registry, to.as_deref())?;
            let canonical = registry::add_source(&mut registry, &name, &path)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            save_registry(&reg_path, &registry)?;
            println!("Added {canonical} to '{name}'");
            Ok(())
        }
        ConstellationAction::Remove { path, from } => {
            let (reg_path, mut registry) = load_registry()?;
            let name = resolve_name(&registry, from.as_deref())?;
            let canonical = registry::remove_source(&mut registry, &name, &path)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            save_registry(&reg_path, &registry)?;
            println!("Removed {canonical} from '{name}'");
            Ok(())
        }
        ConstellationAction::List => {
            let (_path, registry) = load_registry()?;
            if registry.constellations.is_empty() {
                println!("No constellations. Create one: forgeplan constellation create <name>");
                return Ok(());
            }
            for (name, def) in &registry.constellations {
                let marker = if registry.current.as_deref() == Some(name.as_str()) {
                    "*"
                } else {
                    " "
                };
                println!("{marker} {name} — {} source(s)", def.sources.len());
            }
            Ok(())
        }
        ConstellationAction::Use { name } => {
            let (path, mut registry) = load_registry()?;
            registry::use_constellation(&mut registry, &name)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            save_registry(&path, &registry)?;
            println!("Active constellation: {name}");
            Ok(())
        }
        ConstellationAction::Sources { of } => {
            let (_path, registry) = load_registry()?;
            let name = resolve_name(&registry, of.as_deref())?;
            let def = registry
                .constellations
                .get(&name)
                .ok_or_else(|| anyhow::anyhow!("no constellation named {name:?}"))?;
            if def.sources.is_empty() {
                println!("'{name}' has no sources. Add one: forgeplan constellation add <path>");
                return Ok(());
            }
            for source in &def.sources {
                println!("{}", source.path);
            }
            Ok(())
        }
        ConstellationAction::Doctor => {
            let (_path, registry) = load_registry()?;
            let rows = registry::doctor(&registry);
            if rows.is_empty() {
                println!("No sources registered.");
                return Ok(());
            }
            let mut problems = 0usize;
            for (constellation, health) in &rows {
                let state = if !health.readable {
                    problems += 1;
                    "broken"
                } else if health.indexed {
                    "indexed"
                } else {
                    "markdown"
                };
                let detail = health
                    .detail
                    .as_deref()
                    .map(|d| format!(" — {d}"))
                    .unwrap_or_default();
                println!("{state:<8} [{constellation}] {}{detail}", health.path);
            }
            if problems > 0 {
                println!("{problems} source(s) need attention.");
            } else {
                println!("All sources healthy.");
            }
            Ok(())
        }
        ConstellationAction::Snapshot => {
            let (_path, registry) = load_registry()?;
            let name = registry
                .current
                .clone()
                .ok_or_else(|| anyhow::anyhow!("no active constellation to snapshot"))?;
            let specs = registry::current_sources(&registry).unwrap_or_default();
            if specs.is_empty() {
                anyhow::bail!("the active constellation has no sources");
            }
            let snapshot =
                snap::capture_snapshot(&specs, Some(&name)).map_err(|e| anyhow::anyhow!("{e}"))?;
            let home = dirs::home_dir()
                .ok_or_else(|| anyhow::anyhow!("cannot locate the user home directory"))?;
            let path = snap::snapshot_path(&home, &snapshot.snapshot_id);
            snap::write_snapshot(&path, &snapshot).map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("Snapshot {} → {}", snapshot.snapshot_id, path.display());
            for entry in &snapshot.sources {
                println!(
                    "  {} @ {} ({} files)",
                    entry.checkout_root,
                    &entry.git_oid[..12],
                    entry.files
                );
            }
            for skip in &snapshot.skipped {
                eprintln!("  skipped {} — {}", skip.path, skip.reason);
            }
            Ok(())
        }
        ConstellationAction::Search {
            query,
            limit,
            snapshot,
            json,
        } => {
            let (_path, registry) = load_registry()?;
            let outcome = if let Some(id) = &snapshot {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow::anyhow!("cannot locate the user home directory"))?;
                let snap = snap::load_snapshot(&snap::snapshot_path(&home, id))
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                snap::snapshot_search(&snap, &query, limit)?
            } else {
                let specs = registry::current_sources(&registry).ok_or_else(|| {
                    anyhow::anyhow!(
                        "no active constellation — `forgeplan constellation create` then `add`"
                    )
                })?;
                if specs.is_empty() {
                    anyhow::bail!(
                        "the active constellation has no sources — `forgeplan constellation add <path>`"
                    );
                }
                federation::federated_search(&specs, &query, limit).await?
            };
            if json {
                print_search_json(&outcome)?;
            } else {
                print_search_text(&outcome);
            }
            Ok(())
        }
        ConstellationAction::Get { id, snapshot, json } => {
            let (_path, registry) = load_registry()?;
            // Stage 4 — qualified refs: `ADR-001@fp-ws-b` or
            // `ADR-001@src:b7d6…`. The qualifier narrows the source set
            // before reading; an unmatched qualifier is an error that
            // names what was registered, never a silent wider search.
            let (bare_id, qualifier) = match id.split_once('@') {
                Some((k, q)) if !k.is_empty() && !q.is_empty() => {
                    (k.to_string(), Some(q.to_string()))
                }
                _ => (id.clone(), None),
            };
            let id = bare_id;
            if let Some(snap_id) = &snapshot {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow::anyhow!("cannot locate the user home directory"))?;
                let snap = snap::load_snapshot(&snap::snapshot_path(&home, snap_id))
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                let found = snap::snapshot_get(&snap, &id)?;
                return match found.len() {
                    0 => {
                        println!("Not found at snapshot {snap_id}: {id}");
                        Ok(())
                    }
                    1 => {
                        let (source, rel, bytes) = &found[0];
                        if json {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&serde_json::json!({
                                    "result": "found",
                                    "source": source,
                                    "relative_path": rel,
                                    "body": String::from_utf8_lossy(bytes),
                                }))?
                            );
                        } else {
                            println!("{id} at {} [{}]", rel, &source.version_key[..40]);
                            print!("{}", String::from_utf8_lossy(bytes));
                        }
                        Ok(())
                    }
                    _ => {
                        println!(
                            "'{id}' is claimed by {} sources at this snapshot:",
                            found.len()
                        );
                        for (source, rel, _) in &found {
                            println!("  {} ({})", rel, source.source_id);
                        }
                        Ok(())
                    }
                };
            }
            let mut specs = registry::current_sources(&registry)
                .ok_or_else(|| anyhow::anyhow!("no active constellation"))?;
            if let Some(qualifier) = &qualifier {
                specs.retain(|spec| {
                    spec.path.display().to_string().contains(qualifier.as_str())
                        || federation::spec_source_id(spec)
                            .as_deref()
                            .is_some_and(|sid| sid == qualifier || sid.starts_with(qualifier))
                });
                if specs.is_empty() {
                    let known: Vec<String> = registry::current_sources(&registry)
                        .unwrap_or_default()
                        .iter()
                        .map(|s| {
                            let id = federation::spec_source_id(s)
                                .unwrap_or_else(|| "unreadable".into());
                            format!("{} ({id})", s.path.display())
                        })
                        .collect();
                    anyhow::bail!(
                        "qualifier @{qualifier} matches no source. Registered:
  {}",
                        known.join(
                            "
  "
                        )
                    );
                }
            }
            let result = federation::federated_get(&specs, &id).await?;
            match result {
                FederatedGet::Found(artifact) => {
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "result": "found",
                                "canonical_key": artifact.canonical_key,
                                "display_id": artifact.display_id,
                                "title": artifact.title,
                                "kind": artifact.kind,
                                "status": artifact.status,
                                "semantic_state": artifact.semantic_state,
                                "body": artifact.body,
                                "source": artifact.source,
                            }))?
                        );
                    } else {
                        println!(
                            "{} ({}) — {} [{}:{}]",
                            artifact
                                .display_id
                                .as_deref()
                                .unwrap_or(&artifact.canonical_key),
                            artifact.kind.as_deref().unwrap_or("artifact"),
                            artifact.title,
                            artifact.source.channel.channel_str(),
                            &artifact.source.source_id,
                        );
                        println!("{}", artifact.body);
                    }
                }
                FederatedGet::Ambiguous { sources } => {
                    println!(
                        "'{id}' is claimed by {} sources — qualify which you mean:",
                        sources.len()
                    );
                    for source in &sources {
                        println!("  {} ({})", source.path, source.source_id);
                    }
                }
                FederatedGet::NotFound { failures } => {
                    println!("Not found: {id}");
                    for failure in &failures {
                        println!("  (unread source: {} — {})", failure.path, failure.reason);
                    }
                }
            }
            Ok(())
        }
    }
}

fn print_search_text(outcome: &federation::FederatedSearchOutcome) {
    for failure in &outcome.failures {
        eprintln!(
            "warning: source {} unread — {}",
            failure.path, failure.reason
        );
    }
    if outcome.hits.is_empty() {
        println!("No matches.");
        return;
    }
    for hit in &outcome.hits {
        println!(
            "{:<20} {:<9} {:<8} {}",
            hit.display_id.as_deref().unwrap_or(&hit.canonical_key),
            hit.source.channel.channel_str(),
            hit.semantic_state.as_deref().unwrap_or("—"),
            hit.title,
        );
        println!("    {} ({})", hit.source.path, hit.source.source_id);
    }
    if outcome.truncated {
        println!("(truncated — raise --limit)");
    }
    if outcome.partial {
        println!("(partial — some sources did not answer)");
    }
}

fn print_search_json(outcome: &federation::FederatedSearchOutcome) -> Result<()> {
    let hits: Vec<serde_json::Value> = outcome
        .hits
        .iter()
        .map(|h| {
            serde_json::json!({
                "canonical_key": h.canonical_key,
                "display_id": h.display_id,
                "title": h.title,
                "kind": h.kind,
                "status": h.status,
                "semantic_state": h.semantic_state,
                "source": h.source,
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "hits": hits,
            "sources": outcome.sources,
            "failures": outcome.failures,
            "truncated": outcome.truncated,
            "partial": outcome.partial,
        }))?
    );
    Ok(())
}
