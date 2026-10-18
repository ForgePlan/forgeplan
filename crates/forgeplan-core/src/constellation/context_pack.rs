//! Stage 6 — ContextPack: an explainable, budgeted context bundle for an
//! agent's task.
//!
//! The pack answers "which documents should the agent read for this task,
//! what do they cost, and what did we leave out". Every unit carries its
//! source identity, the version it was read at, the exact excerpt bytes
//! that earned selection, and the reasons for selection — a receipt an
//! agent can audit, not a bag of text.
//!
//! * **Lexical ranking** — case-insensitive term scoring over canonical
//!   key, display id, title and body; title/key hits outrank body density.
//! * **Evidence units** — every artifact joins under a *role* the spec
//!   names: normative documents, evidence, project instructions and other
//!   material are distinguished, not flattened into one pile.
//! * **Tokenizer & receipt** — tokens are estimated honestly
//!   (`chars/4`, the same shorthand every cheap estimator uses, labelled
//!   `estimated`); the receipt records budget, spend, per-role counts and
//!   the omissions list — nothing is dropped silently.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;

use super::capture::parse_artifact_file;
use super::federation::{
    QueryMode, SourceChannel, SourceRef, SourceSpec, failure_is_fatal, source_unavailable,
};
use super::jobs::DEFAULT_BODY_PREFIX_CHARS;
use super::snapshot::ConstellationSnapshot;
use super::{ConstellationError, sha256_hex};

/// An artifact's role inside the pack — the spec's four buckets:
/// project instructions, normative artifacts, evidence, other material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitRole {
    /// How the project wants work done (guides, conventions).
    Instruction,
    /// Binding decisions and requirements: adr, rfc, spec, prd.
    Normative,
    /// What was measured or proven.
    Evidence,
    /// Everything else: notes, problems, solutions, epics…
    Material,
}

fn role_for(kind: &str) -> UnitRole {
    match kind {
        "adr" | "rfc" | "spec" | "prd" => UnitRole::Normative,
        "evidence" => UnitRole::Evidence,
        "guide" | "playbook" | "template" => UnitRole::Instruction,
        _ => UnitRole::Material,
    }
}

/// One selected fragment — an evidence unit.
#[derive(Debug, Clone, Serialize)]
pub struct EvidenceUnit {
    pub unit_id: String,
    pub source: SourceRef,
    pub canonical_key: String,
    pub display_id: Option<String>,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub role: UnitRole,
    /// The exact excerpt bytes around the first match — pointable, not a
    /// paraphrase. Byte-sliced on char boundaries.
    pub excerpt: String,
    pub relative_path: String,
    /// Honest token estimate (`chars/4`), labelled by `token_kind`.
    pub tokens: usize,
    pub score: f64,
    /// Why this unit was selected — "term 'auth' in title (x3)", …
    pub reasons: Vec<String>,
}

/// A document that did not make the pack, with why.
#[derive(Debug, Clone, Serialize)]
pub struct Omission {
    pub canonical_key: String,
    pub source_id: String,
    /// `below_threshold` | `over_budget`
    pub reason: String,
    pub score: f64,
}

/// The spend report the spec asks for: budget, spend, per-role counts,
/// per-source counts, and the estimator's label.
#[derive(Debug, Clone, Serialize)]
pub struct BudgetReceipt {
    pub budget_tokens: usize,
    pub used_tokens: usize,
    pub token_kind: &'static str,
    pub units: usize,
    pub per_role: BTreeMap<String, usize>,
    pub per_source: BTreeMap<String, usize>,
}

/// The pack itself.
#[derive(Debug, Serialize)]
pub struct ContextPack {
    /// What the pack was built for — echoed so the receipt is auditable.
    pub task: String,
    /// Version keys of the sources that answered (generation ids or
    /// capture digests or git OIDs — the channel says which).
    pub sources: Vec<SourceRef>,
    pub units: Vec<EvidenceUnit>,
    pub omitted: Vec<Omission>,
    pub receipt: BudgetReceipt,
    /// Sources that could not answer at all — the pack is `partial`.
    pub failures: Vec<super::federation::SourceFailure>,
}

const TOKEN_KIND: &str = "estimated_chars_div_4";
/// Excerpt half-window around a match, in chars.
const EXCERPT_HALF: usize = 200;
/// Score floor for a unit to be considered at all.
const SCORE_FLOOR: f64 = 1.0;

fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

/// Task text → normalized query terms (3+ chars, de-duplicated).
fn terms_of(task: &str) -> Vec<String> {
    let mut terms: Vec<String> = task
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(str::to_string)
        .collect();
    terms.sort();
    terms.dedup();
    terms
}

/// Byte-slice `body` around `offset` on char boundaries, ~2×HALF chars.
fn excerpt_around(body: &str, offset: Option<usize>) -> String {
    let start = offset.map(|o| o.saturating_sub(EXCERPT_HALF)).unwrap_or(0);
    let mut s = start.min(body.len());
    while s > 0 && !body.is_char_boundary(s) {
        s -= 1;
    }
    let mut e = (s + 2 * EXCERPT_HALF).min(body.len());
    while e < body.len() && !body.is_char_boundary(e) {
        e += 1;
    }
    body[s..e].to_string()
}

/// The parsed form a packable document takes from any channel.
#[derive(Clone)]
struct Packable {
    canonical_key: String,
    display_id: Option<String>,
    title: String,
    kind: String,
    status: String,
    body: String,
    relative_path: String,
}

/// Read every artifact in one live source via its markdown channel.
async fn collect_live(
    spec: &SourceSpec,
) -> Result<(SourceRef, Vec<Packable>), super::federation::SourceFailure> {
    let canonical = spec
        .path
        .canonicalize()
        .map_err(|e| super::federation::SourceFailure {
            path: spec.path.display().to_string(),
            reason: format!("cannot canonicalize: {e}"),
        })?;
    let captured = super::capture::capture_source(
        &super::capture::CaptureRequest::new(&canonical),
        super::capture::CaptureLimits::default(),
    )
    .await
    .map_err(|e| super::federation::SourceFailure {
        path: spec.path.display().to_string(),
        reason: format!("{e}"),
    })?;
    let source = SourceRef {
        source_id: format!(
            "src:{}",
            &sha256_hex(canonical.to_string_lossy().as_bytes())[..16]
        ),
        path: canonical.display().to_string(),
        channel: SourceChannel::Markdown,
        version_key: captured.source_set_digest.clone(),
    };
    let docs = captured
        .artifacts
        .iter()
        .map(|a| Packable {
            canonical_key: a.canonical_key.clone(),
            display_id: Some(a.display_id.clone()),
            title: a.title.clone(),
            kind: a.kind.clone(),
            status: a.status.clone(),
            body: a.body.to_string(),
            relative_path: a.relative_path.clone(),
        })
        .collect();
    Ok((source, docs))
}

/// Every artifact in a snapshot source, parsed from pinned git objects.
fn collect_snapshot(
    entry: &super::snapshot::SnapshotSourceEntry,
) -> Result<(SourceRef, Vec<Packable>), String> {
    let repo = PathBuf::from(&entry.checkout_root);
    let names =
        super::snapshot::list_files_at(&repo, &entry.git_oid).map_err(|e| format!("{e}"))?;
    let source = SourceRef {
        source_id: format!(
            "src:{}",
            &sha256_hex(format!("{}@{}", entry.checkout_root, entry.git_oid).as_bytes())[..16]
        ),
        path: entry.source_path.clone(),
        channel: SourceChannel::PinnedGit,
        version_key: format!(
            "{}:{}:{}",
            entry.git_oid, entry.tree_oid, entry.bytes_digest
        ),
    };
    let mut docs = Vec::new();
    for rel in names {
        let bytes = super::snapshot::read_file_at(&repo, &entry.git_oid, &rel)
            .map_err(|e| format!("{e}"))?;
        if let Ok(p) = parse_artifact_file(&rel, Arc::from(&bytes[..]), DEFAULT_BODY_PREFIX_CHARS) {
            docs.push(Packable {
                canonical_key: p.canonical_key.clone(),
                display_id: Some(p.display_id.clone()),
                title: p.title.clone(),
                kind: p.kind.clone(),
                status: p.status.clone(),
                body: p.body.to_string(),
                relative_path: p.relative_path.clone(),
            });
        }
    }
    Ok((source, docs))
}

fn build_pack(
    task: &str,
    budget_tokens: usize,
    sources: Vec<(SourceRef, Vec<Packable>)>,
    failures: Vec<super::federation::SourceFailure>,
) -> ContextPack {
    let terms = terms_of(task);
    // Score everything; hold the (source, doc, score, reasons, offset).
    struct Scored {
        source: SourceRef,
        doc: Packable,
        score: f64,
        reasons: Vec<String>,
        offset: Option<usize>,
    }
    let mut scored: Vec<Scored> = Vec::new();
    for (source, docs) in &sources {
        for doc in docs.iter().cloned() {
            // Re-parse through the same scorer — Packable mirrors the
            // fields ParsedArtifact offers.
            let key = doc.canonical_key.to_lowercase();
            let did = doc.display_id.clone().unwrap_or_default().to_lowercase();
            let title = doc.title.to_lowercase();
            let body = doc.body.to_lowercase();
            let mut score = 0.0;
            let mut reasons = Vec::new();
            let mut first_offset: Option<usize> = None;
            for term in &terms {
                if key.contains(term) || did.contains(term) {
                    score += 4.0;
                    reasons.push(format!("term '{term}' in id (x4)"));
                }
                if title.contains(term) {
                    score += 3.0;
                    reasons.push(format!("term '{term}' in title (x3)"));
                }
                let occurrences = body.matches(term).count();
                if occurrences > 0 {
                    score += occurrences.min(3) as f64;
                    reasons.push(format!("term '{term}' in body (x{occurrences})"));
                    if first_offset.is_none() {
                        first_offset = body.find(term);
                    }
                }
            }
            scored.push(Scored {
                source: source.clone(),
                doc,
                score,
                reasons,
                offset: first_offset,
            });
        }
    }
    // Deterministic order: score desc, then canonical_key, then source id.
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.doc.canonical_key.cmp(&b.doc.canonical_key))
            .then_with(|| a.source.source_id.cmp(&b.source.source_id))
    });

    let mut units = Vec::new();
    let mut omitted = Vec::new();
    let mut used = 0usize;
    for (rank, item) in scored.into_iter().enumerate() {
        // A zero score means nothing matched. Naming such a unit in
        // `omitted` is evidence next to a real answer ("seen and rejected"),
        // but on a zero-match task it would list the entire corpus as
        // omitted — noise instead of signal. `scored` is sorted by score
        // desc, so every included unit is already in `units` before the
        // first zero-score row arrives.
        if item.score <= 0.0 {
            if !units.is_empty() {
                omitted.push(Omission {
                    canonical_key: item.doc.canonical_key,
                    source_id: item.source.source_id.clone(),
                    reason: "below_threshold".into(),
                    score: item.score,
                });
            }
            continue;
        }
        if item.score < SCORE_FLOOR {
            omitted.push(Omission {
                canonical_key: item.doc.canonical_key,
                source_id: item.source.source_id.clone(),
                reason: "below_threshold".into(),
                score: item.score,
            });
            continue;
        }
        let excerpt = excerpt_around(&item.doc.body, item.offset);
        let tokens = estimate_tokens(&excerpt) + estimate_tokens(&item.doc.title) + 8;
        if used + tokens > budget_tokens {
            omitted.push(Omission {
                canonical_key: item.doc.canonical_key,
                source_id: item.source.source_id.clone(),
                reason: "over_budget".into(),
                score: item.score,
            });
            continue;
        }
        used += tokens;
        units.push(EvidenceUnit {
            unit_id: format!("ev-{:03}", rank + 1),
            source: item.source,
            canonical_key: item.doc.canonical_key,
            display_id: item.doc.display_id,
            title: item.doc.title,
            kind: item.doc.kind.clone(),
            status: item.doc.status,
            role: role_for(&item.doc.kind),
            excerpt,
            relative_path: item.doc.relative_path,
            tokens,
            score: item.score,
            reasons: item.reasons,
        });
    }
    let mut per_role: BTreeMap<String, usize> = BTreeMap::new();
    let mut per_source: BTreeMap<String, usize> = BTreeMap::new();
    for unit in &units {
        *per_role
            .entry(format!("{:?}", unit.role).to_lowercase())
            .or_default() += 1;
        *per_source.entry(unit.source.source_id.clone()).or_default() += 1;
    }
    let unit_count = units.len();
    ContextPack {
        task: task.to_string(),
        sources: sources.iter().map(|(s, _)| s.clone()).collect(),
        units,
        omitted,
        receipt: BudgetReceipt {
            budget_tokens,
            used_tokens: used,
            token_kind: TOKEN_KIND,
            units: unit_count,
            per_role,
            per_source,
        },
        failures,
    }
}

/// Pack a task's context from live scope sources.
///
/// `mode` applies the required/optional contract: a `required` source
/// that cannot answer aborts the strict pack with `source_unavailable` —
/// a pack assembled without it would silently under-rank what the task
/// needed. An `optional` source, or any source under
/// [`QueryMode::AllowPartial`], lands in `failures` instead.
pub async fn pack_live(
    specs: &[SourceSpec],
    task: &str,
    budget_tokens: usize,
    mode: QueryMode,
) -> Result<ContextPack, ConstellationError> {
    let mut sources = Vec::new();
    let mut failures = Vec::new();
    for spec in specs {
        match collect_live(spec).await {
            Ok(pair) => sources.push(pair),
            Err(f) => {
                if failure_is_fatal(spec, mode) {
                    return Err(source_unavailable(&f));
                }
                failures.push(f);
            }
        }
    }
    Ok(build_pack(task, budget_tokens, sources, failures))
}

/// Pack a task's context from a recorded snapshot — pinned revisions.
pub fn pack_snapshot(
    snapshot: &ConstellationSnapshot,
    task: &str,
    budget_tokens: usize,
) -> Result<ContextPack, ConstellationError> {
    let mut sources = Vec::new();
    let mut failures: Vec<super::federation::SourceFailure> = snapshot
        .skipped
        .iter()
        .map(|s| super::federation::SourceFailure {
            path: s.path.clone(),
            reason: format!("source was not pinned: {}", s.reason),
        })
        .collect();
    for entry in &snapshot.sources {
        match collect_snapshot(entry) {
            Ok(pair) => sources.push(pair),
            Err(reason) => failures.push(super::federation::SourceFailure {
                path: entry.source_path.clone(),
                reason,
            }),
        }
    }
    Ok(build_pack(task, budget_tokens, sources, failures))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const AUTH_DOC: &str = r#"---
id: ADR-001
kind: adr
status: accepted
title: Auth storage decision
---
# Auth storage decision

We decided auth tokens live in the token vault service, not in cookies.
The token vault encrypts at rest and rotates every hour.
"#;

    const COOKIES_DOC: &str = r#"---
id: NOTE-007
kind: note
status: draft
---
# Frontend styling roundup

Fonts, colors and spacing. No backend concerns here.
"#;

    /// A `.forgeplan` root with `kind/` artifact directories — the same
    /// shape federation fixtures use.
    fn ws_root() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".forgeplan");
        std::fs::create_dir_all(root.join("adrs")).unwrap();
        std::fs::create_dir_all(root.join("notes")).unwrap();
        std::fs::write(root.join("adrs/ADR-001-auth.md"), AUTH_DOC).unwrap();
        std::fs::write(root.join("notes/NOTE-007-style.md"), COOKIES_DOC).unwrap();
        tmp
    }

    #[tokio::test]
    async fn pack_selects_relevant_unit_with_reasons_and_receipt() {
        let ws = ws_root();
        let specs = vec![SourceSpec::new(ws.path().join(".forgeplan"))];

        let pack = pack_live(&specs, "auth token storage", 10_000, QueryMode::Strict)
            .await
            .unwrap();
        assert!(pack.failures.is_empty(), "{:?}", pack.failures);
        assert_eq!(pack.units.len(), 1);
        let unit = &pack.units[0];
        assert_eq!(unit.display_id.as_deref(), Some("ADR-001"));
        assert_eq!(unit.role, UnitRole::Normative);
        assert!(unit.reasons.iter().any(|r| r.contains("'auth' in title")));
        assert!(unit.reasons.iter().any(|r| r.contains("'auth'")));
        assert!(unit.excerpt.contains("token vault") || unit.excerpt.contains("auth"));
        // The irrelevant doc is named in omissions, not silently absent.
        assert!(
            pack.omitted
                .iter()
                .any(|o| o.canonical_key.contains("NOTE-007") && o.reason == "below_threshold")
        );
        assert_eq!(pack.receipt.units, 1);
        assert_eq!(pack.receipt.token_kind, "estimated_chars_div_4");
        assert_eq!(*pack.receipt.per_role.get("normative").unwrap(), 1);
    }

    #[tokio::test]
    async fn budget_excludes_units_and_reports_over_budget() {
        let ws = ws_root();
        let specs = vec![SourceSpec::new(ws.path().join(".forgeplan"))];
        let pack = pack_live(&specs, "auth", 20, QueryMode::Strict)
            .await
            .unwrap();
        assert_eq!(pack.units.len(), 0);
        assert!(
            pack.omitted
                .iter()
                .any(|o| o.reason == "over_budget" && o.canonical_key.contains("ADR-001"))
        );
        assert_eq!(pack.receipt.used_tokens, 0);
    }

    #[tokio::test]
    async fn same_task_packs_identically_twice() {
        let ws = ws_root();
        let specs = vec![SourceSpec::new(ws.path().join(".forgeplan"))];
        let p1 = pack_live(&specs, "auth", 10_000, QueryMode::Strict)
            .await
            .unwrap();
        let p2 = pack_live(&specs, "auth", 10_000, QueryMode::Strict)
            .await
            .unwrap();
        let j1 = serde_json::to_string(&p1.units).unwrap();
        let j2 = serde_json::to_string(&p2.units).unwrap();
        assert_eq!(j1, j2);
    }

    #[tokio::test]
    async fn a_dead_required_source_fails_the_pack_strictly() {
        // A required source's outage must not produce a pack that reads
        // as complete — the strict read errors; optional sources (or an
        // explicit AllowPartial call) keep the failures envelope.
        let ws = ws_root();
        let specs = vec![
            SourceSpec::new(ws.path().join(".forgeplan")),
            SourceSpec::new("/definitely/not/a/workspace"),
        ];
        let err = pack_live(&specs, "auth", 10_000, QueryMode::Strict)
            .await
            .unwrap_err();
        assert!(matches!(err, ConstellationError::SourceUnavailable { .. }));

        let pack = pack_live(&specs, "auth", 10_000, QueryMode::AllowPartial)
            .await
            .unwrap();
        assert_eq!(pack.failures.len(), 1);
        assert_eq!(pack.units.len(), 1);

        let specs = vec![
            SourceSpec::new(ws.path().join(".forgeplan")),
            SourceSpec::new_optional("/definitely/not/a/workspace"),
        ];
        let pack = pack_live(&specs, "auth", 10_000, QueryMode::Strict)
            .await
            .unwrap();
        assert_eq!(pack.failures.len(), 1);
        assert_eq!(pack.units.len(), 1);
    }
}
