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

use std::collections::BTreeMap;
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

/// Role names of the two active tables, in the order `storage::ACTIVE_TABLES`
/// declares them.
pub const ROLE_ARTIFACTS: &str = "artifacts";
pub const ROLE_RELATIONS: &str = "relations";

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

/// Result of comparing the physical heads against the published pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadComparison {
    /// Both physical heads are exactly the published versions. Nothing is
    /// restored and nothing is committed — this is the rule that stops a
    /// pointless commit before every stage.
    Matches,
    /// One or both heads moved past the published pair, so the affected table
    /// must be rewound before any delta write.
    Diverged {
        artifacts_diverged: bool,
        relations_diverged: bool,
    },
    /// A published version is gone. Explicitly unavailable; never a jump to
    /// latest.
    MissingPublishedVersion { role: &'static str },
}

/// Compare physical head versions with the published pair.
///
/// `published` of `None` means nothing has been published, which matches by
/// definition: there is no baseline to diverge from.
pub fn compare_heads(
    published: Option<&GenerationManifest>,
    artifacts_head: u64,
    relations_head: u64,
) -> HeadComparison {
    let Some(published) = published else {
        return HeadComparison::Matches;
    };
    if !published.table_versions.is_publishable() {
        return HeadComparison::MissingPublishedVersion { role: "pair" };
    }
    let artifacts_diverged = artifacts_head != published.table_versions.artifacts;
    let relations_diverged = relations_head != published.table_versions.relations;
    if !artifacts_diverged && !relations_diverged {
        HeadComparison::Matches
    } else {
        HeadComparison::Diverged {
            artifacts_diverged,
            relations_diverged,
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
        let location = IndexLocation::new(index_root);
        // Verify the canonical parent before anything else: the owner lock
        // lives there, and creating the root is bootstrap's job, after
        // ownership is established.
        let _ = location.existing_parent()?;

        let owner = super::owner::acquire_owner(&location, Duration::from_millis(OWNER_BUDGET_MS))?;

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
    ) -> Result<CurrentDescriptor, ConstellationError> {
        let index_root = self.index_root();

        // 1. Closure barrier, over the whole new storage closure.
        let write_set = self.tracker.finish_writes()?;
        self.tracker
            .sync_written_set(&write_set, index_root, self.platform.as_ref())?;

        // 2. Generation manifest barrier. A durable manifest alone is an
        //    orphan: readers ignore it until CURRENT names it.
        let manifest_sha256 =
            storage::write_generation_manifest(index_root, manifest, self.platform.as_ref())?;

        // 3 + 4. Visibility point, then the durable ACK.
        let descriptor = storage::descriptor_for(manifest, self.owner.epoch(), &manifest_sha256);
        storage::publish_current(index_root, &descriptor, self.platform.as_ref())?;

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
        let mut marker =
            ProtocolMarker::initializing(identity.index_epoch(), identity.checkout_id(), 1);
        publisher.mark_active(&mut marker)?;
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

    // Legacy first, so there is never a moment with two live paths.
    backup_legacy_tables(&index_root)?;

    // The marker is written BEFORE the tables exist.
    let mut marker =
        publisher.announce_bootstrap(identity.checkout_id(), first_index_epoch()?, 1)?;

    // Create, or adopt what a previous attempt left behind.
    let (tables, created) = match super::lance_tables::create_active_tables(
        &index_root,
        &marker,
        std::sync::Arc::clone(publisher.tracking_store()),
    )
    .await
    {
        Ok(tables) => (tables, true),
        Err(ConstellationError::TableAlreadyExists { .. }) => {
            // The restart transition, not a failure: `create_empty` always
            // errors on an existing dataset (`table.rs:1596-1600`), so this is
            // what every second bootstrap sees. The existing tables are
            // adopted through the *same* wrapper, so the stage keeps one
            // tracker, and the baseline is rebuilt from a full capture rather
            // than resumed from whatever the interrupted attempt left.
            let tables = super::lance_tables::open_active_tables(
                &index_root,
                &marker,
                std::sync::Arc::clone(publisher.tracking_store()),
            )
            .await?;
            (tables, false)
        }
        Err(e) => return Err(e),
    };

    // A complete capture, under the workspace lock, released before any write.
    let captured = capture_under_workspace_lock(workspace, capture_limits).await?;

    // Write the rows the manifest is about to claim. The generation must
    // never name rows that do not exist — publishing a manifest over an
    // unwritten table is exactly the silent-empty index this protocol exists
    // to prevent. On a restart the adopted tables may hold a torn prefix;
    // `full_source_batch` attaches the source-delete so the first generation
    // also reclaims whatever the interrupted attempt left behind.
    if captured.is_complete() && !captured.artifacts.is_empty() {
        let artifacts = artifacts_batch(
            &captured.artifacts,
            captured.index_epoch,
            &captured.checkout_id,
        )?;
        tables.merge_into_artifacts(artifacts, true).await?;
        let relations = relations_batch(&captured.artifacts)?;
        if relations.num_rows() > 0 {
            tables.merge_into_relations(relations, true).await?;
        }
    }

    // Versions are read AFTER the writes, so the manifest names the versions
    // a pinned reader will actually open.
    let versions = tables.head_versions().await?;
    // Sanitised through the same rule as maintenance generations: the raw
    // RFC3339 stamp carries `:` — illegal on NTFS — and this id becomes a
    // filename verbatim.
    let generation_id = generation_id_for(&captured.evaluated_at);

    // `first_generation` refuses an incomplete capture through
    // `GenerationManifest::validate_for_publication`, so completeness is not
    // re-decided here.
    let manifest = first_generation(&captured, &generation_id, versions)?;

    // `publish` runs finish_writes → sync_written_set → write_generation_manifest
    // → descriptor_for → publish_current, with the real owner epoch from the
    // guard. Only after that ACK is the marker flipped.
    publisher.publish(&manifest)?;
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

/// Rename the legacy tables aside. They are never deleted, never reopened as
/// writers, and their vectors are never migrated: a vector without verified
/// input/profile bindings is not current by definition.
fn backup_legacy_tables(index_root: &Path) -> Result<(), ConstellationError> {
    for legacy in crate::constellation::manifest::LEGACY_TABLES {
        let source = index_root.join(format!("{legacy}.lance"));
        if !source.exists() {
            continue;
        }
        let destination = index_root.join(format!("{}.lance", legacy_backup_name(legacy)));
        if destination.exists() {
            // Already backed up. Overwriting would destroy the previous
            // backup, so the source is left where it is and the marker — not
            // the rename — records that the migration already happened.
            continue;
        }
        std::fs::rename(&source, &destination).map_err(|e| ConstellationError::Io {
            detail: format!(
                "cannot back up the legacy {} table to {}: {e}",
                source.display(),
                destination.display()
            ),
        })?;
    }
    Ok(())
}

/// Capture under the workspace lock, then **release** it before any write.
///
/// The guard is bound to a named variable so its scope is the read, and it is
/// dropped explicitly so nothing downstream can inherit it.
pub(crate) async fn capture_under_workspace_lock(
    workspace: &Path,
    limits: super::capture::CaptureLimits,
) -> Result<std::sync::Arc<super::capture::CapturedSourceView>, ConstellationError> {
    let guard = crate::workspace::acquire_workspace_lock(workspace)
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot take the workspace lock for capture: {e}"),
        })?;
    let captured =
        super::capture::capture_source(&super::capture::CaptureRequest::new(workspace), limits)
            .await?;
    drop(guard);
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

/// Publish a batch of vectors the caller computed, against the capture.
///
/// The caller does the **encoding** — explicit `embed` is one of the three
/// permitted model obtainers — and this does the **writing**, which is where
/// the binding actually matters.
///
/// It re-decides nothing. The guard is [`jobs::Binding::predicate`], the
/// version rule is the one from P2, and the stale semantics are
/// [`jobs::EmbeddingPublish::stale`]. All three already exist and are tested.
///
/// # Keys that are not in the capture
///
/// A vector whose key has no [`SourceEntry`] is **refused**, not written.
/// `vectors` is keyed by canonical key precisely so this can be checked;
/// without the capture a key would be unbindable and publishing it would be
/// the bypass in another name.
pub async fn publish_embeddings(
    tables: &super::lance_tables::IndexTables,
    captured: &CapturedSourceView,
    profile_id: &str,
    vectors: &BTreeMap<String, Vec<f32>>,
) -> Result<Vec<jobs::EmbeddingPublish>, ConstellationError> {
    let mut published = Vec::with_capacity(vectors.len());
    for (canonical_key, vector) in vectors {
        let entry = captured
            .entries
            .iter()
            .find(|entry| &entry.canonical_key == canonical_key)
            .ok_or_else(|| ConstellationError::Io {
                detail: format!(
                    "refusing to publish `{canonical_key}`: it is not in the capture, so it has \
                     no binding"
                ),
            })?;
        let binding = binding_for(entry, profile_id)?;
        published.push(
            tables
                .publish_embedding_into_artifacts(&binding, vector)
                .await?,
        );
    }
    Ok(published)
}

/// Which table a commit belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitTable {
    Artifacts,
    /// The edge half of the same atomic pair. A stage that writes artifacts
    /// and forgets relations publishes a projection with no links, so the
    /// enum names both.
    Relations,
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
}

impl CommitOp {
    pub fn table(&self) -> CommitTable {
        match self {
            Self::MergeUpsert { .. }
            | Self::ColumnUpdate { .. }
            | Self::DeleteByPredicate { .. } => CommitTable::Artifacts,
            Self::RelationsDelete { .. } | Self::RelationsMerge { .. } => CommitTable::Relations,
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
pub fn stage_sequence(
    commit: &CommitPlan,
    merge_rows: usize,
    delete_keys: Vec<String>,
    relations: &RelationsPlan,
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
    CommitSequence { ops }
}

/// Build the artifacts `RecordBatch` for a capture.
///
/// The column list is **derived from** [`artifacts_v1_schema`] by iterating
/// its fields, never restated. A restated list is how a batch and its schema
/// drift apart, and `merge_insert` rejects a batch whose columns do not match
/// the table.
///
/// # The three null rules
///
/// 1. A field the capture does not declare becomes a **null**. Absence stays
///    absence on the way to storage: substituting `now` for a missing
///    `created_at` would make every capture look like an edit.
/// 2. `embedding` is null unless a vector is genuinely present. There is no
///    zero-vector, because a zero-filled `FixedSizeListArray` is
///    indistinguishable from a real vector to every downstream reader.
/// 3. `semantic_state` carries `unavailable` until P3 supplies a profile. Not
///    `ready`, not `failed`.
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

    for artifact in captured {
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
        profile_id.append_option(artifact.profile_id.as_deref());
        semantic_state.append_value(artifact.semantic_state.as_str());
        retry_after.append_null();
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
    let embedding_field = schema
        .field_with_name("embedding")
        .map_err(|e| unknown_column("embedding", e))?
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
    // `FixedSizeListArray::new_null` takes the *child* field plus the list
    // length — passing the outer `embedding` field here wraps it a second
    // time and yields `FixedSizeList(1024 x FixedSizeList(1024 x Float32))`,
    // which the schema check rightly rejects. Unpack the item field out of
    // the declared embedding type instead.
    let embedding_item_field = match embedding_field.data_type() {
        arrow_schema::DataType::FixedSizeList(child, _) => child.clone(),
        other => {
            return Err(unknown_column(
                "embedding",
                format!("expected FixedSizeList item type, found {other}"),
            ));
        }
    };
    let embedding: arrow_array::ArrayRef = Arc::new(arrow_array::FixedSizeListArray::new_null(
        embedding_item_field,
        super::lance_tables::EMBEDDING_DIM,
        row_count,
    ));

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
            detail: format!("column update on the artifacts table failed: {e}"),
        })?;

    let (version, version_source) = if result.version != 0 {
        (result.version, VersionSource::Reported)
    } else {
        let read = table.version().await.map_err(|e| ConstellationError::Io {
            detail: format!("cannot read the artifacts version after a column update: {e}"),
        })?;
        if read == 0 {
            return Err(ConstellationError::Io {
                detail: "the artifacts table reported version 0 after a column update".to_string(),
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
    /// The reader is not on the version the commits reported.
    VersionMismatch { expected: u64, found: u64 },
    /// The relations reader is not on the version the commits reported.
    RelationsVersionMismatch { expected: u64, found: u64 },
    /// The relations table does not carry the edge count the capture implies.
    /// Per-edge digests are not re-checked — the artifact digest comparison
    /// is the integrity check; this proves the edge set is the right size.
    EdgeCountDisagreement { expected: usize, found: usize },
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
            Self::VersionMismatch { expected, found } => write!(
                f,
                "version_mismatch: the artifacts reader is on version {found}, the commits reported {expected}"
            ),
            Self::RelationsVersionMismatch { expected, found } => write!(
                f,
                "relations_version_mismatch: the relations reader is on version {found}, \
                 the commits reported {expected}"
            ),
            Self::EdgeCountDisagreement { expected, found } => write!(
                f,
                "edge_count_disagreement: the relations table holds {found} edges, \
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
/// Accumulation is `O(expected keys)`, not `O(corpus)`. The expected map is
/// borrowed and never cloned; found keys are counted, not collected; and the
/// first unexpected row returns immediately rather than being accumulated.
/// That matters because the whole design rests on not copying the corpus per
/// job — a verifier that materialised every row would reintroduce exactly the
/// cost the capture avoided.
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
            if let Some((borrowed, _)) = expected.get_key_value(&key) {
                seen.insert(borrowed.as_str());
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

    // The relations half. Same discipline: a separately opened reader pinned
    // to the committed version, never `checkout_latest`. The check is the
    // edge count, not per-edge digests — the artifact digest comparison is
    // the integrity boundary; this proves the edge set is the right size.
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

    Ok(VerificationReport {
        reader_version,
        expected_keys: expected.len(),
        verified_keys: seen.len(),
        digests_compared,
        relations_version,
        expected_edges,
        verified_edges,
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
            detail: format!("merge_insert on the artifacts table failed: {e}"),
        })?;

    let (version, version_source) = if result.version != 0 {
        (result.version, VersionSource::Reported)
    } else {
        let read = table.version().await.map_err(|e| ConstellationError::Io {
            detail: format!("cannot read the artifacts version after a merge: {e}"),
        })?;
        if read == 0 {
            return Err(ConstellationError::Io {
                detail: "the artifacts table reported version 0 after a merge".to_string(),
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

/// Execute one bounded delete against the artifacts table.
///
/// This is the sharpest of the three ops, and deliberately the first to be
/// executed: `DeleteResult` carries **no** version field
/// (`lancedb-0.27.2/src/table/delete.rs:9-13`), so the recording rule has to
/// be real rather than pass-through. The version is read from the table after
/// the commit, under the single owner, and never taken from the result.
///
/// The predicate comes from [`delete_predicate_for`] — an enumerated
/// `IN (…)` over escaped literals, never a wildcard, and an empty key set is
/// refused rather than producing a predicate that matches every row.
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
    let CommitOp::DeleteByPredicate { keys } = op else {
        return Err(ConstellationError::IndexProtocolMismatch {
            detail: format!(
                "execute_delete was handed a {op:?} operation; it executes only \
                 DeleteByPredicate"
            ),
        });
    };
    if keys.is_empty() {
        return Err(ConstellationError::StorageIo {
            detail: "refusing to execute a delete with no keys: the predicate would match every \
                      row"
            .to_string(),
        });
    }
    let predicate = delete_predicate_for(keys)?;

    let result = table
        .delete(&predicate)
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("bounded delete on the artifacts table failed: {e}"),
        })?;

    // `DeleteResult` has no version field, so this is the only source. It is
    // read after the commit, under the owner, and a `0` here would mean the
    // table never committed — which is not a version this protocol can record.
    let version = table.version().await.map_err(|e| ConstellationError::Io {
        detail: format!("cannot read the artifacts version after a delete: {e}"),
    })?;
    if version == 0 {
        return Err(ConstellationError::Io {
            detail: "the artifacts table reported version 0 after a delete".to_string(),
        });
    }

    Ok(DeleteOutcome {
        version,
        rows_deleted: result.num_deleted_rows,
        keys: keys.clone(),
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
    let literals: Vec<String> = canonical_keys
        .iter()
        .map(|key| escape_predicate_literal(key))
        .collect();
    Ok(format!("canonical_key IN ({})", literals.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::capture::{
        CaptureLimits, CaptureRequest, Coverage, FilesystemSupport, SourceEntry, UnusableSource,
    };
    use crate::constellation::jobs::SemanticState;
    use crate::constellation::manifest::GenerationManifest;
    use crate::constellation::reconciler::plan_reconcile;
    use crate::workspace::ARTIFACT_DIRS;
    use arrow_array::Array;
    use std::path::PathBuf;
    use tempfile::TempDir;

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

    fn manifest_with_versions(artifacts: u64, relations: u64) -> GenerationManifest {
        GenerationManifest {
            schema_version: crate::constellation::FORMAT_VERSION,
            generation_id: "gen-1".to_string(),
            checkout_id: "local-x".to_string(),
            index_epoch: 1,
            source_set_digest: "ss".to_string(),
            config_digest: "cd".to_string(),
            table_versions: TableVersions::new(artifacts, relations),
            source_entries: Vec::new(),
            tombstones: Vec::new(),
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
        assert_eq!(compare_heads(None, 7, 9), HeadComparison::Matches);
    }

    #[test]
    fn matching_heads_restore_nothing_and_commit_nothing() {
        let published = manifest_with_versions(3, 2);
        assert_eq!(
            compare_heads(Some(&published), 3, 2),
            HeadComparison::Matches
        );
    }

    #[test]
    fn divergence_names_exactly_the_table_that_moved() {
        let published = manifest_with_versions(3, 2);
        assert_eq!(
            compare_heads(Some(&published), 4, 2),
            HeadComparison::Diverged {
                artifacts_diverged: true,
                relations_diverged: false
            }
        );
        assert_eq!(
            compare_heads(Some(&published), 3, 5),
            HeadComparison::Diverged {
                artifacts_diverged: false,
                relations_diverged: true
            }
        );
        assert_eq!(
            compare_heads(Some(&published), 4, 5),
            HeadComparison::Diverged {
                artifacts_diverged: true,
                relations_diverged: true
            }
        );
    }

    #[test]
    fn a_published_pair_with_a_missing_half_is_unavailable_not_divergence() {
        let published = manifest_with_versions(0, 2);
        assert_eq!(
            compare_heads(Some(&published), 4, 2),
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
            artifacts: Vec::new(),
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
        GenerationManifest::from_capture(&complete(entries), "gen-1", TableVersions::new(1, 1))
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

        let manifest = first_generation(&captured, "gen-1", TableVersions::new(1, 1)).unwrap();
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

        let err = first_generation(&captured, "gen-1", TableVersions::new(1, 1)).unwrap_err();
        assert_eq!(
            err.code(),
            DiagnosticCode::SourceBusy,
            "an empty first generation is indistinguishable from an empty workspace"
        );
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

    #[test]
    fn a_stage_with_confirmed_deletes_emits_one_bounded_delete_op() {
        let sequence = stage_sequence(
            &commit_plan(true, 0, 0, 1),
            0,
            vec!["PRD-002".to_string(), "PRD-003".to_string()],
            &no_relations(),
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
        let sequence = stage_sequence(&commit_plan(true, 1, 1, 0), 2, vec![], &no_relations());
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
            &no_relations(),
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
            &RelationsPlan {
                delete_keys: vec!["PRD-001".into(), "PRD-002".into()],
                merge_rows: 3,
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
    fn a_partial_capture_never_attaches_the_bulk_delete_clause() {
        let sequence = stage_sequence(&commit_plan(false, 1, 0, 0), 1, vec![], &no_relations());
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
    fn a_stage_that_changes_nothing_emits_no_commits() {
        let sequence = stage_sequence(&commit_plan(true, 0, 0, 0), 0, vec![], &no_relations());
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
        let batch = artifacts_batch(&captured, 1, "local-test").unwrap();
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
        let batch = artifacts_batch(&captured, 1, "local-test").unwrap();
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
        let batch = artifacts_batch(&captured, 1, "local-test").unwrap();
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
        let batch = artifacts_batch(&captured, 1, "local-test").unwrap();

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
        let batch =
            artifacts_batch(&[fixture_artifact(None, None, None)], 1, "local-test").unwrap();
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
        let batch = artifacts_batch(&[], 1, "local-test").unwrap();
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
        let batch = artifacts_batch(&[artifact], 1, "local-test").unwrap();
        let column = batch
            .column_by_name("diagnostics")
            .expect("diagnostics column")
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .expect("diagnostics is a string column");
        assert_eq!(column.value(0), "invalid_links,unreadable_source");
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
            CommitOp::ColumnUpdate { keys, .. } => keys.clone(),
            CommitOp::MergeUpsert { .. } | CommitOp::RelationsMerge { .. } => Vec::new(),
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
}
