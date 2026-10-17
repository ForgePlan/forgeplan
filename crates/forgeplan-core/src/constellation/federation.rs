//! Stage 1 — federated reads over explicitly named `.forgeplan` sources.
//!
//! Two ideas from the architecture document drive everything here:
//!
//! * **Path-only resolution.** A caller names a `.forgeplan` directory and
//!   gets answers. Nothing here opens a `LanceStore`, runs migrations, or
//!   fires project hooks — the same boundary `resolve_workspace_dir` gives
//!   the MCP Constellation tools, applied to more than one root.
//! * **An honest envelope.** Every result names the source it came from and
//!   the version the answer was read at — the pinned generation when a
//!   Constellation index exists, the captured source-set digest when it
//!   does not. An unavailable source is reported as unavailable, never as
//!   zero matches.
//!
//! Reading an unindexed workspace falls back to parsing the canonical
//! markdown directly through [`capture_source`]. That path is slower than a
//! pinned table and carries no `semantic_state` coverage, and the envelope
//! says so rather than pretending otherwise.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::capture::{CaptureLimits, CaptureRequest, capture_source};
use super::{ConstellationError, sha256_hex};

/// A `.forgeplan` directory named by the caller — the unit of federation.
///
/// The path must exist, canonicalize without crossing a symlink, and look
/// like a Forgeplan workspace (at least one artifact directory). A path
/// that fails any of those is a per-source diagnostic, not a fatal error.
#[derive(Debug, Clone)]
pub struct SourceSpec {
    pub path: PathBuf,
}

impl SourceSpec {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

/// Identity of a source inside one answer — stable for a canonical path.
///
/// `source_id` is a digest of the canonical `.forgeplan` path, so the same
/// workspace reached through different spellings reports one identity and
/// two genuinely different checkouts report two — the ambiguity the spec
/// names (`ADR-001` in two projects must never merge).
#[derive(Debug, Clone, Serialize)]
pub struct SourceRef {
    pub source_id: String,
    pub path: String,
    /// `pinned` — answered from a published generation; `markdown` — read
    /// from canonical bytes because no generation exists.
    pub channel: SourceChannel,
    /// The version the answer was read at: a generation id when pinned, the
    /// capture's `sourceSetDigest` when reading markdown directly.
    pub version_key: String,
}

/// Which read channel produced a source's rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceChannel {
    /// Opened the published Constellation tables at the manifest's pinned
    /// versions — immutable, verified.
    Pinned,
    /// Parsed the canonical markdown on disk. Reflects files as captured
    /// now; never claims the durability guarantees of a published index.
    Markdown,
    /// Read from pinned Git objects (`git show` at a recorded OID) —
    /// stage-3 snapshot channel. As immutable as `pinned`, without needing
    /// a published Constellation index at all.
    PinnedGit,
}

impl SourceChannel {
    /// The short label CLI/MCP output prints per hit.
    pub fn channel_str(&self) -> &'static str {
        match self {
            Self::Pinned => "pinned",
            Self::Markdown => "markdown",
            Self::PinnedGit => "git",
        }
    }
}

/// Per-source outcome when the source could not be read.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct SourceFailure {
    /// The path as the caller spelled it — there may be no canonical form.
    pub path: String,
    pub reason: String,
}

/// One search hit, stamped with the source that produced it.
#[derive(Debug, Clone, Serialize)]
pub struct FederatedHit {
    pub source: SourceRef,
    pub canonical_key: String,
    pub display_id: Option<String>,
    pub title: String,
    pub kind: Option<String>,
    pub status: Option<String>,
    /// Stored `semantic_state` for pinned rows; `None` on the markdown
    /// channel — an unindexed workspace has no vector coverage to report,
    /// and inventing `unavailable` would describe a queue that does not exist.
    pub semantic_state: Option<String>,
}

/// The envelope every federated read returns, success or partial.
#[derive(Debug)]
pub struct FederatedSearchOutcome {
    /// Sources that answered, with the version each answer was read at.
    pub sources: Vec<SourceRef>,
    /// Sources that did not. `partial` is true iff this is non-empty —
    /// "not read" is never folded into "no matches".
    pub failures: Vec<SourceFailure>,
    pub hits: Vec<FederatedHit>,
    pub truncated: bool,
    pub partial: bool,
}

/// The artifact a `Found` resolved to — boxed inside the enum so one
/// verbose variant does not inflate `Ambiguous` and `NotFound`.
#[derive(Debug)]
pub struct FederatedArtifact {
    pub source: SourceRef,
    pub canonical_key: String,
    pub display_id: Option<String>,
    pub title: String,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub body: String,
    pub relative_path: String,
    pub source_digest: String,
    pub semantic_state: Option<String>,
}

/// One artifact exactly, resolved against every named source.
#[derive(Debug)]
pub enum FederatedGet {
    /// Exactly one source holds this key — the envelope is unambiguous.
    Found(Box<FederatedArtifact>),
    /// More than one source claims the key. Names every claimer so the
    /// caller can qualify — never picks one silently.
    Ambiguous { sources: Vec<SourceRef> },
    /// No source holds the key. Sources that failed to answer still appear
    /// in `failures` — "not found" means "not found where we read".
    NotFound { failures: Vec<SourceFailure> },
}

// -- Resolution -------------------------------------------------------------

/// The `.forgeplan` directories capture iterates — imported lazily so this
/// module does not name a second artifact-dir contract.
fn looks_like_forgeplan_dir(dir: &Path) -> bool {
    crate::workspace::ARTIFACT_DIRS
        .iter()
        .any(|d| dir.join(d).is_dir())
}

/// Canonicalize a caller-named source without following a symlink boundary.
///
/// Same confinement rule `capture` applies at its root: a `.forgeplan`
/// spelled through an alias would read bytes outside the stated boundary,
/// so the alias is refused, not silently resolved.
fn resolve_source(spec: &SourceSpec) -> Result<PathBuf, SourceFailure> {
    let spelled = spec.path.display().to_string();
    let meta = std::fs::symlink_metadata(&spec.path).map_err(|e| SourceFailure {
        path: spelled.clone(),
        reason: format!("cannot stat the source root: {e}"),
    })?;
    if meta.file_type().is_symlink() {
        return Err(SourceFailure {
            path: spelled,
            reason: "the source root is a symlink; aliases are refused at the boundary".into(),
        });
    }
    if !meta.is_dir() {
        return Err(SourceFailure {
            path: spelled,
            reason: "source root is not a directory".into(),
        });
    }
    let canonical = spec.path.canonicalize().map_err(|e| SourceFailure {
        path: spelled.clone(),
        reason: format!("cannot canonicalize the source root: {e}"),
    })?;
    if !looks_like_forgeplan_dir(&canonical) {
        return Err(SourceFailure {
            path: spelled,
            reason: format!(
                "{} does not look like a .forgeplan workspace (no artifact directories)",
                canonical.display()
            ),
        });
    }
    Ok(canonical)
}

pub(crate) fn source_id_for(canonical: &Path) -> String {
    format!(
        "src:{}",
        &sha256_hex(canonical.to_string_lossy().as_bytes())[..16]
    )
}

/// The [`SourceRef::source_id`] a spec will report, if it resolves.
///
/// CLI uses this to match a user qualifier (`@src:ab12…` or a path
/// fragment) against named sources *before* reading — resolution failures
/// mean `None`, and the qualifier match then happens on what was spelled.
pub fn spec_source_id(spec: &SourceSpec) -> Option<String> {
    resolve_source(spec).ok().map(|c| source_id_for(&c))
}

// -- Qualified references (`KEY@qualifier`) ------------------------------------

/// Split a qualified reference into its artifact key and its source
/// qualifier: `ADR-001@src:b7d6…` → `("ADR-001", Some("src:b7d6…"))`.
///
/// The qualifier exists so a key claimed by two sources can be narrowed
/// to one — `ADR-001` in two projects must never merge. The split is
/// deliberately *syntactic only*: it accepts the first `@` as the
/// separator and returns no qualifier when either side would be empty,
/// so `a@`, `@b`, and a bare `ADR-001` all come back as
/// `(input, None)`. Which characters a qualifier may contain when the
/// ref is *written* is a separate, stricter question — see
/// [`crate::db::store::validate_link_target`].
///
/// A second `@` lands inside the qualifier (`a@b@c` → `("a",
/// Some("b@c"))`) — callers that store the ref validate the charset;
/// callers that only *match* it against sources can live with a
/// qualifier that will simply never match.
pub fn parse_qualified_ref(input: &str) -> (&str, Option<&str>) {
    match input.split_once('@') {
        Some((key, qualifier)) if !key.is_empty() && !qualifier.is_empty() => {
            (key, Some(qualifier))
        }
        _ => (input, None),
    }
}

/// The sources a `@qualifier` narrows to.
///
/// Two readings, the same pair `registry::find_source` uses for rebind
/// selectors and the read commands use for `KEY@q`:
///
/// * the qualifier is a **fragment of the source's spelled path** —
///   `@ws-b` names the source whose path contains `ws-b`;
/// * the qualifier **is or prefixes the source's live `src:` id** —
///   `@src:b7d6` names the source whose id begins that way.
///
/// [`spec_source_id`] resolves the id live, so a source that cannot be
/// read right now still answers to its path fragment but not to its
/// id — the qualifier-miss diagnostic is the caller's job; this helper
/// only answers "which of these match". Zero matches is a legitimate
/// answer, not an error.
pub fn specs_matching<'a>(specs: &'a [SourceSpec], qualifier: &str) -> Vec<&'a SourceSpec> {
    specs
        .iter()
        .filter(|spec| {
            spec.path.display().to_string().contains(qualifier)
                || spec_source_id(spec)
                    .as_deref()
                    .is_some_and(|sid| sid == qualifier || sid.starts_with(qualifier))
        })
        .collect()
}

// -- Per-source read paths ----------------------------------------------------

async fn read_pinned(
    canonical: &Path,
    query: &str,
    limit: usize,
) -> Result<(SourceRef, Vec<FederatedHit>), ConstellationError> {
    let index_root = canonical.join("lance");
    let outcome = super::facade::search_pinned_lexical(&index_root, query, limit).await?;
    let source = SourceRef {
        source_id: source_id_for(canonical),
        path: canonical.display().to_string(),
        channel: SourceChannel::Pinned,
        version_key: outcome.generation_id.clone(),
    };
    let hits = outcome
        .hits
        .into_iter()
        .map(|h| FederatedHit {
            source: source.clone(),
            canonical_key: h.canonical_key,
            display_id: h.display_id,
            title: h.title,
            kind: h.kind,
            status: h.status,
            semantic_state: h.semantic_state,
        })
        .collect();
    Ok((source, hits))
}

async fn read_markdown(
    canonical: &Path,
    query: &str,
    limit: usize,
) -> Result<(SourceRef, Vec<FederatedHit>), ConstellationError> {
    let captured =
        capture_source(&CaptureRequest::new(canonical), CaptureLimits::default()).await?;
    let source = SourceRef {
        source_id: source_id_for(canonical),
        path: canonical.display().to_string(),
        channel: SourceChannel::Markdown,
        version_key: captured.source_set_digest.clone(),
    };
    let needle = query.to_lowercase();
    let mut hits = Vec::new();
    'outer: for artifact in &captured.artifacts {
        if artifact.canonical_key.to_lowercase().contains(&needle)
            || artifact.display_id.to_lowercase().contains(&needle)
            || artifact.title.to_lowercase().contains(&needle)
            || artifact.body.to_lowercase().contains(&needle)
        {
            hits.push(FederatedHit {
                source: source.clone(),
                canonical_key: artifact.canonical_key.clone(),
                display_id: Some(artifact.display_id.clone()),
                title: artifact.title.clone(),
                kind: Some(artifact.kind.clone()),
                status: Some(artifact.status.clone()),
                semantic_state: None,
            });
            if hits.len() >= limit {
                break 'outer;
            }
        }
    }
    Ok((source, hits))
}

/// Whether a source has a *published* generation to pin to. Three states:
///
/// * no `lance/` at all → markdown channel, clearly;
/// * `lance/` exists but no `CURRENT.json` → the index directory was
///   created (e.g. by `init`) but never published; the markdown channel is
///   still the honest answer, not an error — nothing was ever born there;
/// * `CURRENT.json` present → the pinned channel; a failure to *read* it
///   is a genuine index fault and surfaces as a source failure, not a
///   silent downgrade to markdown.
fn has_published_index(canonical: &Path) -> bool {
    canonical
        .join("lance")
        .join(super::storage::CURRENT_FILE_NAME)
        .is_file()
}

// -- The facade ---------------------------------------------------------------

/// Search every named source at whatever honest channel it offers.
///
/// A source with a published Constellation index answers from the pinned
/// generation; a source without one is parsed from canonical markdown.
/// Sources are queried in order, hits are interleaved as returned, and
/// `partial` tells the caller some sources did not answer — the spec's
/// rule that an unreadable source must not read as an empty corpus.
pub async fn federated_search(
    specs: &[SourceSpec],
    query: &str,
    limit: usize,
) -> Result<FederatedSearchOutcome, ConstellationError> {
    if limit == 0 {
        return Err(ConstellationError::Io {
            detail: "a federated search with limit 0 cannot return anything".to_string(),
        });
    }
    let mut sources = Vec::new();
    let mut failures = Vec::new();
    let mut hits = Vec::new();
    for spec in specs {
        let canonical = match resolve_source(spec) {
            Ok(c) => c,
            Err(f) => {
                failures.push(f);
                continue;
            }
        };
        // A present-but-broken index is a failure the caller sees, not a
        // silent downgrade to the markdown channel: reporting markdown
        // hits under a "pinned" banner would hide that the index rotted.
        let result = if has_published_index(&canonical) {
            read_pinned(&canonical, query, limit).await
        } else {
            read_markdown(&canonical, query, limit).await
        };
        match result {
            Ok((source, source_hits)) => {
                sources.push(source);
                hits.extend(source_hits);
                if hits.len() >= limit {
                    break;
                }
            }
            Err(e) => failures.push(SourceFailure {
                path: spec.path.display().to_string(),
                reason: format!("{e}"),
            }),
        }
    }
    let truncated = hits.len() > limit;
    hits.truncate(limit);
    Ok(FederatedSearchOutcome {
        partial: !failures.is_empty(),
        sources,
        failures,
        hits,
        truncated,
    })
}

/// Read one canonical key across every named source.
///
/// Two checkouts may legitimately carry the same `ADR-001`; `Ambiguous`
/// names them all and the caller qualifies the ref. Sources that failed to
/// answer appear on `NotFound::failures` — on `Found` and `Ambiguous` the
/// caller pairs this read with [`federated_search`]'s envelope, which is
/// where per-source failures live.
pub async fn federated_get(
    specs: &[SourceSpec],
    key_or_id: &str,
) -> Result<FederatedGet, ConstellationError> {
    let mut found: Vec<(SourceRef, super::facade::PinnedArtifact)> = Vec::new();
    let mut failures = Vec::new();
    for spec in specs {
        let canonical = match resolve_source(spec) {
            Ok(c) => c,
            Err(f) => {
                failures.push(f);
                continue;
            }
        };
        if has_published_index(&canonical) {
            match super::facade::get_pinned_artifact(&canonical.join("lance"), key_or_id).await {
                Ok(Some((artifact, generation_id, _))) => {
                    found.push((
                        SourceRef {
                            source_id: source_id_for(&canonical),
                            path: canonical.display().to_string(),
                            channel: SourceChannel::Pinned,
                            version_key: generation_id,
                        },
                        artifact,
                    ));
                }
                Ok(None) => {}
                Err(e) => failures.push(SourceFailure {
                    path: spec.path.display().to_string(),
                    reason: format!("{e}"),
                }),
            }
        } else {
            match capture_source(&CaptureRequest::new(&canonical), CaptureLimits::default()).await {
                Ok(captured) => {
                    if let Some(artifact) = captured
                        .artifacts
                        .iter()
                        .find(|a| {
                            a.canonical_key.eq_ignore_ascii_case(key_or_id)
                                || a.display_id.eq_ignore_ascii_case(key_or_id)
                        })
                        .cloned()
                    {
                        let source = SourceRef {
                            source_id: source_id_for(&canonical),
                            path: canonical.display().to_string(),
                            channel: SourceChannel::Markdown,
                            version_key: captured.source_set_digest.clone(),
                        };
                        found.push((
                            source,
                            super::facade::PinnedArtifact {
                                canonical_key: artifact.canonical_key.clone(),
                                display_id: Some(artifact.display_id.clone()),
                                title: artifact.title.clone(),
                                kind: Some(artifact.kind.clone()),
                                status: Some(artifact.status.clone()),
                                body: Some(artifact.body.to_string()),
                                source_path: Some(artifact.relative_path.clone()),
                                source_digest: Some(artifact.source_digest.clone()),
                                semantic_state: None,
                            },
                        ));
                    }
                }
                Err(e) => failures.push(SourceFailure {
                    path: spec.path.display().to_string(),
                    reason: format!("{e}"),
                }),
            }
        }
    }
    if found.len() > 1 {
        return Ok(FederatedGet::Ambiguous {
            sources: found.iter().map(|(source, _)| source.clone()).collect(),
        });
    }
    if let Some((source, artifact)) = found.into_iter().next() {
        return Ok(FederatedGet::Found(Box::new(FederatedArtifact {
            source,
            canonical_key: artifact.canonical_key,
            display_id: artifact.display_id,
            title: artifact.title,
            kind: artifact.kind,
            status: artifact.status,
            body: artifact.body.unwrap_or_default(),
            relative_path: artifact.source_path.unwrap_or_default(),
            source_digest: artifact.source_digest.unwrap_or_default(),
            semantic_state: artifact.semantic_state,
        })));
    }
    Ok(FederatedGet::NotFound { failures })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::capture::CaptureLimits;
    use crate::constellation::facade::{MaintainCapability, maintain_index, resume_admission};
    use crate::workspace::ARTIFACT_DIRS;
    use tempfile::TempDir;

    /// A `.forgeplan` directory with the given `(dir, filename, markdown)`
    /// artifacts. The tempdir itself plays the `.forgeplan` role — same
    /// convention the facade tests use.
    fn workspace_with(files: &[(&str, &str, &str)]) -> TempDir {
        let tmp = TempDir::new().unwrap();
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        for (dir, name, body) in files {
            std::fs::write(tmp.path().join(dir).join(name), body).unwrap();
        }
        tmp
    }

    fn artifact(id: &str, kind: &str, title: &str, body: &str) -> String {
        format!("---\nid: {id}\nkind: {kind}\nstatus: draft\ntitle: {title}\n---\n\n{body}\n")
    }

    async fn publish(tmp: &TempDir) {
        resume_admission();
        let capability =
            MaintainCapability::for_strict_maintenance(tmp.path().canonicalize().unwrap());
        maintain_index(
            &capability,
            CaptureLimits::default(),
            std::time::Instant::now() + std::time::Duration::from_secs(120),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn search_answers_two_sources_on_their_own_channels() {
        // One indexed source answers from its pinned generation; the other
        // has no index and answers from parsed markdown. Both hits carry a
        // source identity that tells the two apart.
        let indexed = workspace_with(&[(
            "adrs",
            "ADR-010-cache.md",
            &artifact("ADR-010", "adr", "Cache policy", "sharedterm lives here"),
        )]);
        publish(&indexed).await;
        let markdown = workspace_with(&[(
            "prds",
            "PRD-010-ingest.md",
            &artifact("PRD-010", "prd", "Ingest", "sharedterm appears too"),
        )]);
        // A read must not create files, locks or markers in the source —
        // spec stage 1: "без изменений исходников". Snapshot the tree.
        let listing_before: Vec<String> = walkdir::WalkDir::new(markdown.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.path().display().to_string())
            .collect();

        let outcome = federated_search(
            &[
                SourceSpec::new(indexed.path()),
                SourceSpec::new(markdown.path()),
            ],
            "sharedterm",
            10,
        )
        .await
        .unwrap();

        assert!(!outcome.partial, "{:?}", outcome.failures);
        assert_eq!(outcome.sources.len(), 2);
        assert_eq!(outcome.hits.len(), 2);
        let mut channels: Vec<SourceChannel> =
            outcome.hits.iter().map(|h| h.source.channel).collect();
        channels.sort_by_key(|c| *c as u8);
        assert_eq!(
            channels,
            vec![SourceChannel::Pinned, SourceChannel::Markdown]
        );
        assert_ne!(
            outcome.hits[0].source.source_id, outcome.hits[1].source.source_id,
            "two workspaces must never collapse into one source identity"
        );
        // The markdown channel's version key is a capture digest, not a
        // generation id — callers can tell which channel answered.
        let md_hit = outcome
            .hits
            .iter()
            .find(|h| h.source.channel == SourceChannel::Markdown)
            .unwrap();
        assert!(!md_hit.source.version_key.is_empty());
        assert_eq!(md_hit.semantic_state, None);

        let listing_after: Vec<String> = walkdir::WalkDir::new(markdown.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.path().display().to_string())
            .collect();
        assert_eq!(
            listing_before, listing_after,
            "a federated read must not write into the source workspace"
        );
    }

    #[tokio::test]
    async fn an_unreadable_source_is_reported_not_folded_into_empty() {
        let good = workspace_with(&[(
            "specs",
            "SPEC-001-a.md",
            &artifact("SPEC-001", "spec", "Spec", "needle"),
        )]);

        let outcome = federated_search(
            &[
                SourceSpec::new(good.path()),
                SourceSpec::new("/definitely/not/a/workspace"),
            ],
            "needle",
            10,
        )
        .await
        .unwrap();

        assert!(outcome.partial);
        assert_eq!(outcome.failures.len(), 1);
        assert_eq!(outcome.hits.len(), 1);
        assert_eq!(outcome.hits[0].canonical_key, "SPEC-001");
    }

    #[tokio::test]
    async fn get_is_ambiguous_when_two_sources_claim_one_key() {
        // Two checkouts legitimately carrying ADR-001 must never merge —
        // the caller sees both claimers and qualifies the ref.
        let a = workspace_with(&[(
            "adrs",
            "ADR-001-a.md",
            &artifact("ADR-001", "adr", "Alpha decision", "alpha body"),
        )]);
        let b = workspace_with(&[(
            "adrs",
            "ADR-001-b.md",
            &artifact("ADR-001", "adr", "Beta decision", "beta body"),
        )]);

        let result = federated_get(
            &[SourceSpec::new(a.path()), SourceSpec::new(b.path())],
            "ADR-001",
        )
        .await
        .unwrap();

        match result {
            FederatedGet::Ambiguous { sources } => {
                assert_eq!(sources.len(), 2);
                assert_ne!(sources[0].source_id, sources[1].source_id);
            }
            other => panic!("two claimers must be ambiguous, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_reads_a_key_through_the_markdown_channel() {
        let tmp = workspace_with(&[(
            "notes",
            "NOTE-001-field.md",
            &artifact("NOTE-001", "note", "Field note", "the exact bytes"),
        )]);

        let result = federated_get(&[SourceSpec::new(tmp.path())], "NOTE-001")
            .await
            .unwrap();

        match result {
            FederatedGet::Found(artifact) => {
                assert_eq!(artifact.source.channel, SourceChannel::Markdown);
                assert!(artifact.body.contains("the exact bytes"));
                assert_eq!(artifact.semantic_state, None);
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_prefers_the_pinned_generation_when_it_exists() {
        let tmp = workspace_with(&[(
            "adrs",
            "ADR-020-pinned.md",
            &artifact("ADR-020", "adr", "Pinned", "stored body"),
        )]);
        publish(&tmp).await;

        let result = federated_get(&[SourceSpec::new(tmp.path())], "ADR-020")
            .await
            .unwrap();

        match result {
            FederatedGet::Found(artifact) => {
                assert_eq!(artifact.source.channel, SourceChannel::Pinned);
                assert!(!artifact.source.version_key.is_empty());
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_symlinked_source_root_is_refused() {
        let real = workspace_with(&[(
            "adrs",
            "ADR-030-a.md",
            &artifact("ADR-030", "adr", "Real", "body"),
        )]);
        // The alias lives inside its own tempdir — the shared system temp
        // dir would leak a stale `alias-link` between runs.
        let holder = TempDir::new().unwrap();
        let link = holder.path().join("alias-link");
        std::os::unix::fs::symlink(real.path(), &link).unwrap();

        let outcome = federated_search(&[SourceSpec::new(&link)], "body", 10)
            .await
            .unwrap();

        assert!(outcome.partial);
        assert_eq!(outcome.failures.len(), 1);
        assert!(outcome.failures[0].reason.contains("symlink"));
        assert!(outcome.hits.is_empty());
    }

    // -- Qualified refs (`KEY@qualifier`) --------------------------------------

    #[test]
    fn parse_qualified_ref_splits_on_the_first_at() {
        assert_eq!(
            parse_qualified_ref("ADR-001@src:b7d6f0e1a2c3d4e5"),
            ("ADR-001", Some("src:b7d6f0e1a2c3d4e5"))
        );
        assert_eq!(
            parse_qualified_ref("ADR-001@ws-b"),
            ("ADR-001", Some("ws-b"))
        );
        // No `@` — the input is the whole key.
        assert_eq!(parse_qualified_ref("ADR-001"), ("ADR-001", None));
        // Empty halves are not a qualifier — the ref comes back whole.
        assert_eq!(parse_qualified_ref("ADR-001@"), ("ADR-001@", None));
        assert_eq!(parse_qualified_ref("@ws-b"), ("@ws-b", None));
        // A second `@` is qualifier text, not a second separator.
        assert_eq!(parse_qualified_ref("a@b@c"), ("a", Some("b@c")));
    }

    /// A forgeplan-shaped dir at an explicit path — `workspace_with` cannot
    /// name its tempdir, and path-fragment matching needs a known fragment.
    fn workspace_at(path: &Path) {
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(path.join(dir)).unwrap();
        }
    }

    #[test]
    fn specs_matching_narrows_by_id_prefix_and_path_fragment() {
        let holder = TempDir::new().unwrap();
        let ws_a = holder.path().join("ws-alpha");
        let ws_b = holder.path().join("ws-beta");
        workspace_at(&ws_a);
        workspace_at(&ws_b);
        let specs = vec![SourceSpec::new(&ws_a), SourceSpec::new(&ws_b)];

        // Exact `src:` id — the full id names exactly one source.
        let id_a = spec_source_id(&specs[0]).expect("ws-a resolves");
        let id_b = spec_source_id(&specs[1]).expect("ws-beta resolves");
        assert_ne!(id_a, id_b);
        let by_id = specs_matching(&specs, &id_a);
        assert_eq!(by_id.len(), 1);
        assert_eq!(by_id[0].path, specs[0].path);

        // A strict prefix of the id still narrows to one.
        let prefix = &id_a[..id_a.len() - 4];
        assert!(prefix.starts_with("src:"));
        let by_prefix = specs_matching(&specs, prefix);
        assert_eq!(by_prefix.len(), 1);
        assert_eq!(by_prefix[0].path, specs[0].path);

        // A path fragment names the source the string way.
        let by_path = specs_matching(&specs, "ws-beta");
        assert_eq!(by_path.len(), 1);
        assert_eq!(by_path[0].path, specs[1].path);

        // A fragment shared by both paths matches both — ambiguity is the
        // caller's diagnostic, not this helper's.
        assert_eq!(specs_matching(&specs, "ws-").len(), 2);

        // Zero matches is an answer, not an error.
        assert!(specs_matching(&specs, "no-such-source").is_empty());
    }
}
