//! Windows / local NTFS write barrier.
//!
//! # Status: a real, documented platform gap
//!
//! This adapter has **never been compiled or executed**. It is `cfg(windows)`
//! on a macOS host and no Windows runner exists in this environment, so it
//! will be judged by static review only. Specifically, these remain
//! **unverified hypotheses**, not results:
//!
//! * whether the aggregate namespace/metadata ordering below is *complete* —
//!   that is, whether there is no interleaving of another writer that
//!   reorders what these calls rely on;
//! * Windows native fault behaviour — what these calls actually return
//!   across a power cut, a controller reset, or a filter-driver failure;
//! * hardware power-loss durability of the storage stack underneath.
//!
//! Choosing documented Win32 operations is not the same as having proved
//! them. Nothing in this file should be read as a claim that Windows support
//! is ready.
//!
//! # What was rejected, and why
//!
//! | Alternative | Rejected because |
//! |---|---|
//! | POSIX `fsync` on a directory | No such operation on Windows; borrowing the POSIX shape would be pretending |
//! | TxF (`MoveFileTransactedW`) | Deprecated API, and it buys transactional semantics this protocol does not want |
//! | Whole-volume flush, admin privileges | Needs elevation; the protocol's blast radius is its own index |
//! | `MOVEFILE_COPY_ALLOWED` | A copy-and-delete fallback silently degrades a same-volume rename into a cross-volume copy, which is not atomic and can leave a half-written destination |
//! | `MOVEFILE_DELAY_UNTIL_REBOOT` | Defers durability past the process; the contract is about this generation, now |
//! | `MOVEFILE_CREATE_HARDLINK` | A second name for the same object would break the "one final path" closure the tracker records |
//!
//! # How the guarantees are discharged on NTFS
//!
//! There is no directory fsync. The three NTFS primitives below carry the
//! whole guarantee, and each one's *reason* is stated at its call site:
//!
//! 1. **Parents are created write-through, ahead of the child write**
//!    ([`WindowsNtfsDurability::ensure_dir`]). The namespace entry for a
//!    directory therefore exists and is durable before anything is written
//!    into it, so a later child write never has to create a parent as an
//!    unregistered side effect.
//! 2. **File data *and metadata* are flushed**
//!    ([`WindowsNtfsDurability::sync_file`]) — including for empty files,
//!    where there is no data to write and only the directory entry and
//!    attributes to make durable.
//! 3. **The visible name is replaced write-through, on one volume**
//!    ([`WindowsNtfsDurability::replace_atomically`]). This is the
//!    *visibility point*; the durability ACK is a separate, later fact.
//!
//! [`WindowsNtfsDurability::sync_dir`] is therefore an explicit, documented
//! no-op. See its own doc comment for why that is the honest choice rather
//! than a false ACK.

use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use windows_sys::Win32::Foundation::{
    CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, CREATE_NEW, CreateDirectoryW, CreateFileW,
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAG_WRITE_THROUGH, FILE_ID_INFO, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, FileIdInfo, FlushFileBuffers, GetDriveTypeW,
    GetFileAttributesW, GetFileInformationByHandle, GetFileInformationByHandleEx,
    GetFinalPathNameByHandleW, GetVolumeInformationByHandleW, INVALID_FILE_ATTRIBUTES, VOLUME_NAME_GUID,
    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW, OPEN_EXISTING, WriteFile,
};
use windows_sys::Win32::System::WindowsProgramming::DRIVE_FIXED;

use super::PlatformDurability;
use crate::constellation::ConstellationError;

/// Filesystem name this adapter is willing to claim a barrier on.
const REQUIRED_FILESYSTEM: &str = "NTFS";

/// Bytes per `WriteFile` call.
///
/// `WriteFile` takes a `u32` length, and it is permitted to write **fewer**
/// bytes than asked for, so the loop below is required rather than defensive.
/// 1 MiB keeps each call comfortably inside that limit and inside a single
/// cache line's worth of bookkeeping on the storage stack.
const WRITE_CHUNK: usize = 1024 * 1024;

/// Monotonic counter making each temporary sibling name unique within a
/// process. Combined with the pid, a name cannot collide with a leftover temp
/// from a previous run of the same pid.
static TEMP_NONCE: AtomicU64 = AtomicU64::new(0);

/// NTFS write barrier.
///
/// Public Win32 only. No administrator rights, no volume flush, no
/// transactions.
#[derive(Debug, Default)]
pub(crate) struct WindowsNtfsDurability;

impl PlatformDurability for WindowsNtfsDurability {
    fn adapter_name(&self) -> &'static str {
        "windows-ntfs-write-through"
    }

    /// Flush one existing final file's data and metadata.
    ///
    /// Opened `OPEN_EXISTING` with `GENERIC_WRITE`,
    /// `FILE_FLAG_WRITE_THROUGH | FILE_FLAG_OPEN_REPARSE_POINT` and full
    /// sharing; the handle is verified to be a non-reparse regular file;
    /// `FlushFileBuffers`; handle closed.
    ///
    /// Empty files are included deliberately. There is no data to flush, but
    /// NTFS requires `FlushFileBuffers` for **file metadata**, and a
    /// write-through open also flushes associated metadata changes — which is
    /// what makes a zero-byte file's existence and attributes durable. Skipping
    /// them would ACK a file whose metadata is not on disk.
    fn sync_file(&self, path: &Path) -> Result<(), ConstellationError> {
        let name = wide(path);

        // GENERIC_WRITE: `FlushFileBuffers` is only meaningful on a handle
        // opened for writing; on a read-only handle it has nothing to push.
        //
        // Full sharing: a reader may hold this file, another writer may be
        // writing it, and a delete-pending handle must not block us. Omitting
        // FILE_SHARE_DELETE would make the open fail whenever a reader
        // legitimately holds the file without delete sharing.
        //
        // OPEN_EXISTING: this is a barrier over a file that already exists.
        // CREATE_ALWAYS or OPEN_ALWAYS here would create or truncate it.
        //
        // FILE_FLAG_WRITE_THROUGH: writes through this handle bypass the
        // cache and go to the storage stack, and associated metadata is
        // flushed with them. This is also the flag that covers empty files.
        //
        // FILE_FLAG_OPEN_REPARSE_POINT: without it `CreateFileW` follows a
        // symlink or junction, and the handle — and therefore the flush —
        // would land on a file outside the index while we recorded the
        // in-index path. With it, the handle refers to the reparse point
        // itself, which is exactly what the verification below then rejects.
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_WRITE_THROUGH | FILE_FLAG_OPEN_REPARSE_POINT,
                std::ptr::null_mut(),
            )
        };

        let handle = OwnedHandle::open(handle, path, "CreateFileW(OPEN_EXISTING)")?;

        // Verify the handle is the thing we meant to flush. Because the open
        // carried FILE_FLAG_OPEN_REPARSE_POINT, this reads the reparse point's
        // own attributes rather than its target's.
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe { GetFileInformationByHandle(handle.0, &mut info) } == 0 {
            return Err(last_error("GetFileInformationByHandle", path));
        }

        let attributes = info.dwFileAttributes;
        if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(ConstellationError::DurabilityUnavailable {
                detail: format!(
                    "{} is a reparse point; the index will not flush through an alias",
                    path.display()
                ),
            });
        }
        if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            return Err(ConstellationError::DurabilityUnavailable {
                detail: format!(
                    "{} is a directory, not a final regular file",
                    path.display()
                ),
            });
        }

        if unsafe { FlushFileBuffers(handle.0) } == 0 {
            return Err(last_error("FlushFileBuffers", path));
        }
        Ok(())
    }

    /// **Deliberate no-op. This is not an ACK.**
    ///
    /// Windows has no operation corresponding to `fsync` on a directory, so
    /// there is nothing to call. Inventing one — opening a directory with
    /// backup semantics and flushing it — would produce a call that returns
    /// success without any documented durability meaning, which is precisely
    /// the false ACK this protocol forbids.
    ///
    /// The guarantee `sync_written_set` would otherwise obtain here is
    /// discharged elsewhere on this platform, and only there:
    ///
    /// * [`WindowsNtfsDurability::ensure_dir`] makes every directory entry
    ///   durable, write-through, **before** a child write exists;
    /// * [`WindowsNtfsDurability::sync_file`] flushes data *and* file
    ///   metadata for each touched final file;
    /// * [`WindowsNtfsDurability::replace_atomically`] makes the visible name
    ///   durable on the same volume with a write-through move.
    ///
    /// Returning `Ok(())` here is therefore truthful **only** because those
    /// three carry the guarantee. An implementation that cannot make the same
    /// three claims must return
    /// [`ConstellationError::UnsupportedIndexFilesystem`] instead of `Ok(())`
    /// — the trait documents that requirement at the method.
    fn sync_dir(&self, _path: &Path) -> Result<(), ConstellationError> {
        Ok(())
    }

    /// Create `dir` and every missing parent, each through a unique
    /// temporary sibling installed with a write-through move.
    ///
    /// Parent before child, top-down: a child's temporary sibling must live in
    /// an existing parent, so the order is forced by the filesystem rather
    /// than chosen.
    fn ensure_dir(&self, dir: &Path) -> Result<(), ConstellationError> {
        let mut current = PathBuf::new();
        for component in dir.components() {
            current.push(component);
            if matches!(component, std::path::Component::Prefix(_)) {
                continue;
            }
            match existing_kind(&current).map_err(|error| super::io_error("inspect directory", &current, error))? {
                Some(EntryKind::Directory) => continue,
                Some(EntryKind::ReparsePoint) => {
                    return Err(ConstellationError::DurabilityUnavailable {
                        detail: format!(
                            "index path component {} is a reparse point; the index namespace must \
                             not be substituted",
                            current.display()
                        ),
                    });
                }
                Some(EntryKind::File) => {
                    return Err(ConstellationError::Io {
                        detail: format!(
                            "{} exists as a file where a directory is required",
                            current.display()
                        ),
                    });
                }
                None => create_directory_write_through(&current)?,
            }
        }
        Ok(())
    }

    /// Install `src` at `dest`, replacing what is there, durably.
    ///
    /// This is the **visibility point**. `MOVEFILE_WRITE_THROUGH` makes the
    /// move complete on disk before returning; the caller's later ACK depends
    /// additionally on verifying the descriptor's checksum and the pinned table
    /// versions, which this layer cannot see.
    fn replace_atomically(&self, src: &Path, dest: &Path) -> Result<(), ConstellationError> {
        self.move_write_through(src, dest, true)
    }
}

impl WindowsNtfsDurability {
    /// `MoveFileExW` with or without `MOVEFILE_REPLACE_EXISTING`.
    ///
    /// `replace == false` is the immutable-generation install: an immutable
    /// generation must never overwrite an existing one, and leaving the flag
    /// off makes the collision a hard error rather than a silent clobber.
    ///
    /// `MOVEFILE_COPY_ALLOWED` and `MOVEFILE_DELAY_UNTIL_REBOOT` are never
    /// set. The first would turn a same-volume rename into a copy-and-delete
    /// on failure, which is neither atomic nor durable in the same way; the
    /// second defers durability past this process, which is not a guarantee
    /// about *this* generation.
    pub(crate) fn move_write_through(
        &self,
        src: &Path,
        dest: &Path,
        replace: bool,
    ) -> Result<(), ConstellationError> {
        let mut flags = MOVEFILE_WRITE_THROUGH;
        if replace {
            flags |= MOVEFILE_REPLACE_EXISTING;
        }
        let src_wide = wide(src);
        let dest_wide = wide(dest);

        if unsafe { MoveFileExW(src_wide.as_ptr(), dest_wide.as_ptr(), flags) } == 0 {
            return Err(last_error(
                if replace {
                    "MoveFileExW(REPLACE_EXISTING|WRITE_THROUGH)"
                } else {
                    "MoveFileExW(WRITE_THROUGH)"
                },
                dest,
            ));
        }
        Ok(())
    }

    /// Create `path` new, write every byte, flush, close.
    ///
    /// `CREATE_NEW` fails if the path already exists. For an immutable
    /// generation that failure is the point: it is how an attempt to publish
    /// the same `generationId` twice becomes an error rather than an
    /// overwrite.
    ///
    /// The write loop is required: `WriteFile` may write fewer bytes than
    /// requested, and treating a short write as success would publish a
    /// truncated manifest.
    pub(crate) fn write_file_create_new(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> Result<(), ConstellationError> {
        let name = wide(path);
        // Same flag rationale as `sync_file`, minus OPEN_REPARSE_POINT: the
        // path must not exist, so there is no reparse point to open through,
        // and CREATE_NEW would fail on one anyway.
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_WRITE,
                FILE_SHARE_READ,
                std::ptr::null(),
                CREATE_NEW,
                FILE_FLAG_WRITE_THROUGH,
                std::ptr::null_mut(),
            )
        };
        let handle = OwnedHandle::open(handle, path, "CreateFileW(CREATE_NEW)")?;

        let mut written_total: usize = 0usize;
        while written_total < bytes.len() {
            let end = written_total.saturating_add(WRITE_CHUNK).min(bytes.len());
            let chunk = &bytes[written_total..end];
            let mut written: u32 = 0;
            if unsafe {
                WriteFile(
                    handle.0,
                    chunk.as_ptr(),
                    chunk.len() as u32,
                    &mut written,
                    std::ptr::null_mut(),
                )
            } == 0
            {
                return Err(last_error("WriteFile", path));
            }
            if written == 0 {
                // A zero-length progress on a non-empty buffer means the file
                // can make no further progress. Continuing would spin.
                return Err(ConstellationError::StorageIo {
                    detail: format!(
                        "WriteFile made no progress at byte {written_total} of {} for {}",
                        bytes.len(),
                        path.display()
                    ),
                });
            }
            written_total += written as usize;
        }

        if unsafe { FlushFileBuffers(handle.0) } == 0 {
            return Err(last_error("FlushFileBuffers", path));
        }
        Ok(())
    }

    /// Write an immutable file into `dir` under `name` and install it at that
    /// name, refusing to replace anything.
    ///
    /// P2d's generation-manifest writer. Exists here because the Windows
    /// sequencing — create-new write-through, flush, close, write-through
    /// move without replace — is a property of this platform and must not be
    /// re-derived at the call site.
    pub(crate) fn install_new_file(
        &self,
        dir: &Path,
        name: &str,
        bytes: &[u8],
    ) -> Result<(), ConstellationError> {
        self.ensure_dir(dir)?;
        let temp = temp_sibling(dir, name, "new");
        self.write_file_create_new(&temp, bytes)?;
        // The generation-manifest bytes are written and flushed; the
        // write-through move that will install them has not happened, so
        // `CURRENT` still names the previous generation and this file is an
        // orphan. On Windows the manifest's own durability barrier is
        // `FlushFileBuffers` inside `write_file_create_new`, and `before_
        // manifest_sync` fires immediately after it returns — the equivalent
        // internal point, not an outer call standing in for it.
        crate::constellation::fault_barrier_if_selected("before_manifest_sync");
        let final_path = dir.join(name);
        match self.move_write_through(&temp, &final_path, false) {
            Ok(()) => Ok(()),
            Err(e) => {
                // The temporary is a half-finished object in the index root.
                // Removing it is a best-effort cleanup of our own litter, not
                // part of any acknowledged write.
                let _ = std::fs::remove_file(&temp);
                Err(e)
            }
        }
    }
}

/// Open a directory itself, rather than following a final reparse point.
/// Full sharing permits the caller's later same-volume namespace rename.
pub(crate) fn open_directory_no_reparse(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use std::os::windows::io::AsRawHandle;

    root_is_own(path).map_err(std::io::Error::other)?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(std::io::Error::other("not a non-reparse directory"));
    }
    verify_local_ntfs_handle(file.as_raw_handle()).map_err(std::io::Error::other)?;
    Ok(file)
}

/// Read at most 16 KiB plus one byte, so the caller can reject oversize state.
/// The final reparse point is never followed; the caller excludes noncooperating
/// namespace changes.
pub(crate) fn read_mutable_file(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};

    let parent = path.parent().ok_or_else(|| std::io::Error::other("file has no parent"))?;
    verify_local_ntfs(parent).map_err(std::io::Error::other)?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(std::io::Error::other("mutable file is not a non-reparse regular file"));
    }
    let mut bytes = Vec::new();
    file.take(16 * 1024 + 1).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Replace a mutable file using a flushed same-directory temporary and a
/// write-through move. The caller excludes noncooperating namespace mutations.
pub(crate) fn replace_mutable_file(path: &Path, bytes: &[u8]) -> Result<(), ConstellationError> {
    use std::os::windows::fs::MetadataExt;

    let parent = path.parent().ok_or_else(|| ConstellationError::DurabilityUnavailable {
        detail: format!("{} has no parent", path.display()),
    })?;
    verify_local_ntfs(parent).map_err(|detail| ConstellationError::DurabilityUnavailable { detail })?;
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0 => {}
        Ok(_) => return Err(ConstellationError::DurabilityUnavailable {
            detail: format!("{} is not a non-reparse regular file", path.display()),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(super::io_error("inspect mutable file", path, error)),
    }
    let temporary = temp_sibling(parent, "mutable", "replace");
    WindowsNtfsDurability.write_file_create_new(&temporary, bytes)?;
    WindowsNtfsDurability.move_write_through(&temporary, path, true)
}

/// Seal only an exclusive, unpublished staging tree with a durable parent.
/// The caller must hold its workspace lock and close all tasks and handles.
/// On any error, abandon the stage and retain any temporary siblings.
pub(crate) fn seal_unpublished_tree(root: &Path) -> Result<(), ConstellationError> {
    root_is_own(root).and_then(|()| verify_local_ntfs(root)).map_err(|detail| {
        ConstellationError::DurabilityUnavailable { detail }
    })?;
    seal_private_entry(root)
}

fn seal_private_entry(path: &Path) -> Result<(), ConstellationError> {
    use std::os::windows::fs::MetadataExt;

    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| super::io_error("inspect private stage", path, error))?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(ConstellationError::DurabilityUnavailable {
            detail: format!("private stage entry {} is a reparse point", path.display()),
        });
    }
    if metadata.is_dir() {
        let entries = std::fs::read_dir(path)
            .map_err(|error| super::io_error("enumerate private stage", path, error))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| super::io_error("enumerate private stage", path, error))?;
        for entry in entries {
            seal_private_entry(&entry.path())?;
        }
    } else if metadata.is_file() {
        WindowsNtfsDurability.sync_file(path)?;
    } else {
        return Err(ConstellationError::DurabilityUnavailable {
            detail: format!("private stage entry {} is not a regular file or directory", path.display()),
        });
    }
    let parent = path.parent().ok_or_else(|| ConstellationError::DurabilityUnavailable {
        detail: format!("private stage entry {} has no parent", path.display()),
    })?;
    let temporary = temp_sibling(parent, "stage", "seal");
    WindowsNtfsDurability.move_write_through(path, &temporary, false)?;
    WindowsNtfsDurability.move_write_through(&temporary, path, false)
}


/// What exists at a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    Directory,
    File,
    ReparsePoint,
}

/// Classify an existing path without following a reparse point.
///
/// `GetFileAttributesW` reports the attributes of the entry itself, which is
/// what the reparse check needs; a symlink reports `REPARSE_POINT` rather
/// than its target's attributes.
fn existing_kind(path: &Path) -> Result<Option<EntryKind>, std::io::Error> {
    let name = wide(path);
    let attributes = unsafe { GetFileAttributesW(name.as_ptr()) };
    if attributes == INVALID_FILE_ATTRIBUTES {
        let error = std::io::Error::last_os_error();
        return if error.kind() == std::io::ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(error)
        };
    }
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Ok(Some(EntryKind::ReparsePoint));
    }
    if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        return Ok(Some(EntryKind::Directory));
    }
    Ok(Some(EntryKind::File))
}

/// Create one directory through a unique temporary sibling, then install it.
///
/// `CreateDirectoryW` makes the directory live under its final name
/// immediately. Going via a temporary sibling instead puts the namespace
/// mutation into one explicit `MoveFileExW(MOVEFILE_WRITE_THROUGH)` whose
/// durability the caller can order against the file writes that follow.
///
/// `MOVEFILE_REPLACE_EXISTING` is deliberately **not** set: replacing a
/// directory that already exists would swap an entire populated subtree, and
/// a lost race here must be an error rather than a silent replacement.
fn create_directory_write_through(dir: &Path) -> Result<(), ConstellationError> {
    let parent = dir.parent().ok_or_else(|| ConstellationError::Io {
        detail: format!("{} has no parent directory", dir.display()),
    })?;
    let leaf = dir.file_name().ok_or_else(|| ConstellationError::Io {
        detail: format!("{} has no final component", dir.display()),
    })?;
    let temp = temp_sibling(parent, &leaf.to_string_lossy(), "dir");

    let temp_wide = wide(&temp);
    if unsafe { CreateDirectoryW(temp_wide.as_ptr(), std::ptr::null()) } == 0 {
        return Err(last_error("CreateDirectoryW", &temp));
    }

    let adapter = WindowsNtfsDurability;
    if let Err(e) = adapter.move_write_through(&temp, dir, false) {
        // Best-effort removal of our own empty temporary directory.
        let _ = std::fs::remove_dir(&temp);
        return Err(e);
    }
    Ok(())
}

/// Read the physical identity of an already-open handle.
///
/// Four arguments, exactly as `GetFileInformationByHandleEx` is bound in
/// `windows-sys 0.61.2`: the handle, the information class, the output buffer
/// and its size. `FileIdInfo` is the only `FILE_INFO_BY_HANDLE_CLASS` that
/// returns a full 128-bit id — `BY_HANDLE_FILE_INFORMATION` gives a 64-bit
/// index and a 32-bit volume serial, which collide on ReFS and on volumes with
/// recycled serials — so the buffer is passed as a raw pointer with its own
/// `size_of`. A class that disagrees with the buffer size fails with
/// `ERROR_INVALID_PARAMETER` rather than truncating.
pub(crate) fn identity_from_handle(
    handle: HANDLE,
    what: &str,
) -> Result<super::PhysicalIdentity, String> {
    let mut attributes: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(handle, &mut attributes) } == 0 {
        return Err(format!(
            "GetFileInformationByHandle failed for {what}: {}",
            std::io::Error::last_os_error()
        ));
    }
    if attributes.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(format!("{what} is a reparse point"));
    }
    let mut info: FILE_ID_INFO = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast::<std::ffi::c_void>(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    };
    if ok == 0 {
        let error = std::io::Error::last_os_error();
        return Err(format!(
            "GetFileInformationByHandleEx(FileIdInfo) failed for {what}: {error}"
        ));
    }
    Ok(super::PhysicalIdentity {
        volume: info.VolumeSerialNumber,
        id: info.FileId.Identifier,
    })
}

/// The identity of whatever `path` currently names.
///
/// Opened with full sharing including `FILE_SHARE_DELETE`, and with
/// `FILE_FLAG_OPEN_REPARSE_POINT`. Full sharing is required because this is a
/// *re-check*, not a guard: it has to succeed while a writer holds the file, or
/// the re-check would report "cannot verify" for a perfectly good pin. The
/// reparse flag keeps the handle on the link itself, which is then rejected.
/// Backup semantics also permit opening directories.
pub(crate) fn identity_of_path(path: &Path) -> Result<super::PhysicalIdentity, String> {
    let name = wide(path);
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };
    let handle = OwnedHandle::open(handle, path, "CreateFileW(OPEN_EXISTING, read-only)")
        .map_err(|error| error.to_string())?;
    identity_from_handle(handle.0, &path.display().to_string())
}

/// Open an immutable generation manifest for a **pinned read**.
///
/// # The one difference from every other open in this file
///
/// `FILE_SHARE_DELETE` is deliberately **absent**. On Windows a delete is only
/// permitted while every open handle on the file shares delete access, so a
/// reader that omits it makes `DeleteFile` / `MoveFileExW(REPLACE_EXISTING)`
/// fail with `ERROR_SHARING_VIOLATION` for as long as the pin lives. That is
/// what closes the unlink-while-open gap the POSIX `flock` closes for free:
/// a GC pass that tries to prune the generation a reader is serving is refused
/// by the kernel, not merely noticed afterwards by an identity re-check.
///
/// The cost is that the file cannot be renamed or deleted while a read holds it,
/// which is precisely the intended contract — a generation manifest is
/// immutable, so nothing legitimate needs to replace it.
///
/// `FILE_FLAG_OPEN_REPARSE_POINT` keeps the handle on the reparse point itself,
/// so a symlink substituted at this path is refused by the caller's
/// reparse/identity check instead of being read through.
pub(crate) fn open_manifest_for_pinning(path: &Path) -> Result<std::fs::File, String> {
    use std::os::windows::io::FromRawHandle;

    let name = wide(path);
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ,
            // No FILE_SHARE_DELETE — see the doc comment above.
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        let error = std::io::Error::last_os_error();
        return Err(format!(
            "cannot open {} for a pinned read: {error}",
            path.display()
        ));
    }
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(handle, &mut info) } == 0 {
        let error = std::io::Error::last_os_error();
        unsafe { CloseHandle(handle) };
        return Err(format!(
            "GetFileInformationByHandle failed for {}: {error}",
            path.display()
        ));
    }
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0
    {
        unsafe { CloseHandle(handle) };
        return Err(format!(
            "{} is not a plain file; a pinned manifest must not be read through an alias",
            path.display()
        ));
    }
    // The handle becomes the `File` that carries the `LockFileEx` region for
    // the whole pin. `into_raw_handle`/`from_raw_handle` is the ownership
    // transfer; there is no point at which both sides own it.
    Ok(unsafe { std::fs::File::from_raw_handle(handle.into()) })
}

/// A unique sibling path next to `dir`, never colliding with a real object.
fn temp_sibling(dir: &Path, name: &str, tag: &str) -> PathBuf {
    let nonce = TEMP_NONCE.fetch_add(1, Ordering::Relaxed);
    dir.join(format!(
        ".constellation-{tag}-{}-{}-{name}",
        std::process::id(),
        nonce
    ))
}

/// Refuse anything that is not a local fixed NTFS volume.
///
/// ReFS, FAT, exFAT, a network share (`DRIVE_REMOTE`) and any volume whose
/// name is not `NTFS` all return an error. This is the "never a false ACK"
/// rule: an index on a filesystem we cannot reason about is reported as
/// unsupported rather than written to and called durable.
///
/// # Errors
///
/// A message describing the rejected volume.
pub(crate) fn verify_local_ntfs(index_root: &Path) -> Result<(), String> {
    let mut existing = index_root;
    loop {
        match std::fs::symlink_metadata(existing) {
            Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => break,
            Ok(_) => return Err(format!("{} is not a plain directory", existing.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                existing = existing.parent().ok_or_else(|| format!("{} has no existing directory ancestor", index_root.display()))?;
            }
            Err(error) => return Err(format!("cannot inspect {}: {error}", existing.display())),
        }
    }
    root_is_own(existing)?;
    let name = wide(existing);
    let raw = unsafe {
        CreateFileW(name.as_ptr(), GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(), OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, std::ptr::null_mut())
    };
    let handle = OwnedHandle::open(raw, existing, "open volume verification directory")
        .map_err(|error| error.to_string())?;
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(handle.0, &mut info) } == 0 {
        return Err(format!("cannot inspect directory handle {}: {}", existing.display(), std::io::Error::last_os_error()));
    }
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
    {
        return Err(format!("{} is not a non-reparse directory", existing.display()));
    }
    verify_local_ntfs_handle(handle.0)
}

pub(crate) fn verify_local_ntfs_handle(handle: HANDLE) -> Result<(), String> {
    let mut filesystem = [0u16; 64];
    let ok = unsafe {
        GetVolumeInformationByHandleW(handle, std::ptr::null_mut(), 0,
            std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(),
            filesystem.as_mut_ptr(), filesystem.len() as u32)
    };
    if ok == 0 {
        return Err(format!("cannot determine filesystem from directory handle: {}", std::io::Error::last_os_error()));
    }
    let end = filesystem.iter().position(|unit| *unit == 0)
        .ok_or_else(|| "filesystem name is unterminated".to_string())?;
    let filesystem = String::from_utf16(&filesystem[..end]).map_err(|error| error.to_string())?;
    if !filesystem.eq_ignore_ascii_case(REQUIRED_FILESYSTEM) {
        return Err(format!("filesystem `{filesystem}` is not `{REQUIRED_FILESYSTEM}`"));
    }
    let mut final_path = vec![0u16; 32768];
    let len = unsafe { GetFinalPathNameByHandleW(handle, final_path.as_mut_ptr(), final_path.len() as u32, VOLUME_NAME_GUID) };
    if len == 0 || len as usize >= final_path.len() {
        return Err(format!("cannot obtain handle-bound volume GUID path: {}", std::io::Error::last_os_error()));
    }
    let path = String::from_utf16(&final_path[..len as usize]).map_err(|error| error.to_string())?;
    let end = path.find("}\\").ok_or_else(|| "handle does not resolve to a local volume GUID".to_string())?;
    let root = &path[..end + 2];
    if !root.starts_with(r"\\?\Volume{") {
        return Err("handle does not resolve to a local volume GUID".to_string());
    }
    let volume_wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
    let drive_type = unsafe { GetDriveTypeW(volume_wide.as_ptr()) };
    if drive_type != DRIVE_FIXED {
        return Err(format!("handle volume is not local fixed storage (drive type {drive_type})"));
    }
    Ok(())
}


/// NUL-terminated UTF-16 form of a path, as every `*W` Win32 call takes.
fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// An owned Win32 handle that closes itself.
///
/// Closing on drop matters: a leaked handle keeps the file open, which blocks
/// a later `MoveFileExW` on it, and the failure would then look like a
/// namespace problem rather than a leak.
struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn open(handle: HANDLE, path: &Path, what: &str) -> Result<Self, ConstellationError> {
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return Err(last_error(what, path));
        }
        Ok(Self(handle))
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// Turn the current `GetLastError` into a typed storage error.
///
/// The `what` and the path go in the message because Win32 error codes are
/// opaque numbers without them.
fn last_error(what: &str, path: &Path) -> ConstellationError {
    let code = std::io::Error::last_os_error();
    ConstellationError::Io {
        detail: format!("{what} failed for {}: {code}", path.display()),
    }
}

/// Verify every existing component of `root` is a real directory and not a
/// symlink or junction.
///
/// P1's root-ownership check, kept and extended: this now also rejects
/// junctions, which `std`'s `is_symlink` does not report. A junction inside
/// the index namespace is an identity substitution for the same reason a
/// symlink is.
pub(crate) fn root_is_own(root: &Path) -> Result<(), String> {
    let mut current = PathBuf::new();
    for component in root.components() {
        current.push(component);
        if matches!(component, std::path::Component::Prefix(_)) {
            continue;
        }
        // A missing tail is bootstrap's job, not an alias problem.
        let Some(kind) = existing_kind(&current)
            .map_err(|error| format!("cannot inspect {}: {error}", current.display()))? else {
            return Ok(());
        };
        match kind {
            EntryKind::ReparsePoint => {
                return Err(format!(
                    "index root component {} is a reparse point; a reparse alias would create a \
                     second physical owner",
                    current.display()
                ));
            }
            EntryKind::File => {
                return Err(format!(
                    "index root component {} is not a directory",
                    current.display()
                ));
            }
            EntryKind::Directory => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod windows_only_tests {
    use super::*;
    use crate::constellation::capture::FilesystemSupport;

    fn temp_index_root() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join(".forgeplan").join("lance");
        std::fs::create_dir_all(&root).unwrap();
        (tmp, root)
    }

    fn adapter_barrier(root: &Path) -> Arc<dyn PlatformDurability> {
        Arc::new(WindowsNtfsDurability)
    }

    #[test]
    fn the_adapter_names_itself_distinctly() {
        assert_eq!(
            WindowsNtfsDurability.adapter_name(),
            "windows-ntfs-write-through"
        );
        assert_ne!(
            WindowsNtfsDurability.adapter_name(),
            super::super::PosixDurability.adapter_name()
        );
    }

    #[test]
    fn sync_dir_is_a_documented_no_op_and_not_a_directory_flush() {
        let (_tmp, root) = temp_index_root();
        // Succeeding here is the documented behaviour: the guarantee is
        // carried by ensure_dir / sync_file / replace_atomically, and there is
        // no Windows call that would mean anything more.
        assert!(WindowsNtfsDurability.sync_dir(&root).is_ok());
    }

    #[test]
    fn ensure_dir_creates_missing_parents_and_is_idempotent() {
        let (_tmp, root) = temp_index_root();
        let nested = root.join("generations").join("deeper");
        WindowsNtfsDurability.ensure_dir(&nested).unwrap();
        assert!(nested.is_dir());
        // Second call finds every component present and must not fail.
        WindowsNtfsDurability.ensure_dir(&nested).unwrap();
    }

    #[test]
    fn seal_private_tree_preserves_nested_files() {
        let (_tmp, root) = temp_index_root();
        let nested = root.join("nested");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(nested.join("data"), b"payload").unwrap();
        seal_unpublished_tree(&root).unwrap();
        assert_eq!(std::fs::read(nested.join("data")).unwrap(), b"payload");
    }

    #[test]
    fn seal_private_tree_rejects_reparse_points() {
        let (_tmp, root) = temp_index_root();
        let file = root.join("data");
        std::fs::write(&file, b"payload").unwrap();
        std::os::windows::fs::symlink_file(&file, root.join("link")).unwrap();
        assert!(seal_unpublished_tree(&root).is_err());
    }

    #[test]
    fn ensure_dir_refuses_a_file_where_a_directory_belongs() {
        let (_tmp, root) = temp_index_root();
        let blocker = root.join("blocker");
        std::fs::write(&blocker, b"x").unwrap();
        let err = WindowsNtfsDurability
            .ensure_dir(&blocker.join("child"))
            .unwrap_err();
        assert_eq!(err.code(), crate::constellation::DiagnosticCode::StorageIo);
    }

    #[test]
    fn install_new_file_writes_flushes_and_refuses_to_replace() {
        let (_tmp, root) = temp_index_root();
        let target = root.join("gen-1.json");
        WindowsNtfsDurability
            .install_new_file(&root, "gen-1.json", b"{\"a\":1}")
            .unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"{\"a\":1}");

        // An immutable generation must never overwrite an existing one.
        let err = WindowsNtfsDurability
            .install_new_file(&root, "gen-1.json", b"{\"a\":2}")
            .unwrap_err();
        assert_eq!(err.code(), crate::constellation::DiagnosticCode::StorageIo);
        assert_eq!(
            std::fs::read(&target).unwrap(),
            b"{\"a\":1}",
            "the first generation must be untouched"
        );
    }

    #[test]
    fn install_new_file_writes_an_empty_file_durably() {
        let (_tmp, root) = temp_index_root();
        WindowsNtfsDurability
            .install_new_file(&root, "empty.json", b"")
            .unwrap();
        assert_eq!(
            std::fs::read(root.join("empty.json")).unwrap(),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn sync_file_accepts_a_real_file() {
        let (_tmp, root) = temp_index_root();
        let file = root.join("a.lance");
        std::fs::write(&file, b"payload").unwrap();
        let barrier = adapter_barrier(&root);
        barrier.sync_file(&file).unwrap();
    }

    #[test]
    fn sync_file_accepts_an_empty_file() {
        // Empty files have no data to flush, only metadata; they are flushed
        // anyway, because NTFS requires FlushFileBuffers for file metadata.
        let (_tmp, root) = temp_index_root();
        let file = root.join("empty.lance");
        std::fs::write(&file, b"").unwrap();
        WindowsNtfsDurability.sync_file(&file).unwrap();
    }

    #[test]
    fn sync_file_refuses_a_directory() {
        let (_tmp, root) = temp_index_root();
        let err = WindowsNtfsDurability.sync_file(&root).unwrap_err();
        assert_eq!(
            err.code(),
            crate::constellation::DiagnosticCode::DurabilityUnavailable
        );
    }

    #[test]
    fn sync_file_refuses_a_reparse_point() {
        let (_tmp, root) = temp_index_root();
        let real = root.join("real.lance");
        std::fs::write(&real, b"payload").unwrap();
        let link = root.join("link.lance");
        std::os::windows::fs::symlink_file(&real, &link).unwrap();
        let err = WindowsNtfsDurability.sync_file(&link).unwrap_err();
        assert_eq!(
            err.code(),
            crate::constellation::DiagnosticCode::DurabilityUnavailable,
            "flushing through a symlink would ACK a file outside the index"
        );
    }

    #[test]
    fn a_non_ntfs_or_remote_volume_is_refused_rather_than_claimed() {
        let (_tmp, root) = temp_index_root();
        // On a real NTFS machine this passes; on ReFS, FAT or a network mount
        // it must fail, and either way the contract under test is "never a
        // false ACK": no path returns Ok for a volume it cannot identify.
        if let Err(detail) = verify_local_ntfs(&root) {
            assert!(!detail.is_empty());
        }
    }

    #[test]
    fn volume_classification_uses_filesystem_name_not_volume_label() {
        let (_tmp, root) = temp_index_root();
        let name = wide(&root);
        let handle = OwnedHandle::open(
            unsafe {
                CreateFileW(name.as_ptr(), GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null(), OPEN_EXISTING,
                    FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, std::ptr::null_mut())
            },
            &root,
            "open test directory",
        ).unwrap();
        let mut label = [0u16; 256];
        let mut filesystem = [0u16; 64];
        assert_ne!(unsafe {
            GetVolumeInformationByHandleW(handle.0, label.as_mut_ptr(), label.len() as u32,
                std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(),
                filesystem.as_mut_ptr(), filesystem.len() as u32)
        }, 0);
        let end = filesystem.iter().position(|unit| *unit == 0).unwrap();
        let filesystem = String::from_utf16(&filesystem[..end]).unwrap();
        if filesystem.eq_ignore_ascii_case("NTFS") {
            assert!(verify_local_ntfs_handle(handle.0).is_ok(),
                "a non-NTFS volume label must not make an NTFS filesystem fail");
        } else {
            assert!(verify_local_ntfs_handle(handle.0).is_err(),
                "an NTFS volume label must not make a different filesystem pass");
        }
    }

    #[test]
    fn primitive_barrier_on_windows_selects_the_ntfs_adapter() {
        let (_tmp, root) = temp_index_root();
        let (support, barrier, diagnostic) = super::super::windows_ntfs_primitive_barrier(&root);
        assert_eq!(support, FilesystemSupport::Supported);
        assert!(diagnostic.is_none());
        let barrier = barrier.expect("a supported local NTFS root gets a barrier");
        assert_eq!(barrier.adapter_name(), "windows-ntfs-write-through");
    }

    #[test]
    fn root_is_own_rejects_a_junction_component() {
        let (_tmp, root) = temp_index_root();
        let real = root.join("real");
        std::fs::create_dir_all(&real).unwrap();
        let link = root.join("linked");
        // A directory junction needs the junction-specific API, so this uses
        // a directory symlink: both are REPARSE_POINTs and both must be
        // refused for the same reason.
        std::os::windows::fs::symlink_dir(&real, &link).unwrap();
        let err = root_is_own(&link.join("lance")).unwrap_err();
        assert!(err.contains("reparse point"), "{err}");
    }

    // ── physical identity ──────────────────────────────────────────────────
    //
    // These run only on a real Windows host with a real NTFS volume. They are
    // the evidence for the Windows identity claim; anywhere else the claim
    // stays NOT RUN rather than assumed from the code reading correctly.

    /// `GetFileInformationByHandleEx(FileIdInfo)` actually returns a usable
    /// identity for a real file — a non-zero volume serial and an id that is
    /// not the all-zero "unresolved" pattern.
    #[test]
    fn file_id_info_returns_a_real_identity() {
        let (_tmp, root) = temp_index_root();
        let file = root.join("id.bin");
        std::fs::write(&file, b"payload").unwrap();

        let id = identity_of_path(&file).unwrap();
        assert_ne!(
            id.volume,
            0,
            "a zero volume serial means the class or the buffer size was wrong"
        );
        assert_ne!(
            id.id,
            [0u8; 16],
            "an all-zero file id is the kernel saying 'no id', not id zero"
        );
    }

    /// The whole reason this exists: a same-length, same-mtime replacement is
    /// a **different** file. `BY_HANDLE_FILE_INFORMATION`'s size/time fields
    /// cannot see this; the 128-bit id can.
    #[test]
    fn a_same_length_replacement_gets_a_different_file_id() {
        let (_tmp, root) = temp_index_root();
        let file = root.join("swap.bin");
        let staging = root.join("swap.staging");
        std::fs::write(&file, b"AAAA").unwrap();
        let before = identity_of_path(&file).unwrap();

        std::fs::write(&staging, b"BBBB").unwrap();
        std::fs::rename(&staging, &file).unwrap();

        let after = identity_of_path(&file).unwrap();
        assert_eq!(before.volume, after.volume, "same volume");
        assert_ne!(
            before.id,
            after.id,
            "a replaced file must not keep its NTFS file id — this is the \
             substitution the size+mtime check could not see"
        );
    }

    /// The handle-based reader agrees with the path-based one, which is the
    /// property the pin's re-check rests on: the open descriptor and the name
    /// must report one identity.
    #[test]
    fn handle_and_path_report_one_identity() {
        let (_tmp, root) = temp_index_root();
        let file = root.join("agree.bin");
        std::fs::write(&file, b"payload").unwrap();
        let handle = std::fs::File::open(&file).unwrap();
        let from_handle = identity_from_handle(
            {
                use std::os::windows::io::AsRawHandle;
                handle.as_raw_handle()
            },
            "the test handle",
        )
        .unwrap();
        assert_eq!(from_handle, identity_of_path(&file).unwrap());
    }

    #[test]
    fn directory_identity_matches_an_open_directory_handle() {
        let (_tmp, root) = temp_index_root();
        let directory = open_directory_no_reparse(&root).unwrap();
        assert_eq!(
            super::super::physical_identity_of(&directory).unwrap(),
            identity_of_path(&root).unwrap()
        );
        assert_ne!(identity_of_path(&root).unwrap(), identity_of_path(root.parent().unwrap()).unwrap());
    }

    #[test]
    fn open_directory_handle_retains_identity_across_rename() {
        let (_tmp, root) = temp_index_root();
        let before = root.join("before");
        let after = root.join("after");
        std::fs::create_dir(&before).unwrap();
        let held = open_directory_no_reparse(&before).unwrap();
        let identity = super::super::physical_identity_of(&held).unwrap();
        WindowsNtfsDurability.move_write_through(&before, &after, false).unwrap();
        assert!(!before.exists());
        assert_eq!(super::super::physical_identity_of(&held).unwrap(), identity);
        assert_eq!(identity_of_path(&after).unwrap(), identity);
    }

    #[test]
    fn directory_opener_refuses_a_regular_file() {
        let (_tmp, root) = temp_index_root();
        let file = root.join("file");
        std::fs::write(&file, b"x").unwrap();
        assert!(open_directory_no_reparse(&file).is_err());
    }

    #[test]
    fn directory_opener_refuses_reparse_point() {
        let (_tmp, root) = temp_index_root();
        let directory = root.join("directory");
        let link = root.join("link");
        std::fs::create_dir(&directory).unwrap();
        match std::os::windows::fs::symlink_dir(&directory, &link) {
            Ok(()) => {}
            Err(error) if error.raw_os_error() == Some(1314) => {
                eprintln!("skipping symlink test: Windows symlink privilege is unavailable");
                return;
            }
            Err(error) => panic!("cannot create directory symlink: {error}"),
        }
        assert!(open_directory_no_reparse(&link).is_err());
    }

    #[test]
    fn identity_rejects_file_and_directory_reparse_points() {
        let (_tmp, root) = temp_index_root();
        let file = root.join("file");
        let file_link = root.join("file-link");
        let dir_link = root.join("dir-link");
        std::fs::write(&file, b"x").unwrap();
        std::os::windows::fs::symlink_file(&file, &file_link).unwrap();
        std::os::windows::fs::symlink_dir(&root, &dir_link).unwrap();
        assert!(identity_of_path(&file_link).unwrap_err().contains("reparse"));
        assert!(identity_of_path(&dir_link).unwrap_err().contains("reparse"));
    }

    #[test]
    fn no_replace_move_preserves_both_files_on_collision() {
        let (_tmp, root) = temp_index_root();
        let src = root.join("src");
        let dest = root.join("dest");
        std::fs::write(&src, b"source").unwrap();
        std::fs::write(&dest, b"destination").unwrap();
        assert!(super::super::move_noreplace_write_through(&src, &dest).is_err());
        assert_eq!(std::fs::read(&src).unwrap(), b"source");
        assert_eq!(std::fs::read(&dest).unwrap(), b"destination");
        let vacant = root.join("vacant");
        super::super::move_noreplace_write_through(&src, &vacant).unwrap();
        assert!(!src.exists());
        assert_eq!(std::fs::read(vacant).unwrap(), b"source");
    }

    /// A missing file has no identity, and the error says so rather than
    /// returning something a comparison would accept.
    #[test]
    fn a_missing_file_has_no_identity() {
        let (_tmp, root) = temp_index_root();
        let err = identity_of_path(&root.join("absent.bin")).unwrap_err();
        assert!(
            err.contains("CreateFileW"),
            "the failure must name the call that failed: {err}"
        );
    }

    /// The pinned open withholds delete sharing, which is what makes GC's
    /// `remove_file` fail while a read is in progress — and what a `std` open
    /// (which always requests `FILE_SHARE_DELETE`) would not have done.
    #[test]
    fn the_pinned_open_withholds_delete_sharing() {
        let (_tmp, root) = temp_index_root();
        let file = root.join("pinned.json");
        std::fs::write(&file, b"{}").unwrap();

        let pinned = open_manifest_for_pinning(&file).unwrap();
        let refused = std::fs::remove_file(&file);
        assert!(
            refused.is_err(),
            "a manifest opened for pinning must not be deletable while pinned"
        );
        assert_eq!(refused.unwrap_err().raw_os_error(), Some(32));
        drop(pinned);
        std::fs::remove_file(&file).expect("the pin must not leave protection behind");
    }

    /// …and it refuses to open a reparse point, so a symlink substituted at a
    /// manifest path is never read through.
    #[test]
    fn the_pinned_open_refuses_a_reparse_point() {
        let (_tmp, root) = temp_index_root();
        let real = root.join("real.json");
        let link = root.join("link.json");
        std::fs::write(&real, b"{}").unwrap();
        std::os::windows::fs::symlink_file(&real, &link).unwrap();

        let err = open_manifest_for_pinning(&link).unwrap_err();
        assert!(
            err.contains("plain file"),
            "a reparse point must be refused by name: {err}"
        );
    }
}
