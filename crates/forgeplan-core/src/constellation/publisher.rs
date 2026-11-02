//! Bootstrap driver and publisher — the only writer.
//!
//! Everything here runs **only** under an explicit Strict maintain holding the
//! [`MaintainCapability`](super::facade::MaintainCapability). A read never
//! reaches this module, which is what keeps the read path free of table
//! creation, migrations and lock acquisition.
//!
//! # The ordering this module guarantees
//!
//! ```text
//! joined storage writes
//!   → closure barrier             (StageTracker::sync_written_set)
//!   → generation manifest barrier (write_generation_manifest)
//!   → same-filesystem CURRENT replacement   (the visibility point)
//!   → platform durability ACK                (a different guarantee)
//! ```
//!
//! Every step below is placed by that ordering, not by convenience. Taking the
//! closure before the manifest, and the manifest before the pointer, is what
//! makes a crash at any point leave either the previous generation or the new
//! one — never a pointer to a manifest that was never fully written.
//!
//! # Why there is one writer
//!
//! The physical owner serialises publishers. Two processes staging against one
//! index would each compute a delta from the same published baseline and each
//! publish it, and the second would silently overwrite the first's work while
//! both reported success.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use super::capture::{CapturedSourceView, SourceEntry};
use super::jobs;

use super::durability::{PlatformDurability, StageTracker, platform_barrier};
use super::owner::{IndexIdentity, IndexLocation};
use super::reconciler::{DeltaAction, ReconcilePlan};
use super::storage::{self, IndexProtocolState};
use super::{ConstellationError, DiagnosticCode, checked_increment, rfc3339_now};
use crate::constellation::manifest::{
    BootstrapState, CurrentDescriptor, GenerationManifest, ProtocolMarker, TableVersions,
};

/// Budget for acquiring the physical owner before reporting `index_busy`.
const OWNER_BUDGET_MS: u64 = super::owner::OWNER_ACQUIRE_BUDGET_MS;

/// Role names of the three active tables, in the order `storage::ACTIVE_TABLES`
/// declares them.
pub const ROLE_ARTIFACTS: &str = "artifacts";
pub const ROLE_RELATIONS: &str = "relations";
/// The per-section embedding table. Its cardinality is the corpus's section
/// count, not the artifact count — one row per `"{parent}#s{ordinal}"`.
pub const ROLE_SECTIONS: &str = "sections";

/// What a restart found, and what has to happen next.
///
/// Computed without touching Lance, so the restart laws can be tested on their
/// own — they are decisions about **which failure** an index is in, and getting
/// one wrong is how a first run turns into a silent reinitialisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapPlan {
    /// No marker, or a marker that says `initializing` with no usable
    /// pointer. Rebuild from a full canonical capture in the declared new
    /// tables. Partial candidates from the interrupted attempt are ignored,
    /// never adopted.
    BuildInitialBaseline,
    /// `initializing` with a valid pointer: the first generation was published
    /// and only the marker flip is missing. Validate, then finish.
    FinishActivation { generation_id: String },
    /// The pointer exists but does not resolve. Bootstrap is **not** restarted:
    /// the index published something once and has since lost the ability to
    /// describe it.
    Unavailable { generation_id: String },
}

/// Decide what a restart must do, from what is on disk.
///
/// `current` is `Ok(None)` for an absent pointer and `Err` for one that
/// exists but does not parse — the distinction is the whole point, because the
/// first is a first run and the second is a damaged index.
pub fn plan_bootstrap(
    state: &IndexProtocolState,
    current: Result<Option<CurrentDescriptor>, ConstellationError>,
) -> BootstrapPlan {
    match (state, current) {
        // An absent pointer under a live marker is the normal crash-during-
        // bootstrap case: rebuild, do not adopt partial candidates.
        (IndexProtocolState::Uninitialized, Ok(None)) => BootstrapPlan::BuildInitialBaseline,
        (IndexProtocolState::Uninitialized, Ok(Some(_))) => BootstrapPlan::Unavailable {
            generation_id: "unknown".to_string(),
        },
        (IndexProtocolState::Uninitialized, Err(e)) => BootstrapPlan::Unavailable {
            generation_id: format!("<unreadable: {e}>"),
        },

        (IndexProtocolState::Initializing(_), Ok(None)) => BootstrapPlan::BuildInitialBaseline,
        (IndexProtocolState::Initializing(_), Ok(Some(descriptor))) => {
            // A valid pointer under `initializing` means the rename happened
            // and only the ACK or the marker flip was lost. Finishing is safe;
            // rebuilding would discard a generation that may already be served.
            BootstrapPlan::FinishActivation {
                generation_id: descriptor.generation_id,
            }
        }
        // A corrupt pointer under `initializing` is the publication-uncertain
        // case. It is never masked as a clean bootstrap.
        (IndexProtocolState::Initializing(_), Err(_)) => BootstrapPlan::Unavailable {
            generation_id: "corrupt".to_string(),
        },

        // An active index with a usable pointer needs no bootstrap at all.
        (IndexProtocolState::Active(_), Ok(Some(descriptor))) => BootstrapPlan::FinishActivation {
            generation_id: descriptor.generation_id,
        },
        // `active` + missing or corrupt pointer is an explicit unavailable. An
        // automatic empty reinitialisation here would replace a real index with
        // an empty one and report success.
        (IndexProtocolState::Active(_), Ok(None)) | (IndexProtocolState::Active(_), Err(_)) => {
            BootstrapPlan::Unavailable {
                generation_id: "missing".to_string(),
            }
        }
    }
}

/// Result of comparing the physical heads against the published set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadComparison {
    /// Every physical head is exactly the published version. Nothing is
    /// restored and nothing is committed — this is the rule that stops a
    /// pointless commit before every stage.
    Matches,
    /// One or more heads moved past the published versions, so the affected
    /// table must be rewound before any delta write.
    Diverged {
        artifacts_diverged: bool,
        relations_diverged: bool,
        sections_diverged: bool,
    },
    /// A published version is gone, or the published set is not a complete
    /// triple. Explicitly unavailable; never a jump to latest.
    MissingPublishedVersion { role: &'static str },
}

/// Compare physical head versions with the published set.
///
/// `published` of `None` means nothing has been published, which matches by
/// definition: there is no baseline to diverge from.
///
/// All three heads participate unconditionally. There used to be an exemption
/// for a generation that pinned no sections version, on the reasoning that a
/// pre-sections generation had nothing to diverge from — but that exemption is
/// what let a stale sections head survive into a published generation, and an
/// absent pin now means a torn set, reported as
/// [`HeadComparison::MissingPublishedVersion`].
pub fn compare_heads(
    published: Option<&GenerationManifest>,
    artifacts_head: u64,
    relations_head: u64,
    sections_head: u64,
) -> HeadComparison {
    let Some(published) = published else {
        return HeadComparison::Matches;
    };
    if published.table_versions.artifacts == 0 || published.table_versions.relations == 0 {
        return HeadComparison::MissingPublishedVersion { role: "pair" };
    }
    if published.table_versions.sections == 0 {
        // `0` is not a real version — the empty creation commit is v1. It is
        // torn state, not "this generation has no sections": a generation with
        // no semantic sections pins the version its empty commit produced.
        return HeadComparison::MissingPublishedVersion { role: "sections" };
    }
    let artifacts_diverged = artifacts_head != published.table_versions.artifacts;
    let relations_diverged = relations_head != published.table_versions.relations;
    let sections_diverged = sections_head != published.table_versions.sections;
    if !artifacts_diverged && !relations_diverged && !sections_diverged {
        HeadComparison::Matches
    } else {
        HeadComparison::Diverged {
            artifacts_diverged,
            relations_diverged,
            sections_diverged,
        }
    }
}

/// The commit plan for one stage.
///
/// Computed purely, so the delete law and the orphan-B case are testable
/// without a database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitPlan {
    pub creates: usize,
    pub updates: usize,
    pub deletes: usize,
    pub retained: usize,
    /// Whether the stage may use `when_not_matched_by_source_delete`.
    /// That clause requires a **full** source batch; a partial scan must never
    /// reach it, because it would delete every row the scan could not see.
    pub full_source_batch: bool,
}

impl CommitPlan {
    /// Whether anything at all would be written.
    pub fn is_empty(&self) -> bool {
        self.creates == 0 && self.updates == 0 && self.deletes == 0
    }
}

/// Turn a reconcile plan into a commit plan.
///
/// `when_not_matched_by_source_delete` is permitted **only** when the capture
/// is complete. Every other delete goes through an explicit bounded predicate
/// built from confirmed absences, so an unreadable file can never widen into a
/// delete of a healthy row.
pub fn plan_commits(plan: &ReconcilePlan) -> CommitPlan {
    let mut commit = CommitPlan {
        creates: 0,
        updates: 0,
        deletes: 0,
        retained: 0,
        full_source_batch: plan.publishable,
    };
    for delta in &plan.deltas {
        match delta.action {
            DeltaAction::Create => commit.creates += 1,
            DeltaAction::Update => commit.updates += 1,
            DeltaAction::Delete => commit.deletes += 1,
            // `Retain` and `Exclude` are deliberately not deletions. `Retain`
            // is a published row this capture could not judge; `Exclude` is a
            // row that produced no current value. Neither is evidence of
            // absence.
            DeltaAction::Retain => commit.retained += 1,
            DeltaAction::Exclude => {}
            DeltaAction::Unchanged => {}
        }
    }
    commit
}

/// A publisher for one physical index, holding the owner for its whole run.
///
/// The owner guard is a field, so it cannot be dropped early by accident: it
/// is released when the `IndexPublisher` is dropped, after everything has
/// joined.
pub struct IndexPublisher {
    location: IndexLocation,
    owner: super::owner::OwnerGuard,
    tracker: Arc<StageTracker>,
    /// The tracking store handed to Lance at table creation.
    ///
    /// It is stored here, built from the caller's inner store **and the same
    /// `tracker`** above, so the wrapper passed to
    /// `NativeTable::create_empty` is the instance whose closure this
    /// publisher's `publish` reads. Two stores with two trackers would each
    /// see half the closure and `finish_writes` would report the other half
    /// untouched — the defect P2b rejected when it narrowed the wrapper's
    /// contract.
    store: Arc<super::durability::DurabilityTrackingStore>,
    platform: Arc<dyn PlatformDurability>,
}

impl std::fmt::Debug for IndexPublisher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexPublisher")
            .field("root", &self.location.root())
            .field("owner_epoch", &self.owner.epoch())
            .field("adapter", &self.platform.adapter_name())
            .finish()
    }
}

impl IndexPublisher {
    /// Acquire the physical owner and open a stage.
    ///
    /// Lock order is `actor permit → OS owner → WorkspaceLock`; this is the
    /// middle step and must never be taken while a `WorkspaceLock` is held.
    /// `WorkspaceLock` is taken per storage commit by the caller, never here
    /// and never across a model load or a queue wait.
    ///
    /// # Errors
    ///
    /// [`ConstellationError::IndexBusy`] when another live process owns the
    /// index — reported, never stolen.
    pub fn acquire(
        index_root: &Path,
        inner_store: Arc<dyn object_store::ObjectStore>,
    ) -> Result<Self, ConstellationError> {
        #[cfg(not(unix))]
        refuse_unpinned_non_unix_publisher()?;
        let location = IndexLocation::new(index_root);
        // Verify the canonical parent before anything else: the owner lock
        // lives there, and creating the root is bootstrap's job, after
        // ownership is established.
        let _ = location.existing_parent()?;

        let owner = super::owner::acquire_owner(&location, Duration::from_millis(OWNER_BUDGET_MS))?;
        // The OS guard is **already held** here — this fires after a real
        // acquisition, never instead of one. A second process reaching this
        // line after the first dies is the takeover case, and the parent
        // distinguishes the two by the pid and token in the report.
        super::fault_barrier_if_selected("owner_acquired");

        let (_support, platform, diagnostic) = platform_barrier(index_root);
        if let Some(diagnostic) = diagnostic {
            return Err(ConstellationError::UnsupportedIndexFilesystem {
                detail: diagnostic.detail,
            });
        }
        let platform = platform.ok_or_else(|| ConstellationError::UnsupportedIndexFilesystem {
            detail: "the index root has no write barrier".to_string(),
        })?;

        let tracker = Arc::new(StageTracker::default());
        tracker.begin_stage(owner.epoch());
        // One store, one tracker: built here from the caller's inner store and
        // the tracker above, so the two cannot drift apart.
        let store = super::storage::tracking_pair_with_pair(inner_store, Arc::clone(&tracker));

        Ok(Self {
            location,
            owner,
            tracker,
            store,
            platform,
        })
    }

    /// The tracking store to hand to Lance at table creation.
    ///
    /// Named for what it is: a store, not a tracker. It shares this
    /// publisher's [`Self::tracker`], which is the property that makes one
    /// closure barrier cover both tables.
    pub fn tracking_store(&self) -> &Arc<super::durability::DurabilityTrackingStore> {
        &self.store
    }

    /// The owner epoch, for the descriptor.
    pub fn owner_epoch(&self) -> u64 {
        self.owner.epoch()
    }

    /// The shared tracker. Both tables' wrappers write into this one.
    pub fn tracker(&self) -> &Arc<StageTracker> {
        &self.tracker
    }

    // `barrier()` was deleted. It was added so a caller could reach the
    // adapter for diagnostics, but the publisher uses `self.platform.as_ref()`
    // directly at every barrier site, so nothing ever called it — and when it
    // was narrowed to `pub(crate)` it lost its last reader. The adapter is
    // still reachable where it is actually used, at the call that performs
    // the barrier.

    pub fn index_root(&self) -> &Path {
        self.location.root()
    }

    /// Republish `CURRENT.json` under this publisher's durability barrier.
    ///
    /// **Only the claimed embedding space may move this way.** It used to also
    /// carry the embedding heads, which is how `CURRENT` became a second,
    /// weaker source of table versions: the descriptor could name a sections
    /// head the immutable manifest never pinned, and `effective_sections_version`
    /// had to reconcile two authorities. A drain now publishes a **new
    /// generation** — it already did for its pending set — and the descriptor
    /// carries nothing but the checksum binding to that manifest.
    ///
    /// # This runs the same barrier `publish` runs
    ///
    /// The order is the module's ordering invariant, not a convenience:
    ///
    /// ```text
    /// finish_writes → sync_written_set → CURRENT replacement
    /// ```
    ///
    /// Before this, the republish reached straight for
    /// [`storage::publish_current`], so the drain's Lance writes were tracked
    /// but never fsynced before the pointer moved. A crash there left a visible
    /// `CURRENT` whose directory entry the tables had not yet acknowledged —
    /// the C04 hole.
    ///
    /// The barrier runs over **this publisher's** [`Self::tracker`], the same
    /// instance all three tables' wrappers write into, so it sees the drain's
    /// whole closure. A second tracker here would see an empty set and ACK
    /// nothing.
    ///
    /// Safe to call only while this publisher is held — the write assumes no
    /// other writer can move the pointer between the caller's read and this
    /// republish, which is exactly what the owner lock guarantees.
    ///
    /// # Errors
    ///
    /// [`ConstellationError::DurabilityUnavailable`] when the closure barrier
    /// fails. Nothing is visible yet, so the previously published `CURRENT`
    /// is left exactly as it was and the tracked set stays on record.
    /// [`ConstellationError::PublicationUncertain`] when the pointer became
    /// visible and its directory entry was not acknowledged — from
    /// [`storage::publish_current`], unchanged. Neither a rollback nor a
    /// success.
    pub(crate) fn republish_current(
        &self,
        descriptor: &super::manifest::CurrentDescriptor,
    ) -> Result<(), ConstellationError> {
        let index_root = self.index_root();

        // 1. Closure barrier over the shared tracker. `finish_writes` refuses
        //    while a write is in flight, a destination is unresolved, or the
        //    stage is poisoned, so a barrier can only run over a joined
        //    closure.
        let write_set = self.tracker.finish_writes()?;
        self.tracker
            .sync_written_set(&write_set, index_root, self.platform.as_ref())?;

        // 2. Visibility point, then the durable ACK — both inside
        //    `publish_current`, which is what makes a failure after the
        //    rename `PublicationUncertain` rather than a plain error.
        storage::publish_current(index_root, descriptor, self.platform.as_ref())?;
        super::fault_barrier_if_selected("after_current_ack");
        self.tracker.begin_stage(self.owner.epoch());
        Ok(())
    }

    /// Write the bootstrap marker announcing an in-progress build.
    ///
    /// Called **before** any new active table exists, so a crash during table
    /// creation is recognisable as "bootstrap was interrupted" rather than
    /// "this index never existed".
    pub fn announce_bootstrap(
        &self,
        checkout_id: &str,
        index_epoch: u64,
        attempt: u32,
    ) -> Result<ProtocolMarker, ConstellationError> {
        let marker = ProtocolMarker::initializing(index_epoch, checkout_id, attempt);
        storage::write_marker(self.index_root(), &marker, self.platform.as_ref())?;
        Ok(marker)
    }

    /// Promote the marker to `active` once the generation is published.
    pub fn mark_active(&self, marker: &mut ProtocolMarker) -> Result<(), ConstellationError> {
        marker.state = BootstrapState::Active;
        // `bootstrap_attempt` is a `u32`; `checked_increment` is the `u64` counter.
        // Overflow still refuses to wrap, via `CheckedCounter::u32_inc`.
        marker.bootstrap_attempt =
            super::CheckedCounter::u32_inc("bootstrap_attempt", marker.bootstrap_attempt)?;
        storage::write_marker(self.index_root(), marker, self.platform.as_ref())
    }

    /// Run the closure barrier and publish one generation.
    ///
    /// The order is fixed by the ordering invariant at the top of this file and
    /// is not negotiable: `finish_writes` may only run once every Lance commit,
    /// restore, migration and metadata operation has joined, and it refuses
    /// otherwise. The closure barrier precedes the manifest, which precedes the
    /// pointer, which precedes the ACK.
    ///
    /// # `profile_id` is an input, never an inference
    ///
    /// The descriptor's claimed space is whatever this argument says. It was
    /// previously read off `manifest.pending_jobs`' first value, which made
    /// the space a function of whichever job happened to sort first — and of
    /// nothing at all once `pending_jobs` was empty, which is the state
    /// production actually publishes. A reconcile that dropped the profile
    /// would unpublish the space pinned semantic reads query in.
    ///
    /// So each caller states it: `None` on a first bootstrap (no space has
    /// been claimed yet), the currently published descriptor's profile on an
    /// ordinary reconcile (carry it forward), and the verified profile on an
    /// admission/completion that establishes one.
    ///
    /// # Errors
    ///
    /// [`ConstellationError::DurabilityUnavailable`] when the closure barrier
    /// fails — publication is not reached at all.
    /// [`ConstellationError::PublicationUncertain`] when the pointer became
    /// visible and the ACK did not follow. The caller must treat the
    /// generation as neither confirmed nor rolled back.
    pub fn publish(
        &self,
        manifest: &GenerationManifest,
        profile_id: Option<&str>,
    ) -> Result<CurrentDescriptor, ConstellationError> {
        let index_root = self.index_root();

        // 1. Closure barrier, over the whole new storage closure.
        let write_set = self.tracker.finish_writes()?;
        self.tracker
            .sync_written_set(&write_set, index_root, self.platform.as_ref())?;
        // All three writes joined and the full WriteSet barrier returned
        // success. Not merely `finish_writes`, and not a partial set.
        super::fault_barrier_if_selected("after_storage_sync");

        // 2. Generation manifest barrier. A durable manifest alone is an
        //    orphan: readers ignore it until CURRENT names it.
        let manifest_sha256 =
            storage::write_generation_manifest(index_root, manifest, self.platform.as_ref())?;
        // The manifest is installed and its directory durability returned;
        // `CURRENT` still names the previous generation, so this file is an
        // orphan a reader is right to ignore.
        super::fault_barrier_if_selected("after_manifest_sync");

        // 3 + 4. Visibility point, then the durable ACK. The profile is placed
        //      on the descriptor **before** the single `publish_current`: an
        //      intermediate pointer without it would be a generation whose
        //      semantic reads have no space to query.
        let descriptor =
            storage::descriptor_for(manifest, self.owner.epoch(), &manifest_sha256, profile_id);
        storage::publish_current(index_root, &descriptor, self.platform.as_ref())?;
        // The pointer is visible, the platform durability ACK returned, and the
        // descriptor's checksum and triple have been re-read below by the
        // caller. Nothing is left unproven at this point.
        super::fault_barrier_if_selected("after_current_ack");
        self.tracker.begin_stage(self.owner.epoch());

        Ok(descriptor)
    }
}

/// Identity of the workspace the publisher is building for.
///
/// `pub(crate)`, not `pub`, and deliberately so.
///
/// `IndexIdentity` is `pub(crate)` in the private `owner` module, so a `pub`
/// function here would return a type callers outside the crate cannot name —
/// the "private type in public interface" shape, which widens nothing and
/// promises something unreachable. The only consumer is the bootstrap wiring
/// inside this crate, which lives in `facade`. Narrowing the function to match
/// the type is the fix that adds no public surface at all.
///
/// If a future surface genuinely needs an identity outside the crate, the
/// right change is to make `IndexIdentity` public with real accessors — not
/// to invent a string-returning shim to make some call site compile.
pub(crate) fn identity_for(index_root: &Path) -> IndexIdentity {
    IndexIdentity::derive(&IndexLocation::new(index_root))
}

/// The first published `indexEpoch`.
///
/// Published epochs start at `1`; `0` is reserved by the owner module for
/// "never published". The first publish therefore expects the same epoch the
/// marker announced, not a fictional version 0.
pub fn first_index_epoch() -> Result<u64, ConstellationError> {
    checked_increment("index_epoch", 0)
}

/// Build the first generation manifest from a complete capture.
///
/// # Errors
///
/// [`ConstellationError::SourceBusy`] when the capture is incomplete. A
/// partial scan must never publish a generation — not complete, and not empty.
/// Publishing an empty first generation would look identical, to every
/// reader, to "this workspace has no artifacts".
pub fn first_generation(
    captured: &CapturedSourceView,
    generation_id: &str,
    table_versions: TableVersions,
) -> Result<GenerationManifest, ConstellationError> {
    if !captured.is_complete() {
        return Err(ConstellationError::SourceBusy {
            attempts: super::capture::DEFAULT_CAPTURE_ATTEMPTS,
            elapsed_ms: 0,
        });
    }
    let manifest = GenerationManifest::from_capture(captured, generation_id, table_versions);
    manifest.validate_for_publication()?;
    Ok(manifest)
}

/// Reject a candidate whose base has moved under it.
///
/// Compared immediately before the pointer switch, in the same owner →
/// workspace order as the staging. A candidate computed against a different
/// published generation, a different epoch, or different source bytes is
/// **discarded**, never published: publishing it would make the visible
/// generation describe a source set that no longer exists.
#[allow(clippy::too_many_arguments)]
pub fn candidate_is_current(
    base_generation: Option<&str>,
    candidate_base: Option<&str>,
    base_epoch: u64,
    candidate_epoch: u64,
    base_source_set_digest: &str,
    candidate_source_set_digest: &str,
    base_config_digest: &str,
    candidate_config_digest: &str,
) -> Result<(), ConstellationError> {
    if base_generation != candidate_base {
        return Err(ConstellationError::SourceBusy {
            attempts: 1,
            elapsed_ms: 0,
        });
    }
    if base_epoch != candidate_epoch || candidate_epoch == 0 {
        return Err(ConstellationError::IndexProtocolMismatch {
            detail: format!(
                "candidate epoch {candidate_epoch} does not match the published epoch {base_epoch}"
            ),
        });
    }
    if base_source_set_digest != candidate_source_set_digest
        || base_config_digest != candidate_config_digest
    {
        return Err(ConstellationError::SourceBusy {
            attempts: 1,
            elapsed_ms: 0,
        });
    }
    Ok(())
}

/// Where an index's legacy tables stay once bootstrap has run.
///
/// They are renamed to `*.legacy-backup` and never reopened as writers. A
/// second live path is how two protocols end up writing the same logical
/// index, and vectors without verified input/profile bindings are not current
/// by definition.
pub fn legacy_backup_name(table_name: &str) -> String {
    // Backups only ever apply to the pre-constellation names. If a caller
    // hands this an active table, strip the `constellation_` prefix so the
    // backup name can never sit in the active-table namespace and be
    // mistaken for a live `constellation_*` dataset.
    let stem = table_name
        .strip_prefix("constellation_")
        .unwrap_or(table_name);
    format!("{stem}.legacy-backup")
}

/// Whether a directory entry is one of the legacy tables awaiting backup.
pub fn is_legacy_table_name(table_name: &str) -> bool {
    crate::constellation::manifest::LEGACY_TABLES.contains(&table_name)
}

/// Absolute path of an active table's dataset directory.
pub fn active_dataset_path(index_root: &Path, table_name: &str) -> PathBuf {
    index_root.join(format!("{table_name}.lance"))
}

/// Timestamp helper kept next to its only users, so a generation id and its
/// manifest cannot disagree about when they were made.
pub fn generation_id_for(evaluated_at: &str) -> String {
    // RFC3339 carries `:` — an illegal NTFS name character — and the id
    // becomes `generations/<id>.json` verbatim. Mapping to `-` keeps the
    // timestamp readable and the path legal on every filesystem the index
    // may live on.
    let sanitized: String = evaluated_at
        .chars()
        .map(|c| match c {
            ':' | '<' | '>' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => c,
        })
        .collect();
    format!("gen-{sanitized}")
}

/// Current instant, named here so the publisher's timestamps are one decision.
pub fn now() -> String {
    rfc3339_now()
}

/// A diagnostic for an index whose pointer cannot be resolved.
pub fn unavailable_detail(detail: impl Into<String>) -> (DiagnosticCode, String) {
    (DiagnosticCode::IndexGenerationUnavailable, detail.into())
}

/// How far a bootstrap or recovery pass got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapProgress {
    /// Both active tables were created by this run and the first generation
    /// was published.
    CreatedTables { generation_id: String },
    /// The tables already existed from an interrupted attempt. They were
    /// adopted and the baseline was rebuilt from a **full** canonical capture;
    /// no partial candidate was resumed.
    RestartedExistingTables { generation_id: String },
    /// A generation was already published and only the marker flip was lost.
    FinishedActivation { generation_id: String },
}

/// Advance one index from whatever a restart found to `Active`.
///
/// `plan_bootstrap` already encodes the restart laws; this **drives** them
/// and does not re-decide them. The four outcomes it distinguishes:
///
/// | Found | This does |
/// |---|---|
/// | `Uninitialized` | legacy backup → marker `initializing` → create tables → full capture → first publish → marker `active` |
/// | `initializing` + no CURRENT | same rebuild path; the interrupted attempt's partial state is ignored, never adopted |
/// | `initializing` + valid CURRENT | validate manifest checksum and versions → marker `active` |
/// | `initializing` + corrupt CURRENT | `publication_uncertain` — never masked as a clean bootstrap |
/// | `active` + missing/corrupt pointer | `index_generation_unavailable` — never an automatic empty re-init |
///
/// Ordering that is not negotiable: the marker is written **before** any
/// active table exists, and `active` is written **only after** the
/// publication ACK. A crash between the two leaves a recognisable
/// "interrupted bootstrap" rather than an index that half-exists.
///
/// # Errors
///
/// [`ConstellationError::IndexGenerationUnavailable`] when a pointer exists
/// but does not resolve. [`ConstellationError::SourceBusy`] when the capture
/// is incomplete, so a first generation is never published complete *or*
/// empty — an empty first generation is indistinguishable, to every reader,
/// from a workspace with no artifacts.
pub async fn bootstrap_or_recover(
    publisher: &IndexPublisher,
    workspace: &Path,
    capture_limits: super::capture::CaptureLimits,
) -> Result<BootstrapProgress, ConstellationError> {
    bootstrap_or_recover_inner(publisher, workspace, capture_limits, None).await
}

/// Resume an existing published Constellation index for one pending repair.
/// Never bootstraps a legacy or uninitialized index as a substitute.
pub(crate) async fn bootstrap_or_recover_for_repair(
    publisher: &IndexPublisher,
    workspace: &Path,
    capture_limits: super::capture::CaptureLimits,
    admission: &crate::workspace::lock::RepairIndexAdmission<'_>,
) -> Result<BootstrapProgress, ConstellationError> {
    bootstrap_or_recover_inner(publisher, workspace, capture_limits, Some(admission)).await
}

async fn bootstrap_or_recover_inner(
    publisher: &IndexPublisher,
    workspace: &Path,
    capture_limits: super::capture::CaptureLimits,
    admission: Option<&crate::workspace::lock::RepairIndexAdmission<'_>>,
) -> Result<BootstrapProgress, ConstellationError> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_publisher()?;
    if let Some(admission) = admission
        && (!std::ptr::eq(admission.owner(), publisher) || admission.workspace() != workspace)
    {
        return Err(ConstellationError::IndexProtocolMismatch {
            detail: "repair admission is bound to another owner or workspace".to_string(),
        });
    }
    let index_root = publisher.index_root().to_path_buf();
    let identity = identity_for(&index_root);

    let marker_state = super::storage::read_marker(&index_root)?;
    let current_state = super::storage::read_current(&index_root);
    let plan = plan_bootstrap(&marker_state, current_state);

    // A marker that exists but names another checkout is refused before
    // anything is created or read as ours. The comparison is against the
    // path-bound id, not the adopted identity — the adopted one IS the
    // marker's own `checkout_id`, and comparing them can never disagree.
    if let Some(marker) = marker_state.marker() {
        let expected =
            IndexIdentity::path_bound_checkout_id(&super::owner::IndexLocation::new(&index_root));
        marker.check_identity(&expected)?;
    }

    if let BootstrapPlan::FinishActivation { generation_id } = &plan {
        // The generation is already published; only the marker flip was lost.
        // Validate it before declaring the index active, so a pointer that
        // does not resolve cannot be papered over by a marker flip.
        let descriptor = super::storage::read_current(&index_root)?.ok_or_else(|| {
            ConstellationError::IndexGenerationUnavailable {
                detail: format!("generation {generation_id} vanished during activation"),
            }
        })?;
        let manifest = super::storage::read_manifest(&index_root, &descriptor)?;
        if !manifest.table_versions.is_publishable() {
            return Err(ConstellationError::IndexGenerationUnavailable {
                detail: format!(
                    "generation {generation_id} names table versions {:?}",
                    manifest.table_versions
                ),
            });
        }
        let workspace_guard = match admission {
            Some(admission) => crate::workspace::lock::acquire_repair_workspace_lock(admission).await,
            None => crate::workspace::acquire_workspace_lock(workspace).await,
        }.map_err(|error| ConstellationError::Io {
            detail: format!("cannot take the workspace lock to finish legacy archival: {error}"),
        })?;
        backup_legacy_tables(&index_root, || publisher.platform.sync_dir(&index_root))?;
        let mut marker =
            ProtocolMarker::initializing(identity.index_epoch(), identity.checkout_id(), 1);
        publisher.mark_active(&mut marker)?;
        drop(workspace_guard);
        return Ok(BootstrapProgress::FinishedActivation {
            generation_id: generation_id.clone(),
        });
    }

    if matches!(plan, BootstrapPlan::Unavailable { .. }) {
        // An active index whose pointer is missing or corrupt, or an
        // initializing one with a corrupt pointer. Neither is bootstrapped:
        // an automatic empty reinitialisation would replace real data and
        // report success.
        return Err(ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "{:?} — this index is not bootstrappable; it is never reinitialised as empty",
                plan
            ),
        });
    }
    if admission.is_some() {
        return Err(ConstellationError::IndexGenerationUnavailable {
            detail: "identity recovery requires an existing published Constellation index; legacy and uninitialized indexes remain pending".to_string(),
        });
    }

    // Serialize the ownership handoff with canonical writers. A mutator that
    // waited for this guard must observe the initializing marker before it
    // decides whether the legacy projection is still writable.
    let workspace_guard = crate::workspace::acquire_workspace_lock(workspace)
        .await
        .map_err(|error| ConstellationError::Io {
            detail: format!("cannot take the workspace lock for migration: {error}"),
        })?;
    // Validate the legacy plane before changing ownership. Announce durable
    // Constellation ownership before moving even the first legacy path, so a
    // crash cannot turn a partially archived v5 index into a v4 fallback.
    validate_legacy_archive(&index_root)?;
    let mut marker =
        publisher.announce_bootstrap(identity.checkout_id(), first_index_epoch()?, 1)?;
    backup_legacy_tables(&index_root, || publisher.platform.sync_dir(&index_root))?;
    drop(workspace_guard);

    // CURRENT is absent and the physical owner is held. An interrupted
    // bootstrap may have created any prefix of the three datasets.
    let (tables, created) = super::lance_tables::resume_bootstrap_tables(
        &index_root,
        &marker,
        std::sync::Arc::clone(publisher.tracking_store()),
        workspace,
    )
    .await?;

    // A complete capture, under the workspace lock, released before any write.
    //
    // No identity baseline, and that is not an oversight: this branch runs
    // only when no generation has ever been published (`FinishActivation`
    // returned above), so there is no published row to continue from. Every
    // artifact is genuinely observed for the first time, which is exactly
    // what `incarnation = 1` says.
    let captured = capture_under_workspace_lock(workspace, capture_limits, None).await?;

    // Reject incomplete captures before replacing any previously staged rows.
    let generation_id = generation_id_for(&captured.evaluated_at);
    first_generation(&captured, &generation_id, tables.head_versions().await?)?;
    let artifacts = artifacts_batch(
        &captured.artifacts,
        captured.index_epoch,
        &captured.checkout_id,
        &BTreeMap::new(),
    )?;
    let relations = relations_batch(&captured.artifacts)?;
    let sections = sections_batch(&captured, &BTreeMap::new())?;

    // An empty merge is explicitly rejected by execute_merge, so clear that
    // unpublished dataset instead. All three replacements run under the same
    // owner and workspace barriers as normal staging.
    let guard = crate::workspace::acquire_workspace_lock(workspace)
        .await
        .map_err(|error| ConstellationError::Io {
            detail: format!("cannot take the workspace lock for bootstrap staging: {error}"),
        })?;
    if artifacts.num_rows() == 0 {
        tables.clear_empty_bootstrap_dataset("artifacts").await?;
    } else {
        tables.merge_into_artifacts(Arc::clone(&artifacts), true).await?;
    }
    if relations.num_rows() == 0 {
        tables.clear_empty_bootstrap_dataset("relations").await?;
    } else {
        tables.merge_into_relations(relations, true).await?;
    }
    if sections.num_rows() == 0 {
        tables.clear_empty_bootstrap_dataset("sections").await?;
    } else {
        tables.merge_into_sections(Arc::clone(&sections), true).await?;
    }
    drop(guard);

    // Versions are read AFTER the writes, so the manifest names the versions
    // a pinned reader will actually open.
    let versions = tables.head_versions().await?;
    let expected = captured.entries.iter()
        .map(|entry| (entry.canonical_key.clone(), entry.source_digest.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut section_digests = BTreeMap::new();
    for section in &captured.sections {
        let digest = expected.get(&section.parent_key).ok_or_else(|| ConstellationError::Io {
            detail: format!("captured section {} has no parent entry", section.canonical_key),
        })?;
        section_digests.insert(section.canonical_key.clone(), digest.clone());
    }
    let section_expectation = SectionsExpectation {
        version: versions.sections,
        digests: section_digests,
    };
    let expected_edges = captured.artifacts.iter().map(|artifact| artifact.outgoing.len()).sum();
    verify_staged(
        &index_root, &marker, versions.artifacts, &expected, versions.relations,
        expected_edges, &captured, &section_expectation,
    ).await.map_err(|error| ConstellationError::Io {
        detail: format!("bootstrap post-commit verification failed: {error}"),
    })?;
    verify_bootstrap_payload(&index_root, &marker, "artifacts", versions.artifacts, &artifacts).await?;
    verify_bootstrap_payload(&index_root, &marker, "sections", versions.sections, &sections).await?;

    // `first_generation` refuses an incomplete capture through
    // `GenerationManifest::validate_for_publication`, so completeness is not
    // re-decided here.
    let manifest = first_generation(&captured, &generation_id, versions)?;

    // `publish` runs finish_writes → sync_written_set → write_generation_manifest
    // → descriptor_for → publish_current, with the real owner epoch from the
    // guard. Only after that ACK is the marker flipped.
    //
    // `None`: a first generation claims no embedding space. Nothing can be
    // embedded before a generation is published — a drain writes onto rows —
    // so a profile here would be a claim no row backs.
    publish_verified_bootstrap(publisher, workspace, capture_limits, &captured, &manifest).await?;
    drop(tables);
    publisher.mark_active(&mut marker)?;

    Ok(if created {
        BootstrapProgress::CreatedTables {
            generation_id: generation_id.clone(),
        }
    } else {
        BootstrapProgress::RestartedExistingTables {
            generation_id: generation_id.clone(),
        }
    })
}

/// Recheck the canonical source while holding the same guard through CURRENT.
async fn publish_verified_bootstrap(
    publisher: &IndexPublisher,
    workspace: &Path,
    limits: super::capture::CaptureLimits,
    staged: &CapturedSourceView,
    manifest: &GenerationManifest,
) -> Result<(), ConstellationError> {
    let guard = crate::workspace::acquire_workspace_lock(workspace)
        .await
        .map_err(|error| ConstellationError::Io {
            detail: format!("cannot take the workspace lock for bootstrap publication: {error}"),
        })?;
    let current = capture_while_locked(workspace, limits, None).await?;
    if !current.is_complete() || current.checkout_id != staged.checkout_id {
        return Err(ConstellationError::SourceBusy { attempts: 1, elapsed_ms: 0 });
    }
    candidate_is_current(
        None, None, current.index_epoch, staged.index_epoch,
        &current.source_set_digest, &staged.source_set_digest,
        &current.config_digest, &staged.config_digest,
    )?;
    publisher.publish(manifest, None)?;
    drop(guard);
    Ok(())
}

/// Compare every column of an unpublished bootstrap batch against its pinned
/// reader. This is separate from normal reconciliation, where semantic columns
/// can legitimately be preserved from an earlier published generation.
async fn verify_bootstrap_payload(
    index_root: &Path,
    marker: &ProtocolMarker,
    role: &str,
    version: u64,
    expected: &arrow_array::RecordBatch,
) -> Result<(), ConstellationError> {
    verify_pinned_payload(index_root, marker, role, version, expected, true).await
}

async fn verify_pinned_payload(
    index_root: &Path,
    marker: &ProtocolMarker,
    role: &str,
    version: u64,
    expected: &arrow_array::RecordBatch,
    strict_bootstrap: bool,
) -> Result<(), ConstellationError> {
    let reader = super::lance_tables::open_table_at_version(index_root, marker, role, version).await?;
    if reader.version().await? != version {
        return Err(ConstellationError::Io {
            detail: format!("{role} payload reader returned a different version"),
        });
    }
    let schema = reader.schema().await?;
    let batches = reader.query_batches().await?;
    verify_payload_batches(role, expected, &schema, &batches, strict_bootstrap, true)
}

fn verify_payload_batches(
    role: &str,
    expected: &arrow_array::RecordBatch,
    schema: &arrow_schema::Schema,
    batches: &[arrow_array::RecordBatch],
    strict_bootstrap: bool,
    compare_source_diagnostics: bool,
) -> Result<(), ConstellationError> {
    use arrow_array::{Array, StringArray};

    let error = |detail: String| ConstellationError::Io {
        detail: format!("{role} payload verification failed: {detail}"),
    };
    let keys = expected.column_by_name("canonical_key")
        .and_then(|column| column.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| error("expected canonical_key is missing or not UTF-8".into()))?;
    let mut expected_rows = BTreeMap::new();
    for row in 0..expected.num_rows() {
        if keys.is_null(row) || expected_rows.insert(keys.value(row), row).is_some() {
            return Err(error("expected canonical keys are null or duplicated".into()));
        }
    }
    let expected_schema = expected.schema();
    if schema.fields().len() != expected_schema.fields().len()
        || schema.fields().iter().zip(expected_schema.fields()).any(|(actual, wanted)| {
            actual.name() != wanted.name() || actual.data_type() != wanted.data_type()
        })
    {
        return Err(error("pinned schema differs from the expected schema".into()));
    }
    let mut seen = BTreeSet::new();
    for batch in batches {
        let found_keys = batch.column_by_name("canonical_key")
            .and_then(|column| column.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| error("pinned canonical_key is missing or not UTF-8".into()))?;
        for row in 0..batch.num_rows() {
            if found_keys.is_null(row) {
                return Err(error("pinned canonical_key is null".into()));
            }
            let key = found_keys.value(row);
            let wanted_row = *expected_rows.get(key)
                .ok_or_else(|| error(format!("unexpected row {key}")))?;
            if !seen.insert(key.to_owned()) {
                return Err(error(format!("duplicate row {key}")));
            }
            for (index, field) in expected.schema().fields().iter().enumerate() {
                // Reconciliation may preserve these non-source values from
                // the published baseline. Bootstrap verifies even these.
                if !strict_bootstrap && matches!(
                    field.name().as_str(),
                    "embedding" | "profile_id" | "semantic_state" | "retry_after"
                        | "body_hash" | "r_eff_score" | "tags"
                ) {
                    continue;
                }
                // This column is also overwritten by model failure and may
                // survive a later admission. Only a merge that rewrote the
                // source row can assert the parser-derived value.
                if !strict_bootstrap && !compare_source_diagnostics
                    && field.name() == "diagnostics"
                {
                    continue;
                }
                let found = batch.column_by_name(field.name())
                    .ok_or_else(|| error(format!("missing column {}", field.name())))?;
                let wanted = expected.column(index);
                if found.data_type() != wanted.data_type()
                    || found.to_data().slice(row, 1) != wanted.to_data().slice(wanted_row, 1)
                {
                    return Err(error(format!("row {key} differs in column {}", field.name())));
                }
            }
        }
    }
    if seen.len() != expected_rows.len() {
        return Err(error("one or more expected rows are absent".into()));
    }
    Ok(())
}

/// The legacy derived plane. The v5 marker is deliberately last.
fn legacy_archive_paths(index_root: &Path) -> Vec<(PathBuf, PathBuf, bool)> {
    let tables = crate::constellation::manifest::LEGACY_TABLES.iter().copied().chain([
        "evidence",
        crate::db::migrate::IDENTITY_V5_ARTIFACTS_TABLE,
        crate::db::migrate::IDENTITY_V5_RELATIONS_TABLE,
        crate::db::migrate::IDENTITY_V5_EVIDENCE_TABLE,
    ]);
    let mut paths: Vec<_> = tables.map(|name| (
        index_root.join(format!("{name}.lance")),
        index_root.join(format!("{}.lance", legacy_backup_name(name))),
        true,
    )).collect();
    paths.push((
        index_root.join(crate::db::migrate::IDENTITY_V5_MARKER_FILE),
        index_root.join("IDENTITY_V5.json.legacy-backup"),
        false,
    ));
    paths
}

fn legacy_path_exists(path: &Path, directory: bool) -> Result<bool, ConstellationError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if !meta.file_type().is_symlink()
            && if directory { meta.is_dir() } else { meta.is_file() } => Ok(true),
        Ok(_) => Err(ConstellationError::IndexProtocolMismatch {
            detail: format!("unsafe legacy archive path {}", path.display()),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(ConstellationError::Io {
            detail: format!("cannot inspect {}: {error}", path.display()),
        }),
    }
}

fn validate_legacy_archive(index_root: &Path) -> Result<(), ConstellationError> {
    let paths = legacy_archive_paths(index_root);
    for (source, destination, directory) in &paths {
        let live = legacy_path_exists(source, *directory)?;
        let archived = legacy_path_exists(destination, *directory)?;
        if live && archived {
            return Err(ConstellationError::IndexProtocolMismatch {
                detail: format!("both legacy source {} and backup {} exist", source.display(), destination.display()),
            });
        }
    }
    let (marker, backup, _) = paths.last().expect("the marker is always present in the path list");
    let marker_path = if legacy_path_exists(marker, false)? {
        Some(marker)
    } else if legacy_path_exists(backup, false)? {
        Some(backup)
    } else {
        None
    };
    if let Some(marker_path) = marker_path {
        let bytes = std::fs::read(marker_path).map_err(|error| ConstellationError::Io {
            detail: format!("cannot read {}: {error}", marker_path.display()),
        })?;
        let marker: crate::db::migrate::IdentityV5Marker = serde_json::from_slice(&bytes)
            .map_err(|error| ConstellationError::IndexProtocolMismatch {
                detail: format!("invalid identity v5 marker {}: {error}", marker_path.display()),
            })?;
        if marker.version != crate::db::migrate::IDENTITY_V5_VERSION || !marker.completed
            || marker.artifacts_table != crate::db::migrate::IDENTITY_V5_ARTIFACTS_TABLE
            || marker.relations_table != crate::db::migrate::IDENTITY_V5_RELATIONS_TABLE
            || marker.evidence_table != crate::db::migrate::IDENTITY_V5_EVIDENCE_TABLE
            || marker.source_digest.len() != 64
            || !marker.source_digest.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ConstellationError::IndexProtocolMismatch {
                detail: format!("invalid identity v5 marker {}", marker_path.display()),
            });
        }
        for (source, destination, _) in &paths[paths.len() - 4..paths.len() - 1] {
            if !legacy_path_exists(source, true)? && !legacy_path_exists(destination, true)? {
                return Err(ConstellationError::IndexProtocolMismatch {
                    detail: format!("identity v5 marker names missing table {}", source.display()),
                });
            }
        }
    }
    Ok(())
}

/// Archive without replacing a previous backup. Each rename is durable
/// before the next, and the identity marker moves only after its three tables.
fn backup_legacy_tables(
    index_root: &Path,
    mut sync: impl FnMut() -> Result<(), ConstellationError>,
) -> Result<(), ConstellationError> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_publisher()?;
    validate_legacy_archive(index_root)?;
    for (source, destination, directory) in legacy_archive_paths(index_root) {
        if !legacy_path_exists(&source, directory)? {
            continue;
        }
        rename_legacy_no_replace(&source, &destination).map_err(|error| ConstellationError::Io {
            detail: format!("cannot archive {} as {}: {error}", source.display(), destination.display()),
        })?;
        sync()?;
    }
    // A previous rename may have succeeded before its directory sync failed.
    sync()?;
    Ok(())
}

#[cfg(not(unix))]
fn refuse_unpinned_non_unix_publisher() -> Result<(), ConstellationError> {
    Err(ConstellationError::UnsupportedIndexFilesystem {
        detail: "index publication is unavailable on non-Unix platforms until all namespace writes use pinned, no-reparse handles with durable barriers".to_string(),
    })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn rename_legacy_no_replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let source = std::ffi::CString::new(source.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "NUL in source path"))?;
    let destination = std::ffi::CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "NUL in destination path"))?;
    // Both pointers are valid NUL-terminated paths for the duration of the
    // syscall. The exclusive flag makes an occupied destination an error.
    #[cfg(target_os = "macos")]
    let result = unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(libc::AT_FDCWD, source.as_ptr(), libc::AT_FDCWD, destination.as_ptr(), libc::RENAME_NOREPLACE)
    };
    if result == 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn rename_legacy_no_replace(_source: &Path, _destination: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "atomic no-replace legacy archive is unavailable on this platform",
    ))
}

/// Capture under the workspace lock, then **release** it before any write.
///
/// The guard is bound to a named variable so its scope is the read, and it is
/// dropped explicitly so nothing downstream can inherit it.
///
/// # `previous` — the identity baseline is not optional here
///
/// Every production call site passes the manifest `CURRENT` names. A capture
/// with no baseline re-observes the corpus from nothing: `incarnation` and
/// `row_revision` come back as `1` on every pass, so
///
/// * a delete followed by a recreate with **byte-identical** content at the
///   same path reproduces the deleted row's `(incarnation, row_revision)`
///   exactly, and `SemanticColumns::is_preserved_for` — which compares
///   exactly those fields plus the input digest — preserves the deleted
///   document's vector onto its replacement; and
/// * an outstanding `EmbeddingJob`, whose binding carries the same tuple,
///   still matches the recreated row, so a stale completion is applied rather
///   than reported stale.
///
/// Only `bootstrap_or_recover` passes `None`, and that is correct there: it
/// runs before any generation has ever been published, so there is no baseline
/// to carry and nothing that could have inherited from one.
pub(crate) async fn capture_under_workspace_lock(
    workspace: &Path,
    limits: super::capture::CaptureLimits,
    previous: Option<std::sync::Arc<super::manifest::GenerationManifest>>,
) -> Result<std::sync::Arc<super::capture::CapturedSourceView>, ConstellationError> {
    let guard = crate::workspace::acquire_workspace_lock(workspace)
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot take the workspace lock for capture: {e}"),
        })?;
    let captured = capture_while_locked(workspace, limits, previous).await?;
    drop(guard);
    Ok(captured)
}

/// Capture under the narrow, owner-bound lock for one pending repair.
pub(crate) async fn capture_under_repair_lock(
    workspace: &Path,
    limits: super::capture::CaptureLimits,
    previous: Option<std::sync::Arc<super::manifest::GenerationManifest>>,
    admission: &crate::workspace::lock::RepairIndexAdmission<'_>,
) -> Result<std::sync::Arc<super::capture::CapturedSourceView>, ConstellationError> {
    if workspace != admission.workspace() {
        return Err(ConstellationError::IndexProtocolMismatch {
            detail: "repair capture workspace differs from its admission".to_string(),
        });
    }
    let guard = crate::workspace::lock::acquire_repair_workspace_lock(admission)
        .await
        .map_err(|error| ConstellationError::Io {
            detail: format!("cannot take the identity recovery workspace lock: {error}"),
        })?;
    let captured = capture_while_locked(workspace, limits, previous).await?;
    drop(guard);
    Ok(captured)
}

/// The capture half of [`capture_under_workspace_lock`], for a caller that
/// **already holds** the workspace lock.
///
/// This exists because "take the lock, look at the source, then decide" cannot
/// be built out of a helper that takes and releases the lock itself. When
/// [`crate::constellation::facade::maintain_index`] validated the source and
/// then published through a second lock acquisition, anything that changed the
/// workspace in between was not observed by either — the pointer could move to
/// a generation describing bytes that no longer exist. One lock, held across
/// the recheck **and** the pointer switch, closes that window; the lock is not
/// reentrant, so nesting two acquisitions would deadlock rather than widen it.
///
/// Nothing else moves under this lock: the caller does its planning and
/// publication with no other workspace I/O in between.
pub(crate) async fn capture_while_locked(
    workspace: &Path,
    limits: super::capture::CaptureLimits,
    previous: Option<std::sync::Arc<super::manifest::GenerationManifest>>,
) -> Result<std::sync::Arc<super::capture::CapturedSourceView>, ConstellationError> {
    let mut request = super::capture::CaptureRequest::new(workspace);
    if let Some(previous) = previous {
        request = request.with_published_manifest(previous);
    }
    let captured = super::capture::capture_source(&request, limits).await?;
    Ok(std::sync::Arc::new(captured))
}

/// The binding a vector must be published against, for one captured row.
///
/// # Which type supplies which field — the impedance mismatch, resolved
///
/// The CLI cannot do this. `ArtifactRecord` carries `id`, `body_hash` and an
/// `embedding` column; it has **no** `incarnation`, **no** `row_revision`, no
/// `input_digest` and no `profile_id`. Any binding it constructed would be a
/// subset, and a subset is what a stale job satisfies.
///
/// [`SourceEntry`] carries all eight of those directly, because capture is
/// where they are computed:
///
/// | Field | From |
/// |---|---|
/// | `index_epoch`, `checkout_id` | `SourceEntry` — stamped from the derived manifest at capture |
/// | `canonical_key` | `SourceEntry` — ADR-012 key, not the display id |
/// | `incarnation`, `row_revision` | `SourceEntry` — incremented by capture when the row changed |
/// | `source_digest` | `SourceEntry` — SHA-256 of the captured bytes |
/// | `input_digest` | `SourceEntry` — SHA-256 of the bytes handed to the tokenizer |
/// | `profile_id` | the argument — the one field capture cannot know, because capture does not resolve a profile |
///
/// Nothing is substituted for a field that is absent. If `input_digest` is
/// `None` the caller has no binding and no vector may be published, because
/// publishing without it is exactly the unguarded write this replaces.
pub fn binding_for(
    entry: &SourceEntry,
    profile_id: &str,
) -> Result<jobs::Binding, ConstellationError> {
    let input_digest = entry
        .input_digest
        .clone()
        .ok_or_else(|| ConstellationError::Io {
            detail: format!(
                "captured row `{}` has no input digest; there is no binding to publish against",
                entry.canonical_key
            ),
        })?;
    Ok(jobs::Binding {
        index_epoch: entry.index_epoch,
        checkout_id: entry.checkout_id.clone(),
        canonical_key: entry.canonical_key.clone(),
        incarnation: entry.incarnation,
        row_revision: entry.row_revision,
        source_digest: entry.source_digest.clone(),
        input_digest,
        profile_id: profile_id.to_string(),
    })
}

/// The binding a vector must be published against, for one captured
/// **section**.
///
/// A section borrows its parent's durable identity — epoch, checkout,
/// incarnation, row revision, source digest — because the section row is
/// rewritten wholesale whenever the parent's bytes move; the section's own
/// fields are only its `canonical_key` (`"{parent}#s{ordinal}"`) and its
/// `input_digest`. Publishing a section vector against the *artifact's*
/// input digest would bind the vector to a different string than the one it
/// was computed from, which is exactly the stale-vector hole the digest
/// separation exists to close.
///
/// # Errors
///
/// Never fails on digest absence — a [`CapturedSection`] always carries its
/// `input_digest`. `Ok` is unconditional so this mirrors [`binding_for`]'s
/// shape without borrowing its error case.
pub fn binding_for_section(
    section: &super::capture::CapturedSection,
    parent: &SourceEntry,
    profile_id: &str,
) -> Result<jobs::Binding, ConstellationError> {
    Ok(jobs::Binding {
        index_epoch: parent.index_epoch,
        checkout_id: parent.checkout_id.clone(),
        canonical_key: section.canonical_key.clone(),
        incarnation: parent.incarnation,
        row_revision: parent.row_revision,
        source_digest: parent.source_digest.clone(),
        input_digest: section.input_digest.clone(),
        profile_id: profile_id.to_string(),
    })
}

/// Publish a batch of **computed** results, each carrying the job it was
/// computed for.
///
/// The signature is the contract: it takes
/// [`jobs::ComputedEmbedding`], not `(captured, profile, vectors)`. There is
/// no longer any route by which a caller hands this function a canonical key
/// and a float vector and lets it work out the binding — that is what let a
/// vector computed from superseded bytes be rebind onto the capture taken
/// after the compute. The binding comes from the job, and the job came from
/// admission under the owner.
///
/// The caller does the **encoding** — explicit `embed` is one of the three
/// permitted model obtainers — and this does the **writing**, which is where
/// the binding actually matters.
///
/// The guard is [`jobs::Binding::predicate`], the version rule is the one from
/// P2, and the stale semantics are [`jobs::EmbeddingPublish::stale`]. All three
/// already exist and are tested.
///
/// # Which table a result lands on
///
/// [`jobs::EmbeddingJob::row_role`], from the key's shape. Not from a
/// capture: a completion must not re-derive what it is publishing onto, and the
/// two key namespaces are disjoint by construction — `section_key` is the only
/// producer of a `"{parent}#s{ordinal}"` key.
///
/// # Results that name the same key twice
///
/// Refused. Two vectors for one key is either a worker that answered twice or
/// a caller that meant to publish two different rows; neither may be resolved
/// by picking one, and the update is the only thing that could.
pub async fn publish_embeddings(
    tables: &super::lance_tables::IndexTables,
    results: &[jobs::ComputedEmbedding],
) -> Result<Vec<jobs::EmbeddingPublish>, ConstellationError> {
    let mut seen = BTreeSet::new();
    for result in results {
        let key = result.job.canonical_key.as_str();
        if !seen.insert(key.to_string()) {
            return Err(ConstellationError::Io {
                detail: format!(
                    "refusing to publish `{key}`: two results name the same row, and choosing \
                     one would discard the other's vector"
                ),
            });
        }
    }

    let mut published = Vec::with_capacity(results.len());
    for result in results {
        let binding = result.binding();
        let outcome = match result.row_role() {
            jobs::EmbeddingRowRole::Artifact => {
                tables
                    .publish_embedding_into_artifacts(&binding, &result.vector)
                    .await?
            }
            jobs::EmbeddingRowRole::Section => {
                tables
                    .publish_embedding_into_sections(&binding, &result.vector)
                    .await?
            }
        };
        published.push(outcome);
    }
    Ok(published)
}

/// Which table a commit belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitTable {
    Artifacts,
    /// The edge half of the same atomic set. A stage that writes artifacts
    /// and forgets relations publishes a projection with no links, so the
    /// enum names both.
    Relations,
    /// The per-section embedding table — the third leg of the same atomic
    /// set. Its rows are the section-level vectors; a stage that publishes
    /// artifacts without them leaves the long tail unsearchable.
    Sections,
}

/// One bounded delete commit — the only op this phase can execute.
///
/// # Why the enum has exactly one variant
///
/// The three Lance operations an ideal stage needs are `merge_insert`,
/// `delete` and `update`. Only `delete` is executed here. The other two are
/// deliberately **absent rather than present-and-refused**: an op type that
/// can be planned but not executed is a trap, because the planner would emit
/// work the executor silently drops and the stage would report success over a
/// write that never happened. When they are implemented they join this enum
/// and `stage_sequence` starts emitting them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitOp {
    /// `merge_insert(["canonical_key"])` with an upsert policy.
    MergeUpsert {
        /// Number of rows in the batch this commit carries.
        rows: usize,
        /// Whether `when_not_matched_by_source_delete` may be attached.
        ///
        /// This is `CommitPlan::full_source_batch`, which is already gated on
        /// the capture being complete. It is carried on the op so the
        /// executor cannot recompute it from something weaker.
        full_source_batch: bool,
    },
    /// `Table::update().only_if(..).column(..).execute()` over a bounded key
    /// set.
    ///
    /// Emitted after the merge so it patches rows the merge has just written,
    /// and before the delete so a confirmed absence still wins.
    ColumnUpdate {
        keys: Vec<String>,
        assignments: Vec<ColumnAssignment>,
    },
    /// `Table::delete(&predicate)` over an enumerated, bounded key set.
    ///
    /// The keys live on the op, not in a count, so the predicate is built
    /// from the exact keys that were confirmed absent. A count cannot build a
    /// predicate; only a set can.
    DeleteByPredicate { keys: Vec<String> },
    /// Delete every edge whose `canonical_key` is in `keys` — the first half
    /// of replacing an artifact's outgoing set. Carries the keys for the
    /// same reason [`Self::DeleteByPredicate`] does.
    RelationsDelete { keys: Vec<String> },
    /// `merge_insert` the edges of changed artifacts against
    /// [`RELATIONS_MERGE_KEY`]. Runs after [`Self::RelationsDelete`] so the
    /// merged edges are the complete new set, not a union with residue.
    RelationsMerge {
        /// Number of edge rows the batch carries.
        rows: usize,
    },
    /// `merge_insert` the **whole captured section set** against
    /// [`MERGE_KEY`] on the sections table — the merge key is the section's
    /// own `canonical_key` (`"{parent}#s{ordinal}"`), so a heading rename is
    /// an insert under the new key plus a source-delete of the old one, all
    /// inside the same commit.
    SectionsMerge {
        /// Number of section rows the batch carries.
        rows: usize,
        /// Same gate as [`Self::MergeUpsert`]: `CommitPlan::full_source_batch`,
        /// attachable only when the capture is complete.
        full_source_batch: bool,
    },
    /// Delete every section row whose `parent_key` is in `parent_keys` —
    /// the only shape the section delete takes. A section is never deleted
    /// individually: a displaced ordinal or renamed heading is removed by the
    /// next `SectionsMerge`'s source-delete clause, under the parent staying
    /// alive.
    SectionsDelete { parent_keys: Vec<String> },
}

impl CommitOp {
    pub fn table(&self) -> CommitTable {
        match self {
            Self::MergeUpsert { .. }
            | Self::ColumnUpdate { .. }
            | Self::DeleteByPredicate { .. } => CommitTable::Artifacts,
            Self::RelationsDelete { .. } | Self::RelationsMerge { .. } => CommitTable::Relations,
            Self::SectionsMerge { .. } | Self::SectionsDelete { .. } => CommitTable::Sections,
        }
    }

    /// Whether this op may run at all.
    ///
    /// A delete or an update with no keys would produce a predicate matching
    /// every row; an update with no assignments would match rows and change
    /// nothing. Both are refused here rather than at the store.
    pub fn is_executable(&self) -> bool {
        match self {
            Self::MergeUpsert { rows, .. } => *rows > 0,
            Self::ColumnUpdate { keys, assignments } => !keys.is_empty() && !assignments.is_empty(),
            Self::DeleteByPredicate { keys } | Self::RelationsDelete { keys } => !keys.is_empty(),
            Self::RelationsMerge { rows } => *rows > 0,
            Self::SectionsMerge { rows, .. } => *rows > 0,
            Self::SectionsDelete { parent_keys } => !parent_keys.is_empty(),
        }
    }
}

/// The ordered sequence of commits one stage will execute.
///
/// Separate commits rather than one fused write, for a reason that is about
/// partial failure: a crash between two commits leaves a **prefix** applied,
/// which the tracker records and the next stage's head comparison rewinds. One
/// fused write would leave an unnameable half-state instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitSequence {
    pub ops: Vec<CommitOp>,
}

impl CommitSequence {
    /// The number of commits, which is also the number of version bumps the
    /// stage may produce.
    pub fn commit_count(&self) -> usize {
        self.ops.len()
    }

    /// Whether nothing at all would be written.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

/// What a stage must do to the relations table.
///
/// `delete_keys` are the artifacts whose outgoing edges must be fully
/// replaced (changed by this reconcile) or removed (confirmed deleted).
/// `merge_rows` is the number of edge rows the caller's filtered batch
/// carries — the edges of changed artifacts only. Supplied, not counted, for
/// the same reason as `merge_rows` and `delete_keys`: a count cannot build a
/// predicate and a predicate cannot be built from a count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationsPlan {
    pub delete_keys: Vec<String>,
    pub merge_rows: usize,
}

/// What a stage must do to the sections table.
///
/// Simpler than [`RelationsPlan`] for the same reason a section's identity
/// is its own `canonical_key`: the merge batch is always the **whole**
/// captured section set, so `when_not_matched_by_source_delete` inside the
/// upsert removes displaced ordinals and renamed headings itself. The only
/// separate delete is by `parent_key` — every section of an artifact that
/// vanished outright.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionsPlan {
    /// Parents whose every section must be deleted (artifact confirmed gone).
    pub parent_delete_keys: Vec<String>,
    /// Number of section rows the whole-corpus batch carries. `0` means no
    /// merge is emitted — an empty corpus's sections are still deleted by
    /// `parent_delete_keys` when the capture is complete.
    pub merge_rows: usize,
    /// The captured section set differs from the published generation's
    /// recorded membership, so the merge must run even on a pass whose
    /// artifact side is a no-op.
    ///
    /// It replaces the old `bootstrap` flag, which meant "the published
    /// generation pins no sections version". That condition no longer exists —
    /// the triple is always complete — and the flag it replaced was wrong for
    /// the case that matters: a capture that changes the **layout**, or that
    /// publishes an empty set because metadata went absent, moves the section
    /// set without moving a single artifact row. ADR-029 §4.3 allows an
    /// unchanged physical version only when the exact set matches, so "differs"
    /// is the condition, not "the artifact side changed".
    pub set_changed: bool,
}

/// Whether the capture's section set differs from the published membership.
///
/// Compares the **exact** key set and each row's own binding identity — parent,
/// ordinal, byte range, input digests and layout fingerprint. A layout change
/// alone is enough: the same keys would mean different bytes under a different
/// layout, and keeping the old physical version would publish section rows
/// whose stored text no longer matches what the key binds.
pub fn section_set_changed(
    captured: &super::capture::CapturedSourceView,
    published: Option<&GenerationManifest>,
) -> bool {
    let Some(published) = published else {
        return true;
    };
    let layout = captured.layout.fingerprint();
    let captured_rows: Vec<(&str, &str, u32, usize, usize, &str, &str)> = captured
        .sections
        .iter()
        .map(|section| {
            (
                section.canonical_key.as_str(),
                section.parent_key.as_str(),
                section.ordinal,
                section.byte_start,
                section.byte_end,
                section.input_digest.as_str(),
                layout.as_str(),
            )
        })
        .collect();
    let published_rows: Vec<(&str, &str, u32, usize, usize, &str, &str)> = published
        .section_bindings
        .iter()
        .map(|binding| {
            (
                binding.section_key.as_str(),
                binding.parent_key.as_str(),
                binding.ordinal,
                binding.byte_start,
                binding.byte_end,
                binding.section_input_digest.as_str(),
                binding.layout_fingerprint.as_str(),
            )
        })
        .collect();
    // `from_capture` sorts the bindings by key and the capture emits sections
    // parent-key-sorted then ordinal, which is the same order for keys of the
    // form `{parent}#s{n}` — but only under a lexically identical parent set.
    // Sorting both sides removes that assumption rather than relying on it.
    let mut captured_sorted = captured_rows.clone();
    let mut published_sorted = published_rows.clone();
    captured_sorted.sort();
    published_sorted.sort();
    if captured_sorted.len() != published_sorted.len() {
        return true;
    }
    captured_sorted != published_sorted
}

/// Turn a commit plan into the sequence of commits that can actually run.
///
/// `merge_rows` is the size of the artifact batch the caller built,
/// `delete_keys` the canonical keys a complete capture proved absent, and
/// `relations` what the same reconcile owes the edge table.
///
/// # Why the artifact delete is emitted only when there is no merge
///
/// A publishable plan carries `full_source_batch`, which attaches
/// `when_not_matched_by_source_delete` to the merge: confirmed absences are
/// removed **inside** the upsert commit, atomically, keyed against rows the
/// capture did not produce. A separate canonical-key delete after the merge
/// is not only redundant — it deletes the row a rename just wrote (same
/// `canonical_key`, new `relative_path`) and livelocks the publish.
///
/// The one case the merge cannot serve is **delete-all**: an empty capture
/// produces an empty batch, and [`execute_merge`] refuses empty batches.
/// That is the only time [`CommitOp::DeleteByPredicate`] is emitted, and it
/// is safe there — an empty capture contains no renamed row that could
/// collide.
///
/// `retired_keys` are canonical keys a *re-keyed Update* vacated in place —
/// the same path now claims a different key. They are not tombstones (the
/// file is not absent) and they are not counted in `commit.deletes`, so
/// they need their own emitter. The caller filters out any key this capture
/// re-claims under a different path: deleting one of those would kill the
/// row the merge just wrote.
pub fn stage_sequence(
    commit: &CommitPlan,
    merge_rows: usize,
    delete_keys: Vec<String>,
    retired_keys: Vec<String>,
    relations: &RelationsPlan,
    sections: &SectionsPlan,
) -> CommitSequence {
    let mut ops = Vec::new();
    if merge_rows > 0 && (commit.creates > 0 || commit.updates > 0) {
        ops.push(CommitOp::MergeUpsert {
            rows: merge_rows,
            // The gate stays exactly where `plan_commits` put it: on the
            // capture being complete. It is not recomputed here.
            full_source_batch: commit.full_source_batch,
        });
    }
    // Unchanged survivors make merge_rows nonzero in a deletion-only pass.
    if (commit.creates == 0 && commit.updates == 0 || !commit.full_source_batch)
        && commit.deletes > 0
        && !delete_keys.is_empty()
    {
        ops.push(CommitOp::DeleteByPredicate { keys: delete_keys });
    }
    // A vacated key is proven by the re-keyed Update itself — observed, not
    // inferred — so retiring its row is bounded exactly like a tombstone
    // delete. Under a full source batch the merge's own
    // `when_not_matched_by_source_delete` has already removed it (the old key
    // is not in the batch), and the op is emitted only for the partial shape
    // that clause cannot serve.
    if !commit.full_source_batch && !retired_keys.is_empty() {
        ops.push(CommitOp::DeleteByPredicate { keys: retired_keys });
    }
    // Relations come after the artifact ops: an edge must never be written
    // for a row that does not exist yet, and the delete half must run before
    // the merge so the new edge set replaces the old one exactly.
    if !relations.delete_keys.is_empty() {
        ops.push(CommitOp::RelationsDelete {
            keys: relations.delete_keys.clone(),
        });
    }
    if relations.merge_rows > 0 {
        ops.push(CommitOp::RelationsMerge {
            rows: relations.merge_rows,
        });
    }
    // Sections last, delete before merge, same law as relations.
    // `parent_delete_keys` are the same tombstones the artifact delete saw —
    // they only exist when the capture proved absence, so this `IN` list is
    // always confirmed, never a guess. Under a full source batch the merge's
    // own source-delete would also remove them — the op is redundant there
    // but harmless — while the **delete-all** case (empty section batch, no
    // merge emitted) has no other way to retire the rows.
    if !sections.parent_delete_keys.is_empty() {
        ops.push(CommitOp::SectionsDelete {
            parent_keys: sections.parent_delete_keys.clone(),
        });
    }
    // The merge is gated the same way as `MergeUpsert` plus one extra case:
    // `set_changed` means the captured section set differs from the published
    // membership — a layout change, or an empty set because metadata went
    // absent — so the table must be rewritten even on a pass whose artifact
    // side is a no-op. Without it, a layout change would leave the previous
    // generation's rows published under keys that now mean different bytes.
    if sections.merge_rows > 0 && (!commit.is_empty() || sections.set_changed) {
        ops.push(CommitOp::SectionsMerge {
            rows: sections.merge_rows,
            full_source_batch: commit.full_source_batch,
        });
    }
    CommitSequence { ops }
}

/// The semantic columns a published row already carries.
///
/// A reconcile's merge batch cannot produce them — the capture does not
/// compute embeddings — so an unconditional `when_matched_update_all` would
/// write the batch's nulls over a live vector: one edited file would
/// invalidate the whole corpus. Splicing the stored values into the batch
/// for rows whose content did not move is the preserve half; the capture's
/// nulls remain the invalidate half for rows it cannot prove unchanged.
///
/// Read from the published table at the baseline head, keyed by
/// `canonical_key` — see [`super::lance_tables::IndexTables::artifact_semantics`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SemanticColumns {
    /// The generation identity the stored row was published under. A rebind
    /// that bumps the epoch, or an index that moved to another checkout,
    /// retires every pending vector job — the same tuple fences the guarded
    /// update, so it fences the splice.
    pub index_epoch: Option<u64>,
    pub checkout_id: Option<String>,
    /// Incarnation: it changes only on a path change or a delete/recreate.
    /// That is exactly the event after which a stored vector describes a
    /// document that no longer exists under this name.
    pub incarnation: Option<u64>,
    /// The digest of the bytes the stored vector was computed from. This is
    /// the freshness a vector answers to.
    ///
    /// The stored row's `source_digest` and `row_revision` are deliberately
    /// **not** part of this struct and **not** part of the decision: both move
    /// on any byte change, including a status, tag or `valid_until` edit,
    /// while `input_digest` covers only what the tokenizer receives. Reading
    /// them here would invalidate the whole corpus on the first frontmatter
    /// edit.
    pub input_digest: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub profile_id: Option<String>,
    pub semantic_state: Option<String>,
    pub retry_after: Option<String>,
}

impl SemanticColumns {
    /// Whether the stored semantic tuple answers to exactly this identity and
    /// input.
    ///
    /// The canonical key is the map key, so it is not repeated. What remains
    /// is the tuple a stale completion is fenced by — `(index_epoch,
    /// checkout_id, incarnation)` — plus the input digest.
    ///
    /// An artifact passes its own identity and `input_digest`; a section
    /// passes its **parent's** identity (the section row stores the parent's
    /// `incarnation`/epoch/checkout) and the section's own `input_digest`,
    /// because a parent's unrelated edit does not invalidate a section whose
    /// text did not move.
    fn is_preserved_for(
        &self,
        index_epoch: u64,
        checkout_id: &str,
        incarnation: u64,
        input_digest: &str,
    ) -> bool {
        self.index_epoch == Some(index_epoch)
            && self.checkout_id.as_deref() == Some(checkout_id)
            && self.incarnation == Some(incarnation)
            && self.input_digest.as_deref() == Some(input_digest)
    }
}

/// Build the artifacts `RecordBatch` for a capture.
///
/// The column list is **derived from** [`artifacts_v1_schema`] by iterating
/// its fields, never restated. A restated list is how a batch and its schema
/// drift apart, and `merge_insert` rejects a batch whose columns do not match
/// the table.
///
/// `preserved` is the semantic column splice, read back from the published
/// head by [`super::lance_tables::IndexTables::artifact_semantics`]. A row
/// whose stored `(index_epoch, checkout_id, incarnation)` and `input_digest`
/// still match the capture carries its stored `embedding`, `profile_id`,
/// `semantic_state` and `retry_after` into the batch, so the merge writes them
/// back unchanged instead of the capture's nulls.
///
/// An absent key, a different generation, a new incarnation or a changed
/// input digest gets the reset: no vector, no profile, no retry, and
/// `semantic_state = "unavailable"`. That is written explicitly rather than
/// defaulted from the capture, so a row can never keep a profile or a `ready`
/// claim while its vector is gone.
///
/// A metadata-only edit — `status`, `tags`, `valid_until` — moves
/// `source_digest` and `row_revision` but not `input_digest`, so it
/// **preserves**. Requiring the source digest here would invalidate the entire
/// corpus the first time anyone touched a frontmatter line.
///
/// # The three null rules
///
/// 1. A field the capture does not declare becomes a **null**. Absence stays
///    absence on the way to storage: substituting `now` for a missing
///    `created_at` would make every capture look like an edit.
/// 2. `embedding` is null unless a vector is genuinely present — either the
///    drain's stored one, spliced back for an unchanged row, or none. There
///    is no zero-vector, because a zero-filled `FixedSizeListArray` is
///    indistinguishable from a real vector to every downstream reader.
/// 3. `semantic_state` is `unavailable` unless a preserved tuple says
///    otherwise. Not `ready`, not `failed` — a row that carries no vector has
///    no business claiming it is embedded.
///
/// Zero rows is legal and produces an empty batch with the right schema.
///
/// # Errors
///
/// [`ConstellationError::IndexProtocolMismatch`] when the schema declares a
/// column this builder does not know. That is the failure mode worth
/// catching: a new column must not be silently dropped from the batch.
pub fn artifacts_batch(
    captured: &[Arc<super::capture::ParsedArtifact>],
    index_epoch: u64,
    checkout_id: &str,
    preserved: &BTreeMap<String, SemanticColumns>,
) -> Result<Arc<arrow_array::RecordBatch>, ConstellationError> {
    let schema = super::lance_tables::artifacts_v1_schema();
    let row_count = captured.len();

    // Builders live in `arrow_array::builder`, not at the crate root
    // (`arrow-array-57.3.0/src/builder/mod.rs:417,422` for the string
    // aliases; the primitive builders follow the same module).
    let mut id = arrow_array::builder::StringBuilder::new();
    let mut kind = arrow_array::builder::StringBuilder::new();
    let mut status = arrow_array::builder::StringBuilder::new();
    let mut title = arrow_array::builder::StringBuilder::new();
    let mut body = arrow_array::builder::LargeStringBuilder::new();
    let mut depth = arrow_array::builder::StringBuilder::new();
    let mut author = arrow_array::builder::StringBuilder::new();
    let mut parent_epic = arrow_array::builder::StringBuilder::new();
    let mut valid_until = arrow_array::builder::StringBuilder::new();
    let mut created_at = arrow_array::builder::StringBuilder::new();
    let mut updated_at = arrow_array::builder::StringBuilder::new();
    let mut body_hash = arrow_array::builder::StringBuilder::new();
    let mut canonical_key = arrow_array::builder::StringBuilder::new();
    let mut relative_path = arrow_array::builder::StringBuilder::new();
    let mut source_digest = arrow_array::builder::StringBuilder::new();
    let mut input_digest = arrow_array::builder::StringBuilder::new();
    let mut profile_id = arrow_array::builder::StringBuilder::new();
    let mut semantic_state = arrow_array::builder::StringBuilder::new();
    let mut retry_after = arrow_array::builder::StringBuilder::new();
    let mut diagnostics = arrow_array::builder::StringBuilder::new();
    let mut incarnation = arrow_array::builder::UInt64Builder::new();
    let mut row_revision = arrow_array::builder::UInt64Builder::new();
    let mut index_epoch_builder = arrow_array::builder::UInt64Builder::new();
    let mut checkout_id_builder = arrow_array::builder::StringBuilder::new();
    let mut r_eff_score = arrow_array::builder::Float64Builder::new();
    // Rule 2's builder half: each slot is either the stored vector spliced
    // back for a proven-unchanged row or a null — never a zero fill.
    let mut embedding_builder = arrow_array::builder::FixedSizeListBuilder::new(
        arrow_array::builder::Float32Builder::new(),
        super::lance_tables::EMBEDDING_DIM,
    );

    for artifact in captured {
        // The stored semantic tuple is a unit: a row whose identity and input
        // the table still proves keeps all of it, a row it cannot prove keeps
        // none of it. Note what is *not* compared — `source_digest` and
        // `row_revision`. Both move on a metadata-only edit, and neither
        // changes what the tokenizer receives, so requiring them would throw
        // away every vector the first time someone edited a `status:` line.
        let preserved_columns = preserved.get(&artifact.canonical_key).filter(|columns| {
            columns.is_preserved_for(
                index_epoch,
                checkout_id,
                artifact.incarnation,
                artifact.input_digest.as_str(),
            )
        });
        id.append_value(artifact.display_id.as_str());
        kind.append_value(artifact.kind.as_str());
        status.append_value(artifact.status.as_str());
        title.append_value(artifact.title.as_str());
        body.append_value(artifact.body.as_ref());
        depth.append_option(artifact.depth.as_deref());
        author.append_option(artifact.author.as_deref());
        parent_epic.append_option(artifact.parent_epic.as_deref());
        valid_until.append_option(artifact.valid_until.as_deref());
        // Rule 1: absence is null, never a fabricated timestamp.
        created_at.append_option(artifact.created_at.as_deref());
        updated_at.append_option(artifact.updated_at.as_deref());
        // No legacy body hash is published by capture; the column exists for
        // compatibility and stays null rather than carrying a value this
        // protocol did not compute.
        body_hash.append_null();
        canonical_key.append_value(artifact.canonical_key.as_str());
        relative_path.append_value(artifact.relative_path.as_str());
        source_digest.append_value(artifact.source_digest.as_str());
        input_digest.append_value(artifact.input_digest.as_str());
        // Invalidation is written as an explicit reset, not as a fallback to
        // whatever capture happens to carry. Capture declares no profile and
        // `unavailable` today, so both spellings agree — but a row that lost
        // its vector must not inherit a profile or a `ready` claim from the
        // capture path if that ever changes. The whole tuple goes: no vector,
        // no profile, no retry, and the honest state.
        profile_id
            .append_option(preserved_columns.and_then(|columns| columns.profile_id.as_deref()));
        semantic_state.append_value(
            preserved_columns
                .and_then(|columns| columns.semantic_state.as_deref())
                .unwrap_or(super::jobs::SemanticState::Unavailable.as_str()),
        );
        retry_after
            .append_option(preserved_columns.and_then(|columns| columns.retry_after.as_deref()));
        match preserved_columns.and_then(|columns| columns.embedding.as_ref()) {
            Some(vector) => {
                for value in vector {
                    embedding_builder.values().append_value(*value);
                }
                embedding_builder.append(true);
            }
            None => {
                embedding_builder
                    .values()
                    .append_nulls(super::lance_tables::EMBEDDING_DIM as usize);
                embedding_builder.append(false);
            }
        }
        // Bound first: a temporary would be dropped while borrowed.
        let codes = join_diagnostic_codes(&artifact.diagnostics);
        diagnostics.append_option(if codes.is_empty() {
            None
        } else {
            Some(codes.as_str())
        });
        incarnation.append_value(artifact.incarnation);
        row_revision.append_value(artifact.row_revision);
        // Generation identity is per-batch: every row of this publication
        // carries the epoch and checkout it was published under, so a stale
        // job from another generation can be fenced out.
        index_epoch_builder.append_value(index_epoch);
        checkout_id_builder.append_value(checkout_id);
        // The score is derived, and capture does not compute it in P1/P2. A
        // null is the honest value; a fabricated 0.0 would read as "scored
        // zero", which is a different statement.
        r_eff_score.append_null();
    }

    let tags_field = schema
        .field_with_name("tags")
        .map_err(|e| unknown_column("tags", e))?
        .clone();

    // Rule 2: no vector is present until P3, so the column is all-null. Tags
    // are the one nested column with real data, but they are carried in the
    // manifest rather than rebuilt here, so this batch leaves them null too
    // rather than writing an empty list that would erase them on merge.
    // Built as `ArrayRef` here, once. `new_null` returns the concrete array, and
    // `Arc::clone` on a concrete `T` yields `T` rather than unsizing to
    // `Arc<dyn Array>` — so the coercion belongs at construction, and the
    // match arms use a cheap refcount clone.
    // `ListArray::new_null` takes the *child* field — the outer `tags`
    // field would wrap a second `List` around it.
    let tags_item_field = match tags_field.data_type() {
        arrow_schema::DataType::List(child) | arrow_schema::DataType::LargeList(child) => {
            child.clone()
        }
        other => {
            return Err(unknown_column(
                "tags",
                format!("expected List item type, found {other}"),
            ));
        }
    };
    let tags: arrow_array::ArrayRef =
        Arc::new(arrow_array::ListArray::new_null(tags_item_field, row_count));
    // The child field is `item: Float32`, which is exactly what
    // `FixedSizeListBuilder` produces — the builder above already holds each
    // row's spliced vector or null slot, so there is no array-level null
    // trick left to explain.
    let embedding: arrow_array::ArrayRef = Arc::new(embedding_builder.finish());

    // Built with an explicit loop rather than `collect`, because the
    // unknown-column arm must return an error from inside the iteration —
    // an iterator yielding `Result` cannot be collected into `Vec<ArrayRef>`.
    let mut columns: Vec<arrow_array::ArrayRef> = Vec::with_capacity(schema.fields().len());
    for field in schema.fields() {
        let column: arrow_array::ArrayRef = match field.name().as_str() {
            "id" => Arc::new(id.finish()),
            "kind" => Arc::new(kind.finish()),
            "status" => Arc::new(status.finish()),
            "title" => Arc::new(title.finish()),
            "body" => Arc::new(body.finish()),
            "depth" => Arc::new(depth.finish()),
            "author" => Arc::new(author.finish()),
            "parent_epic" => Arc::new(parent_epic.finish()),
            "r_eff_score" => Arc::new(r_eff_score.finish()),
            "valid_until" => Arc::new(valid_until.finish()),
            "created_at" => Arc::new(created_at.finish()),
            "updated_at" => Arc::new(updated_at.finish()),
            "body_hash" => Arc::new(body_hash.finish()),
            "embedding" => embedding.clone(),
            "tags" => tags.clone(),
            "canonical_key" => Arc::new(canonical_key.finish()),
            "relative_path" => Arc::new(relative_path.finish()),
            "source_digest" => Arc::new(source_digest.finish()),
            "input_digest" => Arc::new(input_digest.finish()),
            "profile_id" => Arc::new(profile_id.finish()),
            "incarnation" => Arc::new(incarnation.finish()),
            "row_revision" => Arc::new(row_revision.finish()),
            "index_epoch" => Arc::new(index_epoch_builder.finish()),
            "checkout_id" => Arc::new(checkout_id_builder.finish()),
            "semantic_state" => Arc::new(semantic_state.finish()),
            "retry_after" => Arc::new(retry_after.finish()),
            "diagnostics" => Arc::new(diagnostics.finish()),
            other => return Err(unknown_column(other, String::new())),
        };
        columns.push(column);
    }

    arrow_array::RecordBatch::try_new(schema, columns)
        .map(Arc::new)
        .map_err(|e| ConstellationError::IndexProtocolMismatch {
            detail: format!("the artifacts batch does not match artifacts_v1_schema: {e}"),
        })
}

fn unknown_column(name: &str, source: impl std::fmt::Display) -> ConstellationError {
    ConstellationError::IndexProtocolMismatch {
        detail: format!(
            "the schema declares `{name}`, which this builder does not produce; \
             a new column must not be silently dropped from the batch ({source})"
        ),
    }
}

/// Build the relations `RecordBatch` for a capture.
///
/// One row per `(artifact, outgoing edge)` pair. The edge carries its
/// artifact's binding columns (`source_digest`, `incarnation`,
/// `row_revision`) so a reader can judge an edge by the same rules as the
/// row it hangs off. Columns are derived from `relations_v1_schema` — same
/// discipline as [`artifacts_batch`], and a zero-edge corpus produces an
/// empty batch with the right schema rather than a skipped table.
pub fn relations_batch(
    captured: &[Arc<super::capture::ParsedArtifact>],
) -> Result<Arc<arrow_array::RecordBatch>, ConstellationError> {
    let schema = super::lance_tables::relations_v1_schema();

    let mut canonical_key = arrow_array::builder::StringBuilder::new();
    let mut target = arrow_array::builder::StringBuilder::new();
    let mut relation = arrow_array::builder::StringBuilder::new();
    let mut source_digest = arrow_array::builder::StringBuilder::new();
    let mut row_revision = arrow_array::builder::UInt64Builder::new();
    let mut incarnation = arrow_array::builder::UInt64Builder::new();

    for artifact in captured {
        for edge in &artifact.outgoing {
            canonical_key.append_value(artifact.canonical_key.as_str());
            target.append_value(edge.target.as_str());
            relation.append_value(edge.relation.as_str());
            source_digest.append_value(artifact.source_digest.as_str());
            row_revision.append_value(artifact.row_revision);
            incarnation.append_value(artifact.incarnation);
        }
    }

    let mut columns: Vec<arrow_array::ArrayRef> = Vec::with_capacity(schema.fields().len());
    for field in schema.fields() {
        let column: arrow_array::ArrayRef = match field.name().as_str() {
            "canonical_key" => Arc::new(canonical_key.finish()),
            "target" => Arc::new(target.finish()),
            "relation" => Arc::new(relation.finish()),
            "source_digest" => Arc::new(source_digest.finish()),
            "row_revision" => Arc::new(row_revision.finish()),
            "incarnation" => Arc::new(incarnation.finish()),
            other => return Err(unknown_column(other, String::new())),
        };
        columns.push(column);
    }

    arrow_array::RecordBatch::try_new(schema, columns)
        .map(Arc::new)
        .map_err(|e| ConstellationError::IndexProtocolMismatch {
            detail: format!("the relations batch does not match relations_v1_schema: {e}"),
        })
}

/// Build the sections `RecordBatch` for a capture.
///
/// One row per captured section — the batch is always the **whole** section
/// set of the capture, because the merge's `when_not_matched_by_source_delete`
/// clause is what retires displaced ordinals and renamed headings atomically.
/// A section row borrows its parent's durable binding columns (`source_digest`,
/// `incarnation`, `row_revision`, epoch, checkout) and carries its own
/// `canonical_key`, `parent_key`, `ordinal`, `heading`, and `input_digest`.
/// `embedding`/`profile_id`/`semantic_state` follow the same null rules as
/// [`artifacts_batch`]. `preserved` is the same stored-column splice, read
/// back by [`super::lance_tables::IndexTables::section_semantics`] and judged
/// against the **parent's** `(index_epoch, checkout_id, incarnation)` plus the
/// section's own `input_digest`. A section whose *text* did not move keeps its
/// vector even when the parent's `source_digest` and `row_revision` did; a
/// parent that was recreated at a new path retires every one of its sections.
///
/// Columns are derived from [`super::lance_tables::sections_v1_schema`] — the
/// same discipline as the other two builders — so a schema drift fails loudly
/// here rather than silently at the merge.
///
/// # Errors
///
/// [`ConstellationError::IndexProtocolMismatch`] when the schema declares a
/// column this builder does not produce.
pub fn sections_batch(
    captured: &super::capture::CapturedSourceView,
    preserved: &BTreeMap<String, SemanticColumns>,
) -> Result<Arc<arrow_array::RecordBatch>, ConstellationError> {
    let schema = super::lance_tables::sections_v1_schema();
    let mut canonical_key = arrow_array::builder::StringBuilder::new();
    let mut parent_key = arrow_array::builder::StringBuilder::new();
    let mut parent_display_id = arrow_array::builder::StringBuilder::new();
    let mut ordinal = arrow_array::builder::UInt64Builder::new();
    let mut byte_start = arrow_array::builder::UInt64Builder::new();
    let mut byte_end = arrow_array::builder::UInt64Builder::new();
    let mut heading = arrow_array::builder::StringBuilder::new();
    let mut level = arrow_array::builder::UInt64Builder::new();
    let mut title = arrow_array::builder::StringBuilder::new();
    let mut content = arrow_array::builder::LargeStringBuilder::new();
    let mut source_digest = arrow_array::builder::StringBuilder::new();
    let mut input_digest = arrow_array::builder::StringBuilder::new();
    let mut profile_id = arrow_array::builder::StringBuilder::new();
    let mut incarnation = arrow_array::builder::UInt64Builder::new();
    let mut row_revision = arrow_array::builder::UInt64Builder::new();
    let mut index_epoch = arrow_array::builder::UInt64Builder::new();
    let mut checkout_id = arrow_array::builder::StringBuilder::new();
    let mut semantic_state = arrow_array::builder::StringBuilder::new();
    let mut retry_after = arrow_array::builder::StringBuilder::new();
    let mut embedding_builder = arrow_array::builder::FixedSizeListBuilder::new(
        arrow_array::builder::Float32Builder::new(),
        super::lance_tables::EMBEDDING_DIM,
    );

    for section in &captured.sections {
        // The display half comes from the parsed parent (title, kind,
        // display_id); the binding half from its SourceEntry. Both are in the
        // same view by construction — `sections_for` only mints sections from
        // artifacts the capture parsed — but the lookups are checked rather
        // than assumed: a hand-built view is possible in a test, and a
        // section with no parent is an integrity error, not a skipped row.
        let artifact = captured.artifact(&section.parent_key).ok_or_else(|| {
            ConstellationError::IndexProtocolMismatch {
                detail: format!(
                    "section `{}` names parent `{}`, which has no parsed artifact",
                    section.canonical_key, section.parent_key
                ),
            }
        })?;
        let entry = captured
            .entries
            .iter()
            .find(|entry| entry.canonical_key == section.parent_key)
            .ok_or_else(|| ConstellationError::IndexProtocolMismatch {
                detail: format!(
                    "section `{}` names parent `{}`, which has no source entry",
                    section.canonical_key, section.parent_key
                ),
            })?;

        // The section's own `input_digest` plus its parent's identity is the
        // whole freshness question for its vector: the parent's durable
        // columns are rewritten below, but a vector only describes this text.
        // A parent's metadata edit moves `source_digest` and `row_revision`
        // without moving this section's bytes, so neither is compared.
        let preserved_columns = preserved.get(&section.canonical_key).filter(|columns| {
            columns.is_preserved_for(
                entry.index_epoch,
                entry.checkout_id.as_str(),
                entry.incarnation,
                section.input_digest.as_str(),
            )
        });

        canonical_key.append_value(section.canonical_key.as_str());
        parent_key.append_value(section.parent_key.as_str());
        parent_display_id.append_value(artifact.display_id.as_str());
        ordinal.append_value(u64::from(section.ordinal));
        byte_start.append_value(section.byte_start as u64);
        byte_end.append_value(section.byte_end as u64);
        heading.append_option(section.heading.as_deref());
        level.append_option(section.level.map(u64::from));
        title.append_value(artifact.title.as_str());
        content.append_value(section.content.as_ref());
        // The durable half is the parent's — the row is rewritten wholesale
        // whenever the parent's bytes move.
        source_digest.append_value(entry.source_digest.as_str());
        input_digest.append_value(section.input_digest.as_str());
        // Explicit reset, same law as `artifacts_batch`: a section whose text
        // moved keeps no profile, no retry and no vector, and says
        // `unavailable`. Never the parent's state — the parent may be `ready`
        // on the body prefix, which would falsely claim section coverage.
        profile_id
            .append_option(preserved_columns.and_then(|columns| columns.profile_id.as_deref()));
        incarnation.append_value(entry.incarnation);
        row_revision.append_value(entry.row_revision);
        index_epoch.append_value(entry.index_epoch);
        checkout_id.append_value(entry.checkout_id.as_str());
        // The section's own embedding has not been computed at capture time,
        // so `unavailable` — not the parent's state, which may be `ready` on
        // the prefix and would falsely claim section coverage. A preserved
        // row keeps whatever the table already claimed.
        semantic_state.append_value(
            preserved_columns
                .and_then(|columns| columns.semantic_state.as_deref())
                .unwrap_or(super::jobs::SemanticState::Unavailable.as_str()),
        );
        retry_after
            .append_option(preserved_columns.and_then(|columns| columns.retry_after.as_deref()));
        match preserved_columns.and_then(|columns| columns.embedding.as_ref()) {
            Some(vector) => {
                for value in vector {
                    embedding_builder.values().append_value(*value);
                }
                embedding_builder.append(true);
            }
            None => {
                embedding_builder
                    .values()
                    .append_nulls(super::lance_tables::EMBEDDING_DIM as usize);
                embedding_builder.append(false);
            }
        }
    }

    let embedding: arrow_array::ArrayRef = Arc::new(embedding_builder.finish());

    let mut columns: Vec<arrow_array::ArrayRef> = Vec::with_capacity(schema.fields().len());
    for field in schema.fields() {
        let column: arrow_array::ArrayRef = match field.name().as_str() {
            "canonical_key" => Arc::new(canonical_key.finish()),
            "parent_key" => Arc::new(parent_key.finish()),
            "parent_display_id" => Arc::new(parent_display_id.finish()),
            "ordinal" => Arc::new(ordinal.finish()),
            "byte_start" => Arc::new(byte_start.finish()),
            "byte_end" => Arc::new(byte_end.finish()),
            "heading" => Arc::new(heading.finish()),
            "level" => Arc::new(level.finish()),
            "title" => Arc::new(title.finish()),
            "content" => Arc::new(content.finish()),
            "embedding" => embedding.clone(),
            "source_digest" => Arc::new(source_digest.finish()),
            "input_digest" => Arc::new(input_digest.finish()),
            "profile_id" => Arc::new(profile_id.finish()),
            "incarnation" => Arc::new(incarnation.finish()),
            "row_revision" => Arc::new(row_revision.finish()),
            "index_epoch" => Arc::new(index_epoch.finish()),
            "checkout_id" => Arc::new(checkout_id.finish()),
            "semantic_state" => Arc::new(semantic_state.finish()),
            "retry_after" => Arc::new(retry_after.finish()),
            other => return Err(unknown_column(other, String::new())),
        };
        columns.push(column);
    }

    arrow_array::RecordBatch::try_new(schema, columns)
        .map(Arc::new)
        .map_err(|e| ConstellationError::IndexProtocolMismatch {
            detail: format!("the sections batch does not match sections_v1_schema: {e}"),
        })
}

/// Comma-separated diagnostic codes, so a reader can see them without parsing.
fn join_diagnostic_codes(diagnostics: &[super::SourceDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|d| d.code.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

/// A bounded update predicate over a fixed set of canonical keys.
///
/// Same machinery and same refusal as [`delete_predicate_for`]: an
/// enumerated `IN (…)` with escaped literals, never a wildcard, and an empty
/// key set refused rather than producing a predicate that matches every row.
pub fn update_predicate_for(canonical_keys: &[String]) -> Result<String, ConstellationError> {
    delete_predicate_for(canonical_keys)
}

/// One column assignment: `(column, expression)`.
///
/// The value is an **expression**, not a literal, because Lance evaluates it
/// against the row's previous value (`table/update.rs:52-56`). Callers that
/// want a constant write one; callers that want a derived value say so
/// explicitly rather than being handed a pre-formatted string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnAssignment {
    pub column: String,
    pub expression: String,
}

/// What one committed column update produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateOutcome {
    /// The version after the commit. Never `0`.
    pub version: u64,
    /// Whether the version came from `UpdateResult.version` or from a
    /// `Table::version()` read.
    pub version_source: VersionSource,
    /// Rows actually changed. `0` means the predicate matched nothing, which
    /// is a different outcome from "the commit failed".
    pub rows_updated: u64,
    /// The predicate that ran.
    pub predicate: String,
}

/// Execute a bounded column update against the artifacts table.
///
/// # The builder surface, verified
///
/// `Table::update() -> UpdateBuilder` (`lancedb-0.27.2/src/table.rs:598`), then
/// `UpdateBuilder::only_if(impl Into<String>) -> Self` (`table/update.rs:43-46`)
/// and `UpdateBuilder::column(column_name, update_expr) -> Self`
/// (`table/update.rs:57-63`), then `execute() -> Result<UpdateResult>`
/// (`table/update.rs:65-73`).
///
/// Note the real signature is `.column(name, expr)` — there is **no** `.set()`
/// on this builder. The doc phrase "evaluated against the previous row's
/// value" (`table/update.rs:52-56`) is why the second argument is an
/// expression and not a value.
///
/// The predicate is bounded and escaped, never concatenated user SQL, and an
/// empty key set is refused. An empty column list is also refused here rather
/// than left for Lance to reject as invalid input
/// (`table/update.rs:66-69`) — the early error names the cause.
///
/// Version recording: `UpdateResult { rows_updated, version }`
/// (`table/update.rs:14-21`); a reported `0` means the backend returned none
/// (`table/update.rs:18-20`), so it is replaced by a `Table::version()` read,
/// and a read of `0` is an error rather than a recorded value.
/// `pub(crate)`, not `pub`, for the same reason as [`execute_merge`]: the
/// table it writes to belongs to a sealed owning type.
pub(crate) async fn execute_column_update(
    table: &lancedb::Table,
    role: &str,
    canonical_keys: &[String],
    assignments: &[ColumnAssignment],
) -> Result<UpdateOutcome, ConstellationError> {
    let predicate = update_predicate_for(canonical_keys)?;
    if assignments.is_empty() {
        return Err(ConstellationError::StorageIo {
            detail: format!(
                "refusing to execute a column update with no assignments against `{predicate}`: \
                 it would match rows and change nothing"
            ),
        });
    }
    for assignment in assignments {
        if assignment.column.is_empty() {
            return Err(ConstellationError::StorageIo {
                detail: "a column assignment with an empty column name".to_string(),
            });
        }
    }

    let mut builder = table.update().only_if(predicate.clone());
    for assignment in assignments {
        builder = builder.column(assignment.column.clone(), assignment.expression.clone());
    }

    let result = builder
        .execute()
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("column update on the {role} table failed: {e}"),
        })?;

    let (version, version_source) = if result.version != 0 {
        (result.version, VersionSource::Reported)
    } else {
        let read = table.version().await.map_err(|e| ConstellationError::Io {
            detail: format!("cannot read the {role} version after a column update: {e}"),
        })?;
        if read == 0 {
            return Err(ConstellationError::Io {
                detail: "the {role} table reported version 0 after a column update".to_string(),
            });
        }
        (read, VersionSource::ReadFromTable)
    };

    Ok(UpdateOutcome {
        version,
        version_source,
        rows_updated: result.rows_updated,
        predicate,
    })
}

/// A named verification failure.
///
/// Each variant carries the values that broke the invariant. There is no
/// `verify_failed()`: a single unnamed failure is the same as not verifying,
/// because the reader cannot tell whether a schema drifted, a row vanished or
/// a digest disagreed — and those three have completely different responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationError {
    /// An expected `canonical_key` was not present at the pinned version.
    RowMissing { canonical_key: String },
    /// The row is present but its `source_digest` is not the captured one.
    DigestDisagreement {
        canonical_key: String,
        expected: String,
        found: String,
    },
    /// A row exists that the capture did not describe.
    UnexpectedRow { canonical_key: String },
    /// More than one pinned row claims the same canonical key.
    DuplicateRow { canonical_key: String },
    /// The reader is not on the version the commits reported.
    VersionMismatch { expected: u64, found: u64 },
    /// The relations reader is not on the version the commits reported.
    RelationsVersionMismatch { expected: u64, found: u64 },
    /// The sections reader is not on the version the commits reported.
    SectionsVersionMismatch { expected: u64, found: u64 },
    /// The relations table does not carry the edge count the capture implies.
    /// Per-edge digests are not re-checked — the artifact digest comparison
    /// is the integrity check; this proves the edge set is the right size.
    EdgeCountDisagreement { expected: usize, found: usize },
    /// The sections table does not carry the section set the capture
    /// implies — checked by count, with per-row key/digest agreement proven
    /// by the same RowMissing/DigestDisagreement/UnexpectedRow variants the
    /// artifacts half uses.
    SectionCountDisagreement { expected: usize, found: usize },
    /// The reader's schema is not `artifacts_v1_schema()`.
    SchemaMismatch { detail: String },
    /// The query itself failed. Named, with the underlying message.
    Query { detail: String },
}

impl std::fmt::Display for VerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RowMissing { canonical_key } => {
                write!(
                    f,
                    "row_missing: `{canonical_key}` is absent at the pinned version"
                )
            }
            Self::DigestDisagreement {
                canonical_key,
                expected,
                found,
            } => write!(
                f,
                "digest_disagreement: `{canonical_key}` carries `{found}`, the capture says \
                 `{expected}`"
            ),
            Self::UnexpectedRow { canonical_key } => write!(
                f,
                "unexpected_row: `{canonical_key}` is present but the capture does not describe it"
            ),
            Self::DuplicateRow { canonical_key } => write!(
                f,
                "duplicate_row: `{canonical_key}` occurs more than once at the pinned version"
            ),
            Self::VersionMismatch { expected, found } => write!(
                f,
                "version_mismatch: the artifacts reader is on version {found}, the commits reported {expected}"
            ),
            Self::RelationsVersionMismatch { expected, found } => write!(
                f,
                "relations_version_mismatch: the relations reader is on version {found}, \
                 the commits reported {expected}"
            ),
            Self::SectionsVersionMismatch { expected, found } => write!(
                f,
                "sections_version_mismatch: the sections reader is on version {found}, \
                 the commits reported {expected}"
            ),
            Self::EdgeCountDisagreement { expected, found } => write!(
                f,
                "edge_count_disagreement: the relations table holds {found} edges, \
                 the capture implies {expected}"
            ),
            Self::SectionCountDisagreement { expected, found } => write!(
                f,
                "section_count_disagreement: the sections table holds {found} rows, \
                 the capture implies {expected}"
            ),
            Self::SchemaMismatch { detail } => write!(f, "schema_mismatch: {detail}"),
            Self::Query { detail } => write!(f, "verify_query_failed: {detail}"),
        }
    }
}

impl std::error::Error for VerificationError {}

/// What verification actually checked.
///
/// Every field is populated from work that happened. `verified_keys` and
/// `expected_keys` together let a caller tell "verified" from "there was
/// nothing to verify", which is the difference between a clean result and a
/// vacuous one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReport {
    pub reader_version: u64,
    pub expected_keys: usize,
    pub verified_keys: usize,
    pub digests_compared: usize,
    /// The relations version the pinned edge reader opened.
    pub relations_version: u64,
    /// The edge count the capture implies across every artifact.
    pub expected_edges: usize,
    /// The edge count actually present at the pinned relations version.
    pub verified_edges: usize,
    /// The sections version the pinned reader opened. Always present: the
    /// check always runs, and an empty expectation verifies that the pinned
    /// version really holds no rows.
    pub sections_version: u64,
    /// The section count the capture implies. `0` is a real answer — a
    /// generation whose metadata was absent publishes no sections.
    pub expected_sections: usize,
    /// The section rows actually present at the pinned sections version.
    pub verified_sections: usize,
}

/// What the sections half of [`verify_staged`] must find.
///
/// `version` is the committed sections head; `digests` maps each section
/// `canonical_key` to the **parent** `source_digest` it must carry — the same
/// per-row integrity check the artifacts half performs, just keyed on the
/// section key.
pub struct SectionsExpectation {
    pub version: u64,
    pub digests: std::collections::BTreeMap<String, String>,
}

/// Verify a committed artifacts pair through an independently opened reader.
///
/// The reader comes from [`super::lance_tables::open_table_at_version`], which
/// opens a **separate** handle and `checkout`s the **exact** version. A
/// `Table::clone` is not used: Lance's `Table` holds an `Arc<dyn BaseTable>`
/// (`table.rs:369-371`) and a clone shares it, which is not isolation.
/// `checkout_latest` is never called — a reader that refreshed itself would
/// answer a different question than the one that pinned it.
///
/// # Boundedness
///
/// The reader batches and the source-derived expected batches are retained for
/// the comparison. Keys are indexed once; unexpected or mismatched rows fail
/// immediately. Semantic columns may be preserved across reconciliation and
/// are not derived from the capture.
///
/// # Errors
///
/// [`VerificationError`] with a named variant carrying the values that broke
/// the invariant. [`ConstellationError::IndexGenerationUnavailable`] when the
/// pinned version cannot be opened at all; there is no fallback to latest.
pub async fn verify_staged(
    index_root: &std::path::Path,
    marker: &crate::constellation::manifest::ProtocolMarker,
    expected_version: u64,
    expected: &std::collections::BTreeMap<String, String>,
    expected_relations_version: u64,
    expected_edges: usize,
    expected_capture: &super::capture::CapturedSourceView,
    expected_sections: &SectionsExpectation,
) -> Result<VerificationReport, VerifyFailure> {
    verify_staged_impl(index_root, marker, expected_version, expected, expected_relations_version,
        expected_edges, Some(expected_capture), expected_sections, true).await
}

/// Verify a published or untouched row whose diagnostics may contain an
/// operational embedding failure. Identity and all other source fields remain
/// checked; this mode never relaxes bootstrap's full payload comparison.
pub(super) async fn verify_staged_with_operational_diagnostics(
    index_root: &std::path::Path,
    marker: &crate::constellation::manifest::ProtocolMarker,
    expected_version: u64,
    expected: &std::collections::BTreeMap<String, String>,
    expected_relations_version: u64,
    expected_edges: usize,
    expected_capture: &super::capture::CapturedSourceView,
    expected_sections: &SectionsExpectation,
) -> Result<VerificationReport, VerifyFailure> {
    verify_staged_impl(index_root, marker, expected_version, expected, expected_relations_version,
        expected_edges, Some(expected_capture), expected_sections, false).await
}

/// Check only the physical pins of a historical generation whose original
/// edge oracle is unavailable. This never certifies a staged publication.
pub(super) async fn verify_historical_pins(
    index_root: &std::path::Path,
    marker: &crate::constellation::manifest::ProtocolMarker,
    expected_version: u64,
    expected: &std::collections::BTreeMap<String, String>,
    expected_relations_version: u64,
    expected_edges: usize,
    expected_sections: &SectionsExpectation,
) -> Result<(), VerifyFailure> {
    verify_staged_impl(index_root, marker, expected_version, expected, expected_relations_version,
        expected_edges, None, expected_sections, false).await.map(|_| ())
}

async fn verify_staged_impl(
    index_root: &std::path::Path,
    marker: &crate::constellation::manifest::ProtocolMarker,
    expected_version: u64,
    expected: &std::collections::BTreeMap<String, String>,
    expected_relations_version: u64,
    expected_edges: usize,
    expected_capture: Option<&super::capture::CapturedSourceView>,
    expected_sections: &SectionsExpectation,
    compare_source_diagnostics: bool,
) -> Result<VerificationReport, VerifyFailure> {
    use std::collections::BTreeSet;

    use arrow_array::{Array, StringArray};

    let reader = super::lance_tables::open_table_at_version(
        index_root,
        marker,
        "artifacts",
        expected_version,
    )
    .await
    .map_err(|e| VerifyFailure::Unavailable(e.to_string()))?;

    // The schema is compared against `artifacts_v1_schema()`, never against
    // a literal, so a schema change surfaces here instead of as an obscure
    // merge failure later.
    let wanted = super::lance_tables::artifacts_v1_schema();
    let reader_schema = reader.schema().await.map_err(|e| {
        VerifyFailure::Invariant(VerificationError::Query {
            detail: format!("cannot read the reader schema: {e}"),
        })
    })?;
    let found_fields: Vec<(String, String)> = reader_schema
        .fields()
        .iter()
        .map(|f| (f.name().clone(), format!("{:?}", f.data_type())))
        .collect();
    let wanted_fields: Vec<(String, String)> = wanted
        .fields()
        .iter()
        .map(|f| (f.name().clone(), format!("{:?}", f.data_type())))
        .collect();
    if found_fields != wanted_fields {
        return Err(VerifyFailure::Invariant(
            VerificationError::SchemaMismatch {
                detail: format!(
                    "reader has {found_fields:?}, the active schema declares {wanted_fields:?}"
                ),
            },
        ));
    }

    let reader_version = reader.version().await.map_err(|e| {
        VerifyFailure::Invariant(VerificationError::Query {
            detail: format!("cannot read the reader version: {e}"),
        })
    })?;
    if reader_version != expected_version {
        return Err(VerifyFailure::Invariant(
            VerificationError::VersionMismatch {
                expected: expected_version,
                found: reader_version,
            },
        ));
    }

    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut digests_compared = 0usize;

    let batches = reader.query_batches().await.map_err(|e| {
        VerifyFailure::Invariant(VerificationError::Query {
            detail: format!("reading the pinned reader failed: {e}"),
        })
    })?;

    for batch in &batches {
        let keys = batch
            .column_by_name("canonical_key")
            .ok_or_else(|| {
                VerifyFailure::Invariant(VerificationError::SchemaMismatch {
                    detail: "the reader has no `canonical_key` column".to_string(),
                })
            })?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| {
                VerifyFailure::Invariant(VerificationError::SchemaMismatch {
                    detail: "`canonical_key` is not a string column".to_string(),
                })
            })?;
        let digests = batch
            .column_by_name("source_digest")
            .ok_or_else(|| {
                VerifyFailure::Invariant(VerificationError::SchemaMismatch {
                    detail: "the reader has no `source_digest` column".to_string(),
                })
            })?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| {
                VerifyFailure::Invariant(VerificationError::SchemaMismatch {
                    detail: "`source_digest` is not a string column".to_string(),
                })
            })?;

        for row in 0..batch.num_rows() {
            if keys.is_null(row) || digests.is_null(row) {
                return Err(VerifyFailure::Invariant(VerificationError::SchemaMismatch {
                    detail: "pinned artifact has a null key or source digest".to_string(),
                }));
            }
            let key = keys.value(row).to_string();
            let Some(wanted_digest) = expected.get(&key) else {
                // Returned immediately: accumulating unexpected keys would
                // make the verifier `O(corpus)` for no benefit.
                return Err(VerifyFailure::Invariant(VerificationError::UnexpectedRow {
                    canonical_key: key,
                }));
            };
            let found_digest = digests.value(row).to_string();
            if found_digest != *wanted_digest {
                return Err(VerifyFailure::Invariant(
                    VerificationError::DigestDisagreement {
                        canonical_key: key,
                        expected: wanted_digest.clone(),
                        found: found_digest,
                    },
                ));
            }
            digests_compared += 1;
            // Borrowed from `expected` rather than cloned: `seen` holds `&str`
            // so verification stays `O(expected keys)` with no allocation per
            // row.
            if let Some((borrowed, _)) = expected.get_key_value(&key)
                && !seen.insert(borrowed.as_str())
            {
                return Err(VerifyFailure::Invariant(VerificationError::DuplicateRow { canonical_key: key }));
            }
        }
    }

    // Every expected key must have been observed. An empty `expected` map
    // passes here, which is why the report carries both counts: a caller can
    // see that nothing was verified rather than inferring success.
    for key in expected.keys() {
        if !seen.contains(key.as_str()) {
            return Err(VerifyFailure::Invariant(VerificationError::RowMissing {
                canonical_key: key.clone(),
            }));
        }
    }
    if let Some(captured) = expected_capture {
        let batch = artifacts_batch(
            &captured.artifacts, captured.index_epoch, &captured.checkout_id, &BTreeMap::new(),
        ).map_err(|error| VerifyFailure::Invariant(VerificationError::Query {
            detail: format!("cannot build expected artifact payload: {error}"),
        }))?;
        verify_payload_batches("artifacts", &batch, &reader_schema, &batches, false, compare_source_diagnostics)
            .map_err(|error| VerifyFailure::Invariant(VerificationError::Query {
                detail: error.to_string(),
            }))?;
    }

    // The reader is pinned to the committed version. When the capture is
    // available, compare every edge binding, not just its cardinality.
    // A historical generation whose source has moved has no captured edge
    // oracle; callers may only check its pinned count in that case.
    let relations_reader = super::lance_tables::open_table_at_version(
        index_root,
        marker,
        "relations",
        expected_relations_version,
    )
    .await
    .map_err(|e| VerifyFailure::Unavailable(e.to_string()))?;

    let relations_version = relations_reader.version().await.map_err(|e| {
        VerifyFailure::Invariant(VerificationError::Query {
            detail: format!("cannot read the relations reader version: {e}"),
        })
    })?;
    if relations_version != expected_relations_version {
        return Err(VerifyFailure::Invariant(
            VerificationError::RelationsVersionMismatch {
                expected: expected_relations_version,
                found: relations_version,
            },
        ));
    }

    let relation_batches = relations_reader.query_batches().await.map_err(|e| {
        VerifyFailure::Invariant(VerificationError::Query {
            detail: format!("reading the pinned relations reader failed: {e}"),
        })
    })?;
    let verified_edges: usize = relation_batches.iter().map(|b| b.num_rows()).sum();
    if verified_edges != expected_edges {
        return Err(VerifyFailure::Invariant(
            VerificationError::EdgeCountDisagreement {
                expected: expected_edges,
                found: verified_edges,
            },
        ));
    }
    if let Some(captured) = expected_capture {
        super::facade::verify_captured_edge_batches(&relation_batches, captured)
            .map_err(|error| VerifyFailure::Invariant(VerificationError::Query {
                detail: format!("pinned relation membership differs from capture: {error}"),
            }))?;
    }

    // The sections half — **always** run. It used to be skipped when the
    // generation pinned no sections version, which is what let a generation
    // with an empty set and a generation that predates the table look
    // identical to a verifier. Now an empty expectation verifies that the
    // pinned version really holds no rows: pinned reader, version equality,
    // then per-row key + parent-digest agreement. A row the capture did not
    // mint fails as `UnexpectedRow` exactly as an orphan artifact row would.
    let expectation = expected_sections;
    let sections_reader = super::lance_tables::open_table_at_version(
        index_root,
        marker,
        "sections",
        expectation.version,
    )
    .await
    .map_err(|e| VerifyFailure::Unavailable(e.to_string()))?;

    let sections_version = sections_reader.version().await.map_err(|e| {
        VerifyFailure::Invariant(VerificationError::Query {
            detail: format!("cannot read the sections reader version: {e}"),
        })
    })?;
    if sections_version != expectation.version {
        return Err(VerifyFailure::Invariant(
            VerificationError::SectionsVersionMismatch {
                expected: expectation.version,
                found: sections_version,
            },
        ));
    }
    let expected_section_count = expectation.digests.len();
    let mut verified_section_count = 0usize;

    {
        let mut seen_sections: BTreeSet<&str> = BTreeSet::new();
        let section_batches = sections_reader.query_batches().await.map_err(|e| {
            VerifyFailure::Invariant(VerificationError::Query {
                detail: format!("reading the pinned sections reader failed: {e}"),
            })
        })?;
        for batch in &section_batches {
            let keys = batch
                .column_by_name("canonical_key")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .ok_or_else(|| {
                    VerifyFailure::Invariant(VerificationError::SchemaMismatch {
                        detail: "the sections reader has no string `canonical_key` column"
                            .to_string(),
                    })
                })?;
            let digests = batch
                .column_by_name("source_digest")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .ok_or_else(|| {
                    VerifyFailure::Invariant(VerificationError::SchemaMismatch {
                        detail: "the sections reader has no string `source_digest` column"
                            .to_string(),
                    })
                })?;
            for row in 0..batch.num_rows() {
                if keys.is_null(row) || digests.is_null(row) {
                    return Err(VerifyFailure::Invariant(VerificationError::SchemaMismatch {
                        detail: "pinned section has a null key or source digest".to_string(),
                    }));
                }
                let key = keys.value(row).to_string();
                let Some(wanted) = expectation.digests.get(&key) else {
                    return Err(VerifyFailure::Invariant(VerificationError::UnexpectedRow {
                        canonical_key: key,
                    }));
                };
                let found_digest = digests.value(row).to_string();
                if found_digest != *wanted {
                    return Err(VerifyFailure::Invariant(
                        VerificationError::DigestDisagreement {
                            canonical_key: key,
                            expected: wanted.clone(),
                            found: found_digest,
                        },
                    ));
                }
                verified_section_count += 1;
                if let Some((borrowed, _)) = expectation.digests.get_key_value(&key) {
                    seen_sections.insert(borrowed.as_str());
                }
            }
        }
        if verified_section_count != expected_section_count {
            return Err(VerifyFailure::Invariant(
                VerificationError::SectionCountDisagreement {
                    expected: expected_section_count,
                    found: verified_section_count,
                },
            ));
        }
        for key in expectation.digests.keys() {
            if !seen_sections.contains(key.as_str()) {
                return Err(VerifyFailure::Invariant(VerificationError::RowMissing {
                    canonical_key: key.clone(),
                }));
            }
        }
        if let Some(captured) = expected_capture {
            let batch = sections_batch(captured, &BTreeMap::new())
                .map_err(|error| VerifyFailure::Invariant(VerificationError::Query {
                    detail: format!("cannot build expected section payload: {error}"),
                }))?;
            let schema = sections_reader.schema().await
                .map_err(|error| VerifyFailure::Invariant(VerificationError::Query {
                    detail: format!("cannot read section schema: {error}"),
                }))?;
            verify_payload_batches("sections", &batch, &schema, &section_batches, false, compare_source_diagnostics)
                .map_err(|error| VerifyFailure::Invariant(VerificationError::Query {
                    detail: error.to_string(),
                }))?;
        }
    }

    Ok(VerificationReport {
        reader_version,
        expected_keys: expected.len(),
        verified_keys: seen.len(),
        digests_compared,
        relations_version,
        expected_edges,
        verified_edges,
        sections_version,
        expected_sections: expected_section_count,
        verified_sections: verified_section_count,
    })
}

/// Why verification could not be completed at all.
///
/// Distinct from an invariant break: a missing pinned version means the
/// generation is unavailable, which is a different fact from "the generation
/// is there and disagrees with the capture".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyFailure {
    /// A pinned version could not be opened. Never a fallback to latest.
    Unavailable(String),
    /// A named invariant broke.
    Invariant(VerificationError),
}

impl std::fmt::Display for VerifyFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(detail) => {
                write!(f, "index_generation_unavailable: {detail}")
            }
            Self::Invariant(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for VerifyFailure {}

/// The column `merge_insert` keys on.
///
/// Must match a column the active artifacts schema actually declares, or the
/// merge is keyed against nothing.
pub const MERGE_KEY: [&str; 1] = ["canonical_key"];

/// The merge key for the relations table: an edge's identity is the full
/// `(key, target, relation)` triple. Keying on `canonical_key` alone would
/// collapse an artifact's several edges into one row.
pub const RELATIONS_MERGE_KEY: [&str; 3] = ["canonical_key", "target", "relation"];

/// What one committed merge produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeOutcome {
    /// The version after the commit. Never `0`.
    pub version: u64,
    /// Whether the version came from `MergeResult.version` or from a
    /// `Table::version()` read.
    pub version_source: VersionSource,
    pub num_inserted_rows: u64,
    pub num_updated_rows: u64,
    pub num_deleted_rows: u64,
}

/// Where a recorded version came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionSource {
    /// Returned by `MergeResult.version` (`table/merge.rs:19-38`).
    Reported,
    /// Read from `Table::version()` after the commit.
    ReadFromTable,
}

impl VersionSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reported => "reported",
            Self::ReadFromTable => "read_from_table",
        }
    }
}

/// Execute one `merge_insert` against the artifacts table.
///
/// `full_source_batch` is `CommitPlan::full_source_batch` and is **not**
/// recomputed here. When it is true the bulk-delete clause is attached; when
/// it is false it is not, and a partial scan can never reach it — that clause
/// deletes every row the scan did not see.
///
/// Version recording: `MergeResult.version` exists but is documented `0` when
/// the backend does not return one (`table/merge.rs:21-23`), so a reported
/// `0` is **not** a version. It is replaced by a `Table::version()` read, and
/// a read of `0` is an error rather than a recorded value: a descriptor naming
/// version 0 points at a generation that does not exist.
///
/// The batch is shared by `Arc`, so the corpus is built once per capture and
/// every reader of this op sees the same bytes.
///
/// # Errors
///
/// [`ConstellationError::StorageIo`] when the op carries no rows or the batch
/// is empty. [`ConstellationError::Io`] when the merge or the version read
/// fails.
/// `pub(crate)`, not `pub`.
///
/// The handle that owns the connection is
/// [`super::lance_tables::IndexTables`], and this takes the bare `&Table` it
/// exists to withhold. Narrowing it means a `Table` can only be obtained from
/// inside the two owning types, whose fields are private — so "no `pub`
/// function in this crate names `lancedb::Table`" holds, and the seal is
/// structural rather than a convention anyone can route around.
///
/// The **result** type stays `pub`: a caller legitimately reads what a commit
/// produced. The handle that performed it is not theirs to hold.
pub(crate) async fn execute_merge(
    table: &lancedb::Table,
    role: &str,
    batch: Arc<arrow_array::RecordBatch>,
    full_source_batch: bool,
    merge_key: &[&str],
) -> Result<MergeOutcome, ConstellationError> {
    if batch.num_rows() == 0 {
        return Err(ConstellationError::StorageIo {
            detail: "refusing to execute a merge over an empty batch: it would commit a version \
                      that changed nothing"
                .to_string(),
        });
    }
    // The emptiness guard reads `batch.num_rows()` directly, and `MergeOutcome`
    // takes its counts from `MergeResult` — the post-merge truth — rather
    // than from what was merely offered. `verify_staged` independently
    // confirms membership and digests afterwards, so a second copy of the
    // offered row count would have had no reader.
    let mut builder = table.merge_insert(merge_key);
    builder.when_matched_update_all(None);
    builder.when_not_matched_insert_all();
    if full_source_batch {
        builder.when_not_matched_by_source_delete(None);
    }

    // `RecordBatchIterator::new(iter, schema)` — the iterator comes FIRST
    // (`arrow-array-57.3.0/src/record_batch.rs:902`), and
    // `RecordBatchIterator` itself implements `RecordBatchReader`
    // (`record_batch.rs:925`), so it can be boxed directly rather than
    // wrapped in a trait-object helper.
    let iterator =
        arrow_array::RecordBatchIterator::new(vec![Ok(batch.as_ref().clone())], batch.schema());
    let reader: Box<dyn arrow_array::RecordBatchReader + Send> = Box::new(iterator);

    let result = builder
        .execute(reader)
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("merge_insert on the {role} table failed: {e}"),
        })?;

    let (version, version_source) = if result.version != 0 {
        (result.version, VersionSource::Reported)
    } else {
        let read = table.version().await.map_err(|e| ConstellationError::Io {
            detail: format!("cannot read the {role} version after a merge: {e}"),
        })?;
        if read == 0 {
            return Err(ConstellationError::Io {
                detail: "the {role} table reported version 0 after a merge".to_string(),
            });
        }
        (read, VersionSource::ReadFromTable)
    };

    Ok(MergeOutcome {
        version,
        version_source,
        num_inserted_rows: result.num_inserted_rows,
        num_updated_rows: result.num_updated_rows,
        num_deleted_rows: result.num_deleted_rows,
    })
}

/// Execute one bounded delete against whichever table the op targets.
///
/// `DeleteByPredicate` and `RelationsDelete` enumerate `canonical_key`s;
/// `SectionsDelete` enumerates `parent_key`s — the columns differ, the
/// boundedness does not.
///
/// This is the sharpest of the three ops, and deliberately the first to be
/// executed: `DeleteResult` carries **no** version field
/// (`lancedb-0.27.2/src/table/delete.rs:9-13`), so the recording rule has to
/// be real rather than pass-through. The version is read from the table after
/// the commit, under the single owner, and never taken from the result.
///
/// The predicate comes from [`delete_predicate_for`] or
/// [`parent_key_predicate_for`] — an enumerated `IN (…)` over escaped
/// literals, never a wildcard, and an empty key set is refused rather than
/// producing a predicate that matches every row.
///
/// The caller holds the `WorkspaceLock` for the duration of this call and
/// releases it afterwards, in owner → workspace order. Nothing else runs
/// under it: not a model load, not an inference, not a queue wait.
///
/// # Errors
///
/// [`ConstellationError::StorageIo`] when the op carries no keys, when the
/// predicate cannot be built, or when the commit fails.
///
/// # What the result distinguishes
///
/// `rows_deleted == 0` with a successful version is a **different outcome**
/// from "deleted three rows": the predicate matched nothing. Both are
/// legitimate outcomes of a complete capture, and conflating them is how a
/// delete that silently matched nothing gets reported as a successful
/// reconciliation.
/// `pub(crate)`, not `pub`, for the same reason as [`execute_merge`]: the
/// table it writes to belongs to a sealed owning type.
pub(crate) async fn execute_delete(
    table: &lancedb::Table,
    op: &CommitOp,
) -> Result<DeleteOutcome, ConstellationError> {
    // `let ... else` rather than a `todo!()` arm. Handing a merge or a column
    // update to the delete executor is a programming error, not a runtime
    // condition, and it must be **named**: a `todo!()` here would panic
    // inside the writer's commit path, and a silent fall-through would let
    // the stage report success over a delete that never ran.
    let (predicate, keys) = match op {
        CommitOp::DeleteByPredicate { keys } | CommitOp::RelationsDelete { keys } => {
            if keys.is_empty() {
                return Err(ConstellationError::StorageIo {
                    detail: "refusing to execute a delete with no keys: the predicate would \
                             match every row"
                        .to_string(),
                });
            }
            (delete_predicate_for(keys)?, keys.clone())
        }
        CommitOp::SectionsDelete { parent_keys } => {
            if parent_keys.is_empty() {
                return Err(ConstellationError::StorageIo {
                    detail: "refusing to execute a sections delete with no parent keys: the \
                             predicate would match every row"
                        .to_string(),
                });
            }
            (parent_key_predicate_for(parent_keys)?, parent_keys.clone())
        }
        other => {
            return Err(ConstellationError::IndexProtocolMismatch {
                detail: format!(
                    "execute_delete was handed a {other:?} operation; it executes only \
                     DeleteByPredicate, RelationsDelete and SectionsDelete"
                ),
            });
        }
    };

    let result = table
        .delete(&predicate)
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("bounded delete with predicate `{predicate}` failed: {e}"),
        })?;

    // `DeleteResult` has no version field, so this is the only source. It is
    // read after the commit, under the owner, and a `0` here would mean the
    // table never committed — which is not a version this protocol can record.
    let version = table.version().await.map_err(|e| ConstellationError::Io {
        detail: format!("cannot read the table version after a delete: {e}"),
    })?;
    if version == 0 {
        return Err(ConstellationError::Io {
            detail: "the table reported version 0 after a delete".to_string(),
        });
    }

    Ok(DeleteOutcome {
        version,
        rows_deleted: result.num_deleted_rows,
        keys,
        predicate,
    })
}

/// What one committed delete produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteOutcome {
    /// The version after the commit. Never `0`.
    pub version: u64,
    /// Rows actually removed. `0` means the predicate matched nothing, which
    /// is not the same as "the commit failed".
    pub rows_deleted: u64,
    /// The exact keys the predicate enumerated.
    pub keys: Vec<String>,
    /// The predicate that ran, kept so a caller can report what it matched on
    /// rather than reconstructing it.
    pub predicate: String,
}

/// Escape a literal for a bounded SQL predicate.
///
/// Type1/Arrow expression syntax uses single quotes for string literals, and a
/// literal containing one has to have it doubled. User-supplied text —
/// a target id, a title — never goes into a predicate unescaped, because a
/// predicate is the one place where a quote in the data becomes structure.
pub fn escape_predicate_literal(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('\'');
    for ch in value.chars() {
        if ch == '\'' {
            out.push('\'');
        }
        out.push(ch);
    }
    out.push('\'');
    out
}

/// A bounded delete predicate over a fixed set of canonical keys.
///
/// `IN (…)` over an enumerated list, never a wildcard: a wildcard predicate
/// derived from a partial capture is the exact shape that deletes healthy rows.
pub fn delete_predicate_for(canonical_keys: &[String]) -> Result<String, ConstellationError> {
    if canonical_keys.is_empty() {
        return Err(ConstellationError::StorageIo {
            detail: "an empty delete predicate would match every row".to_string(),
        });
    }
    Ok(format!(
        "canonical_key IN ({})",
        in_literals(canonical_keys)
    ))
}

/// A bounded delete predicate over `parent_key` — the sections table's
/// delete column. Sections are removed only by parent ("every section of
/// these gone artifacts"), never individually, so the predicate is keyed on
/// the column that names the parent rather than the row.
pub fn parent_key_predicate_for(parent_keys: &[String]) -> Result<String, ConstellationError> {
    if parent_keys.is_empty() {
        return Err(ConstellationError::StorageIo {
            detail: "an empty delete predicate would match every row".to_string(),
        });
    }
    Ok(format!("parent_key IN ({})", in_literals(parent_keys)))
}

/// The escaped, quoted literal list for one `IN` clause.
fn in_literals(keys: &[String]) -> String {
    keys.iter()
        .map(|key| escape_predicate_literal(key))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::capture::{
        CaptureLimits, CaptureRequest, CapturedSection, Coverage, FilesystemSupport, SourceEntry,
        UnusableSource,
    };
    use crate::constellation::jobs::SemanticState;
    use crate::constellation::manifest::GenerationManifest;
    use crate::constellation::reconciler::plan_reconcile;
    use crate::workspace::ARTIFACT_DIRS;
    use arrow_array::Array;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[cfg(not(unix))]
    #[test]
    fn non_unix_publisher_refuses_before_creating_owner_or_index() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("lance");
        let store = std::sync::Arc::new(object_store::memory::InMemory::new());
        assert!(IndexPublisher::acquire(&root, store).is_err());
        assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 0);
    }

    // -------------------------------------------------------------------
    // Bootstrap planning
    // -------------------------------------------------------------------

    fn marker(state: BootstrapState) -> ProtocolMarker {
        ProtocolMarker {
            protocol_version: crate::constellation::FORMAT_VERSION,
            state,
            index_epoch: 1,
            checkout_id: "local-x".to_string(),
            table_names: ProtocolMarker::active_table_names(),
            storage_scheme: crate::constellation::manifest::STORAGE_SCHEME_FILE_OBJECT_STORE
                .to_string(),
            bootstrap_attempt: 1,
        }
    }

    fn descriptor_for(generation_id: &str) -> CurrentDescriptor {
        CurrentDescriptor {
            format_version: crate::constellation::FORMAT_VERSION,
            index_epoch: 1,
            checkout_id: "local-x".to_string(),
            generation_id: generation_id.to_string(),
            owner_epoch: 3,
            source_set_digest: "ss".to_string(),
            config_digest: "cd".to_string(),
            profile_id: None,
            evaluated_at: "2026-10-17T00:00:00.000Z".to_string(),
            manifest_path: format!("generations/{generation_id}.json"),
            manifest_sha256: "aa".to_string(),
        }
    }

    #[test]
    fn a_missing_marker_and_pointer_is_a_first_run() {
        assert_eq!(
            plan_bootstrap(&IndexProtocolState::Uninitialized, Ok(None)),
            BootstrapPlan::BuildInitialBaseline
        );
    }

    #[test]
    fn an_interrupted_bootstrap_rebuilds_and_never_adopts_partial_candidates() {
        assert_eq!(
            plan_bootstrap(
                &IndexProtocolState::Initializing(marker(BootstrapState::Initializing)),
                Ok(None)
            ),
            BootstrapPlan::BuildInitialBaseline
        );
    }

    #[test]
    fn an_initializing_marker_with_a_valid_pointer_finishes_activation() {
        assert_eq!(
            plan_bootstrap(
                &IndexProtocolState::Initializing(marker(BootstrapState::Initializing)),
                Ok(Some(descriptor_for("gen-7")))
            ),
            BootstrapPlan::FinishActivation {
                generation_id: "gen-7".to_string()
            }
        );
    }

    #[test]
    fn a_corrupt_pointer_under_initializing_is_unavailable_not_a_clean_bootstrap() {
        let err = ConstellationError::IndexGenerationUnavailable {
            detail: "CURRENT.json does not parse".to_string(),
        };
        let plan = plan_bootstrap(
            &IndexProtocolState::Initializing(marker(BootstrapState::Initializing)),
            Err(err),
        );
        assert!(
            matches!(plan, BootstrapPlan::Unavailable { .. }),
            "masking this as a clean bootstrap would discard a published \
             generation: {plan:?}"
        );
    }

    #[test]
    fn an_active_index_with_a_missing_pointer_is_unavailable_not_an_empty_reinit() {
        assert!(matches!(
            plan_bootstrap(
                &IndexProtocolState::Active(marker(BootstrapState::Active)),
                Ok(None)
            ),
            BootstrapPlan::Unavailable { .. }
        ));
        let err = ConstellationError::IndexGenerationUnavailable {
            detail: "corrupt".to_string(),
        };
        assert!(matches!(
            plan_bootstrap(
                &IndexProtocolState::Active(marker(BootstrapState::Active)),
                Err(err)
            ),
            BootstrapPlan::Unavailable { .. }
        ));
    }

    #[test]
    fn an_active_index_with_a_valid_pointer_needs_no_rebuild() {
        assert_eq!(
            plan_bootstrap(
                &IndexProtocolState::Active(marker(BootstrapState::Active)),
                Ok(Some(descriptor_for("gen-9")))
            ),
            BootstrapPlan::FinishActivation {
                generation_id: "gen-9".to_string()
            }
        );
    }

    // -------------------------------------------------------------------
    // Head comparison
    // -------------------------------------------------------------------

    fn manifest_with_versions(artifacts: u64, relations: u64, sections: u64) -> GenerationManifest {
        GenerationManifest {
            schema_version: crate::constellation::FORMAT_VERSION,
            generation_id: "gen-1".to_string(),
            checkout_id: "local-x".to_string(),
            index_epoch: 1,
            source_set_digest: "ss".to_string(),
            config_digest: "cd".to_string(),
            table_versions: TableVersions::new(artifacts, relations, sections),
            source_entries: Vec::new(),
            section_bindings: Vec::new(),
            tombstones: Vec::new(),
            identity_retirements: Default::default(),
            conflict_inventory: Vec::new(),
            semantic_blocked_paths: Vec::new(),
            pending_jobs: Default::default(),
            coverage: Coverage {
                expected: 0,
                read: 0,
                invalid: 0,
                unavailable: 0,
                excluded: 0,
            },
            diagnostics: Vec::new(),
            evaluated_at: "2026-10-17T00:00:00.000Z".to_string(),
        }
    }

    #[test]
    fn nothing_published_means_nothing_to_diverge_from() {
        assert_eq!(compare_heads(None, 7, 9, 4), HeadComparison::Matches);
    }

    #[test]
    fn matching_three_way_heads_restore_nothing_and_commit_nothing() {
        let published = manifest_with_versions(3, 2, 5);
        assert_eq!(
            compare_heads(Some(&published), 3, 2, 5),
            HeadComparison::Matches
        );
    }

    #[test]
    fn divergence_names_exactly_the_table_that_moved() {
        let published = manifest_with_versions(3, 2, 5);
        assert_eq!(
            compare_heads(Some(&published), 4, 2, 5),
            HeadComparison::Diverged {
                artifacts_diverged: true,
                relations_diverged: false,
                sections_diverged: false
            }
        );
        assert_eq!(
            compare_heads(Some(&published), 3, 5, 5),
            HeadComparison::Diverged {
                artifacts_diverged: false,
                relations_diverged: true,
                sections_diverged: false
            }
        );
        assert_eq!(
            compare_heads(Some(&published), 3, 2, 6),
            HeadComparison::Diverged {
                artifacts_diverged: false,
                relations_diverged: false,
                sections_diverged: true
            }
        );
        assert_eq!(
            compare_heads(Some(&published), 4, 5, 7),
            HeadComparison::Diverged {
                artifacts_diverged: true,
                relations_diverged: true,
                sections_diverged: true
            }
        );
    }

    /// A stale sections head is now divergence like any other. It used to be
    /// exempt whenever the manifest pinned no sections version, and that
    /// exemption is what let a head an abandoned pass left behind survive into
    /// a published generation.
    #[test]
    fn a_moved_sections_head_is_divergence_and_is_rewound() {
        let published = manifest_with_versions(3, 2, 5);
        assert_eq!(
            compare_heads(Some(&published), 3, 2, 42),
            HeadComparison::Diverged {
                artifacts_diverged: false,
                relations_diverged: false,
                sections_diverged: true
            },
            "a sections head nobody published must not be served as current"
        );
    }

    #[test]
    fn a_sections_pin_of_zero_is_torn_not_divergence() {
        // `0` is not a real version — the empty creation commit is v1. It
        // means the manifest was torn mid-write, not that the table moved.
        let published = manifest_with_versions(3, 2, 0);
        assert_eq!(
            compare_heads(Some(&published), 3, 2, 3),
            HeadComparison::MissingPublishedVersion { role: "sections" }
        );
    }

    #[test]
    fn a_published_pair_with_a_missing_half_is_unavailable_not_divergence() {
        let published = manifest_with_versions(0, 2, 1);
        assert_eq!(
            compare_heads(Some(&published), 4, 2, 1),
            HeadComparison::MissingPublishedVersion { role: "pair" }
        );
    }

    // -------------------------------------------------------------------
    // Commit planning
    // -------------------------------------------------------------------

    fn entry(key: &str, path: &str, digest: &str) -> SourceEntry {
        SourceEntry {
            canonical_key: key.to_string(),
            relative_path: path.to_string(),
            checkout_id: "local-x".to_string(),
            index_epoch: 1,
            incarnation: 1,
            row_revision: 1,
            source_digest: digest.to_string(),
            input_digest: Some("in".to_string()),
            profile_id: None,
            semantic_state: SemanticState::Unavailable,
            diagnostic_codes: Vec::new(),
            observed_at: "2026-10-17T00:00:00.000Z".to_string(),
        }
    }

    fn view(entries: Vec<SourceEntry>, unusable: &[&str], invalid: usize) -> CapturedSourceView {
        let enumerated: Vec<String> = entries
            .iter()
            .map(|e| e.relative_path.clone())
            .chain(unusable.iter().map(|p| (*p).to_string()))
            .collect();
        CapturedSourceView {
            workspace: PathBuf::from("/tmp/ws/.forgeplan"),
            checkout_id: "local-x".to_string(),
            index_epoch: 1,
            snapshot_id: "snap".to_string(),
            evaluated_at: "2026-10-17T00:00:00.000Z".to_string(),
            source_set_digest: "ss".to_string(),
            config_digest: "cd".to_string(),
            layout: crate::embed::tokenizer::SectionLayout::from_metadata(
                "tok".to_string(),
                "tokcfg".to_string(),
                "cfgjson".to_string(),
                Some(crate::embed::tokenizer::MAX_TOKENS_CEILING),
                2,
            ),
            tokenizer: None,
            body_prefix_chars: super::super::jobs::DEFAULT_BODY_PREFIX_CHARS,
            artifacts: Vec::new(),
            conflict_artifacts: Vec::new(),
            sections: Vec::new(),
            coverage: Coverage {
                expected: enumerated.len(),
                read: entries.len(),
                invalid,
                unavailable: 0,
                excluded: 0,
            },
            entries,
            unusable: unusable
                .iter()
                .map(|p| UnusableSource {
                    relative_path: (*p).to_string(),
                    canonical_key: None,
                    code: DiagnosticCode::UnreadableSource,
                    detail: "fixture".to_string(),
                })
                .collect(),
            enumerated_paths: enumerated,
            diagnostics: Vec::new(),
            confirmed: true,
            filesystem_support: FilesystemSupport::Supported,
            attempts: 1,
        }
    }

    fn complete(entries: Vec<SourceEntry>) -> CapturedSourceView {
        view(entries, &[], 0)
    }

    fn published(entries: Vec<SourceEntry>) -> GenerationManifest {
        GenerationManifest::from_capture(&complete(entries), "gen-1", TableVersions::new(1, 1, 1))
    }

    #[test]
    fn a_matching_capture_produces_an_empty_commit_plan() {
        let entries = vec![entry("PRD-001", "prds/PRD-001-a.md", "d1")];
        let plan = plan_reconcile(&complete(entries.clone()), Some(&published(entries)));
        let commits = plan_commits(&plan);
        assert!(
            commits.is_empty(),
            "a stage that changes nothing must commit nothing"
        );
        assert!(commits.full_source_batch);
    }

    #[test]
    fn a_changed_digest_produces_exactly_one_update() {
        let previous = published(vec![entry("PRD-001", "prds/PRD-001-a.md", "d1")]);
        let plan = plan_reconcile(
            &complete(vec![entry("PRD-001", "prds/PRD-001-a.md", "d2")]),
            Some(&previous),
        );
        let commits = plan_commits(&plan);
        assert_eq!(commits.updates, 1);
        assert_eq!(commits.creates, 0);
        assert_eq!(commits.deletes, 0);
    }

    #[test]
    fn a_complete_capture_that_lost_a_path_commits_one_delete() {
        let previous = published(vec![
            entry("PRD-001", "prds/PRD-001-a.md", "d1"),
            entry("PRD-002", "prds/PRD-002-b.md", "d2"),
        ]);
        let plan = plan_reconcile(
            &complete(vec![entry("PRD-001", "prds/PRD-001-a.md", "d1")]),
            Some(&previous),
        );
        let commits = plan_commits(&plan);
        assert_eq!(commits.deletes, 1);
        assert!(commits.full_source_batch);
    }

    #[test]
    fn a_partial_capture_never_gets_the_bulk_delete_clause() {
        let previous = published(vec![
            entry("PRD-001", "prds/PRD-001-a.md", "d1"),
            entry("PRD-002", "prds/PRD-002-b.md", "d2"),
        ]);
        let plan = plan_reconcile(
            &view(
                vec![entry("PRD-001", "prds/PRD-001-a.md", "d1")],
                &["prds/PRD-003-c.md"],
                1,
            ),
            Some(&previous),
        );
        let commits = plan_commits(&plan);
        assert_eq!(commits.deletes, 0);
        assert_eq!(commits.retained, 1);
        assert!(
            !commits.full_source_batch,
            "`when_not_matched_by_source_delete` on a partial scan would delete \
             every row the scan could not see"
        );
    }

    /// The orphan-B case: generation A had X, an unpublished B changed X, and
    /// canonical has now returned X to exactly A. The delta from the
    /// *published* baseline contains no X, so staging on top of the orphan B
    /// would leave B's X in the published generation.
    #[test]
    fn delta_from_the_published_baseline_does_not_inherit_an_orphan_change() {
        let published_entries = vec![
            entry("PRD-001", "prds/PRD-001-a.md", "d1"),
            // X as A had it.
            entry("X", "prds/x.md", "x-from-A"),
        ];
        let previous = published(published_entries);

        // Canonical is back to A: both digests match the baseline.
        let current = complete(vec![
            entry("PRD-001", "prds/PRD-001-a.md", "d1"),
            entry("X", "prds/x.md", "x-from-A"),
        ]);

        let plan = plan_reconcile(&current, Some(&previous));
        let commits = plan_commits(&plan);
        assert_eq!(
            commits.updates, 0,
            "X is back to A; the delta must not mention it"
        );
        assert_eq!(commits.creates, 0);
        assert_eq!(commits.deletes, 0);
        assert!(commits.is_empty());

        // And the plan therefore tells the publisher there is nothing to
        // write, so there is nothing to inherit from the orphan head either.
        assert!(
            plan.deltas
                .iter()
                .all(|d| d.action == DeltaAction::Unchanged)
        );
    }

    // -------------------------------------------------------------------
    // Candidate currency
    // -------------------------------------------------------------------

    #[test]
    fn a_candidate_matching_everything_publishes() {
        assert!(
            candidate_is_current(Some("gen-1"), Some("gen-1"), 1, 1, "ss", "ss", "cd", "cd")
                .is_ok()
        );
    }

    #[test]
    fn a_candidate_built_on_a_moved_base_is_discarded() {
        let err = candidate_is_current(Some("gen-2"), Some("gen-1"), 1, 1, "ss", "ss", "cd", "cd")
            .unwrap_err();
        assert_eq!(err.code(), DiagnosticCode::SourceBusy);
    }

    #[test]
    fn a_candidate_for_a_different_source_set_or_config_is_discarded() {
        assert!(candidate_is_current(Some("g"), Some("g"), 1, 1, "ss", "ss2", "cd", "cd").is_err());
        assert!(candidate_is_current(Some("g"), Some("g"), 1, 1, "ss", "ss", "cd", "cd2").is_err());
    }

    #[test]
    fn a_candidate_for_epoch_zero_is_a_protocol_mismatch() {
        // Epoch 0 is reserved for "never published"; a candidate claiming it
        // is not a first publication but a missing base.
        let err =
            candidate_is_current(Some("g"), Some("g"), 1, 0, "ss", "ss", "cd", "cd").unwrap_err();
        assert_eq!(err.code(), DiagnosticCode::IndexProtocolMismatch);
    }

    // -------------------------------------------------------------------
    // First generation
    // -------------------------------------------------------------------

    /// A capture that runs under a lock the caller already holds.
    ///
    /// `maintain_index` holds the workspace lock across its final source
    /// validation and the pointer switch, and the recheck that validates has
    /// to run inside that same lock — a lock is not reentrant, so a helper
    /// that acquired its own would block until the timeout and fail. This
    /// test holds the lock and calls the seam directly: it passes only while
    /// the seam does not take a second acquisition.
    #[tokio::test]
    async fn a_capture_runs_under_a_workspace_lock_the_caller_already_holds() {
        let tmp = TempDir::new().unwrap();
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        std::fs::write(
            tmp.path().join("prds").join("PRD-001-auth.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\n---\n\nBody.\n",
        )
        .unwrap();

        let guard = crate::workspace::acquire_workspace_lock(tmp.path())
            .await
            .expect("the test must hold the lock");
        let captured = capture_while_locked(tmp.path(), CaptureLimits::default(), None)
            .await
            .expect("a capture under an already-held lock must not try to take it again");
        assert_eq!(
            captured.entries.len(),
            1,
            "the capture must actually observe the workspace, not return an empty view"
        );
        drop(guard);
    }

    #[tokio::test]
    async fn a_complete_capture_publishes_a_first_generation() {
        let tmp = TempDir::new().unwrap();
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        std::fs::write(
            tmp.path().join("prds").join("PRD-001-auth.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\n---\n\nBody.\n",
        )
        .unwrap();
        let captured = crate::constellation::capture::capture_source(
            &CaptureRequest::new(tmp.path()),
            CaptureLimits::default(),
        )
        .await
        .unwrap();

        let manifest = first_generation(&captured, "gen-1", TableVersions::new(1, 1, 1)).unwrap();
        assert_eq!(manifest.generation_id, "gen-1");
        assert_eq!(manifest.source_entries.len(), 1);
        assert!(manifest.coverage.is_complete());
        assert_eq!(manifest.checkout_id, captured.checkout_id);
    }

    #[tokio::test]
    async fn a_partial_capture_publishes_neither_complete_nor_empty() {
        let tmp = TempDir::new().unwrap();
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        std::fs::write(
            tmp.path().join("prds").join("PRD-001-auth.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\n---\n\nBody.\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("prds").join("PRD-002-b.md"),
            "---\nbroken\n",
        )
        .unwrap();
        let captured = crate::constellation::capture::capture_source(
            &CaptureRequest::new(tmp.path()),
            CaptureLimits::default(),
        )
        .await
        .unwrap();

        let err = first_generation(&captured, "gen-1", TableVersions::new(1, 1, 1)).unwrap_err();
        assert_eq!(
            err.code(),
            DiagnosticCode::SourceBusy,
            "an empty first generation is indistinguishable from an empty workspace"
        );
    }

    #[cfg(unix)]
    fn bootstrap_test_publisher(root: &Path) -> IndexPublisher {
        IndexPublisher::acquire(root, Arc::new(object_store::memory::InMemory::new())).unwrap()
    }

    #[cfg(unix)]
    fn replace_test_column(
        batch: &arrow_array::RecordBatch,
        name: &str,
        value: arrow_array::ArrayRef,
    ) -> arrow_array::RecordBatch {
        let index = batch.schema().index_of(name).unwrap();
        let mut columns = batch.columns().to_vec();
        columns[index] = value;
        arrow_array::RecordBatch::try_new(batch.schema(), columns).unwrap()
    }

    #[cfg(unix)]
    async fn create_partial_dataset(
        publisher: &IndexPublisher,
        marker: &ProtocolMarker,
        role: &str,
    ) {
        let uri = super::super::storage::table_uri_for_role(marker, role, publisher.index_root()).unwrap();
        let wrapper: Arc<dyn lance_io::object_store::WrappingObjectStore> =
            publisher.tracking_store().clone();
        lancedb::table::NativeTable::create_empty(
            &uri, role, Vec::new(), super::super::lance_tables::schema_for_role(role).unwrap(),
            Some(wrapper), None, None, None, false,
        ).await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn bootstrap_recovers_each_partial_dataset_and_each_sequential_prefix() {
        for roles in [
            &["artifacts"][..], &["relations"][..], &["sections"][..],
            &["artifacts", "relations"][..], &["artifacts", "relations", "sections"][..],
        ] {
            let tmp = TempDir::new().unwrap();
            let workspace = tmp.path().canonicalize().unwrap();
            for dir in ARTIFACT_DIRS {
                std::fs::create_dir_all(workspace.join(dir)).unwrap();
            }
            std::fs::write(workspace.join("prds/PRD-001-auth.md"),
                "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\n---\n\nBody.\n").unwrap();
            let root = workspace.join("lance");
            let publisher = bootstrap_test_publisher(&root);
            let checkout = IndexIdentity::path_bound_checkout_id(&IndexLocation::new(&root));
            let marker = publisher.announce_bootstrap(&checkout, 1, 1).unwrap();
            for role in roles {
                create_partial_dataset(&publisher, &marker, role).await;
            }
            drop(publisher);

            let publisher = bootstrap_test_publisher(&root);
            let progress = bootstrap_or_recover(&publisher, &workspace, CaptureLimits::default())
                .await.unwrap();
            assert!(matches!(progress, BootstrapProgress::RestartedExistingTables { .. }));
            let descriptor = super::super::storage::read_current(&root).unwrap().unwrap();
            let manifest = super::super::storage::read_manifest(&root, &descriptor).unwrap();
            assert_eq!(manifest.source_entries.len(), 1);
            for (role, version) in [
                ("artifacts", manifest.table_versions.artifacts),
                ("relations", manifest.table_versions.relations),
                ("sections", manifest.table_versions.sections),
            ] {
                let reader = super::super::lance_tables::open_table_at_version(
                    &root, &marker, role, version,
                ).await.unwrap();
                let count: usize = reader.query_batches().await.unwrap().iter()
                    .map(|batch| batch.num_rows()).sum();
                if role == "artifacts" { assert_eq!(count, 1); }
                if role == "relations" { assert_eq!(count, 0); }
            }
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn bootstrap_refuses_source_changed_after_pinned_verification() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().canonicalize().unwrap();
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(workspace.join(dir)).unwrap();
        }
        let path = workspace.join("prds/PRD-001-auth.md");
        std::fs::write(&path, "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\n---\n\nOriginal.\n").unwrap();
        let root = workspace.join("lance");
        let publisher = bootstrap_test_publisher(&root);
        let checkout = IndexIdentity::path_bound_checkout_id(&IndexLocation::new(&root));
        let marker = publisher.announce_bootstrap(&checkout, 1, 1).unwrap();
        let (tables, _) = super::super::lance_tables::resume_bootstrap_tables(
            &root, &marker, publisher.tracking_store().clone(), &workspace,
        ).await.unwrap();
        let captured = capture_under_workspace_lock(&workspace, CaptureLimits::default(), None).await.unwrap();
        let artifact_batch = artifacts_batch(
            &captured.artifacts, captured.index_epoch, &captured.checkout_id, &BTreeMap::new(),
        ).unwrap();
        let section_batch = sections_batch(&captured, &BTreeMap::new()).unwrap();
        tables.merge_into_artifacts(Arc::clone(&artifact_batch), true).await.unwrap();
        if section_batch.num_rows() != 0 {
            tables.merge_into_sections(Arc::clone(&section_batch), true).await.unwrap();
        }
        let versions = tables.head_versions().await.unwrap();
        let expected: BTreeMap<_, _> = captured.entries.iter()
            .map(|entry| (entry.canonical_key.clone(), entry.source_digest.clone())).collect();
        let section_digests = captured.sections.iter()
            .map(|section| (section.canonical_key.clone(), expected[&section.parent_key].clone()))
            .collect();
        verify_staged(
            &root, &marker, versions.artifacts, &expected, versions.relations, 0, &captured,
            &SectionsExpectation { version: versions.sections, digests: section_digests },
        ).await.unwrap();
        verify_bootstrap_payload(&root, &marker, "artifacts", versions.artifacts, &artifact_batch).await.unwrap();
        verify_bootstrap_payload(&root, &marker, "sections", versions.sections, &section_batch).await.unwrap();
        let manifest = first_generation(&captured, "gen-before-edit", versions).unwrap();

        // This edit happens deterministically after verification and before the
        // exact publication helper used by bootstrap acquires its final guard.
        std::fs::write(&path, "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\n---\n\nChanged.\n").unwrap();
        let result = publish_verified_bootstrap(
            &publisher, &workspace, CaptureLimits::default(), &captured, &manifest,
        ).await;
        assert!(matches!(result, Err(ConstellationError::SourceBusy { .. })));
        assert!(super::super::storage::read_current(&root).unwrap().is_none());
        std::fs::write(&path, "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\n---\n\nOriginal.\n").unwrap();
        std::fs::write(workspace.join("config.yaml"), "embedding:\n  chunk_size: 10\n").unwrap();
        let result = publish_verified_bootstrap(
            &publisher, &workspace, CaptureLimits::default(), &captured, &manifest,
        ).await;
        assert!(matches!(result, Err(ConstellationError::SourceBusy { .. })));
        assert!(super::super::storage::read_current(&root).unwrap().is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn interrupted_bootstrap_replaces_stale_rows_when_replacements_are_empty() {
        for empty_artifacts in [false, true] {
            let tmp = TempDir::new().unwrap();
            let workspace = tmp.path().canonicalize().unwrap();
            for dir in ARTIFACT_DIRS {
                std::fs::create_dir_all(workspace.join(dir)).unwrap();
            }
            let path = workspace.join("prds/PRD-001-auth.md");
            std::fs::write(&path, "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\nlinks:\n  - target: ADR-002\n    relation: refines\n---\n\nBody.\n").unwrap();
            let root = workspace.join("lance");
            let publisher = bootstrap_test_publisher(&root);
            let checkout = IndexIdentity::path_bound_checkout_id(&IndexLocation::new(&root));
            let marker = publisher.announce_bootstrap(&checkout, 1, 1).unwrap();
            let (tables, _) = super::super::lance_tables::resume_bootstrap_tables(
                &root, &marker, publisher.tracking_store().clone(), &workspace,
            ).await.unwrap();
            let captured = capture_under_workspace_lock(&workspace, CaptureLimits::default(), None)
                .await.unwrap();
            let mut old = captured.as_ref().clone();
            let mut section = captured_section();
            section.parent_key = old.entries[0].canonical_key.clone();
            section.canonical_key = format!("{}#stale", section.parent_key);
            old.sections = vec![section];
            tables.merge_into_artifacts(artifacts_batch(
                &old.artifacts, old.index_epoch, &old.checkout_id, &BTreeMap::new(),
            ).unwrap(), true).await.unwrap();
            tables.merge_into_relations(relations_batch(&old.artifacts).unwrap(), true).await.unwrap();
            tables.merge_into_sections(sections_batch(&old, &BTreeMap::new()).unwrap(), true).await.unwrap();
            if !empty_artifacts {
                let versions = tables.head_versions().await.unwrap();
                let artifact_batch = artifacts_batch(
                    &old.artifacts, old.index_epoch, &old.checkout_id, &BTreeMap::new(),
                ).unwrap();
                let section_batch = sections_batch(&old, &BTreeMap::new()).unwrap();
                verify_bootstrap_payload(&root, &marker, "artifacts", versions.artifacts, &artifact_batch).await.unwrap();
                verify_bootstrap_payload(&root, &marker, "sections", versions.sections, &section_batch).await.unwrap();
                let expected: BTreeMap<_, _> = old.entries.iter()
                    .map(|entry| (entry.canonical_key.clone(), entry.source_digest.clone())).collect();
                let section_expectation = SectionsExpectation {
                    version: versions.sections,
                    digests: old.sections.iter().map(|section| (
                        section.canonical_key.clone(), expected[&section.parent_key].clone()
                    )).collect(),
                };
                verify_staged(&root, &marker, versions.artifacts, &expected, versions.relations,
                    1, &old, &section_expectation).await.unwrap();
                let mut changed = old.clone();
                Arc::make_mut(&mut changed.artifacts[0]).body = Arc::from("altered body");
                assert!(verify_staged(&root, &marker, versions.artifacts, &expected,
                    versions.relations, 1, &changed, &section_expectation).await.is_err());
                let mut changed = old.clone();
                changed.sections[0].content = Arc::from("altered section");
                assert!(verify_staged(&root, &marker, versions.artifacts, &expected,
                    versions.relations, 1, &changed, &section_expectation).await.is_err());
                let mut changed = old.clone();
                Arc::make_mut(&mut changed.artifacts[0]).row_revision += 1;
                assert!(verify_staged(&root, &marker, versions.artifacts, &expected,
                    versions.relations, 1, &changed, &section_expectation).await.is_err());
                assert!(verify_staged_with_operational_diagnostics(&root, &marker, versions.artifacts,
                    &expected, versions.relations, 1, &changed, &section_expectation).await.is_err());
                let wrong_diagnostics = replace_test_column(
                    &artifact_batch, "diagnostics",
                    Arc::new(arrow_array::StringArray::from(vec!["different parser diagnostics"])),
                );
                let reader = super::super::lance_tables::open_table_at_version(
                    &root, &marker, "artifacts", versions.artifacts,
                ).await.unwrap();
                let schema = reader.schema().await.unwrap();
                let batches = reader.query_batches().await.unwrap();
                assert!(verify_payload_batches("artifacts", &wrong_diagnostics, &schema, &batches, false, true).is_err());
                assert!(verify_payload_batches("artifacts", &wrong_diagnostics, &schema, &batches, false, false).is_ok());
                assert!(verify_payload_batches("artifacts", &wrong_diagnostics, &schema, &batches, true, false).is_err());
                for (name, value) in [
                    ("title", Arc::new(arrow_array::StringArray::from(vec!["changed"])) as arrow_array::ArrayRef),
                    ("body", Arc::new(arrow_array::LargeStringArray::from(vec!["changed"])) as arrow_array::ArrayRef),
                ] {
                    let altered = replace_test_column(&artifact_batch, name, value);
                    assert!(verify_bootstrap_payload(&root, &marker, "artifacts", versions.artifacts, &altered).await.is_err(), "{name}");
                }
                for (name, value) in [
                    ("content", Arc::new(arrow_array::LargeStringArray::from(vec!["changed"])) as arrow_array::ArrayRef),
                    ("input_digest", Arc::new(arrow_array::StringArray::from(vec!["changed"])) as arrow_array::ArrayRef),
                    ("byte_start", Arc::new(arrow_array::UInt64Array::from(vec![999])) as arrow_array::ArrayRef),
                    ("byte_end", Arc::new(arrow_array::UInt64Array::from(vec![999])) as arrow_array::ArrayRef),
                ] {
                    let altered = replace_test_column(&section_batch, name, value);
                    assert!(verify_bootstrap_payload(&root, &marker, "sections", versions.sections, &altered).await.is_err(), "{name}");
                }
            }
            if empty_artifacts {
                // Each independent pinned reader must reject its own stale
                // row, including when the expected cardinality is zero.
                let mut empty = old.clone();
                empty.artifacts.clear();
                empty.entries.clear();
                empty.sections.clear();
                for role in ["artifacts", "relations", "sections"] {
                    let versions = tables.head_versions().await.unwrap();
                    let result = verify_staged(
                        &root, &marker, versions.artifacts, &BTreeMap::new(),
                        versions.relations, 0, &empty,
                        &SectionsExpectation { version: versions.sections, digests: BTreeMap::new() },
                    ).await;
                    assert!(result.is_err(), "stale {role} rows must fail pinned verification");
                    assert!(super::super::storage::read_current(&root).unwrap().is_none());
                    tables.clear_empty_bootstrap_dataset(role).await.unwrap();
                }
                let versions = tables.head_versions().await.unwrap();
                verify_staged(
                    &root, &marker, versions.artifacts, &BTreeMap::new(),
                    versions.relations, 0, &empty,
                    &SectionsExpectation { version: versions.sections, digests: BTreeMap::new() },
                ).await.unwrap();
                // Restore the interrupted attempt for the actual retry.
                tables.merge_into_artifacts(artifacts_batch(
                    &old.artifacts, old.index_epoch, &old.checkout_id, &BTreeMap::new(),
                ).unwrap(), true).await.unwrap();
                tables.merge_into_relations(relations_batch(&old.artifacts).unwrap(), true).await.unwrap();
                tables.merge_into_sections(sections_batch(&old, &BTreeMap::new()).unwrap(), true).await.unwrap();
            }
            drop(tables);
            drop(publisher);
            if empty_artifacts {
                std::fs::remove_file(&path).unwrap();
            } else {
                std::fs::write(&path, "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\n---\n\nBody.\n").unwrap();
            }
            let expected = capture_under_workspace_lock(&workspace, CaptureLimits::default(), None)
                .await.unwrap();
            let publisher = bootstrap_test_publisher(&root);
            bootstrap_or_recover(&publisher, &workspace, CaptureLimits::default()).await.unwrap();
            let descriptor = super::super::storage::read_current(&root).unwrap().unwrap();
            let manifest = super::super::storage::read_manifest(&root, &descriptor).unwrap();
            for (role, version, count) in [
                ("artifacts", manifest.table_versions.artifacts, expected.artifacts.len()),
                ("relations", manifest.table_versions.relations, 0),
                ("sections", manifest.table_versions.sections, expected.sections.len()),
            ] {
                let reader = super::super::lance_tables::open_table_at_version(
                    &root, &marker, role, version,
                ).await.unwrap();
                let actual: usize = reader.query_batches().await.unwrap().iter()
                    .map(|batch| batch.num_rows()).sum();
                assert_eq!(actual, count, "{role}, empty_artifacts={empty_artifacts}");
            }
        }
    }

    #[test]
    fn the_first_published_epoch_is_one_not_zero() {
        assert_eq!(first_index_epoch().unwrap(), 1);
    }

    #[test]
    fn legacy_tables_are_named_for_backup_and_never_reopened() {
        assert!(is_legacy_table_name("artifacts"));
        assert!(is_legacy_table_name("relations"));
        assert_eq!(legacy_backup_name("artifacts"), "artifacts.legacy-backup");
        assert!(
            !is_legacy_table_name(crate::constellation::manifest::TABLE_ARTIFACTS_V1),
            "an active table must never be swept up by the legacy backup"
        );
    }

    #[test]
    fn legacy_v5_archive_resumes_after_every_rename_without_losing_ownership() {
        for interrupted_after in 1..=legacy_archive_paths(Path::new("/unused")).len() {
            let tmp = TempDir::new().unwrap();
            let root = tmp.path().canonicalize().unwrap().join("lance");
            std::fs::create_dir_all(&root).unwrap();
            let paths = legacy_archive_paths(&root);
            for (source, _, directory) in &paths {
                if *directory {
                    std::fs::create_dir(source).unwrap();
                    std::fs::write(source.join("payload"), source.to_string_lossy().as_bytes()).unwrap();
                }
            }
            let marker = crate::db::migrate::IdentityV5Marker {
                version: crate::db::migrate::IDENTITY_V5_VERSION,
                artifacts_table: crate::db::migrate::IDENTITY_V5_ARTIFACTS_TABLE.to_string(),
                relations_table: crate::db::migrate::IDENTITY_V5_RELATIONS_TABLE.to_string(),
                evidence_table: crate::db::migrate::IDENTITY_V5_EVIDENCE_TABLE.to_string(),
                source_digest: "a".repeat(64),
                document_count: 1,
                completed: true,
                embeddings: None,
                semantic: None,
            };
            std::fs::write(root.join(crate::db::migrate::IDENTITY_V5_MARKER_FILE), serde_json::to_vec(&marker).unwrap()).unwrap();
            let publisher = IndexPublisher::acquire(&root, Arc::new(object_store::memory::InMemory::new())).unwrap();
            publisher.announce_bootstrap("local-x", 1, 1).unwrap();
            let mut synced = 0;
            let failed = backup_legacy_tables(&root, || {
                synced += 1;
                if synced == interrupted_after {
                    return Err(ConstellationError::Io { detail: "injected sync failure".to_string() });
                }
                Ok(())
            });
            assert!(failed.is_err());
            assert!(matches!(super::storage::read_marker(&root).unwrap(), IndexProtocolState::Initializing(_)));
            let (marker_source, marker_backup, _) = paths.last().unwrap();
            if marker_backup.exists() {
                assert!(paths[..paths.len() - 1].iter().all(|(source, backup, _)| !source.exists() && backup.exists()));
            } else {
                assert!(marker_source.exists());
            }
            backup_legacy_tables(&root, || Ok(())).unwrap();
            backup_legacy_tables(&root, || Ok(())).unwrap();
            for (source, backup, _) in &paths {
                assert!(!source.exists());
                assert!(backup.exists());
            }
        }
    }

    #[test]
    fn legacy_archive_refuses_simultaneous_source_and_backup() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("lance");
        std::fs::create_dir_all(&root).unwrap();
        let paths = legacy_archive_paths(&root);
        std::fs::create_dir(&paths[0].0).unwrap();
        std::fs::create_dir(&paths[0].1).unwrap();
        assert!(backup_legacy_tables(&root, || Ok(())).is_err());
        assert!(paths[0].0.exists() && paths[0].1.exists());
    }
    #[test]
    fn atomic_legacy_archive_never_replaces_a_concurrent_destination() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source");
        let destination = tmp.path().join("destination");
        std::fs::write(&source, b"source").unwrap();
        std::fs::write(&destination, b"unrelated").unwrap();
        assert!(rename_legacy_no_replace(&source, &destination).is_err());
        assert_eq!(std::fs::read(source).unwrap(), b"source");
        assert_eq!(std::fs::read(destination).unwrap(), b"unrelated");
    }


    #[test]
    fn legacy_archive_refuses_a_damaged_v5_marker_before_any_rename() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("lance");
        std::fs::create_dir_all(&root).unwrap();
        let paths = legacy_archive_paths(&root);
        std::fs::create_dir(&paths[0].0).unwrap();
        let marker = root.join(crate::db::migrate::IDENTITY_V5_MARKER_FILE);
        std::fs::write(&marker, b"{broken").unwrap();
        assert!(backup_legacy_tables(&root, || Ok(())).is_err());
        assert!(paths[0].0.exists());
        assert!(!paths[0].1.exists());
        assert!(marker.exists());
    }

    #[test]
    fn a_diagnostic_names_the_unavailable_code() {
        let (code, detail) = unavailable_detail("pointer missing");
        assert_eq!(code, DiagnosticCode::IndexGenerationUnavailable);
        assert_eq!(detail, "pointer missing");
    }

    #[test]
    fn a_generation_id_is_derived_from_its_evaluation_time() {
        let evaluated = "2026-10-17T00:00:00.000Z";
        assert_eq!(
            generation_id_for(evaluated),
            "gen-2026-10-17T00-00-00.000Z",
            "the id becomes a filename verbatim, so NTFS-illegal `:` is              mapped to `-` rather than failing the write on Windows"
        );
    }

    #[test]
    fn an_active_dataset_path_lives_under_the_index_root() {
        let root = Path::new("/ws/.forgeplan/lance");
        assert_eq!(
            active_dataset_path(root, crate::constellation::manifest::TABLE_ARTIFACTS_V1),
            PathBuf::from("/ws/.forgeplan/lance/constellation_artifacts_v1.lance")
        );
    }

    // -------------------------------------------------------------------
    // Commit sequencing
    // -------------------------------------------------------------------

    fn commit_plan(full: bool, creates: usize, updates: usize, deletes: usize) -> CommitPlan {
        CommitPlan {
            creates,
            updates,
            deletes,
            retained: 0,
            full_source_batch: full,
        }
    }

    fn no_relations() -> RelationsPlan {
        RelationsPlan {
            delete_keys: vec![],
            merge_rows: 0,
        }
    }

    fn no_sections() -> SectionsPlan {
        SectionsPlan {
            parent_delete_keys: vec![],
            merge_rows: 0,
            set_changed: false,
        }
    }

    #[test]
    fn a_stage_with_confirmed_deletes_emits_one_bounded_delete_op() {
        let sequence = stage_sequence(
            &commit_plan(true, 0, 0, 1),
            0,
            vec!["PRD-002".to_string(), "PRD-003".to_string()],
            vec![],
            &no_relations(),
            &no_sections(),
        );
        assert_eq!(sequence.commit_count(), 1);
        assert_eq!(
            sequence.ops,
            vec![CommitOp::DeleteByPredicate {
                keys: vec!["PRD-002".to_string(), "PRD-003".to_string()]
            }],
            "the confirmed keys ride on the op, so the predicate is built from \
             exactly what was proved absent"
        );
        assert!(sequence.ops[0].is_executable());
    }

    #[test]
    fn creates_and_updates_emit_a_merge_carrying_the_full_source_gate() {
        let sequence = stage_sequence(
            &commit_plan(true, 1, 1, 0),
            2,
            vec![],
            vec![],
            &no_relations(),
            &no_sections(),
        );
        assert_eq!(sequence.commit_count(), 1);
        assert_eq!(
            sequence.ops,
            vec![CommitOp::MergeUpsert {
                rows: 2,
                full_source_batch: true
            }],
            "the gate is `CommitPlan::full_source_batch`, carried on the op and \
             not recomputed by the executor"
        );
    }

    #[test]
    fn confirmed_deletes_ride_inside_the_merge_not_a_separate_op() {
        // A `DeleteByPredicate` after the upsert deletes the row a rename
        // just wrote — same `canonical_key`, new `relative_path` — and
        // livelocks the publish. With a merge present the source-delete
        // clause is the only delete mechanism, so the plan must be exactly
        // one op carrying the gate.
        let sequence = stage_sequence(
            &commit_plan(true, 1, 0, 1),
            2,
            vec!["PRD-002".into()],
            vec![],
            &no_relations(),
            &no_sections(),
        );
        assert_eq!(
            sequence.ops,
            vec![CommitOp::MergeUpsert {
                rows: 2,
                full_source_batch: true
            }],
            "absent keys are removed by when_not_matched_by_source_delete \
             inside the merge, never by a second canonical-key delete"
        );
    }

    #[test]
    fn relations_ops_follow_the_artifact_ops() {
        let sequence = stage_sequence(
            &commit_plan(true, 1, 0, 1),
            2,
            vec!["PRD-002".into()],
            vec![],
            &RelationsPlan {
                delete_keys: vec!["PRD-001".into(), "PRD-002".into()],
                merge_rows: 3,
            },
            &no_sections(),
        );
        assert_eq!(
            sequence.ops,
            vec![
                CommitOp::MergeUpsert {
                    rows: 2,
                    full_source_batch: true
                },
                CommitOp::RelationsDelete {
                    keys: vec!["PRD-001".into(), "PRD-002".into()]
                },
                CommitOp::RelationsMerge { rows: 3 },
            ],
            "edges are replaced after the rows they point at exist"
        );
        assert_eq!(
            sequence.ops[1].table(),
            CommitTable::Relations,
            "the enum names which table each commit touches"
        );
    }

    #[test]
    fn sections_ops_follow_the_artifact_and_relations_ops() {
        let sequence = stage_sequence(
            &commit_plan(true, 1, 0, 1),
            2,
            vec!["PRD-002".into()],
            vec![],
            &RelationsPlan {
                delete_keys: vec!["PRD-001".into()],
                merge_rows: 3,
            },
            &SectionsPlan {
                parent_delete_keys: vec!["PRD-002".into()],
                merge_rows: 4,
                set_changed: false,
            },
        );
        assert_eq!(
            sequence.ops,
            vec![
                CommitOp::MergeUpsert {
                    rows: 2,
                    full_source_batch: true
                },
                CommitOp::RelationsDelete {
                    keys: vec!["PRD-001".into()]
                },
                CommitOp::RelationsMerge { rows: 3 },
                CommitOp::SectionsDelete {
                    parent_keys: vec!["PRD-002".into()]
                },
                CommitOp::SectionsMerge {
                    rows: 4,
                    full_source_batch: true
                },
            ],
            "sections are replaced after the artifacts they belong to, \
             delete before merge exactly like relations"
        );
        assert_eq!(sequence.ops[4].table(), CommitTable::Sections);
    }

    #[test]
    fn a_noop_artifact_pass_still_populates_sections_when_the_set_changed() {
        // The artifact side staged nothing, but the captured section set
        // differs from the published membership — a layout change, or a
        // capture whose metadata went absent. The table must still be
        // rewritten, or the previous generation's rows stay published under
        // keys that now mean different bytes.
        let sequence = stage_sequence(
            &commit_plan(true, 0, 0, 0),
            0,
            vec![],
            vec![],
            &no_relations(),
            &SectionsPlan {
                parent_delete_keys: vec![],
                merge_rows: 5,
                set_changed: true,
            },
        );
        assert_eq!(
            sequence.ops,
            vec![CommitOp::SectionsMerge {
                rows: 5,
                full_source_batch: true
            }],
            "a changed section set is published even when no artifact row moved — a layout \
             change, or an empty set because metadata went absent, moves the sections and nothing \
             else"
        );

        let same_when_unchanged = stage_sequence(
            &commit_plan(true, 0, 0, 0),
            0,
            vec![],
            vec![],
            &no_relations(),
            &SectionsPlan {
                parent_delete_keys: vec![],
                merge_rows: 5,
                set_changed: false,
            },
        );
        assert!(
            same_when_unchanged.is_empty(),
            "with an unchanged section set a no-op pass emits nothing — the \
             sections merge is gated on real work"
        );
    }

    // -----------------------------------------------------------------------
    // `section_set_changed` itself.
    //
    // The test above consumes a hand-written `SectionsPlan`, so it proves the
    // gate is wired up; it cannot prove the predicate decides correctly. These
    // do that, and the layout case is the one that matters: a capture whose
    // tokenizer or token limit changed holds the **same keys** binding
    // **different bytes**, which no key-set comparison would ever notice.
    // -----------------------------------------------------------------------

    /// A capture whose sections are published, for `section_set_changed`.
    fn captured_with_section() -> CapturedSourceView {
        let mut captured = complete(vec![entry("PRD-001", "prds/PRD-001-a.md", "sd")]);
        captured.artifacts = vec![fixture_artifact(None, None, None)];
        captured.sections = vec![captured_section()];
        captured
    }

    #[test]
    fn an_unpublished_index_always_has_work_to_do_on_the_sections() {
        assert!(
            section_set_changed(&captured_with_section(), None),
            "with nothing published there is nothing to match, so the set has changed"
        );
    }

    #[test]
    fn an_unchanged_capture_reports_no_section_change() {
        let captured = captured_with_section();
        let published =
            GenerationManifest::from_capture(&captured, "gen-1", TableVersions::new(1, 1, 1));
        assert!(
            !section_set_changed(&captured, Some(&published)),
            "the same capture published as itself is the unchanged case; a false `true` here \
             would rewrite the sections table on every pass"
        );
    }

    /// The case a key-set comparison cannot see.
    ///
    /// Identical keys, identical ordinals, identical byte ranges, identical
    /// input digests — and a different layout. The rows mean something else,
    /// because the window that produced them was cut differently, so the
    /// published membership is not the captured one.
    #[test]
    fn a_layout_change_alone_is_a_section_change() {
        let captured = captured_with_section();
        let published =
            GenerationManifest::from_capture(&captured, "gen-1", TableVersions::new(1, 1, 1));

        let mut relaid = captured.clone();
        // A different declared limit resolves to a different actual limit,
        // which is part of the layout's fingerprint.
        relaid.layout = crate::embed::tokenizer::SectionLayout::from_metadata(
            "tok".to_string(),
            "tokcfg".to_string(),
            "cfgjson".to_string(),
            Some(256),
            2,
        );
        assert_ne!(
            captured.layout.fingerprint(),
            relaid.layout.fingerprint(),
            "the fixture must actually change the layout, or this proves nothing"
        );
        assert_eq!(
            captured.sections.len(),
            relaid.sections.len(),
            "the section rows are identical; only the layout they were measured under moved"
        );

        assert!(
            section_set_changed(&relaid, Some(&published)),
            "the same keys under a different layout bind different bytes, so the published \
             membership no longer describes the capture"
        );
    }

    #[test]
    fn a_moved_byte_range_or_changed_input_is_a_section_change() {
        let captured = captured_with_section();
        let published =
            GenerationManifest::from_capture(&captured, "gen-1", TableVersions::new(1, 1, 1));

        let mut moved = captured.clone();
        moved.sections[0].byte_end += 4;
        assert!(
            section_set_changed(&moved, Some(&published)),
            "a window that covers different bytes is a different row under the same key"
        );

        let mut redigested = captured.clone();
        redigested.sections[0].input_digest = "a-different-input".to_string();
        assert!(
            section_set_changed(&redigested, Some(&published)),
            "a different input digest means the stored vector would not answer for these bytes"
        );

        let mut reparented = captured.clone();
        reparented.sections[0].parent_key = "PRD-002".to_string();
        assert!(
            section_set_changed(&reparented, Some(&published)),
            "the parent is part of the row's binding identity, not just its key"
        );

        let mut reordered = captured.clone();
        reordered.sections[0].ordinal = 7;
        assert!(
            section_set_changed(&reordered, Some(&published)),
            "the ordinal is positional identity: the key is derived from it"
        );
    }

    #[test]
    fn a_dropped_or_added_section_is_a_section_change() {
        let captured = captured_with_section();
        let published =
            GenerationManifest::from_capture(&captured, "gen-1", TableVersions::new(1, 1, 1));

        let dropped = complete(vec![entry("PRD-001", "prds/PRD-001-a.md", "sd")]);
        assert!(
            section_set_changed(&dropped, Some(&published)),
            "a capture that published no sections against a generation that published one is a \
             change — this is the `semantic_unavailable` transition"
        );

        let mut added = captured.clone();
        added.sections.push(CapturedSection {
            canonical_key: "PRD-001#s2".to_string(),
            parent_key: "PRD-001".to_string(),
            ordinal: 2,
            byte_start: 99,
            byte_end: 120,
            heading: Some("Goals".to_string()),
            level: Some(2),
            content: Arc::from("goals"),
            input_digest: "another-input".to_string(),
            parent_input_digest: "id".to_string(),
        });
        assert!(
            section_set_changed(&added, Some(&published)),
            "a new window is a new row even when every existing row is untouched"
        );
    }

    #[test]
    fn a_sections_delete_is_emitted_for_confirmed_parent_losses_only() {
        // Delete-all case: the section batch is empty, so no merge exists to
        // carry the source-delete — the parent-key delete is the only way
        // the rows retire.
        let sequence = stage_sequence(
            &commit_plan(true, 0, 0, 1),
            0,
            vec!["PRD-009".into()],
            vec![],
            &no_relations(),
            &SectionsPlan {
                parent_delete_keys: vec!["PRD-009".into()],
                merge_rows: 0,
                set_changed: false,
            },
        );
        assert_eq!(
            sequence.ops,
            vec![
                CommitOp::DeleteByPredicate {
                    keys: vec!["PRD-009".into()]
                },
                CommitOp::SectionsDelete {
                    parent_keys: vec!["PRD-009".into()]
                },
            ],
            "the parent-key delete is the only section retiree when no merge runs"
        );
    }

    #[test]
    fn a_partial_capture_never_gives_the_sections_merge_a_source_gate() {
        // The completeness gate lives on the op: under a partial capture the
        // merge still upserts but may not delete by source — and the
        // parent-key list stays empty because tombstones only exist when the
        // capture proved absence.
        let sequence = stage_sequence(
            &commit_plan(false, 1, 0, 0),
            1,
            vec![],
            vec![],
            &no_relations(),
            &SectionsPlan {
                parent_delete_keys: vec![],
                merge_rows: 5,
                set_changed: false,
            },
        );
        assert_eq!(
            sequence.ops,
            vec![
                CommitOp::MergeUpsert {
                    rows: 1,
                    full_source_batch: false
                },
                CommitOp::SectionsMerge {
                    rows: 5,
                    full_source_batch: false
                },
            ],
            "without a complete capture the merge may upsert but may not delete"
        );
    }

    #[test]
    fn a_partial_capture_never_attaches_the_bulk_delete_clause() {
        let sequence = stage_sequence(
            &commit_plan(false, 1, 0, 0),
            1,
            vec![],
            vec![],
            &no_relations(),
            &no_sections(),
        );
        assert_eq!(
            sequence.ops,
            vec![CommitOp::MergeUpsert {
                rows: 1,
                full_source_batch: false
            }],
            "on a partial scan the bulk clause would delete every row the scan \
             could not see"
        );
    }

    #[test]
    fn a_retired_key_is_deleted_only_when_the_merge_cannot_cover_it() {
        // A re-keyed Update leaves the old key's row behind. Under a full
        // source batch the merge's source-delete removes it — no extra op.
        let full = stage_sequence(
            &commit_plan(true, 0, 1, 0),
            2,
            vec![],
            vec!["prd-alpha".into()],
            &no_relations(),
            &no_sections(),
        );
        assert_eq!(
            full.ops,
            vec![CommitOp::MergeUpsert {
                rows: 2,
                full_source_batch: true
            }],
            "the merge's source-delete already retires the vacated row"
        );

        // Under a partial scan that clause is absent, so the vacated row
        // gets its own bounded delete — the retire is proven by the update
        // delta, not by the `deletes` count (which is zero here).
        let partial = stage_sequence(
            &commit_plan(false, 0, 1, 0),
            2,
            vec![],
            vec!["prd-alpha".into()],
            &no_relations(),
            &no_sections(),
        );
        assert_eq!(
            partial.ops,
            vec![
                CommitOp::MergeUpsert {
                    rows: 2,
                    full_source_batch: false
                },
                CommitOp::DeleteByPredicate {
                    keys: vec!["prd-alpha".to_string()]
                },
            ],
            "a vacated key with no merge coverage would survive forever"
        );
    }

    #[test]
    fn a_stage_that_changes_nothing_emits_no_commits() {
        let sequence = stage_sequence(
            &commit_plan(true, 0, 0, 0),
            0,
            vec![],
            vec![],
            &no_relations(),
            &no_sections(),
        );
        assert!(sequence.is_empty());
        assert_eq!(sequence.commit_count(), 0);
    }

    #[test]
    fn a_merge_with_no_rows_is_not_executable() {
        let op = CommitOp::MergeUpsert {
            rows: 0,
            full_source_batch: true,
        };
        assert!(
            !op.is_executable(),
            "a merge over an empty batch would commit a version that changed nothing"
        );
    }

    #[test]
    fn a_delete_op_with_no_keys_is_not_executable() {
        let op = CommitOp::DeleteByPredicate { keys: vec![] };
        assert!(
            !op.is_executable(),
            "an empty key set would build a predicate matching every row"
        );
        assert!(delete_predicate_for(&op_keys(&op)).is_err());
        assert_eq!(op.table(), CommitTable::Artifacts);
    }

    #[test]
    fn a_delete_op_with_keys_is_executable() {
        let op = CommitOp::DeleteByPredicate {
            keys: vec!["PRD-001".to_string()],
        };
        assert!(op.is_executable());
        assert!(delete_predicate_for(&op_keys(&op)).is_ok());
    }

    #[test]
    fn the_merge_key_is_a_column_the_active_schema_declares() {
        let schema = super::super::lance_tables::artifacts_v1_schema();
        for key in MERGE_KEY {
            assert!(
                schema.field_with_name(key).is_ok(),
                "`{key}` must exist in the artifacts schema or the merge is keyed \
                 against nothing"
            );
        }
    }

    #[test]
    fn version_sources_are_distinguishable() {
        assert_eq!(VersionSource::Reported.as_str(), "reported");
        assert_eq!(VersionSource::ReadFromTable.as_str(), "read_from_table");
        assert_ne!(VersionSource::Reported, VersionSource::ReadFromTable);
    }

    #[test]
    fn a_source_with_no_created_at_produces_a_null_not_a_timestamp() {
        let captured = vec![fixture_artifact(None, None, None)];
        let batch = artifacts_batch(&captured, 1, "local-test", &BTreeMap::new()).unwrap();
        let column = batch
            .column_by_name("created_at")
            .expect("created_at column");
        assert_eq!(
            column.null_count(),
            1,
            "an absent timestamp must stay null; fabricating `now` would make \
             every capture look like an edit"
        );
    }

    #[test]
    fn a_source_with_created_at_produces_exactly_that_value() {
        let captured = vec![fixture_artifact(
            Some("2026-10-01T09:00:00.000Z"),
            None,
            None,
        )];
        let batch = artifacts_batch(&captured, 1, "local-test", &BTreeMap::new()).unwrap();
        let column = batch
            .column_by_name("created_at")
            .expect("created_at column")
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .expect("created_at is a string column");
        assert_eq!(column.null_count(), 0);
        assert_eq!(column.value(0), "2026-10-01T09:00:00.000Z");
    }

    #[test]
    fn valid_until_and_updated_at_follow_the_same_absence_rule() {
        let captured = vec![fixture_artifact(
            Some("2026-10-01T09:00:00.000Z"),
            None,
            None,
        )];
        let batch = artifacts_batch(&captured, 1, "local-test", &BTreeMap::new()).unwrap();
        for name in ["valid_until", "updated_at"] {
            assert_eq!(
                batch.column_by_name(name).unwrap().null_count(),
                1,
                "`{name}` must stay null when the source does not declare it"
            );
        }
    }

    #[test]
    fn embedding_is_null_and_semantic_state_is_unavailable_for_a_fresh_capture() {
        let captured = vec![fixture_artifact(None, None, None)];
        let batch = artifacts_batch(&captured, 1, "local-test", &BTreeMap::new()).unwrap();

        let embedding = batch.column_by_name("embedding").expect("embedding column");
        assert_eq!(
            embedding.null_count(),
            1,
            "a zero-filled FixedSizeListArray is indistinguishable from a real \
             vector to every reader"
        );

        let state = batch
            .column_by_name("semantic_state")
            .expect("semantic_state column")
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .expect("semantic_state is a string column");
        assert_eq!(
            state.value(0),
            "unavailable",
            "no profile is resolved until P3; `ready` and `failed` would both lie"
        );
        assert_eq!(batch.num_rows(), 1);
    }

    #[test]
    fn the_batch_schema_equals_the_active_artifacts_schema() {
        let batch = artifacts_batch(
            &[fixture_artifact(None, None, None)],
            1,
            "local-test",
            &BTreeMap::new(),
        )
        .unwrap();
        let expected = super::super::lance_tables::artifacts_v1_schema();

        let batch_schema = batch.schema();
        let actual: Vec<(&str, &arrow_schema::DataType)> = batch_schema
            .fields()
            .iter()
            .map(|f| (f.name().as_str(), f.data_type()))
            .collect();
        let wanted: Vec<(&str, &arrow_schema::DataType)> = expected
            .fields()
            .iter()
            .map(|f| (f.name().as_str(), f.data_type()))
            .collect();

        assert_eq!(
            actual, wanted,
            "field names, types **and order** must match; merge_insert rejects a \
             batch whose columns differ from the table"
        );
    }

    #[test]
    fn zero_rows_produces_an_empty_batch_with_the_right_schema() {
        let batch = artifacts_batch(&[], 1, "local-test", &BTreeMap::new()).unwrap();
        assert_eq!(batch.num_rows(), 0);
        assert_eq!(
            batch.num_columns(),
            super::super::lance_tables::artifacts_v1_schema()
                .fields()
                .len(),
            "the count is derived from the schema, not restated — a literal              here is how batch and schema drift apart"
        );
        assert_eq!(
            batch.schema(),
            super::super::lance_tables::artifacts_v1_schema()
        );
    }

    #[test]
    fn diagnostic_codes_are_joined_into_the_diagnostics_column() {
        use super::super::DiagnosticCode;
        let mut artifact = fixture_artifact(None, None, None);
        Arc::get_mut(&mut artifact).unwrap().diagnostics = vec![
            super::super::SourceDiagnostic::new(DiagnosticCode::InvalidLinks, None, "a"),
            super::super::SourceDiagnostic::new(DiagnosticCode::UnreadableSource, None, "b"),
        ];
        let batch = artifacts_batch(&[artifact], 1, "local-test", &BTreeMap::new()).unwrap();
        let column = batch
            .column_by_name("diagnostics")
            .expect("diagnostics column")
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .expect("diagnostics is a string column");
        assert_eq!(column.value(0), "invalid_links,unreadable_source");
    }

    // -------------------------------------------------------------------
    // Semantic preservation — what a reconcile splices and what it clears
    // -------------------------------------------------------------------

    /// A stored tuple as `artifact_semantics` would have read it back: the
    /// row is `ready` under profile `p1`, with a real vector and a retry the
    /// batch must carry forward verbatim.
    fn stored_ready() -> SemanticColumns {
        SemanticColumns {
            index_epoch: Some(7),
            checkout_id: Some("local-x".to_string()),
            incarnation: Some(1),
            input_digest: Some("id".to_string()),
            embedding: Some(vec![
                0.25;
                super::super::lance_tables::EMBEDDING_DIM as usize
            ]),
            profile_id: Some("p1".to_string()),
            semantic_state: Some("ready".to_string()),
            retry_after: Some("2026-10-20T00:00:00Z".to_string()),
        }
    }

    fn preserved_for(
        artifact: &super::super::capture::ParsedArtifact,
        columns: SemanticColumns,
    ) -> BTreeMap<String, SemanticColumns> {
        let mut map = BTreeMap::new();
        map.insert(artifact.canonical_key.clone(), columns);
        map
    }

    fn embedding_of(batch: &arrow_array::RecordBatch, row: usize) -> Option<Vec<f32>> {
        let column = batch.column_by_name("embedding").expect("embedding column");
        if column.is_null(row) {
            return None;
        }
        let list = column
            .as_any()
            .downcast_ref::<arrow_array::FixedSizeListArray>()
            .expect("embedding is a fixed-size list");
        let values = list.value(row);
        let floats = values
            .as_any()
            .downcast_ref::<arrow_array::Float32Array>()
            .expect("list items are f32");
        Some((0..floats.len()).map(|i| floats.value(i)).collect())
    }

    fn text_of(batch: &arrow_array::RecordBatch, name: &str, row: usize) -> Option<String> {
        let column = batch.column_by_name(name).expect(name);
        if column.is_null(row) {
            return None;
        }
        Some(
            column
                .as_any()
                .downcast_ref::<arrow_array::StringArray>()
                .expect("a string column")
                .value(row)
                .to_string(),
        )
    }

    /// The load-bearing Wave 2 case: a metadata-only edit moves
    /// `source_digest` and `row_revision` but not the input, so the stored
    /// tuple must survive while the durable columns take the capture's new
    /// values.
    #[test]
    fn a_metadata_only_edit_keeps_the_stored_tuple_and_takes_the_new_source_digest() {
        let artifact = fixture_artifact(None, None, None);
        let stored = stored_ready();

        // A frontmatter edit: new bytes, new revision — same path, so the same
        // incarnation, and the same title+body, so the same input digest.
        let mut edited = (*artifact).clone();
        edited.status = "active".to_string();
        edited.source_digest = "sd-after-edit".to_string();
        edited.row_revision = 2;

        let batch = artifacts_batch(
            &[Arc::new(edited)],
            7,
            "local-x",
            &preserved_for(&artifact, stored.clone()),
        )
        .unwrap();

        assert_eq!(
            embedding_of(&batch, 0),
            stored.embedding,
            "an unchanged input must keep the exact vector, not a fresh null or a zero fill"
        );
        assert_eq!(text_of(&batch, "profile_id", 0).as_deref(), Some("p1"));
        assert_eq!(
            text_of(&batch, "semantic_state", 0).as_deref(),
            Some("ready")
        );
        assert_eq!(
            text_of(&batch, "retry_after", 0).as_deref(),
            Some("2026-10-20T00:00:00Z"),
            "the retry schedule is part of the tuple and survives too"
        );
        assert_eq!(
            text_of(&batch, "source_digest", 0).as_deref(),
            Some("sd-after-edit"),
            "the durable source binding comes from the capture, never from the stored row"
        );
        assert_eq!(
            text_of(&batch, "input_digest", 0).as_deref(),
            Some("id"),
            "the input the vector was bound to is unchanged, and is written as captured"
        );
    }

    /// A recreate bumps the incarnation. The vector described a document that
    /// no longer exists under this name, so the whole tuple goes — and the
    /// row must say `unavailable` rather than inherit the capture's own
    /// value, which is what makes the invalidation explicit.
    #[test]
    fn a_new_incarnation_clears_the_whole_tuple() {
        let artifact = fixture_artifact(None, None, None);
        let stored = stored_ready();

        let mut recreated = (*artifact).clone();
        recreated.incarnation = 2;
        recreated.row_revision = 1;
        recreated.source_digest = "sd-recreated".to_string();

        let batch = artifacts_batch(
            &[Arc::new(recreated)],
            7,
            "local-x",
            &preserved_for(&artifact, stored),
        )
        .unwrap();

        assert_eq!(
            embedding_of(&batch, 0),
            None,
            "a recreated document keeps no vector — and never a zero vector"
        );
        assert_eq!(text_of(&batch, "profile_id", 0), None);
        assert_eq!(text_of(&batch, "retry_after", 0), None);
        assert_eq!(
            text_of(&batch, "semantic_state", 0).as_deref(),
            Some("unavailable"),
            "a cleared row says unavailable, not ready"
        );
        assert_eq!(
            text_of(&batch, "source_digest", 0).as_deref(),
            Some("sd-recreated"),
            "the new bytes are recorded even though the vector was dropped"
        );
    }

    /// The body moved, so the tokenizer input moved with it.
    #[test]
    fn a_changed_input_digest_clears_the_whole_tuple() {
        let artifact = fixture_artifact(None, None, None);
        let mut edited = (*artifact).clone();
        edited.input_digest = "id-after-edit".to_string();
        edited.source_digest = "sd-after-edit".to_string();

        let batch = artifacts_batch(
            &[Arc::new(edited)],
            7,
            "local-x",
            &preserved_for(&artifact, stored_ready()),
        )
        .unwrap();

        assert_eq!(embedding_of(&batch, 0), None);
        assert_eq!(text_of(&batch, "profile_id", 0), None);
        assert_eq!(
            text_of(&batch, "semantic_state", 0).as_deref(),
            Some("unavailable")
        );
    }

    /// A rebind bumps the epoch, which retires every pending job. The same
    /// tuple fences the guarded update, so it must fence the splice.
    #[test]
    fn a_bumped_epoch_or_a_moved_checkout_clears_the_whole_tuple() {
        let artifact = fixture_artifact(None, None, None);

        for (epoch, checkout) in [(8u64, "local-x"), (7u64, "local-y")] {
            let batch = artifacts_batch(
                &[Arc::clone(&artifact)],
                epoch,
                checkout,
                &preserved_for(&artifact, stored_ready()),
            )
            .unwrap();
            assert_eq!(
                embedding_of(&batch, 0),
                None,
                "epoch {epoch} at checkout {checkout} is a different generation"
            );
            assert_eq!(text_of(&batch, "profile_id", 0), None);
        }
    }

    /// A key the table does not hold gets no splice, even when the stored
    /// tuple would otherwise match.
    #[test]
    fn an_absent_key_gets_nulls_rather_than_a_borrowed_tuple() {
        let artifact = fixture_artifact(None, None, None);
        let mut other = (*artifact).clone();
        other.canonical_key = "PRD-002".to_string();

        let batch = artifacts_batch(
            &[Arc::new(other)],
            7,
            "local-x",
            &preserved_for(&artifact, stored_ready()),
        )
        .unwrap();
        assert_eq!(embedding_of(&batch, 0), None);
        assert_eq!(text_of(&batch, "profile_id", 0), None);
    }

    /// The same law on the sections side: a section is judged by its own
    /// input digest and its parent's identity, so a parent's metadata edit
    /// leaves every section vector alone.
    #[test]
    fn a_section_keeps_its_vector_when_only_its_parent_metadata_moved() {
        let mut captured = complete(vec![entry("PRD-001", "prds/PRD-001-a.md", "sd-before")]);
        captured.artifacts = vec![fixture_artifact(None, None, None)];
        captured.sections = vec![super::super::capture::CapturedSection {
            canonical_key: "PRD-001#s0".to_string(),
            parent_key: "PRD-001".to_string(),
            ordinal: 0,
            byte_start: 0,
            byte_end: "Body.\n".len(),
            heading: None,
            level: None,
            content: Arc::from("Body.\n"),
            input_digest: "section-in".to_string(),
            parent_input_digest: "id".to_string(),
        }];
        // The capture's durable half now describes the edited bytes. The
        // section's own input digest is untouched.
        captured.entries[0].source_digest = "sd-after".to_string();
        captured.entries[0].row_revision = 2;

        let mut stored = stored_ready();
        // A section row carries its **parent's** epoch/checkout/incarnation,
        // and the `entry` fixture publishes under epoch 1 / `local-x`.
        stored.index_epoch = Some(1);
        stored.input_digest = Some("section-in".to_string());
        let mut preserved = BTreeMap::new();
        preserved.insert("PRD-001#s0".to_string(), stored.clone());

        let batch = sections_batch(&captured, &preserved).unwrap();

        assert_eq!(
            embedding_of(&batch, 0),
            stored.embedding,
            "the section's text did not move, so its vector stands"
        );
        assert_eq!(text_of(&batch, "profile_id", 0).as_deref(), Some("p1"));
        assert_eq!(
            text_of(&batch, "semantic_state", 0).as_deref(),
            Some("ready")
        );
        assert_eq!(
            text_of(&batch, "source_digest", 0).as_deref(),
            Some("sd-after"),
            "the section row takes its parent's new source digest wholesale"
        );
    }

    /// The parent's identity moving — a recreate — retires the section's
    /// vector too, because the section row is bound to its parent.
    #[test]
    fn a_new_parent_incarnation_clears_the_section_tuple() {
        let mut captured = complete(vec![entry("PRD-001", "prds/PRD-001-a.md", "sd")]);
        captured.entries[0].incarnation = 2;
        captured.artifacts = vec![fixture_artifact(None, None, None)];
        captured.sections = vec![super::super::capture::CapturedSection {
            canonical_key: "PRD-001#s0".to_string(),
            parent_key: "PRD-001".to_string(),
            ordinal: 0,
            byte_start: 0,
            byte_end: "Body.\n".len(),
            heading: None,
            level: None,
            content: Arc::from("Body.\n"),
            input_digest: "section-in".to_string(),
            parent_input_digest: "id".to_string(),
        }];

        let mut stored = stored_ready();
        stored.index_epoch = Some(1);
        stored.input_digest = Some("section-in".to_string());
        let mut preserved = BTreeMap::new();
        preserved.insert("PRD-001#s0".to_string(), stored);

        let batch = sections_batch(&captured, &preserved).unwrap();
        assert_eq!(embedding_of(&batch, 0), None);
        assert_eq!(text_of(&batch, "profile_id", 0), None);
        assert_eq!(
            text_of(&batch, "semantic_state", 0).as_deref(),
            Some("unavailable"),
            "a section never inherits its parent's state; the parent may be ready \
             on the body prefix and that says nothing about this section"
        );
    }

    fn fixture_artifact(
        created_at: Option<&str>,
        updated_at: Option<&str>,
        valid_until: Option<&str>,
    ) -> Arc<super::super::capture::ParsedArtifact> {
        Arc::new(super::super::capture::ParsedArtifact {
            canonical_key: "PRD-001".to_string(),
            display_id: "PRD-001".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Auth".to_string(),
            body: Arc::from("Body.\n"),
            depth: Some("standard".to_string()),
            author: None,
            parent_epic: None,
            valid_until: valid_until.map(str::to_string),
            created_at: created_at.map(str::to_string),
            updated_at: updated_at.map(str::to_string),
            tags: vec!["source=code".to_string()],
            identity: Default::default(),
            frontmatter: Default::default(),
            relative_path: "prds/PRD-001-auth.md".to_string(),
            outgoing: Default::default(),
            raw_bytes: Arc::from(&b"x"[..]),
            source_digest: "sd".to_string(),
            input_digest: "id".to_string(),
            profile_id: None,
            semantic_state: super::super::jobs::SemanticState::Unavailable,
            incarnation: 1,
            row_revision: 1,
            diagnostics: Vec::new(),
        })
    }

    #[test]
    fn a_column_update_predicate_is_bounded_and_refuses_an_empty_key_set() {
        let predicate =
            update_predicate_for(&["PRD-001".to_string(), "O'Brien".to_string()]).unwrap();
        assert_eq!(
            predicate, "canonical_key IN ('PRD-001', 'O''Brien')",
            "the same escaped enumeration the delete uses; never concatenated user SQL"
        );
        let err = update_predicate_for(&[]).unwrap_err();
        assert_eq!(err.code(), DiagnosticCode::StorageIo);
    }

    #[test]
    fn a_column_update_op_with_no_keys_is_not_executable() {
        let op = CommitOp::ColumnUpdate {
            keys: vec![],
            assignments: vec![ColumnAssignment {
                column: "status".to_string(),
                expression: "'active'".to_string(),
            }],
        };
        assert!(!op.is_executable());
    }

    #[test]
    fn a_column_update_op_with_no_assignments_is_not_executable() {
        // Lance rejects a columnless update as invalid input
        // (`table/update.rs:66-69`); refusing here names the cause early.
        let op = CommitOp::ColumnUpdate {
            keys: vec!["PRD-001".to_string()],
            assignments: vec![],
        };
        assert!(!op.is_executable());
    }

    #[test]
    fn a_column_update_op_with_keys_and_assignments_is_executable() {
        let op = CommitOp::ColumnUpdate {
            keys: vec!["PRD-001".to_string()],
            assignments: vec![ColumnAssignment {
                column: "status".to_string(),
                expression: "'active'".to_string(),
            }],
        };
        assert!(op.is_executable());
        assert_eq!(op.table(), CommitTable::Artifacts);
    }

    #[test]
    fn every_named_verification_error_names_its_own_invariant() {
        let named = [
            VerificationError::RowMissing {
                canonical_key: "PRD-001".into(),
            },
            VerificationError::DigestDisagreement {
                canonical_key: "PRD-001".into(),
                expected: "want".into(),
                found: "got".into(),
            },
            VerificationError::UnexpectedRow {
                canonical_key: "PRD-002".into(),
            },
            VerificationError::VersionMismatch {
                expected: 3,
                found: 4,
            },
            VerificationError::SchemaMismatch {
                detail: "column order differs".into(),
            },
            VerificationError::Query {
                detail: "stream failed".into(),
            },
        ];
        // Each names its own invariant, so a reader can tell which rule broke
        // without parsing prose.
        let prefixes = [
            "row_missing",
            "digest_disagreement",
            "unexpected_row",
            "version_mismatch",
            "schema_mismatch",
            "verify_query_failed",
        ];
        for (error, prefix) in named.iter().zip(prefixes) {
            assert!(
                error.to_string().starts_with(prefix),
                "{error} should announce `{prefix}`"
            );
        }
    }

    #[test]
    fn a_digest_disagreement_names_the_expected_and_found_values() {
        let text = VerificationError::DigestDisagreement {
            canonical_key: "PRD-001".into(),
            expected: "abc".into(),
            found: "xyz".into(),
        }
        .to_string();
        assert!(text.contains("abc"), "{text}");
        assert!(text.contains("xyz"), "{text}");
    }

    #[test]
    fn a_version_mismatch_names_both_versions() {
        let text = VerificationError::VersionMismatch {
            expected: 3,
            found: 4,
        }
        .to_string();
        assert!(text.contains('3') && text.contains('4'), "{text}");
    }

    #[test]
    fn a_missing_pinned_version_is_unavailable_not_an_invariant_break() {
        let failure = VerifyFailure::Unavailable("no version 9".into());
        assert!(
            failure
                .to_string()
                .starts_with("index_generation_unavailable")
        );
        assert!(matches!(failure, VerifyFailure::Unavailable(_)));
    }

    #[test]
    fn an_already_existing_table_is_reported_as_its_own_variant_not_as_a_failure() {
        let error = ConstellationError::TableAlreadyExists {
            role: "artifacts".to_string(),
        };
        assert_eq!(error.code(), DiagnosticCode::IndexProtocolMismatch);
        let text = error.to_string();
        assert!(text.contains("restart transition"), "{text}");
        assert!(
            !text.contains("storage_io"),
            "a restart must never read as a storage failure: {text}"
        );
    }

    #[test]
    fn bootstrap_progress_distinguishes_creation_from_adoption() {
        let created = BootstrapProgress::CreatedTables {
            generation_id: "gen-1".into(),
        };
        let adopted = BootstrapProgress::RestartedExistingTables {
            generation_id: "gen-1".into(),
        };
        assert_ne!(
            created, adopted,
            "a first-time bootstrap and an adopted restart are different facts, \
             even when the generation id matches"
        );
        assert_eq!(
            BootstrapProgress::FinishedActivation {
                generation_id: "gen-0".into()
            },
            BootstrapProgress::FinishedActivation {
                generation_id: "gen-0".into()
            }
        );
    }

    #[test]
    fn legacy_backup_never_names_an_active_table() {
        for legacy in crate::constellation::manifest::LEGACY_TABLES {
            let backup = legacy_backup_name(legacy);
            assert!(backup.ends_with(".legacy-backup"), "{backup}");
        }
        assert!(
            !legacy_backup_name(crate::constellation::manifest::TABLE_ARTIFACTS_V1)
                .contains("constellation")
        );
    }

    #[test]
    fn a_legacy_table_is_recognised_and_an_active_one_is_not() {
        assert!(is_legacy_table_name("artifacts"));
        assert!(is_legacy_table_name("relations"));
        assert!(!is_legacy_table_name(
            crate::constellation::manifest::TABLE_ARTIFACTS_V1
        ));
        assert!(!is_legacy_table_name(
            crate::constellation::manifest::TABLE_RELATIONS_V1
        ));
    }

    fn op_keys(op: &CommitOp) -> Vec<String> {
        match op {
            CommitOp::DeleteByPredicate { keys } => keys.clone(),
            CommitOp::RelationsDelete { keys } => keys.clone(),
            CommitOp::SectionsDelete { parent_keys } => parent_keys.clone(),
            CommitOp::ColumnUpdate { keys, .. } => keys.clone(),
            CommitOp::MergeUpsert { .. }
            | CommitOp::RelationsMerge { .. }
            | CommitOp::SectionsMerge { .. } => Vec::new(),
        }
    }

    #[test]
    fn predicate_literals_are_escaped_and_delimited() {
        assert_eq!(escape_predicate_literal("PRD-001"), "'PRD-001'");
        assert_eq!(
            escape_predicate_literal("O'Brien"),
            "'O''Brien'",
            "an unescaped quote in the data would become structure in the predicate"
        );
    }

    #[test]
    fn a_delete_predicate_is_bounded_and_never_a_wildcard() {
        let predicate =
            delete_predicate_for(&["PRD-001".to_string(), "O'Brien".to_string()]).unwrap();
        assert_eq!(
            predicate, "canonical_key IN ('PRD-001', 'O''Brien')",
            "a fixed enumeration, not a pattern that could match unconfirmed rows"
        );
        assert!(!predicate.contains('*'));
    }

    #[test]
    fn an_empty_delete_predicate_is_refused_rather_than_matching_every_row() {
        let err = delete_predicate_for(&[]).unwrap_err();
        assert_eq!(err.code(), DiagnosticCode::StorageIo);
        assert!(err.to_string().contains("every row"), "{err}");
    }

    fn source_entry(input_digest: Option<&str>) -> super::super::capture::SourceEntry {
        super::super::capture::SourceEntry {
            canonical_key: "PRD-001".to_string(),
            relative_path: "prds/PRD-001-auth.md".to_string(),
            checkout_id: "local-a".to_string(),
            index_epoch: 5,
            incarnation: 2,
            row_revision: 7,
            source_digest: "sd".to_string(),
            input_digest: input_digest.map(str::to_string),
            profile_id: None,
            semantic_state: super::super::jobs::SemanticState::Unavailable,
            diagnostic_codes: Vec::new(),
            observed_at: "2026-10-17T00:00:00.000Z".to_string(),
        }
    }

    #[test]
    fn a_binding_takes_every_field_from_the_capture_entry() {
        let binding = binding_for(&source_entry(Some("id")), "pid").unwrap();
        assert_eq!(binding.index_epoch, 5);
        assert_eq!(binding.checkout_id, "local-a");
        assert_eq!(binding.canonical_key, "PRD-001");
        assert_eq!(binding.incarnation, 2);
        assert_eq!(binding.row_revision, 7);
        assert_eq!(binding.source_digest, "sd");
        assert_eq!(binding.input_digest, "id");
        assert_eq!(
            binding.profile_id, "pid",
            "the profile id is the one field the capture cannot know"
        );
        assert_eq!(
            binding.predicate().matches("'PRD-001'").count(),
            1,
            "the captured key must appear as a predicate term"
        );
    }

    #[test]
    fn a_captured_row_without_an_input_digest_has_no_binding_to_publish_against() {
        let err = binding_for(&source_entry(None), "pid").unwrap_err();
        assert!(
            err.to_string().contains("no binding"),
            "publishing without an input digest is the bypass in another name: {err}"
        );
    }

    fn captured_section() -> super::super::capture::CapturedSection {
        super::super::capture::CapturedSection {
            canonical_key: "PRD-001#s1".to_string(),
            parent_key: "PRD-001".to_string(),
            ordinal: 1,
            byte_start: 0,
            byte_end: "section body".len(),
            heading: Some("Problem".to_string()),
            level: Some(2),
            content: std::sync::Arc::from("section body"),
            input_digest: "section-input-digest".to_string(),
            parent_input_digest: "id".to_string(),
        }
    }

    #[test]
    fn a_section_binding_borrows_the_parent_and_keeps_its_own_key_and_digest() {
        let parent = source_entry(Some("id"));
        let binding = binding_for_section(&captured_section(), &parent, "pid").unwrap();

        // Durable identity comes from the parent entry wholesale.
        assert_eq!(binding.index_epoch, 5);
        assert_eq!(binding.checkout_id, "local-a");
        assert_eq!(binding.incarnation, 2);
        assert_eq!(binding.row_revision, 7);
        assert_eq!(binding.source_digest, "sd");
        // But the row identity and the input digest are the section's own —
        // publishing under the parent's `input_digest` would bind a vector to
        // bytes it was never computed from.
        assert_eq!(binding.canonical_key, "PRD-001#s1");
        assert_eq!(binding.input_digest, "section-input-digest");
        assert_ne!(
            binding.input_digest,
            parent.input_digest.unwrap(),
            "the section digest is the section input's hash, never the parent's"
        );
        assert_eq!(binding.profile_id, "pid");
        assert_eq!(
            binding.predicate().matches("'PRD-001#s1'").count(),
            1,
            "the predicate binds the section key so a stale parent move still refuses the write"
        );
    }

    // -------------------------------------------------------------------
    // republish_current durability barrier (C04)
    // -------------------------------------------------------------------

    /// A recording barrier: delegates the real fsyncs to the POSIX adapter
    /// and keeps the call order, optionally refusing one file's sync or the
    /// directory ACK that follows the `CURRENT` rename. The order log is the
    /// assertion — a republish that moves `CURRENT` before the tracked
    /// writes are durable is visible as a `replace` earlier than the
    /// `sync_file`.
    #[derive(Debug, Default)]
    struct SpyDurability {
        calls: std::sync::Mutex<Vec<String>>,
        fail_sync_on: std::sync::Mutex<Option<String>>,
        /// Refuse every `sync_dir` issued **after** `CURRENT.json` has been
        /// renamed into place.
        ///
        /// Gated on the rename rather than on a path so it lands on exactly
        /// one call: the closure barrier's own `sync_dir(index_root)` runs
        /// before the visibility point and must still succeed, and a spy
        /// that failed every `sync_dir` would abort before the rename and
        /// prove nothing about the ACK. This is the "the pointer is visible
        /// but its directory entry was never acknowledged" case.
        fail_dir_sync_after_current: bool,
    }

    impl SpyDurability {
        fn calls(&self) -> Vec<String> {
            self.log().clone()
        }

        /// The recorded call order.
        ///
        /// Takes the guard by value so no `lock().unwrap()` sits here: a
        /// poisoned test-only log must not turn a durability assertion into
        /// an unrelated panic.
        fn log(&self) -> std::sync::MutexGuard<'_, Vec<String>> {
            self.calls.lock().unwrap_or_else(|e| e.into_inner())
        }

        fn record(&self, what: &str, path: &std::path::Path) {
            self.log().push(format!("{what}:{}", path.display()));
        }

        /// Arm a file-sync refusal, after any publication that must succeed
        /// first. Construction-time arming cannot express "publish the old
        /// generation, then fail", which is the state the barrier has to
        /// preserve.
        fn fail_sync_of(&self, needle: &str) {
            let mut armed = self.fail_sync_on.lock().unwrap_or_else(|e| e.into_inner());
            *armed = Some(needle.to_string());
        }

        fn refuses(&self, path: &std::path::Path) -> bool {
            self.fail_sync_on
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_deref()
                .is_some_and(|needle| path.display().to_string().contains(needle))
        }

        fn current_is_renamed(&self) -> bool {
            self.current_renames() > 0
        }

        /// How many times `CURRENT.json` has been renamed into place.
        ///
        /// A count rather than a flag: "the failed republish left the pointer
        /// alone" is only meaningful against the one rename the *successful*
        /// publication already made, and a boolean cannot tell those apart.
        fn current_renames(&self) -> usize {
            self.log()
                .iter()
                .filter(|call| {
                    call.starts_with("replace:") && call.contains(storage::CURRENT_FILE_NAME)
                })
                .count()
        }
    }

    impl super::super::durability::PlatformDurability for SpyDurability {
        fn adapter_name(&self) -> &'static str {
            "spy-durability"
        }

        fn sync_file(&self, path: &std::path::Path) -> Result<(), ConstellationError> {
            self.record("sync_file", path);
            if self.refuses(path) {
                return Err(ConstellationError::DurabilityUnavailable {
                    detail: format!("spy refused the sync of {}", path.display()),
                });
            }
            super::super::durability::PosixDurability.sync_file(path)
        }

        fn sync_dir(&self, path: &std::path::Path) -> Result<(), ConstellationError> {
            self.record("sync_dir", path);
            if self.fail_dir_sync_after_current && self.current_is_renamed() {
                return Err(ConstellationError::DurabilityUnavailable {
                    detail: format!(
                        "spy refused the directory ACK of {} after CURRENT became visible",
                        path.display()
                    ),
                });
            }
            super::super::durability::PosixDurability.sync_dir(path)
        }

        fn ensure_dir(&self, dir: &std::path::Path) -> Result<(), ConstellationError> {
            self.record("ensure_dir", dir);
            super::super::durability::PosixDurability.ensure_dir(dir)
        }

        fn replace_atomically(
            &self,
            src: &std::path::Path,
            dest: &std::path::Path,
        ) -> Result<(), ConstellationError> {
            self.record("replace", dest);
            super::super::durability::PosixDurability.replace_atomically(src, dest)
        }
    }

    fn spy_publisher(index_root: &std::path::Path, spy: &Arc<SpyDurability>) -> IndexPublisher {
        let mut publisher =
            IndexPublisher::acquire(index_root, Arc::new(object_store::memory::InMemory::new()))
                .unwrap();
        publisher.platform =
            Arc::clone(spy) as Arc<dyn super::super::durability::PlatformDurability>;
        publisher
    }

    /// C04: `republish_current` must run the same closure barrier `publish`
    /// runs — finish the tracked writes, sync that exact set, and only then
    /// move `CURRENT`. Before the fix the drain's Lance writes were tracked
    /// but never fsynced before the pointer moved.
    #[test]
    fn republish_current_syncs_the_write_closure_before_moving_the_pointer() {
        let tmp = TempDir::new().unwrap();
        // Canonicalise before the publisher sees the path: on macOS a temp dir
        // is reached through `/var/...`, which resolves to `/private/var/...`,
        // and the publisher refuses a symlinked component by design — a
        // symlinked alias would create a second physical owner. The refusal is
        // correct; handing it an alias from a fixture is not.
        let index_root = tmp.path().canonicalize().unwrap().join("lance");
        // A file the stage pretends a table commit wrote, at a path inside
        // the index root so the closure barrier may sync it.
        let touched = index_root.join("constellation_artifacts_v1.lance/data/0.bin");
        std::fs::create_dir_all(touched.parent().unwrap()).unwrap();
        std::fs::write(&touched, b"payload").unwrap();

        let spy = Arc::new(SpyDurability::default());
        let publisher = spy_publisher(&index_root, &spy);

        // Tracked keys are filesystem-root-relative: the object store
        // strips the leading `/` from the absolute path it was opened
        // against.
        let key = touched
            .to_str()
            .unwrap()
            .trim_start_matches('/')
            .to_string();
        publisher.tracker().register_intent(&key);
        publisher.tracker().record_write(&key);

        publisher
            .republish_current(&descriptor_for("gen-1"))
            .unwrap();

        let calls = spy.calls();
        let synced = calls
            .iter()
            .position(|call| call.starts_with("sync_file:") && call.contains("0.bin"))
            .expect(
                "the tracked table write must be fsynced before the pointer \
                 moves: a republish that skips the closure is the C04 hole",
            );
        let renamed = calls
            .iter()
            .position(|call| call.starts_with("replace:") && call.contains("CURRENT.json"))
            .expect("the CURRENT rename is the visibility point");
        assert!(
            synced < renamed,
            "the write closure must be durable before CURRENT moves: {calls:?}"
        );
        assert!(
            index_root.join("CURRENT.json").exists(),
            "the republish itself must still land"
        );
    }

    /// The other half of the barrier: when the closure sync fails, the
    /// pointer must not move — and the **previously published** pointer, not
    /// merely the absence of one, is what must survive. The barrier sits in
    /// front of an index that already serves a generation, so "no CURRENT"
    /// would be the wrong assertion: a drain's failed republish must leave
    /// the previous generation readable, byte for byte.
    #[test]
    fn a_failed_closure_sync_preserves_the_previously_published_pointer() {
        let tmp = TempDir::new().unwrap();
        // Canonicalise before the publisher sees the path: on macOS a temp dir
        // is reached through `/var/...`, which resolves to `/private/var/...`,
        // and the publisher refuses a symlinked component by design — a
        // symlinked alias would create a second physical owner. The refusal is
        // correct; handing it an alias from a fixture is not.
        let index_root = tmp.path().canonicalize().unwrap().join("lance");
        let touched = index_root.join("constellation_artifacts_v1.lance/data/0.bin");
        std::fs::create_dir_all(touched.parent().unwrap()).unwrap();
        std::fs::write(&touched, b"payload").unwrap();

        let spy = Arc::new(SpyDurability::default());
        let publisher = spy_publisher(&index_root, &spy);

        // The generation that was already serving reads, published through
        // the same publisher so its bytes are what a reader would have seen.
        publisher
            .republish_current(&descriptor_for("gen-1"))
            .unwrap();
        let published_bytes = std::fs::read(index_root.join(storage::CURRENT_FILE_NAME)).unwrap();
        assert!(
            !published_bytes.is_empty(),
            "the precondition is a published pointer, not an empty index"
        );

        let key = touched
            .to_str()
            .unwrap()
            .trim_start_matches('/')
            .to_string();
        publisher.tracker().register_intent(&key);
        publisher.tracker().record_write(&key);

        // Now arm the failure. The previous pointer is the state under test,
        // so the refusal has to be settable after that publication — a spy
        // that refused from construction could never reach it.
        spy.fail_sync_of("0.bin");

        let err = publisher
            .republish_current(&descriptor_for("gen-2"))
            .unwrap_err();
        assert!(
            matches!(err, ConstellationError::DurabilityUnavailable { .. }),
            "a failed barrier must surface as durability_unavailable: {err:?}"
        );
        assert_eq!(
            std::fs::read(index_root.join(storage::CURRENT_FILE_NAME)).unwrap(),
            published_bytes,
            "a failed closure must leave the old CURRENT byte-identical: a \
             partially rewritten pointer, or a deletion, would strand readers"
        );
        assert_eq!(
            spy.current_renames(),
            1,
            "the failed republish must not attempt a second rename: {:?}",
            spy.calls()
        );

        let set = publisher.tracker().finish_writes().unwrap();
        assert!(
            set.files.iter().any(|file| file.contains("0.bin")),
            "the tracked write must still be on record after the failed sync"
        );
    }

    /// A failure *after* the rename is a different fact from a failure
    /// before it: the new pointer is already visible, so the generation is
    /// neither confirmed nor rolled back. That distinction is the whole
    /// reason `PublicationUncertain` exists, and a republish must not
    /// collapse it into a plain durability error or, worse, into success.
    #[test]
    fn a_lost_directory_ack_after_the_rename_is_publication_uncertain() {
        let tmp = TempDir::new().unwrap();
        // Canonicalise before the publisher sees the path: on macOS a temp dir
        // is reached through `/var/...`, which resolves to `/private/var/...`,
        // and the publisher refuses a symlinked component by design — a
        // symlinked alias would create a second physical owner. The refusal is
        // correct; handing it an alias from a fixture is not.
        let index_root = tmp.path().canonicalize().unwrap().join("lance");
        let touched = index_root.join("constellation_artifacts_v1.lance/data/0.bin");
        std::fs::create_dir_all(touched.parent().unwrap()).unwrap();
        std::fs::write(&touched, b"payload").unwrap();

        let spy = Arc::new(SpyDurability {
            fail_dir_sync_after_current: true,
            ..SpyDurability::default()
        });
        let publisher = spy_publisher(&index_root, &spy);

        let key = touched
            .to_str()
            .unwrap()
            .trim_start_matches('/')
            .to_string();
        publisher.tracker().register_intent(&key);
        publisher.tracker().record_write(&key);

        let err = publisher
            .republish_current(&descriptor_for("gen-1"))
            .unwrap_err();
        assert!(
            matches!(err, ConstellationError::PublicationUncertain { .. }),
            "a lost ACK after the visibility point is publication_uncertain, \
             not durability_unavailable: {err:?}"
        );
        assert!(
            spy.current_is_renamed(),
            "the refusal must land after the rename, or this proves nothing \
             about the ACK: {:?}",
            spy.calls()
        );

        // The new pointer is on disk and readable: the state the caller has
        // to treat as neither confirmed nor rolled back. A reader that sees
        // it must be able to name the generation the caller cannot confirm.
        let visible = storage::read_current(&index_root)
            .unwrap()
            .expect("the renamed pointer is visible");
        assert_eq!(
            visible.generation_id, "gen-1",
            "the visible pointer names the generation whose ACK was lost"
        );
    }

    /// A barrier that fails loudly is only half the contract; the successful
    /// republish has to leave a pointer a reader actually resolves versions
    /// from. This asserts the reader's own path — read the pointer off disk,
    /// then ask the descriptor which table version it claims — rather than
    /// that the writer's in-memory descriptor still says so.
    ///
    /// The republish now carries **only** the claimed embedding space. A drain
    /// that moved a head publishes a new generation instead, so there is no
    /// second authority for versions to disagree with.
    #[test]
    fn a_republish_carries_the_profile_and_never_a_table_version() {
        let tmp = TempDir::new().unwrap();
        // Canonicalise before the publisher sees the path: on macOS a temp dir
        // is reached through `/var/...`, which resolves to `/private/var/...`,
        // and the publisher refuses a symlinked component by design — a
        // symlinked alias would create a second physical owner. The refusal is
        // correct; handing it an alias from a fixture is not.
        let index_root = tmp.path().canonicalize().unwrap().join("lance");
        let touched = index_root.join("constellation_artifacts_v1.lance/data/0.bin");
        std::fs::create_dir_all(touched.parent().unwrap()).unwrap();
        std::fs::write(&touched, b"payload").unwrap();

        let spy = Arc::new(SpyDurability::default());
        let publisher = spy_publisher(&index_root, &spy);

        let manifest = manifest_with_versions(3, 2, 5);
        publisher
            .republish_current(&descriptor_for("gen-1"))
            .unwrap();
        assert_eq!(
            storage::read_current(&index_root)
                .unwrap()
                .expect("a published pointer")
                .effective_artifacts_version(&manifest)
                .unwrap(),
            3,
            "the reader resolves the manifest's own pin"
        );

        let mut moved = descriptor_for("gen-1");
        moved.profile_id = Some("profile-a".to_string());

        // The tracked write the drain's completion produced: the barrier has
        // to sync it before the pointer moves.
        let key = touched
            .to_str()
            .unwrap()
            .trim_start_matches('/')
            .to_string();
        publisher.tracker().register_intent(&key);
        publisher.tracker().record_write(&key);

        publisher.republish_current(&moved).unwrap();

        // Read the pointer the way a reader does — off disk, not from the
        // writer's copy.
        let visible = storage::read_current(&index_root)
            .unwrap()
            .expect("a published pointer");
        assert_eq!(
            visible.effective_artifacts_version(&manifest).unwrap(),
            3,
            "a republish cannot move a table version; only a new generation can"
        );
        assert_eq!(
            visible.effective_sections_version(&manifest).unwrap(),
            5,
            "and it cannot move the sections version either"
        );
        assert_eq!(
            visible.profile_id.as_deref(),
            Some("profile-a"),
            "the claimed embedding space travels with the pointer, so pinned \
             semantic reads query a space that exists"
        );

        // And the closure was durable before that pointer became visible —
        // the same ordering, now observed on the path that moved the head.
        let calls = spy.calls();
        let synced = calls
            .iter()
            .position(|call| call.starts_with("sync_file:") && call.contains("0.bin"))
            .expect("the drain's tracked write is synced");
        let renamed = calls
            .iter()
            .rposition(|call| {
                call.starts_with("replace:") && call.contains(storage::CURRENT_FILE_NAME)
            })
            .expect("the republish renames CURRENT");
        assert!(
            synced < renamed,
            "the head the pointer names must be durable first: {calls:?}"
        );
    }
}
