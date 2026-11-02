//! Descriptor-relative, create-only restore primitives.
//!
//! Directory handles pin the directories we open; they do not prevent a
//! non-cooperating process from renaming those directories or modifying open
//! inodes. Workspace mutation locks serialize only cooperating writers.

use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = Sha256::digest(bytes);
    let mut result = String::with_capacity(bytes.len() * 2);
    for &byte in bytes.iter() {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 15) as usize] as char);
    }
    result
}

pub(super) fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// The completion marker may be visible, but its durability could not be
/// confirmed. Reopen the same receipt and retry the completion barrier.
#[derive(Debug, thiserror::Error)]
#[error("restore completion publication for {receipt_id} (receipt SHA-256 {receipt_digest}) is visible or uncertain, but durability is uncertain: {reason}; recheck the exact receipt and retry its completion barrier")]
pub struct CompletionDurabilityUncertain {
    pub receipt_id: String,
    pub receipt_digest: String,
    pub reason: String,
}

pub(super) struct Document {
    pub(super) bytes: Vec<u8>,
    #[cfg(unix)]
    identity: (u64, u64),
    #[cfg(windows)]
    identity: (u64, [u8; 16]),
}

#[cfg(unix)]
mod imp {
    use super::*;
    use anyhow::Context as _;
    use std::ffi::{CStr, CString, OsStr, OsString};
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;

    fn cstr(name: &OsStr) -> anyhow::Result<CString> {
        Ok(CString::new(name.as_bytes()).context("path contains NUL")?)
    }

    fn child(dir: &File, name: &OsStr, flags: libc::c_int, mode: libc::mode_t) -> anyhow::Result<File> {
        let name = cstr(name)?;
        // SAFETY: name is NUL-terminated and dir remains open for the call.
        let fd = unsafe { libc::openat(dir.as_raw_fd(), name.as_ptr(), flags | libc::O_CLOEXEC | libc::O_NOFOLLOW, mode as libc::c_uint) };
        if fd < 0 { return Err(std::io::Error::last_os_error().into()); }
        // SAFETY: openat returned a new owned descriptor.
        Ok(unsafe { File::from_raw_fd(fd) })
    }

    fn dir_at(dir: &File, name: &str) -> anyhow::Result<File> {
        let file = child(dir, OsStr::new(name), libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        if !file.metadata()?.is_dir() { anyhow::bail!("{name} is not a directory"); }
        Ok(file)
    }
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    fn errno_ptr() -> Option<*mut libc::c_int> {
        // SAFETY: libc returns this thread's errno storage.
        Some(unsafe { libc::__error() })
    }

    #[cfg(any(target_os = "linux", target_os = "dragonfly"))]
    fn errno_ptr() -> Option<*mut libc::c_int> {
        // SAFETY: libc returns this thread's errno storage.
        Some(unsafe { libc::__errno_location() })
    }

    #[cfg(any(target_os = "android", target_os = "netbsd", target_os = "openbsd"))]
    fn errno_ptr() -> Option<*mut libc::c_int> {
        // SAFETY: libc returns this thread's errno storage.
        Some(unsafe { libc::__errno() })
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "freebsd",
        target_os = "linux", target_os = "dragonfly", target_os = "android",
        target_os = "netbsd", target_os = "openbsd")))]
    fn errno_ptr() -> Option<*mut libc::c_int> { None }

    /// Enumerate a pinned directory without reopening it through a pathname.
    /// Opening "." relative to the descriptor gives each scan its own
    /// directory offset; dup alone would share offsets between scans.
    fn directory_entries(dir: &File) -> anyhow::Result<Vec<OsString>> {
        let errno = errno_ptr().ok_or_else(|| anyhow::anyhow!("descriptor directory enumeration is unsupported on this platform"))?;
        let stream_fd = child(dir, OsStr::new("."), libc::O_RDONLY | libc::O_DIRECTORY, 0)?.into_raw_fd();
        // SAFETY: stream_fd is a newly owned directory descriptor.
        let stream = unsafe { libc::fdopendir(stream_fd) };
        if stream.is_null() {
            let error = std::io::Error::last_os_error();
            // SAFETY: fdopendir failed and did not consume the descriptor.
            unsafe { libc::close(stream_fd); }
            return Err(error.into());
        }
        struct Stream(*mut libc::DIR);
        impl Drop for Stream {
            fn drop(&mut self) {
                // SAFETY: the stream was returned by fdopendir and is owned here.
                unsafe { libc::closedir(self.0); }
            }
        }
        let stream = Stream(stream);
        let mut names = Vec::new();
        loop {
            // SAFETY: errno points to thread-local libc storage and stream is live.
            unsafe { *errno = 0; }
            let entry = unsafe { libc::readdir(stream.0) };
            if entry.is_null() {
                let error = unsafe { *errno };
                if error != 0 { return Err(std::io::Error::from_raw_os_error(error).into()); }
                break;
            }
            // SAFETY: readdir returns a live entry with a NUL-terminated d_name.
            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if name != b"." && name != b".." {
                names.push(OsStr::from_bytes(name).to_os_string());
            }
        }
        Ok(names)
    }


    fn read_at(dir: &File, name: &str) -> anyhow::Result<(File, Vec<u8>)> {
        let mut file = child(dir, OsStr::new(name), libc::O_RDONLY | libc::O_NONBLOCK, 0)?;
        let meta = file.metadata()?;
        if !meta.is_file() || meta.nlink() != 1 { anyhow::bail!("{name} is not a singly-linked regular file"); }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok((file, bytes))
    }

    fn marker(dir: &File, name: &str, bytes: &[u8]) -> anyhow::Result<()> {
        match child(dir, OsStr::new(name), libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL, 0o600) {
            Ok(mut file) => {
                file.write_all(bytes)?;
                file.sync_all()?;
                dir.sync_all()?;
                Ok(())
            }
            Err(error) if error.downcast_ref::<std::io::Error>().is_some_and(|e| e.kind() == std::io::ErrorKind::AlreadyExists) => {
                let (_, current) = read_at(dir, name)?;
                if current != bytes { anyhow::bail!("restore marker {name} conflicts or is incomplete"); }
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    fn exists_error(error: &anyhow::Error) -> bool {
        error.downcast_ref::<std::io::Error>().is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound)
    }

    pub(in crate::undo) struct Context {
        trash: File,
        workspace: File,
        receipt_file: File,
        pub(in crate::undo) receipt_bytes: Vec<u8>,
        pub(in crate::undo) receipt_hash: String,
        id: String,
    }

    impl Context {
        pub(in crate::undo) fn open(workspace: &Path, id: &str) -> anyhow::Result<Self> {
            if !valid_id(id) { anyhow::bail!("invalid receipt id"); }
            let name = cstr(workspace.as_os_str())?;
            // SAFETY: the path is NUL-terminated; returned descriptor is owned.
            let fd = unsafe { libc::open(name.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
            if fd < 0 { return Err(std::io::Error::last_os_error().into()); }
            let workspace = unsafe { File::from_raw_fd(fd) };
            let trash = dir_at(&workspace, "trash")?;
            let (receipt_file, receipt_bytes) = read_at(&trash, &format!("receipt-{id}.json"))?;
            let receipt_hash = digest(&receipt_bytes);
            Ok(Self { trash, workspace, receipt_file, receipt_bytes, receipt_hash, id: id.into() })
        }

        pub(in crate::undo) fn verify_receipt(&self) -> anyhow::Result<()> {
            let mut file = self.receipt_file.try_clone()?;
            file.seek(SeekFrom::Start(0))?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            if bytes != self.receipt_bytes { anyhow::bail!("receipt changed while restore was in progress"); }
            Ok(())
        }

        pub(in crate::undo) fn body(&self) -> anyhow::Result<Vec<u8>> {
            let (_, bytes) = read_at(&self.trash, &format!("body-{}.md", self.id))?;
            Ok(bytes)
        }

        pub(in crate::undo) fn consumed(&self) -> anyhow::Result<bool> {
            let name = format!("consumed-{}-{}.json", self.id, self.receipt_hash);
            match read_at(&self.trash, &name) {
                Ok((_, bytes)) if bytes == self.receipt_hash.as_bytes() => Ok(true),
                Ok(_) => anyhow::bail!("invalid completion marker {name}"),
                Err(e) if exists_error(&e) => Ok(false),
                Err(e) => Err(e),
            }
        }

        pub(in crate::undo) fn consume(&self) -> anyhow::Result<()> {
            self.consume_with_sync(|file, dir| { file.sync_all()?; dir.sync_all() })
        }

        fn consume_with_sync<F>(&self, sync: F) -> anyhow::Result<()>
        where F: FnOnce(&File, &File) -> std::io::Result<()> {
            self.verify_receipt()?;
            let name = format!("consumed-{}-{}.json", self.id, self.receipt_hash);
            let uncertain = |error: std::io::Error| CompletionDurabilityUncertain {
                receipt_id: self.id.clone(), receipt_digest: self.receipt_hash.clone(), reason: error.to_string(),
            };
            let file = match child(&self.trash, OsStr::new(&name), libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL, 0o600) {
                Ok(mut file) => {
                    file.write_all(self.receipt_hash.as_bytes()).map_err(&uncertain)?;
                    file
                }
                Err(error) if error.downcast_ref::<std::io::Error>().is_some_and(|e| e.kind() == std::io::ErrorKind::AlreadyExists) => {
                    let mut file = child(&self.trash, OsStr::new(&name), libc::O_RDWR | libc::O_NONBLOCK, 0)?;
                    let meta = file.metadata()?;
                    if !meta.is_file() || meta.nlink() != 1 { anyhow::bail!("invalid completion marker {name}"); }
                    let mut bytes = Vec::new();
                    file.read_to_end(&mut bytes)?;
                    if bytes != self.receipt_hash.as_bytes() { anyhow::bail!("invalid completion marker {name}"); }
                    file
                }
                Err(error) => return Err(error),
            };
            sync(&file, &self.trash).map_err(uncertain)?;
            Ok(())
        }

        #[cfg(test)]
        pub(in crate::undo) fn consume_with_sync_failure(&self) -> anyhow::Result<()> {
            self.consume_with_sync(|_, _| Err(std::io::Error::other("injected completion fsync failure")))
        }

        pub(in crate::undo) fn has_journal(&self) -> anyhow::Result<bool> {
            let prefix = format!("restore-{}-", self.id);
            for name in directory_entries(&self.trash)? {
                if name.to_string_lossy().starts_with(&prefix) { return Ok(true); }
            }
            Ok(false)
        }

        pub(in crate::undo) fn purge(self) -> anyhow::Result<()> {
            self.verify_receipt()?;
            let receipt = cstr(OsStr::new(&format!("receipt-{}.json", self.id)))?;
            let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
            // SAFETY: valid directory and filename; stat is initialized on success.
            if unsafe { libc::fstatat(self.trash.as_raw_fd(), receipt.as_ptr(), stat.as_mut_ptr(), libc::AT_SYMLINK_NOFOLLOW) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let stat = unsafe { stat.assume_init() };
            let meta = self.receipt_file.metadata()?;
            if stat.st_dev as u64 != meta.dev() || stat.st_ino as u64 != meta.ino() {
                anyhow::bail!("receipt changed before purge");
            }
            // SAFETY: unlinkat acts on a single leaf in the pinned trash directory.
            if unsafe { libc::unlinkat(self.trash.as_raw_fd(), receipt.as_ptr(), 0) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let body = cstr(OsStr::new(&format!("body-{}.md", self.id)))?;
            if unsafe { libc::unlinkat(self.trash.as_raw_fd(), body.as_ptr(), 0) } != 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() != std::io::ErrorKind::NotFound { return Err(error.into()); }
            }
            self.trash.sync_all()?;
            Ok(())
        }

        pub(in crate::undo) fn ensure_ready(&self) -> anyhow::Result<()> {
            let prefix = format!("restore-{}-", self.id);
            for entry in directory_entries(&self.trash)? {
                let name = entry.to_string_lossy().into_owned();
                if !name.starts_with(&prefix) || !name.ends_with(".prepared") { continue; }
                let base = name.trim_end_matches(".prepared");
                match read_at(&self.trash, &format!("{base}.postimage")) {
                    Ok(_) => match read_at(&self.trash, &format!("{base}.updated")) {
                        Ok((_, completed)) => {
                            let (_, prepared) = read_at(&self.trash, &name)?;
                            if completed != prepared { anyhow::bail!("restore update marker is damaged"); }
                        }
                        Err(e) if exists_error(&e) => anyhow::bail!("restore has an incomplete document update; preserve journal {base} for manual recovery"),
                        Err(e) => return Err(e),
                    },
                    Err(e) if exists_error(&e) => {}
                    Err(e) => return Err(e),
                }
            }
            Ok(())
        }

        pub(in crate::undo) fn documents(&self) -> anyhow::Result<Vec<(std::path::PathBuf, Vec<u8>)>> {
            let mut result = Vec::new();
            for &kind in crate::workspace::ARTIFACT_DIRS {
                let dir = match dir_at(&self.workspace, kind) {
                    Ok(dir) => dir,
                    Err(error) if exists_error(&error) => continue,
                    Err(error) => return Err(error.context(format!("cannot open artifact directory {kind}"))),
                };
                for name in directory_entries(&dir)? {
                    let path = std::path::Path::new(&name);
                    if path.extension().and_then(|ext| ext.to_str()) != Some("md") { continue; }
                    let leaf = name.to_str().ok_or_else(|| anyhow::anyhow!("artifact filename is not UTF-8"))?;
                    let (_, bytes) = read_at(&dir, leaf).with_context(|| format!("cannot read {kind}/{leaf}"))?;
                    result.push((std::path::PathBuf::from(kind).join(name), bytes));
                }
            }
            Ok(result)
        }

        pub(in crate::undo) fn read_document(&self, kind_dir: &str, leaf: &str) -> anyhow::Result<Document> {
            let dir = dir_at(&self.workspace, kind_dir)?;
            let (file, bytes) = read_at(&dir, leaf)?;
            let meta = file.metadata()?;
            if meta.nlink() != 1 { anyhow::bail!("restore refuses a multiply-linked document"); }
            Ok(Document { bytes, identity: (meta.dev(), meta.ino()) })
        }

        pub(in crate::undo) fn replace_document(&self, kind_dir: &str, leaf: &str, before: &Document, after: &[u8]) -> anyhow::Result<()> {
            self.verify_receipt()?;
            let dir = dir_at(&self.workspace, kind_dir)?;
            let mut file = child(&dir, OsStr::new(leaf), libc::O_RDWR | libc::O_NONBLOCK, 0)?;
            let meta = file.metadata()?;
            if !meta.is_file() || meta.nlink() != 1 || (meta.dev(), meta.ino()) != before.identity {
                anyhow::bail!("restore document identity changed before update");
            }
            let mut current = Vec::new();
            file.read_to_end(&mut current)?;
            if current != before.bytes { anyhow::bail!("restore document changed before update"); }
            if current == after { return Ok(()); }
            let operation = format!("{}\n{}\n{}\n{}\n{}\n", self.receipt_hash, kind_dir, leaf, digest(&current), digest(after));
            let token = digest(operation.as_bytes());
            marker(&self.trash, &format!("restore-{}-{token}.preimage", self.id), &current)?;
            marker(&self.trash, &format!("restore-{}-{token}.postimage", self.id), after)?;
            marker(&self.trash, &format!("restore-{}-{token}.prepared", self.id), operation.as_bytes())?;
            file.seek(SeekFrom::Start(0))?;
            file.write_all(after)?;
            file.set_len(after.len() as u64)?;
            file.sync_all()?;
            dir.sync_all()?;
            marker(&self.trash, &format!("restore-{}-{token}.updated", self.id), operation.as_bytes())?;
            Ok(())
        }

        pub(in crate::undo) fn completed_target(&self, kind_dir: &str, leaf: &str, bytes: &[u8]) -> anyhow::Result<bool> {
            let operation = format!("{}\n{}\n{}\n{}\n", self.receipt_hash, kind_dir, leaf, digest(bytes));
            let token = digest(operation.as_bytes());
            let prepared = format!("restore-{}-{token}.prepared", self.id);
            let claimed = format!("restore-{}-{token}.claimed", self.id);
            let (_, stored) = match read_at(&self.trash, &prepared) {
                Ok(value) => value,
                Err(e) if exists_error(&e) => return Ok(false),
                Err(e) => return Err(e),
            };
            if stored != operation.as_bytes() { anyhow::bail!("restore prepared marker is damaged"); }
            let (_, claim) = match read_at(&self.trash, &claimed) {
                Ok(value) => value,
                Err(e) if exists_error(&e) => return Ok(false),
                Err(e) => return Err(e),
            };
            let dir = dir_at(&self.workspace, kind_dir)?;
            let (target, current) = match read_at(&dir, leaf) {
                Ok(value) => value,
                Err(e) if exists_error(&e) => return Ok(false),
                Err(e) => return Err(e),
            };
            let metadata = target.metadata()?;
            if claim != format!("{}:{}", metadata.dev(), metadata.ino()).as_bytes() || current != bytes {
                anyhow::bail!("restore target conflicts with its claimed postimage");
            }
            Ok(true)
        }

        pub(in crate::undo) fn check_create(&self, kind_dir: &str, leaf: &str, bytes: &[u8]) -> anyhow::Result<()> {
            if leaf.is_empty() || Path::new(leaf).components().count() != 1 || !leaf.ends_with(".md") {
                anyhow::bail!("invalid restore target filename");
            }
            self.verify_receipt()?;
            let dir = dir_at(&self.workspace, kind_dir)?;
            if self.completed_target(kind_dir, leaf, bytes)? { return Ok(()); }
            let name = cstr(OsStr::new(leaf))?;
            let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
            // SAFETY: dir and name remain valid; fstatat initializes stat on success.
            let result = unsafe { libc::fstatat(dir.as_raw_fd(), name.as_ptr(), stat.as_mut_ptr(), libc::AT_SYMLINK_NOFOLLOW) };
            if result == 0 {
                anyhow::bail!("restore destination already exists and may hold a different canonical document; refusing to overwrite");
            }
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::NotFound { return Err(error.into()); }
            Ok(())
        }

        /// Publish verified bytes. A durable claim records the physical target;
        /// unknown or partial postimages are conflicts, never auto-overwritten.
        pub(in crate::undo) fn create(&self, kind_dir: &str, leaf: &str, bytes: &[u8]) -> anyhow::Result<()> {
            self.check_create(kind_dir, leaf, bytes)?;
            let dir = dir_at(&self.workspace, kind_dir)?;
            if self.completed_target(kind_dir, leaf, bytes)? { return Ok(()); }
            let operation = format!("{}\n{}\n{}\n{}\n", self.receipt_hash, kind_dir, leaf, digest(bytes));
            let token = digest(operation.as_bytes());
            let prepared = format!("restore-{}-{token}.prepared", self.id);
            let claimed = format!("restore-{}-{token}.claimed", self.id);
            marker(&self.trash, &prepared, operation.as_bytes())?;
            let mut target = match child(&dir, OsStr::new(leaf), libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL, 0o600) {
                Ok(file) => {
                    let metadata = file.metadata()?;
                    let identity = format!("{}:{}", metadata.dev(), metadata.ino());
                    marker(&self.trash, &claimed, identity.as_bytes())?;
                    file
                }
                Err(error) if error.downcast_ref::<std::io::Error>().is_some_and(|e| e.kind() == std::io::ErrorKind::AlreadyExists) => {
                    let (_, claim) = read_at(&self.trash, &claimed).context("restore target occupied without a durable ownership claim")?;
                    let (target, current) = read_at(&dir, leaf)?;
                    let metadata = target.metadata()?;
                    let identity = format!("{}:{}", metadata.dev(), metadata.ino());
                    if claim != identity.as_bytes() || current != bytes {
                        anyhow::bail!("restore target conflicts with its claimed postimage");
                    }
                    return Ok(());
                }
                Err(error) => return Err(error),
            };
            target.write_all(bytes)?;
            target.sync_all()?;
            dir.sync_all()?;
            Ok(())
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use anyhow::Context as _;
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use std::os::windows::io::AsRawHandle;
    use std::path::{Component, PathBuf};
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO, FILE_READ_ATTRIBUTES, FILE_SHARE_READ,
        FileIdInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
    };

    fn directory(path: &Path) -> anyhow::Result<File> {
        let file = OpenOptions::new().read(true).access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        let meta = file.metadata()?;
        if !meta.is_dir() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            anyhow::bail!("{} is not a non-reparse directory", path.display());
        }
        Ok(file)
    }

    fn pin_workspace(path: &Path) -> anyhow::Result<Vec<File>> {
        if !path.is_absolute() { anyhow::bail!("safe restore requires an absolute workspace path"); }
        let mut pins = Vec::new();
        let mut current = PathBuf::new();
        for component in path.components() {
            match component {
                Component::Prefix(_) => current.push(component.as_os_str()),
                Component::RootDir | Component::Normal(_) => {
                    current.push(component.as_os_str());
                    pins.push(directory(&current)?);
                }
                _ => anyhow::bail!("workspace path contains traversal"),
            }
        }
        Ok(pins)
    }

    fn plain(file: &File) -> anyhow::Result<()> {
        let meta = file.metadata()?;
        if !meta.is_file() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            anyhow::bail!("restore file is not a regular non-reparse file");
        }
        Ok(())
    }

    fn identity(file: &File) -> anyhow::Result<((u64, [u8; 16]), u32)> {
        let handle = file.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE;
        // SAFETY: the handle is live and the output buffers have the declared sizes.
        unsafe {
            let mut id: FILE_ID_INFO = std::mem::zeroed();
            let mut info: BY_HANDLE_FILE_INFORMATION = std::mem::zeroed();
            if GetFileInformationByHandleEx(handle, FileIdInfo, (&mut id as *mut FILE_ID_INFO).cast(), std::mem::size_of::<FILE_ID_INFO>() as u32) == 0
                || GetFileInformationByHandle(handle, &mut info) == 0
            {
                return Err(std::io::Error::last_os_error().into());
            }
            Ok(((id.VolumeSerialNumber, id.FileId.Identifier), info.nNumberOfLinks))
        }
    }

    fn read_file(path: &Path) -> anyhow::Result<(File, Vec<u8>)> {
        let mut file = OpenOptions::new().read(true).share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT).open(path)?;
        plain(&file)?;
        if identity(&file)?.1 != 1 { anyhow::bail!("restore refuses a multiply-linked file"); }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok((file, bytes))
    }

    fn marker(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
        match OpenOptions::new().write(true).create_new(true).share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT).open(path)
        {
            Ok(mut file) => { file.write_all(bytes)?; file.sync_all()?; Ok(()) }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let (_, existing) = read_file(path)?;
                if existing != bytes { anyhow::bail!("restore marker conflicts or is incomplete"); }
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    }

    fn missing(error: &anyhow::Error) -> bool {
        error.downcast_ref::<std::io::Error>().is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound)
    }

    fn leaf(name: &str) -> anyhow::Result<()> {
        let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
        if name.is_empty() || name.ends_with('.') || name.ends_with(' ') || name.contains('/') || name.contains('\\') || name.contains(':')
            || name.chars().any(|c| c.is_control())
            || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6" | "COM7" | "COM8" | "COM9" | "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9")
        { anyhow::bail!("invalid Windows restore filename"); }
        Ok(())
    }

    pub(in crate::undo) struct Context {
        workspace: PathBuf,
        trash: PathBuf,
        _pins: Vec<File>,
        _trash_pin: File,
        receipt_file: File,
        pub(in crate::undo) receipt_bytes: Vec<u8>,
        pub(in crate::undo) receipt_hash: String,
        id: String,
    }
    impl Context {
        pub(in crate::undo) fn open(workspace: &Path, id: &str) -> anyhow::Result<Self> {
            if !valid_id(id) { anyhow::bail!("invalid receipt id"); }
            let pins = pin_workspace(workspace)?;
            let trash = workspace.join("trash");
            let trash_pin = directory(&trash)?;
            let (receipt_file, receipt_bytes) = read_file(&trash.join(format!("receipt-{id}.json")))?;
            let receipt_hash = digest(&receipt_bytes);
            Ok(Self { workspace: workspace.into(), trash, _pins: pins, _trash_pin: trash_pin, receipt_file, receipt_bytes, receipt_hash, id: id.into() })
        }
        pub(in crate::undo) fn verify_receipt(&self) -> anyhow::Result<()> {
            let mut file = self.receipt_file.try_clone()?;
            file.seek(SeekFrom::Start(0))?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            if bytes != self.receipt_bytes { anyhow::bail!("receipt changed while restore was in progress"); }
            Ok(())
        }
        pub(in crate::undo) fn body(&self) -> anyhow::Result<Vec<u8>> {
            Ok(read_file(&self.trash.join(format!("body-{}.md", self.id)))?.1)
        }
        pub(in crate::undo) fn consumed(&self) -> anyhow::Result<bool> {
            let path = self.trash.join(format!("consumed-{}-{}.json", self.id, self.receipt_hash));
            match read_file(&path) {
                Ok((_, bytes)) if bytes == self.receipt_hash.as_bytes() => Ok(true),
                Ok(_) => anyhow::bail!("invalid completion marker"),
                Err(error) if missing(&error) => Ok(false),
                Err(error) => Err(error),
            }
        }
        pub(in crate::undo) fn consume(&self) -> anyhow::Result<()> {
            self.consume_with_sync(|file| file.sync_all())
        }
        fn consume_with_sync<F>(&self, sync: F) -> anyhow::Result<()>
        where F: FnOnce(&File) -> std::io::Result<()> {
            self.verify_receipt()?;
            let path = self.trash.join(format!("consumed-{}-{}.json", self.id, self.receipt_hash));
            let uncertain = |error: std::io::Error| CompletionDurabilityUncertain {
                receipt_id: self.id.clone(), receipt_digest: self.receipt_hash.clone(), reason: error.to_string(),
            };
            let file = match OpenOptions::new().write(true).create_new(true).share_mode(FILE_SHARE_READ)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT).open(&path) {
                Ok(mut file) => {
                    file.write_all(self.receipt_hash.as_bytes()).map_err(&uncertain)?;
                    file
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let mut file = OpenOptions::new().read(true).write(true).share_mode(FILE_SHARE_READ)
                        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT).open(&path)?;
                    plain(&file)?;
                    if identity(&file)?.1 != 1 { anyhow::bail!("invalid completion marker"); }
                    let mut bytes = Vec::new();
                    file.read_to_end(&mut bytes)?;
                    if bytes != self.receipt_hash.as_bytes() { anyhow::bail!("invalid completion marker"); }
                    file
                }
                Err(error) => return Err(error.into()),
            };
            sync(&file).map_err(uncertain)?;
            Ok(())
        }
        #[cfg(test)]
        pub(in crate::undo) fn consume_with_sync_failure(&self) -> anyhow::Result<()> {
            self.consume_with_sync(|_| Err(std::io::Error::other("injected completion fsync failure")))
        }
        pub(in crate::undo) fn has_journal(&self) -> anyhow::Result<bool> {
            let prefix = format!("restore-{}-", self.id);
            for entry in std::fs::read_dir(&self.trash)? {
                if entry?.file_name().to_string_lossy().starts_with(&prefix) { return Ok(true); }
            }
            Ok(false)
        }
        pub(in crate::undo) fn purge(self) -> anyhow::Result<()> {
            self.verify_receipt()?;
            let Self { trash, _pins, _trash_pin, receipt_file, id, .. } = self;
            drop(receipt_file);
            std::fs::remove_file(trash.join(format!("receipt-{id}.json")))?;
            match std::fs::remove_file(trash.join(format!("body-{id}.md"))) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            drop((_pins, _trash_pin));
            Ok(())
        }

        pub(in crate::undo) fn ensure_ready(&self) -> anyhow::Result<()> {
            let prefix = format!("restore-{}-", self.id);
            for entry in std::fs::read_dir(&self.trash)? {
                let name = entry?.file_name().to_string_lossy().into_owned();
                if !name.starts_with(&prefix) || !name.ends_with(".prepared") { continue; }
                let base = name.trim_end_matches(".prepared");
                match read_file(&self.trash.join(format!("{base}.postimage"))) {
                    Ok(_) => match read_file(&self.trash.join(format!("{base}.updated"))) {
                        Ok((_, completed)) => {
                            let (_, prepared) = read_file(&self.trash.join(&name))?;
                            if completed != prepared { anyhow::bail!("restore update marker is damaged"); }
                        }
                        Err(e) if missing(&e) => anyhow::bail!("restore has an incomplete document update; preserve journal {base} for manual recovery"),
                        Err(e) => return Err(e),
                    },
                    Err(e) if missing(&e) => {}
                    Err(e) => return Err(e),
                }
            }
            Ok(())
        }

        pub(in crate::undo) fn documents(&self) -> anyhow::Result<Vec<(PathBuf, Vec<u8>)>> {
            let mut result = Vec::new();
            for &kind in crate::workspace::ARTIFACT_DIRS {
                let dir = self.workspace.join(kind);
                let _pin = match directory(&dir) {
                    Ok(pin) => pin,
                    Err(error) if missing(&error) => continue,
                    Err(error) => return Err(error.context(format!("cannot open artifact directory {kind}"))),
                };
                for entry in std::fs::read_dir(&dir)? {
                    let entry = entry?;
                    let name = entry.file_name();
                    if Path::new(&name).extension().and_then(|ext| ext.to_str()) != Some("md") { continue; }
                    let filename = name.to_str().ok_or_else(|| anyhow::anyhow!("artifact filename is not UTF-8"))?;
                    leaf(filename)?;
                    let (_, bytes) = read_file(&dir.join(&name)).with_context(|| format!("cannot read {kind}/{filename}"))?;
                    result.push((PathBuf::from(kind).join(name), bytes));
                }
            }
            Ok(result)
        }

        pub(in crate::undo) fn read_document(&self, kind: &str, name: &str) -> anyhow::Result<Document> {
            leaf(kind)?; leaf(name)?;
            let dir = self.workspace.join(kind);
            let _pin = directory(&dir)?;
            let (file, bytes) = read_file(&dir.join(name))?;
            let (identity, links) = identity(&file)?;
            if links != 1 { anyhow::bail!("restore refuses a multiply-linked document"); }
            Ok(Document { bytes, identity })
        }
        pub(in crate::undo) fn replace_document(&self, kind: &str, name: &str, before: &Document, after: &[u8]) -> anyhow::Result<()> {
            self.verify_receipt()?;
            leaf(kind)?; leaf(name)?;
            let dir = self.workspace.join(kind);
            let _pin = directory(&dir)?;
            let mut file = OpenOptions::new().read(true).write(true).share_mode(FILE_SHARE_READ)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT).open(dir.join(name))?;
            plain(&file)?;
            let (id, links) = identity(&file)?;
            if links != 1 || id != before.identity { anyhow::bail!("restore document identity changed before update"); }
            let mut current = Vec::new();
            file.read_to_end(&mut current)?;
            if current != before.bytes { anyhow::bail!("restore document changed before update"); }
            if current == after { return Ok(()); }
            let operation = format!("{}\\n{}\\n{}\\n{}\\n{}\\n", self.receipt_hash, kind, name, digest(&current), digest(after));
            let token = digest(operation.as_bytes());
            marker(&self.trash.join(format!("restore-{}-{token}.preimage", self.id)), &current)?;
            marker(&self.trash.join(format!("restore-{}-{token}.postimage", self.id)), after)?;
            marker(&self.trash.join(format!("restore-{}-{token}.prepared", self.id)), operation.as_bytes())?;
            file.seek(SeekFrom::Start(0))?;
            file.write_all(after)?;
            file.set_len(after.len() as u64)?;
            file.sync_all()?;
            marker(&self.trash.join(format!("restore-{}-{token}.updated", self.id)), operation.as_bytes())
        }
        pub(in crate::undo) fn completed_target(&self, kind: &str, name: &str, bytes: &[u8]) -> anyhow::Result<bool> {
            leaf(kind)?; leaf(name)?;
            let operation = format!("{}\\n{}\\n{}\\n{}\\n", self.receipt_hash, kind, name, digest(bytes));
            let token = digest(operation.as_bytes());
            let prep = self.trash.join(format!("restore-{}-{token}.prepared", self.id));
            let claim = self.trash.join(format!("restore-{}-{token}.claimed", self.id));
            let (_, saved) = match read_file(&prep) { Ok(value) => value, Err(e) if missing(&e) => return Ok(false), Err(e) => return Err(e) };
            if saved != operation.as_bytes() { anyhow::bail!("restore prepared marker is damaged"); }
            let (_, claimed) = match read_file(&claim) { Ok(value) => value, Err(e) if missing(&e) => return Ok(false), Err(e) => return Err(e) };
            let dir = self.workspace.join(kind);
            let _pin = directory(&dir)?;
            let (file, current) = match read_file(&dir.join(name)) { Ok(value) => value, Err(e) if missing(&e) => return Ok(false), Err(e) => return Err(e) };
            let (id, links) = identity(&file)?;
            if links != 1 || claimed != format!("{}:{:02x?}", id.0, id.1).as_bytes() || current != bytes {
                anyhow::bail!("restore target conflicts with its claimed postimage");
            }
            Ok(true)
        }
        pub(in crate::undo) fn check_create(&self, kind: &str, name: &str, bytes: &[u8]) -> anyhow::Result<()> {
            self.verify_receipt()?;
            leaf(kind)?; leaf(name)?;
            if !name.ends_with(".md") { anyhow::bail!("invalid restore target filename"); }
            let dir = self.workspace.join(kind);
            let _pin = directory(&dir)?;
            if self.completed_target(kind, name, bytes)? { return Ok(()); }
            match std::fs::symlink_metadata(dir.join(name)) {
                Ok(_) => anyhow::bail!("restore destination already exists and may hold a different canonical document; refusing to overwrite"),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.into()),
            }
        }

        pub(in crate::undo) fn create(&self, kind: &str, name: &str, bytes: &[u8]) -> anyhow::Result<()> {
            self.check_create(kind, name, bytes)?;
            let dir = self.workspace.join(kind);
            let _pin = directory(&dir)?;
            if self.completed_target(kind, name, bytes)? { return Ok(()); }
            let operation = format!("{}\\n{}\\n{}\\n{}\\n", self.receipt_hash, kind, name, digest(bytes));
            let token = digest(operation.as_bytes());
            marker(&self.trash.join(format!("restore-{}-{token}.prepared", self.id)), operation.as_bytes())?;
            let mut file = OpenOptions::new().write(true).create_new(true).share_mode(FILE_SHARE_READ)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT).open(dir.join(name))?;
            let (id, _) = identity(&file)?;
            marker(&self.trash.join(format!("restore-{}-{token}.claimed", self.id)), format!("{}:{:02x?}", id.0, id.1).as_bytes())?;
            file.write_all(bytes)?;
            file.sync_all()?;
            Ok(())
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod imp {
    use super::*;
    pub(in crate::undo) struct Context { pub(in crate::undo) receipt_bytes: Vec<u8>, pub(in crate::undo) receipt_hash: String }
    impl Context {
        pub(in crate::undo) fn open(_: &Path, _: &str) -> anyhow::Result<Self> { anyhow::bail!("safe restore requires descriptor-relative filesystem support on this platform") }
        pub(in crate::undo) fn verify_receipt(&self) -> anyhow::Result<()> { anyhow::bail!("safe restore unavailable") }
        pub(in crate::undo) fn body(&self) -> anyhow::Result<Vec<u8>> { anyhow::bail!("safe restore unavailable") }
        pub(in crate::undo) fn consumed(&self) -> anyhow::Result<bool> { anyhow::bail!("safe restore unavailable") }
        pub(in crate::undo) fn consume(&self) -> anyhow::Result<()> { anyhow::bail!("safe restore unavailable") }
        pub(in crate::undo) fn ensure_ready(&self) -> anyhow::Result<()> { anyhow::bail!("safe restore unavailable") }
        pub(in crate::undo) fn has_journal(&self) -> anyhow::Result<bool> { anyhow::bail!("safe restore unavailable") }
        pub(in crate::undo) fn check_create(&self, _: &str, _: &str, _: &[u8]) -> anyhow::Result<()> { anyhow::bail!("safe restore unavailable") }
        pub(in crate::undo) fn purge(self) -> anyhow::Result<()> { anyhow::bail!("safe restore unavailable") }
        pub(in crate::undo) fn documents(&self) -> anyhow::Result<Vec<(std::path::PathBuf, Vec<u8>)>> { anyhow::bail!("safe restore unavailable") }
        pub(in crate::undo) fn create(&self, _: &str, _: &str, _: &[u8]) -> anyhow::Result<()> { anyhow::bail!("safe restore unavailable") }
        pub(in crate::undo) fn completed_target(&self, _: &str, _: &str, _: &[u8]) -> anyhow::Result<bool> { anyhow::bail!("safe restore unavailable") }
        pub(in crate::undo) fn read_document(&self, _: &str, _: &str) -> anyhow::Result<Document> { anyhow::bail!("safe restore unavailable") }
        pub(in crate::undo) fn replace_document(&self, _: &str, _: &str, _: &Document, _: &[u8]) -> anyhow::Result<()> { anyhow::bail!("safe restore unavailable") }
    }
}

pub(super) use imp::Context;

/// Capture a complete no-follow document set, then use the shared selector.
pub(super) fn resolve(
    context: &Context,
    workspace: &Path,
    selector: &str,
) -> Result<Option<std::path::PathBuf>, crate::artifact::canonical_key::ResolveError> {
    use crate::artifact::canonical_key::{ArtifactCandidate, ResolveError, resolve_artifact};
    let documents = context.documents().map_err(|error| ResolveError::IncompleteCapture {
        message: error.to_string(),
    })?;
    let mut candidates = Vec::with_capacity(documents.len());
    for (relative_path, bytes) in documents {
        let text = String::from_utf8_lossy(&bytes);
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&text).map_err(|error| {
            ResolveError::IncompleteCapture { message: format!("{} cannot be read: {error}", relative_path.display()) }
        })?;
        let Some(id) = fm.get("id").and_then(|value| value.as_str()) else { continue; };
        candidates.push(ArtifactCandidate {
            canonical_key: crate::artifact::canonical_key::canonical_key_from_document(id, &text),
            display_id: id.to_string(),
            title: fm.get("title").and_then(|value| value.as_str()).unwrap_or_default().to_string(),
            relative_path,
        });
    }
    candidates.sort_by(|a, b| a.canonical_key.cmp(&b.canonical_key).then_with(|| a.relative_path.cmp(&b.relative_path)));
    Ok(resolve_artifact(selector, &candidates)?.map(|resolved| workspace.join(resolved.relative_path)))
}
