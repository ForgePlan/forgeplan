use forgeplan_core::artifact::frontmatter::{
    assigned_number_from_frontmatter, parse_frontmatter, predicted_number_from_frontmatter,
    slug_from_frontmatter,
};
use forgeplan_core::artifact::store::ArtifactSummary;
use forgeplan_core::artifact::types::{ArtifactKind, render_display_id, validate_slug};
use forgeplan_core::db::store::ArtifactRecord;
use forgeplan_core::validation::{Finding, ValidationResult};

use crate::types::{
    ArtifactRecordDto, ArtifactSummaryDto, ValidationFindingDto, ValidationResultDto,
};

/// Identity triple extracted from an artifact's frontmatter for PROB-060
/// / SPEC-005 / ADR-012 (CD-2 binding).
///
/// Bundles the canonical slug, predicted/assigned numbers, pretty display id,
/// and lookup-stable canonical id into one value. Computed once from the
/// artifact's stored body (which still contains the frontmatter) so we don't
/// re-parse for every consumer.
///
/// Legacy artifacts that pre-date Phase 1 frontmatter return `None` for
/// `slug`/`predicted_number`/`assigned_number` and fall back to the existing
/// display-id form for `id_canonical` / `id_display`.
pub(crate) struct IdentityFields {
    pub slug: Option<String>,
    pub predicted_number: Option<u32>,
    pub assigned_number: Option<u32>,
    pub id_canonical: String,
    pub id_display: String,
}

/// Compute [`IdentityFields`] from a record's stored body.
///
/// `body` here is the raw markdown payload as persisted in LanceDB —
/// frontmatter is included (see `LanceStore::resolve_id`, which also calls
/// `parse_frontmatter(&record.body)`). Failure to parse frontmatter is
/// treated as "legacy artifact" — we still emit a usable display id so
/// callers don't have to special-case `None`.
///
/// **HIGH-3 (Round-1 audit)**: a slug surviving from a tampered file or a
/// hand-edited DB row could carry shell metacharacters or invisible
/// Unicode. The slug we surface to MCP DTO consumers is gated through
/// [`validate_slug`] — anything that doesn't satisfy SPEC-005's slug
/// grammar is dropped to `None` rather than propagated. Downstream hint
/// sites then either use the post-merge display id or apply
/// `sanitize_for_hint` as a second layer. CWE-117 + prompt-injection
/// defence in depth.
pub(crate) fn identity_from_record(id: &str, kind_str: &str, body: &str) -> IdentityFields {
    let (slug, predicted, assigned) = match parse_frontmatter(body) {
        Ok((fm, _body)) => (
            slug_from_frontmatter(&fm)
                .filter(|s| {
                    // HIGH-3: drop slugs that violate the SPEC-005 grammar.
                    // We log a structured warning so an operator can spot
                    // the bad row, but we never let a malformed slug flow
                    // into agent-visible DTO fields.
                    let ok = validate_slug(s).is_ok();
                    if !ok {
                        tracing::warn!(
                            target: "forgeplan_mcp::convert",
                            artifact_id = %id,
                            kind = %kind_str,
                            "dropping invalid slug from frontmatter (HIGH-3 defence)"
                        );
                    }
                    ok
                })
                .map(|s| s.to_string()),
            predicted_number_from_frontmatter(&fm),
            assigned_number_from_frontmatter(&fm),
        ),
        Err(_) => (None, None, None),
    };

    // id_canonical: slug if we have it, else legacy lowercased display id.
    // The lowercased form mirrors what `LanceStore::resolve_id` accepts so
    // an agent can round-trip the value without normalization surprises.
    let id_canonical = slug.clone().unwrap_or_else(|| id.to_ascii_lowercase());

    // id_display: render via the same helper used by CLI / Web. When we
    // can't determine a kind (corrupt frontmatter) or have no predicted
    // number to anchor the `?` form, fall back to the verbatim id.
    let id_display = match (kind_str.parse::<ArtifactKind>(), predicted) {
        (Ok(k), Some(p)) => render_display_id(&k, p, assigned),
        _ => id.to_string(),
    };

    IdentityFields {
        slug,
        predicted_number: predicted,
        assigned_number: assigned,
        id_canonical,
        id_display,
    }
}

impl From<ArtifactSummary> for ArtifactSummaryDto {
    fn from(s: ArtifactSummary) -> Self {
        // Note: ArtifactSummary has no body, so we cannot extract identity
        // fields here. Callers that need the identity triple should build
        // the DTO from an `ArtifactRecord` (see `From<ArtifactRecord>`),
        // not from a summary. This impl is preserved for back-compat with
        // call sites that don't care about identity (e.g. dispatch
        // candidate listing).
        let id_canonical = s.id.to_ascii_lowercase();
        let id_display = s.id.clone();
        Self {
            id: s.id,
            kind: s.kind,
            status: s.status,
            title: s.title,
            slug: None,
            predicted_number: None,
            assigned_number: None,
            id_canonical,
            id_display,
        }
    }
}

impl From<ArtifactRecord> for ArtifactRecordDto {
    fn from(r: ArtifactRecord) -> Self {
        let identity = identity_from_record(&r.id, &r.kind, &r.body);
        Self {
            id: r.id,
            kind: r.kind,
            status: r.status,
            title: r.title,
            body: r.body,
            depth: r.depth,
            author: r.author,
            parent_epic: r.parent_epic,
            r_eff_score: r.r_eff_score,
            valid_until: r.valid_until,
            created_at: r.created_at,
            updated_at: r.updated_at,
            slug: identity.slug,
            predicted_number: identity.predicted_number,
            assigned_number: identity.assigned_number,
            id_canonical: identity.id_canonical,
            id_display: identity.id_display,
            // #447 — this conversion has no store handle, so edges cannot be
            // read here. The caller that does have one fills them in; see
            // `forgeplan_get` in server.rs. Left empty rather than optional so
            // a caller who forgets still emits `{"outbound": [], "inbound": []}`
            // instead of silently dropping the field.
            links: Default::default(),
        }
    }
}

impl From<ArtifactRecord> for ArtifactSummaryDto {
    fn from(r: ArtifactRecord) -> Self {
        let identity = identity_from_record(&r.id, &r.kind, &r.body);
        Self {
            id: r.id,
            kind: r.kind,
            status: r.status,
            title: r.title,
            slug: identity.slug,
            predicted_number: identity.predicted_number,
            assigned_number: identity.assigned_number,
            id_canonical: identity.id_canonical,
            id_display: identity.id_display,
        }
    }
}

impl From<Finding> for ValidationFindingDto {
    fn from(f: Finding) -> Self {
        Self {
            rule_id: f.rule_id,
            severity: f.severity.to_string(),
            message: f.message,
            section: f.section,
        }
    }
}

impl From<ValidationResult> for ValidationResultDto {
    fn from(r: ValidationResult) -> Self {
        let passed = r.passed();
        let error_count = r.error_count();
        let warning_count = r.warning_count();
        Self {
            artifact_id: r.artifact_id,
            kind: r.kind,
            depth: r.depth,
            passed,
            error_count,
            warning_count,
            findings: r.findings.into_iter().map(Into::into).collect(),
        }
    }
}

// ── Shared artifact-reference resolution (CLI + MCP) ─────────
//
// One resolver, one taxonomy, one implementation — shared by the CLI and
// the MCP server so the two surfaces can never drift apart.
//
// The rules live in `forgeplan_core::artifact::canonical_key::resolve_artifact`:
// the document set decides, the algorithm picks, and the outcome is one of
// four distinguishable facts (exactly one document, none, several, or an
// unusable reference / incomplete document set).

use forgeplan_core::artifact::canonical_key::{
    ArtifactCandidate, ArtifactIdentity, ResolveError, ResolvedArtifact,
    resolve_artifact as resolve_artifact_shared,
};
use std::path::{Path, PathBuf};

/// The outcome of resolving one artifact reference.
///
/// The outcomes are deliberately distinct — a caller must be able to tell
/// "no document carries this name" from "several do" from "this is not a
/// reference at all" from "the document set could not be read in full".
/// Collapsing any two of them is how a write lands on the wrong document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// Exactly one document answers the reference.
    Found(ResolvedArtifact),
    /// No document answers the reference.
    NotFound,
    /// Several documents answer the reference — every candidate is named.
    Ambiguous {
        /// The selector as the caller wrote it.
        input: String,
        /// Every document that matched, sorted by canonical key then path.
        candidates: Vec<ArtifactCandidate>,
    },
    /// Several documents claim ONE canonical key.
    ///
    /// A different fact from an ambiguity: the name is not shared, the
    /// IDENTITY is. The selector named a key and two documents hold it, so
    /// the document set itself is broken — a conflict to repair, not a
    /// selector to disambiguate. It stays its own variant so the code that
    /// names the cause can say so.
    ConflictingCanonicalKey {
        /// The contested canonical key.
        key: String,
        /// Every document claiming the key, sorted by canonical key then path.
        candidates: Vec<ArtifactCandidate>,
    },
    /// The reference is not a usable selector.
    InvalidSelector {
        /// The selector as the caller wrote it.
        input: String,
    },
    /// The document set could not be read in full, so no answer over it
    /// would be complete.
    IncompleteCapture {
        /// What was missing from the document set.
        message: String,
    },
}

impl Resolution {
    /// The canonical key of the one document this reference names.
    ///
    /// `None` for every other outcome — a caller has to branch on the
    /// outcome anyway, and a key read out of an ambiguity would be exactly
    /// the silent winner this type exists to prevent.
    pub fn canonical_key(&self) -> Option<&str> {
        match self {
            Self::Found(resolved) => Some(resolved.identity.canonical_key.as_str()),
            _ => None,
        }
    }

    /// The display id of the one document this reference names.
    pub fn display_id(&self) -> Option<&str> {
        match self {
            Self::Found(resolved) => Some(resolved.identity.display_id.as_str()),
            _ => None,
        }
    }

    /// Whether the reference named exactly one document.
    pub fn is_found(&self) -> bool {
        matches!(self, Self::Found(_))
    }
}

impl From<Result<Option<ResolvedArtifact>, ResolveError>> for Resolution {
    fn from(result: Result<Option<ResolvedArtifact>, ResolveError>) -> Self {
        match result {
            Ok(Some(resolved)) => Self::Found(resolved),
            Ok(None) => Self::NotFound,
            Err(ResolveError::Ambiguous { input, candidates }) => {
                Self::Ambiguous { input, candidates }
            }
            Err(ResolveError::InvalidSelector { input }) => Self::InvalidSelector { input },
            Err(ResolveError::IncompleteCapture { message }) => {
                Self::IncompleteCapture { message }
            }
            // A contested canonical key is its own fact: the name is not
            // shared, the IDENTITY is. It keeps its own variant so the
            // caller can name the cause — "two documents claim this key"
            // is a repair task, "several documents answer this name" is a
            // choice the caller can make.
            Err(ResolveError::ConflictingCanonicalKey { key, candidates }) => {
                Self::ConflictingCanonicalKey { key, candidates }
            }
        }
    }
}

/// Resolve one selector against a document set.
///
/// This is the one resolver both surfaces call. It adds nothing to
/// `forgeplan_core::artifact::canonical_key::resolve_artifact` except the
/// outcome type the CLI and the MCP server share.
pub fn resolve_reference(input: &str, documents: &[ArtifactCandidate]) -> Resolution {
    resolve_artifact_shared(input, documents).into()
}

/// Build the document set of a legacy (LanceDB) workspace.
///
/// The legacy index is keyed by the canonical key, so its rows ARE the
/// document set. The relative path is left empty because the legacy index
/// does not record where a row's markdown lives — a candidate without one
/// still names itself by canonical key and title.
///
/// The implementation lives in core — the document set is a workspace
/// capability, not an MCP one — and is re-exported here so the call sites
/// read the way they always did.
pub use forgeplan_core::artifact::documents::{
    candidates_from_records, candidates_from_summaries,
};

/// The candidate list of an ambiguity, rendered for a human or an agent.
///
/// Every candidate is named by its canonical key, its title and its
/// workspace-relative path. No absolute path and no other source's data
/// ever reaches the output.
pub fn describe_candidates(candidates: &[ArtifactCandidate]) -> Vec<String> {
    candidates
        .iter()
        .map(|candidate| {
            if candidate.relative_path.as_os_str().is_empty() {
                format!("{} — {}", candidate.canonical_key, candidate.title)
            } else {
                format!(
                    "{} — {} ({})",
                    candidate.canonical_key,
                    candidate.title,
                    candidate.relative_path.display()
                )
            }
        })
        .collect()
}

/// The machine-readable code for a resolution outcome.
///
/// The MCP error envelope carries it so an agent can branch on the failure
/// without parsing prose.
pub fn resolution_code(resolution: &Resolution) -> &'static str {
    match resolution {
        Resolution::Found(_) => "ok",
        Resolution::NotFound => "not_found",
        Resolution::Ambiguous { .. } => "ambiguous",
        Resolution::ConflictingCanonicalKey { .. } => "conflicting_canonical_key",
        Resolution::InvalidSelector { .. } => "invalid_selector",
        Resolution::IncompleteCapture { .. } => "incomplete_capture",
    }
}

/// Resolve a reference against a captured (constellation) workspace read.
///
/// The capture is the document set: it already knows each document's
/// canonical key, display id, title and relative path, so the shared
/// algorithm decides over exactly the documents the read view holds.
pub fn resolve_in_capture(
    read: &forgeplan_core::constellation::workspace_read::WorkspaceRead,
    input: &str,
) -> Resolution {
    read.resolve_artifact(input).into()
}

/// Resolve a reference against the workspace's own markdown files.
///
/// The file-level counterpart of [`resolve_in_capture`]: the documents on
/// disk ARE the document set, so the same shared algorithm decides over
/// them and answers with the same four outcomes. A migrated workspace
/// answers from here when no capture is open.
///
/// The workspace argument is the `.forgeplan` directory — the same root the
/// projection writes into — so the relative path of a candidate is the one
/// an operator can open.
pub async fn resolve_in_workspace_files(workspace: &Path, input: &str) -> Resolution {
    forgeplan_core::artifact::documents::resolve_in_workspace(workspace, input)
        .await
        .into()
}

/// Resolve a reference against a legacy (LanceDB) index.
///
/// The index is the document set on this layout, so the index's own
/// resolution decides whether the reference names exactly one row. When it
/// does not, the candidate set is rebuilt from the index's rows to say
/// WHY: an alias two rows share is an ambiguity, not a miss.
///
/// Candidates carry the row's own canonical key — the key the index stores
/// and addresses rows by — so a candidate names the same document the
/// index would have written to.
pub async fn resolve_in_legacy_index(
    store: &forgeplan_core::db::store::LanceStore,
    workspace: &std::path::Path,
    input: &str,
) -> anyhow::Result<Resolution> {
    if let Some(canonical_key) = store.resolve_canonical_key(input).await? {
        let summary = store.get_artifact(&canonical_key).await?;
        let display_id = summary
            .as_ref()
            .map(|summary| summary.id.clone())
            .unwrap_or_else(|| canonical_key.clone());
        return Ok(Resolution::Found(ResolvedArtifact {
            identity: ArtifactIdentity {
                canonical_key,
                display_id,
            },
            relative_path: PathBuf::new(),
        }));
    }

    // The index named no single row. Rebuild the candidate set from the
    // index's own rows and let the shared algorithm classify the refusal.
    //
    // The paths come from the workspace's own documents — the SAME scan the
    // file route reads — so a refusal points at a file an operator can open
    // without a second walk of the directories.
    let records = store.list_records(None).await?;
    let mut candidates = candidates_from_records(&records);
    if !candidates.is_empty() {
        let paths = forgeplan_core::artifact::documents::workspace_document_paths(workspace).await?;
        for candidate in &mut candidates {
            if let Some(path) = paths.get(&candidate.canonical_key) {
                candidate.relative_path = path.clone();
            }
        }
    }
    Ok(match resolve_reference(input, &candidates) {
        // The index resolved nothing, so a rebuilt set that resolves
        // something would contradict the index. The index wins.
        Resolution::Found(_) => Resolution::NotFound,
        other => other,
    })
}

/// A reference resolved against a document set, for a caller that may
/// accept a document that does not exist yet.
///
/// An edge endpoint is one of those callers: a link to a document that has
/// not been created is a legal forward reference, so "no document answers"
/// is a usable answer there. It is NOT a usable answer everywhere else,
/// which is why this type exists next to [`Resolution`] rather than
/// replacing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    /// Exactly one document answers the reference.
    Document {
        /// The document's permanent identity.
        canonical_key: String,
        /// The document's human-facing alias.
        display_id: String,
    },
    /// No document answers the reference yet — a forward reference.
    ///
    /// The declared text is kept verbatim so the caller can write it
    /// through unchanged.
    Forward(String),
}

/// Resolve a reference that may name a document that does not exist yet.
///
/// The rules are the shared ones, with one difference in what a MISS means:
/// here a miss is a forward reference, not an error.
///
/// An AMBIGUITY is still an error here. A forward reference is a promise
/// about a document that does not exist; an ambiguity is two documents that
/// already exist and both answer to the name, and writing to either of them
/// is the silent pick this whole resolver exists to prevent. So the
/// ambiguity — and a contested canonical key, an unusable selector, an
/// incomplete document set — come back as `Err`.
pub fn resolve_endpoint(
    input: &str,
    documents: &[ArtifactCandidate],
) -> Result<Endpoint, Resolution> {
    match resolve_reference(input, documents) {
        Resolution::Found(resolved) => Ok(Endpoint::Document {
            canonical_key: resolved.identity.canonical_key,
            display_id: resolved.identity.display_id,
        }),
        // A miss is legal here: the document may still be created.
        Resolution::NotFound => Ok(Endpoint::Forward(input.to_string())),
        // Everything else is an error, and stays one.
        other => Err(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a body with the given slug field.
    fn body_with_slug(slug: &str, assigned: Option<u32>) -> String {
        let assigned_field = match assigned {
            Some(n) => format!("assigned_number: {n}"),
            None => "assigned_number: null".to_string(),
        };
        format!(
            "---\nid: PRD-074\nslug: \"{slug}\"\npredicted_number: 74\n{assigned_field}\n---\n\n# body\n"
        )
    }

    // HIGH-3 (Round-1 audit): identity_from_record must drop slugs that
    // fail SPEC-005's grammar. Otherwise tampered frontmatter would
    // propagate through to MCP DTOs and into agent-visible hint strings.

    #[test]
    fn identity_drops_slug_with_shell_metacharacters() {
        // `;` and space are not in the valid slug charset.
        let body = body_with_slug("; rm -rf /", None);
        let id = identity_from_record("PRD-074", "prd", &body);
        assert_eq!(
            id.slug, None,
            "slug with shell metacharacters must be rejected"
        );
        // id_canonical falls back to lowercased display id.
        assert_eq!(id.id_canonical, "prd-074");
    }

    #[test]
    fn identity_drops_uppercase_slug() {
        // SPEC-005 requires lowercase ASCII.
        let body = body_with_slug("PRD-Auth-System", None);
        let id = identity_from_record("PRD-074", "prd", &body);
        assert_eq!(id.slug, None);
    }

    #[test]
    fn identity_drops_unknown_kind_prefix_slug() {
        let body = body_with_slug("xyz-some-thing", None);
        let id = identity_from_record("PRD-074", "prd", &body);
        assert_eq!(id.slug, None);
    }

    #[test]
    fn identity_keeps_valid_slug() {
        let body = body_with_slug("prd-auth-system", Some(74));
        let id = identity_from_record("PRD-074", "prd", &body);
        assert_eq!(id.slug.as_deref(), Some("prd-auth-system"));
        assert_eq!(id.id_canonical, "prd-auth-system");
    }

    #[test]
    fn identity_drops_zero_width_slug() {
        // Zero-width chars are non-ASCII so validate_slug rejects on the
        // ASCII gate.
        let body = "---\nid: PRD-074\nslug: \"prd-\u{200B}auth\"\npredicted_number: 74\nassigned_number: null\n---\n\n# body\n";
        let id = identity_from_record("PRD-074", "prd", body);
        assert_eq!(id.slug, None);
    }

    #[test]
    fn identity_drops_reserved_prefix_slug() {
        // "tmp-" / "draft-" / "pending-" suffixes are reserved.
        let body = body_with_slug("prd-tmp-something", None);
        let id = identity_from_record("PRD-074", "prd", &body);
        assert_eq!(id.slug, None);
    }

    // ── Shared resolution ────────────────────────────────────

    /// A candidate with a canonical key, a display alias and a title.
    fn candidate(key: &str, display: &str, title: &str, path: &str) -> ArtifactCandidate {
        ArtifactCandidate {
            canonical_key: key.to_string(),
            display_id: display.to_string(),
            title: title.to_string(),
            relative_path: PathBuf::from(path),
        }
    }

    #[test]
    fn a_unique_display_alias_resolves_to_that_one_document() {
        let documents = vec![
            candidate("prd-alpha-k0123", "PRD-001", "Alpha", "prds/alpha.md"),
            candidate("prd-beta-k0456", "PRD-002", "Beta", "prds/beta.md"),
        ];
        let resolution = resolve_reference("PRD-002", &documents);
        assert!(resolution.is_found(), "a unique alias must resolve");
        assert_eq!(resolution.canonical_key(), Some("prd-beta-k0456"));
        assert_eq!(resolution.display_id(), Some("PRD-002"));
        assert_eq!(resolution_code(&resolution), "ok");
    }

    #[test]
    fn an_alias_shared_by_two_documents_is_an_ambiguity_naming_every_candidate() {
        let documents = vec![
            candidate("prd-alpha-k0123", "PRD-001", "Alpha", "prds/alpha.md"),
            candidate("prd-beta-k0456", "PRD-001", "Beta", "prds/beta.md"),
        ];
        let resolution = resolve_reference("PRD-001", &documents);
        assert!(
            !resolution.is_found(),
            "an alias two documents share must not resolve to a winner"
        );
        assert_eq!(resolution_code(&resolution), "ambiguous");
        match &resolution {
            Resolution::Ambiguous { candidates, .. } => {
                assert_eq!(
                    candidates.len(),
                    2,
                    "both claimants must be named, or the ambiguity hides one"
                );
            }
            other => panic!("expected an ambiguity, got {other:?}"),
        }
    }

    #[test]
    fn a_selector_no_document_carries_is_not_found() {
        let documents = vec![candidate("prd-alpha-k0123", "PRD-001", "Alpha", "prds/alpha.md")];
        let resolution = resolve_reference("PRD-404", &documents);
        assert_eq!(resolution_code(&resolution), "not_found");
        assert_eq!(resolution.canonical_key(), None);
    }

    #[test]
    fn an_unusable_selector_is_reported_as_such_and_not_as_a_miss() {
        let documents = vec![candidate("prd-alpha-k0123", "PRD-001", "Alpha", "prds/alpha.md")];
        // A qualified cross-source reference is not a local selector.
        let resolution = resolve_reference("prd-alpha-k0123@other-source", &documents);
        assert_eq!(
            resolution_code(&resolution),
            "invalid_selector",
            "an unusable reference must be distinguishable from a miss"
        );
    }

    #[test]
    fn candidates_are_described_by_key_title_and_relative_path() {
        let candidates = vec![candidate(
            "prd-alpha-k0123",
            "PRD-001",
            "Alpha",
            "prds/alpha.md",
        )];
        let described = describe_candidates(&candidates);
        assert_eq!(described.len(), 1);
        assert!(described[0].contains("prd-alpha-k0123"));
        assert!(described[0].contains("Alpha"));
        assert!(
            described[0].contains("prds/alpha.md"),
            "the relative path must be part of the description"
        );
        assert!(
            !described[0].contains('/'),
            "the description must never carry an absolute path"
        );
    }

    #[test]
    fn legacy_summaries_become_candidates_keyed_by_their_canonical_key() {
        let summaries = vec![ArtifactSummary {
            canonical_key: "prd-alpha-k0123".to_string(),
            id: "PRD-001".to_string(),
            title: "Alpha".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
        }];
        let documents = candidates_from_summaries(&summaries);
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].canonical_key, "prd-alpha-k0123");
        assert_eq!(documents[0].display_id, "PRD-001");
        // The legacy index does not record paths, so the field stays empty
        // rather than being guessed at.
        assert!(documents[0].relative_path.as_os_str().is_empty());
    }

    #[test]
    fn two_documents_claiming_one_key_is_its_own_outcome_not_an_ambiguity() {
        // Two documents hold the SAME canonical key. The name is not shared
        // — the identity is — so this is a conflict, not a choice.
        let documents = vec![
            candidate("prd-alpha-k0123", "PRD-001", "Alpha", "prds/alpha.md"),
            candidate("prd-alpha-k0123", "PRD-002", "Alpha again", "prds/alpha2.md"),
        ];
        let resolution = resolve_reference("prd-alpha-k0123", &documents);
        assert_eq!(
            resolution_code(&resolution),
            "conflicting_canonical_key",
            "a contested key must be distinguishable from a shared alias"
        );
        match &resolution {
            Resolution::ConflictingCanonicalKey { key, candidates } => {
                assert_eq!(key, "prd-alpha-k0123");
                assert_eq!(
                    candidates.len(),
                    2,
                    "every claimant must be named, or the conflict hides one"
                );
            }
            other => panic!("expected a key conflict, got {other:?}"),
        }
    }

    #[test]
    fn a_resolved_endpoint_names_the_one_document_it_answers() {
        let documents = vec![
            candidate("prd-alpha-k0123", "PRD-001", "Alpha", "prds/alpha.md"),
            candidate("prd-beta-k0456", "PRD-002", "Beta", "prds/beta.md"),
        ];
        let endpoint = resolve_endpoint("PRD-002", &documents).expect("a unique alias resolves");
        match endpoint {
            Endpoint::Document {
                canonical_key,
                display_id,
            } => {
                assert_eq!(canonical_key, "prd-beta-k0456");
                assert_eq!(display_id, "PRD-002");
            }
            other => panic!("expected a resolved document, got {other:?}"),
        }
    }

    #[test]
    fn a_resolved_endpoint_for_a_missing_document_is_a_forward_reference() {
        let documents = vec![candidate("prd-alpha-k0123", "PRD-001", "Alpha", "prds/alpha.md")];
        let endpoint = resolve_endpoint("PRD-404", &documents).expect("a miss is a forward reference");
        match endpoint {
            Endpoint::Forward(declared) => {
                assert_eq!(
                    declared, "PRD-404",
                    "a forward reference keeps the declared text verbatim"
                );
            }
            other => panic!("expected a forward reference, got {other:?}"),
        }
    }

    #[test]
    fn a_resolved_endpoint_refuses_an_ambiguous_reference() {
        let documents = vec![
            candidate("prd-alpha-k0123", "PRD-001", "Alpha", "prds/alpha.md"),
            candidate("prd-beta-k0456", "PRD-001", "Beta", "prds/beta.md"),
        ];
        let error = resolve_endpoint("PRD-001", &documents)
            .expect_err("an ambiguous reference must not pick a winner");
        assert_eq!(
            resolution_code(&error),
            "ambiguous",
            "an ambiguity stays an error even where a forward reference is allowed"
        );
    }

    #[tokio::test]
    async fn a_workspace_selector_resolves_over_the_workspace_document_set() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let workspace = tmp.path();
        let prds = workspace.join("prds");
        std::fs::create_dir_all(&prds).expect("mkdir");
        std::fs::write(
            prds.join("PRD-001-alpha.md"),
            "---\nid: PRD-001\ntitle: Alpha\nslug: prd-alpha-k0123\nkind: prd\nstatus: draft\n---\n\nbody\n",
        )
        .expect("write");

        let resolution = resolve_in_workspace_files(workspace, "PRD-001").await;
        assert!(resolution.is_found(), "the document must resolve");
        assert_eq!(resolution.canonical_key(), Some("prd-alpha-k0123"));
        assert_eq!(resolution.display_id(), Some("PRD-001"));
    }

    #[tokio::test]
    async fn a_selector_two_workspace_documents_share_is_an_ambiguity_over_the_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let workspace = tmp.path();
        let prds = workspace.join("prds");
        std::fs::create_dir_all(&prds).expect("mkdir");
        for (file, slug) in [("PRD-001-alpha.md", "prd-alpha-k0123"), ("PRD-001-beta.md", "prd-beta-k0456")] {
            std::fs::write(
                prds.join(file),
                format!("---\nid: PRD-001\ntitle: T\nslug: {slug}\nkind: prd\nstatus: draft\n---\n\nbody\n"),
            )
            .expect("write");
        }

        let resolution = resolve_in_workspace_files(workspace, "PRD-001").await;
        assert_eq!(
            resolution_code(&resolution),
            "ambiguous",
            "two documents sharing one display id must not resolve to a winner"
        );
    }
}
