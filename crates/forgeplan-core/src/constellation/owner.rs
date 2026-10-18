//! Physical index identity — the key every ownership decision is made on.
//!
//! P1 establishes identity; P2 adds the RAII OS guard on top of it. Two
//! facts have to be settled before any lock can be taken, and both are
//! settled here rather than at each call site:
//!
//! 1. **Where is the index, physically?** [`IndexLocation`] resolves the
//!    canonical local root and, with it, the stable sibling owner-lock file
//!    name. The lock lives in the *existing canonical parent*, not inside the
//!    index root — the root does not exist yet on a first run, and a lock
//!    inside it would be created and destroyed together with the data it is
//!    meant to protect.
//!
//! 2. **Whose index is it?** [`IndexIdentity`] derives the checkout id and
//!    epoch. The id comes from the physical root, so two linked worktrees —
//!    which the resolver deliberately resolves to two different `.forgeplan`
//!    directories — get two different ids without consulting git. That is
//!    the isolation L13 requires, obtained without a branch-name lookup that
//!    two worktrees on the same branch would share.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use fs2::FileExt;
use serde::{Deserialize, Serialize};

use super::{ConstellationError, rfc3339_now, sha256_hex};

/// Budget for acquiring the physical owner, inside the overall barrier.
///
/// Project parameter, not an SLA. Exhausting it is `index_busy` — never a
/// forced steal.
pub const OWNER_ACQUIRE_BUDGET_MS: u64 = 750;

/// Epoch counter shared by every owner guard in this process.
///
/// Diagnostics only. It is deliberately **not** storage fencing: a stale
/// `owner_epoch` must never be used to unlink, replace or steal anybody's
/// lock. Serialisation is the OS lock's job.
static OWNER_EPOCH: AtomicU64 = AtomicU64::new(0);

/// Process-unique nonce so two runs of the same pid across a fork/exec cycle
/// are distinguishable in the lock file. Diagnosis only.
static START_NONCE: AtomicU64 = AtomicU64::new(1);

/// What is written into the owner-lock file so an operator can tell *who* is
/// holding a live index.
///
/// This is diagnosis only. Nothing reads it to decide whether to take over:
/// takeover happens only after `try_lock_exclusive` succeeds, i.e. after the
/// previous owner's process is actually gone. A pid recorded in a file whose
/// owner died minutes ago must never cause a steal, and a pid recycled by the
/// OS must never cause a false one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct OwnerMetadata {
    pub pid: u32,
    pub start_nonce: u64,
    pub owner_epoch: u64,
    pub index_root: String,
    pub acquired_at: String,
}

/// Name of the durable protocol marker inside the index root.
///
/// Written by P2 during bootstrap. Read here so a restarted process keeps the
/// identity its predecessor published under.
const PROTOCOL_FILE: &str = "constellation/PROTOCOL.json";

/// Epoch reserved for an index that has never published a generation.
///
/// `Uninitialized` is the normal state of a fresh checkout, not corruption.
/// Epoch `0` is therefore never a *published* epoch — published generations
/// start at `1` — so `0` unambiguously means "no published identity yet".
const UNINITIALIZED_EPOCH: u64 = 0;

/// Canonical physical location of one constellation index, plus the stable
/// name of the owner lock that guards it.
pub(crate) struct IndexLocation {
    root: PathBuf,
    owner_lock_name: String,
}

impl IndexLocation {
    /// Build the identity for an index root directory.
    ///
    /// The root need not exist yet: bootstrap creates it after ownership is
    /// established. That is why the lock name is derived from the *path*,
    /// not from anything read inside the root.
    pub(crate) fn new(root: &Path) -> Self {
        let owner_lock_name = format!(
            ".constellation-owner-{}.lock",
            sha256_hex(root.as_os_str().as_encoded_bytes())
        );
        Self {
            root: root.to_path_buf(),
            owner_lock_name,
        }
    }

    /// The canonical index root.
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// Stable sibling owner-lock path, in the root's existing parent.
    ///
    /// Its identity is `sha256(canonical IndexLocation)`, so two different
    /// indexes never contend for one lock and one index is never addressed
    /// through two different locks by writing the same bytes under a second
    /// spelling.
    pub(crate) fn owner_lock_path(&self) -> PathBuf {
        self.root
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&self.owner_lock_name)
    }

    /// Verify the precondition ADR-029 §1 states for owner acquisition: the
    /// canonical parent directory must already exist, because that is where
    /// the owner lock is created and the lock is the one bootstrap operation
    /// allowed before `WorkspaceLock`.
    ///
    /// Returns the parent path on success.
    pub(crate) fn existing_parent(&self) -> Result<PathBuf, ConstellationError> {
        let lock_path = self.owner_lock_path();
        let parent = lock_path
            .parent()
            .ok_or_else(|| ConstellationError::Io {
                detail: "index root has no parent directory".to_string(),
            })?
            .to_path_buf();
        if !parent.is_dir() {
            return Err(ConstellationError::Io {
                detail: format!(
                    "canonical parent {} does not exist; the owner lock cannot be created before it",
                    parent.display()
                ),
            });
        }
        Ok(parent)
    }
}

/// Checkout identity plus published epoch of one physical index.
pub(crate) struct IndexIdentity {
    checkout_id: String,
    index_epoch: u64,
}

use super::manifest::ProtocolMarker;

impl IndexIdentity {
    /// Read the durable marker when one exists, otherwise derive a fresh
    /// identity for this workspace.
    ///
    /// Deriving the checkout id from the canonical root rather than from the
    /// branch name is deliberate: two worktrees can be on the same branch,
    /// and a branch-derived id would silently merge their indexes.
    pub(crate) fn derive(location: &IndexLocation) -> Self {
        let marker_path = location.root().join(PROTOCOL_FILE);
        if let Ok(text) = std::fs::read_to_string(&marker_path)
            && let Ok(marker) = serde_json::from_str::<ProtocolMarker>(&text)
            && !marker.checkout_id.is_empty()
        {
            return Self {
                checkout_id: marker.checkout_id,
                index_epoch: marker.index_epoch,
            };
        }

        Self {
            checkout_id: Self::path_bound_checkout_id(location),
            index_epoch: UNINITIALIZED_EPOCH,
        }
    }

    /// The checkout id this location would produce with **no** marker — the
    /// identity bound to the path alone.
    ///
    /// [`Self::derive`] adopts the marker's `checkout_id` when one exists,
    /// which makes `marker.check_identity(identity.checkout_id())` compare
    /// the marker with itself and `IndexIdentityMismatch` unreachable. The
    /// honest comparison is marker-vs-path: a foreign index copied under
    /// this root carries a different path hash and is refused.
    pub(crate) fn path_bound_checkout_id(location: &IndexLocation) -> String {
        format!(
            "local-{}",
            &sha256_hex(location.root().as_os_str().as_encoded_bytes())[..16]
        )
    }

    pub(crate) fn checkout_id(&self) -> &str {
        &self.checkout_id
    }

    pub(crate) fn index_epoch(&self) -> u64 {
        self.index_epoch
    }
}

/// Exclusive physical ownership of one index, held as an OS advisory lock.
///
/// # What this type guarantees
///
/// * The lock file is a **stable sibling** in the verified existing canonical
///   parent, so it exists before the index root does — a lock inside the root
///   would be created and destroyed together with the data it protects.
/// * The [`std::fs::File`] is never duplicated, cloned, or `try_clone`d. A
///   duplicated descriptor is the classic `fs2` footgun: `flock` is per
///   open-file-description, and unlocking through one copy releases the lock
///   the other copy believes it holds.
/// * The inode is **never unlinked or replaced**, here or at shutdown.
///   Removing it creates a second inode, and the next acquirer locks the
///   second one while the first owner's lock is still live — two owners.
/// * Dropping closes the descriptor, which releases the lock.
///
/// # Lock order
///
/// Acquisition order is `actor permit → OS owner → WorkspaceLock`, and this
/// type is only ever the middle step. It must never be acquired while a
/// `WorkspaceLock` is held: a short CLI mutation that already holds the
/// workspace lock and then wanted the owner would deadlock against a
/// long-running publisher that holds the owner and wants the workspace lock.
pub(crate) struct OwnerGuard {
    /// Never cloned, never read: held only so that closing it releases the
    /// lock. `flock`/`LockFileEx` bind to the open file description, so this
    /// descriptor *is* the ownership.
    _file: std::fs::File,
    path: PathBuf,
    epoch: u64,
    // No in-memory copy of `OwnerMetadata`. Its purpose is operator diagnosis
    // *through the lock file*, and `acquire_owner` serialises it there
    // directly; a copy on the guard would only ever be read by an accessor
    // nothing calls.
}

impl std::fmt::Debug for OwnerGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Deliberately omits the descriptor: `File`'s Debug would print a path
        // that belongs in a log, not in a diagnostic dump.
        f.debug_struct("OwnerGuard")
            .field("path", &self.path)
            .field("epoch", &self.epoch)
            .finish()
    }
}

impl OwnerGuard {
    /// Monotonic per-process owner epoch. Diagnostics only.
    pub(crate) fn epoch(&self) -> u64 {
        self.epoch
    }
}
// `path()` and `metadata()` were removed: nothing in or out of the crate read
// them. The lock file's contents serve diagnosis directly, and `path` is
// still surfaced through the `Debug` impl above. Re-adding accessors needs a
// reader, not an intention.

/// There is deliberately no `Drop` impl. The lock is released when the
/// descriptor closes, which is exactly what dropping the field does; an
/// explicit `Drop` could only add a way to get it wrong — unlocking
/// explicitly, or touching the inode on the way out.
///
/// Acquire the physical owner of `location`.
///
/// The child index root is **not** created here — an existing canonical parent
/// is the precondition, which is exactly why the lock lives there. Creating
/// the root is the first thing [`crate::constellation::durability`] does after
/// this returns, under owner → workspace order.
///
/// Retries `try_lock_exclusive` until `budget` is spent. Contention is the
/// expected case for a second `serve`/CLI pair, not an error to route around:
/// the answer is `index_busy`, and the caller waits its own bounded amount.
///
/// # Errors
///
/// [`ConstellationError::IndexBusy`] when the budget is spent.
/// [`ConstellationError::Io`] when the canonical parent is missing or the
/// lock file cannot be opened for a reason that is not contention.
pub(crate) fn acquire_owner(
    location: &IndexLocation,
    budget: Duration,
) -> Result<OwnerGuard, ConstellationError> {
    let _ = location.existing_parent()?;
    let path = location.owner_lock_path();

    // A planted symlink at the lock path would be followed by `create(true)`
    // and `set_len(0)` would truncate whatever it pointed at — same refusal
    // as `workspace::lock::refuse_if_symlink`.
    match std::fs::symlink_metadata(&path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(ConstellationError::Io {
                detail: format!(
                    "the owner lock {} is a symlink; refusing to open an alias",
                    path.display()
                ),
            });
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(ConstellationError::Io {
                detail: format!("cannot stat the owner lock {}: {e}", path.display()),
            });
        }
    }

    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot open owner lock {}: {e}", path.display()),
        })?;

    let started = Instant::now();
    loop {
        match file.try_lock_exclusive() {
            Ok(()) => break,
            Err(e) if is_contended(&e) => {
                if started.elapsed() >= budget {
                    return Err(ConstellationError::IndexBusy {
                        detail: format!(
                            "owner of {} held by another process for {}ms",
                            path.display(),
                            started.elapsed().as_millis()
                        ),
                    });
                }
                // Short, bounded backoff. Not a "sleep and hope": the loop is
                // bounded by `budget`, and the failure mode is an explicit
                // `index_busy`.
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => {
                return Err(ConstellationError::Io {
                    detail: format!("cannot lock owner file {}: {e}", path.display()),
                });
            }
        }
    }

    let epoch = OWNER_EPOCH.fetch_add(1, Ordering::SeqCst);
    let metadata = OwnerMetadata {
        pid: std::process::id(),
        start_nonce: START_NONCE.fetch_add(1, Ordering::SeqCst),
        owner_epoch: epoch,
        index_root: location.root().display().to_string(),
        acquired_at: rfc3339_now(),
    };

    // Diagnosis only. A failure to write the metadata does not fail the
    // acquisition: the OS lock — not this file's contents — is the authority,
    // and losing the metadata must not turn a free index into `index_busy`.
    if let Ok(serialized) = serde_json::to_vec(&metadata) {
        use std::io::{Seek, SeekFrom, Write};
        let mut handle = &file;
        let _ = handle.set_len(0);
        let _ = handle.seek(SeekFrom::Start(0));
        let _ = handle.write_all(&serialized);
        let _ = handle.flush();
    }

    Ok(OwnerGuard {
        _file: file,
        path,
        epoch,
    })
}

/// Whether a lock error is contention rather than a real failure.
///
/// `flock` reports `EWOULDBLOCK` on unix and `LockFileEx` reports sharing
/// violations on Windows; both are "somebody else has it", never "this index
/// is broken". Anything else is a genuine I/O failure.
fn is_contended(error: &std::io::Error) -> bool {
    if error.kind() == std::io::ErrorKind::WouldBlock {
        return true;
    }
    match (
        error.raw_os_error(),
        fs2::lock_contended_error().raw_os_error(),
    ) {
        (Some(actual), Some(expected)) => actual == expected,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn owner_lock_name_is_sha256_of_the_canonical_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".forgeplan").join("lance");
        let location = IndexLocation::new(&root);
        let expected = sha256_hex(root.as_os_str().as_encoded_bytes());
        assert_eq!(
            location.owner_lock_name,
            format!(".constellation-owner-{expected}.lock")
        );
    }

    #[test]
    fn owner_lock_lives_in_the_parent_not_in_the_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".forgeplan").join("lance");
        let location = IndexLocation::new(&root);
        let lock = location.owner_lock_path();
        assert_eq!(lock.parent().unwrap(), root.parent().unwrap());
        assert_ne!(lock.parent().unwrap(), location.root());
    }

    #[test]
    fn two_indexes_never_share_one_owner_lock() {
        let tmp = TempDir::new().unwrap();
        let a = IndexLocation::new(&tmp.path().join("a").join("lance"));
        let b = IndexLocation::new(&tmp.path().join("b").join("lance"));
        assert_ne!(a.owner_lock_name, b.owner_lock_name);
        // Same root, same lock — idempotent by construction.
        let a2 = IndexLocation::new(&tmp.path().join("a").join("lance"));
        assert_eq!(a.owner_lock_name, a2.owner_lock_name);
    }

    #[test]
    fn existing_parent_fails_when_the_canonical_parent_is_absent() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("missing").join("lance");
        let location = IndexLocation::new(&root);
        let err = location.existing_parent().unwrap_err();
        assert_eq!(
            err.code(),
            super::super::DiagnosticCode::StorageIo,
            "a missing parent is an I/O precondition failure, not a durability verdict"
        );
    }

    #[test]
    fn existing_parent_succeeds_for_an_existing_canonical_parent() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".forgeplan").join("lance");
        std::fs::create_dir_all(&root).unwrap();
        let location = IndexLocation::new(&root);
        assert_eq!(
            location.existing_parent().unwrap(),
            tmp.path().join(".forgeplan")
        );
    }

    #[test]
    fn uninitialized_identity_uses_the_reserved_zero_epoch() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".forgeplan").join("lance");
        std::fs::create_dir_all(&root).unwrap();
        let identity = IndexIdentity::derive(&IndexLocation::new(&root));
        assert_eq!(identity.index_epoch(), UNINITIALIZED_EPOCH);
        assert!(identity.checkout_id().starts_with("local-"));
        assert_eq!(identity.checkout_id().len(), "local-".len() + 16);
    }

    #[test]
    fn durable_marker_overrides_the_derived_identity() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".forgeplan").join("lance");
        std::fs::create_dir_all(root.join("constellation")).unwrap();
        std::fs::write(
            root.join(PROTOCOL_FILE),
            r#"{"protocol_version":1,"state":"active","index_epoch":9,"checkout_id":"checkout-from-marker","table_names":["constellation_artifacts_v1","constellation_relations_v1"],"storage_scheme":"file-object-store","bootstrap_attempt":1,"unknown_future_field":1}"#,
        )
        .unwrap();
        let identity = IndexIdentity::derive(&IndexLocation::new(&root));
        assert_eq!(identity.checkout_id(), "checkout-from-marker");
        assert_eq!(identity.index_epoch(), 9);
    }

    #[test]
    fn corrupt_marker_falls_back_to_a_derived_identity_instead_of_failing() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".forgeplan").join("lance");
        std::fs::create_dir_all(root.join("constellation")).unwrap();
        std::fs::write(root.join(PROTOCOL_FILE), "{not json").unwrap();
        let identity = IndexIdentity::derive(&IndexLocation::new(&root));
        assert!(identity.checkout_id().starts_with("local-"));
        assert_eq!(identity.index_epoch(), UNINITIALIZED_EPOCH);
    }

    #[test]
    fn two_worktrees_get_distinct_checkout_ids() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("wt-a").join(".forgeplan").join("lance");
        let b = tmp.path().join("wt-b").join(".forgeplan").join("lance");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let ia = IndexIdentity::derive(&IndexLocation::new(&a));
        let ib = IndexIdentity::derive(&IndexLocation::new(&b));
        assert_ne!(ia.checkout_id(), ib.checkout_id());
    }

    // -----------------------------------------------------------------------
    // Owner guard
    // -----------------------------------------------------------------------

    fn guarded_root() -> (TempDir, IndexLocation) {
        let tmp = TempDir::new().unwrap();
        let parent = tmp.path().join(".forgeplan");
        std::fs::create_dir_all(&parent).unwrap();
        let location = IndexLocation::new(&parent.join("lance"));
        (tmp, location)
    }

    #[test]
    fn acquiring_creates_the_lock_in_the_parent_and_leaves_the_root_alone() {
        let (_tmp, location) = guarded_root();
        assert!(
            !location.root().exists(),
            "the index root must not exist before ownership"
        );

        let guard = acquire_owner(&location, Duration::from_millis(100)).unwrap();
        assert!(guard.path.exists());
        assert_eq!(
            guard.path.parent().unwrap(),
            location.root().parent().unwrap()
        );
        assert!(
            !location.root().exists(),
            "acquiring ownership must not create the index root"
        );
        let written: OwnerMetadata =
            serde_json::from_slice(&std::fs::read(&guard.path).unwrap()).unwrap();
        assert_eq!(written.pid, std::process::id());
        assert_eq!(written.owner_epoch, guard.epoch);
    }

    #[test]
    fn the_lock_file_outlives_every_guard_that_held_it() {
        let (_tmp, location) = guarded_root();
        let path = {
            let guard = acquire_owner(&location, Duration::from_millis(100)).unwrap();
            guard.path.to_path_buf()
        };
        let first_path = {
            let guard = acquire_owner(&location, Duration::from_millis(100)).unwrap();
            assert_eq!(guard.path, path, "the lock path must be stable");
            guard.path.to_path_buf()
        };
        assert_eq!(first_path, path);
        assert!(
            path.exists(),
            "the lock file must never be unlinked: a second inode would \
             admit a second owner while the first lock is still live"
        );
        assert_eq!(
            std::fs::read_dir(location.root().parent().unwrap())
                .unwrap()
                .count(),
            1,
            "the canonical parent holds exactly the lock and nothing else"
        );
    }

    #[test]
    fn a_second_acquisition_inside_this_process_is_index_busy_not_a_steal() {
        let (_tmp, location) = guarded_root();
        let _first = acquire_owner(&location, Duration::from_millis(100)).unwrap();

        let started = Instant::now();
        let err = acquire_owner(&location, Duration::from_millis(50)).unwrap_err();
        assert_eq!(
            err.code(),
            super::super::DiagnosticCode::IndexBusy,
            "a live owner is contention, never something to force"
        );
        assert!(
            started.elapsed() >= Duration::from_millis(50),
            "the caller must actually have waited its budget"
        );
    }

    #[test]
    fn ownership_is_released_when_the_guard_drops() {
        let (_tmp, location) = guarded_root();
        {
            let _guard = acquire_owner(&location, Duration::from_millis(100)).unwrap();
        }
        // Would fail with index_busy if the descriptor had not closed.
        let _second = acquire_owner(&location, Duration::from_millis(200)).unwrap();
    }

    #[test]
    fn a_missing_canonical_parent_is_an_io_failure_not_a_lock() {
        let tmp = TempDir::new().unwrap();
        let location = IndexLocation::new(&tmp.path().join("absent").join("lance"));
        let err = acquire_owner(&location, Duration::from_millis(50)).unwrap_err();
        assert_eq!(err.code(), super::super::DiagnosticCode::StorageIo);
    }

    #[test]
    fn two_indexes_never_contend_for_one_owner_lock() {
        let tmp = TempDir::new().unwrap();
        let parent_a = tmp.path().join("a");
        let parent_b = tmp.path().join("b");
        std::fs::create_dir_all(&parent_a).unwrap();
        std::fs::create_dir_all(&parent_b).unwrap();
        let a = acquire_owner(
            &IndexLocation::new(&parent_a.join("lance")),
            Duration::from_millis(100),
        )
        .unwrap();
        let b = acquire_owner(
            &IndexLocation::new(&parent_b.join("lance")),
            Duration::from_millis(100),
        )
        .unwrap();
        assert_ne!(a.path, b.path);
    }

    #[test]
    fn owner_epochs_advance_across_acquisitions() {
        let (_tmp, location) = guarded_root();
        let first = acquire_owner(&location, Duration::from_millis(100)).unwrap();
        let first_epoch = first.epoch;
        drop(first);
        let second = acquire_owner(&location, Duration::from_millis(100)).unwrap();
        assert!(
            second.epoch > first_epoch,
            "owner epochs are monotonic so a stale one is recognisable"
        );
    }

    #[test]
    fn owner_metadata_round_trips_and_is_diagnosis_only() {
        let (_tmp, location) = guarded_root();
        let guard = acquire_owner(&location, Duration::from_millis(100)).unwrap();
        let parsed: OwnerMetadata =
            serde_json::from_slice(&std::fs::read(&guard.path).unwrap()).unwrap();
        assert!(parsed.start_nonce > 0);
        assert!(parsed.acquired_at.ends_with('Z'));
    }
}
