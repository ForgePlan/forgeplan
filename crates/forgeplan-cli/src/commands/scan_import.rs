use std::path::Path;

use anyhow::Result;
use console::style;

use forgeplan_core::hints::{self, Hint};
use forgeplan_core::scan::detect::DetectionTier;
use forgeplan_core::scan::import::{ImportStatus, ScanImportOptions, scan_and_import_to_workspace};

use crate::commands::common;

/// The canonical key a source file declares, read the way the import reads it.
///
/// ADR-030: the key of an imported document is the slug its frontmatter
/// declares when that slug is a valid canonical key, and its display id
/// when it declares none. This is the same rule the import itself applies
/// (`canonical_key_for_import` in the core), so the report names the key the
/// document was actually written under.
///
/// Returns `None` only when the file itself cannot be read: the report then
/// names the path alone rather than guessing a key. A file whose frontmatter
/// does not parse keeps its display id as its key — the same fallback the
/// import applies.
fn source_canonical_key(scan_root: &Path, relative_path: &str, display_id: &str) -> Option<String> {
    let text = std::fs::read_to_string(scan_root.join(relative_path)).ok()?;
    let key = match forgeplan_core::artifact::frontmatter::parse_frontmatter(&text) {
        Ok((frontmatter, _)) => {
            match forgeplan_core::artifact::frontmatter::slug_from_frontmatter(&frontmatter) {
                Some(slug) if forgeplan_core::artifact::types::is_canonical_key(slug) => {
                    slug.to_string()
                }
                _ => display_id.to_string(),
            }
        }
        Err(_) => display_id.to_string(),
    };
    Some(key)
}

/// `forgeplan scan-import [--path <dir>] [--dry-run]`
///
/// ADR-030: an imported document keeps the key it was written with — the
/// slug its frontmatter declares when that slug is valid, and its display id
/// when it declares none. An import never mints a key, and the derived index
/// row is keyed by that same key.
pub async fn run(path: Option<&str>, dry_run: bool) -> Result<()> {
    // A dry run holds a shared permit rather than a writer lock or store.
    let (ws, lock) = if dry_run {
        (common::find_workspace_path()?, None)
    } else {
        let (ws, lock) = common::open_workspace_locked().await?;
        (ws, Some(lock))
    };
    let read_guard = if dry_run {
        Some(forgeplan_core::workspace::lock::acquire_workspace_read_lock(&ws).await?)
    } else {
        None
    };
    let project_root = ws
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine project root"))?;

    let options = ScanImportOptions {
        dry_run,
        custom_path: path.map(|s| s.to_string()),
    };
    let (store, plan, allocation) = if let Some(lock) = lock.as_ref() {
        let kinds = forgeplan_core::scan::import::scan_import_kinds(project_root, &options)?;
        let allocation =
            forgeplan_core::artifact::id_alloc::acquire_allocation_guard(&ws, lock, &kinds).await?;
        let before_store = if common::index_migrated(&ws) {
            None
        } else {
            Some(forgeplan_core::db::store::LanceStore::open(&ws).await?)
        };
        let before = forgeplan_core::scan::import::plan_scan_import(
            project_root,
            &ws,
            lock,
            &allocation,
            before_store.as_ref(),
            &options,
        )
        .await?;
        drop(before_store);
        let store = common::prepare_store_locked_checked(&ws, lock, &[]).await?;
        let after = forgeplan_core::scan::import::plan_scan_import(
            project_root,
            &ws,
            lock,
            &allocation,
            store.as_ref(),
            &options,
        )
        .await?;
        if before != after {
            anyhow::bail!("scan identities or sources changed while reopening the index");
        }
        (store, Some(after), Some(allocation))
    } else {
        (None, None, None)
    };
    let migrated = store.is_none() && !dry_run;

    if dry_run {
        println!(
            "  {} Dry-run mode — no changes will be made\n",
            style("⊘").dim()
        );
    }

    // PRD-058 FR-001 + S1-R2: the canonical Markdown is written FIRST for
    // every imported document, and the legacy projection is taken afterwards
    // only where one still exists. No source file is ever moved or deleted.

    let result = match (&plan, &lock, &allocation) {
        (Some(plan), Some(lock), Some(allocation)) => {
            forgeplan_core::scan::import::execute_scan_import_plan(
                project_root,
                &ws,
                lock,
                allocation,
                store.as_ref(),
                &options,
                plan,
            )
            .await
        }
        _ => {
            scan_and_import_to_workspace(
                project_root,
                &ws,
                None,
                read_guard.as_ref().map(|guard| {
                    guard as &dyn forgeplan_core::workspace::lock::WorkspaceReadPermit
                }),
                None,
                &options,
            )
            .await
        }
    }
    .map_err(|error| {
        common::describe_possible_legacy_partial_commit(!dry_run && !migrated, error)
    })?;
    drop(allocation);
    drop(lock);

    // ONE reconcile for the whole successful batch, after the lock is
    // released — the reconcile re-acquires it per storage commit, so holding
    // it here would turn the pass into a lock timeout. Only a migrated
    // workspace needs it: on a legacy layout the batch just took its rows.
    if !dry_run && migrated && result.imported > 0 {
        if result.failed > 0 {
            // A partial batch is not a complete one: say so instead of
            // printing a clean "reconciled" line over a half-finished import.
            eprintln!(
                "  note: {} document(s) failed — the reconcile below publishes only the \
                 {} that landed; re-run after fixing them.",
                result.failed, result.imported
            );
        }
        common::reconcile_after_file_mutation(&ws).await;
    } else if !dry_run && !migrated {
        drop(store);
        if let Err(error) = common::drain_legacy_semantic_after_mutation(&ws).await {
            let failures: Vec<_> = result
                .entries
                .iter()
                .filter_map(|entry| match &entry.status {
                    ImportStatus::Failed(message) => {
                        Some(format!("{}: {message}", entry.relative_path))
                    }
                    _ => None,
                })
                .collect();
            if !failures.is_empty() {
                anyhow::bail!(
                    "scan import errors: {}; canonical state may be committed; semantic backlog pending: {error}",
                    failures.join("; ")
                );
            }
            return Err(error);
        }
    }

    // Print results
    if result.entries.is_empty() {
        println!("  No markdown documents found.");
        // PRD-071: empty scan — direct user to create artifacts.
        let next_hints: Vec<Hint> = vec![
            Hint::info("No documents to import")
                .with_action("forgeplan new prd \"<title>\"".to_string()),
        ];
        print!("{}", hints::render_next_action_line(&next_hints));
        return Ok(());
    }

    println!(
        "  Found {} document(s):\n",
        style(result.total_found).bold()
    );

    for entry in &result.entries {
        let status_icon = match &entry.status {
            ImportStatus::Imported => style("+").green().to_string(),
            ImportStatus::Skipped => style("~").yellow().to_string(),
            ImportStatus::Unknown => style("?").dim().to_string(),
            ImportStatus::Failed(_) => style("✗").red().to_string(),
        };

        let kind_str = entry
            .detected_kind
            .as_ref()
            .map(|k| k.template_key().to_uppercase().to_string())
            .unwrap_or_else(|| "???".to_string());

        let tier_str = entry
            .detection_tier
            .as_ref()
            .map(|t| match t {
                DetectionTier::Frontmatter => "fm",
                DetectionTier::Filename => "fn",
                DetectionTier::Content => "ct",
            })
            .unwrap_or("-");

        let id_str = entry.artifact_id.as_deref().unwrap_or("-");

        let status_note = match &entry.status {
            ImportStatus::Skipped => " (exists)".to_string(),
            ImportStatus::Failed(msg) => format!(" ({})", msg),
            _ => String::new(),
        };

        println!(
            "  {} {:5} [{:2}] {:10} {}{}",
            status_icon,
            kind_str,
            tier_str,
            id_str,
            entry.relative_path,
            style(status_note).dim()
        );

        // R2 audit rust-pro HIGH: surface per-entry warnings (unknown
        // status mapping, projection write failure). PRD-058 R-2
        // fail-loud: the core emits; CLI must display.
        for w in &entry.warnings {
            println!("    {} {}", style("⚠").yellow(), style(w).yellow().dim());
        }
    }

    // Aggregate warning count for the summary line.
    let warnings_total: usize = result.entries.iter().map(|e| e.warnings.len()).sum();

    println!();
    println!(
        "  Summary: {} imported, {} skipped, {} unknown, {} failed{}",
        style(result.imported).green().bold(),
        style(result.skipped).yellow(),
        style(result.unknown).dim(),
        if result.failed > 0 {
            style(result.failed).red().bold().to_string()
        } else {
            style(result.failed).dim().to_string()
        },
        if warnings_total > 0 {
            format!(", {} warning(s)", style(warnings_total).yellow().bold())
        } else {
            String::new()
        }
    );
    if result.failed > 0 && !dry_run {
        anyhow::bail!(
            "scan import failed for {} document(s); canonical changes may have been committed",
            result.failed
        );
    }

    // A shared DISPLAY id is legal under ADR-030 / SPEC-009 V3: the canonical
    // key is the document's identity, and the display id is an alias that may
    // repeat inside a workspace. Two documents that answer to one alias are
    // therefore NOT a stale duplicate to be deleted — the resolver refuses the
    // alias (exit 5) and each document is addressed by its own canonical key.
    //
    // The report names the canonical key of every file in the group, so the
    // operator can address each document exactly. It never tells them to
    // remove a document: under this model two files that share an alias are
    // two documents, and deleting one loses data.
    if !result.duplicate_ids.is_empty() {
        let scan_root = match path {
            Some(custom) => project_root.join(custom),
            None => project_root.to_path_buf(),
        };
        println!();
        println!(
            "  {} {} shared display id(s) — several documents answer to one artifact id:",
            style("⚠").yellow().bold(),
            style(result.duplicate_ids.len()).yellow().bold()
        );
        for grp in &result.duplicate_ids {
            println!("    {}", style(&grp.id).red().bold());
            for source in &grp.paths {
                match source_canonical_key(&scan_root, source, &grp.id) {
                    Some(key) => {
                        println!(
                            "      {} {} → {}",
                            style("·").dim(),
                            source,
                            style(&key).bold()
                        )
                    }
                    None => println!(
                        "      {} {} → {}",
                        style("·").dim(),
                        source,
                        style("key not readable").yellow()
                    ),
                }
            }
        }
        println!(
            "    {} a display id may be shared: address each document by its canonical key — \
             the shared id alone names none of them.",
            style("Note:").yellow().bold()
        );
    }

    if dry_run && result.imported > 0 {
        println!("\n  Run without {} to import.", style("--dry-run").cyan());
    }

    // PRD-071 contract: emit primary next-action.
    // - dry-run with imports → re-run without --dry-run
    // - imports happened → run health to surface integrity issues
    // - only skipped/failed → reindex to refresh DB state
    let next_hints: Vec<Hint> = if dry_run && result.imported > 0 {
        let cmd = match path {
            Some(p) => format!("forgeplan scan-import --path {}", p),
            None => "forgeplan scan-import".to_string(),
        };
        vec![
            Hint::info(format!("{} document(s) ready to import", result.imported)).with_action(cmd),
        ]
    } else if result.imported > 0 {
        vec![
            Hint::info("Import complete — verify integrity")
                .with_action("forgeplan health".to_string()),
        ]
    } else {
        vec![
            Hint::info("Nothing imported — refresh index")
                .with_action("forgeplan reindex".to_string()),
        ]
    };
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, name: &str, body: &str) {
        std::fs::write(root.join(name), body).unwrap();
    }

    #[test]
    fn a_declared_slug_that_is_a_canonical_key_is_the_documents_key() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "alpha.md",
            "---\nid: PRD-001\nslug: prd-alpha\nkind: prd\n---\n\nBody\n",
        );
        assert_eq!(
            source_canonical_key(dir.path(), "alpha.md", "PRD-001").as_deref(),
            Some("prd-alpha"),
            "the declared slug is the key"
        );
    }

    #[test]
    fn a_document_without_a_slug_keeps_its_display_id_as_its_key() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "plain.md",
            "---\nid: PRD-001\nkind: prd\n---\n\nBody\n",
        );
        assert_eq!(
            source_canonical_key(dir.path(), "plain.md", "PRD-001").as_deref(),
            Some("PRD-001"),
            "a document that declares no slug keeps its display id"
        );
    }

    #[test]
    fn a_slug_that_is_not_a_canonical_key_is_not_an_identity() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "invalid.md",
            "---\nid: PRD-001\nslug: Not A Slug\nkind: prd\n---\n\nBody\n",
        );
        assert_eq!(
            source_canonical_key(dir.path(), "invalid.md", "PRD-001").as_deref(),
            Some("PRD-001"),
            "an invalid slug is not an identity — the display id stands"
        );
    }

    #[test]
    fn an_unreadable_file_yields_no_key_rather_than_a_guess() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            source_canonical_key(dir.path(), "missing.md", "PRD-001"),
            None,
            "a file that cannot be read must not be given a guessed key"
        );
    }
}
