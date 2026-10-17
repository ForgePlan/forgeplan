//! The narrow surface CLI and MCP call.
//!
//! Everything a caller needs to *read* goes through [`capture_for_read`] and
//! [`ensure_read_view`], and nothing gives a caller a way to write an index
//! row without presenting a [`MaintainCapability`]. That asymmetry is the
//! protocol: a read handler that could reach the publisher would, on a fresh
//! checkout, start creating tables from inside a query — which is how a read
//! path acquires migrations and hidden model downloads it was never supposed
//! to have.
//!
//! ADR-016's resolution chain stays the single source of truth for *which*
//! workspace is addressed. This module never re-implements it; the
//! [`ResolutionPolicy`] below only records which branch of that chain
//! produced the workspace path.
//!
//! # What is here and what is not
//!
//! Present: the two read boundaries and [`shutdown_runtime`].
//!
//! Absent, deliberately: `maintain_index` and the publisher behind it. That
//! function needs the staged-row path — the `merge_insert` / `Table::delete` /
//! `UpdateBuilder` commits, the head comparison and the conditional restore —
//! and none of that exists yet. A `maintain_index` that fabricated a
//! generation manifest and pointed `CURRENT.json` at it would publish an
//! index whose rows were never written, and every reader after it would serve
//! them as current. The absence is reported rather than filled.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::ConstellationError;
use super::capture::{CaptureLimits, CaptureRequest, CapturedSourceView, capture_source};
use super::manifest::{CurrentDescriptor, GenerationManifest};
use super::storage::{self, IndexProtocolState};
use super::view::ReadView;

/// Whether this process still admits maintenance.
///
/// Read paths are unaffected: a query must keep answering during a shutdown,
/// because refusing reads would turn a graceful stop into an outage.
static ADMITTING: AtomicBool = AtomicBool::new(true);

/// Which branch of the ADR-016 resolution chain produced the workspace path.
///
/// `SoftFallback` is the read policy: it never escalates to a maintain
/// capability and never errors on multi-worktree detection, because a read
/// that cannot determine a workspace still has an honest answer to give. It
/// is not a permission level — it does not unlock writing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionPolicy {
    /// Read path. No owner, no `WorkspaceLock`, no index writes, no
    /// migrations, no model download.
    SoftFallback,
    /// Mutation path. The resolver errors instead of guessing.
    Strict,
}

/// Proof that the caller came through the resolver's Strict branch and is
/// allowed to maintain one physical index.
///
/// Opaque on purpose: a caller cannot construct its way to a write by
/// assembling a struct literal, and the token names the one index root it is
/// good for. SoftFallback never receives one.
///
/// # Why it carries the workspace, not just the index root
///
/// It previously stored only an index root, and `maintain_index` recovered
/// the workspace with `index_root.parent()`. That inference is unsound: nothing
/// established that the root was `<workspace>/lance`, so a root that was not a
/// direct child of a workspace made every capture in that function read a
/// different tree — silently, because the capture would simply have found
/// fewer artifacts.
///
/// So the relationship is **established here and nowhere else**: the root is
/// *derived* from the workspace at construction, and neither field can be
/// supplied independently. `maintain_index` never infers one from the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintainCapability {
    workspace: PathBuf,
    index_root: PathBuf,
}

impl MaintainCapability {
    /// Directory name of the derived index, under a workspace's `.forgeplan`.
    pub const INDEX_DIR: &'static str = "lance";

    /// Issued by the resolver's Strict branch for one **workspace**.
    pub fn for_strict_maintenance(workspace: impl Into<PathBuf>) -> Self {
        let workspace = workspace.into();
        let index_root = workspace.join(Self::INDEX_DIR);
        Self {
            workspace,
            index_root,
        }
    }

    /// The workspace whose canonical markdown this capability may reconcile.
    ///
    /// This is what capture reads. It is a field, never a `parent()` of the
    /// index root.
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    /// The single index root this capability covers.
    ///
    /// Derived from [`Self::workspace`] at construction, so a capability can
    /// never name an index that does not belong to the workspace it reads.
    /// A capability that could name several roots is one that could let a
    /// caller publish into a worktree it never resolved.
    pub fn index_root(&self) -> &Path {
        &self.index_root
    }
}

/// Read-only capture for CLI and MCP handlers.
///
/// Takes no owner, no `WorkspaceLock`, writes nothing — not to the index, not
/// to the derived tables, not to canonical markdown — and never triggers
/// maintenance or a model download.
///
/// `previous` lets a caller keep `incarnation` and `row_revision` moving
/// across calls. Passing `None` observes every artifact as new, which is
/// correct for a first read and wrong for a steady-state one.
///
/// # Errors
///
/// [`ConstellationError::SourceBusy`] when the source is moving; the caller
/// may return `index_pending` with honest lexical coverage rather than
/// pretending the previous generation is current.
pub async fn capture_for_read(
    workspace: &Path,
    previous: Option<Arc<CapturedSourceView>>,
    limits: CaptureLimits,
) -> Result<ReadView, ConstellationError> {
    let mut request = CaptureRequest::new(workspace);
    if let Some(previous) = previous {
        request = request.with_previous(previous);
    }
    let captured = capture_source(&request, limits).await?;
    Ok(ReadView::from_capture(Arc::new(captured)))
}

/// Read-only freshness barrier: persisted view when the published manifest
/// still matches the capture, ephemeral otherwise.
///
/// Takes no owner, no `WorkspaceLock`, writes nothing, and waits for no
/// owner. It is **not** reconcile persistence: matching digests only decide
/// *which kind* of view the caller gets, never whether to publish one.
///
/// * No pointer at all → ephemeral. A first run must answer, and answering
///   must not create anything.
/// * Pointer present, but the captured bytes or config moved → ephemeral.
///   The published generation describes a different source set, so serving it
///   would be stale and persisting the new one from a read would be a write.
/// * Pointer present and matching → the published generation, after its
///   manifest and checksum are verified. A pointer that does not resolve is
///   not a fast path.
///
/// # Errors
///
/// [`ConstellationError::IndexGenerationUnavailable`] when a pointer exists
/// but its manifest is missing or its checksum disagrees. That is never
/// downgraded to an ephemeral view: "I could not read what was published" and
/// "nothing has been published" are different facts, and only one of them
/// should be answered with fresh bytes.
pub async fn ensure_read_view(
    workspace: &Path,
    captured: Arc<CapturedSourceView>,
) -> Result<ReadView, ConstellationError> {
    let index_root = workspace.join("lance");
    let descriptor = storage::read_current(&index_root)?;

    let Some(descriptor) = descriptor else {
        return Ok(ReadView::from_capture(captured));
    };

    if descriptor.source_set_digest != captured.source_set_digest
        || descriptor.config_digest != captured.config_digest
    {
        return Ok(ReadView::from_capture(captured));
    }

    let manifest = Arc::new(storage::read_manifest(&index_root, &descriptor)?);
    if manifest.source_set_digest != descriptor.source_set_digest {
        return Err(ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "manifest {} disagrees with CURRENT about the source set",
                descriptor.manifest_path
            ),
        });
    }
    Ok(ReadView::from_manifest(manifest))
}

/// A pinned view: exact table versions, nothing newer.
///
/// There is deliberately no `open_pinned_view_at_latest` and no
/// retry-on-stale variant. A reader that refreshed itself would be answering a
/// different question than the one that pinned it, and the one that pinned it
/// is the one whose answer the caller is going to trust.
#[derive(Debug, Clone)]
pub struct PinnedVectorView {
    pub descriptor: CurrentDescriptor,
    pub manifest: Arc<GenerationManifest>,
    pub artifacts_version: u64,
    pub relations_version: u64,
}

/// Open the generation a descriptor names, at its **exact** versions.
///
/// # Errors
///
/// [`ConstellationError::IndexGenerationUnavailable`] when the manifest is
/// missing, fails its checksum, names a different generation than `CURRENT`,
/// or names a pair with a missing half. Never a fallback to latest.
pub fn open_pinned_view(
    index_root: &Path,
    descriptor: &CurrentDescriptor,
) -> Result<PinnedVectorView, ConstellationError> {
    let manifest = Arc::new(storage::read_manifest(index_root, descriptor)?);

    if manifest.generation_id != descriptor.generation_id {
        return Err(ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "CURRENT names `{}` but its manifest is `{}`",
                descriptor.generation_id, manifest.generation_id
            ),
        });
    }
    if !manifest.table_versions.is_publishable() {
        return Err(ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "generation `{}` names table versions {:?}; a coherent pair needs both",
                manifest.generation_id, manifest.table_versions
            ),
        });
    }

    Ok(PinnedVectorView {
        descriptor: descriptor.clone(),
        artifacts_version: manifest.table_versions.artifacts,
        relations_version: manifest.table_versions.relations,
        manifest,
    })
}

/// Stop admitting maintenance.
///
/// Admission stops first; joins come next; the owner is released last. The
/// function gets that ordering for free by not holding an owner itself:
/// `acquire_owner` returns a guard scoped to a single maintenance pass, so by
/// the time this runs no guard of ours is live.
///
/// Reads are unaffected — a query keeps answering during a graceful stop,
/// because refusing reads would turn a shutdown into an outage.
///
/// A model soft timeout does **not** kill a native call. That belongs to the
/// inference worker, and it is why shutdown joins rather than detaches: a
/// detached worker still holding publisher capability would let a
/// shutting-down process publish.
pub fn shutdown_runtime() -> Result<(), ConstellationError> {
    ADMITTING.store(false, Ordering::SeqCst);
    // Retention stays off across a restart too: nothing here removes a
    // generation, a table version, or a lock inode.
    storage::assert_retention_policy()
}

/// What one explicit embedding batch achieved.
///
/// The fields exist so that "embedded 3" can never be confused with
/// "embedded 0 because 4 could not be judged". Both are the same count of
/// successes over a different total, and only `eligible_keys` beside
/// `cannot_judge` tells them apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingBatchOutcome {
    /// The profile every vector was published against. Resolved here, never
    /// supplied by a caller.
    pub profile_id: String,
    /// The work list: keys whose vector was not current.
    pub eligible_keys: Vec<String>,
    /// Vectors that were written.
    pub published: Vec<String>,
    /// Keys whose completion matched no row — the row moved. **Not** a
    /// failure; it is the expected outcome of a stale worker.
    pub stale: Vec<String>,
    /// Rows whose eligibility could not be judged, with the reason.
    pub cannot_judge: Vec<(String, String)>,
}

impl EmbeddingBatchOutcome {
    /// Whether every captured row's eligibility was decided.
    pub fn is_complete(&self) -> bool {
        self.cannot_judge.is_empty()
    }
}

/// The engine this build embeds with.
///
/// Identifies the **engine**, not the crate: it must change whenever the
/// `tract-onnx` dependency or the pooling/normalisation it performs changes,
/// because vectors from two engines are not interchangeable.
///
/// Lives here rather than in `embed` because it is part of the *profile
/// identity* and must travel with it; a caller must never supply it, for the
/// same reason it must never supply a `profile_id`.
///
/// Gated with the only body that uses it, so the default build does not warn
/// on a constant it cannot act on.
#[cfg(feature = "semantic-search")]
pub const ENGINE_VERSION: &str = "tract-onnx-0.23";

/// The text each artifact contributes to its vector, for every key the capture
/// can bind.
///
/// **Read-only. No owner, no tables, no store write.** The text is the
/// **capture's own** data — [`ParsedArtifact::title`] and
/// [`ParsedArtifact::body`] — run through
/// [`embedding_input_text`](super::jobs::embedding_input_text), which is
/// exactly the string the tokenizer receives. Nothing here reads the store,
/// and a caller that could read the store would be a caller that could write
/// it.
///
/// Separated from [`embed_with_publisher`] because the vectors depend on this
/// text and the **model must not run under the owner guard**. Splitting them
/// is what keeps model compute outside every lock.
///
/// # Superset, deliberately
///
/// It returns an input for every bindable captured row rather than only the
/// rows the index says are stale. Over-computing is safe and under-computing
/// is not: a vector computed for an already-current row is rejected by
/// [`embed_with_publisher`]'s guard and reported in `stale`, whereas a row
/// wrongly omitted is never embedded at all.
///
/// # Errors
///
/// [`ConstellationError::SourceBusy`] when the capture cannot be confirmed.
#[cfg(feature = "semantic-search")]
pub async fn eligible_inputs(
    workspace: &Path,
) -> Result<Vec<(String, String)>, ConstellationError> {
    use super::publisher::capture_under_workspace_lock;

    let captured =
        capture_under_workspace_lock(workspace, super::capture::CaptureLimits::default()).await?;

    let prefix_chars = crate::workspace::init::load_config(workspace)
        .ok()
        .and_then(|config| config.embedding.map(|embedding| embedding.chunk_size))
        .unwrap_or(super::jobs::DEFAULT_BODY_PREFIX_CHARS);

    Ok(captured
        .artifacts
        .iter()
        // A row with no input digest cannot be bound, so it cannot be
        // published; returning its text would produce work that can only be
        // refused.
        .filter(|artifact| !artifact.input_digest.is_empty())
        .map(|artifact| {
            (
                artifact.canonical_key.clone(),
                super::jobs::embedding_input_text(&artifact.title, &artifact.body, prefix_chars),
            )
        })
        .collect())
}

/// Publish explicitly-computed vectors against the captured source.
///
/// The caller **computes** — explicit `embed` is one of the three permitted
/// model obtainers — and this **writes**, which is where the binding matters.
/// The illegitimacy is never a caller computing; it is a caller writing.
///
/// Takes a **workspace**, like `maintain_index`, and never an
/// `&IndexTables` and never a `profile_id`: both would put the identity
/// decision back in the caller's hands.
///
/// # Ordering
///
/// Owner → profile resolution → capture → open tables → eligibility →
/// publish → release. **One** store, **one** tracker, **one** connection set
/// for the whole run; opening tables twice is a bug, not a style choice,
/// because two trackers would each see half the closure.
///
/// It does not duplicate `maintain_index`: that reconciles rows from a
/// capture, this publishes vectors a caller explicitly computed.
#[cfg(feature = "semantic-search")]
pub async fn embed_with_publisher(
    workspace: &Path,
    vectors: &BTreeMap<String, Vec<f32>>,
) -> Result<EmbeddingBatchOutcome, ConstellationError> {
    use super::jobs::{DEFAULT_MODEL_REPO, EmbeddingPublish, snapshot_checksums};
    use super::lance_tables::open_active_tables;
    use super::publisher::{
        IndexPublisher, capture_under_workspace_lock, identity_for, publish_embeddings,
    };

    // 1. Owner first, before anything else.
    let index_root = workspace.join("lance");
    let publisher = IndexPublisher::acquire(&index_root, inner_store_for(&index_root))?;

    // 2. Profile resolved HERE, never supplied. The cache directory comes from
    //    the embed module's own resolver (`embed/mod.rs:85`), which honours
    //    `FORGEPLAN_MODEL_CACHE` then the platform user cache; a caller-chosen
    //    path would be a caller choosing which weights to trust.
    let cache_dir = crate::embed::resolve_cache_dir();
    let checksums = snapshot_checksums(&cache_dir, DEFAULT_MODEL_REPO)?;
    let profile = super::jobs::ResolvedProfile::open(
        &cache_dir,
        DEFAULT_MODEL_REPO,
        &checksums,
        ENGINE_VERSION,
    )?;
    let profile_id = profile.profile.profile_id();

    // 3. Capture under the workspace lock, released before any write.
    let captured =
        capture_under_workspace_lock(workspace, super::capture::CaptureLimits::default()).await?;

    let identity = identity_for(&index_root);
    let marker = super::manifest::ProtocolMarker {
        protocol_version: super::FORMAT_VERSION,
        state: super::manifest::BootstrapState::Active,
        index_epoch: identity.index_epoch(),
        checkout_id: identity.checkout_id().to_string(),
        table_names: super::manifest::ProtocolMarker::active_table_names(),
        storage_scheme: super::manifest::STORAGE_SCHEME_FILE_OBJECT_STORE.to_string(),
        bootstrap_attempt: 1,
    };

    // 4. One store, one tracker, one connection set. `tracking_store()` lends a
    //    reference; `open_active_tables` takes ownership, so the `Arc` is
    //    cloned — the same instance, one strong count added, not a new store
    //    and therefore not a second tracker.
    let tables =
        open_active_tables(&index_root, &marker, Arc::clone(publisher.tracking_store())).await?;

    // 5. What needs doing, judged against the captured binding.
    let eligibility = tables.artifacts_eligibility(&captured, &profile_id).await?;
    let eligible_keys: Vec<String> = eligibility
        .eligible_keys()
        .into_iter()
        .map(str::to_string)
        .collect();

    // 6. Publish. A key absent from the capture is refused inside
    //    `publish_embeddings`, not here.
    let results: Vec<EmbeddingPublish> =
        publish_embeddings(&tables, &captured, &profile_id, vectors).await?;

    // 7. Release: tables first, then the owner with the publisher.
    drop(tables);
    drop(publisher);

    Ok(EmbeddingBatchOutcome {
        profile_id,
        eligible_keys,
        published: results
            .iter()
            .filter(|outcome| outcome.published())
            .map(|outcome| outcome.binding.canonical_key.clone())
            .collect(),
        stale: results
            .iter()
            .filter(|outcome| !outcome.published())
            .map(|outcome| outcome.binding.canonical_key.clone())
            .collect(),
        cannot_judge: eligibility
            .cannot_judge()
            .into_iter()
            .map(|(key, reason)| (key.to_string(), reason.to_string()))
            .collect(),
    })
}

/// Feature-off is `unavailable`, not `failed`.
///
/// This arm acquires **nothing** — no owner, no tables, no tracker — and does
/// no partial work, because a caller seeing "0 embedded" would reasonably
/// conclude work had been attempted. It does not report success either:
/// `ModelUnavailable` is the state SPEC-008 defines for a missing feature,
/// and naming the feature is what lets a caller tell the user what to do.
#[cfg(not(feature = "semantic-search"))]
pub async fn embed_with_publisher(
    _workspace: &Path,
    _vectors: &BTreeMap<String, Vec<f32>>,
) -> Result<EmbeddingBatchOutcome, ConstellationError> {
    Err(ConstellationError::ModelUnavailable {
        detail: "this build was compiled without the `semantic-search` feature, so no vector can \
                 be produced. This is `unavailable`, not a failed embedding."
            .to_string(),
    })
}

/// Whether this process still admits maintenance.
pub fn is_admitting() -> bool {
    ADMITTING.load(Ordering::SeqCst)
}

/// One commit that ran, as reported to the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitSummary {
    /// `merge`, `column_update` or `delete`.
    pub kind: &'static str,
    pub version: u64,
    /// Rows affected. `0` is a legitimate outcome — "matched nothing" is not
    /// the same as "the commit failed", and the two are told apart here.
    pub rows_affected: u64,
}

/// What one maintenance pass did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileOutcome {
    pub progress: super::publisher::BootstrapProgress,
    pub generation_id: String,
    pub table_versions: super::manifest::TableVersions,
    pub commits: Vec<CommitSummary>,
    /// Populated when a stage ran and was verified. `None` when nothing was
    /// staged — a real outcome, and `commits` says which it was.
    pub verification: Option<super::publisher::VerificationReport>,
    /// Head versions **after** any conditional restore. The pre-restore pair
    /// is never what gets published.
    pub head_versions: super::manifest::TableVersions,
}

/// A published generation, as the caller receives it.
#[derive(Debug, Clone)]
pub struct PublishedReadView {
    pub descriptor: CurrentDescriptor,
    pub manifest: Arc<GenerationManifest>,
}

/// Run one steady-state maintenance pass.
///
/// Reachable only with a [`MaintainCapability`], which only the resolver's
/// Strict branch issues. [`ResolutionPolicy::SoftFallback`] never receives
/// one, so a read cannot reach this and cannot cause a table write.
///
/// # Guard discipline
///
/// * The **owner** spans recover → capture → stage → verify → publish and is
///   released when the publisher drops. It is never held across model
///   loading, inference or a queue wait — none of which this function does.
/// * The **WorkspaceLock** is taken once per storage commit and released
///   before the next, in owner → workspace order. Never across two commits,
///   never acquired before the owner.
/// * An expired deadline is checked **before** any work starts. Once work has
///   begun it is not enforced mid-commit: the guard is held until every child
///   operation finishes and the caller is told `index_busy`. Releasing a lock
///   on timeout would leave a Lance commit unsupervised.
///
/// # The two-connections assumption
///
/// `verify_staged` opens a **separate** reader through
/// `lance_tables::open_table_at_version` while the writers are still open, so
/// two connections to one index exist at once. That is presumed fine on the
/// `file-object-store` scheme and is **untested**. The reader is deliberately
/// **not** wrapped to make the two look symmetric: a wrapped reader shares
/// the writer's `ObjectStore` instance and could observe in-flight writes,
/// which is exactly what a pinned reader must not do.
///
/// # Errors
///
/// [`ConstellationError::IndexBusy`] when another live process owns the
/// index, or when the deadline had already expired. A `publication_uncertain`
/// surfaces as itself — never a rollback, never a success. A
/// `durability_unavailable` from the closure barrier stops before any
/// generation or CURRENT write, because the barrier runs inside `publish`
/// before either.
pub async fn maintain_index(
    capability: &MaintainCapability,
    capture_limits: super::capture::CaptureLimits,
    deadline: std::time::Instant,
) -> Result<(PublishedReadView, ReconcileOutcome), ConstellationError> {
    use super::publisher::{
        HeadComparison, IndexPublisher, artifacts_batch, capture_under_workspace_lock,
        compare_heads, plan_commits, stage_sequence,
    };

    if !is_admitting() {
        return Err(ConstellationError::IndexBusy {
            detail: "this process is shutting down and admits no new maintenance".to_string(),
        });
    }
    if std::time::Instant::now() >= deadline {
        // Refused before anything starts, so no guard exists to leak.
        return Err(ConstellationError::IndexBusy {
            detail: "the maintenance deadline had already expired; no work was started".to_string(),
        });
    }

    // Both paths come from the capability as fields. Nothing here infers one
    // from the other — that inference was unsound and is gone.
    let index_root = capability.index_root().to_path_buf();
    let workspace = capability.workspace().to_path_buf();

    let publisher = IndexPublisher::acquire(&index_root, inner_store_for(&index_root))?;

    // Bootstrap on a first run, or recover on a restart. Either leaves the
    // index with a published generation or an error.
    let progress =
        super::publisher::bootstrap_or_recover(&publisher, &workspace, capture_limits).await?;

    let descriptor = super::storage::read_current(&index_root)?.ok_or_else(|| {
        ConstellationError::IndexGenerationUnavailable {
            detail: "maintenance finished without a published pointer".to_string(),
        }
    })?;
    let published = Arc::new(super::storage::read_manifest(&index_root, &descriptor)?);

    // Recover the baseline. Matching heads restore nothing and commit
    // nothing — the rule that stops a pointless commit before every stage.
    let marker = marker_for(&published);
    let tables = super::lance_tables::open_active_tables(
        &index_root,
        &marker,
        std::sync::Arc::clone(publisher.tracking_store()),
    )
    .await?;

    match compare_heads(
        Some(&published),
        tables.artifacts_version().await?,
        tables.relations_version().await?,
    ) {
        HeadComparison::Matches => {}
        HeadComparison::Diverged {
            artifacts_diverged,
            relations_diverged,
        } => {
            // Only the affected table is rewound. Each restore creates a NEW
            // physical head, so versions are re-read afterwards and that
            // post-restore pair is what gets recorded.
            if artifacts_diverged {
                tables
                    .restore_artifacts_to(published.table_versions.artifacts)
                    .await?;
            }
            if relations_diverged {
                tables
                    .restore_relations_to(published.table_versions.relations)
                    .await?;
            }
        }
        HeadComparison::MissingPublishedVersion { role } => {
            return Err(ConstellationError::IndexGenerationUnavailable {
                detail: format!(
                    "the published {role} version is gone; there is no latest fallback"
                ),
            });
        }
    }
    // Re-read after any restore: what is recorded is what exists now, never
    // the pre-restore numbers.
    let head_versions = tables.head_versions().await?;

    // Capture under the workspace lock, released before the first commit.
    let captured = capture_under_workspace_lock(&workspace, capture_limits).await?;
    let reconcile = super::reconciler::plan_reconcile(&captured, Some(&published));
    let commits_plan = plan_commits(&reconcile);
    // Tombstones name artifacts a complete capture proved absent — their
    // whole edge set goes. Changed artifacts get their outgoing edges
    // replaced: delete everything the key owns, then merge the new set.
    let delete_keys: Vec<String> = reconcile
        .tombstones
        .iter()
        .map(|t| t.canonical_key.clone())
        .collect();
    let changed_keys: std::collections::BTreeSet<String> = reconcile
        .deltas
        .iter()
        .filter(|delta| {
            matches!(
                delta.action,
                super::reconciler::DeltaAction::Create | super::reconciler::DeltaAction::Update
            )
        })
        .filter_map(|delta| delta.canonical_key.clone())
        .collect();
    let relations_delete_keys: Vec<String> = changed_keys
        .iter()
        .cloned()
        .chain(delete_keys.iter().cloned())
        .collect();
    let changed_artifacts: Vec<Arc<super::capture::ParsedArtifact>> = captured
        .artifacts
        .iter()
        .filter(|artifact| changed_keys.contains(&artifact.canonical_key))
        .cloned()
        .collect();
    let edge_batch = super::publisher::relations_batch(&changed_artifacts)?;
    let relations_plan = super::publisher::RelationsPlan {
        delete_keys: relations_delete_keys,
        merge_rows: edge_batch.num_rows(),
    };
    let sequence = stage_sequence(
        &commits_plan,
        captured.artifacts.len(),
        delete_keys,
        &relations_plan,
    );

    let mut commits: Vec<CommitSummary> = Vec::new();
    let mut verification: Option<super::publisher::VerificationReport> = None;
    let mut committed_versions = head_versions;

    if !sequence.is_empty() {
        let batch = artifacts_batch(
            &captured.artifacts,
            captured.index_epoch,
            &captured.checkout_id,
        )?;

        for op in &sequence.ops {
            // One lock per commit, released before the next. Never across two.
            let guard = crate::workspace::acquire_workspace_lock(&workspace)
                .await
                .map_err(|e| ConstellationError::Io {
                    detail: format!("cannot take the workspace lock for a commit: {e}"),
                })?;
            match op {
                super::publisher::CommitOp::MergeUpsert {
                    full_source_batch, ..
                } => {
                    let outcome = tables
                        .merge_into_artifacts(std::sync::Arc::clone(&batch), *full_source_batch)
                        .await?;
                    commits.push(CommitSummary {
                        kind: "merge",
                        version: outcome.version,
                        rows_affected: outcome.num_inserted_rows
                            + outcome.num_updated_rows
                            + outcome.num_deleted_rows,
                    });
                }
                super::publisher::CommitOp::ColumnUpdate { keys, assignments } => {
                    let outcome = tables.update_artifacts_columns(keys, assignments).await?;
                    commits.push(CommitSummary {
                        kind: "column_update",
                        version: outcome.version,
                        rows_affected: outcome.rows_updated,
                    });
                }
                super::publisher::CommitOp::DeleteByPredicate { keys } => {
                    let outcome = tables.delete_from_artifacts(keys).await?;
                    commits.push(CommitSummary {
                        kind: "delete",
                        version: outcome.version,
                        rows_affected: outcome.rows_deleted,
                    });
                }
                super::publisher::CommitOp::RelationsDelete { keys } => {
                    let outcome = tables.delete_from_relations(keys).await?;
                    commits.push(CommitSummary {
                        kind: "relations_delete",
                        version: outcome.version,
                        rows_affected: outcome.rows_deleted,
                    });
                }
                super::publisher::CommitOp::RelationsMerge { .. } => {
                    let outcome = tables
                        .merge_into_relations(std::sync::Arc::clone(&edge_batch), false)
                        .await?;
                    commits.push(CommitSummary {
                        kind: "relations_merge",
                        version: outcome.version,
                        rows_affected: outcome.num_inserted_rows
                            + outcome.num_updated_rows
                            + outcome.num_deleted_rows,
                    });
                }
            }
            drop(guard);
        }

        // The versions the manifest will pin must be the ones the commits
        // produced — re-read after the last commit. Pinning the pre-commit
        // pair would publish a generation whose readers open versions that
        // do not carry the rows it claims.
        committed_versions = tables.head_versions().await?;

        // Read back through a SEPARATE reader while the writers are still
        // open. Two connections at once is presumed fine and is untested; the
        // reader stays unwrapped so it cannot see in-flight writes.
        let expected: std::collections::BTreeMap<String, String> = captured
            .entries
            .iter()
            .map(|entry| (entry.canonical_key.clone(), entry.source_digest.clone()))
            .collect();
        let expected_edges: usize = captured
            .artifacts
            .iter()
            .map(|artifact| artifact.outgoing.len())
            .sum();
        verification = Some(
            super::publisher::verify_staged(
                &index_root,
                &marker,
                committed_versions.artifacts,
                &expected,
                committed_versions.relations,
                expected_edges,
            )
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("post-commit verification failed: {e}"),
            })?,
        );
    }

    // A pass that committed nothing publishes nothing: the pointer, the
    // manifest directory and the fsyncs all stay where they are, and the
    // caller gets the already-published generation back. Publishing an
    // identical manifest under a new id on every pass would churn the
    // generations directory for no benefit.
    if sequence.is_empty() {
        return Ok((
            PublishedReadView {
                descriptor,
                manifest: Arc::clone(&published),
            },
            ReconcileOutcome {
                progress,
                generation_id: published.generation_id.clone(),
                table_versions: head_versions,
                commits,
                verification: None,
                head_versions,
            },
        ));
    }

    // Final source validation, under the workspace lock, before the pointer
    // moves. The digests compared are the ones the staging was computed
    // against — `captured`'s, not the previously published manifest's:
    // comparing against `published.source_set_digest` would refuse every
    // real update, since a changed source is exactly what a publish exists
    // to record. Generation/epoch still come from the published pointer:
    // those catch the CURRENT having moved under us.
    let recheck = capture_under_workspace_lock(&workspace, capture_limits).await?;
    super::publisher::candidate_is_current(
        Some(published.generation_id.as_str()),
        Some(descriptor.generation_id.as_str()),
        published.index_epoch,
        descriptor.index_epoch,
        captured.source_set_digest.as_str(),
        recheck.source_set_digest.as_str(),
        captured.config_digest.as_str(),
        recheck.config_digest.as_str(),
    )?;
    drop(recheck);

    let generation_id = super::publisher::generation_id_for(&captured.evaluated_at);
    let mut next = (*published).clone();
    next.generation_id = generation_id.clone();
    next.source_entries = captured.entries.clone();
    next.coverage = captured.coverage;
    next.evaluated_at = captured.evaluated_at.clone();
    next.table_versions = committed_versions;

    // finish_writes → sync_written_set → write_generation_manifest →
    // descriptor_for(real owner epoch) → publish_current, in that order.
    publisher.publish(&next)?;
    drop(tables);
    // The owner is released when `publisher` drops at the end of this scope.
    // Waking readers is P4's runtime; nothing here has one to signal yet.

    let new_descriptor =
        super::storage::read_current(&index_root)?.ok_or_else(|| ConstellationError::Io {
            detail: "publication left no pointer".to_string(),
        })?;
    let new_manifest = Arc::new(super::storage::read_manifest(&index_root, &new_descriptor)?);

    Ok((
        PublishedReadView {
            descriptor: new_descriptor,
            manifest: Arc::clone(&new_manifest),
        },
        ReconcileOutcome {
            progress,
            generation_id,
            table_versions: committed_versions,
            commits,
            verification,
            head_versions,
        },
    ))
}

/// What one embedding-queue drain did, as the caller reports it.
#[derive(Debug)]
pub struct EmbeddingQueueOutcome {
    /// The generation the vectors were published onto.
    pub generation_id: String,
    /// The profile the queue ran under.
    pub profile_id: String,
    /// Eligibility *before* the drain — what the index asked for.
    pub before: super::jobs::EligibilityReport,
    /// Vectors that landed (rows actually updated, not merely attempted).
    pub embedded: usize,
    /// Completions that arrived stale — the row moved between eligibility
    /// and publish, and the conditional update matched zero rows. Counted,
    /// not errored: stale is information, not failure.
    pub stale_completions: usize,
    /// Eligibility *after* the drain — the caller-visible coverage proof.
    pub after: super::jobs::EligibilityReport,
}

/// Drain the embedding queue for one workspace, under one profile.
///
/// This is the 0B "queue in serve/watch" runner. The model itself is not in
/// core — `embed` is injected by the caller (`forgeplan-cli` passes the
/// `semantic-search` embedder), so the core stays model-agnostic and the
/// whole fence/eligibility machinery is testable without weights.
///
/// Order of operations, and why:
///
/// 1. The index must already exist and be publishable — the queue is a
///    writer onto rows, never a bootstrapper.
/// 2. `artifacts_eligibility` answers "which rows need a vector for this
///    profile" by scalar scan of the nine stored binding fields — never a
///    vector query (see [`super::jobs::eligibility_for`]).
/// 3. Vectors are computed **before** any publish lock is taken: inference
///    is the slow part and the workspace lock is a writer-serialisation
///    primitive, not a semaphore for the model.
/// 4. Each publish takes the workspace lock for exactly one conditional
///    update and releases it — the same one-lock-per-commit rule the merge
///    stage follows. The predicate is the row's own stored binding, so a
///    completion that arrives after the row moved updates zero rows; that
///    is counted in `stale_completions`, not retried here.
/// 5. Eligibility is re-read at the end so the outcome reports coverage the
///    index actually holds, not coverage the queue hoped to produce.
///
/// `body_prefix_chars` bounds how much body goes into the embedding input —
/// the same bound the captured `input_digest` was computed over, so a caller
/// that changes it changes the digests and thereby correctly re-eligibilises
/// every row.
pub async fn drain_embedding_queue(
    capability: &MaintainCapability,
    profile_id: &str,
    body_prefix_chars: usize,
    mut embed: impl FnMut(&[(String, String)]) -> Result<Vec<Vec<f32>>, ConstellationError>,
    deadline: std::time::Instant,
) -> Result<EmbeddingQueueOutcome, ConstellationError> {
    if !is_admitting() {
        return Err(ConstellationError::IndexBusy {
            detail: "this process is shutting down and admits no new maintenance".to_string(),
        });
    }
    if std::time::Instant::now() >= deadline {
        return Err(ConstellationError::IndexBusy {
            detail: "the queue deadline had already expired; no work was started".to_string(),
        });
    }

    let index_root = capability.index_root().to_path_buf();
    let workspace = capability.workspace().to_path_buf();

    // (1) The queue never bootstraps: an index that is not yet publishable is
    // `maintain_index`'s job, and silently creating one here would hide the
    // distinction in exactly the wrong place.
    let descriptor = super::storage::read_current(&index_root)?.ok_or_else(|| {
        ConstellationError::IndexGenerationUnavailable {
            detail: "no published generation; run index maintenance first".to_string(),
        }
    })?;
    let published = Arc::new(super::storage::read_manifest(&index_root, &descriptor)?);
    if !published.table_versions.is_publishable() {
        return Err(ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "generation `{}` names non-publishable table versions {:?}",
                published.generation_id, published.table_versions
            ),
        });
    }

    let publisher =
        super::publisher::IndexPublisher::acquire(&index_root, inner_store_for(&index_root))?;
    let marker = marker_for(&published);
    let tables = super::lance_tables::open_active_tables(
        &index_root,
        &marker,
        std::sync::Arc::clone(publisher.tracking_store()),
    )
    .await?;

    // The capture is under the workspace lock and the lock is released
    // before any write — same rule as the merge stage.
    let captured = super::publisher::capture_under_workspace_lock(
        &workspace,
        super::capture::CaptureLimits::default(),
    )
    .await?;

    // (2) What the index asks for, before we spend a model call.
    let before = tables.artifacts_eligibility(&captured, profile_id).await?;

    let work_keys: Vec<String> = before
        .eligible_keys()
        .iter()
        .map(|key| (*key).to_string())
        .collect();

    let mut vectors: std::collections::BTreeMap<String, Vec<f32>> =
        std::collections::BTreeMap::new();
    if !work_keys.is_empty() {
        // (3) Inference before any publish lock. Inputs are built from the
        // same captured bytes the binding digests came from, so the vector
        // and its binding can never drift apart.
        let mut inputs: Vec<(String, String)> = Vec::with_capacity(work_keys.len());
        for key in &work_keys {
            let Some(artifact) = captured
                .artifacts
                .iter()
                .find(|artifact| &artifact.canonical_key == key)
            else {
                // Eligible but not captured: leave it for the next drain and
                // let the `after` report say so.
                continue;
            };
            inputs.push((
                key.clone(),
                super::jobs::embedding_input_text(
                    &artifact.title,
                    &artifact.body,
                    body_prefix_chars,
                ),
            ));
        }

        const EMBED_BATCH: usize = 32;
        for chunk in inputs.chunks(EMBED_BATCH) {
            if std::time::Instant::now() >= deadline {
                return Err(ConstellationError::IndexBusy {
                    detail: "the queue deadline expired mid-inference;                              computed vectors were not published"
                        .to_string(),
                });
            }
            let batch = embed(chunk)?;
            if batch.len() != chunk.len() {
                return Err(ConstellationError::ModelUnavailable {
                    detail: format!(
                        "the embedder returned {} vectors for {} inputs",
                        batch.len(),
                        chunk.len()
                    ),
                });
            }
            for ((key, _), vector) in chunk.iter().zip(batch.into_iter()) {
                vectors.insert(key.clone(), vector);
            }
        }
    }

    // (4) One lock per publish, released before the next. `publish_embeddings`
    // refuses a key absent from the capture and the row-side predicate does
    // the staleness work, so a vector is never installed onto a moved row.
    let mut embedded = 0usize;
    let mut stale_completions = 0usize;
    for (key, vector) in vectors {
        if std::time::Instant::now() >= deadline {
            return Err(ConstellationError::IndexBusy {
                detail: format!(
                    "the queue deadline expired after {embedded} publishes;                      remaining vectors are dropped, not staged"
                ),
            });
        }
        let guard = crate::workspace::acquire_workspace_lock(&workspace)
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("cannot take the workspace lock for a vector publish: {e}"),
            })?;
        let mut single = std::collections::BTreeMap::new();
        single.insert(key, vector);
        for outcome in
            super::publisher::publish_embeddings(&tables, &captured, profile_id, &single).await?
        {
            if outcome.stale {
                stale_completions += 1;
            } else {
                embedded += 1;
            }
        }
        drop(guard);
    }

    // (5) Coverage as the index holds it, not as the queue hoped.
    let after = tables.artifacts_eligibility(&captured, profile_id).await?;

    drop(tables);
    Ok(EmbeddingQueueOutcome {
        generation_id: published.generation_id.clone(),
        profile_id: profile_id.to_string(),
        before,
        embedded,
        stale_completions,
        after,
    })
}

/// One lexical hit at the pinned artifacts version.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LexicalHit {
    pub canonical_key: String,
    pub display_id: Option<String>,
    pub title: String,
    pub kind: Option<String>,
    pub status: Option<String>,
    /// The row's stored `semantic_state` as-is — reported honestly, never
    /// interpreted: `ready`/`pending`/`failed`/`unavailable`/absent are all
    /// legal, and the caller sees which.
    pub semantic_state: Option<String>,
}

/// What a lexical search over the pinned projection returned.
#[derive(Debug)]
pub struct LexicalSearchOutcome {
    pub generation_id: String,
    pub pinned_artifacts_version: u64,
    pub rows_scanned: usize,
    pub truncated: bool,
    pub hits: Vec<LexicalHit>,
}

/// Case-insensitive substring search over the artifacts table **at the
/// version the published manifest pins** — never latest.
///
/// This is the read-side of the 0A/0B query surface: it opens an independent
/// pinned reader, scans scalar columns, and matches on `canonical_key`,
/// `title` and `body`. It is deliberately lexical — the spec's semantic
/// search is gated on profile-bound vectors, and this function never
/// pretends substring matching is similarity. `hits` carry the stored
/// `semantic_state` verbatim so a caller can tell coverage apart from
/// absence.
///
/// Reads only: no locks, no maintenance trigger, no writes of any kind.
pub async fn search_pinned_lexical(
    index_root: &Path,
    query: &str,
    limit: usize,
) -> Result<LexicalSearchOutcome, ConstellationError> {
    if limit == 0 {
        return Err(ConstellationError::Io {
            detail: "a lexical search with limit 0 cannot return anything".to_string(),
        });
    }
    let descriptor = super::storage::read_current(index_root)?.ok_or_else(|| {
        ConstellationError::IndexGenerationUnavailable {
            detail: "no published generation to search".to_string(),
        }
    })?;
    let manifest = super::storage::read_manifest(index_root, &descriptor)?;
    if !manifest.table_versions.is_publishable() {
        return Err(ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "generation `{}` names non-publishable table versions {:?}",
                manifest.generation_id, manifest.table_versions
            ),
        });
    }
    let marker = marker_for(&manifest);
    let table = super::lance_tables::open_table_at_version(
        index_root,
        &marker,
        "artifacts",
        manifest.table_versions.artifacts,
    )
    .await?;
    let batches = table.query_batches().await?;

    let read_str = |batch: &arrow_array::RecordBatch, col: &str, row: usize| -> Option<String> {
        use arrow_array::Array;
        let column = batch.column_by_name(col)?;
        if row >= column.len() || column.is_null(row) {
            return None;
        }
        // `body` is LargeUtf8, the rest are Utf8 — read both.
        if let Some(a) = column.as_any().downcast_ref::<arrow_array::StringArray>() {
            return Some(a.value(row).to_string());
        }
        column
            .as_any()
            .downcast_ref::<arrow_array::LargeStringArray>()
            .map(|a| a.value(row).to_string())
    };

    let needle = query.to_lowercase();
    let mut hits: Vec<LexicalHit> = Vec::new();
    let mut rows_scanned = 0usize;
    'outer: for batch in &batches {
        for row in 0..batch.num_rows() {
            rows_scanned += 1;
            let key = read_str(batch, "canonical_key", row).unwrap_or_default();
            let title = read_str(batch, "title", row).unwrap_or_default();
            let body = read_str(batch, "body", row).unwrap_or_default();
            if !key.to_lowercase().contains(&needle)
                && !title.to_lowercase().contains(&needle)
                && !body.to_lowercase().contains(&needle)
            {
                continue;
            }
            hits.push(LexicalHit {
                canonical_key: key,
                display_id: read_str(batch, "id", row),
                title,
                kind: read_str(batch, "kind", row),
                status: read_str(batch, "status", row),
                semantic_state: read_str(batch, "semantic_state", row),
            });
            if hits.len() >= limit {
                break 'outer;
            }
        }
    }

    Ok(LexicalSearchOutcome {
        generation_id: manifest.generation_id.clone(),
        pinned_artifacts_version: manifest.table_versions.artifacts,
        rows_scanned,
        truncated: hits.len() >= limit,
        hits,
    })
}

/// One semantic hit at the pinned generation.
#[derive(Debug, serde::Serialize)]
pub struct SemanticHit {
    pub canonical_key: String,
    pub display_id: Option<String>,
    pub title: String,
    /// Cosine similarity derived from lance's `_distance`: `1.0 - distance`.
    /// Reported as a score, never as a fake rank position.
    pub similarity: f64,
}

/// What a semantic search over the pinned projection returned.
#[derive(Debug)]
pub struct SemanticSearchOutcome {
    pub generation_id: String,
    pub pinned_artifacts_version: u64,
    /// How many stored vectors were eligible to rank — ready under *this*
    /// profile. Distinct from `hits.len()`: a corpus can hold zero ready
    /// rows under the profile and still contain plenty under another.
    pub eligible_rows: usize,
    pub hits: Vec<SemanticHit>,
}

/// ANN search over the artifacts table **at the version the published
/// manifest pins**, restricted to rows whose vectors were computed for
/// `profile_id` and reached `ready`.
///
/// The `query_vector` is produced by the caller — the core never picks or
/// loads a model — and its profile is the caller's claim. The prefilter
/// (`semantic_state = 'ready' AND profile_id = '<profile>'`) is what makes
/// that claim real: a vector stored under another profile never enters the
/// ranking, so a similarity number is never computed against a foreign
/// embedding space.
///
/// Returns `Ok(outcome)` even with an empty `hits` — "no vectors ready" is
/// honest coverage the client sees via `eligible_rows`, not an error.
pub async fn search_pinned_semantic(
    index_root: &Path,
    query_vector: &[f32],
    profile_id: &str,
    limit: usize,
) -> Result<SemanticSearchOutcome, ConstellationError> {
    if limit == 0 {
        return Err(ConstellationError::Io {
            detail: "a semantic search with limit 0 cannot return anything".to_string(),
        });
    }
    let descriptor = super::storage::read_current(index_root)?.ok_or_else(|| {
        ConstellationError::IndexGenerationUnavailable {
            detail: "no published generation to search".to_string(),
        }
    })?;
    let manifest = super::storage::read_manifest(index_root, &descriptor)?;
    if !manifest.table_versions.is_publishable() {
        return Err(ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "generation `{}` names non-publishable table versions {:?}",
                manifest.generation_id, manifest.table_versions
            ),
        });
    }
    let marker = marker_for(&manifest);
    let table = super::lance_tables::open_table_at_version(
        index_root,
        &marker,
        "artifacts",
        manifest.table_versions.artifacts,
    )
    .await?;

    let predicate = format!(
        "semantic_state = 'ready' AND profile_id = '{}'",
        super::publisher::escape_predicate_literal(profile_id)
    );
    let batches = table
        .nearest_embeddings(query_vector, &predicate, limit)
        .await?;

    let read_str = |batch: &arrow_array::RecordBatch, col: &str, row: usize| -> Option<String> {
        use arrow_array::Array;
        let column = batch.column_by_name(col)?;
        if row >= column.len() || column.is_null(row) {
            return None;
        }
        if let Some(a) = column.as_any().downcast_ref::<arrow_array::StringArray>() {
            return Some(a.value(row).to_string());
        }
        column
            .as_any()
            .downcast_ref::<arrow_array::LargeStringArray>()
            .map(|a| a.value(row).to_string())
    };
    let read_dist = |batch: &arrow_array::RecordBatch, row: usize| -> Option<f64> {
        use arrow_array::Array;
        let column = batch.column_by_name("_distance")?;
        if row >= column.len() || column.is_null(row) {
            return None;
        }
        column
            .as_any()
            .downcast_ref::<arrow_array::Float32Array>()
            .map(|a| f64::from(a.value(row)))
            .or_else(|| {
                column
                    .as_any()
                    .downcast_ref::<arrow_array::Float64Array>()
                    .map(|a| a.value(row))
            })
    };

    let mut hits = Vec::new();
    let mut eligible_rows = 0usize;
    for batch in &batches {
        eligible_rows += batch.num_rows();
        for row in 0..batch.num_rows() {
            hits.push(SemanticHit {
                canonical_key: read_str(batch, "canonical_key", row).unwrap_or_default(),
                display_id: read_str(batch, "id", row),
                title: read_str(batch, "title", row).unwrap_or_default(),
                similarity: read_dist(batch, row).map_or(0.0, |d| 1.0 - d),
            });
        }
    }
    hits.truncate(limit);

    Ok(SemanticSearchOutcome {
        generation_id: manifest.generation_id.clone(),
        pinned_artifacts_version: manifest.table_versions.artifacts,
        eligible_rows,
        hits,
    })
}

/// A single artifact row at the pinned generation, by canonical key or
/// display id — the `get` half of the read surface.
#[derive(Debug)]
pub struct PinnedArtifact {
    pub canonical_key: String,
    pub display_id: Option<String>,
    pub title: String,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub body: Option<String>,
    pub source_path: Option<String>,
    pub source_digest: Option<String>,
    pub semantic_state: Option<String>,
}

/// Exact read at the pinned generation: one row by `canonical_key`, or by
/// `display_id` when the caller hands over the human-facing id (`SPEC-001`).
///
/// Like the lexical search this opens an independent pinned reader — the row
/// returned is the row the published manifest guarantees, not whatever a
/// later commit may have landed. A miss is a miss: `Ok(None)`, never a
/// substitute row.
pub async fn get_pinned_artifact(
    index_root: &Path,
    key_or_id: &str,
) -> Result<Option<(PinnedArtifact, String, u64)>, ConstellationError> {
    if key_or_id.trim().is_empty() {
        return Err(ConstellationError::Io {
            detail: "an empty key cannot identify an artifact".to_string(),
        });
    }
    let descriptor = super::storage::read_current(index_root)?.ok_or_else(|| {
        ConstellationError::IndexGenerationUnavailable {
            detail: "no published generation to read".to_string(),
        }
    })?;
    let manifest = super::storage::read_manifest(index_root, &descriptor)?;
    if !manifest.table_versions.is_publishable() {
        return Err(ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "generation `{}` names non-publishable table versions {:?}",
                manifest.generation_id, manifest.table_versions
            ),
        });
    }
    let marker = marker_for(&manifest);
    let table = super::lance_tables::open_table_at_version(
        index_root,
        &marker,
        "artifacts",
        manifest.table_versions.artifacts,
    )
    .await?;
    let batches = table.query_batches().await?;

    let read_str = |batch: &arrow_array::RecordBatch, col: &str, row: usize| -> Option<String> {
        use arrow_array::Array;
        let column = batch.column_by_name(col)?;
        if row >= column.len() || column.is_null(row) {
            return None;
        }
        // `body` is LargeUtf8, the rest are Utf8 — read both.
        if let Some(a) = column.as_any().downcast_ref::<arrow_array::StringArray>() {
            return Some(a.value(row).to_string());
        }
        column
            .as_any()
            .downcast_ref::<arrow_array::LargeStringArray>()
            .map(|a| a.value(row).to_string())
    };

    let wanted = key_or_id.to_lowercase();
    for batch in &batches {
        for row in 0..batch.num_rows() {
            let key = read_str(batch, "canonical_key", row).unwrap_or_default();
            let display = read_str(batch, "id", row);
            if key.to_lowercase() != wanted
                && display.as_deref().map(str::to_lowercase).as_deref() != Some(wanted.as_str())
            {
                continue;
            }
            return Ok(Some((
                PinnedArtifact {
                    canonical_key: key,
                    display_id: display,
                    title: read_str(batch, "title", row).unwrap_or_default(),
                    kind: read_str(batch, "kind", row),
                    status: read_str(batch, "status", row),
                    body: read_str(batch, "body", row),
                    source_path: read_str(batch, "relative_path", row),
                    source_digest: read_str(batch, "source_digest", row),
                    semantic_state: read_str(batch, "semantic_state", row),
                },
                manifest.generation_id.clone(),
                manifest.table_versions.artifacts,
            )));
        }
    }
    Ok(None)
}

/// The inner object store the publisher's tracking store wraps.
///
/// The registry builds this for the index URI; constructing it here keeps the
/// wrapper paired with the index the caller actually resolved, rather than
/// with whatever a global happened to point at.
fn inner_store_for(index_root: &Path) -> Arc<dyn object_store::ObjectStore> {
    // Root-anchored, not index-root-prefixed: lance addresses every object
    // by the absolute path the `file-object-store` URI carries, so the inner
    // store's own prefix must be the filesystem root for keys and files to
    // agree. A `new_with_prefix(index_root)` store would double the root into
    // every location — and its keys are what the tracker records, so the
    // path model downstream assumes fs-root-relative strings.
    let _ = index_root;
    Arc::new(object_store::local::LocalFileSystem::new())
}
/// The protocol marker a reader or writer should be opened against.
fn marker_for(manifest: &GenerationManifest) -> super::manifest::ProtocolMarker {
    super::manifest::ProtocolMarker {
        protocol_version: super::FORMAT_VERSION,
        state: super::manifest::BootstrapState::Active,
        index_epoch: manifest.index_epoch,
        checkout_id: manifest.checkout_id.clone(),
        table_names: super::manifest::ProtocolMarker::active_table_names(),
        storage_scheme: super::manifest::STORAGE_SCHEME_FILE_OBJECT_STORE.to_string(),
        bootstrap_attempt: 1,
    }
}

/// Restore admission, for a supervisor that reuses the process.
pub fn resume_admission() {
    ADMITTING.store(true, Ordering::SeqCst)
}

/// What a restart found on disk, re-exported so a caller can report it
/// without importing the storage module directly.
pub type RestartState = IndexProtocolState;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::manifest::{
        BootstrapState, GenerationManifest, ProtocolMarker, STORAGE_SCHEME_FILE_OBJECT_STORE,
        TableVersions,
    };
    use crate::workspace::ARTIFACT_DIRS;
    use tempfile::TempDir;

    fn workspace() -> TempDir {
        let tmp = TempDir::new().unwrap();
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        std::fs::write(
            tmp.path().join("prds").join("PRD-001-auth.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\n---\n\nBody.\n",
        )
        .unwrap();
        tmp
    }

    async fn capture(root: &Path) -> Arc<CapturedSourceView> {
        Arc::new(
            capture_source(&CaptureRequest::new(root), CaptureLimits::default())
                .await
                .unwrap(),
        )
    }

    fn index_root(tmp: &TempDir) -> std::path::PathBuf {
        tmp.path().join("lance")
    }

    fn barrier_for(
        root: &std::path::Path,
    ) -> std::sync::Arc<dyn super::super::durability::PlatformDurability> {
        let (_support, barrier, diagnostic) = super::super::durability::platform_barrier(root);
        assert!(diagnostic.is_none());
        barrier.expect("a supported root gets a barrier")
    }

    fn manifest_for(captured: &CapturedSourceView, generation_id: &str) -> GenerationManifest {
        GenerationManifest::from_capture(captured, generation_id, TableVersions::new(1, 1))
    }

    #[tokio::test]
    async fn no_pointer_yields_an_ephemeral_view_and_creates_nothing() {
        let tmp = workspace();
        let captured = capture(tmp.path()).await;
        let before = listing(tmp.path());

        let view = ensure_read_view(tmp.path(), Arc::clone(&captured))
            .await
            .unwrap();

        assert!(!view.is_persisted());
        assert_eq!(view.snapshot_id(), captured.snapshot_id);
        assert_eq!(
            listing(tmp.path()),
            before,
            "a read must not create a pointer, a marker or a lock"
        );
    }

    #[tokio::test]
    async fn a_matching_pointer_yields_the_persisted_generation() {
        let tmp = workspace();
        let root = index_root(&tmp);
        let captured = capture(tmp.path()).await;
        let barrier = barrier_for(&root);

        let manifest = manifest_for(&captured, "gen-1");
        let sha =
            super::super::storage::write_generation_manifest(&root, &manifest, barrier.as_ref())
                .unwrap();
        let descriptor = super::super::storage::descriptor_for(&manifest, 7, &sha);
        super::super::storage::publish_current(&root, &descriptor, barrier.as_ref()).unwrap();

        let view = ensure_read_view(tmp.path(), Arc::clone(&captured))
            .await
            .unwrap();
        assert!(view.is_persisted());
        assert_eq!(view.snapshot_id(), "gen-1");
        assert_eq!(view.coverage().read, captured.entries.len());
    }

    #[tokio::test]
    async fn a_moved_source_yields_an_ephemeral_view_rather_than_a_stale_one() {
        let tmp = workspace();
        let root = index_root(&tmp);
        let barrier = barrier_for(&root);
        let captured = capture(tmp.path()).await;

        let manifest = manifest_for(&captured, "gen-1");
        let sha =
            super::super::storage::write_generation_manifest(&root, &manifest, barrier.as_ref())
                .unwrap();
        let descriptor = super::super::storage::descriptor_for(&manifest, 7, &sha);
        super::super::storage::publish_current(&root, &descriptor, barrier.as_ref()).unwrap();

        // The canonical source moves after publication.
        std::fs::write(
            tmp.path().join("prds").join("PRD-001-auth.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\n---\n\nDifferent.\n",
        )
        .unwrap();
        let fresh = capture(tmp.path()).await;

        let view = ensure_read_view(tmp.path(), Arc::clone(&fresh))
            .await
            .unwrap();
        assert!(
            !view.is_persisted(),
            "serving gen-1 here would answer from bytes that no longer exist"
        );
        assert_eq!(view.snapshot_id(), fresh.snapshot_id);
        assert_eq!(view.source_set_digest(), fresh.source_set_digest);
    }

    #[tokio::test]
    async fn a_pointer_whose_manifest_is_missing_is_unavailable_not_ephemeral() {
        let tmp = workspace();
        let root = index_root(&tmp);
        let barrier = barrier_for(&root);
        let captured = capture(tmp.path()).await;
        let manifest = manifest_for(&captured, "gen-1");
        let sha =
            super::super::storage::write_generation_manifest(&root, &manifest, barrier.as_ref())
                .unwrap();
        let descriptor = super::super::storage::descriptor_for(&manifest, 7, &sha);
        super::super::storage::publish_current(&root, &descriptor, barrier.as_ref()).unwrap();

        // The pointer survives; the manifest it names does not.
        std::fs::remove_file(root.join("generations").join("gen-1.json")).unwrap();

        let err = ensure_read_view(tmp.path(), Arc::clone(&captured))
            .await
            .unwrap_err();
        assert_eq!(
            err.code(),
            crate::constellation::DiagnosticCode::IndexGenerationUnavailable,
            "an unreadable pointer is a fact about the index, not a first run"
        );
    }

    #[tokio::test]
    async fn a_pointer_whose_manifest_checksum_disagrees_is_unavailable() {
        let tmp = workspace();
        let root = index_root(&tmp);
        let barrier = barrier_for(&root);
        let captured = capture(tmp.path()).await;
        let manifest = manifest_for(&captured, "gen-1");
        let sha =
            super::super::storage::write_generation_manifest(&root, &manifest, barrier.as_ref())
                .unwrap();
        let descriptor = super::super::storage::descriptor_for(&manifest, 7, &sha);
        super::super::storage::publish_current(&root, &descriptor, barrier.as_ref()).unwrap();

        // Mutate the manifest behind the pointer's back.
        let path = root.join("generations").join("gen-1.json");
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push('\n');
        std::fs::write(&path, text).unwrap();

        let err = ensure_read_view(tmp.path(), Arc::clone(&captured))
            .await
            .unwrap_err();
        assert_eq!(
            err.code(),
            crate::constellation::DiagnosticCode::IndexGenerationUnavailable
        );
    }

    #[tokio::test]
    async fn open_pinned_view_returns_the_exact_versions_and_refuses_a_half_pair() {
        let tmp = workspace();
        let root = index_root(&tmp);
        let barrier = barrier_for(&root);
        let captured = capture(tmp.path()).await;
        let manifest = manifest_for(&captured, "gen-9");
        let sha =
            super::super::storage::write_generation_manifest(&root, &manifest, barrier.as_ref())
                .unwrap();
        let descriptor = super::super::storage::descriptor_for(&manifest, 3, &sha);

        let pinned = open_pinned_view(&root, &descriptor).unwrap();
        assert_eq!(pinned.artifacts_version, 1);
        assert_eq!(pinned.relations_version, 1);
        assert_eq!(pinned.manifest.generation_id, "gen-9");
        assert_eq!(pinned.descriptor.owner_epoch, 3);

        // A descriptor naming a different generation than its manifest.
        let mut wrong = descriptor.clone();
        wrong.generation_id = "gen-8".to_string();
        let err = open_pinned_view(&root, &wrong).unwrap_err();
        assert_eq!(
            err.code(),
            crate::constellation::DiagnosticCode::IndexGenerationUnavailable
        );
    }

    #[tokio::test]
    async fn a_generation_naming_only_one_table_is_never_served_as_a_pair() {
        let tmp = workspace();
        let root = index_root(&tmp);
        let barrier = barrier_for(&root);
        let captured = capture(tmp.path()).await;
        let manifest = manifest_for(&captured, "gen-10");
        // One half of the pair is version 0: never committed, never coherent.
        let manifest = GenerationManifest {
            table_versions: TableVersions::new(1, 0),
            ..manifest
        };
        let sha =
            super::super::storage::write_generation_manifest(&root, &manifest, barrier.as_ref())
                .unwrap();
        let descriptor = super::super::storage::descriptor_for(&manifest, 1, &sha);
        let err = open_pinned_view(&root, &descriptor).unwrap_err();
        assert_eq!(
            err.code(),
            crate::constellation::DiagnosticCode::IndexGenerationUnavailable
        );
    }

    #[tokio::test]
    async fn an_immutable_generation_may_not_be_republished_under_the_same_id() {
        let tmp = workspace();
        let root = index_root(&tmp);
        let barrier = barrier_for(&root);
        let captured = capture(tmp.path()).await;
        let manifest = manifest_for(&captured, "gen-11");

        super::super::storage::write_generation_manifest(&root, &manifest, barrier.as_ref())
            .unwrap();
        let err =
            super::super::storage::write_generation_manifest(&root, &manifest, barrier.as_ref())
                .unwrap_err();
        assert_eq!(
            err.code(),
            crate::constellation::DiagnosticCode::IndexProtocolMismatch
        );
    }

    #[test]
    fn shutdown_stops_admission_and_leaves_reads_and_retention_alone() {
        resume_admission();
        assert!(is_admitting());
        shutdown_runtime().unwrap();
        assert!(!is_admitting());
        // Retention is a separate promise and is unaffected by shutdown.
        // `AUTOMATIC_RETENTION_ENABLED` is a compile-time false; the live
        // check is the policy assertion below.
        assert!(super::super::storage::assert_retention_policy().is_ok());
        resume_admission();
        assert!(is_admitting());
    }

    #[test]
    fn a_protocol_marker_round_trips_and_an_initializing_one_is_not_uninitialized() {
        let tmp = workspace();
        let root = index_root(&tmp);
        let barrier = barrier_for(&root);
        std::fs::create_dir_all(&root).unwrap();

        assert_eq!(
            super::super::storage::read_marker(&root).unwrap(),
            IndexProtocolState::Uninitialized,
            "a missing marker is a normal first run, not corruption"
        );

        let marker = ProtocolMarker {
            protocol_version: crate::constellation::FORMAT_VERSION,
            state: BootstrapState::Initializing,
            index_epoch: 1,
            checkout_id: "local-x".to_string(),
            table_names: ProtocolMarker::active_table_names(),
            storage_scheme: STORAGE_SCHEME_FILE_OBJECT_STORE.to_string(),
            bootstrap_attempt: 1,
        };
        super::super::storage::write_marker(&root, &marker, barrier.as_ref()).unwrap();

        match super::super::storage::read_marker(&root).unwrap() {
            IndexProtocolState::Initializing(read) => {
                assert_eq!(read.checkout_id, "local-x");
                assert_eq!(read.index_epoch, 1);
            }
            other => panic!("expected Initializing, got {other:?}"),
        }
    }

    #[test]
    fn a_corrupt_marker_is_a_protocol_mismatch_not_an_uninitialized_index() {
        let tmp = workspace();
        let root = index_root(&tmp);
        std::fs::create_dir_all(root.join("constellation")).unwrap();
        std::fs::write(root.join("constellation").join("PROTOCOL.json"), "{nope").unwrap();

        let err = super::super::storage::read_marker(&root).unwrap_err();
        assert_eq!(
            err.code(),
            crate::constellation::DiagnosticCode::IndexProtocolMismatch,
            "silently restarting a bootstrap over an index that was mid-flight \
             is exactly what this must not do"
        );
    }

    #[test]
    fn a_marker_naming_the_wrong_scheme_is_refused() {
        let tmp = workspace();
        let root = index_root(&tmp);
        let barrier = barrier_for(&root);
        std::fs::create_dir_all(&root).unwrap();
        let marker = ProtocolMarker {
            protocol_version: crate::constellation::FORMAT_VERSION,
            state: BootstrapState::Active,
            index_epoch: 1,
            checkout_id: "local-x".to_string(),
            table_names: ProtocolMarker::active_table_names(),
            storage_scheme: "file".to_string(),
            bootstrap_attempt: 1,
        };
        let err =
            super::super::storage::write_marker(&root, &marker, barrier.as_ref()).unwrap_err();
        assert_eq!(
            err.code(),
            crate::constellation::DiagnosticCode::IndexProtocolMismatch
        );
    }

    #[test]
    fn a_capability_derives_its_index_root_from_the_workspace_it_names() {
        let a = MaintainCapability::for_strict_maintenance("/ws/a");
        let b = MaintainCapability::for_strict_maintenance("/ws/b");
        assert_ne!(a, b);
        // The relationship is established at construction, not inferred later:
        // a capability cannot name an index that is not under its workspace.
        assert_eq!(a.workspace(), Path::new("/ws/a"));
        assert_eq!(a.index_root(), Path::new("/ws/a").join("lance"));
        assert_eq!(b.index_root(), Path::new("/ws/b").join("lance"));
    }

    #[test]
    fn two_capabilities_for_one_workspace_are_identical() {
        let a = MaintainCapability::for_strict_maintenance("/ws/a");
        let b = MaintainCapability::for_strict_maintenance("/ws/a");
        assert_eq!(
            a, b,
            "the same workspace must yield the same capability, so two \
             processes agree on which index they are allowed to publish"
        );
    }

    #[test]
    fn resolution_policy_values_are_distinct() {
        assert_ne!(ResolutionPolicy::SoftFallback, ResolutionPolicy::Strict);
    }

    #[tokio::test]
    async fn capture_for_read_returns_an_ephemeral_view_and_writes_nothing() {
        let tmp = workspace();
        let before = listing(tmp.path());

        let view = capture_for_read(tmp.path(), None, CaptureLimits::default())
            .await
            .unwrap();

        assert!(!view.is_persisted());
        assert_eq!(view.coverage().read, 1);
        assert_eq!(
            listing(tmp.path()),
            before,
            "a read must not create files, locks or markers"
        );
    }

    #[tokio::test]
    async fn capture_for_read_carries_the_previous_capture_forward() {
        let tmp = workspace();
        let first = capture_for_read(tmp.path(), None, CaptureLimits::default())
            .await
            .unwrap();
        let captured = Arc::new(first.captured().unwrap().clone());

        let second = capture_for_read(tmp.path(), Some(captured), CaptureLimits::default())
            .await
            .unwrap();
        let entry = &second.captured().unwrap().entries[0];
        assert_eq!(entry.row_revision, 1, "identical bytes keep their revision");
        assert_eq!(entry.incarnation, 1);
    }

    fn listing(root: &Path) -> Vec<String> {
        let mut out = Vec::new();
        for dir in ARTIFACT_DIRS {
            let path = root.join(dir);
            let Ok(entries) = std::fs::read_dir(path) else {
                continue;
            };
            for entry in entries.flatten() {
                out.push(entry.path().to_string_lossy().to_string());
            }
        }
        out.sort();
        out
    }

    // -------------------------------------------------------------------
    // End-to-end publish: the tests above cover each boundary in isolation;
    // this one runs the real pipeline. It is the test that makes a manifest
    // unable to claim rows its tables do not carry — the failure mode the
    // whole protocol exists to prevent.
    // -------------------------------------------------------------------

    fn linked_workspace() -> TempDir {
        let tmp = TempDir::new().unwrap();
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        std::fs::write(
            tmp.path().join("adrs").join("ADR-002-storage.md"),
            "---\nid: ADR-002\nkind: adr\nstatus: active\ntitle: Storage\n---\n\nBody.\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("prds").join("PRD-001-auth.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\nlinks:\n  - target: ADR-002\n    relation: refines\n---\n\nBody.\n",
        )
        .unwrap();
        tmp
    }

    #[tokio::test]
    async fn a_first_publish_writes_the_rows_the_manifest_claims() {
        resume_admission();
        let tmp = linked_workspace();
        // `/var` is a symlink on macOS; the owner-location check refuses a
        // spelled root whose components are aliases, so the capability is
        // built on the canonical spelling — same rule as the POSIX fixtures.
        let canonical = tmp.path().canonicalize().unwrap();
        let capability = MaintainCapability::for_strict_maintenance(canonical);

        let (view, outcome) = maintain_index(
            &capability,
            CaptureLimits::default(),
            std::time::Instant::now() + std::time::Duration::from_secs(120),
        )
        .await
        .unwrap();

        assert_eq!(view.manifest.source_entries.len(), 2);
        // A first pass publishes exactly one generation — the bootstrap's;
        // the reconcile pass on an unchanged source commits nothing and the
        // early return above keeps the same pointer.
        assert!(
            matches!(
                outcome.progress,
                crate::constellation::publisher::BootstrapProgress::CreatedTables { .. }
            ),
            "a first publish must report table creation, got {:?}",
            outcome.progress
        );
        assert!(
            !view.manifest.generation_id.contains(':'),
            "the id becomes a filename; `:` is illegal on NTFS"
        );

        // The pinned artifacts version must hold exactly the claimed rows —
        // read through an independent pinned reader, never latest.
        let marker = marker_for(&view.manifest);
        let artifacts = super::super::lance_tables::open_table_at_version(
            capability.index_root(),
            &marker,
            "artifacts",
            view.manifest.table_versions.artifacts,
        )
        .await
        .unwrap();
        let rows: usize = artifacts
            .query_batches()
            .await
            .unwrap()
            .iter()
            .map(|b| b.num_rows())
            .sum();
        assert_eq!(
            rows, 2,
            "the manifest claims two artifacts; the pinned table must carry them"
        );

        let relations = super::super::lance_tables::open_table_at_version(
            capability.index_root(),
            &marker,
            "relations",
            view.manifest.table_versions.relations,
        )
        .await
        .unwrap();
        let edges: usize = relations
            .query_batches()
            .await
            .unwrap()
            .iter()
            .map(|b| b.num_rows())
            .sum();
        assert_eq!(edges, 1, "PRD-001 -> ADR-002 refines must be written");

        // A second pass over an unchanged source commits nothing new.
        let (second_view, second) = maintain_index(
            &capability,
            CaptureLimits::default(),
            std::time::Instant::now() + std::time::Duration::from_secs(120),
        )
        .await
        .unwrap();
        assert_eq!(
            second_view.manifest.generation_id,
            view.manifest.generation_id
        );
        assert!(
            second.commits.is_empty(),
            "an unchanged source must not produce commits: {:?}",
            second.commits
        );
    }
}
