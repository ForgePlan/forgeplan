use forgeplan_core::artifact::canonical_key::ArtifactCandidate;
use forgeplan_core::hints;
use forgeplan_core::link;
use forgeplan_core::workspace::lock::WorkspaceLock;
use forgeplan_mcp::convert::{Endpoint, Resolution, resolve_endpoint};

use crate::commands::common;


/// Resolve one endpoint of an edge against a document set.
///
/// The SOURCE of an edge must name exactly one document: the edge is
/// written into one document's `links:` block, and "the first document that
/// matched" is the silent pick the shared resolver exists to prevent.
///
/// The TARGET may name no document yet — a forward reference is a legal
/// edge — but it may NOT be ambiguous: two documents answering one name is
/// an error, not a choice.
///
/// A qualified reference (`KEY@qualifier`) names a document in ANOTHER
/// workspace: it is not a local selector, so the local resolver must not
/// judge it. It stays verbatim, exactly as a forward reference does.
fn resolve_edge_endpoint(
    reference: &str,
    documents: &[ArtifactCandidate],
) -> Result<Endpoint, Resolution> {
    if reference.contains('@') {
        return Ok(Endpoint::Forward(reference.to_string()));
    }
    resolve_endpoint(reference, documents)
}

/// Every name one endpoint answers to.
///
/// A document answers under its display id AND under its canonical key —
/// the same document, two names — and an edge may be written down under
/// either. A forward reference answers under the text it was declared with.
fn endpoint_names(endpoint: &Endpoint, documents: &[ArtifactCandidate]) -> Vec<String> {
    match endpoint {
        Endpoint::Document { canonical_key, display_id } => {
            let mut names = vec![canonical_key.clone()];
            if matches!(
                resolve_edge_endpoint(display_id, documents),
                Ok(Endpoint::Document { canonical_key: ref key, .. }) if key == canonical_key
            ) && display_id != canonical_key {
                names.push(display_id.clone());
            }
            names
        }
        Endpoint::Forward(declared) => vec![declared.clone()],
    }
}

/// The name one endpoint is addressed by when it is written down.
///
/// A document is addressed by its key and a forward reference by its declaration.
fn endpoint_label(endpoint: &Endpoint) -> String {
    match endpoint {
        Endpoint::Document { canonical_key, .. } => canonical_key.clone(),
        Endpoint::Forward(declared) => declared.clone(),
    }
}

pub async fn run(
    source_id: &str,
    target_id: &str,
    relation: &str,
    replace: bool,
    auto_activate_complete: bool,
) -> anyhow::Result<()> {
    // Normalize relation
    let relation = link::normalize_relation(relation)?;

    let (ws, _lock, store) = common::open_store_locked_checked(&[(source_id, true), (target_id, false)]).await?;
    let Some(store) = store else {
        return run_file_first(
            ws,
            _lock,
            source_id,
            target_id,
            &relation,
            replace,
            auto_activate_complete,
        )
        .await;
    };

    // PROB-060 / SPEC-005 Phase 1.5b — accept slug or display id for both
    // ends.
    //
    // The SOURCE must name exactly one document: the edge is written into
    // one document's `links:` block, and "the first document that matched"
    // is the silent pick the shared resolver exists to prevent.
    //
    // The TARGET may name no document yet — a cross-PR forward reference is
    // a legal edge — but it may NOT be ambiguous: two documents answering
    // one name is an error, not a choice.
    //
    let documents = forgeplan_core::artifact::documents::workspace_candidates(&ws).await?;

    let resolution = common::resolve_against_workspace(&ws, source_id).await?;
    let (source_id_owned, source_display) = common::require_found_identity(source_id, &resolution)?;
    let source_id = source_id_owned.as_str();
    let target_display = match resolve_edge_endpoint(target_id, &documents) {
        Ok(Endpoint::Document { display_id, .. }) => display_id,
        _ => target_id.to_string(),
    };

    // A qualified `KEY@qualifier` target is foreign by definition — "not in
    // this workspace" is the contract, not a warning — so instead we check
    // the qualifier against the registered constellation sources and warn
    // when it names none (a typo there reads as a deliberate forward
    // reference otherwise).
    //
    // Warn-only either way: forward refs stay legal. Registry problems
    // never block the write — a missing registry means "no constellation".
    // The `is_ok` guard keeps a *malformed* qualified ref on the plain
    // local-miss path: its validation error downstream is the real answer.
    let target_id_owned: String =
        match forgeplan_core::constellation::federation::parse_qualified_ref(target_id) {
            (_, Some(qualifier))
                if forgeplan_core::db::store::validate_link_target(target_id).is_ok() =>
            {
                match crate::commands::constellation::load_registry().map(|(_, registry)| registry) {
                    Ok(registry) => {
                        match forgeplan_core::constellation::registry::current_sources(&registry) {
                            Some(specs) => {
                                if forgeplan_core::constellation::federation::specs_matching(
                                    &specs, qualifier,
                                )
                                .is_empty()
                                {
                                    eprintln!(
                                        "Warning: qualifier @{qualifier} matches no registered source \
                                         (creating link anyway — forward references are legal)\n\
                                         Fix: forgeplan constellation sources"
                                    );
                                }
                            }
                            None => {
                                eprintln!(
                                    "Warning: no active constellation to check qualifier @{qualifier} \
                                     against (creating link anyway)\n\
                                     Fix: forgeplan constellation list"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: could not check qualifier @{qualifier} against the \
                             constellation registry ({e}) — creating link anyway"
                        );
                    }
                }
                target_id.to_string()
            }
            _ => match resolve_edge_endpoint(target_id, &documents) {
                Ok(Endpoint::Document { canonical_key, .. }) => canonical_key,
                // A miss is a legal forward reference.
                Ok(Endpoint::Forward(declared)) => {
                    eprintln!(
                        "Warning: Target artifact '{declared}' not found in workspace (creating link anyway)"
                    );
                    declared
                }
                Err(outcome) => {
                    return Err(common::refuse_resolution(target_id, &outcome, &documents));
                }
            },
        };
    let target_id = target_id_owned.as_str();

    // Issue #286: `--replace` selects the upsert helper, which collapses
    // any pre-existing edge between (source, target) onto the requested
    // relation. Without the flag we use the strict additive helper that
    // rejects duplicates and parallel-different-relation edges.
    common::ensure_ordinary_mutation_allowed(&ws)?;
    let ctx = forgeplan_core::projection::MutationContext::new(&ws, &_lock, Some(&store));
    let outcome = if replace {
        Some(
            forgeplan_core::projection::replace_link_with_projection(
                &ctx, source_id, target_id, &relation,
            )
            .await.map_err(|e| common::describe_possible_legacy_partial_commit(true, e.into()))?,
        )
    } else {
        forgeplan_core::projection::add_link_with_projection(&ctx, source_id, target_id, &relation)
            .await.map_err(|e| common::describe_possible_legacy_partial_commit(true, e.into()))?;
        None
    };

    common::log_change_field(&ws, &_lock, Some(&store), source_id,
    "link",
    (
        "relation",
        None,
        Some(&format!("{}:{}", target_id, relation)),
    ),
    "cli",)
    .await;


    // PRD-075 FR-001: sync recompute closes stale-cache window. Failure is
    // non-fatal (mutation succeeded) — the wrapper surfaces a Fix: marker so
    // agents/operators see the score-all recovery path.
    common::sync_score_target_or_warn(&store, &_lock, source_id).await;

    // Issue #288 Part A — CLI parity for auto-activate. Same predicate as
    // the MCP `forgeplan_link` tool. Failure is non-fatal: the link itself
    // already committed; we warn and let the user retry forgeplan_activate
    // manually.
    let mut auto_activated = false;
    if auto_activate_complete
        && let Ok(Some(src_record)) = store.get_record(&_lock, source_id).await
        && src_record.kind.eq_ignore_ascii_case("evidence")
        && src_record.status.eq_ignore_ascii_case("draft")
        && forgeplan_core::scoring::evidence::is_evidence_complete(&src_record.body)
    {
        match forgeplan_core::lifecycle::activate(&_lock, &store, source_id, false).await {
            Ok(_) => {
                if let Err(e) =
                    forgeplan_core::projection::render_after_mutation(&_lock, &ws, &store, source_id).await
                {
                    eprintln!("Warning: auto-activate post-render for {source_id} failed: {e}");
                }
                auto_activated = true;
            }
            Err(e) => {
                eprintln!(
                    "Warning: auto-activate of {source_id} failed: {e}\nFix: forgeplan activate {source_id}"
                );
            }
        }
    }
    drop(store);
    drop(_lock);
    common::drain_legacy_semantic_after_mutation(&ws).await?;

    // Surface the outcome verbatim when `--replace` was used so operators
    // can see whether the call was a no-op, a swap, or a creation. Without
    // the flag we keep the original one-liner for back-compat.
    match outcome {
        Some(forgeplan_core::projection::LinkUpsertOutcome::Replaced { old_relation }) => {
            println!(
                "Replaced: {} --{}--> {} (was: --{}-->)",
                source_display, relation, target_display, old_relation
            );
        }
        Some(forgeplan_core::projection::LinkUpsertOutcome::Unchanged) => {
            println!(
                "Unchanged: {} --{}--> {} already exists",
                source_display, relation, target_display
            );
        }
        Some(forgeplan_core::projection::LinkUpsertOutcome::Created) | None => {
            println!("Linked: {} --{}--> {}", source_display, relation, target_display);
        }
    }
    if auto_activated {
        println!("Auto-activated: {source_display} (draft → active)");
    }


    // PRD-075 FR-009: hint string lives in core (forgeplan_core::hints) so
    // mutator call sites cannot drift independently.
    let hints_vec = vec![hints::reconcile_parents_hint()];
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}

/// File-first `link` for constellation-migrated workspaces: no `LanceStore`
/// exists, so the edge is written into the source file's `links:` frontmatter
/// and one reconcile pass publishes it to `constellation_*_v1`.
/// Markdown remains authoritative and the derived index is reconciled after
/// mutations. `--auto-activate-complete` is handled against the captured
/// canonical workspace.
/// Audit events use auxiliary history; scores are computed from canonical
/// links when requested rather than stored in a cache.
///
/// Semantics preserved from the store path: slug/display-id source
/// resolution (over the captured document set), the qualified
/// `KEY@qualifier` registry check, forward-reference warning, `--replace`
/// upsert outcomes, verbatim target storage, normalized relations.
async fn run_file_first(
    ws: std::path::PathBuf,
    lock: WorkspaceLock,
    source_id: &str,
    target_id: &str,
    relation: &str,
    replace: bool,
    auto_activate_complete: bool,
) -> anyhow::Result<()> {
    // The document set of this layout: the captured workspace read. The
    // capture knows every document's canonical key, display id, title and
    // workspace-relative path — so the source FILE is the candidate's own
    // path, and the identity decision is the shared resolver's.
    //
    // Boundary discipline mirrors the store path: a malformed source just
    // fails resolution below (same "not found" refusal), and the target is
    // validated inside `add_link_verbatim` / `replace_link` — the file-first
    // helpers apply `validate_link_target` exactly where the projection
    // helpers do.
    let read = forgeplan_core::constellation::facade::open_workspace_read_with_permit(&ws, &lock).await?;
    let resolution = common::resolve_against_capture(&read, source_id);
    let (_, source_display) = common::require_found_identity(source_id, &resolution)?;
    let documents = forgeplan_core::artifact::documents::capture_candidates(&read);
    let target_display = match resolve_edge_endpoint(target_id, &documents) {
        Ok(Endpoint::Document { display_id, .. }) => display_id,
        _ => target_id.to_string(),
    };

    let source_endpoint = match resolve_edge_endpoint(source_id, &documents) {
        Ok(endpoint) => endpoint,
        // An ambiguity, a contested canonical key, an unusable selector —
        // refusals, and none of them is "not found".
        Err(outcome) => return Err(common::refuse_resolution(source_id, &outcome, &documents)),
    };
    let (source_id_owned, source_path) = match &source_endpoint {
        Endpoint::Document {
            canonical_key,
            display_id: _,
        } => {
            // The document's own file: the capture's workspace-relative
            // path, joined onto the workspace.
            let relative = documents
                .iter()
                .find(|document| document.canonical_key == *canonical_key)
                .map(|document| document.relative_path.clone())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Source artifact '{source_id}' not found\nFix: forgeplan list"
                    )
                })?;
            (canonical_key.clone(), ws.join(relative))
        }
        Endpoint::Forward(_) => anyhow::bail!(
            "Source artifact '{source_id}' not found\nFix: forgeplan list"
        ),
    };
    let source_id = source_id_owned.as_str();

    // Target: qualified refs go through the same registry check as the
    // store path; local targets resolve to the one document they name, and
    // a miss stays a legal forward reference with the same warning.
    let target_id_owned: String;
    match forgeplan_core::constellation::federation::parse_qualified_ref(target_id) {
        // Same `is_ok` guard as the store path: a *malformed* qualified ref
        // drops to the plain local-miss arm, where the core helper's
        // `validate_link_target` produces the real answer downstream.
        (_, Some(qualifier))
            if forgeplan_core::db::store::validate_link_target(target_id).is_ok() =>
        {
            match crate::commands::constellation::load_registry().map(|(_, registry)| registry) {
                Ok(registry) => {
                    match forgeplan_core::constellation::registry::current_sources(&registry) {
                        Some(specs) => {
                            if forgeplan_core::constellation::federation::specs_matching(
                                &specs, qualifier,
                            )
                            .is_empty()
                            {
                                eprintln!(
                                    "Warning: qualifier @{qualifier} matches no registered source \
                                     (creating link anyway — forward references are legal)\n\
                                     Fix: forgeplan constellation sources"
                                );
                            }
                        }
                        None => {
                            eprintln!(
                                "Warning: no active constellation to check qualifier @{qualifier} \
                                 against (creating link anyway)\n\
                                 Fix: forgeplan constellation list"
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: could not check qualifier @{qualifier} against the \
                         constellation registry ({e}) — creating link anyway"
                    );
                }
            }
            target_id_owned = target_id.to_string();
        }
        _ => match resolve_edge_endpoint(target_id, &documents) {
            Ok(Endpoint::Document { canonical_key, .. }) => {
                target_id_owned = canonical_key;
            }
            // A miss is a legal forward reference.
            Ok(Endpoint::Forward(declared)) => {
                eprintln!(
                    "Warning: Target artifact '{declared}' not found in workspace (creating link anyway)"
                );
                target_id_owned = declared;
            }
            Err(outcome) => {
                return Err(common::refuse_resolution(target_id, &outcome, &documents));
            }
        },
    }
    let target_id = target_id_owned.as_str();

    common::ensure_ordinary_mutation_allowed(&ws)?;
    let outcome = if replace {
        Some(link::replace_link(&source_path, target_id, relation).await?)
    } else {
        link::add_link_verbatim(&source_path, target_id, relation).await?;
        None
    };

    common::log_change_field(&ws, &lock, None, source_id,
    "link",
    (
        "relation",
        None,
        Some(&format!("{}:{}", target_id, relation)),
    ),
    "cli",)
    .await;

    match &outcome {
        Some(forgeplan_core::projection::LinkUpsertOutcome::Replaced { old_relation }) => {
            println!(
                "Replaced: {} --{}--> {} (was: --{}-->)",
                source_display, relation, target_display, old_relation
            );
        }
        Some(forgeplan_core::projection::LinkUpsertOutcome::Unchanged) => {
            println!(
                "Unchanged: {} --{}--> {} already exists",
                source_display, relation, target_display
            );
        }
        Some(forgeplan_core::projection::LinkUpsertOutcome::Created) | None => {
            println!("Linked: {} --{}--> {}", source_display, relation, target_display);
        }
    }

    common::note_constellation_owns_index();
    let link_changed = outcome != Some(forgeplan_core::projection::LinkUpsertOutcome::Unchanged);
    let mut auto_activated = false;
    if auto_activate_complete {
        match forgeplan_core::constellation::facade::open_workspace_read_with_permit(&ws, &lock).await {
            Ok(read) => {
                if let Some(src_record) = read.get_record(source_id)
                    && src_record.kind.eq_ignore_ascii_case("evidence")
                    && src_record.status.eq_ignore_ascii_case("draft")
                    && forgeplan_core::scoring::evidence::is_evidence_complete(&src_record.body)
                {
                    let activation = {
                        let ctx =
                            forgeplan_core::projection::MutationContext::new(&ws, &lock, None);
                        forgeplan_core::lifecycle::canonical::activate_captured(
                            &ctx, &read, source_id, false,
                        )
                        .await
                    };
                    match activation {
                        Ok(_) => {
                            auto_activated = true;
                            println!("Auto-activated: {source_display} (draft → active)");
                        }
                        Err(e) => eprintln!(
                            "Warning: auto-activate of {source_id} failed: {e}\n\
                             Fix: forgeplan activate {source_id}"
                        ),
                    }
                }
            }
            Err(e) => eprintln!(
                "Warning: link succeeded, but auto-activation could not read the canonical \
                 workspace: {e}\nFix: forgeplan activate {source_id}"
            ),
        }
    }
    // Release the workspace lock BEFORE reconcile — the pass re-acquires it
    // per storage commit and would deadlock on a held guard (30 s timeout).
    drop(lock);
    if link_changed || auto_activated {
        common::reconcile_after_file_mutation(&ws).await;
    }

    let hints_vec = vec![hints::reconcile_parents_hint()];
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}

pub async fn run_unlink(source_id: &str, target_id: &str, relation: &str) -> anyhow::Result<()> {
    let relation = link::normalize_relation(relation)?;
    let (ws, _lock, store) = common::open_store_locked_checked(&[(source_id, false), (target_id, false)]).await?;
    let Some(store) = store else {
        return run_unlink_file_first(ws, _lock, source_id, target_id, &relation).await;
    };

    let documents = forgeplan_core::artifact::documents::workspace_candidates(&ws).await?;

    let source_endpoint = match resolve_edge_endpoint(source_id, &documents) {
        Ok(endpoint) => endpoint,
        Err(outcome) => return Err(common::refuse_resolution(source_id, &outcome, &documents)),
    };
    let target_endpoint = match resolve_edge_endpoint(target_id, &documents) {
        Ok(endpoint) => endpoint,
        Err(outcome) => return Err(common::refuse_resolution(target_id, &outcome, &documents)),
    };
    // The edge is stored under the canonical key of the document it names,
    // so the lookup accepts every name that document answers to — and the
    // text the operator typed, which is how a forward reference is stored.
    let source_names = endpoint_names(&source_endpoint, &documents);
    let target_names = endpoint_names(&target_endpoint, &documents);

    // Check relation exists before deleting.
    // Use get_all_relations for resilient lookup (works even if source artifact is deleted).
    let all_rels = store.get_all_relations(&_lock).await?;
    let found = all_rels.iter().find(|(s, t, r)| {
        source_names.iter().any(|name| s.eq_ignore_ascii_case(name))
            && target_names.iter().any(|name| t.eq_ignore_ascii_case(name))
            && r == &relation
    });
    let Some((stored_source, stored_target, _)) = found else {
        anyhow::bail!(
            "Relation '{}' from {} to {} not found",
            relation,
            source_id,
            target_id
        );
    };

    // PRD-073 FR-005: update the source's outgoing canonical links.
    common::ensure_ordinary_mutation_allowed(&ws)?;
    forgeplan_core::projection::delete_link_with_projection(
        &forgeplan_core::projection::MutationContext::new(&ws, &_lock, Some(&store)),
        stored_source,
        stored_target,
        &relation,
    )
    .await.map_err(|e| common::describe_possible_legacy_partial_commit(true, e.into()))?;

    common::log_change_field(&ws, &_lock, Some(&store), source_id,
    "unlink",
    (
        "relation",
        Some(&format!("{}:{}", target_id, relation)),
        None,
    ),
    "cli",)
    .await;


    // PRD-075 FR-002: sync recompute after unlink mirrors the link path.
    common::sync_score_target_or_warn(&store, &_lock, &endpoint_label(&source_endpoint)).await;
    drop(store);
    drop(_lock);
    common::drain_legacy_semantic_after_mutation(&ws).await?;

    println!("Unlinked: {} --{}--> {}", match &source_endpoint {
        Endpoint::Document { display_id, .. } => display_id.as_str(),
        Endpoint::Forward(value) => value.as_str(),
    }, relation, match &target_endpoint {
        Endpoint::Document { display_id, .. } => display_id.as_str(),
        Endpoint::Forward(value) => value.as_str(),
    });

    let hints_vec = vec![hints::reconcile_parents_hint()];
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}

/// File-first `unlink` for constellation-migrated workspaces: the edge is
/// dropped from the source file's `links:` frontmatter and one reconcile
/// pass removes it from `constellation_*_v1`. Mirrors `run_file_first` —
/// same resolution, same "relation not found" refusal, same reconcile.
async fn run_unlink_file_first(
    ws: std::path::PathBuf,
    lock: WorkspaceLock,
    source_id: &str,
    target_id: &str,
    relation: &str,
) -> anyhow::Result<()> {
    // The document set of this layout: the captured workspace read. The
    // shared resolver decides which document each end names — an ambiguous
    // reference is a refusal, never a pick.
    let read = forgeplan_core::constellation::facade::open_workspace_read_with_permit(&ws, &lock).await?;
    let resolution = common::resolve_against_capture(&read, source_id);
    if !matches!(&resolution.outcome, Resolution::NotFound | Resolution::Found(_)) {
        return Err(common::refuse_resolution(source_id, &resolution.outcome, &resolution.candidates));
    }
    let documents = forgeplan_core::artifact::documents::capture_candidates(&read);

    let source_endpoint = match resolve_edge_endpoint(source_id, &documents) {
        Ok(endpoint) => endpoint,
        Err(outcome) => return Err(common::refuse_resolution(source_id, &outcome, &documents)),
    };
    let target_endpoint = match resolve_edge_endpoint(target_id, &documents) {
        Ok(endpoint) => endpoint,
        Err(outcome) => return Err(common::refuse_resolution(target_id, &outcome, &documents)),
    };

    // No validators here on purpose: the store path runs none either — a
    // malformed ref simply fails the edge lookup and lands on the same
    // "relation not found" refusal below.
    //
    // The store path reports "relation not found" for ANY miss — including
    // a source that doesn't exist — because it matches the edge in
    // `get_all_relations` first. Keep the same refusal here.
    let source_path = match &source_endpoint {
        Endpoint::Document { canonical_key, .. } => documents
            .iter()
            .find(|document| document.canonical_key == *canonical_key)
            .map(|document| ws.join(&document.relative_path)),
        Endpoint::Forward(_) => None,
    };
    let Some(source_path) = source_path else {
        anyhow::bail!(
            "Relation '{}' from {} to {} not found",
            relation,
            source_id,
            target_id
        );
    };

    // The edge is written down under one of the names the target document
    // answers to — its display id or its canonical key — so both are tried.
    common::ensure_ordinary_mutation_allowed(&ws)?;
    let mut removed = 0;
    for name in endpoint_names(&target_endpoint, &documents) {
        removed = link::remove_link(&source_path, &name, relation).await?;
        if removed > 0 {
            break;
        }
    }
    if removed == 0 {
        anyhow::bail!(
            "Relation '{}' from {} to {} not found",
            relation,
            source_id,
            target_id
        );
    }

    common::log_change_field(&ws, &lock, None, source_id,
    "unlink",
    (
        "relation",
        Some(&format!("{}:{}", target_id, relation)),
        None,
    ),
    "cli",)
    .await;

    println!("Unlinked: {} --{}--> {}", match &source_endpoint {
        Endpoint::Document { display_id, .. } => display_id.as_str(),
        Endpoint::Forward(value) => value.as_str(),
    }, relation, match &target_endpoint {
        Endpoint::Document { display_id, .. } => display_id.as_str(),
        Endpoint::Forward(value) => value.as_str(),
    });

    common::note_constellation_owns_index();
    // Lock released before reconcile — see run_file_first.
    drop(lock);
    common::reconcile_after_file_mutation(&ws).await;

    let hints_vec = vec![hints::reconcile_parents_hint()];
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}
