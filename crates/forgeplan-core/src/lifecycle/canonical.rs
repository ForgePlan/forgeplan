//! Lifecycle transitions over the CANONICAL source (Constellation).
//!
//! # Why this module exists
//!
//! The store-backed [`super`] functions read their relation graph, their
//! evidence links and their dependency list out of `artifacts.lance` /
//! `relations.lance`. The Constellation protocol RETIRES those tables: on a
//! migrated workspace the derived index is `constellation_*_v1`, published by
//! `maintain_index` from the canonical Markdown. A lifecycle command that
//! needs those edges therefore has exactly two honest options — compute them
//! from the same capture reconcile publishes, or refuse. Opening the retired
//! tables to get them would recreate a dataset that shadows the real index,
//! and gating a transition on *nothing* would let an artifact activate because
//! its graph happened to be unreadable.
//!
//! So: this module computes them from a captured
//! [`WorkspaceRead`](crate::constellation::workspace_read::WorkspaceRead) and
//! writes them through the [`MutationContext`] file-first helpers.
//!
//! # What is NOT re-implemented here
//!
//! * **The rules.** Transition validity, gate thresholds, the stub check, the
//!   evidence-link predicate and the MUST/Should split all live in [`super`].
//!   This module only supplies *where the edges come from*. A second copy of
//!   the gate rules is exactly how `review` and `activate` drift apart.
//! * **The write discipline.** Status flips go through
//!   [`projection::update_metadata_with_projection`], body appends through
//!   [`projection::update_body_with_projection`], new edges through
//!   [`projection::add_link_with_projection`], and `valid_until` through
//!   [`projection::update_valid_until_with_projection`] — the same helpers the
//!   legacy path uses, so ADR-003's "Markdown first" order and the
//!   `carry_identity_forward` identity-preservation rule hold identically on
//!   both layouts.
//!
//! # Read-after-write
//!
//! Every function takes the `WorkspaceRead` the CALLER captured. On a migrated
//! workspace that capture is the canonical state at the moment the command
//! decided what to do; nothing here re-opens a store to recount what it just
//! wrote. A caller that needs the post-mutation answer captures a fresh view
//! (`link`/`new` already do this for their auto-activate check).

use crate::artifact::frontmatter::refs_form_from_body;
use crate::artifact::types::Mode;
use crate::db::store::NewArtifact;
use crate::projection::{self, MutationContext};
use crate::validation::{self, Severity};

use super::{
    ActivateResult, GatesReport, RenewResult, ReopenResult, ReviewResult, SupersedeResult,
    transitions,
};
use crate::constellation::workspace_read::WorkspaceRead;

/// Resolve `artifact_id` (display id, normalized `KIND-N`, or ADR-012 slug)
/// against this view, or the documented not-found error.
fn resolve<'a>(
    read: &'a WorkspaceRead,
    artifact_id: &str,
) -> anyhow::Result<&'a crate::db::store::ArtifactRecord> {
    let canonical = read
        .resolve_id(artifact_id)
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {artifact_id}"))?;
    read.get_record(&canonical)
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {artifact_id}"))
}

/// Captured records carry the full Markdown file; the body writer expects only
/// the text below the projection frontmatter.
fn post_frontmatter_body(record: &crate::db::store::ArtifactRecord) -> anyhow::Result<String> {
    crate::artifact::frontmatter::parse_frontmatter(&record.body).map(|(_, body)| body)
}

/// MUST / Should findings from `validation::validate`, in the shape both
/// `review` and `activate` render them.
fn split_findings(
    artifact_id: &str,
    record: &crate::db::store::ArtifactRecord,
    fm: &crate::artifact::frontmatter::Frontmatter,
) -> anyhow::Result<(Vec<String>, Vec<String>)> {
    let kind = record.kind.parse()?;
    let depth = record.depth.parse().unwrap_or(Mode::Standard);
    let result = validation::validate(artifact_id, &record.body, fm, &kind, &depth);
    let must = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Must)
        .map(|f| format!("{}: {}", f.rule_id, f.message))
        .collect();
    let should = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Should)
        .map(|f| format!("{}: {}", f.rule_id, f.message))
        .collect();
    Ok((must, should))
}

/// Run the activation gates against edges from this capture.
///
/// The single-source-of-truth guarantee the store path relies on: both call
/// [`super::activation_gates_from`], so a gate cannot fire on one workspace
/// layout and be silent on the other.
pub fn collect_activation_gates_captured(
    read: &WorkspaceRead,
    record: &crate::db::store::ArtifactRecord,
) -> GatesReport {
    super::activation_gates_from(
        record,
        &read.get_relations(&record.id),
        &read.get_incoming_relations(&record.id),
    )
}

/// `forgeplan review` over the canonical source.
///
/// Build-on-draft warnings resolve their TARGET artifacts from the same
/// capture: re-opening a store to answer "is my dependency still draft?" would
/// resurrect a retired table to satisfy a warning, and would answer about a
/// different snapshot than the one the gates just evaluated.
pub fn review_captured(read: &WorkspaceRead, artifact_id: &str) -> anyhow::Result<ReviewResult> {
    let record = resolve(read, artifact_id)?;
    let fm = read.frontmatter_map_with_links(record);
    let (must_findings, should_findings) = split_findings(&record.id, record, &fm)?;

    let mut warnings = Vec::new();
    for (target_id, _relation) in read.get_relations(&record.id) {
        if let Some(target) = read.get_record(&target_id)
            && target.status == "draft"
        {
            warnings.push(format!(
                "build-on-draft: depends on {} which is still Draft",
                target_id
            ));
        }
    }

    let gates = collect_activation_gates_captured(read, record);
    warnings.extend(gates.errors().into_iter().map(str::to_string));

    Ok(ReviewResult {
        artifact_id: record.id.clone(),
        can_activate: must_findings.is_empty() && gates.all_pass(),
        must_findings,
        should_findings,
        warnings,
    })
}

/// `forgeplan activate` over the canonical source: same transition check,
/// same hypothesis guard, same gates, same `--force` escape — then the status
/// flip as a canonical Markdown edit under the caller's lock.
///
/// The caller owns `ctx`, so this writes through the file-first helper whether
/// or not a legacy sink happens to be attached; on a migrated workspace the
/// helper's `None`-sink arm is what runs and reconcile publishes the row.
pub async fn activate_captured(
    ctx: &MutationContext<'_>,
    read: &WorkspaceRead,
    artifact_id: &str,
    force: bool,
) -> anyhow::Result<ActivateResult> {
    let record = resolve(read, artifact_id)?.clone();

    transitions::validate_transition(&record.status, "active")?;

    // Audit-r7 ARCH-C1: a HYP's `status` is DERIVED from `hypothesis_state`.
    // Flipping it here alone would desynchronise the two. Same refusal, same
    // redirect, as the store path.
    if record.kind.eq_ignore_ascii_case("hypothesis") {
        anyhow::bail!(
            "Hypothesis artifacts (kind=hypothesis) are activated via \
             `forgeplan_hypothesis_promote` (which advances hypothesis_state \
             and derives status atomically). Direct `forgeplan_activate` on \
             {artifact_id} would desynchronise the two fields. Use \
             `forgeplan_hypothesis_promote hypothesis_id={artifact_id} new_state=verified` \
             to activate (sets status=active) or `new_state=refuted` to deprecate."
        );
    }

    let gates = collect_activation_gates_captured(read, &record);
    if !gates.all_pass() && !force && super::supports_lifecycle(&record.kind) {
        anyhow::bail!("{}", activation_gate_message(artifact_id, &gates.errors()));
    }

    // MUST validation — skipped for lightweight kinds exactly as the store
    // path does (a Note has no MUST rules, so running them would be a no-op
    // with different messages).
    let mut must_errors = Vec::new();
    if super::supports_lifecycle(&record.kind) {
        let fm = read.frontmatter_map_with_links(&record);
        let (must, _should) = split_findings(&record.id, &record, &fm)?;
        if !must.is_empty() && !force {
            let mut msg = format!(
                "Validation failed ({} MUST error{}):",
                must.len(),
                if must.len() == 1 { "" } else { "s" }
            );
            for finding in &must {
                msg.push_str(&format!("\n  - {finding}"));
            }
            msg.push_str("\n\nUse --force to override.");
            anyhow::bail!("{msg}");
        }
        must_errors = must;
    }

    projection::update_metadata_with_projection(ctx, &record.id, Some("active"), None)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(ActivateResult {
        artifact_id: record.id.clone(),
        forced: force && !must_errors.is_empty(),
        must_errors,
    })
}

/// The refusal text a failed activation gate produces.
///
/// Shared by the store path and this one so `--force`'s remediation advice —
/// including the five-step "no evidence yet" recipe — cannot differ by
/// layout. Extracted rather than retyped; see [`super::activate`].
pub(super) fn activation_gate_message(artifact_id: &str, errs: &[&str]) -> String {
    let mut msg = format!("Cannot activate {artifact_id}: methodology gates failed:");
    for e in errs {
        msg.push_str(&format!("\n  - {e}"));
    }
    if errs.iter().any(|e| e.contains("No evidence linked")) {
        msg.push_str(&format!(
            "\n\nCanonical fix order (no evidence yet):\n  \
             1. forgeplan_new(kind=\"evidence\", title=\"<verification title>\")\n  \
             2. forgeplan_update(id=<EVID-ID>, body=\"<verdict + congruence_level + evidence_type>\")\n  \
             3. forgeplan_link(source=<EVID-ID>, target={artifact_id}, relation=\"informs\")\n  \
             4. forgeplan_activate(id=<EVID-ID>)\n  \
             5. forgeplan_activate(id={artifact_id})  # now succeeds\n\
             \n\
             Or use --force to bypass (not recommended — leaves R_eff=0)."
        ));
    } else {
        msg.push_str("\n\nUse --force to override (not recommended).");
    }
    msg
}

/// `forgeplan supersede` over the canonical source.
///
/// Dependents come from this capture's INCOMING edges, read before the
/// successor edge is written — the same snapshot the transition was validated
/// against, so the warning list cannot name a dependent that arrived after the
/// decision.
pub async fn supersede_captured(
    ctx: &MutationContext<'_>,
    read: &WorkspaceRead,
    artifact_id: &str,
    replacement_id: &str,
) -> anyhow::Result<SupersedeResult> {
    let record = resolve(read, artifact_id)?.clone();
    let replacement_id = read
        .resolve_id(replacement_id)
        .unwrap_or_else(|| replacement_id.to_string());
    let replacement = read
        .get_record(&replacement_id)
        .ok_or_else(|| anyhow::anyhow!("Replacement not found: {replacement_id}"))?
        .clone();

    transitions::validate_transition(&record.status, "superseded")?;

    let mut warnings = Vec::new();
    if replacement.status == "draft" {
        warnings.push(format!(
            "Replacement {} is still draft — activate before using",
            replacement_id
        ));
    }
    if replacement.status == "superseded" || replacement.status == "deprecated" {
        anyhow::bail!(
            "Replacement {} is already {}. Choose an active or draft artifact as replacement.",
            replacement_id,
            replacement.status
        );
    }

    let dependents: Vec<String> = read
        .get_incoming_relations(&record.id)
        .into_iter()
        .map(|(source, _)| source)
        .collect();

    projection::add_link_with_projection(ctx, &record.id, &replacement_id, "supersedes")
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    projection::update_metadata_with_projection(ctx, &record.id, Some("superseded"), None)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(SupersedeResult {
        dependents,
        warnings,
    })
}

/// `forgeplan deprecate` over the canonical source.
///
/// The reason is appended to the canonical body and the status flipped, both
/// as Markdown edits — the #478 contract that the `## Deprecation` section
/// actually reaches the file. On the store path that needed the
/// `render_after_mutation_with_body` variant; here the body write IS the file
/// write, so the section cannot be dropped.
pub async fn deprecate_captured(
    ctx: &MutationContext<'_>,
    read: &WorkspaceRead,
    artifact_id: &str,
    reason: &str,
) -> anyhow::Result<Vec<String>> {
    let record = resolve(read, artifact_id)?.clone();
    transitions::validate_transition(&record.status, "deprecated")?;

    let dependents: Vec<String> = read
        .get_incoming_relations(&record.id)
        .into_iter()
        .map(|(source, _)| source)
        .collect();

    let body = post_frontmatter_body(&record)?;
    let today = chrono::Utc::now().format("%Y-%m-%d");
    let new_body = format!("{body}\n\n## Deprecation\n\nReason: {reason}\nDate: {today}");
    projection::update_body_with_projection(ctx, &record.id, &new_body)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    projection::update_metadata_with_projection(ctx, &record.id, Some("deprecated"), None)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(dependents)
}

/// `forgeplan renew` over the canonical source (ADR-005).
///
/// Date validation, the `stale → active` transition check and the 500-char
/// reason sanitisation are the store path's, reused verbatim via
/// [`super::validate_date`] / [`super::sanitize_reason`].
pub async fn renew_captured(
    ctx: &MutationContext<'_>,
    read: &WorkspaceRead,
    artifact_id: &str,
    reason: &str,
    new_valid_until: &str,
) -> anyhow::Result<RenewResult> {
    super::validate_date(new_valid_until)?;

    let record = resolve(read, artifact_id)?.clone();
    transitions::validate_transition(&record.status, "active")?;

    let old_valid_until = record.valid_until.clone();
    let safe_reason = super::sanitize_reason(reason);

    let today = chrono::Utc::now().format("%Y-%m-%d");
    let renewal_section = format!(
        "\n\n## Renewal ({today})\n\nReason: {safe_reason}\nExtended until: {new_valid_until}"
    );
    let body = post_frontmatter_body(&record)?;
    let new_body = format!("{body}{renewal_section}");

    projection::update_body_with_projection(ctx, &record.id, &new_body)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    projection::update_metadata_with_projection(ctx, &record.id, Some("active"), None)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    projection::update_valid_until_with_projection(ctx, &record.id, new_valid_until)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(RenewResult {
        artifact_id: record.id.clone(),
        old_valid_until,
        new_valid_until: new_valid_until.to_string(),
    })
}

/// `forgeplan reopen` over the canonical source.
///
/// Ordering is the audit-C3 contract and is preserved exactly: validate every
/// precondition → create the new draft → link it `based_on` the old → only
/// then deprecate the old. A failure before the last step therefore leaves the
/// old artifact intact and recoverable, which is the whole point of creating
/// first.
pub async fn reopen_captured(
    ctx: &MutationContext<'_>,
    read: &WorkspaceRead,
    artifact_id: &str,
    reason: &str,
    new_id: &str,
) -> anyhow::Result<ReopenResult> {
    let record = resolve(read, artifact_id)?.clone();

    // Same terminal / active-or-stale guard as the store path, word for word:
    // the refusal text is what tells an operator to create a fresh artifact.
    if record.status != "stale" && record.status != "active" {
        if transitions::is_terminal(&record.status) {
            anyhow::bail!(
                "Cannot reopen {}: status '{}' is terminal.\n\
                 Use `forgeplan new {} \"...\"` to create a fresh artifact.",
                artifact_id,
                record.status,
                record.kind
            );
        }
        anyhow::bail!(
            "Cannot reopen {}: status '{}'. Reopen requires active or stale.",
            artifact_id,
            record.status
        );
    }

    // Race guard (audit C3): the caller allocated `new_id` from the same
    // snapshot, but the filesystem is the authority on whether it is free.
    if read.get_record(new_id).is_some()
        || crate::artifact::store::resolve_artifact_path(ctx.workspace(), new_id)
            .await
            .is_some()
    {
        anyhow::bail!(
            "Cannot reopen: artifact '{}' already exists. Try again.",
            new_id
        );
    }

    let safe_reason = super::sanitize_reason(reason);
    let today = chrono::Utc::now().format("%Y-%m-%d");

    // Step 1 — create the new draft first (audit C3: atomicity).
    let lineage = format!(
        "## Lineage\n\nReopened from: {artifact_id}\nReason: {safe_reason}\n\n\
         Previous title: {}\n\n---\n",
        record.title
    );
    let new_artifact = NewArtifact {
        // ADR-030: no minted key yet — the document declares no slug, so its
        // canonical key is its display id.
        canonical_key: new_id.to_string(),
        id: new_id.to_string(),
        kind: record.kind.clone(),
        status: "draft".to_string(),
        title: format!("{} (reopened)", record.title),
        body: lineage,
        depth: record.depth.clone(),
        author: record.author.clone(),
        parent_epic: record.parent_epic.clone(),
        valid_until: None,
        tags: record.tags.clone(),
    };
    projection::create_artifact_with_projection(ctx, &new_artifact)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Step 2 — lineage edge, new → old.
    projection::add_link_with_projection(ctx, new_id, &record.id, "based_on")
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Step 3 — only now retire the old artifact.
    let body = post_frontmatter_body(&record)?;
    let new_body =
        format!("{body}\n\n## Reopened ({today})\n\nReason: {safe_reason}\nReplacement: {new_id}");
    projection::update_body_with_projection(ctx, &record.id, &new_body)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    projection::update_metadata_with_projection(ctx, &record.id, Some("deprecated"), None)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(ReopenResult {
        old_id: record.id.clone(),
        new_id: new_id.to_string(),
        new_kind: record.kind,
    })
}

/// Canonical reference form for hints: ADR-012 slug when the capture carries
/// one, display id otherwise.
///
/// The read-side twin of the `refs_form_from_body(&record.body, ..)` calls the
/// store path makes. Re-derived here rather than exported from the store
/// module because the record's body on this path is the canonical Markdown,
/// which is exactly what the slug lives in.
pub fn refs_form(read: &WorkspaceRead, artifact_id: &str) -> String {
    read.get_record(artifact_id)
        .map(|r| refs_form_from_body(&r.body, &r.id))
        .unwrap_or_else(|| artifact_id.to_string())
}
