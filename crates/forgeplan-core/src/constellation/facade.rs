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
//! Present: the two read boundaries, [`maintain_index`], the embedding
//! admission/completion/failure trio, and [`shutdown_runtime`].
//!
//! The embedding path is deliberately **three** functions rather than one
//! fused pass: [`prepare_embedding_batch`] holds the owner only long enough to
//! decide and record what needs computing, the model runs with no guard of any
//! kind in scope, and [`complete_embedding_batch`] reacquires the owner to
//! write each result under its own job's guard. Fusing them is what made the
//! old drain hold an OS-wide exclusive lock across a native inference call.
//! Nothing here exposes a route by which a caller supplies a canonical key and
//! a float vector and has the binding worked out for it — that route is what
//! let a vector computed from superseded bytes be rebind onto the capture
//! taken after the compute.
//!
//! [`fail_embedding_batch`] is the fourth leg: what a caller records when the
//! model failed, so a failing key becomes visible and backs off instead of
//! being handed straight back by the next admission.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::capture::{CaptureLimits, CaptureRequest, CapturedSourceView, capture_source};
use super::manifest::{CurrentDescriptor, GenerationManifest};
use super::storage::{self, IndexProtocolState};
use super::view::ReadView;
use super::{ConstellationError, checked_increment};

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
/// * Pointer present and matching → the published generation, **pinned**: its
///   manifest is opened, checksummed and locked shared before the view exists,
///   and the lock lives as long as the view does. A pointer that does not
///   resolve is not a fast path.
///
/// # Why the pin, and why it is the descriptor that travels with it
///
/// The manifest bytes and the embedding heads live in two places. The heads
/// are on `CURRENT`, which a completion drain republishes; the versions and the
/// source set are in the immutable manifest. A view carrying only the manifest
/// would report the generation's pre-drain heads while serving tables opened
/// at post-drain versions — a reader that says `pending` about rows a previous
/// process already published, with no disagreement anywhere visible to
/// diagnose it. So the view carries both, plus the lock that keeps the
/// generation alive for as long as the answer needs it.
///
/// # Errors
///
/// [`ConstellationError::IndexGenerationUnavailable`] when a pointer exists
/// but its manifest is missing, its checksum disagrees, or the generation
/// cannot be locked. That is never downgraded to an ephemeral view: "I could
/// not read what was published" and "nothing has been published" are
/// different facts, and only one of them should be answered with fresh bytes.
pub async fn ensure_read_view(
    workspace: &Path,
    captured: Arc<CapturedSourceView>,
) -> Result<ReadView, ConstellationError> {
    let index_root = workspace.join("lance");
    let descriptor = storage::read_current(&index_root)?;

    let Some(descriptor) = descriptor else {
        return Ok(ReadView::from_capture(captured));
    };

    // One open, one checksum, one shared lock. `pin_generation_read` refuses
    // an unreadable, replaced or checksum-disagreeing manifest, so there is no
    // second read of the same file that could see different bytes.
    let pin = Arc::new(storage::pin_generation_read(&index_root, &descriptor)?);
    let manifest = pin.manifest_arc();
    if manifest.source_set_digest != descriptor.source_set_digest {
        return Err(ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "manifest {} disagrees with CURRENT about the source set",
                descriptor.manifest_path
            ),
        });
    }
    if descriptor.source_set_digest != captured.source_set_digest
        || descriptor.config_digest != captured.config_digest
    {
        return Ok(ReadView::Ephemeral {
            captured,
            vector_cache: Some(pin),
        });
    }
    ReadView::from_manifest(manifest, descriptor, pin)
}

/// One call that produces both halves of a read view.
///
/// `capture_for_read` followed by `ensure_read_view` written at a call site is
/// the same sequence this performs, with one difference that matters: the
/// capture is kept. `ensure_read_view` returns a `Persisted` view that carries
/// the manifest, not the bytes the caller just confirmed — so a caller that
/// wanted the records as well would have to hold two `Arc`s and hope they were
/// the same capture. Here they provably are: the `Arc` handed in is the one
/// wrapped in the result.
///
/// `previous` carries the prior capture forward so `incarnation` and
/// `row_revision` keep moving across reads; passing `None` observes every
/// artifact as new, which is right for a first read and wrong for a
/// steady-state one.
///
/// Takes no owner, no `WorkspaceLock`, writes nothing, and never triggers
/// maintenance or a model download.
///
/// # Errors
///
/// [`ConstellationError::SourceBusy`] when the source is moving, and
/// [`ConstellationError::IndexGenerationUnavailable`] when a pointer exists but
/// its manifest cannot be verified — the same errors the two halves raise, and
/// never a silent substitution.
pub async fn open_workspace_read(
    workspace: &Path,
    previous: Option<Arc<CapturedSourceView>>,
    limits: CaptureLimits,
) -> Result<super::workspace_read::WorkspaceRead, ConstellationError> {
    let mut request = CaptureRequest::new(workspace);
    if let Some(previous) = previous {
        request = request.with_previous(previous);
    }
    let captured = Arc::new(capture_source(&request, limits).await?);
    if !captured.coverage.is_complete() {
        return Err(ConstellationError::SourceUnavailable {
            detail: format!(
                "canonical source capture is incomplete: {:?}",
                captured.diagnostics
            ),
        });
    }
    let view = ensure_read_view(workspace, Arc::clone(&captured)).await?;
    Ok(super::workspace_read::WorkspaceRead::from_parts(
        workspace, view, captured,
    ))
}

/// Whether this workspace's index has been cut over to Constellation.
///
/// This is the branch a read path takes **before** it decides whether opening a
/// legacy `LanceStore` is legitimate. Reading `CURRENT`/`PROTOCOL.json` is
/// two small file reads; opening a legacy store is a migration run.
///
/// # What counts as migrated
///
/// * A `PROTOCOL.json` in any state other than absent — `Active` (published) or
///   `Initializing` (bootstrap in flight). Both own the index; a bootstrap that
///   was interrupted is still a Constellation index, and treating it as legacy
///   is exactly how a half-built index gets a second set of tables beside it.
/// * A `CURRENT` pointer, which is the visibility point and therefore proof
///   something was published even if the marker is gone.
///
/// # What happens on a damaged marker
///
/// [`IndexProtocolState`] is returned only for a marker that parses. A
/// `PROTOCOL.json` that does **not** parse means the index claims Constellation
/// and cannot say what it published — the two states are "a Constellation index"
/// and "a broken Constellation index", never "a legacy index". Reporting
/// `false` there would hand a damaged workspace to the legacy writer.
pub fn workspace_is_migrated(workspace: &Path) -> bool {
    let index_root = workspace.join(MaintainCapability::INDEX_DIR);
    match super::storage::read_marker(&index_root) {
        Ok(IndexProtocolState::Uninitialized) => {
            matches!(super::storage::read_current(&index_root), Ok(Some(_)))
        }
        // Any marker state, or a marker that does not parse: the index claims
        // Constellation either way.
        Ok(_) | Err(_) => true,
    }
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
    /// The exact sections version this generation pins — `None` for a
    /// generation written before `constellation_sections_v1` existed. `None`
    /// means "section search is not part of this generation", never "open
    /// the latest": there is no fallback to head anywhere in this protocol.
    pub sections_version: Option<u64>,
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
        artifacts_version: descriptor.effective_artifacts_version(&manifest)?,
        relations_version: manifest.table_versions.relations,
        sections_version: descriptor.effective_sections_version(&manifest)?,
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

/// What one completion pass achieved.
///
/// The fields exist so that "embedded 3" can never be confused with
/// "embedded 0 because 4 could not be judged". Both are the same count of
/// successes over a different total, and only `eligible_keys` beside
/// `cannot_judge` tells them apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingBatchOutcome {
    /// The profile every vector was published against. It comes from the
    /// **prepared batch**, which admission resolved — never from a caller and
    /// never from whatever the index happens to name now.
    pub profile_id: String,
    /// The keys this completion confirmed are still live and still bound to
    /// the job they were computed for. A key that moved is not in here: it is
    /// in `stale`, or in `cannot_judge`, and the difference matters.
    pub eligible_keys: Vec<String>,
    /// Vectors that were written — rows the guarded update actually moved.
    pub published: Vec<String>,
    /// Keys whose completion matched no row — the row moved, or the profile
    /// it was admitted under is no longer the row's. **Not** a failure; it is
    /// the expected outcome of a stale worker, and it is counted rather than
    /// retried.
    pub stale: Vec<String>,
    /// Rows whose binding could not be confirmed at all, with the reason.
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

/// The "is there embed work" answer, before the model is paid for.
///
/// Counts come from the same `artifacts_eligibility` the admission path uses,
/// so what this reports is what [`prepare_embedding_batch`] would find.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbedPrecheck {
    /// Rows whose vector the current profile still owes.
    pub eligible: usize,
    /// Rows already bound to the current profile and `ready`.
    pub current: usize,
    /// Rows whose eligibility could not be decided.
    pub cannot_judge: usize,
}

/// Decide whether an explicit `embed` has work **without** loading weights.
///
/// The capture alone cannot answer "nothing to do" — only the stored binding
/// can. This runs the admission path's first half (profile → capture →
/// tables → eligibility) and stops before any job is admitted, any vector is
/// computed, and any row is written.
///
/// `Ok(None)` is an honest abstention, not an error: no published
/// generation, no cached snapshot to checksum, an owner held by another
/// pass — in each case the caller proceeds to the model path, where
/// [`prepare_embedding_batch`] recomputes the same eligibility and does the
/// admission properly. Profile resolution reads only files already in the
/// cache; it never downloads and never loads weights.
#[cfg(feature = "semantic-search")]
pub async fn embedding_precheck(
    workspace: &Path,
) -> Result<Option<EmbedPrecheck>, ConstellationError> {
    use super::jobs::DEFAULT_MODEL_REPO;
    use super::lance_tables::open_active_tables_readonly;
    use super::publisher::{IndexPublisher, capture_under_workspace_lock, identity_for};

    let index_root = workspace.join("lance");

    // The profile from the *cached* snapshot: checksums are file reads. A
    // snapshot that is not cached abstains — the download-capable path is
    // the caller's, not a precheck's. The chunk size is this workspace's,
    // because a profile resolved for one `chunk_size` and published against
    // another is two embedding spaces wearing one name.
    let body_prefix_chars = crate::workspace::load_config(workspace)
        .ok()
        .and_then(|config| config.embedding.map(|embedding| embedding.chunk_size))
        .unwrap_or(super::jobs::DEFAULT_BODY_PREFIX_CHARS);
    let cache_dir = crate::embed::resolve_cache_dir();
    let Ok(profile) = super::jobs::ResolvedProfile::open(
        &cache_dir,
        DEFAULT_MODEL_REPO,
        ENGINE_VERSION,
        body_prefix_chars,
    ) else {
        return Ok(None);
    };
    let profile_id = profile.profile.profile_id();

    // The owner briefly — the same acquire `prepare_embedding_batch` performs.
    // A busy owner means another pass holds the index; abstain and let the
    // real admission surface the contention properly.
    let publisher = match IndexPublisher::acquire(&index_root, inner_store_for(&index_root)) {
        Ok(publisher) => publisher,
        Err(ConstellationError::IndexBusy { .. }) => return Ok(None),
        Err(e) => return Err(e),
    };

    // `CURRENT` is read under the owner, once, and the manifest it names is
    // the capture's identity baseline. Reading it before acquiring would name
    // a generation the owner may already have replaced; capturing without one
    // would re-derive every identity from nothing and report the whole corpus
    // stale — sending the caller into a migration nobody asked for.
    let descriptor = match super::storage::read_current(&index_root)? {
        Some(descriptor) => descriptor,
        None => return Ok(None),
    };
    let published = Arc::new(super::storage::read_manifest(&index_root, &descriptor)?);
    let captured = capture_under_workspace_lock(
        workspace,
        super::capture::CaptureLimits::default(),
        Some(Arc::clone(&published)),
    )
    .await?;
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
    // Opened **read-only**, with no writer store and no creation: a precheck
    // that can bring a table into existence is not a precheck. It holds no
    // publisher whose closure barrier could sync a write, so a creation here
    // would be a dataset nothing ever promised to be durable. On an index
    // published before the sections table existed, `open_active_tables` would
    // create it through the tracking store and this function would then drop
    // the store without ever taking its closure.
    //
    // The read-only open treats that missing dataset as
    // `index_generation_unavailable`, which is an honest abstention here: the
    // caller proceeds to the model path, where
    // [`prepare_embedding_batch`] does the creation properly under an owner
    // whose barrier syncs it.
    let tables = match open_active_tables_readonly(&index_root, &marker).await {
        Ok(tables) => tables,
        Err(ConstellationError::IndexGenerationUnavailable { .. }) => return Ok(None),
        Err(e) => return Err(e),
    };
    let mut eligibility = tables.artifacts_eligibility(&captured, &profile_id).await?;
    eligibility.merge_from(&tables.sections_eligibility(&captured, &profile_id).await?);
    drop(tables);
    drop(publisher);

    Ok(Some(EmbedPrecheck {
        eligible: eligibility.eligible_keys().len(),
        current: eligibility.current_keys().len(),
        cannot_judge: eligibility.cannot_judge().len(),
    }))
}

/// Feature-off abstains the way `Ok(None)` does: nothing to report.
#[cfg(not(feature = "semantic-search"))]
pub async fn embedding_precheck(
    _workspace: &Path,
) -> Result<Option<EmbedPrecheck>, ConstellationError> {
    Ok(None)
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
    /// What the trigram side-index did this pass: `Bound` means it was rebuilt
    /// from the published generation's exact bytes, `Drifted` means a source
    /// moved mid-build and reads must fall back, `Missing`/`Stale` before a
    /// heal attempt stay `None`-adjacent — the field is `None` only when the
    /// pass never reached the point of tagging.
    pub lexical_index: Option<super::lexical_index::LexicalRebind>,
}

/// A published generation, as the caller receives it.
#[derive(Debug, Clone)]
pub struct PublishedReadView {
    pub descriptor: CurrentDescriptor,
    pub manifest: Arc<GenerationManifest>,
}

/// Bind the trigram side-index to `manifest`'s generation.
///
/// Skips the rebuild entirely when the existing tag already names this
/// generation and digest — the common case for no-op passes. A rebuild
/// failure degrades to `None` in the outcome rather than failing the pass:
/// the pinned scan answers the same queries, slower.
fn rebind_lexical_index(
    workspace: &Path,
    manifest: &GenerationManifest,
    entries: &[super::capture::SourceEntry],
) -> Option<super::lexical_index::LexicalRebind> {
    use super::lexical_index::{LexicalBinding, binding_for, rebuild_for_generation};
    if binding_for(workspace, manifest) == LexicalBinding::Bound {
        return None;
    }
    rebuild_for_generation(
        workspace,
        &manifest.generation_id,
        &manifest.source_set_digest,
        entries,
    )
    .ok()
}

/// The manifest a pass treats as its baseline: the published generation with
/// its table versions replaced by the **effective** ones the descriptor names.
///
/// The artifacts half is the descriptor's effective version, because an
/// embedding admission or completion legitimately advances the head past the
/// manifest's pin and records it there. Comparing against the manifest's pin
/// alone would call those writes "divergence" and rewind over committed
/// vectors on every pass — the exact defect `effective_artifacts_version`
/// exists to prevent. Sections follow the same rule through
/// `effective_sections_version`.
///
/// One definition, three callers: [`maintain_index`],
/// [`prepare_embedding_batch`] and [`complete_embedding_batch`] must all agree
/// on what "the published state" is, or a pass that restores will restore to a
/// different place than the one its neighbour published.
fn published_baseline(
    descriptor: &CurrentDescriptor,
    published: &GenerationManifest,
) -> Result<GenerationManifest, ConstellationError> {
    let mut baseline = published.clone();
    baseline.table_versions.artifacts = descriptor.effective_artifacts_version(published)?;
    baseline.table_versions.sections = descriptor.effective_sections_version(published)?;
    Ok(baseline)
}

/// Compare the open tables' physical heads against `baseline` and rewind the
/// diverged ones, returning the versions that exist **after** any restore.
///
/// The shared form of [`maintain_index`]'s recovery step, and the reason
/// admission and completion run it too: all three read the tables as the
/// published baseline, so a head an abandoned pass left behind would
/// otherwise be spliced into their work — a `pending` or `failed` state with
/// no job naming it, or a vector whose binding was never recorded.
///
/// Each restore creates a NEW physical head, so the returned versions are
/// re-read afterwards and are never the pre-restore numbers. Only the affected
/// table is rewound. A pre-sections generation pins `sections: None`, so
/// `sections_diverged` can never be true under one — the head is free, not
/// divergent.
///
/// # Errors
///
/// [`ConstellationError::IndexGenerationUnavailable`] when a pinned version is
/// gone. Explicitly unavailable, never a jump to latest.
async fn restore_diverged_heads(
    tables: &super::lance_tables::IndexTables,
    baseline: &GenerationManifest,
) -> Result<super::manifest::TableVersions, ConstellationError> {
    use super::publisher::{HeadComparison, compare_heads};

    match compare_heads(
        Some(baseline),
        tables.artifacts_version().await?,
        tables.relations_version().await?,
        tables.sections_version().await?,
    ) {
        HeadComparison::Matches => {}
        HeadComparison::Diverged {
            artifacts_diverged,
            relations_diverged,
            sections_diverged,
        } => {
            if artifacts_diverged {
                tables
                    .restore_artifacts_to(baseline.table_versions.artifacts)
                    .await?;
            }
            if relations_diverged {
                tables
                    .restore_relations_to(baseline.table_versions.relations)
                    .await?;
            }
            if sections_diverged {
                let pinned = baseline.table_versions.sections.ok_or_else(|| {
                    ConstellationError::IndexGenerationUnavailable {
                        detail: "sections head diverged but the published manifest pins none"
                            .to_string(),
                    }
                })?;
                tables.restore_sections_to(pinned).await?;
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
    tables.head_versions().await
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
///   never acquired before the owner. It is taken once more — and held across
///   the whole switch — for the final source validation and the pointer
///   publish, so nothing can change the workspace between "the source still
///   looks like what we staged" and "CURRENT now names it".
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
///
/// [`restore_diverged_heads`] and [`published_baseline`] below are the shared
/// recovery step this function, [`prepare_embedding_batch`] and
/// [`complete_embedding_batch`] all run against the same baseline.
pub async fn maintain_index(
    capability: &MaintainCapability,
    capture_limits: super::capture::CaptureLimits,
    deadline: std::time::Instant,
) -> Result<(PublishedReadView, ReconcileOutcome), ConstellationError> {
    use super::publisher::{
        IndexPublisher, artifacts_batch, capture_under_workspace_lock, plan_commits, stage_sequence,
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
    let baseline = published_baseline(&descriptor, &published)?;

    let marker = marker_for(&published);
    let tables = super::lance_tables::open_active_tables(
        &index_root,
        &marker,
        std::sync::Arc::clone(publisher.tracking_store()),
    )
    .await?;

    let head_versions = restore_diverged_heads(&tables, &baseline).await?;

    // Capture under the workspace lock, released before the first commit.
    //
    // The baseline is the generation this pass read at its start, under the
    // owner it already holds. It is what keeps `incarnation` and
    // `row_revision` moving across passes: without it every production pass
    // re-derived them as `1`, and a byte-identical delete/recreate would
    // reproduce a retired row's identity — inheriting its vector and
    // satisfying an outstanding job bound to the deleted document.
    let captured = capture_under_workspace_lock(
        &workspace,
        capture_limits,
        Some(std::sync::Arc::clone(&published)),
    )
    .await?;
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
    // Keys a re-keyed Update left behind. The vacated key's edges must go:
    // the edge rows a changed key owns are deleted under the *new* key, and
    // nothing else names the old one. Rows are retired through
    // `stage_sequence`'s `retired_keys`; keys this capture re-claims under a
    // different path are filtered — deleting one of those would kill the row
    // the merge just wrote.
    let claimed_keys: std::collections::BTreeSet<&str> = captured
        .entries
        .iter()
        .map(|entry| entry.canonical_key.as_str())
        .collect();
    let retired_keys: Vec<String> = reconcile
        .deltas
        .iter()
        .filter_map(|delta| delta.old_canonical_key.clone())
        .filter(|key| !claimed_keys.contains(key.as_str()))
        .collect();
    let relations_delete_keys: Vec<String> = changed_keys
        .iter()
        .cloned()
        .chain(delete_keys.iter().cloned())
        .chain(
            reconcile
                .deltas
                .iter()
                .filter_map(|delta| delta.old_canonical_key.clone()),
        )
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

    // The semantic baseline, read from the open tables at the head this pass
    // is about to merge into. Position is load-bearing on both sides:
    //
    // * **After** `compare_heads` and any restore, so a rewound table
    //   contributes the published rows rather than whatever an abandoned
    //   write left behind.
    // * **Under** the owner this pass already holds, so no other publisher
    //   can move the head between the read and the merge that splices it.
    //
    // The gate mirrors `stage_sequence` exactly, because a missed branch here
    // wipes vectors instead of failing loudly. It emits:
    //
    // * `MergeUpsert` when `creates > 0 || updates > 0`;
    // * `SectionsMerge` when the section batch is non-empty **and** either the
    //   commit plan is non-empty — a delete-only pass still rewrites the whole
    //   section set, so gating on creates/updates alone would hand it an empty
    //   preserve map — or the table is being brought up for the first time.
    //
    // Everything else it emits (`DeleteByPredicate`, `RelationsDelete`,
    // `RelationsMerge`) is key-addressed and never carries a batch whose
    // semantic columns were reset. When this gate is false the sequence is
    // empty, so the empty maps below are never spliced over anything.
    let merges_rows = !commits_plan.is_empty() || baseline.table_versions.sections.is_none();
    let (preserved_artifacts, preserved_sections) = if merges_rows {
        (
            tables.artifact_semantics().await?,
            tables.section_semantics().await?,
        )
    } else {
        (
            BTreeMap::<String, super::publisher::SemanticColumns>::new(),
            BTreeMap::new(),
        )
    };

    let section_batch = super::publisher::sections_batch(&captured, &preserved_sections)?;
    let sections_plan = super::publisher::SectionsPlan {
        // Sections of a deleted parent die by parent key — the tombstones
        // are the same confirmed absences the artifact delete used. A
        // re-keyed parent's sections are equally orphaned under the old key,
        // and the re-claimed parent's sections are re-merged right after the
        // delete, so the old key joins the set unconditionally.
        parent_delete_keys: reconcile
            .tombstones
            .iter()
            .map(|t| t.canonical_key.clone())
            .chain(
                reconcile
                    .deltas
                    .iter()
                    .filter_map(|delta| delta.old_canonical_key.clone()),
            )
            .collect(),
        merge_rows: section_batch.num_rows(),
        // A published generation that predates the sections table has no pin;
        // its first post-upgrade pass must populate the table even when the
        // artifact side is a no-op.
        bootstrap: baseline.table_versions.sections.is_none(),
    };
    let sequence = stage_sequence(
        &commits_plan,
        captured.artifacts.len(),
        delete_keys,
        retired_keys,
        &relations_plan,
        &sections_plan,
    );

    let mut commits: Vec<CommitSummary> = Vec::new();
    let mut verification: Option<super::publisher::VerificationReport> = None;
    let mut committed_versions = head_versions;

    if !sequence.is_empty() {
        let batch = artifacts_batch(
            &captured.artifacts,
            captured.index_epoch,
            &captured.checkout_id,
            &preserved_artifacts,
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
                super::publisher::CommitOp::SectionsDelete { parent_keys } => {
                    let outcome = tables.delete_from_sections(parent_keys).await?;
                    commits.push(CommitSummary {
                        kind: "sections_delete",
                        version: outcome.version,
                        rows_affected: outcome.rows_deleted,
                    });
                }
                super::publisher::CommitOp::SectionsMerge {
                    full_source_batch, ..
                } => {
                    let outcome = tables
                        .merge_into_sections(
                            std::sync::Arc::clone(&section_batch),
                            *full_source_batch,
                        )
                        .await?;
                    commits.push(CommitSummary {
                        kind: "sections_merge",
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
        // The sections check is pinned to the committed head and proves every
        // captured section row carries its parent's source digest. It always
        // runs when a stage ran: this build owns a sections table, so the new
        // manifest will pin `Some(_)` and the rows must actually exist.
        let expected_sections_map: std::collections::BTreeMap<String, String> = captured
            .sections
            .iter()
            .filter_map(|section| {
                captured
                    .entries
                    .iter()
                    .find(|entry| entry.canonical_key == section.parent_key)
                    .map(|entry| (section.canonical_key.clone(), entry.source_digest.clone()))
            })
            .collect();
        let sections_expectation =
            committed_versions
                .sections
                .map(|version| super::publisher::SectionsExpectation {
                    version,
                    digests: expected_sections_map,
                });
        verification = Some(
            super::publisher::verify_staged(
                &index_root,
                &marker,
                committed_versions.artifacts,
                &expected,
                committed_versions.relations,
                expected_edges,
                sections_expectation.as_ref(),
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
    if sequence.is_empty()
        && head_versions == baseline.table_versions
        && captured.source_set_digest == published.source_set_digest
        && captured.config_digest == published.config_digest
    {
        let lexical_index = rebind_lexical_index(&workspace, &published, &captured.entries);
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
                lexical_index,
            },
        ));
    }

    // Final source validation **and** the pointer switch, under one workspace
    // lock, in owner → workspace order like every other step here.
    //
    // They were two lock acquisitions: the recheck captured, released, and the
    // publish re-acquired. A writer that landed in that gap was invisible to
    // the recheck and still got its change published over — a generation whose
    // `source_entries` described bytes the workspace had already moved past.
    // One lock covers both, and nothing else happens inside it: the manifest is
    // assembled from values this pass already holds, so no compute and no
    // second capture is added to the critical section.
    //
    // The digests compared are the ones the staging was computed against —
    // `captured`'s, not the previously published manifest's: comparing against
    // `published.source_set_digest` would refuse every real update, since a
    // changed source is exactly what a publish exists to record.
    // Generation/epoch still come from the published pointer: those catch the
    // CURRENT having moved under us.
    //
    // Same baseline as the staging capture, for the same reason the staged
    // rows need it: identity is derived, and a recheck that derived it
    // differently would be comparing two unrelated observations rather than
    // asking whether the source moved.
    let publish_guard = crate::workspace::acquire_workspace_lock(&workspace)
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot take the workspace lock to publish: {e}"),
        })?;
    let recheck = super::publisher::capture_while_locked(
        &workspace,
        capture_limits,
        Some(std::sync::Arc::clone(&published)),
    )
    .await?;
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
    // The two digests that SAY what this generation describes. `next` is a
    // clone of the pointer this pass started from, so without these two lines
    // it keeps naming the PREVIOUS generation's byte set while carrying the
    // new one's `source_entries` — a manifest whose digest contradicts its own
    // rows, and a `CURRENT` whose `source_set_digest` never moves. A watcher
    // that answers "did the set change?" by comparing that field then reports
    // no change forever, however many generations were published. Every other
    // field refreshed here (`coverage`, `evaluated_at`, `tombstones`,
    // `table_versions`) was carried the same way; these two were missed.
    next.source_set_digest = captured.source_set_digest.clone();
    next.config_digest = captured.config_digest.clone();
    next.diagnostics = captured.diagnostics.clone();
    // The confirmed deletes this pass produced, plus the ones earlier passes
    // produced whose keys are still absent (see `plan_reconcile`). Publishing
    // `reconcile.tombstones` alone would drop the record after one generation,
    // and a recreate two passes later would find nothing to skip past — which
    // is the identity reuse this field exists to prevent.
    next.tombstones = reconcile.tombstones;
    next.coverage = captured.coverage;
    next.evaluated_at = captured.evaluated_at.clone();
    next.table_versions = committed_versions;

    // finish_writes → sync_written_set → write_generation_manifest →
    // descriptor_for(real owner epoch) → publish_current, in that order.
    //
    // The claimed space is carried forward from the pointer this pass read at
    // its start. A manifest does not name the embedding space — the
    // descriptor does, and it outlives every manifest — so passing `None`
    // here would drop the space the rows' vectors were built in and leave
    // pinned semantic reads querying a space the descriptor does not name.
    //
    // The pending set is carried the same way: this manifest is a clone of
    // the one it read, so outstanding jobs survive a reconcile untouched and
    // the jobs an admission minted are not dropped by the next pass.
    publisher.publish(&next, descriptor.profile_id.as_deref())?;
    // The lock is released before the post-publish reads below: they are
    // observation, not part of the switch, and nothing may block on them.
    drop(publish_guard);
    drop(tables);
    // The owner is released when `publisher` drops at the end of this scope.
    // Nothing here signals a watcher. A live runtime learns of this pass
    // because it is the caller; a runtime in another process learns of it from
    // its own full-scan clock.
    //
    // `runtime::mark_dirty` exists and is real, but its only in-tree caller is
    // `forgeplan watch --constellation`, which is also the only place a
    // `MaintenanceSupervisor` is registered. `serve --auto-index` runs a
    // single pass and registers nothing, and no MCP write handler signals it
    // — so an MCP mutation during a `serve` is indexed by whatever maintenance
    // runs next, not by this signal. Stating that here is the point: a comment
    // claiming every writer announces itself would send the next reader
    // looking for wiring that does not exist.

    let new_descriptor =
        super::storage::read_current(&index_root)?.ok_or_else(|| ConstellationError::Io {
            detail: "publication left no pointer".to_string(),
        })?;
    let new_manifest = Arc::new(super::storage::read_manifest(&index_root, &new_descriptor)?);

    // Rebuild + re-verify the trigram index against the bytes this
    // generation pinned — post-publish, so a crash mid-build can at worst
    // leave an untagged index, which every read treats as absent.
    let lexical_index = rebind_lexical_index(&workspace, &new_manifest, &captured.entries);

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
            lexical_index,
        },
    ))
}

/// What [`rebind_index_identity`] changed about one physical index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebindReport {
    /// The checkout the marker claimed before the rewrite.
    pub old_checkout_id: String,
    /// The path-bound checkout id the marker now claims — always
    /// `path_bound_checkout_id(index_root)` for the root the call rebound.
    pub new_checkout_id: String,
    /// The durable epoch before the rewrite.
    pub old_epoch: u64,
    /// The durable epoch after: `old_epoch + 1`, checked. Bumping retires
    /// every pending embedding job honestly — job eligibility is the tuple
    /// `(index_epoch, checkout_id, incarnation)`, and work queued under the
    /// old epoch can never satisfy it again.
    pub new_epoch: u64,
}

/// The checkout id an index at `index_root` would mint with no marker at
/// all — `local-<sha256(root)[..16]>` over the root **as spelled**.
///
/// Pure: a function of the path's bytes, no I/O. Callers spell the root
/// exactly as a maintainer would (canonical), and use the id to reason
/// about a marker's claim *before* deciding to rewrite it — a marker whose
/// `checkout_id` equals this value belongs to this path; anything else is
/// a foreign index. This is the one piece of identity the rebind flow
/// needs outside the owner module, so it is named here rather than through
/// a wider `IndexIdentity` export.
pub fn path_bound_checkout_id(index_root: &Path) -> String {
    super::owner::IndexIdentity::path_bound_checkout_id(&super::owner::IndexLocation::new(
        index_root,
    ))
}

/// Rebind a moved index to the path it now sits at.
///
/// The marker's `checkout_id` is path-derived, so a checkout that moved
/// leaves an index claiming its old home — the next `maintain_index`
/// refuses it with `index_identity_mismatch`, loudly and correctly. This
/// is the explicit operator step that lifts the refusal: take the owner
/// (the same discipline a publisher keeps — a rewrite must never race a
/// live publish), read the marker, rewrite the identity, and put it back
/// under the platform durability barrier.
///
/// Two deliberate choices:
///
/// * `index_epoch` is **bumped**, not preserved. Eligibility for pending
///   embedding jobs is the tuple `(index_epoch, checkout_id, incarnation)`;
///   keeping the epoch would let work queued under the old checkout still
///   apply.
/// * Already-published generations are **not** rewritten. `CURRENT` and
///   its manifest keep the identity they were published under — they are
///   the record of what was true then; the marker governs what the *next*
///   publisher claims.
///
/// Returns `None` when there is nothing to rewrite: no marker at all (a
/// fresh workspace — the next publish mints its own identity), or a marker
/// already bound to this path (a re-run must not burn an epoch).
///
/// # Errors
///
/// [`ConstellationError::IndexBusy`] when another live process owns the
/// index or this process is shutting down, and the marker-read / barrier
/// errors a publisher would also report.
pub fn rebind_index_identity(
    index_root: &Path,
) -> Result<Option<RebindReport>, ConstellationError> {
    if !is_admitting() {
        return Err(ConstellationError::IndexBusy {
            detail: "this process is shutting down and admits no new maintenance".to_string(),
        });
    }
    let location = super::owner::IndexLocation::new(index_root);
    // Owner first, before the marker is read or written: the rewrite is
    // serialised against a live publisher exactly as a publish would be.
    // `acquire_owner` verifies the canonical parent itself.
    let _owner = super::owner::acquire_owner(
        &location,
        std::time::Duration::from_millis(super::owner::OWNER_ACQUIRE_BUDGET_MS),
    )?;
    let marker = match storage::read_marker(index_root)?.marker().cloned() {
        Some(marker) => marker,
        None => return Ok(None),
    };
    let new_checkout_id = super::owner::IndexIdentity::path_bound_checkout_id(&location);
    if marker.checkout_id == new_checkout_id {
        return Ok(None);
    }
    let (_support, platform, diagnostic) = super::durability::platform_barrier(index_root);
    if let Some(diagnostic) = diagnostic {
        return Err(ConstellationError::UnsupportedIndexFilesystem {
            detail: diagnostic.detail,
        });
    }
    let platform = platform.ok_or_else(|| ConstellationError::UnsupportedIndexFilesystem {
        detail: "the index root has no write barrier".to_string(),
    })?;
    let mut rewritten = marker.clone();
    let old_epoch = rewritten.index_epoch;
    rewritten.checkout_id = new_checkout_id.clone();
    rewritten.index_epoch = checked_increment("index_epoch", old_epoch)?;
    storage::write_marker(index_root, &rewritten, platform.as_ref())?;
    Ok(Some(RebindReport {
        old_checkout_id: marker.checkout_id,
        new_checkout_id,
        old_epoch,
        new_epoch: rewritten.index_epoch,
    }))
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
    /// Completions that arrived stale — the row moved between admission
    /// and completion, and the conditional update matched zero rows. Counted,
    /// not errored: stale is information, not failure.
    pub stale_completions: usize,
    /// Keys this drain durably recorded `failed` before giving up on them.
    /// Each carries an advanced `attempt` and a `retry_after`, so the next
    /// pass backs off instead of re-spending the same inference.
    pub failed: usize,
    /// Eligibility *after* the drain — the caller-visible coverage proof.
    pub after: super::jobs::EligibilityReport,
}

/// Largest number of inputs one [`prepare_embedding_batch`] call returns.
///
/// **A chosen bound, not a measurement.** The durable pending set is not
/// capped by this — every eligible key gets a job in the manifest, O(rows) —
/// so exceeding this defers work to the next pass rather than dropping it.
/// It bounds what one compute phase holds in memory at once, and it is the
/// same number the runtime's ready queue admits.
pub const MAX_PREPARED_BATCH: usize = 64;

/// The width of this index's embedding column, in the type a profile declares.
///
/// [`super::lance_tables::EMBEDDING_DIM`] is an `i32` because that is the type
/// Arrow's `FixedSizeList` width is; [`super::jobs::EmbeddingProfile::dimensions`]
/// is a `u32`. The two are the same fact in different types, so the
/// comparison is made in one named place with a **checked** conversion: a bare
/// `as u32` on a schema width that ever went negative or wider than `u32`
/// would compare equal to a profile that declares something else entirely,
/// which is the "profile says 1024, table stores 4294967295" class of bug this
/// whole tree exists to prevent.
fn embedding_column_dimensions() -> Result<u32, ConstellationError> {
    u32::try_from(super::lance_tables::EMBEDDING_DIM).map_err(|_| {
        ConstellationError::IndexProtocolMismatch {
            detail: format!(
                "this index's embedding column declares {} as its width, which is not a \
                 dimension a profile can declare",
                super::lance_tables::EMBEDDING_DIM
            ),
        }
    })
}

/// Point `CURRENT` at `profile_id` as the space this index serves.
///
/// The explicit migration step. [`prepare_embedding_batch`] admits only in the
/// space `CURRENT` already names, so an operator who installs new weights
/// changes the served space **here**, deliberately, and the admission that
/// follows finds the descriptor already agreeing with it.
///
/// # Why this is a separate call rather than a flag on admission
///
/// Because "which space does this index serve" is a decision, and a decision
/// that a routine background drain may take as a side effect is not one. If
/// admission could switch spaces, then any caller holding a profile it
/// resolved at some earlier moment — a `watch` tick, a `serve` pass, a drain
/// that stalled — would be able to move the corpus back by simply being run
/// again. Splitting the two means the only thing that can move the served
/// space is a caller that named the target profile on purpose.
///
/// # What it does and does not touch
///
/// One thing: the descriptor's `profile_id`. It writes no table row, mints no
/// job and loads no weights, so it is cheap and cannot leave a half-admitted
/// corpus — the rows are still `ready` under the old profile until the drain
/// that follows re-points them, and eligibility recomputes that honestly
/// (every row is eligible again, because its stored profile no longer matches
/// the one being admitted).
///
/// Idempotent: adopting the space already served republishes nothing and
/// returns `Ok(false)`.
///
/// # Errors
///
/// [`ConstellationError::IndexBusy`] when the process is shutting down, the
/// deadline had expired, or another process owns the index;
/// [`ConstellationError::IndexGenerationUnavailable`] when no published
/// generation exists — the queue never bootstraps;
/// [`ConstellationError::ProfileIncompatible`] when the profile's dimensions
/// disagree with this index's embedding column.
///
/// Returns `true` when `CURRENT` moved, `false` when it already named
/// `profile_id`.
pub async fn adopt_embedding_profile(
    capability: &MaintainCapability,
    profile: &super::jobs::EmbeddingProfile,
    deadline: std::time::Instant,
) -> Result<bool, ConstellationError> {
    use super::publisher::IndexPublisher;

    if !is_admitting() {
        return Err(ConstellationError::IndexBusy {
            detail: "this process is shutting down and admits no new maintenance".to_string(),
        });
    }
    if std::time::Instant::now() >= deadline {
        return Err(ConstellationError::IndexBusy {
            detail: "the adoption deadline had already expired; the served space is unchanged"
                .to_string(),
        });
    }
    let column_dimensions = embedding_column_dimensions()?;
    if profile.dimensions != column_dimensions {
        return Err(ConstellationError::ProfileIncompatible {
            detail: format!(
                "the resolved profile declares {} dimensions; this index's embedding column is \
                 {} wide. Adopting it would point semantic reads at a space this index cannot \
                 store.",
                profile.dimensions, column_dimensions
            ),
        });
    }

    let index_root = capability.index_root().to_path_buf();
    let workspace = capability.workspace().to_path_buf();
    let profile_id = profile.profile_id();

    let publisher = IndexPublisher::acquire(&index_root, inner_store_for(&index_root))?;
    let descriptor = super::storage::read_current(&index_root)?.ok_or_else(|| {
        ConstellationError::IndexGenerationUnavailable {
            detail: "no published generation; run index maintenance first".to_string(),
        }
    })?;
    // Read under the owner so the decision is made against the pointer that is
    // actually current, and so no other publisher can move it between this
    // read and the republish below.
    if descriptor.profile_id.as_deref() == Some(profile_id.as_str()) {
        return Ok(false);
    }

    let mut next = descriptor;
    next.profile_id = Some(profile_id);
    // Under the workspace lock, like every other pointer switch: a source
    // mutation landing between the decision and the switch would otherwise be
    // invisible to the descriptor this republish installs.
    let guard = crate::workspace::acquire_workspace_lock(&workspace)
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot take the workspace lock to adopt a profile: {e}"),
        })?;
    publisher.republish_current(&next)?;
    drop(guard);
    drop(publisher);
    Ok(true)
}

/// Admission: decide what needs computing, put the rows in `pending`, and
/// hand back the exact inputs — with **no** owner held afterwards.
///
/// # The three phases, and why they are three
///
/// ```text
/// prepare (owner held)  →  compute (no owner, no lock, model thread)
///                       →  complete (owner reacquired, per-job guard)
/// ```
///
/// The model call is the slow part and it is native; holding an OS-wide
/// exclusive owner across it would serialise every other process on the
/// machine behind one inference. So admission takes the owner, records
/// everything a later publish needs to be safe, and **releases it before
/// returning**. Nothing between here and [`complete_embedding_batch`] holds a
/// lock of any kind.
///
/// # What admission durably records
///
/// For each eligible row: the target profile and `pending` are written
/// ([`super::lance_tables::IndexTables::admit_embedding_into_artifacts`]), and
/// a new immutable manifest carries one [`super::jobs::EmbeddingJob`] per
/// eligible key in `pending_jobs`. Three consequences, each load-bearing:
///
/// * the profile term in [`super::jobs::Binding::predicate`] can be compared
///   exactly, because the row already stores the target profile;
/// * a crash before the compute leaves `pending` behind rather than a row
///   that looks current;
/// * a restart re-derives the same jobs from the same capture, because
///   eligibility is recomputed, never read out of the manifest.
///
/// # A pending job that still describes the same work is kept
///
/// An eligible key whose pending job describes the same binding keeps that
/// job — same `captured_input_ref`, same `attempt`, same `retry_after`. Two
/// consequences, both required:
///
/// * an unrelated reconcile between passes no longer mints a new generation
///   for the same outstanding work, so a batch prepared before that reconcile
///   is still completable afterwards;
/// * a key that has failed does not get a fresh attempt counter every pass,
///   so its backoff is real rather than reset by the next admission.
///
/// A key inside its retry delay is not re-admitted at all: it stays pending
/// and its row stays `failed`. That is what stops a drain from spinning on a
/// permanently failing model.
///
/// # Retired keys
///
/// A pending job for a key this capture does not describe — a deleted
/// document, or a section whose parent is gone — is dropped, but **only when
/// the capture is complete**. "Not described" is not evidence of absence under
/// an incomplete capture: an unreadable or identity-colliding artifact is
/// absent from `entries` while its row is still `pending`, and retiring its
/// job would destroy the attempt history and reset its backoff to zero. A row
/// that is present but unreadable is `CannotJudge` in the eligibility report
/// and is never dropped either way.
///
/// # Eligibility is the work list, and it is scalar
///
/// `artifacts_eligibility` / `sections_eligibility` compare stored binding
/// fields; they never run a vector query (see
/// [`super::jobs::eligibility_for`]). A row already current is **not**
/// rewritten — an admission that touched it would move a live row's state for
/// no reason.
///
/// # What the caller does with the result
///
/// Hands `prepared.inputs`' texts to an inference worker, then hands the
/// results to [`complete_embedding_batch`]. The texts are `Arc<str>` and come
/// from this capture; nothing downstream re-reads the files.
///
/// # Errors
///
/// [`ConstellationError::IndexBusy`] when the process is shutting down, the
/// deadline had already expired, or another process owns the index;
/// [`ConstellationError::IndexGenerationUnavailable`] when no publishable
/// generation exists (the queue never bootstraps);
/// [`ConstellationError::ProfileIncompatible`] when the profile's verified
/// dimensions disagree with the table this build writes.
///
/// Not feature-gated: nothing here loads weights or resolves a profile. The
/// caller supplies a verified [`super::jobs::EmbeddingProfile`] — a plain data
/// type that exists in every build — so the whole admission and completion
/// contract is exercisable without `semantic-search`, which is what lets these
/// tests run on a machine that has no model at all.
pub async fn prepare_embedding_batch(
    capability: &MaintainCapability,
    profile: &super::jobs::EmbeddingProfile,
    deadline: std::time::Instant,
) -> Result<super::jobs::PreparedEmbeddingBatch, ConstellationError> {
    use super::jobs::{EmbeddingJob, PreparedEmbeddingBatch, PreparedEmbeddingInput};
    use super::lance_tables::open_active_tables;
    use super::publisher::{IndexPublisher, capture_under_workspace_lock};

    if !is_admitting() {
        return Err(ConstellationError::IndexBusy {
            detail: "this process is shutting down and admits no new maintenance".to_string(),
        });
    }
    if std::time::Instant::now() >= deadline {
        return Err(ConstellationError::IndexBusy {
            detail: "the admission deadline had already expired; no work was started".to_string(),
        });
    }
    let column_dimensions = embedding_column_dimensions()?;
    if profile.dimensions != column_dimensions {
        return Err(ConstellationError::ProfileIncompatible {
            detail: format!(
                "the resolved profile declares {} dimensions; this index's embedding column is \
                 {} wide. Publishing into it would write a vector no reader can decode.",
                profile.dimensions, column_dimensions
            ),
        });
    }

    let index_root = capability.index_root().to_path_buf();
    let workspace = capability.workspace().to_path_buf();
    let profile_id = profile.profile_id();

    // The owner comes **before** CURRENT. Every decision below — which
    // generation this is, which rows are eligible, what the pending set is —
    // describes state another process is free to move the moment the owner is
    // released. Reading the pointer first and acquiring afterwards admits work
    // from a generation that was already superseded, and the admission write
    // then lands on tables the new generation has already pinned.
    let publisher = IndexPublisher::acquire(&index_root, inner_store_for(&index_root))?;

    // The queue never bootstraps: an index that is not yet publishable is
    // `maintain_index`'s job, and creating one here would hide the
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

    // The caller resolved its profile **before** it got here. Between that
    // resolution and this line another process may have re-embedded the
    // corpus under a newer space and republished `CURRENT` naming it — and a
    // stale caller would otherwise re-point every eligible row at the old
    // space, republish `CURRENT` back onto it, and start a full re-embed of
    // the corpus into weights the operator has already replaced.
    //
    // `Binding::predicate` fences such a caller's *completion* (its job is no
    // longer the pending one, so the guarded update matches zero rows), but
    // admission is what moves the rows in the first place: by the time the
    // late result is refused the migration it started has already been
    // written. So the refusal belongs here.
    //
    // Admission may only work in the space `CURRENT` already publishes.
    //
    // `None` — a generation that never claimed a space — admits anything:
    // that is a first claim, not a move.
    //
    // Migrating to a new space is therefore an explicit act, and it happens
    // before this call: the operator's `forgeplan index --embed-profile <id>`
    // validates the id against the cached snapshot it resolved itself and
    // republishes `CURRENT` naming that space, so by the time admission runs
    // the descriptor and the caller agree. Nothing has to be exempted here to
    // let that happen, and nothing can be — which is the property that makes
    // this rule safe: a caller holding any *other* profile resolved it before
    // that migration, and admitting on it would re-point every eligible row
    // back, republish `CURRENT` onto the replaced weights, and start a full
    // re-embed of the corpus into a space the operator already moved off.
    if let Some(published_profile) = descriptor.profile_id.as_deref()
        && published_profile != profile_id
    {
        return Err(ConstellationError::ProfileIncompatible {
            detail: format!(
                "this index currently publishes embedding space `{published_profile}`, but \
                 admission was asked for `{profile_id}`. A different space is reached through \
                 `forgeplan index --embed-profile <profile-id>`, which resolves the profile from \
                 the cached snapshot it verified and republishes what this index serves; a caller \
                 still holding another one resolved it before that migration and would move the \
                 corpus back to weights this index no longer serves."
            ),
        });
    }

    let marker = marker_for(&published);
    let tables =
        open_active_tables(&index_root, &marker, Arc::clone(publisher.tracking_store())).await?;

    // Recover the baseline before anything reads the tables for a decision,
    // exactly as `maintain_index` does: a head an abandoned pass left behind is
    // not the published state, and an admission that read it would record a
    // job for bytes this index does not serve.
    let baseline = published_baseline(&descriptor, &published)?;
    restore_diverged_heads(&tables, &baseline).await?;

    // Capture under the workspace lock; the lock is released before any write,
    // exactly as the merge stage does it.
    //
    // With the published manifest as the identity baseline: every job minted
    // below is bound to `captured`'s `(incarnation, row_revision, digests)`,
    // and the guarded completion predicates on the row's stored copy of that
    // same tuple. A capture without the baseline would derive `1` for a row
    // the table holds at any other revision, so every job would be born
    // matching nothing.
    let captured = capture_under_workspace_lock(
        &workspace,
        super::capture::CaptureLimits::default(),
        Some(Arc::clone(&published)),
    )
    .await?;

    // The prefix comes from the config this capture read, not from a caller
    // argument: the `input_digest` in the binding was computed over exactly
    // that bound, and a different one would produce text whose digest the row
    // never recorded.
    let body_prefix_chars = crate::workspace::init::load_config(&workspace)
        .ok()
        .and_then(|config| config.embedding.map(|embedding| embedding.chunk_size))
        .unwrap_or(super::jobs::DEFAULT_BODY_PREFIX_CHARS);

    let mut eligibility = tables.artifacts_eligibility(&captured, &profile_id).await?;
    eligibility.merge_from(&tables.sections_eligibility(&captured, &profile_id).await?);

    // The capture must describe the config this generation was published
    // under. If it does not, the `input_digest`s in the rows were computed
    // over a different `chunk_size`, and every binding this admission mints
    // would be judged against digests it cannot satisfy. A reconcile has to
    // record the new config first.
    if captured.config_digest != published.config_digest {
        return Err(ConstellationError::ProfileIncompatible {
            detail: format!(
                "the capture sees config digest `{}` but generation `{}` was published under \
                 `{}`; run index maintenance before draining the queue",
                captured.config_digest, published.generation_id, published.config_digest
            ),
        });
    }

    // The pending set this admission will publish, and the generation it will be
    // published under. The generation id is minted **before** the jobs are
    // built, because `captured_input_ref` names it — the ref is the identity
    // of the right to publish, and a job whose ref names a generation that was
    // never published names nothing.
    let admission_generation =
        admission_generation_id(&captured.evaluated_at, &profile_id, &published.pending_jobs);
    let mut jobs: std::collections::BTreeMap<String, super::jobs::EmbeddingJob> =
        published.pending_jobs.clone();
    let mut inputs: Vec<PreparedEmbeddingInput> = Vec::new();
    let now = chrono::Utc::now();

    // Retire the pending jobs this capture can no longer describe — and only
    // from a **complete** capture. A key the capture does not contain is a
    // deleted document, or a section whose parent is gone, and its job names
    // work no completion could ever match; carrying it forward would leave
    // the manifest naming work that can only ever be reported as a gap.
    //
    // "Not contained" means nothing at all when the capture is incomplete.
    // An artifact that is momentarily unreadable, whose frontmatter does not
    // parse, or whose identity collides appears in `unusable` rather than in
    // `entries`, and its row is still sitting in the table as `pending` with
    // its profile set. Retiring from there would delete the durable work
    // record and — because the row is still eligible — the next admission
    // would mint the key a **fresh** job at `attempt: 0` with no
    // `retry_after`. A key that had earned a 32-minute backoff over five
    // attempts would be re-attempted immediately, on the very model failure
    // the backoff existed to survive.
    //
    // So retirement is gated on `captured.is_complete()`, the same gate
    // `reconciler`'s delete planning uses: a partial scan is a diagnostic,
    // never a confirmed delete. A row that is present but unreadable is
    // `CannotJudge` in the eligibility report above and is never touched
    // either way; its job simply survives to the next pass.
    if captured.is_complete() {
        let captured_keys: std::collections::BTreeSet<&str> = captured
            .entries
            .iter()
            .map(|entry| entry.canonical_key.as_str())
            .chain(
                captured
                    .sections
                    .iter()
                    .filter(|section| {
                        captured
                            .entries
                            .iter()
                            .any(|entry| entry.canonical_key == section.parent_key)
                    })
                    .map(|section| section.canonical_key.as_str()),
            )
            .collect();
        jobs.retain(|key, _| captured_keys.contains(key.as_str()));
    }

    for key in eligibility.eligible_keys() {
        let Some((binding, input_text)) =
            admitted_binding_and_input(&captured, key, &profile_id, body_prefix_chars)
        else {
            // Eligible but not bindable from this capture. It is left for the
            // next admission rather than guessed at, and the eligibility
            // report's `cannot_judge` half is where that is said out loud.
            continue;
        };

        // An already-pending job over the same binding is the same work, so it
        // is **kept, not reminted**: a fresh `captured_input_ref` and a fresh
        // attempt counter on every pass would reset a failing key's history
        // and publish a new generation each time an unrelated reconcile
        // republishes. It also keeps a batch prepared before that reconcile
        // completable — the pending token it carries is still the one this
        // index has.
        let retained = jobs.get(key).filter(|job| job.describes(&binding)).cloned();
        let job = match retained {
            Some(job) => job,
            None => EmbeddingJob::for_binding(&binding, &admission_generation)?,
        };
        if !job.retry_is_due(now) {
            // Inside its retry delay. The job stays pending exactly as it is
            // — it was retained from the manifest, not minted — and the row
            // stays `failed`. Re-admitting it now would spend the same failing
            // inference over and over inside one drain.
            continue;
        }

        // One workspace lock per admission write, released before the next —
        // the same one-lock-per-commit rule the merge stage follows.
        let guard = crate::workspace::acquire_workspace_lock(&workspace)
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("cannot take the workspace lock for an admission write: {e}"),
            })?;
        let admitted = match job.row_role() {
            super::jobs::EmbeddingRowRole::Artifact => {
                tables.admit_embedding_into_artifacts(&binding).await?
            }
            super::jobs::EmbeddingRowRole::Section => {
                tables.admit_embedding_into_sections(&binding).await?
            }
        };
        drop(guard);

        if admitted.rows_updated == 0 {
            // The row moved between the eligibility scan and this write. Not
            // an error: the next admission sees the new state, and the row is
            // not in `jobs` because nothing was admitted for it.
            continue;
        }
        // The job recorded under this key, whether newly minted or carried
        // forward from the manifest above. Inserted only once the row actually
        // took the write, so a row that moved between the scan and the write
        // names no job.
        jobs.insert(key.to_string(), job.clone());
        // Beyond the batch bound the job is still admitted and still durable;
        // only the *text* is deferred, so the next prepare hands it over. A
        // deferred job is not a dropped one.
        if inputs.len() < MAX_PREPARED_BATCH {
            inputs.push(PreparedEmbeddingInput {
                job,
                input_text: std::sync::Arc::from(input_text),
            });
        }
    }

    // Publish the new pending set as an immutable manifest **before** the
    // compute starts. A crash between here and completion then leaves durable
    // evidence of what was outstanding, rather than rows that look pending
    // with nothing naming them.
    //
    // Only when the pending set actually moved. A manifest is immutable, so
    // republishing an identical one would mint a second generation id for the
    // same work on every pass — generations every reader must checksum and
    // `gc_generations` must reason about, bought for nothing. When nothing
    // moved, the batch still names the generation the pending jobs are
    // recorded under, which is the one `CURRENT` already serves.
    let admission_generation = if jobs != published.pending_jobs {
        let next = next_manifest_with_pending(
            &published,
            &admission_generation,
            jobs,
            tables.head_versions().await?,
        );
        // The pointer switch happens under the workspace lock, like every
        // other one: the admission writes above each took and released their
        // own lock, and a workspace mutation landing between them and this
        // publish would be invisible to the generation that names the pending
        // set. Nothing is written inside — the manifest is already assembled.
        let guard = crate::workspace::acquire_workspace_lock(&workspace)
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("cannot take the workspace lock to publish admission: {e}"),
            })?;
        publisher.publish(&next, Some(&profile_id))?;
        drop(guard);
        admission_generation
    } else {
        published.generation_id.clone()
    };

    drop(tables);
    drop(publisher);

    Ok(PreparedEmbeddingBatch {
        admission_generation,
        config_digest: captured.config_digest.clone(),
        profile: profile.clone(),
        inputs,
    })
}

/// A generation id for an admission, distinct from every other one.
///
/// `generation_id_for` alone would collide: two admissions inside the same
/// millisecond would mint the same id, and the second `write_generation_manifest`
/// would refuse an immutable generation it had already written. The digest
/// binds the id to what this admission is *about*.
///
/// The timestamp alone is **not** enough, and its millisecond resolution is
/// the reason: two passes under the same profile can share one, and each of
/// them moves the pending set, so the second would collide with the first's
/// freshly written generation and fail the whole pass with
/// `index_protocol_mismatch`. `pending_set` is the pending map the pass starts
/// from — the work this one is about to replace — so two passes that describe
/// the same work are the same generation by construction, and two that
/// describe different work cannot collide even inside one millisecond. Under
/// the index owner the second pass necessarily reads the first pass's
/// published pending set, so the second term is what makes the pair distinct.
fn admission_generation_id(
    evaluated_at: &str,
    profile_id: &str,
    pending_set: &std::collections::BTreeMap<String, super::jobs::EmbeddingJob>,
) -> String {
    let mut digest = super::DigestBuilder::new(ADMISSION_GENERATION_DOMAIN);
    digest
        .field(evaluated_at.as_bytes())
        .field(profile_id.as_bytes());
    // Sorted by key already (`BTreeMap`), so the encoding is one fixed
    // sequence. Every job field that decides what the work *is* is fed:
    // which key, which input it was captured from, how many attempts it has
    // survived, and when it may be tried again. `field` length-prefixes each
    // one, so no two different pending sets can encode to the same bytes.
    for (key, job) in pending_set {
        digest
            .field(key.as_bytes())
            .field(job.captured_input_ref.as_bytes())
            .field(&job.attempt.to_be_bytes())
            .field(job.retry_after.as_deref().unwrap_or("").as_bytes())
            .field(job.profile_id.as_bytes());
    }
    format!(
        "{}-adm-{}",
        super::publisher::generation_id_for(evaluated_at),
        digest.finish_hex()
    )
}

/// Domain separator for [`admission_generation_id`]. Versioned, so a change to
/// what goes into an admission id cannot silently collide with an older one.
const ADMISSION_GENERATION_DOMAIN: &str = "forgeplan.constellation.admission-generation.v2";

/// The binding and the exact input text for one eligible captured key.
///
/// `None` when the key is not in the capture, or when the section's parent is
/// absent — the two states in which no binding can be built and nothing may
/// be published. Returning `None` rather than an error keeps a single
/// unreadable row from aborting an otherwise sound admission; the row stays
/// eligible and is reported again next pass.
fn admitted_binding_and_input(
    captured: &CapturedSourceView,
    key: &str,
    profile_id: &str,
    body_prefix_chars: usize,
) -> Option<(super::jobs::Binding, String)> {
    if let Some(artifact) = captured.artifact(key) {
        let binding = super::publisher::binding_for(
            captured
                .entries
                .iter()
                .find(|entry| entry.canonical_key == key)?,
            profile_id,
        )
        .ok()?;
        let text =
            super::jobs::embedding_input_text(&artifact.title, &artifact.body, body_prefix_chars);
        return Some((binding, text));
    }
    let section = captured.section(key)?;
    let parent = captured
        .entries
        .iter()
        .find(|entry| entry.canonical_key == section.parent_key)?;
    let binding = super::publisher::binding_for_section(section, parent, profile_id).ok()?;
    Some((binding, captured.section_input(section, body_prefix_chars)))
}

/// The manifest admission publishes: same generation identity and source set,
/// new table versions, new pending set.
///
/// Kept as a separate function so "what admission changes about a manifest"
/// is one readable expression rather than a field assignment buried in a
/// write path.
fn next_manifest_with_pending(
    published: &GenerationManifest,
    admission_generation: &str,
    pending_jobs: std::collections::BTreeMap<String, super::jobs::EmbeddingJob>,
    table_versions: super::manifest::TableVersions,
) -> GenerationManifest {
    let mut next = published.clone();
    next.generation_id = admission_generation.to_string();
    next.table_versions = table_versions;
    next.pending_jobs = pending_jobs;
    next
}

/// Completion: write back vectors computed from a [`super::jobs::PreparedEmbeddingBatch`].
///
/// Reacquires the owner, re-reads `CURRENT` and the manifest, and refuses to
/// write anything unless the batch is still the one this index is serving.
/// Then every result is matched **exactly** against the job it was computed
/// for, and the conditional update runs under the row's own stored binding.
///
/// # What is re-checked, and what each check prevents
///
/// | Check | Prevents |
/// |---|---|
/// | `CURRENT.config_digest == prepared.config_digest` | vectors computed under a config the index no longer serves |
/// | `CURRENT.profile_id` is this batch's space (see [`fence_served_profile`]) | a late completion reverting an [`adopt_embedding_profile`] migration — its rows, jobs and config digest all still pass the checks below, because adoption deliberately touches none of them |
/// | the job is byte-identical to the one in `pending_jobs` | a late A landing after a B admission re-pointed the same key — reported stale, the rest of the batch still lands |
/// | the capture still describes the key with this binding | a delete/recreate between admission and completion |
/// | `Binding::predicate` at the row | any of the above, atomically, at write time |
///
/// The generation id is deliberately **not** a check. An unrelated reconcile
/// between admission and completion republishes the manifest carrying the
/// pending set forward; refusing on the id alone would turn ordinary
/// background maintenance into a lost batch. The pending job is the gate.
///
/// The capture re-read is for **verification only**. No binding is ever built
/// from it: the binding that guards the write comes from the job, which came
/// from admission. A result whose row moved is reported `stale` with zero
/// rows updated — it is not rebindded onto whatever the row is now.
///
/// # Unknown and duplicate keys
///
/// Both are refused **before the first write**, not partway through: a batch
/// that names a key this admission never admitted, or names one twice, is a
/// caller bug, and writing the half of it that happens to match would make a
/// bug look like progress.
///
/// # Superseded results are stale, not fatal
///
/// A result whose job is no longer the pending one is reported in `stale` and
/// the rest of the batch completes. Its key's pending job belongs to the
/// admission that replaced it, so it is left in place rather than retired.
///
/// # Errors
///
/// [`ConstellationError::Io`] for a malformed result batch;
/// [`ConstellationError::IndexBusy`] on a shutdown or expired deadline;
/// [`ConstellationError::ModelUnavailable`] for a wrong-dimension or
/// non-finite vector.
///
/// Not feature-gated, for the same reason as [`prepare_embedding_batch`]: the
/// vectors arrive already computed, so no model is involved on this side of
/// the fence at all.
pub async fn complete_embedding_batch(
    capability: &MaintainCapability,
    prepared: &super::jobs::PreparedEmbeddingBatch,
    results: &[super::jobs::ComputedEmbedding],
    deadline: std::time::Instant,
) -> Result<EmbeddingBatchOutcome, ConstellationError> {
    use super::lance_tables::open_active_tables;
    use super::publisher::{IndexPublisher, capture_under_workspace_lock, publish_embeddings};

    if !is_admitting() {
        return Err(ConstellationError::IndexBusy {
            detail: "this process is shutting down and admits no new maintenance".to_string(),
        });
    }
    if std::time::Instant::now() >= deadline {
        return Err(ConstellationError::IndexBusy {
            detail: "the completion deadline had already expired; nothing was written".to_string(),
        });
    }

    // Every check that needs no I/O happens first, so a malformed call never
    // reaches the owner at all.
    let accepted = accepted_results(prepared, results)?;

    let index_root = capability.index_root().to_path_buf();
    let workspace = capability.workspace().to_path_buf();
    let profile_id = prepared.profile.profile_id();

    let publisher = IndexPublisher::acquire(&index_root, inner_store_for(&index_root))?;
    let descriptor = super::storage::read_current(&index_root)?.ok_or_else(|| {
        ConstellationError::IndexGenerationUnavailable {
            detail: "the index published no generation; there is nothing to complete against"
                .to_string(),
        }
    })?;
    let published = Arc::new(super::storage::read_manifest(&index_root, &descriptor)?);

    if descriptor.config_digest != prepared.config_digest {
        return Err(ConstellationError::ProfileIncompatible {
            detail: format!(
                "this batch was admitted against config digest `{}` but the index now serves \
                 `{}`",
                prepared.config_digest, descriptor.config_digest
            ),
        });
    }

    // The served space is a fence, checked under the same owner and before the
    // first write. Everything below it — the pending-job gate, the binding
    // comparison, the row guard — passes for a batch admitted under a space
    // that has since been explicitly replaced, because the rows and the
    // pending jobs were left exactly as that batch left them. Refusing here is
    // what stops a late completion from reverting the migration.
    fence_served_profile(&descriptor, &profile_id, "this completion")?;

    // The generation itself is **not** the gate. An unrelated reconcile
    // between admission and completion republishes the manifest — carrying the
    // pending set forward — and refusing on that alone would turn ordinary
    // background maintenance into a lost batch. The gate is the pending job:
    // a result whose job is not in the current pending set, or is not
    // *identical* to the one there, was computed for a superseded admission,
    // and applying it would migrate a row a newer admission has re-pointed.
    //
    // Superseded is **stale, not fatal**. A mixed batch — three live results
    // and one from an admission that a profile migration replaced — is the
    // normal outcome of a queue that shares a workspace with a reconcile, and
    // aborting it would discard the three live vectors to protect the fourth.
    // The stale result's own pending job belongs to whoever superseded it, so
    // it is reported and left in place: retiring it would retire work this
    // batch never did.
    let mut superseded_keys: Vec<String> = Vec::new();
    let mut admitted: Vec<super::jobs::ComputedEmbedding> = Vec::with_capacity(accepted.len());
    for result in &accepted {
        match published.pending_jobs.get(&result.job.canonical_key) {
            Some(current) if current == &result.job => admitted.push(result.clone()),
            // Either a different job is pending for this key, or none is: both
            // mean this result was computed for work this index no longer has
            // outstanding.
            Some(_) | None => superseded_keys.push(result.job.canonical_key.clone()),
        }
    }

    let marker = marker_for(&published);
    let tables =
        open_active_tables(&index_root, &marker, Arc::clone(publisher.tracking_store())).await?;

    // Recover the published baseline before the tables are read for a decision
    // or written to. A completion publishes a vector **and** a manifest, so it
    // has the same obligation as a reconcile: what the tables physically hold
    // must be the published state first, or the vectors land on top of rows an
    // abandoned pass wrote and the manifest records heads that include them.
    let baseline = published_baseline(&descriptor, &published)?;
    restore_diverged_heads(&tables, &baseline).await?;

    // The same baseline admission used, for the same reason: the check below
    // compares this capture's binding against the job's, and the guarded
    // update compares the job's against the row's. Three views of one
    // identity only agree if all three were derived from the same published
    // state — so a row whose source moved is reported `stale` rather than
    // being misjudged as unchanged.
    let captured = capture_under_workspace_lock(
        &workspace,
        super::capture::CaptureLimits::default(),
        Some(Arc::clone(&published)),
    )
    .await?;

    let body_prefix_chars = crate::workspace::init::load_config(&workspace)
        .ok()
        .and_then(|config| config.embedding.map(|embedding| embedding.chunk_size))
        .unwrap_or(super::jobs::DEFAULT_BODY_PREFIX_CHARS);

    // Verification only: does the live capture still describe each key with
    // the binding the result carries? No binding is built from it, and a
    // mismatch is reported as `stale` rather than silently dropped — "the row
    // moved" is the answer to "why did my vector not land", and dropping it
    // would leave the caller guessing.
    let mut eligible_now: Vec<String> = Vec::new();
    let mut moved: Vec<String> = Vec::new();
    let mut cannot_judge: Vec<(String, String)> = Vec::new();
    let mut live: Vec<super::jobs::ComputedEmbedding> = Vec::with_capacity(admitted.len());
    for result in &admitted {
        let key = result.job.canonical_key.as_str();
        let Some((binding, _)) =
            admitted_binding_and_input(&captured, key, &profile_id, body_prefix_chars)
        else {
            cannot_judge.push((
                key.to_string(),
                "the key is no longer in the capture, so its binding cannot be confirmed"
                    .to_string(),
            ));
            continue;
        };
        if binding != result.binding() {
            // The row now describes different bytes than the vector was
            // computed from. Reported stale, never rebound — and the row
            // guard would refuse the write even if it were submitted.
            moved.push(key.to_string());
            continue;
        }
        eligible_now.push(key.to_string());
        live.push(result.clone());
    }

    // One lock for the whole conditional publish, held across exactly the
    // writes that must not interleave with a workspace mutation.
    let guard = crate::workspace::acquire_workspace_lock(&workspace)
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot take the workspace lock for a completion publish: {e}"),
        })?;
    let publishes = publish_embeddings(&tables, &live).await?;
    drop(guard);

    let mut published_keys = Vec::new();
    // A row the guard refused and a row the capture check found moved are the
    // same fact seen two ways — "the state this vector was computed for no
    // longer exists" — and both are reported as stale.
    //
    // Kept apart from `superseded_keys`: these jobs *are* this batch's and are
    // spent, so they are retired below. A superseded key's pending job belongs
    // to whoever superseded it and must survive untouched.
    let mut spent_keys: Vec<String> = moved;
    let mut artifacts_embedded = 0usize;
    let mut sections_embedded = 0usize;
    for outcome in &publishes {
        if outcome.published() {
            published_keys.push(outcome.binding.canonical_key.clone());
            if super::jobs::section_parent_key(&outcome.binding.canonical_key).is_some() {
                sections_embedded += 1;
            } else {
                artifacts_embedded += 1;
            }
        } else {
            spent_keys.push(outcome.binding.canonical_key.clone());
        }
    }

    // Coverage as the index holds it, not as the batch hoped: a report read
    // under this same owner, after the writes.
    let mut after = tables.artifacts_eligibility(&captured, &profile_id).await?;
    after.merge_from(&tables.sections_eligibility(&captured, &profile_id).await?);
    for (key, reason) in &cannot_judge {
        after.record(
            key.clone(),
            super::jobs::Eligibility::CannotJudge {
                reason: reason.clone(),
            },
        );
    }
    let cannot_judge = after
        .cannot_judge()
        .into_iter()
        .map(|(key, reason)| (key.to_string(), reason.to_string()))
        .collect();

    // Retire every job this pass accounted for — published, refused by the row
    // guard, or dropped because the row had already moved. All three are
    // *spent*: the work was attempted against a state that no longer exists.
    // Leaving any of them pending would make the next admission believe the
    // work is still outstanding, and a crash-to-restart would re-run it
    // forever.
    //
    // A **superseded** result is not in this set. Its pending job names a
    // different admission's binding and belongs to whoever replaced it —
    // retiring it here would delete a live admission's outstanding work.
    let mut remaining = published.pending_jobs.clone();
    for outcome in &publishes {
        remaining.remove(&outcome.binding.canonical_key);
    }
    for key in &spent_keys {
        remaining.remove(key);
    }
    if remaining != published.pending_jobs {
        let next = next_manifest_with_pending(
            &published,
            // A **new** immutable generation, not the one this pass read:
            // `write_generation_manifest` refuses to rewrite a generation that
            // exists, and the pending set genuinely moved.
            &admission_generation_id(
                &super::publisher::now(),
                &profile_id,
                &published.pending_jobs,
            ),
            remaining,
            tables.head_versions().await?,
        );
        // One lock for the switch, released before the republish below, which
        // takes its own. Never two at once: the lock is not reentrant.
        let guard = crate::workspace::acquire_workspace_lock(&workspace)
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("cannot take the workspace lock to publish completion: {e}"),
            })?;
        // The fence above already established that the served space *is*
        // `profile_id` (or that none is claimed yet), so passing it here can
        // only carry the served space forward or make the first claim. The
        // publisher is still held, so no other pass moved the pointer since
        // the fence read it.
        publisher.publish(&next, Some(&profile_id))?;
        drop(guard);
    }

    // The heads this publish moved and the space it claims live on the
    // descriptor: the manifest is immutable and checksum-pinned, so the next
    // reconcile would otherwise read the movement as divergence and restore
    // over the vectors just written. The publisher is still held, so nothing
    // else can move the heads first.
    //
    // Read after the manifest publish above, which advanced the heads itself:
    // the descriptor must name the head the index actually has.
    let descriptor = super::storage::read_current(&index_root)?.ok_or_else(|| {
        ConstellationError::IndexGenerationUnavailable {
            detail: "the manifest publish left no pointer to record the heads on".to_string(),
        }
    })?;
    let mut next = descriptor.clone();
    if artifacts_embedded > 0 {
        next.embedding_artifacts_head = Some(tables.artifacts_version().await?);
    }
    if sections_embedded > 0 {
        next.embedding_sections_head = Some(tables.sections_version().await?);
    }
    // The claimed space is **carried forward**, never written from a batch.
    // `fence_served_profile` already established that this index serves
    // `profile_id` (or names no space yet), so this line can only ever
    // establish a first claim or re-state the one the fence checked — it
    // cannot move the pointer onto the space a batch happens to carry. That is
    // the property `adopt_embedding_profile` exists to own, and it stays owned
    // there.
    if next.profile_id.is_none()
        && (artifacts_embedded + sections_embedded > 0 || !after.current_keys().is_empty())
    {
        next.profile_id = Some(profile_id.clone());
    }
    if next != descriptor {
        // The heads moved with no manifest behind them, so this republish is a
        // pointer switch too — under the workspace lock, for the same reason
        // the manifest publish above is.
        let guard = crate::workspace::acquire_workspace_lock(&workspace)
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("cannot take the workspace lock to record heads: {e}"),
            })?;
        publisher.republish_current(&next)?;
        drop(guard);
    }

    drop(tables);
    drop(publisher);

    // Stale is one list, because the caller asks one question: which of my
    // vectors did not land. Supersession is one of three reasons, and they are
    // not distinguished in the type — `cannot_judge` is where "why" is
    // answered per key, and a superseded key is a *reported* outcome of this
    // batch, not a row the index could not judge.
    let mut stale_keys = spent_keys;
    stale_keys.extend(superseded_keys);

    Ok(EmbeddingBatchOutcome {
        profile_id,
        eligible_keys: eligible_now,
        published: published_keys,
        stale: stale_keys,
        cannot_judge,
    })
}

/// What one recorded model failure did.
///
/// `failed` names the rows this pass actually moved to `failed`; a row whose
/// guard matched nothing is in `skipped` instead, because "the row moved while
/// the model was failing" and "this key failed" are different facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingFailureOutcome {
    pub profile_id: String,
    /// Keys durably marked `failed`, with the attempt they are now at.
    pub failed: Vec<String>,
    /// Keys whose row no longer matched the job, so nothing was written.
    pub skipped: Vec<String>,
    /// The instant every failed key may be attempted again, when any failed.
    pub retry_after: Option<String>,
}

/// Record that a model's attempt at this batch's inputs failed.
///
/// The third leg of the fence, and the one that makes a failing model
/// **visible** instead of invisible. Before it, a worker that died left every
/// admitted row `pending` with `attempt: 0`: the next admission handed the same
/// inputs straight back, the next attempt burned the same inference, and a
/// permanently broken model produced an endless queue that looked like work.
///
/// What this writes:
///
/// * `semantic_state = 'failed'` and `retry_after` on each row, under the
///   **job's own** binding guard — so a row that moved is left alone;
/// * `attempt` advanced on the pending job, which is the only durable count of
///   how many times a key has been tried;
/// * `diagnostics` on the artifacts row, so "this row failed" is answerable
///   from the index rather than from a log nobody kept;
/// * a new manifest carrying the advanced jobs — **still pending**. A failure
///   is not a retirement: the key keeps its place in the durable work list and
///   comes back after its delay.
///
/// Nothing here ever marks a row `ready` or touches `embedding`. A failed
/// recompute leaves any previous vector in place, hidden behind `failed`,
/// which is exactly the state [`super::jobs::SemanticState`] documents.
///
/// # No busy loop
///
/// [`prepare_embedding_batch`] skips a key whose pending job is still inside
/// its retry delay, so a drain that records a failure does not immediately
/// re-admit the same inputs and spin. The delay doubles per attempt and is
/// capped, so a key that keeps failing still converges rather than being
/// parked forever.
///
/// # Errors
///
/// [`ConstellationError::Io`] for a job this batch never prepared — a caller
/// bug, refused before the first write;
/// [`ConstellationError::IndexBusy`] on a shutdown or expired deadline;
/// [`ConstellationError::ProfileIncompatible`] when the index no longer serves
/// this batch's config digest or its embedding space. Both fences are
/// checked under the owner before the first write, exactly as in
/// [`complete_embedding_batch`]: a failure record is a write like any other,
/// and without them a late one from a replaced space would move rows to
/// `failed` under an input digest the index no longer serves and republish
/// `CURRENT` onto the retired space.
///
/// Not feature-gated, exactly like admission and completion: no model is
/// involved here, only the record that one failed.
pub async fn fail_embedding_batch(
    capability: &MaintainCapability,
    prepared: &super::jobs::PreparedEmbeddingBatch,
    failed_jobs: &[super::jobs::EmbeddingJob],
    detail: &str,
    deadline: std::time::Instant,
) -> Result<EmbeddingFailureOutcome, ConstellationError> {
    use super::lance_tables::open_active_tables;
    use super::publisher::IndexPublisher;

    if !is_admitting() {
        return Err(ConstellationError::IndexBusy {
            detail: "this process is shutting down and admits no new maintenance".to_string(),
        });
    }
    if std::time::Instant::now() >= deadline {
        return Err(ConstellationError::IndexBusy {
            detail: "the failure record's deadline had already expired; nothing was written"
                .to_string(),
        });
    }

    // Refused before the owner is taken: a job this batch never issued is a
    // caller bug, and half-applying it is worse than refusing it.
    for job in failed_jobs {
        if !prepared.accepts(&job.canonical_key, job) {
            return Err(ConstellationError::Io {
                detail: format!(
                    "refusing the failure record: `{}` is not a job this admission issued",
                    job.canonical_key
                ),
            });
        }
    }
    if failed_jobs.is_empty() {
        return Ok(EmbeddingFailureOutcome {
            profile_id: prepared.profile.profile_id(),
            failed: Vec::new(),
            skipped: Vec::new(),
            retry_after: None,
        });
    }

    let index_root = capability.index_root().to_path_buf();
    let workspace = capability.workspace().to_path_buf();
    let profile_id = prepared.profile.profile_id();

    // Owner first, then CURRENT — the same order as admission. A failure
    // record written against a generation another process has already
    // replaced would mark rows the new generation does not own.
    let publisher = IndexPublisher::acquire(&index_root, inner_store_for(&index_root))?;
    let descriptor = super::storage::read_current(&index_root)?.ok_or_else(|| {
        ConstellationError::IndexGenerationUnavailable {
            detail: "the index published no generation; there is nothing to record against"
                .to_string(),
        }
    })?;
    let published = Arc::new(super::storage::read_manifest(&index_root, &descriptor)?);

    // The same two fences completion applies, for the same reason. A failure
    // record is a write like any other: it moves rows to `failed`, advances
    // the durable attempt, and republishes a manifest — and without the config
    // check it would reset a backoff against an input digest the index no
    // longer serves, and without the space fence it would land a profile-A
    // record on rows an operator has already migrated to B and then republish
    // `CURRENT` onto A, reverting the migration the same way a late completion
    // does.
    if descriptor.config_digest != prepared.config_digest {
        return Err(ConstellationError::ProfileIncompatible {
            detail: format!(
                "this failure record was admitted against config digest `{}` but the index now \
                 serves `{}`",
                prepared.config_digest, descriptor.config_digest
            ),
        });
    }
    fence_served_profile(&descriptor, &profile_id, "this failure record")?;

    let baseline = published_baseline(&descriptor, &published)?;
    let marker = marker_for(&published);
    let tables =
        open_active_tables(&index_root, &marker, Arc::clone(publisher.tracking_store())).await?;
    restore_diverged_heads(&tables, &baseline).await?;

    let now = chrono::Utc::now();
    let mut remaining = published.pending_jobs.clone();
    let mut failed: Vec<String> = Vec::with_capacity(failed_jobs.len());
    let mut skipped: Vec<String> = Vec::new();
    let mut retry_after: Option<String> = None;
    let mut artifacts_failed = 0usize;
    let mut sections_failed = 0usize;

    for job in failed_jobs {
        let Some(current) = remaining.get(&job.canonical_key) else {
            // The key was retired between admission and this record. There is
            // nothing outstanding for it, so there is nothing to fail.
            skipped.push(job.canonical_key.clone());
            continue;
        };
        if current != job {
            // A newer admission owns this row now; its attempt history and its
            // pending job are that admission's, not this one's.
            skipped.push(job.canonical_key.clone());
            continue;
        }
        let until = super::jobs::retry_after_at(job.attempt, now);
        let binding = job.binding();

        let guard = crate::workspace::acquire_workspace_lock(&workspace)
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("cannot take the workspace lock for a failure record: {e}"),
            })?;
        let outcome = match job.row_role() {
            super::jobs::EmbeddingRowRole::Artifact => {
                tables
                    .fail_embedding_into_artifacts(&binding, &until, detail)
                    .await?
            }
            super::jobs::EmbeddingRowRole::Section => {
                tables
                    .fail_embedding_into_sections(&binding, &until)
                    .await?
            }
        };
        drop(guard);

        if outcome.rows_updated == 0 {
            skipped.push(job.canonical_key.clone());
            continue;
        }
        match job.row_role() {
            super::jobs::EmbeddingRowRole::Artifact => artifacts_failed += 1,
            super::jobs::EmbeddingRowRole::Section => sections_failed += 1,
        }
        remaining.insert(job.canonical_key.clone(), job.failed_at(until.clone())?);
        retry_after = Some(until);
        failed.push(job.canonical_key.clone());
    }

    if remaining != published.pending_jobs {
        let next = next_manifest_with_pending(
            &published,
            &admission_generation_id(
                &super::publisher::now(),
                &profile_id,
                &published.pending_jobs,
            ),
            remaining,
            tables.head_versions().await?,
        );
        let guard = crate::workspace::acquire_workspace_lock(&workspace)
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("cannot take the workspace lock to publish failures: {e}"),
            })?;
        // Carries the fenced served space forward, or makes the first claim —
        // see the same call in `complete_embedding_batch`.
        publisher.publish(&next, Some(&profile_id))?;
        drop(guard);
    }

    // The state writes above moved the physical heads, and the manifest is
    // immutable: without recording them on CURRENT the next reconcile would
    // read the movement as divergence and rewind over the recorded failure,
    // turning a visible `failed` back into an unexplained `pending`.
    if artifacts_failed + sections_failed > 0 {
        let descriptor = super::storage::read_current(&index_root)?.ok_or_else(|| {
            ConstellationError::IndexGenerationUnavailable {
                detail: "the manifest publish left no pointer to record the heads on".to_string(),
            }
        })?;
        let mut next = descriptor.clone();
        if artifacts_failed > 0 {
            next.embedding_artifacts_head = Some(tables.artifacts_version().await?);
        }
        if sections_failed > 0 {
            next.embedding_sections_head = Some(tables.sections_version().await?);
        }
        // Carried forward for the same reason as in completion: the fence
        // above established that the index already serves this space or names
        // none, so a batch can only establish a first claim, never move the
        // pointer onto the space it happens to carry.
        if next.profile_id.is_none() {
            next.profile_id = Some(profile_id.clone());
        }
        if next != descriptor {
            let guard = crate::workspace::acquire_workspace_lock(&workspace)
                .await
                .map_err(|e| ConstellationError::Io {
                    detail: format!("cannot take the workspace lock to record heads: {e}"),
                })?;
            publisher.republish_current(&next)?;
            drop(guard);
        }
    }

    drop(tables);
    drop(publisher);

    Ok(EmbeddingFailureOutcome {
        profile_id,
        failed,
        skipped,
        retry_after,
    })
}

/// Refuse a batch whose embedding space this index no longer serves.
///
/// The served space is a **fence**, not an output. [`adopt_embedding_profile`]
/// changes only the descriptor's `profile_id`: it writes no row, mints no job
/// and leaves `pending_jobs` alone, so between an adoption and the next
/// admission there is a window in which the manifest still carries profile-A
/// jobs and the rows still store profile A, while `CURRENT` already names B.
/// In that window every other check a late A batch faces still passes — its
/// config digest is unchanged, its jobs are still the pending ones, its rows
/// still match its binding — and the writes would land. Without this fence the
/// pointer then moves back to A, and the operator's migration is silently
/// reverted: `CURRENT` names the space that was just replaced, and a crash in
/// the window loses the migration permanently.
///
/// So a batch is refused outright when `CURRENT` names a space other than the
/// batch's own. `None` is not a mismatch: a generation that never claimed a
/// space admits anything, and a completion is what establishes the claim.
/// Refusing — rather than silently reporting every result stale — is
/// deliberate. This is not one superseded key inside an otherwise live batch,
/// which the pending-job gate handles as `stale` and lands the rest; it is the
/// whole batch computed against weights the operator has replaced, and a
/// caller that kept running would go on to drain again in that same space.
///
/// Called **before** any write and under the owner, so the refusal and the
/// acceptance read the same pointer no other publisher can move between them.
fn fence_served_profile(
    descriptor: &super::manifest::CurrentDescriptor,
    profile_id: &str,
    phase: &str,
) -> Result<(), ConstellationError> {
    match descriptor.profile_id.as_deref() {
        None => Ok(()),
        Some(served) if served == profile_id => Ok(()),
        Some(served) => Err(ConstellationError::ProfileIncompatible {
            detail: format!(
                "{phase} was prepared under embedding space `{profile_id}`, but this index now \
                 serves `{served}`. The prepared batch's inputs were captured under the replaced \
                 profile, so publishing them would move the index back to weights the operator \
                 has migrated away from; re-run the queue under `{served}` instead."
            ),
        }),
    }
}

/// The results of one completion that this batch actually admitted.
///
/// Refuses the whole batch on an unknown key, a duplicate key, or a result
/// whose job is not the exact job admission issued. Returns only the results
/// that passed, so the caller writes nothing at all for a malformed call.
fn accepted_results(
    prepared: &super::jobs::PreparedEmbeddingBatch,
    results: &[super::jobs::ComputedEmbedding],
) -> Result<Vec<super::jobs::ComputedEmbedding>, ConstellationError> {
    let mut seen = std::collections::BTreeSet::new();
    for result in results {
        let key = result.job.canonical_key.as_str();
        if !prepared.accepts(key, &result.job) {
            return Err(ConstellationError::Io {
                detail: format!(
                    "refusing the completion: `{key}` is not a job this admission issued. A \
                     result may only carry the exact job it was prepared with."
                ),
            });
        }
        if !seen.insert(key.to_string()) {
            return Err(ConstellationError::Io {
                detail: format!(
                    "refusing the completion: two results name `{key}`, and choosing one would \
                     discard the other's vector"
                ),
            });
        }
    }
    Ok(results.to_vec())
}

/// Drain the embedding queue for one workspace, under one verified profile.
///
/// This is the 0B "queue in serve/watch" runner, and it is now **an
/// orchestrator, not a fused pass**: [`prepare_embedding_batch`] →
/// the inference worker → [`complete_embedding_batch`], repeated until the
/// queue is empty or the deadline expires.
///
/// The split is what keeps the model off the critical section of every lock.
/// Before it, this one function held the OS-wide owner across the inference
/// call; now the owner is held during admission and during completion, and
/// for no time in between. The worker's thread holds no guard of any kind.
///
/// The model is not in core — the worker is injected by the caller, so the
/// core stays model-agnostic and the whole fence/eligibility machinery is
/// testable without weights.
///
/// # `control`: who may stop this drain waiting
///
/// `deadline` bounds how long this function waits; `control` bounds **who
/// else** may end the wait and why. It is checked at the top of every batch
/// and before every poll of the result wait, so a stop or a due control scan
/// ends a drain that a hung native call would otherwise hold for the full soft
/// deadline.
///
/// A give-up is reported as [`ConstellationError::IndexBusy`] and is never a
/// success: the batch is abandoned (so a vector arriving afterwards is
/// discarded rather than attached to a later admission) and the jobs stay
/// durably pending for the next pass. Nothing is recorded `failed`, because
/// nothing failed — a slow model must not spend a retry attempt because the
/// scan came due.
///
/// A storage commit in flight is **not** interrupted: the give-up is only
/// observed between polls and between batches, so a
/// [`complete_embedding_batch`] that has started runs to its end.
#[cfg(feature = "semantic-search")]
pub async fn drain_embedding_queue(
    capability: &MaintainCapability,
    profile: &super::jobs::EmbeddingProfile,
    worker: &mut super::runtime::InferenceWorker,
    deadline: std::time::Instant,
    control: &super::runtime::DrainControl,
) -> Result<EmbeddingQueueOutcome, ConstellationError> {
    use super::runtime::{InferenceJob, Poll};

    if !is_admitting() {
        return Err(ConstellationError::IndexBusy {
            detail: "this process is shutting down and admits no new maintenance".to_string(),
        });
    }
    let profile_id = profile.profile_id();
    // Coverage as the index held it before this drain, read under its own
    // short-lived owner and released again. Holding it across the loop would
    // mean holding an owner across inference, which is the defect this split
    // exists to remove.
    let before = tables_eligibility_report(capability, &profile_id).await?;

    let mut embedded = 0usize;
    let mut stale_completions = 0usize;

    // Bounded by the queue, not by a counter: every iteration either admits at
    // least one input or returns. An admission that hands back nothing has
    // nothing left to hand back, and looping on it would be a livelock.
    loop {
        if std::time::Instant::now() >= deadline {
            return Err(ConstellationError::IndexBusy {
                detail: format!(
                    "the queue deadline expired after {embedded} publishes; the remaining \
                     pending jobs are picked up by the next pass"
                ),
            });
        }
        // The host may need the loop back — to stop, or to run the control
        // scan inside its period — while a native call is in flight. Checked
        // here as well as inside the collection wait, because a give-up
        // between batches costs nothing to honour and this is where a second
        // batch would otherwise be submitted.
        if let Some(reason) = control.give_up() {
            return Err(gave_up_error(reason, embedded));
        }
        let prepared = prepare_embedding_batch(capability, profile, deadline).await?;
        if prepared.inputs.is_empty() {
            break;
        }

        // Submit and collect. No owner, no workspace lock, and no table
        // handle is live here — that is the whole point of the split.
        //
        // One batch token per admission, so every submission below is marked
        // as this pass's work. A drain that times out mid-collection and is
        // run again against the same worker would otherwise hand the *new*
        // batch's vector to the abandoned one, and the late answer would be
        // attributed to work it was never computed for.
        let _batch = worker.begin_batch();

        // `submitted` is what was **actually queued**, not what admission
        // prepared. Waiting on inputs the worker declined would spin to the
        // deadline waiting for an answer that can never arrive, and marking a
        // never-submitted input failed would spend an attempt on an inference
        // that was never run.
        let mut submitted: Vec<super::jobs::EmbeddingJob> = Vec::new();
        for input in &prepared.inputs {
            match worker.submit(InferenceJob {
                job: input.job.clone(),
                input_text: input.input_text.to_string(),
            }) {
                super::runtime::SubmitOutcome::Queued => submitted.push(input.job.clone()),
                // Not a loss: the job stays durably pending in the manifest
                // with its attempt history, and the next admission hands it
                // over again. Drain what is already in flight and let the
                // loop's next iteration pick the rest up.
                super::runtime::SubmitOutcome::QueueFull => break,
                // The worker thread is gone; no submission after this one can
                // succeed either. Exactly the `Poll::Disconnected` case
                // below — every job still queued is outstanding, it is
                // recorded failed so the next pass backs off, and the caller
                // is told the model is why it stopped.
                super::runtime::SubmitOutcome::WorkerStopped => {
                    record_failures(
                        capability,
                        &prepared,
                        &submitted,
                        "the inference worker had already stopped when this batch was submitted",
                    )
                    .await;
                    return Err(ConstellationError::ModelUnavailable {
                        detail: format!(
                            "the inference worker stopped with {} of this batch's {} inputs \
                             queued; they are recorded failed and retried after their delay",
                            submitted.len(),
                            prepared.inputs.len()
                        ),
                    });
                }
            }
        }
        let expected = submitted.len();
        // No job was accepted. Leave it durably pending for the next pass,
        // rather than re-capturing the whole corpus every 20 ms while busy.
        if expected == 0 {
            break;
        }
        let mut results: Vec<super::jobs::ComputedEmbedding> = Vec::with_capacity(expected);
        while results.len() < expected {
            if std::time::Instant::now() >= deadline {
                // The unanswered jobs are recorded as failures before the error
                // leaves. A soft deadline that leaves every row `pending` at
                // `attempt: 0` is indistinguishable from a drain that never
                // ran, and the next pass re-spends the same inferences.
                //
                // Scoped to what was actually **submitted**: an input the
                // worker declined at submit time was never inferred, and
                // marking it failed would burn an attempt on work no model
                // ever saw. It stays pending, which is the truth.
                let unanswered: Vec<super::jobs::EmbeddingJob> = submitted
                    .iter()
                    .filter(|job| {
                        !results
                            .iter()
                            .any(|result| result.job.canonical_key == job.canonical_key)
                    })
                    .cloned()
                    .collect();
                record_failures(
                    capability,
                    &prepared,
                    &unanswered,
                    "the queue deadline expired before this key was answered",
                )
                .await;
                return Err(ConstellationError::IndexBusy {
                    detail: format!(
                        "the queue deadline expired with {} of {expected} inferences answered; \
                         the unanswered jobs are recorded failed and retried after their delay",
                        results.len()
                    ),
                });
            }
            // Checked before every poll of the wait below. This is the check
            // that makes a stop and the control scan responsive while a native
            // call hangs: without it the loop could not leave until the 120 s
            // soft deadline, and the host would be two minutes late noticing
            // either. Nothing is recorded failed — the jobs stay durably
            // pending, and the batch is abandoned so a late vector is
            // discarded rather than attached to a later admission.
            if let Some(reason) = control.give_up() {
                worker.begin_batch();
                return Err(gave_up_error(reason, embedded));
            }
            match worker.try_result() {
                Poll::Ready(result) => {
                    // The job the worker echoes back must be the exact job
                    // **this** admission issued. A worker answering for an
                    // earlier admission cannot have its vector attached to a
                    // different job, and one answering for a key nobody
                    // admitted is refused outright rather than matched by key.
                    if !prepared.accepts(result.canonical_key(), &result.job) {
                        return Err(ConstellationError::Io {
                            detail: format!(
                                "the worker returned a vector for `{}` carrying a job this \
                                 admission never issued",
                                result.canonical_key()
                            ),
                        });
                    }
                    results.push(super::jobs::ComputedEmbedding {
                        job: result.job,
                        vector: result.vector,
                    });
                }
                Poll::Empty => {
                    // A short sleep, not a busy spin: the worker thread is
                    // doing native work and this executor has nothing else to
                    // do with the turn.
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
                Poll::Disconnected => {
                    // The worker is gone. Report what it said, if anything —
                    // a silent stop is the failure mode this channel exists
                    // to make visible.
                    let detail = match worker.try_failure() {
                        Poll::Ready(detail) => detail,
                        _ => "the inference worker stopped without reporting a failure".to_string(),
                    };
                    // Every job this drain actually queued is outstanding:
                    // nothing can answer them now. They are recorded failed so
                    // the next pass backs off rather than re-queueing the same
                    // work into a worker that is already gone. Inputs the
                    // worker declined are not among them — they were never
                    // submitted and stay pending.
                    record_failures(capability, &prepared, &submitted, &detail).await;
                    return Err(ConstellationError::ModelUnavailable { detail });
                }
            }
        }

        let outcome = complete_embedding_batch(capability, &prepared, &results, deadline).await?;
        embedded += outcome.published.len();
        stale_completions += outcome.stale.len();
    }

    let after = tables_eligibility_report(capability, &profile_id).await?;
    let generation_id = current_generation_id(capability)?;
    Ok(EmbeddingQueueOutcome {
        generation_id,
        profile_id,
        before,
        embedded,
        stale_completions,
        failed: 0,
        after,
    })
}

/// The error a drain returns when its control asked it to stop waiting.
///
/// `IndexBusy` is the same code the soft deadline returns, deliberately: both
/// mean "this pass did not finish, the outstanding jobs are still pending, and
/// the next pass picks them up". Distinguishing them in the wire code would
/// invite a caller to treat a scan-due yield as a fault, which it is not — a
/// model that is merely slow must not look broken.
///
/// The detail names the reason, because the caller has to decide what to do
/// next: a stopping host stops, a scan-due drain returns to its loop.
#[cfg(feature = "semantic-search")]
fn gave_up_error(reason: super::runtime::DrainGiveUp, embedded: usize) -> ConstellationError {
    ConstellationError::IndexBusy {
        detail: format!(
            "the queue drain stopped waiting after {embedded} publishes because {}; the \
             outstanding jobs stay pending and are picked up by the next pass",
            super::runtime::DrainControl::describe(reason)
        ),
    }
}

/// Record a failed attempt without letting the record itself mask the failure.
///
/// A bookkeeping write is not allowed to become the error a caller sees: the
/// caller already has a model failure or an expired deadline to act on, and
/// replacing that with a contention or durability error would hide which one
/// actually stopped the drain. The row stays `pending` in that case, which is
/// the honest pre-existing state, and the next pass re-admits it.
///
/// Returns how many keys were durably recorded `failed`.
#[cfg(feature = "semantic-search")]
async fn record_failures(
    capability: &MaintainCapability,
    prepared: &super::jobs::PreparedEmbeddingBatch,
    jobs: &[super::jobs::EmbeddingJob],
    detail: &str,
) -> usize {
    if jobs.is_empty() {
        return 0;
    }
    // Its own short deadline: the compute deadline is what usually expired.
    let record_deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    match fail_embedding_batch(capability, prepared, jobs, detail, record_deadline).await {
        Ok(outcome) => outcome.failed.len(),
        Err(_) => 0,
    }
}

/// The generation `CURRENT` names, or an empty string when nothing is
/// published.
///
/// Read without any guard: replacing `CURRENT.json` is an atomic rename, so a
/// read sees one generation or the other and never a mixture. The answer is
/// used for reporting only, never for a decision that a write depends on.
#[cfg(feature = "semantic-search")]
fn current_generation_id(capability: &MaintainCapability) -> Result<String, ConstellationError> {
    Ok(super::storage::read_current(capability.index_root())?
        .map(|descriptor| descriptor.generation_id)
        .unwrap_or_default())
}

/// Read the eligibility report the index holds right now, under its own
/// short-lived owner.
///
/// A separate pass rather than a value carried out of completion: the
/// completion's own report was read before the pointer moved, and this is
/// what a caller asking "is the queue empty now?" needs answered.
#[cfg(feature = "semantic-search")]
async fn tables_eligibility_report(
    capability: &MaintainCapability,
    profile_id: &str,
) -> Result<super::jobs::EligibilityReport, ConstellationError> {
    use super::lance_tables::open_active_tables;
    use super::publisher::{IndexPublisher, capture_under_workspace_lock};

    let index_root = capability.index_root().to_path_buf();
    // Owner before CURRENT, as everywhere else: this report describes one
    // generation, and a pointer read before the owner names a generation that
    // may already have been replaced.
    let publisher = IndexPublisher::acquire(&index_root, inner_store_for(&index_root))?;
    let Some(descriptor) = super::storage::read_current(&index_root)? else {
        return Ok(super::jobs::EligibilityReport::default());
    };
    let published = Arc::new(super::storage::read_manifest(&index_root, &descriptor)?);
    let tables = open_active_tables(
        &index_root,
        &marker_for(&published),
        Arc::clone(publisher.tracking_store()),
    )
    .await?;
    // Same baseline as every other production capture, because this report is a
    // verdict about the same rows: eligibility compares each stored binding
    // against the capture's, and a capture that re-derived identity from
    // nothing would call every live row stale.
    let captured = capture_under_workspace_lock(
        capability.workspace(),
        super::capture::CaptureLimits::default(),
        Some(Arc::clone(&published)),
    )
    .await?;
    let mut report = tables.artifacts_eligibility(&captured, profile_id).await?;
    report.merge_from(&tables.sections_eligibility(&captured, profile_id).await?);
    drop(tables);
    drop(publisher);
    Ok(report)
}

/// Kind/status restriction a pinned search applies **before** the limit.
///
/// The pinned artifacts row carries `kind` and `status`, so both are pushed
/// into the query itself rather than applied to the result afterwards. The
/// order matters and is the whole point: `forgeplan search x --kind prd
/// --limit 10` over 50 PRDs ranked below 10 ADRs must return the PRDs, and a
/// post-limit filter returns "no results" — a filtered-out row that was never
/// evaluated is indistinguishable from an empty corpus.
///
/// `None` on both fields means no restriction, which is the common case and
/// costs nothing: no predicate term is added.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PinnedFilter {
    pub kind: Option<String>,
    pub status: Option<String>,
}

impl PinnedFilter {
    /// Whether this filter restricts anything.
    pub fn is_empty(&self) -> bool {
        self.kind.is_none() && self.status.is_none()
    }

    /// The SQL predicate for this filter over a row that has both columns, or
    /// `None` when it restricts nothing.
    ///
    /// Values are escaped with the same
    /// [`super::publisher::escape_predicate_literal`] every other predicate
    /// here uses, so a `--kind` containing a quote cannot become structure.
    pub fn row_predicate(&self) -> Option<String> {
        let mut terms = Vec::new();
        if let Some(kind) = &self.kind {
            terms.push(format!(
                "kind = {}",
                super::publisher::escape_predicate_literal(kind)
            ));
        }
        if let Some(status) = &self.status {
            terms.push(format!(
                "status = {}",
                super::publisher::escape_predicate_literal(status)
            ));
        }
        (!terms.is_empty()).then(|| terms.join(" AND "))
    }

    /// The SQL predicate restricting a **section** row by its parent's
    /// kind/status, given the set of parent keys the filter admits — or `None`
    /// when no parent passes, which means no section row can either.
    ///
    /// A section row has no `kind`/`status` column of its own — it stores its
    /// parent's `source_digest`, not the parent's kind — so the filter is
    /// expressed over `parent_key`.
    ///
    /// An empty admitted set is `None` rather than `parent_key IN ()`: the
    /// empty `IN` list is a form whose acceptance is a SQL dialect question,
    /// and "no parent qualifies, so skip the section query" says the same
    /// thing without depending on it.
    fn section_predicate(
        &self,
        admitted_parents: &std::collections::BTreeSet<String>,
    ) -> Option<String> {
        if admitted_parents.is_empty() {
            return None;
        }
        let keys = admitted_parents
            .iter()
            .map(|key| super::publisher::escape_predicate_literal(key))
            .collect::<Vec<_>>()
            .join(", ");
        Some(format!("parent_key IN ({keys})"))
    }
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
    /// `true` when hits came through the bound trigram index; `false` when
    /// the pinned table was scanned row by row. Both answer at the same
    /// generation — the flag only reports which path produced them.
    pub via_trigram: bool,
    pub hits: Vec<LexicalHit>,
}

/// One document in the published artifacts table (not a live filesystem scan).
#[derive(Debug, serde::Serialize)]
pub struct IndexedDocument {
    pub canonical_key: String,
    pub display_id: Option<String>,
    pub relative_path: String,
    pub source_digest: String,
    pub semantic_state: Option<String>,
}

/// One bounded inventory page and the generation it was read from.
#[derive(Debug)]
pub struct IndexedDocumentPage {
    pub generation_id: String,
    pub pinned_artifacts_version: u64,
    pub documents: Vec<IndexedDocument>,
    pub has_more: bool,
}

/// List documents at CURRENT's pinned table version without opening a writer.
pub async fn list_pinned_documents(
    index_root: &Path,
    offset: usize,
    limit: usize,
) -> Result<IndexedDocumentPage, ConstellationError> {
    use arrow_array::{Array, StringArray};

    if !(1..=100).contains(&limit) {
        return Err(ConstellationError::Io {
            detail: "inventory limit must be between 1 and 100".to_string(),
        });
    }
    let descriptor = super::storage::read_current(index_root)?.ok_or_else(|| {
        ConstellationError::IndexGenerationUnavailable {
            detail: "no published generation to list".to_string(),
        }
    })?;
    // Stage 8 / C06 — pin this generation with a shared OS lock on the
    // manifest file itself, held until this function returns. No lease file,
    // no TTL, and nothing under the index root is created or modified.
    let pinned = super::storage::pin_generation_read(index_root, &descriptor)?;
    let manifest = pinned.manifest();
    let artifacts_version = descriptor.effective_artifacts_version(manifest)?;
    let marker = marker_for(manifest);
    let table = super::lance_tables::open_table_at_version(
        index_root,
        &marker,
        "artifacts",
        artifacts_version,
    )
    .await?;
    let batches = table.artifact_page(offset, limit + 1).await?;
    let mut documents = Vec::new();
    for batch in &batches {
        let value = |col: &str, row: usize| {
            batch
                .column_by_name(col)
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .filter(|c| !c.is_null(row))
                .map(|c| c.value(row).to_string())
        };
        for row in 0..batch.num_rows() {
            documents.push(IndexedDocument {
                canonical_key: value("canonical_key", row).unwrap_or_default(),
                display_id: value("id", row),
                relative_path: value("relative_path", row).unwrap_or_default(),
                source_digest: value("source_digest", row).unwrap_or_default(),
                semantic_state: value("semantic_state", row),
            });
        }
    }
    let has_more = documents.len() > limit;
    documents.truncate(limit);
    Ok(IndexedDocumentPage {
        generation_id: manifest.generation_id.clone(),
        pinned_artifacts_version: artifacts_version,
        documents,
        has_more,
    })
}

/// The pinned relation graph of one workspace: every node (id, title,
/// kind) and every exact edge `(from → target, relation)` — the read
/// shape stage-7 impact analysis walks. Read-only, pinned to the
/// published manifest's table versions; never latest, never a writer.
pub struct PinnedGraph {
    pub generation_id: String,
    /// (canonical_key, display_id, title, kind)
    pub nodes: Vec<(String, Option<String>, String, String)>,
    /// (from canonical_key, target, relation) — `target` is whatever the
    /// source wrote, resolved by the caller against this source's keys.
    pub edges: Vec<(String, String, String)>,
}

/// Read the pinned artifacts+relations tables as a graph.
pub async fn list_pinned_graph(index_root: &Path) -> Result<PinnedGraph, ConstellationError> {
    use arrow_array::{Array, StringArray};

    let descriptor = super::storage::read_current(index_root)?.ok_or_else(|| {
        ConstellationError::IndexGenerationUnavailable {
            detail: "no published generation to read the graph from".to_string(),
        }
    })?;
    // Stage 8 / C06 — the shared OS lock on the manifest, held for the
    // whole read; see [`super::storage::pin_generation_read`].
    let pinned = super::storage::pin_generation_read(index_root, &descriptor)?;
    let manifest = pinned.manifest();
    let artifacts_version = descriptor.effective_artifacts_version(manifest)?;
    let marker = marker_for(manifest);
    let artifacts = super::lance_tables::open_table_at_version(
        index_root,
        &marker,
        "artifacts",
        artifacts_version,
    )
    .await?;
    let relations = super::lance_tables::open_table_at_version(
        index_root,
        &marker,
        "relations",
        manifest.table_versions.relations,
    )
    .await?;
    let mut nodes = Vec::new();
    for batch in &artifacts
        .select_columns(&["canonical_key", "id", "title", "kind"])
        .await?
    {
        let value = |col: &str, row: usize| {
            batch
                .column_by_name(col)
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .filter(|c| !c.is_null(row))
                .map(|c| c.value(row).to_string())
        };
        for row in 0..batch.num_rows() {
            nodes.push((
                value("canonical_key", row).unwrap_or_default(),
                value("id", row),
                value("title", row).unwrap_or_default(),
                value("kind", row).unwrap_or_default(),
            ));
        }
    }
    let mut edges = Vec::new();
    for batch in &relations
        .select_columns(&["canonical_key", "target", "relation"])
        .await?
    {
        let value = |col: &str, row: usize| {
            batch
                .column_by_name(col)
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .filter(|c| !c.is_null(row))
                .map(|c| c.value(row).to_string())
        };
        for row in 0..batch.num_rows() {
            edges.push((
                value("canonical_key", row).unwrap_or_default(),
                value("target", row).unwrap_or_default(),
                value("relation", row).unwrap_or_default(),
            ));
        }
    }
    Ok(PinnedGraph {
        generation_id: manifest.generation_id.clone(),
        nodes,
        edges,
    })
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
///
/// `filter` is applied inside the query, never to the returned page: the
/// shortlist predicate and the streaming scan both carry it, so `limit` counts
/// rows that already passed and a corpus whose top `limit` rows are all
/// filtered out still returns the next ones that are not.
pub async fn search_pinned_lexical(
    index_root: &Path,
    query: &str,
    limit: usize,
    filter: &PinnedFilter,
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
    // Stage 8 / C06 — the shared OS lock on the manifest, held for the
    // whole read; see [`super::storage::pin_generation_read`].
    let pinned = super::storage::pin_generation_read(index_root, &descriptor)?;
    let manifest = pinned.manifest();
    let artifacts_version = descriptor.effective_artifacts_version(manifest)?;
    if !manifest.table_versions.is_publishable() {
        return Err(ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "generation `{}` names non-publishable table versions {:?}",
                manifest.generation_id, manifest.table_versions
            ),
        });
    }
    let marker = marker_for(manifest);
    let table = super::lance_tables::open_table_at_version(
        index_root,
        &marker,
        "artifacts",
        artifacts_version,
    )
    .await?;
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

    // Source-byte trigrams cannot see keys derived from filenames. Supplement
    // them with a streamed metadata scan of the same pinned table; on any
    // incomplete or oversized shortlist, the full pinned scan remains safe.
    const SHORTLIST_CAP: usize = 128;
    if let Some(forgeplan_dir) = index_root.parent()
        && super::lexical_index::binding_for(forgeplan_dir, manifest)
            == super::lexical_index::LexicalBinding::Bound
        && query.is_ascii()
        && let Ok(Some(source_paths)) =
            super::lexical_index::query_paths(forgeplan_dir, query, SHORTLIST_CAP)
        && source_paths.len() < SHORTLIST_CAP
        && let Ok(Some(metadata_paths)) = table
            .lexical_metadata_paths(&query.to_lowercase(), SHORTLIST_CAP)
            .await
    {
        let paths: std::collections::BTreeSet<_> =
            source_paths.into_iter().chain(metadata_paths).collect();
        if paths.len() < SHORTLIST_CAP {
            if paths.is_empty()
                && super::lexical_index::binding_for(forgeplan_dir, manifest)
                    == super::lexical_index::LexicalBinding::Bound
            {
                return Ok(LexicalSearchOutcome {
                    generation_id: manifest.generation_id.clone(),
                    pinned_artifacts_version: artifacts_version,
                    rows_scanned: 0,
                    truncated: false,
                    hits: Vec::new(),
                    via_trigram: true,
                });
            }
            if !paths.is_empty() {
                // The kind/status terms are part of the **same** predicate as
                // the path shortlist, not a check over its results: a
                // filtered-out row must never consume a slot of `limit`.
                let predicate = match filter.row_predicate() {
                    Some(restriction) => format!(
                        "relative_path IN ({}) AND ({restriction})",
                        paths
                            .iter()
                            .map(|p| super::publisher::escape_predicate_literal(p))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    None => format!(
                        "relative_path IN ({})",
                        paths
                            .iter()
                            .map(|p| super::publisher::escape_predicate_literal(p))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                };
                match table.query_batches_where(&predicate).await {
                    Ok(batches) => {
                        let needle = query.to_lowercase();
                        let mut hits: Vec<LexicalHit> = Vec::new();
                        let mut rows_scanned = 0;
                        'candidates: for batch in &batches {
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
                                    break 'candidates;
                                }
                            }
                        }
                        if super::lexical_index::binding_for(forgeplan_dir, manifest)
                            == super::lexical_index::LexicalBinding::Bound
                        {
                            return Ok(LexicalSearchOutcome {
                                generation_id: manifest.generation_id.clone(),
                                pinned_artifacts_version: artifacts_version,
                                rows_scanned,
                                truncated: hits.len() >= limit,
                                hits,
                                via_trigram: true,
                            });
                        }
                    }
                    Err(e) => tracing::warn!(
                        "trigram shortlist join failed; falling back to the pinned scan: {e}"
                    ),
                }
            }
        }
    }

    // Stream the pinned fallback one batch at a time; a failed side-index
    // must not force every body and embedding into memory for one query.
    //
    // This path has no SQL predicate to push the filter into — it is a full
    // scalar scan, not a point read — so the check happens here, before the
    // `hits.len() >= limit` break rather than after the page is built. Same
    // ordering guarantee as the shortlist path, different mechanism.
    let needle = query.to_lowercase();
    let mut hits: Vec<LexicalHit> = Vec::new();
    let mut rows_scanned = 0usize;
    table
        .scan_lexical_batches(|batch| {
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
                let row_kind = read_str(batch, "kind", row);
                let row_status = read_str(batch, "status", row);
                if filter
                    .kind
                    .as_ref()
                    .is_some_and(|kind| row_kind.as_deref() != Some(kind.as_str()))
                    || filter
                        .status
                        .as_ref()
                        .is_some_and(|status| row_status.as_deref() != Some(status.as_str()))
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
                    return false;
                }
            }
            true
        })
        .await?;

    Ok(LexicalSearchOutcome {
        generation_id: manifest.generation_id.clone(),
        pinned_artifacts_version: artifacts_version,
        rows_scanned,
        truncated: hits.len() >= limit,
        hits,
        via_trigram: false,
    })
}

/// One semantic hit at the pinned generation — artifact row or section row.
#[derive(Debug, serde::Serialize)]
pub struct SemanticHit {
    /// The matched row's own key: the artifact `canonical_key`, or the
    /// section key `"{parent}#s{ordinal}"` when a section row won.
    pub canonical_key: String,
    /// For a section hit, the artifact it belongs to. `None` on an
    /// artifact-level hit — the key already is the artifact's.
    pub parent_key: Option<String>,
    /// Section ordinal within the parent, `None` on an artifact-level hit.
    pub section_ordinal: Option<u64>,
    /// The section's heading text, `None` for the whole-body fallback row.
    pub section_heading: Option<String>,
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
    /// The pinned sections version the search also queried, or `None` when
    /// this generation predates `constellation_sections_v1` — the hits then
    /// came from artifact rows alone and `semantic_scope` says so.
    pub pinned_sections_version: Option<u64>,
    /// Which embedding space the hits span: `prefix` or `prefix+sections`.
    pub semantic_scope: &'static str,
    /// How many stored vectors were eligible to rank — ready under *this*
    /// profile, across both tables. Counted **before** the limit is applied
    /// (a limit-free count over the same predicate), so it is coverage and not
    /// "how many ranked". Distinct from `hits.len()`: a corpus can hold zero
    /// ready rows under the profile and still contain plenty under another,
    /// and `limit` can cut the ranking far short of `eligible_rows`.
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
///
/// `filter` restricts kind/status **inside** both ANN queries, before
/// `limit`: the artifacts table has the columns, and a section row inherits
/// its parent's restriction through an explicit `parent_key IN (…)` term
/// rather than by being filtered out afterwards. A caller that filtered the
/// returned page would report "no results" for a corpus that simply ranks its
/// matching rows below the cut.
///
/// # This is the CURRENT-lookup operation, and it is not the captured one
///
/// It resolves `CURRENT` **at call time** and pins whatever is published then.
/// That is a distinct question from "answer the view I already captured", and
/// the two must not be confused:
///
/// * a caller that wants *the newest* answer — a CLI `search`, a stand-alone
///   MCP query that resolved its profile from the same `CURRENT` — uses this;
/// * a caller holding a [`ReadView`] — a captured read served through
///   `WorkspaceRead` — uses [`search_semantic_in_view`], because a captured
///   view's records come from generation **G** and scoring them against the
///   vectors of a generation published afterwards attributes another
///   generation's vectors to G's records.
///
/// There is no shared fallback and no aliasing between them. The
/// captured-generation search is a separate function with its own name
/// precisely so that "search the view I have" is not spelled the same way as
/// "search whatever is current now".
pub async fn search_pinned_semantic(
    index_root: &Path,
    query_vector: &[f32],
    profile_id: &str,
    limit: usize,
    filter: &PinnedFilter,
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
    // Stage 8 / C06 — the shared OS lock on the manifest, held for the
    // whole read; see [`super::storage::pin_generation_read`].
    let pinned = super::storage::pin_generation_read(index_root, &descriptor)?;
    search_semantic_at(
        index_root,
        &descriptor,
        pinned.manifest(),
        query_vector,
        profile_id,
        limit,
        filter,
        None,
    )
    .await
}

/// ANN search over the artifacts table **at the generation a captured view was
/// taken at**.
///
/// # Why this exists rather than "re-read `CURRENT` and hope"
///
/// A [`ReadView`] is one self-consistent snapshot: its records, its edges and
/// its bodies all came from one capture of the workspace. The index is a
/// different object with its own lifecycle — a maintenance pass, or an
/// embedding completion, publishes a **new** generation while a reader holds
/// the old one. Calling [`search_pinned_semantic`] from a captured read would
/// therefore score generation **G**'s records against generation **H**'s
/// vectors: the prefilter admits only rows ready under `profile_id`, so the
/// similarity numbers are real and the *attribution* is not. Every field of
/// such a result would look internally consistent, and the one fact that made
/// it wrong — that the vectors belong to a different generation — appears
/// nowhere in it.
///
/// This function cannot make that mistake: it never reads `CURRENT`. It serves
/// the descriptor the view captured, whose embedding heads are that
/// generation's, and it re-checks that the `index_root` it was handed still
/// resolves **that** manifest before it opens a single table.
///
/// # The shared lock is already held
///
/// A persisted view holds `Arc<PinnedGeneration>` for its whole life, so the
/// shared manifest lock this read needs is live for the duration of the search
/// without re-acquiring it. GC cannot remove the generation underneath a view
/// that is still answering.
///
/// # Errors
///
/// [`ConstellationError::IndexGenerationUnavailable`] when
///
/// * the ephemeral view has no verified vector cache. This function never
///   substitutes the generation `CURRENT` might name after capture.
/// * `index_root` does not resolve the manifest this view captured — a
///   different index, or a generation that has since been replaced. Checked
///   rather than assumed, because a root that answers is not proof that it
///   answers *this* generation.
///
/// and [`ConstellationError::Io`] for `limit == 0`, exactly as
/// [`search_pinned_semantic`] does.
pub async fn search_semantic_in_view(
    index_root: &Path,
    view: &ReadView,
    query_vector: &[f32],
    profile_id: &str,
    limit: usize,
    filter: &PinnedFilter,
) -> Result<SemanticSearchOutcome, ConstellationError> {
    if limit == 0 {
        return Err(ConstellationError::Io {
            detail: "a semantic search with limit 0 cannot return anything".to_string(),
        });
    }
    let (descriptor, manifest, captured) = match view {
        ReadView::Persisted {
            descriptor,
            manifest,
            ..
        } => (descriptor, &**manifest, None),
        ReadView::Ephemeral { captured, .. } => {
            let Some(pin) = view.vector_cache() else {
                return Err(ConstellationError::IndexGenerationUnavailable {
                    detail: format!(
                        "this read view is an ephemeral capture of {} (source set {}); it has no \
                         verified vector cache, and this search will not borrow the generation \
                         CURRENT names now — that generation's vectors would be attributed to \
                         this capture's records",
                        view.checkout_id(),
                        view.source_set_digest(),
                    ),
                });
            };
            if captured.checkout_id != pin.manifest().checkout_id {
                return Err(ConstellationError::IndexGenerationUnavailable {
                    detail: "captured source and pinned vector cache belong to different checkouts"
                        .to_string(),
                });
            }
            (pin.descriptor(), pin.manifest(), Some(captured.as_ref()))
        }
    };
    // The root must still resolve **this** manifest. `CURRENT` is deliberately
    // not read: it says nothing about generation G once H has been published,
    // and trusting it here would reintroduce the substitution one branch up.
    // `read_manifest` verifies the descriptor's checksum, so a replaced or
    // truncated manifest is an error rather than a different generation.
    let on_disk = super::storage::read_manifest(index_root, descriptor)?;
    if &on_disk != manifest {
        return Err(ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "{} holds `{}`, not the generation `{}` this view captured; the index root and \
                 the view disagree and this search will not substitute one for the other",
                index_root.display(),
                on_disk.generation_id,
                descriptor.generation_id
            ),
        });
    }
    search_semantic_at(
        index_root,
        descriptor,
        manifest,
        query_vector,
        profile_id,
        limit,
        filter,
        captured,
    )
    .await
}

fn captured_semantic_predicates(
    captured: &CapturedSourceView,
    manifest: &GenerationManifest,
    filter: &PinnedFilter,
    ready: &str,
) -> (String, String) {
    use super::publisher::escape_predicate_literal as quoted;
    let published: std::collections::HashMap<_, _> = manifest
        .source_entries
        .iter()
        .map(|entry| (entry.canonical_key.as_str(), entry))
        .collect();
    let mut parents = std::collections::HashMap::new();
    let mut artifacts = Vec::new();
    for artifact in &captured.artifacts {
        let Some(previous) = published.get(artifact.canonical_key.as_str()) else {
            continue;
        };
        if previous.relative_path != artifact.relative_path
            || filter
                .kind
                .as_ref()
                .is_some_and(|kind| kind != &artifact.kind)
            || filter
                .status
                .as_ref()
                .is_some_and(|status| status != &artifact.status)
        {
            continue;
        }
        artifacts.push(format!(
            "(canonical_key = {} AND input_digest = {} AND incarnation = {} AND row_revision = {})",
            quoted(&artifact.canonical_key),
            quoted(&artifact.input_digest),
            previous.incarnation,
            previous.row_revision,
        ));
        parents.insert(artifact.canonical_key.as_str(), *previous);
    }
    let sections: Vec<_> = captured.sections.iter().filter_map(|section| {
        let previous = parents.get(section.parent_key.as_str())?;
        Some(format!(
            "(canonical_key = {} AND parent_key = {} AND input_digest = {} AND incarnation = {} AND row_revision = {})",
            quoted(&section.canonical_key), quoted(&section.parent_key), quoted(&section.input_digest),
            previous.incarnation, previous.row_revision,
        ))
    }).collect();
    let combine = |bindings: Vec<String>| {
        if bindings.is_empty() {
            "false".to_string()
        } else {
            format!("({ready}) AND ({})", bindings.join(" OR "))
        }
    };
    (combine(artifacts), combine(sections))
}

/// How many rows under `profile_id` are ready at the versions this view pins.
///
/// Cheap and model-free: it opens the pinned tables and counts the eligible
/// predicate. A caller uses it to skip query inference when no vector could
/// match — loading the model dominates a CLI invocation, and paying that to
/// rank an empty eligible set is pure latency.
pub async fn count_eligible_semantic(
    index_root: &Path,
    view: &ReadView,
    profile_id: &str,
) -> Result<usize, ConstellationError> {
    let (descriptor, manifest, captured) = match view {
        ReadView::Persisted {
            descriptor,
            manifest,
            ..
        } => (descriptor, &**manifest, None),
        ReadView::Ephemeral { captured, .. } => match view.vector_cache() {
            Some(pin) => (pin.descriptor(), pin.manifest(), Some(captured.as_ref())),
            None => return Ok(0),
        },
    };
    let artifacts_version = descriptor.effective_artifacts_version(manifest)?;
    let marker = marker_for(manifest);
    let ready = format!(
        "semantic_state = 'ready' AND profile_id = {}",
        super::publisher::escape_predicate_literal(profile_id)
    );
    // An ephemeral view must count with the same exact bindings the search
    // will use: ready rows exist for the *old* inputs, so a coarse count
    // would send the caller into a model load that can match nothing.
    let (artifact_predicate, section_predicate) = match captured {
        Some(captured) => {
            let (artifacts, sections) =
                captured_semantic_predicates(captured, manifest, &PinnedFilter::default(), &ready);
            (artifacts, Some(sections))
        }
        None => (ready.clone(), None),
    };
    let table = super::lance_tables::open_table_at_version(
        index_root,
        &marker,
        "artifacts",
        artifacts_version,
    )
    .await?;
    let mut count = table.count_rows_where(&artifact_predicate).await?;
    if let Some(version) = descriptor.effective_sections_version(manifest)? {
        let sections =
            super::lance_tables::open_table_at_version(index_root, &marker, "sections", version)
                .await?;
        let predicate = section_predicate.unwrap_or_else(|| ready.clone());
        count += sections.count_rows_where(&predicate).await?;
    }
    Ok(count)
}

/// The ranking itself, shared by both entry points above.
///
/// Both published reads and captured vector reuse open the same pinned versions.
/// Captured reuse adds exact input bindings before ranking, not after `limit`.
#[allow(clippy::too_many_arguments)]
async fn search_semantic_at(
    index_root: &Path,
    descriptor: &CurrentDescriptor,
    manifest: &GenerationManifest,
    query_vector: &[f32],
    profile_id: &str,
    limit: usize,
    filter: &PinnedFilter,
    captured: Option<&CapturedSourceView>,
) -> Result<SemanticSearchOutcome, ConstellationError> {
    let artifacts_version = descriptor.effective_artifacts_version(manifest)?;
    if !manifest.table_versions.is_publishable() {
        return Err(ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "generation `{}` names non-publishable table versions {:?}",
                manifest.generation_id, manifest.table_versions
            ),
        });
    }
    let marker = marker_for(manifest);
    let table = super::lance_tables::open_table_at_version(
        index_root,
        &marker,
        "artifacts",
        artifacts_version,
    )
    .await?;

    // The sections half pins to the descriptor's effective version — the
    // manifest's pin or the drain head — and is absent entirely for a
    // pre-sections generation. `None` never means "open latest".
    let sections_version = descriptor.effective_sections_version(manifest)?;
    let sections_table = match sections_version {
        Some(version) => Some(
            super::lance_tables::open_table_at_version(index_root, &marker, "sections", version)
                .await?,
        ),
        None => None,
    };

    // `escape_predicate_literal` already returns a delimited literal —
    // wrapping it in quotes again would produce `''x''`, which parses as an
    // empty string beside a bare identifier and matches nothing.
    let ready_predicate = format!(
        "semantic_state = 'ready' AND profile_id = {}",
        super::publisher::escape_predicate_literal(profile_id)
    );

    let (artifact_predicate, section_predicate) = if let Some(captured) = captured {
        let (artifacts, sections) =
            captured_semantic_predicates(captured, manifest, filter, &ready_predicate);
        (artifacts, sections_table.as_ref().map(|_| sections))
    } else {
        // The artifacts half carries `kind`/`status`, so the restriction is one
        // more AND on the same predicate.
        let artifact_predicate = match filter.row_predicate() {
            Some(restriction) => format!("{ready_predicate} AND ({restriction})"),
            None => ready_predicate.clone(),
        };

        // The sections half does not: a section row stores its parent's
        // `source_digest` and title, not the parent's kind. So the parent's key
        // set is resolved **first** — one narrow key-only scan under the same
        // restriction — and the section query is bounded by it. That keeps the
        // section half inside `limit` after filtering instead of spending the
        // whole budget on rows the caller's filter excludes.
        let section_predicate = match (&sections_table, filter.is_empty()) {
            (Some(_), true) => Some(ready_predicate.clone()),
            (Some(_), false) => {
                let restriction = filter
                    .row_predicate()
                    .expect("a non-empty filter always yields a row predicate");
                let admitted = table
                    .keys_where(&format!("{ready_predicate} AND ({restriction})"))
                    .await?;
                // `None` means no parent passed the filter, so no section row
                // qualifies either and the section query is skipped entirely.
                filter
                    .section_predicate(&admitted)
                    .map(|restriction| format!("{ready_predicate} AND ({restriction})"))
            }
            (None, _) => None,
        };
        (artifact_predicate, section_predicate)
    };

    let batches = table
        .nearest_embeddings(query_vector, &artifact_predicate, limit)
        .await?;
    let section_batches = match (&sections_table, &section_predicate) {
        (Some(sections), Some(predicate)) => {
            sections
                .nearest_embeddings(query_vector, predicate, limit)
                .await?
        }
        _ => Vec::new(),
    };

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
    let read_u64 = |batch: &arrow_array::RecordBatch, col: &str, row: usize| -> Option<u64> {
        use arrow_array::Array;
        let column = batch.column_by_name(col)?;
        if row >= column.len() || column.is_null(row) {
            return None;
        }
        column
            .as_any()
            .downcast_ref::<arrow_array::UInt64Array>()
            .map(|a| a.value(row))
    };

    // Coverage is the size of the **eligible set**, not of what the limit let
    // through. `nearest_embeddings` pushes `limit` into the query, so summing
    // the returned batches counts "how many ranked", and a caller reading that
    // as coverage concludes the corpus is embedded when most of it never
    // entered the ranking at all. The count is therefore a separate,
    // limit-free scan over the same predicate — per table, because the two
    // halves do not share one when a filter is in force.
    let eligible_rows = table.count_rows_where(&artifact_predicate).await?
        + match (&sections_table, &section_predicate) {
            (Some(sections), Some(predicate)) => sections.count_rows_where(predicate).await?,
            _ => 0,
        };

    let mut hits = Vec::new();
    for batch in &batches {
        for row in 0..batch.num_rows() {
            hits.push(SemanticHit {
                canonical_key: read_str(batch, "canonical_key", row).unwrap_or_default(),
                parent_key: None,
                section_ordinal: None,
                section_heading: None,
                display_id: read_str(batch, "id", row),
                title: read_str(batch, "title", row).unwrap_or_default(),
                similarity: read_dist(batch, row).map_or(0.0, |d| 1.0 - d),
            });
        }
    }
    for batch in &section_batches {
        for row in 0..batch.num_rows() {
            hits.push(SemanticHit {
                canonical_key: read_str(batch, "canonical_key", row).unwrap_or_default(),
                parent_key: read_str(batch, "parent_key", row),
                section_ordinal: read_u64(batch, "ordinal", row),
                section_heading: read_str(batch, "heading", row),
                display_id: read_str(batch, "parent_display_id", row),
                title: read_str(batch, "title", row).unwrap_or_default(),
                similarity: read_dist(batch, row).map_or(0.0, |d| 1.0 - d),
            });
        }
    }
    // Merge deterministically: similarity descending, then canonical_key
    // ascending — a bare artifact key sorts before its own `X#sN` sections,
    // so a tie prefers the whole-artifact row over any of its sections. The
    // order is stable across runs because both inputs come from fixed pinned
    // versions and the tie-break is total.
    hits.sort_by(|a, b| {
        b.similarity
            .total_cmp(&a.similarity)
            .then_with(|| a.canonical_key.cmp(&b.canonical_key))
    });
    hits.truncate(limit);

    Ok(SemanticSearchOutcome {
        generation_id: manifest.generation_id.clone(),
        pinned_artifacts_version: artifacts_version,
        pinned_sections_version: sections_version,
        semantic_scope: if sections_version.is_some() {
            super::jobs::SEMANTIC_SCOPE_WITH_SECTIONS
        } else {
            super::jobs::SEMANTIC_SCOPE
        },
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
    // Stage 8 / C06 — the shared OS lock on the manifest, held for the
    // whole read; see [`super::storage::pin_generation_read`].
    let pinned = super::storage::pin_generation_read(index_root, &descriptor)?;
    let manifest = pinned.manifest();
    let artifacts_version = descriptor.effective_artifacts_version(manifest)?;
    if !manifest.table_versions.is_publishable() {
        return Err(ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "generation `{}` names non-publishable table versions {:?}",
                manifest.generation_id, manifest.table_versions
            ),
        });
    }
    let marker = marker_for(manifest);
    let table = super::lance_tables::open_table_at_version(
        index_root,
        &marker,
        "artifacts",
        artifacts_version,
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
                artifacts_version,
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

    /// The unrestricted filter, shared by every search call in this module
    /// that is not about filtering.
    const NO_FILTER: PinnedFilter = PinnedFilter {
        kind: None,
        status: None,
    };

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

    /// The index root this fixture writes into, canonicalised.
    ///
    /// `TempDir::new()` lands under macOS `/var`, which is a symlink to
    /// `/private/var`. The durability barrier refuses a symlinked index-root
    /// component on purpose — two spellings of one index would mean two physical
    /// owners — so the fixture must hand the barrier the physical path, the same
    /// rule `MaintainCapability::for_strict_maintenance` applies to callers.
    fn index_root(tmp: &TempDir) -> std::path::PathBuf {
        tmp.path().canonicalize().unwrap().join("lance")
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

    /// A workspace with no index at all is not migrated — this is the case that
    /// protects every pre-Constellation workspace, and a branch that answered
    /// `true` here would send all of them to an index that does not exist.
    #[tokio::test]
    async fn a_workspace_without_an_index_is_not_migrated() {
        let tmp = workspace();
        assert!(
            !workspace_is_migrated(tmp.path()),
            "no PROTOCOL.json and no CURRENT means no Constellation index"
        );
    }

    /// Once `maintain_index` has published, the same workspace is migrated.
    ///
    /// The fixture is the real publisher rather than a hand-written marker, so
    /// this pins the state the branch reads rather than the shape of the file.
    #[tokio::test]
    async fn a_published_workspace_is_migrated() {
        let tmp = workspace();
        let capability =
            MaintainCapability::for_strict_maintenance(tmp.path().canonicalize().unwrap());
        maintain_index(
            &capability,
            CaptureLimits::default(),
            std::time::Instant::now() + std::time::Duration::from_secs(120),
        )
        .await
        .unwrap();

        assert!(
            workspace_is_migrated(tmp.path()),
            "a published generation is a migrated workspace"
        );
    }

    /// A `PROTOCOL.json` that does not parse still means "Constellation index",
    /// not "legacy index".
    ///
    /// This is the judgement call the branch turns on: the marker exists, so
    /// the index claims Constellation and simply cannot say what it published.
    /// Reporting `false` would route a damaged workspace to the legacy writer,
    /// which would then create tables beside the ones already there.
    #[tokio::test]
    async fn a_damaged_marker_still_counts_as_migrated() {
        let tmp = workspace();
        let marker_path = index_root(&tmp).join(super::super::storage::PROTOCOL_RELATIVE_PATH);
        std::fs::create_dir_all(marker_path.parent().unwrap()).unwrap();
        std::fs::write(&marker_path, b"{ this is not json").unwrap();

        assert!(
            workspace_is_migrated(tmp.path()),
            "an unparseable marker must not be read as 'no Constellation index'"
        );
    }

    /// A `CURRENT` pointer alone is enough. The marker is derived state, so a
    /// workspace that lost it still published something — and the read path must
    /// not fall back to the legacy store for it.
    ///
    /// The descriptor is built through [`CurrentDescriptor`]'s own serde rather
    /// than hand-written JSON, so this fails loudly if the descriptor gains a
    /// required field instead of quietly testing a shape nothing produces.
    #[tokio::test]
    async fn a_current_pointer_without_a_marker_counts_as_migrated() {
        let tmp = workspace();
        let root = index_root(&tmp);
        std::fs::create_dir_all(&root).unwrap();

        let descriptor: CurrentDescriptor = serde_json::from_value(serde_json::json!({
            "format_version": super::super::FORMAT_VERSION,
            "index_epoch": 1,
            "checkout_id": "ck",
            "generation_id": "gen-x",
            "owner_epoch": 1,
            "source_set_digest": "d",
            "config_digest": "c",
            "profile_id": null,
            "evaluated_at": "2026-01-01T00:00:00Z",
            "manifest_path": "generations/gen-x.json",
            "manifest_sha256": "0",
        }))
        .expect("the descriptor is complete");
        std::fs::write(
            root.join(super::super::storage::CURRENT_FILE_NAME),
            serde_json::to_vec(&descriptor).unwrap(),
        )
        .unwrap();

        assert!(
            super::super::storage::read_marker(&root)
                .map(|state| matches!(state, IndexProtocolState::Uninitialized))
                .unwrap_or(false),
            "precondition: the marker really is absent, so CURRENT is the only evidence"
        );
        assert!(
            workspace_is_migrated(tmp.path()),
            "a published generation is proof of migration even without the marker"
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
        let descriptor = super::super::storage::descriptor_for(&manifest, 7, &sha, None);
        super::super::storage::publish_current(&root, &descriptor, barrier.as_ref()).unwrap();

        let view = ensure_read_view(tmp.path(), Arc::clone(&captured))
            .await
            .unwrap();
        assert!(view.is_persisted());
        assert_eq!(view.snapshot_id(), "gen-1");
        assert_eq!(view.coverage().read, captured.entries.len());
    }

    #[tokio::test]
    async fn a_persisted_view_holds_its_generation_against_gc_until_it_is_dropped() {
        let tmp = workspace();
        let root = index_root(&tmp);
        let barrier = barrier_for(&root);
        let captured = capture(tmp.path()).await;

        let manifest = manifest_for(&captured, "gen-1");
        let sha =
            super::super::storage::write_generation_manifest(&root, &manifest, barrier.as_ref())
                .unwrap();
        let mut descriptor = super::super::storage::descriptor_for(&manifest, 7, &sha, None);
        // A completion drain republishes the descriptor with new heads and
        // leaves the immutable manifest alone.
        descriptor.embedding_artifacts_head = Some(11);
        super::super::storage::publish_current(&root, &descriptor, barrier.as_ref()).unwrap();

        let view = ensure_read_view(tmp.path(), Arc::clone(&captured))
            .await
            .unwrap();
        assert_eq!(
            view.embedding_heads(),
            Some((Some(11), None)),
            "the heads a reader serves must be the ones CURRENT published, not the manifest's \
             own table versions"
        );

        // GC runs while the view is alive. The view holds a shared lock on the
        // manifest, so this generation is protected for that reason — and the
        // reason is the point: it is the reader, not a lease file, that keeps
        // the generation it is answering from alive.
        let report = super::super::generations::gc_generations(&root, 0)
            .await
            .unwrap();
        assert!(
            report.removed_manifests.is_empty(),
            "a live read view must protect the generation it answers from: {:?}",
            report.removed_manifests
        );
        assert!(
            report
                .protected
                .iter()
                .any(|(id, why)| { id == "gen-1" && why.starts_with("pinned:") }),
            "the protection must be the reader's, not merely `current`: {:?}",
            report.protected
        );
        assert!(root.join("generations").join("gen-1.json").exists());

        // The pin is released with the view. `gen-1` is still `CURRENT`, so it
        // stays — but it stays for that reason now, and nothing else: no TTL,
        // no lease file, no time passing.
        drop(view);
        let report = super::super::generations::gc_generations(&root, 0)
            .await
            .unwrap();
        assert!(
            !report
                .protected
                .iter()
                .any(|(id, why)| { id == "gen-1" && why.starts_with("pinned:") }),
            "dropping the reader must release its pin, or a long-lived server would pin every \
             generation it ever read: {:?}",
            report.protected
        );
        assert!(root.join("generations").join("gen-1.json").exists());
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
        let descriptor = super::super::storage::descriptor_for(&manifest, 7, &sha, None);
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
        let descriptor = super::super::storage::descriptor_for(&manifest, 7, &sha, None);
        super::super::storage::publish_current(&root, &descriptor, barrier.as_ref()).unwrap();

        // The pointer survives; the manifest it names does not.
        std::fs::remove_file(root.join("generations").join("gen-1.json")).unwrap();

        std::fs::write(tmp.path().join("prds/PRD-001-auth.md"), "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Changed\n---\n\nChanged while the manifest is missing.\n").unwrap();
        let captured = capture(tmp.path()).await;
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
        let descriptor = super::super::storage::descriptor_for(&manifest, 7, &sha, None);
        super::super::storage::publish_current(&root, &descriptor, barrier.as_ref()).unwrap();

        // Mutate the manifest behind the pointer's back.
        let path = root.join("generations").join("gen-1.json");
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push('\n');
        std::fs::write(&path, text).unwrap();
        std::fs::write(tmp.path().join("prds/PRD-001-auth.md"), "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Changed\n---\n\nChanged while the manifest is corrupt.\n").unwrap();
        let captured = capture(tmp.path()).await;

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
        let descriptor = super::super::storage::descriptor_for(&manifest, 3, &sha, None);

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
        let descriptor = super::super::storage::descriptor_for(&manifest, 1, &sha, None);
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
    fn rebind_of_a_root_with_no_marker_rewrites_nothing() {
        let tmp = workspace();
        let root = tmp.path().canonicalize().unwrap().join("lance");
        // The index root itself need not exist — the owner lock lives in
        // the (existing) canonical parent, and there is no marker to move.
        assert_eq!(rebind_index_identity(&root).unwrap(), None);
        assert_eq!(
            super::super::storage::read_marker(&root).unwrap(),
            IndexProtocolState::Uninitialized,
            "a fresh workspace mints its own identity at first publish"
        );
    }

    #[test]
    fn rebind_rewrites_a_moved_marker_and_burns_an_epoch() {
        let tmp = workspace();
        // The root must exist for the marker to exist, and an existing root
        // is classified component-wise — a TempDir's uncanonicalized `/var`
        // is itself a symlink on macOS, so the canonical spelling is used.
        let base = tmp.path().canonicalize().unwrap();
        let root = base.join("lance");
        let barrier = barrier_for(&root);
        std::fs::create_dir_all(&root).unwrap();

        // The marker an index carries after its checkout moved: it still
        // claims the identity of the old location.
        let old_root = tmp.path().join("old-home").join("lance");
        let old_bound = path_bound_checkout_id(&old_root);
        let new_bound = path_bound_checkout_id(&root);
        assert_ne!(old_bound, new_bound);
        let marker = ProtocolMarker {
            protocol_version: crate::constellation::FORMAT_VERSION,
            state: BootstrapState::Active,
            index_epoch: 1,
            checkout_id: old_bound.clone(),
            table_names: ProtocolMarker::active_table_names(),
            storage_scheme: STORAGE_SCHEME_FILE_OBJECT_STORE.to_string(),
            bootstrap_attempt: 2,
        };
        super::super::storage::write_marker(&root, &marker, barrier.as_ref()).unwrap();

        // Before the rebind the path-bound check refuses the index — the
        // `index_identity_mismatch` `maintain_index` would report.
        marker.check_identity(&new_bound).unwrap_err();

        let report = rebind_index_identity(&root)
            .unwrap()
            .expect("a marker exists");
        assert_eq!(report.old_checkout_id, old_bound);
        assert_eq!(report.new_checkout_id, new_bound);
        assert_eq!(report.old_epoch, 1);
        assert_eq!(report.new_epoch, 2);

        // After the rewrite the marker claims this path — `forgeplan index`
        // at the new location passes `check_identity`.
        match super::super::storage::read_marker(&root).unwrap() {
            IndexProtocolState::Active(read) => {
                read.check_identity(&new_bound).unwrap();
                assert_eq!(read.index_epoch, 2);
                // State survives the rewrite: an active index stays active.
                assert_eq!(read.state, BootstrapState::Active);
            }
            other => panic!("expected Active, got {other:?}"),
        }

        // The identity the next publisher derives is the rebound one — jobs
        // bound to the old `(index_epoch, checkout_id)` pair can never match.
        let identity = super::super::publisher::identity_for(&root);
        assert_eq!(identity.checkout_id(), new_bound);
        assert_eq!(identity.index_epoch(), 2);

        // A second rebind is a no-op: already bound, so no epoch is burned.
        assert_eq!(rebind_index_identity(&root).unwrap(), None);
        match super::super::storage::read_marker(&root).unwrap() {
            IndexProtocolState::Active(read) => assert_eq!(read.index_epoch, 2),
            other => panic!("expected Active, got {other:?}"),
        }
    }

    #[test]
    fn path_bound_checkout_id_tracks_the_spelled_root() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a").join("lance");
        let b = tmp.path().join("b").join("lance");
        assert_ne!(path_bound_checkout_id(&a), path_bound_checkout_id(&b));
        assert_eq!(path_bound_checkout_id(&a), path_bound_checkout_id(&a));
        assert!(path_bound_checkout_id(&a).starts_with("local-"));
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

        // The trigram side-index must be bound to this generation, and the
        // lexical search must answer through it — `via_trigram: false` would
        // mean the fast path silently fell back to the row scan.
        assert!(
            matches!(
                outcome.lexical_index,
                Some(crate::constellation::lexical_index::LexicalRebind::Bound { .. })
            ),
            "the lexical index should be bound after publish: {:?}",
            outcome.lexical_index
        );
        let found = search_pinned_lexical(capability.index_root(), "storage", 10, &NO_FILTER)
            .await
            .unwrap();
        assert!(
            found.via_trigram,
            "the bound trigram index must serve the query"
        );
        assert_eq!(found.hits.len(), 1);
        assert_eq!(found.hits[0].canonical_key, "ADR-002");
    }

    #[tokio::test]
    async fn recovery_without_a_source_delta_publishes_the_restored_heads_once() {
        resume_admission();
        let tmp = linked_workspace();
        let root = tmp.path().canonicalize().unwrap();
        let capability = MaintainCapability::for_strict_maintenance(root.clone());
        let deadline = || std::time::Instant::now() + std::time::Duration::from_secs(120);
        let (before, _) = maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .unwrap();
        {
            let publisher = super::super::publisher::IndexPublisher::acquire(
                capability.index_root(),
                inner_store_for(capability.index_root()),
            )
            .unwrap();
            let tables = super::super::lance_tables::open_active_tables(
                capability.index_root(),
                &marker_for(&before.manifest),
                Arc::clone(publisher.tracking_store()),
            )
            .await
            .unwrap();
            tables
                .delete_from_artifacts(&["PRD-001".to_string()])
                .await
                .unwrap();
            assert_ne!(
                tables.head_versions().await.unwrap().artifacts,
                before.manifest.table_versions.artifacts
            );
            // Release the owner with a real committed table delta but no CURRENT switch.
        }
        let (recovered, outcome) =
            maintain_index(&capability, CaptureLimits::default(), deadline())
                .await
                .unwrap();
        let descriptor = storage::read_current(capability.index_root())
            .unwrap()
            .unwrap();
        assert_eq!(
            descriptor
                .effective_artifacts_version(&recovered.manifest)
                .unwrap(),
            outcome.head_versions.artifacts
        );
        assert!(
            get_pinned_artifact(capability.index_root(), "PRD-001")
                .await
                .unwrap()
                .is_some()
        );
        let (_, repeated) = maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .unwrap();
        assert_eq!(
            repeated.head_versions, outcome.head_versions,
            "a second pass must not restore again"
        );
        assert!(repeated.commits.is_empty());
        assert_eq!(
            storage::read_current(capability.index_root())
                .unwrap()
                .unwrap(),
            descriptor
        );
    }

    #[tokio::test]
    async fn a_configuration_only_change_is_published_once_with_updated_inputs() {
        resume_admission();
        let tmp = linked_workspace();
        let root = tmp.path().canonicalize().unwrap();
        let path = root.join("prds/PRD-001-auth.md");
        let original = format!(
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\n---\n\n{}\n",
            "x".repeat(1024)
        );
        std::fs::write(&path, &original).unwrap();
        let capability = MaintainCapability::for_strict_maintenance(root.clone());
        let deadline = || std::time::Instant::now() + std::time::Duration::from_secs(120);
        let (before, _) = maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .unwrap();
        std::fs::write(root.join("config.yaml"), "embedding:\n  chunk_size: 512\n").unwrap();
        let (after, _) = maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .unwrap();
        assert_ne!(before.manifest.config_digest, after.manifest.config_digest);
        let entry = |manifest: &GenerationManifest| {
            manifest
                .source_entries
                .iter()
                .find(|entry| entry.canonical_key == "PRD-001")
                .unwrap()
                .input_digest
                .clone()
        };
        assert_ne!(entry(&before.manifest), entry(&after.manifest));
        let read = open_workspace_read(&root, None, CaptureLimits::default())
            .await
            .unwrap();
        assert!(
            read.is_persisted(),
            "the new configuration must have been published"
        );
        assert_eq!(read.captured().body_prefix_chars, 512);
        let descriptor = storage::read_current(capability.index_root())
            .unwrap()
            .unwrap();
        let (_, unchanged) = maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .unwrap();
        assert!(unchanged.commits.is_empty());
        assert_eq!(
            storage::read_current(capability.index_root())
                .unwrap()
                .unwrap(),
            descriptor
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[tokio::test]
    async fn an_incomplete_canonical_read_is_unavailable_not_an_empty_workspace() {
        let tmp = linked_workspace();
        std::fs::write(
            tmp.path().join("prds/PRD-001-auth.md"),
            "---\ntitle: [broken\n---\nBody.\n",
        )
        .unwrap();
        let error = open_workspace_read(tmp.path(), None, CaptureLimits::default())
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            ConstellationError::SourceUnavailable { .. }
        ));
    }

    #[cfg(feature = "semantic-search")]
    #[tokio::test]
    async fn an_ephemeral_capture_reuses_only_vectors_for_its_exact_current_inputs() {
        resume_admission();
        let tmp = linked_workspace();
        let root = tmp.path().canonicalize().unwrap();
        let capability = MaintainCapability::for_strict_maintenance(root.clone());
        let deadline = || std::time::Instant::now() + std::time::Duration::from_secs(120);
        maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .unwrap();
        let profile = test_profile(2000);
        embed_once(&capability, &profile, 0.25).await.unwrap();
        let pointer = std::fs::read(capability.index_root().join("CURRENT.json")).unwrap();
        let prd = root.join("prds/PRD-001-auth.md");
        std::fs::write(
            &prd,
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\n---\n\nChanged input.\n",
        )
        .unwrap();
        let adr = root.join("adrs/ADR-002-storage.md");
        let text = std::fs::read_to_string(&adr)
            .unwrap()
            .replace("status: active", "status: draft");
        std::fs::write(&adr, text).unwrap();
        std::fs::write(
            root.join("adrs/ADR-003-new.md"),
            "---\nid: ADR-003\nkind: adr\nstatus: draft\ntitle: New\n---\n\nNo vector yet.\n",
        )
        .unwrap();
        let read = open_workspace_read(&root, None, CaptureLimits::default())
            .await
            .unwrap();
        assert!(!read.is_persisted());
        let query = vec![0.25f32; crate::db::schema::EMBEDDING_DIM as usize];
        let found = search_semantic_in_view(
            capability.index_root(),
            read.view(),
            &query,
            read.view()
                .semantic_profile_id()
                .expect("retain the verified vector space"),
            1,
            &PinnedFilter {
                status: Some("draft".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(
            found.eligible_rows, 2,
            "only unchanged ADR-002 and its section remain eligible"
        );
        assert_eq!(found.hits.len(), 1);
        assert_eq!(
            found.hits[0]
                .parent_key
                .as_deref()
                .unwrap_or(&found.hits[0].canonical_key),
            "ADR-002"
        );
        assert_eq!(read.get_record("ADR-002").unwrap().status, "draft");
        std::fs::rename(&adr, root.join("adrs/ADR-002-renamed.md")).unwrap();
        let renamed = open_workspace_read(&root, None, CaptureLimits::default())
            .await
            .unwrap();
        let excluded = search_semantic_in_view(
            capability.index_root(),
            renamed.view(),
            &query,
            &profile.profile_id(),
            10,
            &NO_FILTER,
        )
        .await
        .unwrap();
        assert_eq!(
            excluded.eligible_rows, 0,
            "a moved path cannot reuse its previous incarnation"
        );
        assert!(excluded.hits.is_empty());
        assert_eq!(
            std::fs::read(capability.index_root().join("CURRENT.json")).unwrap(),
            pointer
        );
    }

    #[tokio::test]
    async fn inventory_stays_pinned_and_tracks_updates_and_deletes() {
        resume_admission();
        let tmp = linked_workspace();
        let root = tmp.path().canonicalize().unwrap();
        let capability = MaintainCapability::for_strict_maintenance(root.clone());
        let deadline = || std::time::Instant::now() + std::time::Duration::from_secs(120);
        maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .unwrap();
        let first = list_pinned_documents(capability.index_root(), 0, 100)
            .await
            .unwrap();
        assert_eq!(first.documents.len(), 2);
        assert!(
            first
                .documents
                .iter()
                .all(|row| !row.source_digest.is_empty())
        );
        let adr = root.join("adrs/ADR-002-storage.md");
        std::fs::write(
            &adr,
            "---\nid: ADR-002\nkind: adr\nstatus: active\ntitle: New storage\n---\n\nNew body.\n",
        )
        .unwrap();
        maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .unwrap();
        let updated = list_pinned_documents(capability.index_root(), 0, 100)
            .await
            .unwrap();
        assert_ne!(first.generation_id, updated.generation_id);
        let digest = |page: &IndexedDocumentPage| {
            page.documents
                .iter()
                .find(|row| row.canonical_key == "ADR-002")
                .unwrap()
                .source_digest
                .clone()
        };
        assert_ne!(digest(&first), digest(&updated));
        std::fs::remove_file(&adr).unwrap();
        maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .unwrap();
        let after_delete = list_pinned_documents(capability.index_root(), 0, 100)
            .await
            .unwrap();
        assert_eq!(after_delete.documents.len(), 1);
        assert_eq!(after_delete.documents[0].canonical_key, "PRD-001");
        assert_eq!(
            after_delete.documents[0].source_digest,
            first
                .documents
                .iter()
                .find(|row| row.canonical_key == "PRD-001")
                .unwrap()
                .source_digest
        );
    }

    /// A profile whose dimensions match the table, so admission will accept it.
    /// Distinct profiles differ in `body_prefix_chars`, which changes `profile_id`
    /// and therefore the whole binding identity — that is what makes a
    /// profile-to-profile migration observable here.
    #[cfg(feature = "semantic-search")]
    fn test_profile(body_prefix_chars: usize) -> super::super::jobs::EmbeddingProfile {
        super::super::jobs::EmbeddingProfile {
            model_repo: super::super::jobs::DEFAULT_MODEL_REPO.to_string(),
            weights_revision: "test-revision".to_string(),
            weights_checksums: vec!["weights-sha".to_string()],
            tokenizer_checksum: "tok-sha".to_string(),
            tokenizer_config_checksum: "tokcfg-sha".to_string(),
            dimensions: super::super::lance_tables::EMBEDDING_DIM as u32,
            engine_version: ENGINE_VERSION.to_string(),
            pooling: "cls".to_string(),
            normalization: "l2".to_string(),
            query_prefix: String::new(),
            document_prefix: String::new(),
            body_prefix_chars,
            max_tokens: super::super::jobs::DEFAULT_MAX_TOKENS,
            padding: "none".to_string(),
            truncation: "longest_first".to_string(),
        }
    }

    /// Admit, compute a constant fill, and complete — one full cycle, with no
    /// model. `fill` is the marker that lets one cycle's vectors be told from
    /// another's.
    #[cfg(feature = "semantic-search")]
    async fn embed_once(
        capability: &MaintainCapability,
        profile: &super::super::jobs::EmbeddingProfile,
        fill: f32,
    ) -> Result<EmbeddingBatchOutcome, ConstellationError> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        let prepared = prepare_embedding_batch(capability, profile, deadline).await?;
        if prepared.inputs.is_empty() {
            return Ok(EmbeddingBatchOutcome {
                profile_id: profile.profile_id(),
                eligible_keys: Vec::new(),
                published: Vec::new(),
                stale: Vec::new(),
                cannot_judge: Vec::new(),
            });
        }
        let results: Vec<super::super::jobs::ComputedEmbedding> = prepared
            .inputs
            .iter()
            .map(|input| super::super::jobs::ComputedEmbedding {
                job: input.job.clone(),
                vector: vec![fill; crate::db::schema::EMBEDDING_DIM as usize],
            })
            .collect();
        complete_embedding_batch(capability, &prepared, &results, deadline).await
    }

    /// The completion writes vectors past the manifest's pin and records that
    /// head on the descriptor. The next reconcile must treat it as the
    /// baseline — before this contract existed, `compare_heads` read the
    /// movement as divergence and restored over the vectors on every pass.
    ///
    /// No model: the vectors are a constant fill, which is enough to prove
    /// *which* rows completed and never says anything about similarity.
    #[cfg(feature = "semantic-search")]
    #[tokio::test]
    async fn a_completed_embedding_batch_survives_the_next_reconcile() {
        resume_admission();
        let tmp = linked_workspace();
        let root = tmp.path().canonicalize().unwrap();
        let capability = MaintainCapability::for_strict_maintenance(root.clone());
        let deadline = || std::time::Instant::now() + std::time::Duration::from_secs(120);
        maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .unwrap();

        let profile = test_profile(2000);
        let first = embed_once(&capability, &profile, 0.25).await.unwrap();
        assert!(
            !first.published.is_empty(),
            "the completion must publish at least one vector: {first:?}"
        );
        assert!(first.stale.is_empty(), "nothing moved: {first:?}");

        // A reconcile over unchanged sources commits nothing — and must not
        // rewind the head the completion recorded.
        maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .unwrap();
        let page = list_pinned_documents(capability.index_root(), 0, 100)
            .await
            .unwrap();
        assert!(
            page.documents
                .iter()
                .all(|d| d.semantic_state.as_deref() == Some("ready")),
            "a pinned read of the current generation must serve the completed state"
        );

        // And the queue is closed: a second cycle under the same profile has
        // nothing to admit, so nothing is re-embedded.
        let second = embed_once(&capability, &profile, 0.25).await.unwrap();
        assert!(
            second.published.is_empty(),
            "a completed row must not be re-embedded: {second:?}"
        );

        // The vector itself is reachable: querying with the same vector the
        // completion published must return the row — similarity 1.0 modulo f32.
        let query = vec![0.25f32; crate::db::schema::EMBEDDING_DIM as usize];
        let found = search_pinned_semantic(
            capability.index_root(),
            &query,
            &profile.profile_id(),
            10,
            &NO_FILTER,
        )
        .await
        .unwrap();
        assert!(
            !found.hits.is_empty(),
            "a ready row at the completed head must be queryable: {:?}/{:?}",
            found.eligible_rows,
            found.pinned_artifacts_version
        );
    }

    /// A captured view must keep answering from the generation it captured,
    /// even after a **later** generation has been published.
    ///
    /// This is the substitution the captured path exists to prevent. The view
    /// is taken at generation G; the workspace is then edited and a second pass
    /// publishes H. A search that re-reads `CURRENT` would score G's records
    /// against H's vectors — every field self-consistent, the attribution
    /// wrong — so both entry points are exercised here and must report
    /// different generations.
    ///
    /// No model is involved: the query vector is the caller's, and the
    /// observable that distinguishes the two answers is the generation each one
    /// reports, which is decided before a single row is ranked.
    #[tokio::test]
    async fn a_captured_view_searches_its_own_generation_after_a_later_one_is_published() {
        resume_admission();
        let tmp = linked_workspace();
        let root = tmp.path().canonicalize().unwrap();
        let capability = MaintainCapability::for_strict_maintenance(root.clone());
        let deadline = || std::time::Instant::now() + std::time::Duration::from_secs(120);
        maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .unwrap();

        // The view a read would have captured at G.
        let captured = capture(&root).await;
        let view = ensure_read_view(&root, Arc::clone(&captured))
            .await
            .unwrap();
        assert!(
            view.is_persisted(),
            "an unchanged workspace must read as the published generation"
        );
        let generation_g = view.snapshot_id().to_string();

        // A real change, published. H must be a genuinely different generation,
        // otherwise the assertion below would pass for the wrong reason.
        std::fs::write(
            root.join("adrs").join("ADR-003-payments.md"),
            "---\nid: ADR-003\nkind: adr\nstatus: active\ntitle: Payments\n---\n\nBody.\n",
        )
        .unwrap();
        maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .unwrap();
        let generation_h = super::super::storage::read_current(capability.index_root())
            .unwrap()
            .expect("a pass published a generation")
            .generation_id;
        assert_ne!(
            generation_g, generation_h,
            "the fixture must publish a different generation, or this test proves nothing"
        );

        // A profile no row is bound under: the ranking is empty either way, and
        // what is asserted is which generation answered.
        let profile_id = "profile-that-no-row-is-ready-under";
        let query = vec![0.25f32; crate::db::schema::EMBEDDING_DIM as usize];

        let current =
            search_pinned_semantic(capability.index_root(), &query, profile_id, 10, &NO_FILTER)
                .await
                .unwrap();
        assert_eq!(
            current.generation_id, generation_h,
            "the CURRENT lookup is the operation that answers from whatever is published now"
        );

        let at_g = search_semantic_in_view(
            capability.index_root(),
            &view,
            &query,
            profile_id,
            10,
            &NO_FILTER,
        )
        .await
        .unwrap();
        assert_eq!(
            at_g.generation_id, generation_g,
            "a captured view must keep answering from the generation it captured"
        );
        assert_eq!(
            at_g.pinned_artifacts_version,
            view.descriptor()
                .expect("a persisted view carries its descriptor")
                .effective_artifacts_version(
                    view.manifest()
                        .expect("a persisted view carries its manifest"),
                )
                .unwrap(),
            "the tables it opened must be the ones that generation's descriptor resolves to, \
             not the head CURRENT names now"
        );
    }

    /// A capture without a retained pin must not borrow a later CURRENT.
    #[tokio::test]
    async fn a_capture_without_a_pinned_cache_does_not_borrow_current() {
        resume_admission();
        let tmp = linked_workspace();
        let root = tmp.path().canonicalize().unwrap();
        let capability = MaintainCapability::for_strict_maintenance(root.clone());
        let deadline = || std::time::Instant::now() + std::time::Duration::from_secs(120);
        maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .unwrap();

        // This capture deliberately has no cache; publication alone cannot
        // authorize a later semantic lookup through it.
        std::fs::write(
            root.join("prds").join("PRD-001-auth.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\nlinks:\n  - target: ADR-002\n    relation: refines\n---\n\nEdited after publication.\n",
        )
        .unwrap();
        let moved = capture(&root).await;
        let view = ReadView::from_capture(Arc::clone(&moved));
        assert!(
            !view.is_persisted(),
            "a capture that does not match the published source set is ephemeral"
        );
        assert!(
            super::super::storage::read_current(capability.index_root())
                .unwrap()
                .is_some(),
            "a generation is published — which is exactly what must not be borrowed"
        );

        let query = vec![0.25f32; crate::db::schema::EMBEDDING_DIM as usize];
        let err = search_semantic_in_view(
            capability.index_root(),
            &view,
            &query,
            "any-profile",
            10,
            &NO_FILTER,
        )
        .await
        .expect_err("an uncached capture cannot borrow a generation");
        assert_eq!(
            err.code(),
            crate::constellation::DiagnosticCode::IndexGenerationUnavailable
        );
    }

    /// An index root that does not resolve the captured generation is refused
    /// rather than answered from whatever is there.
    ///
    /// A second published workspace is the honest fixture: both roots have a
    /// `CURRENT`, and both answer — just not with this view's generation.
    #[tokio::test]
    async fn a_captured_view_refuses_an_index_root_that_serves_another_generation() {
        resume_admission();
        let captured_workspace = linked_workspace();
        let other_workspace = linked_workspace();
        // Deliberately different content: two indexes that publish the *same*
        // generation id would make the mismatch below unrepresentable, and the
        // test would pass for the wrong reason.
        std::fs::write(
            other_workspace
                .path()
                .join("adrs")
                .join("ADR-009-identity.md"),
            "---\nid: ADR-009\nkind: adr\nstatus: active\ntitle: Identity\n---\n\nOther index.\n",
        )
        .unwrap();
        let captured_root = captured_workspace.path().canonicalize().unwrap();
        let other_root = other_workspace.path().canonicalize().unwrap();
        let deadline = || std::time::Instant::now() + std::time::Duration::from_secs(120);
        for root in [&captured_root, &other_root] {
            maintain_index(
                &MaintainCapability::for_strict_maintenance(root.clone()),
                CaptureLimits::default(),
                deadline(),
            )
            .await
            .unwrap();
        }

        let captured = capture(&captured_root).await;
        let view = ensure_read_view(&captured_root, Arc::clone(&captured))
            .await
            .unwrap();
        assert!(view.is_persisted());

        let query = vec![0.25f32; crate::db::schema::EMBEDDING_DIM as usize];

        // The same view against its own root answers, so the refusal below is
        // about the root and not about the fixture.
        search_semantic_in_view(
            &captured_root.join(MaintainCapability::INDEX_DIR),
            &view,
            &query,
            "any-profile",
            10,
            &NO_FILTER,
        )
        .await
        .expect("the view's own index root must answer");

        let err = search_semantic_in_view(
            &other_root.join(MaintainCapability::INDEX_DIR),
            &view,
            &query,
            "any-profile",
            10,
            &NO_FILTER,
        )
        .await
        .expect_err("a root that does not hold this generation must not answer for it");
        assert_eq!(
            err.code(),
            super::super::DiagnosticCode::IndexGenerationUnavailable,
            "a root that cannot resolve this generation is unavailable, never a different \
             generation's answer: {err}"
        );
    }

    /// Abandoned head writes are rewound before admission reads the tables.
    ///
    /// A pass that crashed between its table commits and its manifest publish
    /// leaves a head nobody published. Both admission and completion now run the
    /// same recovery a reconcile does, so the rows an admission writes `pending`
    /// onto — and the state a completion reads to decide what is live — are the
    /// published ones rather than whatever the abandoned pass left behind.
    ///
    /// The write below is made **through a real Lance connection**, not a mock, so
    /// it is exactly the situation the restore exists for.
    #[cfg(feature = "semantic-search")]
    #[tokio::test]
    async fn an_abandoned_head_is_restored_before_admission_reads_the_tables() {
        resume_admission();
        let tmp = linked_workspace();
        let root = tmp.path().canonicalize().unwrap();
        let capability = MaintainCapability::for_strict_maintenance(root.clone());
        let deadline = || std::time::Instant::now() + std::time::Duration::from_secs(120);
        maintain_index(&capability, CaptureLimits::default(), deadline())
            .await
            .unwrap();

        let index_root = capability.index_root().to_path_buf();
        let published_head = {
            let descriptor = super::super::storage::read_current(&index_root)
                .unwrap()
                .unwrap();
            let manifest = super::super::storage::read_manifest(&index_root, &descriptor).unwrap();
            descriptor.effective_artifacts_version(&manifest).unwrap()
        };

        // An abandoned pass: a real commit to the artifacts table that no
        // manifest, and therefore no descriptor, ever names.
        {
            let connection = super::super::lance_tables::open_wrapped_connection(&index_root)
                .await
                .unwrap();
            let table = connection
                .open_table(super::super::manifest::TABLE_ARTIFACTS_V1)
                .execute()
                .await
                .unwrap();
            table
                .update()
                .column("semantic_state", "'failed'")
                .execute()
                .await
                .unwrap();
        }

        let profile = test_profile(2000);
        let prepared = prepare_embedding_batch(&capability, &profile, deadline())
            .await
            .expect("admission must recover the published baseline, not fail on it");
        assert!(
            !prepared.inputs.is_empty(),
            "the restored rows are still owed vectors"
        );

        let descriptor = super::super::storage::read_current(&index_root)
            .unwrap()
            .unwrap();
        let manifest = super::super::storage::read_manifest(&index_root, &descriptor).unwrap();
        assert!(
            manifest.table_versions.artifacts > published_head,
            "admission committed its own `pending` writes on top of the restored head"
        );
        let head_after = descriptor.effective_artifacts_version(&manifest).unwrap();
        assert!(
            head_after > published_head,
            "the head admission published must be the one the manifest and the descriptor agree on"
        );

        // The abandoned `failed` state is gone: the rows now say `pending`,
        // because the restore rewound them and admission re-admitted them.
        let page = list_pinned_documents(&index_root, 0, 100).await.unwrap();
        assert!(
            page.documents
                .iter()
                .all(|d| d.semantic_state.as_deref() == Some("pending")),
            "the abandoned pass's state must not survive into the admitted rows: {:?}",
            page.documents
        );
    }

    #[tokio::test]
    async fn incremental_publication_matches_a_fresh_build_after_mixed_changes() {
        resume_admission();
        let incremental = linked_workspace();
        let fresh = linked_workspace();
        let apply_final_state = |root: &Path| {
            std::fs::remove_file(root.join("adrs/ADR-002-storage.md")).unwrap();
            std::fs::rename(
                root.join("prds/PRD-001-auth.md"),
                root.join("prds/PRD-001-renamed.md"),
            )
            .unwrap();
            std::fs::write(
                root.join("prds/PRD-001-renamed.md"),
                "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth revised\n---\n\nChanged.\n",
            )
            .unwrap();
            std::fs::write(
                root.join("specs/SPEC-002-new.md"),
                "---\nid: SPEC-002\nkind: spec\nstatus: draft\ntitle: New\n---\n\nNew document.\n",
            )
            .unwrap();
        };
        let incremental_cap =
            MaintainCapability::for_strict_maintenance(incremental.path().canonicalize().unwrap());
        let fresh_cap =
            MaintainCapability::for_strict_maintenance(fresh.path().canonicalize().unwrap());
        let deadline = || std::time::Instant::now() + std::time::Duration::from_secs(120);
        maintain_index(&incremental_cap, CaptureLimits::default(), deadline())
            .await
            .unwrap();
        apply_final_state(incremental.path());
        apply_final_state(fresh.path());
        // Stage 8 requires the equivalence to be *measured*: both the
        // delta pass and the fresh build report their wall time and their
        // commit counters, and the resulting row sets must coincide.
        let t_delta = std::time::Instant::now();
        let (updated, delta_outcome) =
            maintain_index(&incremental_cap, CaptureLimits::default(), deadline())
                .await
                .unwrap();
        let delta_elapsed = t_delta.elapsed();
        let t_fresh = std::time::Instant::now();
        let (rebuilt, fresh_outcome) =
            maintain_index(&fresh_cap, CaptureLimits::default(), deadline())
                .await
                .unwrap();
        let fresh_elapsed = t_fresh.elapsed();
        let rows_of = |outcome: &ReconcileOutcome| -> u64 {
            outcome.commits.iter().map(|c| c.rows_affected).sum()
        };
        eprintln!(
            "equivalence measurement: delta pass {delta_elapsed:?} ({} commits, {} rows) vs fresh build {fresh_elapsed:?} ({} commits, {} rows)",
            delta_outcome.commits.len(),
            rows_of(&delta_outcome),
            fresh_outcome.commits.len(),
            rows_of(&fresh_outcome),
        );
        // On a two-doc fixture a delta pass can legitimately touch as
        // many rows as a fresh build — deletes and merges are counted on
        // both sides. The assertion that carries the spec's weight is
        // below: identical resulting key/path/digest sets. The counters
        // above exist so a regression that makes "incremental" mean
        // "rewrite everything" shows up in the numbers, not the verdict.
        assert!(
            !delta_outcome.commits.is_empty(),
            "a changed corpus produced no delta commits"
        );
        let page = |rows: Vec<IndexedDocument>| {
            rows.into_iter()
                .map(|row| (row.canonical_key, row.relative_path, row.source_digest))
                .collect::<std::collections::BTreeSet<_>>()
        };
        let delta_rows = list_pinned_documents(incremental_cap.index_root(), 0, 100)
            .await
            .unwrap()
            .documents;
        let fresh_rows = list_pinned_documents(fresh_cap.index_root(), 0, 100)
            .await
            .unwrap()
            .documents;
        assert_eq!(page(delta_rows), page(fresh_rows));
        assert_eq!(updated.manifest.coverage, rebuilt.manifest.coverage);
    }

    #[tokio::test]
    async fn lexical_shortlist_includes_derived_keys_and_falls_back_when_stale() {
        resume_admission();
        let tmp = linked_workspace();
        std::fs::write(
            tmp.path().join("specs/SPEC-003-filenameonly.md"),
            "---\nkind: spec\nstatus: draft\ntitle: Unrelated\n---\n\nNothing about its name.\n",
        )
        .unwrap();
        let capability =
            MaintainCapability::for_strict_maintenance(tmp.path().canonicalize().unwrap());
        maintain_index(
            &capability,
            CaptureLimits::default(),
            std::time::Instant::now() + std::time::Duration::from_secs(120),
        )
        .await
        .unwrap();
        let result = search_pinned_lexical(capability.index_root(), "filenameonly", 10, &NO_FILTER)
            .await
            .unwrap();
        assert!(result.via_trigram);
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].canonical_key, "SPEC-003-filenameonly");

        std::fs::write(
            tmp.path().join("tgrep/generation.json"),
            r#"{"generation_id":"stale","source_set_digest":"stale","built_at":"now"}"#,
        )
        .unwrap();
        let fallback =
            search_pinned_lexical(capability.index_root(), "filenameonly", 10, &NO_FILTER)
                .await
                .unwrap();
        assert!(!fallback.via_trigram);
        assert_eq!(fallback.hits[0].canonical_key, "SPEC-003-filenameonly");
    }
}
