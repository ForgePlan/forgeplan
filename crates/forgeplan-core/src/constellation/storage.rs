//! Active table URIs and their physical root.
//!
//! # The rule
//!
//! Every URI used to **create, reopen, check out or restore** an active table
//! carries the `file-object-store` scheme:
//!
//! ```text
//! file-object-store:///…/.forgeplan/lance/constellation_artifacts_v1.lance
//! file-object-store:///…/.forgeplan/lance/constellation_relations_v1.lance
//! ```
//!
//! Not `file://`. Not a plain filesystem path. In `lance-io 4.0.0` the
//! ordinary `file` scheme reaches `LocalWriter` / `std::fs` directly for
//! object creation and copy, so a store installed through
//! `WrappingObjectStore` observes **nothing** on those paths — the durability
//! tracker would report an empty closure over a generation that wrote real
//! data, and the generation would publish on the strength of a barrier that
//! never ran. A plain path or `file://` remains legitimate only as a local
//! **read** fast path that does not wrap, and
//! [`assert_not_wrapped_route`] is the guard against one leaking into a
//! writer.
//!
//! # Where the names come from
//!
//! From the versioned protocol marker — `ProtocolMarker::table_names` plus
//! `storage_scheme` — and never from rebuilding a name off a plain local
//! root. A URI derived from the filesystem rather than from the marker can
//! drift from what the marker says was published, and then a reader and a
//! writer would address two different datasets under one identity.
//!
//! # Why the alias creates no second owner
//!
//! `file-object-store:///x/y` and the plain path `/x/y` name one directory.
//! [`physical_root`] maps a URI back to that directory, and
//! [`IndexLocation`](super::owner::IndexLocation) keys the OS owner lock off
//! the physical root. The scheme is an alias for storage access, never a
//! second identity — so nobody can acquire a second owner by spelling the
//! index differently.
//!
//! Automatic cleanup, vacuum and `remove_dir_all` stay off. The
//! `file-object-store` provider's `remove_dir_all` has a filesystem tail that
//! deletes more than an object store should, and an index that cleans itself
//! up on open cannot survive a crash mid-publication.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::durability::{DurabilityTrackingStore, PlatformDurability, StageTracker};
use super::manifest::{
    BootstrapState, CurrentDescriptor, GenerationManifest, ProtocolMarker,
    STORAGE_SCHEME_FILE_OBJECT_STORE, TABLE_ARTIFACTS_V1, TABLE_RELATIONS_V1,
};
use super::{ConstellationError, sha256_hex};

/// Relative path of the bootstrap marker inside the index root.
pub const PROTOCOL_RELATIVE_PATH: &str = "constellation/PROTOCOL.json";

/// File name of the visibility pointer inside the index root.
pub const CURRENT_FILE_NAME: &str = "CURRENT.json";

/// Subdirectory holding immutable generation manifests.
pub const GENERATIONS_DIR: &str = "generations";

/// The two active tables, as `(role, name)`.
///
/// Order is fixed so a manifest's `table_names` and a caller's expectation
/// cannot disagree about which is which.
pub const ACTIVE_TABLES: [(&str, &str); 2] = [
    ("artifacts", TABLE_ARTIFACTS_V1),
    ("relations", TABLE_RELATIONS_V1),
];

/// Suffix of the `.lance` dataset directory Lance creates for a table.
const DATASET_SUFFIX: &str = ".lance";

/// Build the active URI for one table role from the versioned protocol.
///
/// # Errors
///
/// [`ConstellationError::IndexProtocolMismatch`] when the marker does not
/// declare this role, or declares a scheme other than
/// `file-object-store`. Not a lookup failure and not a fallback: a marker
/// that does not name the table is a protocol mismatch, and quietly
/// substituting one is how two builds end up writing different schemas into
/// one index.
pub fn table_uri_for_role(
    marker: &ProtocolMarker,
    role: &str,
    index_root: &Path,
) -> Result<String, ConstellationError> {
    marker.validate()?;
    let name = ACTIVE_TABLES
        .iter()
        .find(|(candidate, _)| *candidate == role)
        .map(|(_, name)| *name)
        .ok_or_else(|| ConstellationError::IndexProtocolMismatch {
            detail: format!("unknown active table role `{role}`"),
        })?;

    if !marker.table_names.iter().any(|declared| declared == name) {
        return Err(ConstellationError::IndexProtocolMismatch {
            detail: format!(
                "PROTOCOL.json declares table names {:?}, which do not include `{name}`",
                marker.table_names
            ),
        });
    }

    Ok(build_table_uri(index_root, name))
}

/// Build the URI for one declared table name under `index_root`.
pub fn build_table_uri(index_root: &Path, table_name: &str) -> String {
    format!(
        "{STORAGE_SCHEME_FILE_OBJECT_STORE}://{}",
        encode_path(&index_root.join(format!("{table_name}{DATASET_SUFFIX}")))
    )
}

/// URI of the database (the index root itself), used for the `Connection`
/// that backs `Table::new`.
///
/// It carries the same scheme as the tables. A `Connection` opened on a plain
/// path and a table opened on `file-object-store` would be two storage routes
/// to one directory, and the tracker would only see one of them.
pub fn database_uri(index_root: &Path) -> String {
    format!(
        "{STORAGE_SCHEME_FILE_OBJECT_STORE}://{}",
        encode_path(index_root)
    )
}

/// Refuse a URI that must not be used to write an active table.
///
/// The plain-path and `file://` forms are legal as a local read fast path
/// that does not wrap. They are **not** legal for a writer, and this is the
/// one place that says so in a way a caller cannot skip by accident.
pub fn assert_not_wrapped_route(uri: &str) -> Result<(), ConstellationError> {
    let scheme = uri.split_once("://").map(|(scheme, _)| scheme);
    match scheme {
        Some(STORAGE_SCHEME_FILE_OBJECT_STORE) => Ok(()),
        Some(other) => Err(ConstellationError::IndexProtocolMismatch {
            detail: format!(
                "active writes must use `{STORAGE_SCHEME_FILE_OBJECT_STORE}`, not `{other}`: the \
                 plain local scheme bypasses the wrapped inner store"
            ),
        }),
        None => Err(ConstellationError::IndexProtocolMismatch {
            detail: format!(
                "active writes must use `{STORAGE_SCHEME_FILE_OBJECT_STORE}`, not a plain path: \
                 the local fast path bypasses the wrapped inner store"
            ),
        }),
    }
}

/// Whether a URI is on the wrapped active route.
pub fn is_wrapped_route(uri: &str) -> bool {
    uri.starts_with(&format!("{STORAGE_SCHEME_FILE_OBJECT_STORE}://"))
}

/// Map a `file-object-store` URI back to its physical directory.
///
/// This mirrors the provider: `FileStoreProvider::extract_path` returns
/// `Path::parse(url.path())` — the URL's **path component** — after its
/// `url.to_file_path()` fast path, which does not apply to this scheme. So
/// the mapping is: take the decoded path component, drop the leading `/` that
/// the `///` authority form produces, and interpret it as a filesystem path.
///
/// Windows drives are the reason this is a decode rather than a string
/// replace. `file-object-store:///C:/ws/…` has path `/C:/ws/…`, and the
/// drive letter is part of the path component — a naive
/// `strip_prefix("file-object-store://")` would leave the authority empty, and
/// a naive `trim_start_matches('/')` would strip nothing and keep the path
/// relative to the drive incorrectly.
///
/// # Errors
///
/// [`ConstellationError::IndexProtocolMismatch`] for any scheme other than
/// `file-object-store`. A `file://` or plain-path string is not a wrapped
/// route and must not be quietly mapped through this function.
pub fn physical_root(uri: &str) -> Result<PathBuf, ConstellationError> {
    let prefix = format!("{STORAGE_SCHEME_FILE_OBJECT_STORE}://");
    let rest =
        uri.strip_prefix(&prefix)
            .ok_or_else(|| ConstellationError::IndexProtocolMismatch {
                detail: format!("`{uri}` is not a `{STORAGE_SCHEME_FILE_OBJECT_STORE}` URI"),
            })?;

    // `///rest` — the third slash begins the path component. Two slashes and a
    // non-empty authority would be an empty-host URI we never build.
    let path_component = match rest.strip_prefix('/') {
        Some(tail) => tail,
        None => rest,
    };
    let decoded = decode_path(path_component);
    if decoded.is_empty() {
        return Err(ConstellationError::IndexProtocolMismatch {
            detail: format!("`{uri}` has no path component"),
        });
    }

    // Re-attach the leading separator the URL form consumed. A Windows drive
    // keeps its own `C:` and takes the separator with it: `/C:/ws`.
    if decoded.starts_with('/') {
        Ok(PathBuf::from(decoded))
    } else {
        Ok(PathBuf::from("/").join(decoded))
    }
}

/// The physical index root a table URI lives under — its dataset directory's
/// parent.
pub fn index_root_of(uri: &str) -> Result<PathBuf, ConstellationError> {
    let dataset = physical_root(uri)?;
    dataset.parent().map(Path::to_path_buf).ok_or_else(|| {
        ConstellationError::IndexProtocolMismatch {
            detail: format!("`{uri}` has no containing index root"),
        }
    })
}

/// Percent-encode a filesystem path for a URL path component.
///
/// Only the characters a URL path actually reserves are encoded, plus `%`
/// itself. Spaces and non-ASCII stay literal: the provider decodes the path
/// back, and over-encoding would turn `Эли` into an escape sequence a local
/// filesystem would never produce.
fn encode_path(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '%' => out.push_str("%25"),
            '?' => out.push_str("%3F"),
            '#' => out.push_str("%23"),
            c => out.push(c),
        }
    }
    out
}

/// Decode a URL path component back to a filesystem path.
fn decode_path(component: &str) -> String {
    let bytes = component.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
        {
            out.push(high * 16 + low);
            index += 3;
            continue;
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

// ===========================================================================
// Durable protocol files
//
// The ordering this file implements, and the reason for it:
//
//     joined storage writes
//       → closure barrier            (StageTracker::sync_written_set)
//       → generation manifest barrier
//       → same-filesystem CURRENT replacement   (the visibility point)
//       → platform durability ACK                (a different guarantee)
//
// Reaching the visibility point without the ACK is `publication_uncertain`.
// It is not success, and it is not a licence to report otherwise.
// ===========================================================================

/// What a restart found on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexProtocolState {
    /// No `PROTOCOL.json`. A fresh checkout and a legacy index both look like
    /// this. Normal — not corruption, and a read-only query answers
    /// ephemerally without creating anything.
    Uninitialized,
    /// A durable marker announcing an in-progress bootstrap.
    Initializing(ProtocolMarker),
    /// A published, validated generation.
    Active(ProtocolMarker),
}

impl IndexProtocolState {
    /// The bootstrap phase this state represents.
    pub fn bootstrap_state(&self) -> BootstrapState {
        match self {
            Self::Uninitialized => BootstrapState::Uninitialized,
            Self::Initializing(_) => BootstrapState::Initializing,
            Self::Active(_) => BootstrapState::Active,
        }
    }

    pub fn marker(&self) -> Option<&ProtocolMarker> {
        match self {
            Self::Uninitialized => None,
            Self::Initializing(marker) | Self::Active(marker) => Some(marker),
        }
    }
}

/// Read the bootstrap marker without creating anything.
///
/// A missing marker is `Uninitialized`. A marker that exists but does not
/// parse is **not** treated as `Uninitialized`: that would silently restart a
/// bootstrap over an index that was mid-flight, which is exactly the state
/// where restarting without looking is dangerous.
pub fn read_marker(index_root: &Path) -> Result<IndexProtocolState, ConstellationError> {
    let path = index_root.join(PROTOCOL_RELATIVE_PATH);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(IndexProtocolState::Uninitialized);
        }
        Err(e) => {
            return Err(ConstellationError::Io {
                detail: format!("cannot read {}: {e}", path.display()),
            });
        }
    };
    let marker: ProtocolMarker =
        serde_json::from_slice(&bytes).map_err(|e| ConstellationError::IndexProtocolMismatch {
            detail: format!("{} does not parse: {e}", path.display()),
        })?;
    marker.validate()?;
    Ok(match marker.state {
        BootstrapState::Active => IndexProtocolState::Active(marker),
        _ => IndexProtocolState::Initializing(marker),
    })
}

/// Write the bootstrap marker atomically and durably.
///
/// The marker is written **before** the new active tables are created, so a
/// crash during table creation is recognisable as "bootstrap was
/// interrupted" rather than "this index never existed".
///
/// Written through a same-directory temporary and `replace_atomically`, which
/// is the visibility point; the platform barrier then ACKs the containing
/// directory. A reader that sees the new marker sees a complete one.
/// `pub(crate)`, not `pub`: the durability module is private, so an
/// out-of-crate caller can neither name `PlatformDurability` nor obtain one
/// through the `pub(crate)` [`super::durability::platform_barrier`]. Leaving
/// this `pub` would advertise a signature no caller can satisfy.
pub(crate) fn write_marker(
    index_root: &Path,
    marker: &ProtocolMarker,
    platform: &dyn PlatformDurability,
) -> Result<(), ConstellationError> {
    marker.validate()?;
    let bytes = serde_json::to_vec_pretty(marker).map_err(|e| ConstellationError::Io {
        detail: format!("cannot serialise PROTOCOL.json: {e}"),
    })?;
    // The marker lives at `index_root/constellation/PROTOCOL.json` — the
    // same path `read_marker` reads via `PROTOCOL_RELATIVE_PATH`. The root
    // itself is already the index directory, so joining `constellation`
    // under it is the whole move; a `.parent()` hop here puts the marker a
    // directory too high and `read_marker` reports Uninitialized.
    let dir = index_root.join("constellation");
    write_bytes_via_temp(&dir, "PROTOCOL.json", &bytes, platform)?;
    // The dirent that must be acknowledged is the one inside `constellation/`
    // — the marker's own directory — not the index root above it.
    platform.sync_dir(&dir)?;
    Ok(())
}

/// Write one durable file: temp in the same directory, flush, then replace.
///
/// The temporary shares the destination's directory so the move is same-volume
/// — a cross-volume move would become a copy, and a copy is neither atomic
/// nor durable in the same way.
fn write_bytes_via_temp(
    dir: &Path,
    name: &str,
    bytes: &[u8],
    platform: &dyn PlatformDurability,
) -> Result<(), ConstellationError> {
    // The parent is created through the barrier, so a library
    // `create_dir_all` never becomes an unregistered fast path.
    platform.ensure_dir(dir)?;
    let temp = dir.join(format!(".{name}.tmp-{}", std::process::id()));
    let dest = dir.join(name);

    std::fs::write(&temp, bytes).map_err(|e| ConstellationError::Io {
        detail: format!("cannot write {}: {e}", temp.display()),
    })?;
    // The file's bytes must be durable BEFORE its name is visible: a rename
    // without a data fsync can survive power loss pointing at zeroed pages.
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

/// Read the visibility pointer, or `None` when it is absent.
///
/// A `CURRENT.json` that exists but does not parse is
/// `index_generation_unavailable`, never `None`. "Absent" and "corrupt" lead
/// to different actions: the first may be a first run, the second is an index
/// that has lost the ability to say what it published.
pub fn read_current(index_root: &Path) -> Result<Option<CurrentDescriptor>, ConstellationError> {
    let path = index_root.join(CURRENT_FILE_NAME);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(ConstellationError::Io {
                detail: format!("cannot read {}: {e}", path.display()),
            });
        }
    };
    serde_json::from_slice(&bytes).map(Some).map_err(|e| {
        ConstellationError::IndexGenerationUnavailable {
            detail: format!("{} does not parse: {e}", path.display()),
        }
    })
}

/// Load the generation manifest a descriptor names, verifying its checksum.
///
/// The checksum is what makes `CURRENT.json` meaningful: without it a reader
/// would follow a pointer to a manifest that has been truncated, half-written
/// or replaced, and would serve a generation that was never published.
pub fn read_manifest(
    index_root: &Path,
    descriptor: &CurrentDescriptor,
) -> Result<GenerationManifest, ConstellationError> {
    let path = index_root.join(&descriptor.manifest_path);
    let bytes =
        std::fs::read(&path).map_err(|e| ConstellationError::IndexGenerationUnavailable {
            detail: format!("manifest {} is missing: {e}", descriptor.manifest_path),
        })?;
    let actual = sha256_hex(&bytes);
    if actual != descriptor.manifest_sha256 {
        return Err(ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "manifest {} checksum {} does not match the descriptor's {}",
                descriptor.manifest_path, actual, descriptor.manifest_sha256
            ),
        });
    }
    serde_json::from_slice(&bytes).map_err(|e| ConstellationError::IndexGenerationUnavailable {
        detail: format!("manifest {} does not parse: {e}", descriptor.manifest_path),
    })
}

/// Write an **immutable** generation manifest and return its checksum.
///
/// Immutable means no replace: publishing the same `generationId` twice is a
/// hard error rather than a silent overwrite, so a reader that already holds
/// that generation id keeps reading the bytes it verified.
/// `pub(crate)` for the same reason as [`write_marker`]: a `&dyn
/// PlatformDurability` cannot be named or obtained outside this crate.
pub(crate) fn write_generation_manifest(
    index_root: &Path,
    manifest: &GenerationManifest,
    platform: &dyn PlatformDurability,
) -> Result<String, ConstellationError> {
    let dir = index_root.join(GENERATIONS_DIR);
    platform.ensure_dir(&dir)?;
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|e| ConstellationError::Io {
        detail: format!("cannot serialise generation manifest: {e}"),
    })?;

    let name = format!("{}.json", manifest.generation_id);
    let dest = dir.join(&name);
    if dest.exists() {
        return Err(ConstellationError::IndexProtocolMismatch {
            detail: format!(
                "generation `{}` already exists; an immutable generation must not be republished",
                manifest.generation_id
            ),
        });
    }
    // Install through the platform's immutable-file path: NTFS uses the
    // write-through create-new + write-through move sequence, POSIX uses the
    // equivalent rename. Neither replaces, so a reader that already verified
    // this generation keeps reading the bytes it verified.
    super::durability::install_immutable_file(&dir, &name, &bytes, platform)?;
    platform.sync_dir(&dir)?;
    Ok(sha256_hex(&bytes))
}

/// The descriptor that publishes `manifest`.
///
/// `manifest_sha256` is computed from the bytes that were actually written,
/// not from a re-serialisation: re-serialising could produce different bytes
/// and a checksum that matches nothing on disk.
pub fn descriptor_for(
    manifest: &GenerationManifest,
    owner_epoch: u64,
    manifest_sha256: &str,
) -> CurrentDescriptor {
    CurrentDescriptor::for_manifest(
        manifest,
        owner_epoch,
        format!("{GENERATIONS_DIR}/{}.json", manifest.generation_id),
        manifest_sha256,
    )
}

/// Publish a descriptor by replacing `CURRENT.json`.
///
/// The rename is the **visibility point**; the following `sync_dir` is the
/// durable ACK, and they are separate facts. A failure *after* the rename is
/// `publication_uncertain` — the new pointer is visible and nobody may claim
/// it is durable — never a rollback and never a success.
/// `pub(crate)` for the same reason as [`write_marker`].
pub(crate) fn publish_current(
    index_root: &Path,
    descriptor: &CurrentDescriptor,
    platform: &dyn PlatformDurability,
) -> Result<(), ConstellationError> {
    let bytes = serde_json::to_vec_pretty(descriptor).map_err(|e| ConstellationError::Io {
        detail: format!("cannot serialise CURRENT.json: {e}"),
    })?;
    platform.ensure_dir(index_root)?;

    let temp = index_root.join(format!(".{CURRENT_FILE_NAME}.tmp-{}", std::process::id()));
    let dest = index_root.join(CURRENT_FILE_NAME);
    std::fs::write(&temp, &bytes).map_err(|e| ConstellationError::Io {
        detail: format!("cannot write {}: {e}", temp.display()),
    })?;
    // Data must be durable before the pointer is visible; a rename that
    // outlives its contents is the hole `publication_uncertain` exists to
    // name. A failed fsync here is a plain error — nothing is visible yet.
    if let Err(e) = platform.sync_file(&temp) {
        let _ = std::fs::remove_file(&temp);
        return Err(e);
    }

    // Visibility point.
    platform.replace_atomically(&temp, &dest)?;

    // Durable ACK. Reaching this point without the rename having returned
    // success is the definition of `publication_uncertain`.
    match platform.sync_dir(index_root) {
        Ok(()) => Ok(()),
        Err(e) => Err(ConstellationError::PublicationUncertain {
            detail: format!(
                "CURRENT.json is visible but its directory entry was not acknowledged: {e}. \
                 The new generation must be treated as neither confirmed nor rolled back."
            ),
        }),
    }
}

/// A tracking store plus the tracker it writes into.
///
/// The inner store is the caller's: the publisher supplies the one the object
/// store registry built for this URI, and both tables' wrappers land in one
/// shared closure through it. Hardcoding an inner store here would make the
/// tracker observe a store nothing actually writes to.
pub fn tracking_pair_with(
    inner: Arc<dyn object_store::ObjectStore>,
) -> (Arc<DurabilityTrackingStore>, Arc<StageTracker>) {
    let tracker = Arc::new(StageTracker::default());
    let store = DurabilityTrackingStore::new(
        inner,
        Arc::clone(&tracker),
        STORAGE_SCHEME_FILE_OBJECT_STORE,
    );
    (Arc::new(store), tracker)
}

/// The same pairing, from a tracker the caller already owns.
///
/// Used by the publisher, which opens its stage **before** the store exists so
/// that the stage's epoch is the owner epoch. Building a fresh tracker here
/// instead would give the store and the publisher two trackers, and the
/// closure barrier would then see half the writes — the exact defect
/// [`tracking_pair_with`] avoids by construction.
pub(crate) fn tracking_pair_with_pair(
    inner: Arc<dyn object_store::ObjectStore>,
    tracker: Arc<StageTracker>,
) -> Arc<DurabilityTrackingStore> {
    Arc::new(DurabilityTrackingStore::new(
        inner,
        tracker,
        STORAGE_SCHEME_FILE_OBJECT_STORE,
    ))
}

/// Retention policy for S1: nothing is deleted, ever, automatically.
///
/// Automatic vacuum, `cleanup_old_versions` and generation-file deletion stay
/// off. A pinned reader holds an exact version; deleting a version out from
/// under it would turn a correct answer into `index_generation_unavailable`
/// for a reason that has nothing to do with the question being asked.
///
/// Disk full is `storage_full`: it blocks new commits and never removes
/// canonical markdown or the last completed view.
pub const AUTOMATIC_RETENTION_ENABLED: bool = false;

/// Guard against a caller that assumes deletion happens for it.
pub fn assert_retention_policy() -> Result<(), ConstellationError> {
    if AUTOMATIC_RETENTION_ENABLED {
        return Err(ConstellationError::IndexProtocolMismatch {
            detail: "automatic retention was enabled without a reviewed policy".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tracking_tests {
    use super::*;

    #[test]
    fn a_pair_built_from_one_tracker_shares_that_tracker() {
        let tracker = Arc::new(StageTracker::default());
        let store = tracking_pair_with_pair(
            Arc::new(object_store::memory::InMemory::new()),
            Arc::clone(&tracker),
        );
        assert!(
            Arc::ptr_eq(store.tracker(), &tracker),
            "the store and the publisher must share one StageTracker, or each \
             closure barrier sees half the writes"
        );
    }

    #[test]
    fn wrapping_a_shared_store_yields_two_wrappers_over_one_tracker() {
        use lance_io::object_store::WrappingObjectStore;

        let tracker = Arc::new(StageTracker::default());
        let store = tracking_pair_with_pair(
            Arc::new(object_store::memory::InMemory::new()),
            Arc::clone(&tracker),
        );
        let inner: Arc<dyn object_store::ObjectStore> =
            Arc::new(object_store::memory::InMemory::new());
        let first = store.wrap("file-object-store", Arc::clone(&inner));
        let second = store.wrap("file-object-store", inner);

        // Two distinct wrapper instances...
        assert!(!Arc::ptr_eq(&first, &second));
        // ...and the tracker is still one shared object behind them.
        assert_eq!(
            Arc::strong_count(&tracker),
            4,
            "the owner's handle, the store, and one clone per wrapper — one \
             tracker, never one tracker per wrapper"
        );
    }

    #[test]
    fn a_pair_built_by_value_owns_its_own_tracker() {
        let (store, tracker) = tracking_pair_with(Arc::new(object_store::memory::InMemory::new()));
        assert!(Arc::ptr_eq(store.tracker(), &tracker));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::manifest::BootstrapState;
    use tempfile::TempDir;

    fn marker() -> ProtocolMarker {
        ProtocolMarker {
            protocol_version: crate::constellation::FORMAT_VERSION,
            state: BootstrapState::Active,
            index_epoch: 3,
            checkout_id: "local-test".to_string(),
            table_names: ProtocolMarker::active_table_names(),
            storage_scheme: STORAGE_SCHEME_FILE_OBJECT_STORE.to_string(),
            bootstrap_attempt: 1,
        }
    }

    #[test]
    fn active_uris_carry_the_wrapped_scheme() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".forgeplan").join("lance");
        for role in ["artifacts", "relations"] {
            let uri = table_uri_for_role(&marker(), role, &root).unwrap();
            assert!(uri.starts_with("file-object-store:///"), "{uri}");
            assert!(
                uri.ends_with(DATASET_SUFFIX),
                "a dataset URI names the .lance directory: {uri}"
            );
        }
    }

    #[test]
    fn the_two_roles_get_different_datasets() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let a = table_uri_for_role(&marker(), "artifacts", root).unwrap();
        let r = table_uri_for_role(&marker(), "relations", root).unwrap();
        assert_ne!(a, r);
        assert!(a.contains(TABLE_ARTIFACTS_V1));
        assert!(r.contains(TABLE_RELATIONS_V1));
    }

    #[test]
    fn a_uri_round_trips_back_to_its_dataset_path() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".forgeplan").join("lance");
        let uri = table_uri_for_role(&marker(), "artifacts", &root).unwrap();
        assert_eq!(
            physical_root(&uri).unwrap(),
            root.join(format!("{TABLE_ARTIFACTS_V1}{DATASET_SUFFIX}"))
        );
        assert_eq!(index_root_of(&uri).unwrap(), root);
    }

    #[test]
    fn a_space_in_the_path_survives_the_round_trip() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("My Index");
        let uri = build_table_uri(&root, TABLE_ARTIFACTS_V1);
        assert!(uri.contains("My Index"), "spaces stay literal: {uri}");
        assert_eq!(
            physical_root(&uri).unwrap(),
            root.join(format!("{TABLE_ARTIFACTS_V1}{DATASET_SUFFIX}"))
        );
    }

    #[test]
    fn percent_and_hash_are_encoded_and_decoded() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("weird%dir#name");
        let uri = build_table_uri(&root, TABLE_ARTIFACTS_V1);
        assert!(uri.contains("%25"), "the percent itself is encoded: {uri}");
        assert!(uri.contains("%23"), "the hash is encoded: {uri}");
        assert_eq!(
            physical_root(&uri).unwrap(),
            root.join(format!("{TABLE_ARTIFACTS_V1}{DATASET_SUFFIX}"))
        );
    }

    #[test]
    fn a_windows_drive_keeps_its_letter_in_the_path() {
        let uri = "file-object-store:///C:/ws/.forgeplan/lance/constellation_artifacts_v1.lance";
        assert_eq!(
            physical_root(uri).unwrap(),
            PathBuf::from("/C:/ws/.forgeplan/lance/constellation_artifacts_v1.lance"),
            "the drive letter is part of the decoded path component"
        );
    }

    #[test]
    fn the_database_uri_carries_the_same_scheme_as_the_tables() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".forgeplan").join("lance");
        let db = database_uri(&root);
        assert!(db.starts_with("file-object-store:///"));
        assert_eq!(physical_root(&db).unwrap(), root);
    }

    #[test]
    fn plain_paths_and_file_scheme_are_refused_for_writes() {
        for uri in ["file:///tmp/x.lance", "/tmp/x.lance", "s3://bucket/x.lance"] {
            let err = assert_not_wrapped_route(uri).unwrap_err();
            assert_eq!(
                err.code(),
                crate::constellation::DiagnosticCode::IndexProtocolMismatch,
                "{uri} must not be accepted as a writer route"
            );
        }
        assert_not_wrapped_route("file-object-store:///tmp/x.lance").unwrap();
    }

    #[test]
    fn wrapped_route_detection_is_exact() {
        assert!(is_wrapped_route("file-object-store:///a"));
        assert!(!is_wrapped_route("file:///a"));
        assert!(!is_wrapped_route("/a"));
        assert!(!is_wrapped_route("file-object-storex:///a"));
    }

    #[test]
    fn a_marker_that_does_not_name_the_role_is_a_protocol_mismatch() {
        let tmp = TempDir::new().unwrap();
        let mut m = marker();
        m.table_names = vec![TABLE_ARTIFACTS_V1.to_string()];
        let err = table_uri_for_role(&m, "relations", tmp.path()).unwrap_err();
        assert_eq!(
            err.code(),
            crate::constellation::DiagnosticCode::IndexProtocolMismatch
        );
    }

    #[test]
    fn a_marker_with_the_wrong_scheme_is_refused_before_a_uri_is_built() {
        let tmp = TempDir::new().unwrap();
        let mut m = marker();
        m.storage_scheme = "file".to_string();
        let err = table_uri_for_role(&m, "artifacts", tmp.path()).unwrap_err();
        assert!(
            err.to_string().contains("storage_scheme"),
            "the marker must be rejected, not worked around: {err}"
        );
    }

    #[test]
    fn a_non_wrapped_uri_cannot_be_mapped_back() {
        assert!(physical_root("file:///tmp/x.lance").is_err());
        assert!(physical_root("/tmp/x.lance").is_err());
    }
}
