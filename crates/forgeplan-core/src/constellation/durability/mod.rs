//! Durability seam for the physical index.
//!
//! Three things live here:
//!
//! 1. [`PlatformDurability`] — the platform barrier abstraction P2c attaches
//!    the NTFS implementation to. Everything platform-specific is behind it.
//! 2. [`StageTracker`] — the shared, per-stage record of what a generation's
//!    storage closure actually touched, and the [`WriteSet`] it produces.
//! 3. [`DurabilityTrackingStore`] (in [`store`]) — the
//!    `object_store::ObjectStore` + `lance_io::WrappingObjectStore`
//!    implementation that feeds the tracker.
//!
//! # The ordering this module exists to protect
//!
//! ```text
//! joined storage writes
//!   → closure barrier          (sync_written_set)
//!   → generation manifest barrier
//!   → same-filesystem CURRENT replacement      (the visibility point)
//!   → platform durability ACK                  (a different guarantee)
//! ```
//!
//! Reaching the visibility point without the ACK is `publication_uncertain`.
//! It is **not** success, and it is not a licence to report otherwise. The
//! distinction matters because rename makes the new pointer visible
//! immediately, while the sync that makes it survive a power cut happens
//! after — a caller that conflates the two believes it published when it only
//! published a hope.
//!
//! # Why the `file-object-store` scheme is mandatory
//!
//! In `lance-io 4.0.0` the ordinary `file` scheme reaches `LocalWriter` /
//! `std::fs` directly for object creation and copy, so a store wrapped by a
//! `WrappingObjectStore` sees **nothing** on those paths. The built-in
//! `file-object-store` provider keeps the same `LocalFileSystem` underneath
//! but routes creation and copy through the generic writer/inner path that a
//! wrapper actually intercepts. That is why every active table URI in
//! [`crate::constellation::storage`] carries that scheme, and why
//! [`ProtocolMarker::validate`](super::manifest::ProtocolMarker::validate)
//! refuses a marker that names anything else.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub mod store;

#[cfg(not(windows))]
mod posix;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
mod windows_provider;

pub use store::DurabilityTrackingStore;
#[cfg(windows)]
pub(crate) use windows::{open_directory_no_reparse, read_mutable_file, replace_mutable_file};
#[cfg(windows)]
pub(crate) use windows_provider::windows_wrapped_session;

use super::capture::FilesystemSupport;
use super::{ConstellationError, DiagnosticCode, SourceDiagnostic};

/// Filesystem classification plus, when one can be selected, the barrier that
/// will do the writes.
///
/// A supported platform yields `Supported` **and** a barrier. A filesystem we
/// cannot prove can supply one yields `Unsupported` and no barrier, so the
/// caller cannot accidentally write through a path that will not ACK.
pub(crate) fn platform_barrier(
    index_root: &Path,
) -> (
    FilesystemSupport,
    Option<Arc<dyn PlatformDurability>>,
    Option<SourceDiagnostic>,
) {
    let (support, diagnostic) = classify_index_root(index_root);
    if diagnostic.is_some() {
        return (FilesystemSupport::Unsupported, None, diagnostic);
    }
    #[cfg(windows)]
    {
        let _ = support;
        return (
            FilesystemSupport::Unsupported,
            None,
            Some(SourceDiagnostic::new(
                DiagnosticCode::UnsupportedIndexFilesystem,
                None,
                "Windows live object-store namespace durability is not established",
            )),
        );
    }
    #[cfg(not(windows))]
    {
        let barrier: Arc<dyn PlatformDurability> = Arc::new(selected_barrier());
        (support, Some(barrier), None)
    }
}

/// Select the NTFS primitive adapter after validating the backing volume.
/// Only for explicit write-through primitives and exclusive private-stage
/// sealing, never for the general live object-store route.
#[cfg(windows)]
pub(crate) fn windows_ntfs_primitive_barrier(
    index_root: &Path,
) -> (
    FilesystemSupport,
    Option<Arc<dyn PlatformDurability>>,
    Option<SourceDiagnostic>,
) {
    let (support, diagnostic) = classify_index_root(index_root);
    if diagnostic.is_some() {
        return (FilesystemSupport::Unsupported, None, diagnostic);
    }
    (support, Some(Arc::new(WindowsNtfsDurability)), None)
}

#[cfg(not(windows))]
fn selected_barrier() -> PosixDurability {
    PosixDurability
}

#[cfg(windows)]
use windows::WindowsNtfsDurability;

/// Platform-specific half of root classification.
///
/// Windows additionally requires a local fixed **NTFS** volume. Everything
/// else — ReFS, FAT, exFAT, a network share, a volume we cannot name —
/// is refused, because an index on a filesystem we cannot reason about must
/// be reported as unsupported rather than written to and called durable.
#[cfg(windows)]
fn verify_volume(index_root: &Path) -> Result<(), String> {
    windows::verify_local_ntfs(index_root)
}

/// Verify that the path is backed by a local fixed NTFS volume and its
/// existing directory components are not reparse points.
#[cfg(windows)]
pub(crate) fn verify_local_ntfs(path: &Path) -> Result<(), String> {
    windows::verify_local_ntfs(path)
}

/// Construct a lossless local URI for a session from `windows_wrapped_session`.
/// The default Lance provider cannot decode these path escapes correctly.
#[cfg(windows)]
pub(crate) fn windows_wrapped_uri(path: &Path) -> Result<String, ConstellationError> {
    windows::verify_local_ntfs(path).map_err(|detail| ConstellationError::DurabilityUnavailable { detail })?;
    windows_wrapped_uri_for_path(path)
}

#[cfg(windows)]
fn windows_wrapped_uri_for_path(path: &Path) -> Result<String, ConstellationError> {
    if !path.is_absolute()
        || !matches!(path.components().next(), Some(std::path::Component::Prefix(prefix)) if matches!(prefix.kind(), std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)))
    {
        return Err(ConstellationError::DurabilityUnavailable {
            detail: format!("{} is not an absolute local drive path", path.display()),
        });
    }
    let key = object_store::path::Path::from_absolute_path(path)
        .map_err(|error| ConstellationError::DurabilityUnavailable {
            detail: format!("cannot encode local path {}: {error}", path.display()),
        })?;
    let file_url = reqwest::Url::from_file_path(path).map_err(|_| ConstellationError::DurabilityUnavailable {
        detail: format!("cannot encode local file URL for {}", path.display()),
    })?;
    let actual = object_store::local::LocalFileSystem::new()
        .path_to_filesystem(&key)
        .map_err(|error| ConstellationError::DurabilityUnavailable {
            detail: format!("cannot resolve local path {}: {error}", path.display()),
        })?;
    if !path_is_within(path, &actual) || !path_is_within(&actual, path) {
        return Err(ConstellationError::DurabilityUnavailable {
            detail: format!("wrapped URI does not resolve to {}", path.display()),
        });
    }
    Ok(file_url.as_str().replacen("file:", "file-object-store:", 1))
}

/// Create directories through the Windows write-through namespace primitive.
#[cfg(windows)]
pub(crate) fn ensure_dir(path: &Path) -> Result<(), ConstellationError> {
    WindowsNtfsDurability.ensure_dir(path)
}

/// Move on one volume without replacing an existing destination.
#[cfg(windows)]
pub(crate) fn move_noreplace_write_through(
    src: &Path,
    dest: &Path,
) -> Result<(), ConstellationError> {
    WindowsNtfsDurability.move_write_through(src, dest, false)
}


/// Seal an exclusively owned, unpublished staging tree. An interrupted seal
/// leaves the entire stage unusable and the caller must rebuild a fresh stage.
#[cfg(windows)]
pub(crate) fn seal_unpublished_tree(root: &Path) -> Result<(), ConstellationError> {
    windows::seal_unpublished_tree(root)
}

/// Wrap the local Lance store for a private legacy migration stage.
#[cfg(windows)]
pub(crate) fn tracked_legacy_store(
    root: &Path,
    tracker: Arc<StageTracker>,
    platform: Arc<dyn PlatformDurability>,
) -> Arc<DurabilityTrackingStore> {
    Arc::new(DurabilityTrackingStore::for_private_stage(root, tracker, platform))
}

/// POSIX side of [`verify_volume`]: no volume check applies, because the
/// barrier itself (`sync_all` on files and directories) is available
/// wherever those calls work.
#[cfg(not(windows))]
fn verify_volume(_index_root: &Path) -> Result<(), String> {
    Ok(())
}

/// Classify the filesystem backing `index_root`.
///
/// Returns the support class plus a diagnostic when the root is unusable as
/// a canonical, non-aliased physical index location. A missing root is *not*
/// an error here: bootstrap creates it after ownership is established, and a
/// first run must be able to ask this question.
///
/// The diagnostic carries no `source` path: this is about the index root the
/// caller just handed us, not about a workspace artifact, and repeating an
/// absolute path back adds nothing the caller does not already have.
pub(crate) fn classify_index_root(
    index_root: &Path,
) -> (FilesystemSupport, Option<SourceDiagnostic>) {
    match std::fs::symlink_metadata(index_root) {
        Ok(meta) if !meta.is_dir() => {
            return (
                FilesystemSupport::Unsupported,
                Some(SourceDiagnostic::new(
                    DiagnosticCode::UnsupportedIndexFilesystem,
                    None,
                    "index root exists but is not a directory",
                )),
            );
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return (
                FilesystemSupport::Unsupported,
                Some(SourceDiagnostic::new(
                    DiagnosticCode::UnsupportedIndexFilesystem,
                    None,
                    format!("cannot inspect index root: {error}"),
                )),
            );
        }
    }

    if let Err(detail) = verify_volume(index_root) {
        return (
            FilesystemSupport::Unsupported,
            Some(SourceDiagnostic::new(
                DiagnosticCode::UnsupportedIndexFilesystem,
                None,
                detail,
            )),
        );
    }

    match root_is_own(index_root) {
        Ok(()) => (FilesystemSupport::Supported, None),
        Err(detail) => (
            FilesystemSupport::Unsupported,
            Some(SourceDiagnostic::new(
                DiagnosticCode::UnsupportedIndexFilesystem,
                None,
                detail,
            )),
        ),
    }
}

#[cfg(not(windows))]
use posix::root_is_own;
#[cfg(windows)]
use windows::root_is_own;

// ── pinned-read identity ────────────────────────────────────────────────
//
// One seam, two implementations, and no size/mtime branch anywhere. The pin in
// [`super::storage::pin_generation_read`] asks two questions through it:
//
// * does `name` still resolve to the file `locked` holds open?
// * how do I open an immutable manifest so GC cannot unlink it while I read?

/// The kernel's own statement of which physical object a file is.
///
/// One type for both platforms so the pin's re-check is written once:
///
/// * **Unix** — `volume` is `st_dev` and `id` carries `st_ino` big-endian.
/// * **Windows** — `volume` is the NTFS volume serial and `id` is the full
///   128-bit `FILE_ID_INFO.FileId`.
///
/// # Why not size + mtime
///
/// It was this module's Windows fallback and it decides the wrong answer on
/// precisely the cases the protocol cares about: a same-length rewrite inside
/// one mtime tick compares equal, and `git checkout`, `tar -x` and archive
/// restores preserve mtime outright. A pinned reader that accepted that
/// comparison would serve a generation whose manifest no longer exists under
/// that name — the exact "torn pointer" the pin exists to refuse.
///
/// `volume` is part of the identity rather than decoration: file ids are unique
/// only within one volume, and an index root is reachable through mount points
/// and substituted directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PhysicalIdentity {
    pub(crate) volume: u64,
    pub(crate) id: [u8; 16],
}

impl std::fmt::Display for PhysicalIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "vol={:016x} id=", self.volume)?;
        for byte in &self.id {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Is the file `locked` holds open still the file `name` resolves to?
///
/// Unix compares `(device, inode)`; Windows compares NTFS `FILE_ID_INFO`
/// (volume serial + 128-bit file id). Both are the kernel's own statement of
/// which object a name and a descriptor refer to.
///
/// # Why size + mtime is not an answer
///
/// It was the fallback here for a long time and it decides the wrong answer on
/// both platforms' failure modes: a same-length rewrite inside one mtime tick
/// compares equal, and `git checkout`, `tar -x` and archive restores preserve
/// mtime outright. A reader that accepted that comparison would serve a
/// generation whose manifest no longer exists under that name.
///
/// # Errors
///
/// The platform could not produce an identity — an unopenable file, a handle
/// whose `FileIdInfo` the kernel refused, a filesystem with no file id. The
/// caller reports the generation unavailable; "we could not check" never
/// degrades into "we checked".
#[cfg(windows)]
pub(crate) fn same_physical_file(locked: &std::fs::File, name: &Path) -> Result<bool, String> {
    let locked_id = physical_identity_of(locked)?;
    // A name that no longer resolves is not "same file": the file was unlinked,
    // and the pin is now serving a generation with no name. That is reported as
    // an error rather than as a match.
    let named_id = physical_identity_of_path(name)
        .map_err(|detail| format!("cannot resolve {name:?} for an identity re-check: {detail}"))?;
    Ok(locked_id == named_id)
}

#[cfg(unix)]
pub(crate) fn same_physical_file(locked: &std::fs::File, name: &Path) -> Result<bool, String> {
    let locked_id = physical_identity_of(locked)?;
    let named_id = physical_identity_of_path(name)
        .map_err(|detail| format!("cannot resolve {name:?} for an identity re-check: {detail}"))?;
    Ok(locked_id == named_id)
}

/// The physical identity of an open file or directory descriptor.
#[cfg(windows)]
pub(crate) fn physical_identity_of(file: &std::fs::File) -> Result<PhysicalIdentity, String> {
    use std::os::windows::io::AsRawHandle;
    // `std`'s `RawHandle` and `HANDLE` are both `*mut core::ffi::c_void`.
    windows::identity_from_handle(file.as_raw_handle(), "the open file or directory handle")
}

#[cfg(unix)]
pub(crate) fn physical_identity_of(file: &std::fs::File) -> Result<PhysicalIdentity, String> {
    use std::os::unix::fs::MetadataExt;
    let meta = file.metadata().map_err(|e| e.to_string())?;
    Ok(unix_identity(meta.dev(), meta.ino()))
}

/// `(device, inode)` rendered into the shared identity shape.
#[cfg(unix)]
fn unix_identity(dev: u64, ino: u64) -> PhysicalIdentity {
    let mut id = [0u8; 16];
    id[..8].copy_from_slice(&ino.to_be_bytes());
    PhysicalIdentity { volume: dev, id }
}

/// The physical identity of whatever `path` currently names.
///
/// Opened independently of any pin, with whatever sharing the platform needs to
/// succeed while a writer holds the file — this is a re-check, not a guard.
#[cfg(windows)]
pub(crate) fn physical_identity_of_path(path: &Path) -> Result<PhysicalIdentity, String> {
    windows::identity_of_path(path)
}

#[cfg(unix)]
pub(crate) fn physical_identity_of_path(path: &Path) -> Result<PhysicalIdentity, String> {
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    Ok(unix_identity(meta.dev(), meta.ino()))
}

/// Open an immutable generation manifest for a pinned read.
///
/// Never creates, never writes, never truncates, and works on a read-only mount
/// on both platforms. On Windows it additionally withholds `FILE_SHARE_DELETE`,
/// so the kernel refuses to unlink or replace the manifest while the pin is
/// alive — the guarantee POSIX gets from "an unlinked file cannot be reopened
/// by name" plus `flock`.
///
/// # Errors
///
/// A platform detail naming the path and the Win32/POSIX error that stopped the
/// open. `Pin` callers map this to `index_generation_unavailable`.
#[cfg(windows)]
pub(crate) fn open_pinned_manifest(path: &Path) -> Result<std::fs::File, String> {
    windows::open_manifest_for_pinning(path)
}

#[cfg(not(windows))]
pub(crate) fn open_pinned_manifest(path: &Path) -> Result<std::fs::File, String> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .truncate(false)
        .open(path)
        .map_err(|e| e.to_string())
}

/// The write barrier for one filesystem.
///
/// P2c attaches `WindowsNtfsDurability` to this trait. The POSIX
/// implementation in [`posix`] is the reference shape the NTFS one mirrors:
/// every method is a *barrier*, never a repair, and none of them may silently
/// downgrade to a weaker operation.
pub(crate) trait PlatformDurability: std::fmt::Debug + Send + Sync {
    /// Human-readable name of this adapter, for diagnostics.
    fn adapter_name(&self) -> &'static str;

    /// Flush one existing regular file's data **and metadata**, without
    /// following a symlink.
    ///
    /// Symlinks are refused rather than followed: the index owns its own
    /// namespace, and a link substitution inside it is an identity change,
    /// not a file write.
    fn sync_file(&self, path: &Path) -> Result<(), ConstellationError>;

    /// Flush one directory so creations and renames inside it survive.
    ///
    /// Called bottom-up, deepest first, and the root last: a rename is only
    /// durable once the *containing* directory entry is durable, and the
    /// containing directory is only durable once its own parent says so.
    fn sync_dir(&self, path: &Path) -> Result<(), ConstellationError>;

    /// Create `dir` and any missing parent, so that a child write issued
    /// afterwards has somewhere to land.
    ///
    /// Called *before* delegating to the inner store: a library
    /// `create_dir_all` reaching the filesystem on its own would be an
    /// unregistered fast path, invisible to the tracker and to the barrier.
    fn ensure_dir(&self, dir: &Path) -> Result<(), ConstellationError>;

    /// Replace `dest` with `src` atomically on **one filesystem**.
    ///
    /// This is the visibility point. It returns once the new name is
    /// visible; durability is a separate, later guarantee.
    fn replace_atomically(&self, src: &Path, dest: &Path) -> Result<(), ConstellationError>;
}

/// Install an immutable file under `dir` and never replace an existing one.
///
/// The two platforms need genuinely different sequences for the same
/// guarantee, which is why the dispatch lives here rather than in the caller:
///
/// * **NTFS** goes through [`windows::WindowsNtfsDurability::install_new_file`]:
///   `CreateDirectoryW` parents pre-created write-through, a
///   `CreateFileW(CREATE_NEW, …, FILE_FLAG_WRITE_THROUGH)` temporary, all bytes
///   written, `FlushFileBuffers`, handle closed, then a same-volume
///   `MoveFileExW(MOVEFILE_WRITE_THROUGH)` **without** `REPLACE_EXISTING`.
/// * **POSIX** has no such flag, so the equivalent is `fs::rename` over a
///   pre-created parent, with the durability carried by the closure barrier
///   that runs before this and the directory sync that runs after.
///
/// Both refuse to replace, so a second publication under the same generation
/// id is an error rather than a silent overwrite.
pub(crate) fn install_immutable_file(
    dir: &Path,
    name: &str,
    bytes: &[u8],
    platform: &dyn PlatformDurability,
) -> Result<(), ConstellationError> {
    #[cfg(windows)]
    {
        let _ = platform;
        windows::WindowsNtfsDurability.install_new_file(dir, name, bytes)
    }
    #[cfg(not(windows))]
    {
        let _ = platform.adapter_name();
        platform.ensure_dir(dir)?;
        let dest = dir.join(name);
        if dest.exists() {
            return Err(ConstellationError::IndexProtocolMismatch {
                detail: format!("{name} already exists; an immutable file must not be replaced"),
            });
        }
        let temp = dir.join(format!(".{name}.tmp-{}", std::process::id()));
        std::fs::write(&temp, bytes).map_err(|e| ConstellationError::Io {
            detail: format!("cannot write {}: {e}", temp.display()),
        })?;
        // The generation-manifest bytes are completely written; this is
        // immediately before the file fsync. Inside this function rather than
        // in the publisher that called it, because "before the fsync" is only
        // meaningful at the fsync.
        // `crate::constellation::fault_barrier_if_selected`, not
        // `super::super::…`: the function is re-exported from the
        // `constellation` module itself, and `super::super` from inside
        // `constellation::durability` is `constellation`, so the two are only
        // equivalent if the re-export is what is meant — naming it through the
        // crate root keeps the one canonical path visible at every call site
        // and makes a re-export removal a compile error rather than a silent
        // second route to the same no-op.
        crate::constellation::fault_barrier_if_selected("before_manifest_sync");
        // Durable bytes before a durable name — the same rule as
        // `write_bytes_via_temp`: an immutable file that survives its own
        // rename pointing at unwritten data is worse than no file.
        if let Err(e) = platform.sync_file(&temp) {
            let _ = std::fs::remove_file(&temp);
            return Err(e);
        }
        if let Err(e) = platform.replace_atomically(&temp, &dest) {
            let _ = std::fs::remove_file(&temp);
            return Err(e);
        }
        Ok(())
    }
}

/// Reference barrier: POSIX `sync_all` on files, then directories bottom-up.
///
/// Synchronising a directory that does not exist yet is not an error here —
/// the caller syncs the *changed* directories, and a directory that was
/// created and then removed inside the same stage has nothing left to sync.
/// Its parent's entry was still recorded and is still synced.
#[derive(Debug, Default)]
pub(crate) struct PosixDurability;

impl PlatformDurability for PosixDurability {
    fn adapter_name(&self) -> &'static str {
        "posix-sync-all"
    }

    fn sync_file(&self, path: &Path) -> Result<(), ConstellationError> {
        // Reject a symlink before opening rather than opening through one.
        // `O_NOFOLLOW` would close the check-open race too, but the core
        // crate's `libc` dependency is unix-only and this adapter is compiled
        // on every target — including the Windows build, where the NTFS
        // adapter attaches instead. Stating the residual race beats claiming
        // a guarantee the code does not have: on POSIX the index namespace is
        // only mutated by cooperating processes, and the owning writer holds
        // the physical owner lock for the whole stage.
        match std::fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(ConstellationError::DurabilityUnavailable {
                    detail: format!(
                        "{} is a symlink; the index will not sync through an alias",
                        path.display()
                    ),
                });
            }
            Ok(_) => {}
            Err(e) => return Err(io_error("stat before sync", path, e)),
        }

        let file = std::fs::File::open(path).map_err(|e| io_error("open for sync", path, e))?;
        file.sync_all()
            .map_err(|e| io_error("sync_all file", path, e))
    }

    fn sync_dir(&self, path: &Path) -> Result<(), ConstellationError> {
        // A directory opened for reading and `sync_all`ed is the portable way
        // to fsync a directory's entries. macOS and Linux both accept it;
        // opening a directory read-write is not permitted everywhere.
        let dir = std::fs::File::open(path).map_err(|e| io_error("open dir for sync", path, e))?;
        dir.sync_all()
            .map_err(|e| io_error("sync_all dir", path, e))
    }

    fn ensure_dir(&self, dir: &Path) -> Result<(), ConstellationError> {
        match std::fs::create_dir_all(dir) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
            Err(e) => Err(io_error("create_dir_all", dir, e)),
        }
    }

    fn replace_atomically(&self, src: &Path, dest: &Path) -> Result<(), ConstellationError> {
        std::fs::rename(src, dest).map_err(|e| io_error("atomic replace", dest, e))
    }
}

fn io_error(what: &str, path: &Path, error: std::io::Error) -> ConstellationError {
    ConstellationError::Io {
        detail: format!("{what} {}: {error}", path.display()),
    }
}

/// Deduplicated closure of one stage: the final files that changed and the
/// directories whose entries did.
///
/// Explicitly **not** the whole corpus and **not** guessed from a `.arrow`
/// suffix. A suffix heuristic would silently omit a file Lance wrote under
/// another name — and the omitted file is exactly the one that fails to
/// survive a power cut.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WriteSet {
    /// Relative paths of files that exist and changed.
    pub files: BTreeSet<String>,
    /// Relative paths of directories that were created or had an entry
    /// added, changed or removed.
    pub directories: BTreeSet<String>,
    /// Relative paths removed by a confirmed delete or rename-away.
    pub deletions: BTreeSet<String>,
}

impl WriteSet {
    /// Whether this closure is empty.
    ///
    /// An empty closure is legal — a stage that changed nothing still has to
    /// be able to publish an identical generation — so this is not an error.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.directories.is_empty() && self.deletions.is_empty()
    }
}

/// Outcome of a completed closure barrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurableStage {
    pub files_synced: usize,
    pub directories_synced: usize,
}

#[derive(Debug, Default)]
struct StageState {
    epoch: u64,
    stage_open: bool,
    poisoned: Option<String>,
    in_flight: usize,
    files: BTreeSet<String>,
    directories: BTreeSet<String>,
    deletions: BTreeSet<String>,
    /// Intent registered before a mutation was attempted but not yet
    /// confirmed. A confirmed rename or delete may retire one of these.
    pending: BTreeSet<String>,
    /// `(from, to)` of renames that completed.
    completed_renames: Vec<(String, String)>,
}

/// Shared record of what one stage's storage writes actually touched.
///
/// One tracker serves **both** tables. Two trackers would each see half the
/// closure, and `finish_writes` on either would report the other half as
/// untouched — so the generation would be published on the strength of a
/// barrier that never ran over half its data.
#[derive(Debug, Default)]
pub struct StageTracker {
    state: Mutex<StageState>,
}

impl StageTracker {
    /// Open a stage for `epoch`, discarding whatever the previous one left.
    pub fn begin_stage(&self, epoch: u64) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        *state = StageState {
            epoch,
            stage_open: true,
            ..StageState::default()
        };
    }

    /// The epoch of the open stage.
    pub fn epoch(&self) -> u64 {
        self.state.lock().map(|s| s.epoch).unwrap_or_default()
    }

    /// Whether a mutation failed or was aborted, making the stage unusable.
    pub fn is_poisoned(&self) -> bool {
        self.state
            .lock()
            .map(|s| s.poisoned.is_some())
            .unwrap_or(true)
    }

    /// Record the intent to mutate `path`, before delegating.
    ///
    /// Registration happens first so that a crash *during* the write leaves a
    /// pending entry — an unknown destination — rather than nothing at all.
    pub fn register_intent(&self, path: &str) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.in_flight += 1;
        state.pending.insert(path.to_string());
    }

    /// Record a completed write to `path`.
    pub fn record_write(&self, path: &str) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.in_flight = state.in_flight.saturating_sub(1);
        state.pending.remove(path);
        state.files.insert(path.to_string());
        mark_ancestors(&mut state.directories, path);
    }

    /// Record a completed copy to `to`, leaving `from` in place.
    pub fn record_copy(&self, from: &str, to: &str) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.in_flight = state.in_flight.saturating_sub(1);
        state.pending.remove(from);
        state.files.insert(to.to_string());
        mark_ancestors(&mut state.directories, to);
    }

    /// Record a completed rename `from` → `to`.
    ///
    /// **Both** parents become changed directories: a rename mutates the
    /// source's parent (an entry disappeared) and the destination's parent
    /// (an entry appeared). Syncing only one leaves the other unproven.
    pub fn record_rename(&self, from: &str, to: &str) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.in_flight = state.in_flight.saturating_sub(1);
        state.pending.remove(from);
        state.files.insert(to.to_string());
        state
            .completed_renames
            .push((from.to_string(), to.to_string()));
        mark_ancestors(&mut state.directories, from);
        mark_ancestors(&mut state.directories, to);
    }

    /// Record a completed delete of `path`.
    pub fn record_delete(&self, path: &str) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.in_flight = state.in_flight.saturating_sub(1);
        state.pending.remove(path);
        state.deletions.insert(path.to_string());
        mark_ancestors(&mut state.directories, path);
    }

    /// Poison the stage.
    ///
    /// A failed or aborted mutation makes the whole closure untrustworthy: we
    /// cannot know how far the inner store got, and an unacknowledged
    /// destination must never be treated as absent — that is how a
    /// half-written object gets published as a finished one.
    pub fn poison(&self, detail: impl Into<String>) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let detail = detail.into();
        state.poisoned = Some(match state.poisoned.take() {
            Some(existing) => format!("{existing}; {detail}"),
            None => detail,
        });
    }

    /// Close the stage and return its closure.
    ///
    /// Only legal once **every** Lance commit, restore, migration and metadata
    /// operation has joined: `in_flight == 0`, no background optimize, no
    /// cleanup. A `WriteSet` taken while a child is still writing is a
    /// snapshot of an unfinished closure, and syncing it would ACK data that
    /// has not been written yet.
    pub fn finish_writes(&self) -> Result<WriteSet, ConstellationError> {
        let mut state = self.state.lock().map_err(|_| ConstellationError::Io {
            detail: "stage tracker mutex poisoned".to_string(),
        })?;

        if let Some(detail) = &state.poisoned {
            return Err(ConstellationError::Io {
                detail: format!("stage {} is poisoned: {detail}", state.epoch),
            });
        }
        if state.in_flight > 0 {
            return Err(ConstellationError::Io {
                detail: format!(
                    "stage {} still has {} unfinished writes; a closure cannot be taken until every commit has joined",
                    state.epoch, state.in_flight
                ),
            });
        }
        if !state.pending.is_empty() {
            // A pending entry with nothing in flight means a mutation
            // registered itself and then neither confirmed nor failed.
            return Err(ConstellationError::Io {
                detail: format!(
                    "stage {} has unresolved destinations: {:?}",
                    state.epoch, state.pending
                ),
            });
        }

        state.stage_open = false;
        Ok(WriteSet {
            files: state.files.clone(),
            directories: state.directories.clone(),
            deletions: state.deletions.clone(),
        })
    }

    /// Run the closure barrier over `write_set` with `platform`.
    ///
    /// Order is files, then directories deepest-first, then the root — and it
    /// is a **conservative superset**: a touched file that is already gone is
    /// tolerated when a completed rename or delete explains it, and treated
    /// as `storage_io` when nothing does.
    /// `pub(crate)`, not `pub`. `StageTracker` itself is re-exported from the
    /// `constellation` root, but a `&dyn PlatformDurability` cannot be named
    /// or obtained outside this crate — the module is private and the only
    /// producer, `platform_barrier`, is `pub(crate)`. A `pub` signature here
    /// would advertise an argument no caller can supply.
    pub(crate) fn sync_written_set(
        &self,
        write_set: &WriteSet,
        index_root: &Path,
        platform: &dyn PlatformDurability,
    ) -> Result<DurableStage, ConstellationError> {
        let explained = self.explained_disappearances();

        let mut files_synced = 0usize;
        for relative in &write_set.files {
            // Recorded keys are filesystem-root-relative — lance addresses
            // the `file-object-store` store by absolute path, and the
            // `ObjectPath` key it hands us is that path minus the leading
            // `/`. Joining `index_root` would double the path into a file
            // that never existed; the correct resolution is from the root,
            // and confinement is what `index_root` is then FOR: a tracked
            // write that lands outside it is a violation, not a name.
            let path = tracked_filesystem_path(relative)?;
            if !path_is_within(&path, index_root) {
                return Err(ConstellationError::DurabilityUnavailable {
                    detail: format!(
                        "touched file {} is outside the index root; the stage must not \
                         write beyond its own root",
                        relative
                    ),
                });
            }
            match std::fs::symlink_metadata(&path) {
                Ok(meta) if meta.is_file() => {
                    platform.sync_file(&path)?;
                    files_synced += 1;
                }
                Ok(_) => {
                    return Err(ConstellationError::DurabilityUnavailable {
                        detail: format!(
                            "touched path {} is not a regular file; the index namespace must not be \
                             replaced by a symlink or a directory underneath a writer",
                            relative
                        ),
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    if explained.contains(relative) {
                        // An upload temporary that a completed rename or
                        // delete already moved or removed is expected to be
                        // absent. Anything else is an unexplained hole.
                        continue;
                    }
                    return Err(ConstellationError::StorageIo {
                        detail: format!(
                            "touched file {} is missing with no completed rename or delete to \
                             explain it",
                            relative
                        ),
                    });
                }
                Err(e) => return Err(io_error("stat touched file", &path, e)),
            }
        }

        // Directories deepest-first: a rename inside `a/b/c` is only durable
        // once `a/b/c`, then `a/b`, then `a` all have their entries flushed.
        let mut directories: Vec<&String> = write_set.directories.iter().collect();
        directories.sort_by_key(|dir| std::cmp::Reverse(dir.matches('/').count()));
        let mut directories_synced = 0usize;
        for relative in directories {
            // Same resolution rule as files: keys are fs-root-relative.
            // Parent directories are recorded all the way up to the volume
            // root, and ancestors above the index root are not this stage's
            // durability domain — they are skipped, not refused: a written
            // FILE outside the root is rejected above, while a shared
            // ancestor is just metadata about where that file lives.
            let path = tracked_filesystem_path(relative)?;
            if !path_is_within(&path, index_root) {
                continue;
            }
            if path.is_dir() {
                platform.sync_dir(&path)?;
                directories_synced += 1;
            }
        }
        platform.sync_dir(index_root)?;

        Ok(DurableStage {
            files_synced,
            directories_synced,
        })
    }

    /// Touched files whose disappearance a completed rename or delete
    /// accounts for.
    fn explained_disappearances(&self) -> BTreeSet<String> {
        let Ok(state) = self.state.lock() else {
            return BTreeSet::new();
        };
        // Owned `String`s: returning borrowed keys would tie the result to the
        // mutex guard, which is dropped here.
        let mut explained = BTreeSet::new();
        for (from, _) in &state.completed_renames {
            explained.insert(from.clone());
        }
        for deleted in &state.deletions {
            explained.insert(deleted.clone());
        }
        explained
    }
}

#[cfg(windows)]
fn tracked_filesystem_path(key: &str) -> Result<std::path::PathBuf, ConstellationError> {
    let path = object_store::path::Path::parse(key).map_err(|error| {
        ConstellationError::DurabilityUnavailable { detail: format!("invalid tracked path {key}: {error}") }
    })?;
    object_store::local::LocalFileSystem::new()
        .path_to_filesystem(&path)
        .map_err(|error| ConstellationError::DurabilityUnavailable {
            detail: format!("cannot resolve tracked path {key}: {error}"),
        })
}

#[cfg(not(windows))]
fn tracked_filesystem_path(key: &str) -> Result<std::path::PathBuf, ConstellationError> {
    Ok(Path::new("/").join(key))
}

#[cfg(not(windows))]
fn path_is_within(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

/// Compare local drive paths by components, accepting both ordinary and
/// verbatim drive prefixes. Name components compare exactly to remain safe on
/// case-sensitive NTFS directories.
#[cfg(windows)]
fn path_is_within(path: &Path, root: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Component, Prefix};

    fn eq_component(a: Component<'_>, b: Component<'_>) -> bool {
        match (a, b) {
            (Component::Prefix(a), Component::Prefix(b)) => {
                let drive = |prefix| match prefix {
                    Prefix::Disk(d) | Prefix::VerbatimDisk(d) => Some(d.to_ascii_uppercase()),
                    _ => None,
                };
                matches!((drive(a.kind()), drive(b.kind())), (Some(a), Some(b)) if a == b)
            }
            (Component::RootDir, Component::RootDir) => true,
            (Component::Normal(a), Component::Normal(b)) => {
                a.encode_wide().eq(b.encode_wide())
            }
            _ => false,
        }
    }
    if !path.is_absolute() || !root.is_absolute() {
        return false;
    }
    let mut path_components = path.components();
    for root_component in root.components() {
        if !path_components.next().is_some_and(|component| eq_component(component, root_component)) {
            return false;
        }
    }
    !path_components.any(|component| matches!(component, Component::ParentDir | Component::CurDir | Component::Prefix(_) | Component::RootDir))
}

/// Parent of a relative object path, or `None` at the root.
///
/// Object-store paths always use `/`, including on Windows, so this splits on
/// `/` rather than on a platform separator.
fn parent_of(path: &str) -> Option<&str> {
    path.rfind('/').map(|index| &path[..index])
}

/// Insert `path`'s parent and every ancestor above it into `directories`.
///
/// A directory fsync persists only that directory's entries. When a write
/// lands at `t/data/nested/a.arrow`, the entry was added to `t/data/nested`
/// — but that directory's own mtime changed `t/data`, and so on up to the
/// store root. Recording only the immediate parent would leave the higher
/// levels unacknowledged and a crash could still lose the whole subtree.
fn mark_ancestors(directories: &mut BTreeSet<String>, path: &str) {
    let mut current = parent_of(path);
    while let Some(dir) = current {
        directories.insert(dir.to_string());
        current = parent_of(dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[cfg(windows)]
    #[test]
    fn windows_confinement_accepts_verbatim_drives_but_not_sibling_or_case_alias() {
        assert!(path_is_within(Path::new(r"C:\Data\stage\file"), Path::new(r"\\?\C:\Data\stage")));
        assert!(path_is_within(Path::new(r"\\?\C:\Data\stage\file"), Path::new(r"C:\Data\stage")));
        assert!(!path_is_within(Path::new(r"C:\Data\stage-other\file"), Path::new(r"C:\Data\stage")));
        assert!(!path_is_within(Path::new(r"C:\data\stage\file"), Path::new(r"C:\Data\stage")));
        assert!(!path_is_within(Path::new(r"D:\Data\stage\file"), Path::new(r"C:\Data\stage")));
        assert!(!path_is_within(Path::new(r"C:\Data\stage\..\outside"), Path::new(r"C:\Data\stage")));
    }

    #[cfg(windows)]
    #[test]
    fn wrapped_uri_handles_normal_and_verbatim_paths_without_loss() {
        let normal = Path::new(r"C:\Data\stage\table.lance");
        let verbatim = Path::new(r"\\?\C:\Data\stage\table.lance");
        assert_eq!(windows_wrapped_uri_for_path(normal).unwrap(), "file-object-store:///C:/Data/stage/table.lance");
        assert_eq!(windows_wrapped_uri_for_path(verbatim).unwrap(), "file-object-store:///C:/Data/stage/table.lance");
        assert_eq!(windows_wrapped_uri_for_path(Path::new(r"C:\Data\has space")).unwrap(), "file-object-store:///C:/Data/has%20space");
        assert!(windows_wrapped_uri_for_path(Path::new(r"\\server\share\data")).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn live_barrier_is_never_selected() {
        let tmp = TempDir::new().unwrap();
        let (support, barrier, diagnostic) = platform_barrier(tmp.path());
        assert_eq!(support, FilesystemSupport::Unsupported);
        assert!(barrier.is_none());
        assert!(diagnostic.is_some());
    }

    #[test]
    fn parent_of_splits_on_the_object_store_separator() {
        assert_eq!(parent_of("a/b/c.lance"), Some("a/b"));
        assert_eq!(parent_of("c.lance"), None);
    }

    #[test]
    fn a_stage_records_writes_parents_and_deletions() {
        let tracker = StageTracker::default();
        tracker.begin_stage(4);
        tracker.register_intent("t/data/0.arrow");
        tracker.record_write("t/data/0.arrow");
        tracker.register_intent("t/old.lance");
        tracker.record_delete("t/old.lance");

        let set = tracker.finish_writes().unwrap();
        assert_eq!(
            set.files.iter().cloned().collect::<Vec<_>>(),
            ["t/data/0.arrow"]
        );
        assert_eq!(
            set.deletions.iter().cloned().collect::<Vec<_>>(),
            ["t/old.lance"]
        );
        assert!(
            set.directories.contains("t/data"),
            "the containing directory changed and must be synced"
        );
        assert!(set.directories.contains("t"));
        assert!(!set.is_empty());
    }

    #[test]
    fn a_rename_marks_both_parents_changed() {
        let tracker = StageTracker::default();
        tracker.begin_stage(1);
        tracker.register_intent("old/dir/a.lance");
        tracker.record_rename("old/dir/a.lance", "new/dir/a.lance");
        let set = tracker.finish_writes().unwrap();
        assert!(set.directories.contains("old/dir"), "source parent changed");
        assert!(
            set.directories.contains("new/dir"),
            "destination parent changed"
        );
        assert!(set.files.contains("new/dir/a.lance"));
    }

    #[test]
    fn finish_writes_refuses_while_a_write_is_unfinished() {
        let tracker = StageTracker::default();
        tracker.begin_stage(1);
        tracker.register_intent("t/pending.arrow");
        let err = tracker.finish_writes().unwrap_err();
        assert!(
            err.to_string().contains("unfinished writes"),
            "a closure taken mid-flight would ACK data that has not been written: {err}"
        );
    }

    #[test]
    fn a_failed_mutation_poisons_the_stage_and_is_not_silently_dropped() {
        let tracker = StageTracker::default();
        tracker.begin_stage(1);
        tracker.register_intent("t/half.arrow");
        tracker.poison("multipart abort");
        assert!(tracker.is_poisoned());
        let err = tracker.finish_writes().unwrap_err();
        assert!(err.to_string().contains("poisoned"), "{err}");
    }

    #[test]
    fn poisoning_twice_keeps_both_reasons() {
        let tracker = StageTracker::default();
        tracker.begin_stage(1);
        tracker.poison("first");
        tracker.poison("second");
        let err = tracker.finish_writes().unwrap_err();
        assert!(err.to_string().contains("first"), "{err}");
        assert!(err.to_string().contains("second"), "{err}");
    }

    #[test]
    fn begin_stage_discards_the_previous_stage() {
        let tracker = StageTracker::default();
        tracker.begin_stage(1);
        tracker.register_intent("t/a.arrow");
        tracker.record_write("t/a.arrow");
        tracker.begin_stage(2);
        assert_eq!(tracker.epoch(), 2);
        assert!(tracker.finish_writes().unwrap().is_empty());
    }

    /// The object-store keys a `file-object-store` backend hands the tracker
    /// are filesystem-root-relative (`abs/path` without the leading `/`),
    /// never index-root-relative. Fixtures must record keys in that shape.
    fn fs_key(index_root: &Path, relative: &str) -> String {
        index_root
            .join(relative)
            .to_string_lossy()
            .trim_start_matches('/')
            .to_string()
    }

    #[test]
    fn a_missing_touched_file_without_an_explanation_is_storage_io() {
        let tmp = TempDir::new().unwrap();
        let tracker = StageTracker::default();
        tracker.begin_stage(1);
        tracker.register_intent(&fs_key(tmp.path(), "t/gone.arrow"));
        tracker.record_write(&fs_key(tmp.path(), "t/gone.arrow"));
        let set = tracker.finish_writes().unwrap();

        let err = tracker
            .sync_written_set(&set, tmp.path(), &PosixDurability)
            .unwrap_err();
        assert_eq!(err.code(), DiagnosticCode::StorageIo);
        assert!(
            err.to_string().contains("no completed rename or delete"),
            "{err}"
        );
    }

    #[test]
    fn a_missing_file_explained_by_a_rename_is_tolerated() {
        let tmp = TempDir::new().unwrap();
        let tracker = StageTracker::default();
        tracker.begin_stage(1);
        // Only the source carries an intent — the destination's arrival is
        // the rename itself, and `record_rename` resolves it. Registering
        // `final.lance` here would leave an intent nothing ever resolves.
        tracker.register_intent(&fs_key(tmp.path(), "t/upload.tmp"));
        tracker.record_rename(
            &fs_key(tmp.path(), "t/upload.tmp"),
            &fs_key(tmp.path(), "t/final.lance"),
        );
        // The temporary is genuinely gone; the destination exists.
        std::fs::create_dir_all(tmp.path().join("t")).unwrap();
        std::fs::write(tmp.path().join("t/final.lance"), b"data").unwrap();

        let set = tracker.finish_writes().unwrap();
        let stage = tracker
            .sync_written_set(&set, tmp.path(), &PosixDurability)
            .unwrap();
        assert_eq!(stage.files_synced, 1);
    }

    #[test]
    fn directories_are_synced_deepest_first_and_the_root_last() {
        let tmp = TempDir::new().unwrap();
        for dir in ["t", "t/data", "t/data/nested"] {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        let file = tmp.path().join("t/data/nested/a.arrow");
        std::fs::write(&file, b"payload").unwrap();

        let tracker = StageTracker::default();
        tracker.begin_stage(1);
        tracker.register_intent(&fs_key(tmp.path(), "t/data/nested/a.arrow"));
        tracker.record_write(&fs_key(tmp.path(), "t/data/nested/a.arrow"));
        let set = tracker.finish_writes().unwrap();

        let stage = tracker
            .sync_written_set(&set, tmp.path(), &PosixDurability)
            .unwrap();
        assert_eq!(stage.files_synced, 1);
        assert_eq!(
            stage.directories_synced, 4,
            "nested, data, t and the index root itself are all changed \
             directories — the fs-root-relative key records every ancestor              inside the root"
        );
    }

    #[test]
    fn a_symlink_where_a_touched_file_should_be_is_refused() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("t")).unwrap();
        let real = tmp.path().join("real.arrow");
        std::fs::write(&real, b"payload").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real, tmp.path().join("t/link.arrow")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&real, tmp.path().join("t/link.arrow")).unwrap();

        let tracker = StageTracker::default();
        tracker.begin_stage(1);
        tracker.register_intent(&fs_key(tmp.path(), "t/link.arrow"));
        tracker.record_write(&fs_key(tmp.path(), "t/link.arrow"));
        let set = tracker.finish_writes().unwrap();

        let err = tracker
            .sync_written_set(&set, tmp.path(), &PosixDurability)
            .unwrap_err();
        assert_eq!(err.code(), DiagnosticCode::DurabilityUnavailable);
    }

    #[test]
    fn an_empty_closure_is_legal_and_still_syncs_the_root() {
        let tmp = TempDir::new().unwrap();
        let tracker = StageTracker::default();
        tracker.begin_stage(1);
        let set = tracker.finish_writes().unwrap();
        assert!(set.is_empty());
        let stage = tracker
            .sync_written_set(&set, tmp.path(), &PosixDurability)
            .unwrap();
        assert_eq!(stage.files_synced, 0);
    }

    #[test]
    fn posix_adapter_names_itself() {
        assert_eq!(PosixDurability.adapter_name(), "posix-sync-all");
    }

    /// POSIX live roots yield a self-naming barrier.
    #[cfg(not(windows))]
    #[test]
    fn a_supported_root_yields_a_self_naming_barrier() {
        let tmp = TempDir::new().unwrap();
        // `tempdir()` returns `/var/folders/...` on macOS, and `/var` is a
        // symlink — `root_is_own` would rightly reject the alias. Canonicalise
        // first so the test exercises the root, not the OS tempdir layout.
        let root = tmp
            .path()
            .canonicalize()
            .unwrap()
            .join(".forgeplan")
            .join("lance");
        std::fs::create_dir_all(&root).unwrap();

        let (support, barrier, diagnostic) = platform_barrier(&root);
        assert_eq!(support, FilesystemSupport::Supported);
        assert!(diagnostic.is_none());
        let barrier = barrier.expect("a supported root gets a barrier");
        assert!(!barrier.adapter_name().is_empty());
    }

    /// On this host the selected adapter must be the POSIX one; the assertion
    /// is written against the cfg rather than against a hard-coded name so it
    /// is a real check here and a real check on a Windows runner.
    #[cfg(not(windows))]
    #[test]
    fn posix_hosts_select_the_posix_barrier() {
        let tmp = TempDir::new().unwrap();
        let root = tmp
            .path()
            .canonicalize()
            .unwrap()
            .join(".forgeplan")
            .join("lance");
        std::fs::create_dir_all(&root).unwrap();
        let (_, barrier, _) = platform_barrier(&root);
        assert_eq!(
            barrier.expect("supported root").adapter_name(),
            "posix-sync-all"
        );
    }

    /// A root that does not exist is `Uninitialized`, not an error, and still
    /// yields a barrier so bootstrap can ask the question before creating it.
    #[cfg(not(windows))]
    #[test]
    fn a_missing_root_is_supported_and_yields_a_barrier() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("absent").join("lance");
        let (support, barrier, diagnostic) = platform_barrier(&root);
        assert_eq!(support, FilesystemSupport::Supported);
        assert!(diagnostic.is_none());
        assert!(barrier.is_some());
    }

    /// A refused root never gets a barrier. Handing one out alongside
    /// `Unsupported` would let a caller skip the check and write anyway.
    #[test]
    fn an_unusable_root_is_refused_and_gets_no_barrier() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("not-a-dir");
        std::fs::write(&file, b"x").unwrap();

        let (support, barrier, diagnostic) = platform_barrier(&file);
        assert_eq!(support, FilesystemSupport::Unsupported);
        assert!(barrier.is_none());
        let diagnostic = diagnostic.expect("a refusal is always explained");
        assert_eq!(diagnostic.code, DiagnosticCode::UnsupportedIndexFilesystem);
    }

    /// A root reached through a symlink is refused on this host, so the
    /// rejection path is exercised without any Windows involvement.
    #[cfg(unix)]
    #[test]
    fn a_symlinked_root_is_refused_and_gets_no_barrier() {
        let tmp = TempDir::new().unwrap();
        let real = tmp.path().join("real-lance");
        std::fs::create_dir_all(&real).unwrap();
        let link = tmp.path().join("linked-lance");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let (support, barrier, diagnostic) = platform_barrier(&link);
        assert_eq!(support, FilesystemSupport::Unsupported);
        assert!(barrier.is_none());
        assert_eq!(
            diagnostic.expect("an alias is always explained").code,
            DiagnosticCode::UnsupportedIndexFilesystem
        );
    }

    /// The two adapters must never report the same name, or a diagnostic
    /// cannot say which barrier ran.
    #[test]
    fn adapter_names_distinguish_the_barriers() {
        assert_ne!(PosixDurability.adapter_name(), "windows-ntfs-write-through");
    }
}
