//! Stage 8 — the generation index, kernel-pinned readers, and manual GC.
//!
//! A published generation is a *manifest file* (`generations/<id>.json`)
//! plus the exact Lance table versions it pins inside the shared tables —
//! generations are not directories. "Growing forever" therefore has two
//! parts: manifest files accumulate, and every committed version stays
//! inside `*.lance/` forever. This module makes deletion *safe* instead
//! of forbidden:
//!
//! * **Generation index** — every manifest listed with its pinned
//!   versions, and who protects it (`CURRENT` or a live reader).
//! * **Reader pinning** — a pinned read takes a **shared OS lock on the
//!   generation manifest file itself** ([`super::storage::pin_generation_read`])
//!   and holds it for the whole read. There is no lease file, no heartbeat
//!   and no TTL: the kernel releases the lock when the last descriptor
//!   closes, so a crashed reader cannot leave a stale protection behind and
//!   a live reader is never starved by one.
//! * **Manual GC** — deletes only manifests that are not CURRENT, not
//!   kernel-pinned, and outside the newest `keep`; then prunes physical
//!   table versions, keeping everything a protected manifest pins. GC
//!   proves "not pinned" the only way that means anything — by taking the
//!   **exclusive** lock on the same file — and it holds those handles
//!   until physical pruning has finished.
//!
//! Automatic retention stays off (`AUTOMATIC_RETENTION_ENABLED`): GC is a
//! verb the operator runs (`forgeplan index --gc <keep>`), not a surprise.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;

use super::ConstellationError;
use super::manifest::{GenerationManifest, ProtocolMarker};
use super::storage::{GENERATIONS_DIR, read_current};

/// Conservative recency window for physical version pruning.
///
/// This is **not** what makes pruning safe: safety comes from the exclusive
/// manifest locks GC holds through pruning and from the floor computed over
/// every protected manifest. The window is kept as an extra cushion — it can
/// only make GC keep *more* than strictly necessary, and it costs disk, not
/// correctness.
pub const RECENCY_GUARD_MS: u64 = 30 * 60 * 1000;

/// The marker reported for a generation a live reader has pinned.
///
/// There is no pid and no lease id to report: the kernel knows a lock is
/// held and nothing else. Naming an owner we cannot actually observe would be
/// the invented-PID failure mode again, one layer down.
pub const OS_SHARED_LOCK_PIN: &str = "os-shared-lock";

/// How long GC waits for the owner lock. A busy index is `IndexBusy`,
/// reported — never stolen.
const GC_OWNER_BUDGET: Duration = Duration::from_secs(5);

/// One line of the generation index.
#[derive(Debug, Clone, Serialize)]
pub struct GenerationEntry {
    pub generation_id: String,
    /// Manifest parsed — an unparseable file reports `false` and GC treats
    /// it as *protected* (unknown state never means free).
    pub manifest_ok: bool,
    pub manifest_bytes: u64,
    /// Table versions this generation pins, when known.
    pub artifacts_version: Option<u64>,
    pub relations_version: Option<u64>,
    /// The sections version. `None` only when the manifest did not parse:
    /// every parseable manifest now pins a complete triple, so "no sections"
    /// is expressed as a real version plus an empty membership, not as an
    /// absent field. An unparseable manifest still enters no GC floor, which
    /// errs toward keeping versions.
    pub sections_version: Option<u64>,
    pub evaluated_at: Option<String>,
    pub is_current: bool,
    /// Who pins this generation right now. A kernel-pinned generation
    /// reports exactly `[OS_SHARED_LOCK_PIN]` — the lock has no owner we can
    /// name, and an empty list means only that no *shared* lock was held at
    /// the moment this index was built.
    pub leased_by: Vec<String>,
}

/// The whole index report.
#[derive(Debug, Serialize)]
pub struct GenerationIndex {
    pub current: Option<String>,
    pub generations: Vec<GenerationEntry>,
    /// Whole `lance/` footprint — manifests are small; the table version
    /// history is where disk actually goes.
    pub lance_bytes: u64,
}

/// What a non-blocking exclusive lock on one manifest proves.
#[derive(Debug)]
enum ManifestLock {
    /// The lock was taken; no shared reader holds this generation.
    Acquired(std::fs::File),
    /// Somebody holds a shared lock — a live pinned reader.
    Busy,
    /// The manifest could not be opened or locked for a reason that is not
    /// contention. Unknown state is never "free".
    Unusable(String),
}

/// Take the **exclusive** lock on `generations/<id>.json`, non-blocking.
///
/// This is the only test that means "no live reader is reading this". An
/// `mtime`, a lease file and a directory listing all describe what a writer
/// last did; only the kernel knows what a reader holds right now.
///
/// The returned [`std::fs::File`] carries the lock and must be kept alive for
/// as long as the caller relies on "unpinned" — closing it releases it.
/// `flock`/`LockFileEx` are per open file description, so this is a separate
/// descriptor on purpose and is never `try_clone`d.
fn lock_manifest_exclusive(index_root: &Path, generation_id: &str) -> ManifestLock {
    let path = index_root
        .join(GENERATIONS_DIR)
        .join(format!("{generation_id}.json"));
    let file = match std::fs::OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .truncate(false)
        .open(&path)
    {
        Ok(file) => file,
        Err(e) => return ManifestLock::Unusable(format!("cannot open {}: {e}", path.display())),
    };
    // Fully qualified, for the same reason as `pin_generation_read`: the
    // primitive must be the one the protocol names, not whichever inherent
    // method the toolchain happens to offer.
    match fs2::FileExt::try_lock_exclusive(&file) {
        Ok(()) => ManifestLock::Acquired(file),
        Err(e) if super::owner::is_contended(&e) => ManifestLock::Busy,
        Err(e) => ManifestLock::Unusable(format!("cannot lock {}: {e}", path.display())),
    }
}

fn dir_bytes(dir: &Path) -> u64 {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum()
}

/// Enumerate `generations/*.json` with pinned versions and protection.
///
/// Read-only by contract: the pin probe below opens each manifest without
/// `create`/`write` and releases the lock again before returning, so building
/// this report cannot add, change or lock anything a reader could notice.
pub fn generation_index(index_root: &Path) -> Result<GenerationIndex, ConstellationError> {
    let current = read_current(index_root)?.map(|d| d.generation_id);
    let dir = index_root.join(GENERATIONS_DIR);
    let mut generations = Vec::new();
    // A directory that cannot be enumerated is **not** an empty directory.
    //
    // It used to read as one, and that is the worst shape this function can
    // take: GC would see no generations, conclude that nothing is protected,
    // and — with an empty floor — report "no protected generation pins a table
    // version" while a reader was holding one. The refusal would look exactly
    // like a clean run. Enumerating is how GC learns what exists; failing to
    // enumerate means GC does not know, and not-knowing must stop the pass.
    let entries = std::fs::read_dir(&dir).map_err(|e| ConstellationError::Io {
        detail: format!("cannot enumerate {}: {e}", dir.display()),
    })?;
    for entry in entries {
        // A single unreadable directory entry is the same fact: this pass does
        // not know the whole set, so it must not pretend to.
        let entry = entry.map_err(|e| ConstellationError::Io {
            detail: format!("cannot read an entry of {}: {e}", dir.display()),
        })?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let generation_id = entry
            .file_name()
            .to_string_lossy()
            .trim_end_matches(".json")
            .to_string();
        // Unreadable bytes become an entry that reports itself unreadable, and
        // an unreadable entry is protected — never dropped from the list, since
        // dropping it would delete the only evidence that it exists.
        let bytes = std::fs::read(&path).unwrap_or_default();
        let manifest = serde_json::from_slice::<GenerationManifest>(&bytes).ok();
        // Dropped immediately: this report only asks "is a reader holding
        // this generation right now". GC re-takes the lock and keeps it.
        let pinned = matches!(
            lock_manifest_exclusive(index_root, &generation_id),
            ManifestLock::Busy
        );
        generations.push(GenerationEntry {
            is_current: current.as_deref() == Some(generation_id.as_str()),
            generation_id,
            manifest_ok: manifest.is_some(),
            manifest_bytes: bytes.len() as u64,
            artifacts_version: manifest.as_ref().map(|m| m.table_versions.artifacts),
            relations_version: manifest.as_ref().map(|m| m.table_versions.relations),
            sections_version: manifest.as_ref().map(|m| m.table_versions.sections),
            evaluated_at: manifest.as_ref().map(|m| m.evaluated_at.clone()),
            leased_by: if pinned {
                vec![OS_SHARED_LOCK_PIN.to_string()]
            } else {
                Vec::new()
            },
        });
    }
    generations.sort_by(|a, b| a.generation_id.cmp(&b.generation_id));
    Ok(GenerationIndex {
        current,
        generations,
        lance_bytes: dir_bytes(index_root),
    })
}

/// What GC decided, line by line.
#[derive(Debug, Serialize)]
pub struct GcReport {
    /// Generation manifests physically removed.
    pub removed_manifests: Vec<String>,
    /// (generation_id, why it stayed) — `current` /
    /// `pinned:os-shared-lock` / `within_keep` / `manifest_unreadable` /
    /// `manifest_unreadable: <why>`.
    pub protected: Vec<(String, String)>,
    /// Per table: versions pruned and bytes freed (lance RemovalStats).
    pub pruned: Vec<PruneOutcome>,
    /// Why no physical version was pruned at all, when one exists. `None`
    /// means pruning ran (possibly with nothing old enough to delete, which
    /// each [`PruneOutcome`] reports honestly).
    pub physical_prune_skipped: Option<String>,
    pub keep: usize,
}

#[derive(Debug, Serialize)]
pub struct PruneOutcome {
    pub table: String,
    /// Versions deleted (0 is honest — "nothing older than the floor").
    pub versions_removed: u64,
    pub bytes_removed: u64,
    /// The cutoff timestamp: versions older than this were eligible.
    pub kept_after: String,
    /// Why pruning this table did not run — e.g. the table does not exist
    /// under this root. A skip is named, never a silent zero.
    pub skipped: Option<String>,
}

/// Manual GC: remove manifests that are neither CURRENT, kernel-pinned, nor
/// in the newest `keep`, then prune physical versions older than every
/// still-needed floor.
///
/// Takes the owner lock — a mid-publish manifest or a publisher restoring
/// a table must not race a delete; `IndexBusy` is reported, never stolen.
///
/// **Order matters and is part of the contract.** Every manifest GC unlinks is
/// first locked *exclusively*, and those handles are held until physical
/// pruning has finished. A reader that opened the file before the unlink keeps
/// its shared lock and its already-read bytes; a reader that arrives after the
/// unlink cannot open the file by name at all, so it cannot pin a generation
/// whose versions GC is about to delete.
pub async fn gc_generations(
    index_root: &Path,
    keep: usize,
) -> Result<GcReport, ConstellationError> {
    let location = super::owner::IndexLocation::new(index_root);
    let _owner = super::owner::acquire_owner(&location, GC_OWNER_BUDGET)?;

    let index = generation_index(index_root)?;
    let mut ordered: Vec<&GenerationEntry> = index.generations.iter().collect();
    // Newest first: `gen-<rfc3339>` ids sort lexicographically as time.
    ordered.sort_by(|a, b| b.generation_id.cmp(&a.generation_id));

    let mut removed_manifests = Vec::new();
    let mut protected = Vec::new();
    let mut protected_ids: std::collections::HashSet<String> = Default::default();
    let mut kept = 0usize;
    // Exclusive handles for every manifest this pass removes. Alive until the
    // end of this function — that is what makes the unlink-then-prune order
    // safe, and dropping them earlier would reopen the window this closes.
    let mut removed_locks: Vec<std::fs::File> = Vec::new();
    for entry in ordered.iter() {
        let why = if entry.is_current {
            Some("current".to_string())
        } else if !entry.manifest_ok {
            Some("manifest_unreadable".to_string())
        } else if !entry.leased_by.is_empty() {
            Some(format!("pinned:{}", entry.leased_by.join(",")))
        } else if kept < keep {
            kept += 1;
            Some("within_keep".to_string())
        } else {
            None
        };
        match why {
            Some(reason) => {
                protected_ids.insert(entry.generation_id.clone());
                if !entry.leased_by.is_empty() {
                    let pinned = format!("pinned:{}", entry.leased_by.join(","));
                    if reason != pinned {
                        protected.push((entry.generation_id.clone(), pinned));
                    }
                }
                protected.push((entry.generation_id.clone(), reason));
            }
            None => match lock_manifest_exclusive(index_root, &entry.generation_id) {
                ManifestLock::Acquired(handle) => {
                    let file = index_root
                        .join(GENERATIONS_DIR)
                        .join(format!("{}.json", &entry.generation_id));
                    // A delete that the kernel refuses is not a failed GC, it
                    // is a pin: on Windows a reader opens the manifest without
                    // `FILE_SHARE_DELETE`, so `DeleteFile` fails with
                    // `ERROR_SHARING_VIOLATION` for as long as that read lives.
                    // Reporting the generation as protected is the truthful
                    // answer and keeps the pass going; turning it into an
                    // `Io` error would abort a prune that had already
                    // established every other candidate was safe to remove.
                    match std::fs::remove_file(&file) {
                        Ok(()) => {
                            removed_locks.push(handle);
                            removed_manifests.push(entry.generation_id.clone());
                        }
                        Err(e) => {
                            protected_ids.insert(entry.generation_id.clone());
                            protected
                                .push((entry.generation_id.clone(), format!("unlink_refused:{e}")));
                        }
                    }
                }
                ManifestLock::Busy => {
                    // A reader is in the manifest right now. Unknown-until-now
                    // is still protected.
                    protected_ids.insert(entry.generation_id.clone());
                    protected.push((
                        entry.generation_id.clone(),
                        format!("pinned:{OS_SHARED_LOCK_PIN}"),
                    ));
                }
                ManifestLock::Unusable(detail) => {
                    protected_ids.insert(entry.generation_id.clone());
                    protected.push((
                        entry.generation_id.clone(),
                        format!("manifest_unreadable: {detail}"),
                    ));
                }
            },
        }
    }

    // Physical prune: floor per table = the minimum version any protected
    // manifest still pins, *including* one a live reader currently holds.
    // A protected manifest we cannot read completely makes the whole floor
    // unknown, and an unknown floor is not a licence to guess: pruning is
    // skipped wholesale, with the reason named in the report.
    let mut floor: std::collections::BTreeMap<&'static str, u64> = Default::default();
    let mut floor_blocker: Option<String> = None;
    for entry in ordered
        .iter()
        .filter(|e| protected_ids.contains(&e.generation_id))
    {
        if !entry.manifest_ok {
            floor_blocker = Some(format!(
                "protected generation `{}` has an unreadable manifest",
                entry.generation_id
            ));
            break;
        }
        let (Some(artifacts), Some(relations)) = (entry.artifacts_version, entry.relations_version)
        else {
            floor_blocker = Some(format!(
                "protected generation `{}` does not name artifact and relation versions",
                entry.generation_id
            ));
            break;
        };
        floor
            .entry("artifacts")
            .and_modify(|v| *v = (*v).min(artifacts))
            .or_insert(artifacts);
        floor
            .entry("relations")
            .and_modify(|v| *v = (*v).min(relations))
            .or_insert(relations);
        // A generation that predates the sections table pins nothing for it —
        // `None` contributes no floor, exactly as an unparseable manifest
        // contributes none: unknown state is protected, never freed.
        if let Some(sections) = entry.sections_version {
            floor
                .entry("sections")
                .and_modify(|v| *v = (*v).min(sections))
                .or_insert(sections);
        }
    }

    let mut pruned = Vec::new();
    let physical_prune_skipped = match floor_blocker {
        Some(reason) => Some(reason),
        None if floor.is_empty() => {
            Some("no protected generation pins a table version".to_string())
        }
        None => {
            pruned = prune_tables(index_root, &floor).await?;
            None
        }
    };
    // `removed_locks` drops here, after pruning, releasing every exclusive
    // manifest lock this pass took.

    removed_manifests.sort();
    protected.sort();
    Ok(GcReport {
        removed_manifests,
        protected,
        pruned,
        physical_prune_skipped,
        keep,
    })
}

/// Open the active tables at head, list versions, and prune strictly
/// older than each table's floor — never younger than the recency window.
/// The `sections` role resolves through the same marker; an index that
/// predates the table simply reports "cannot open" and skips it.
async fn prune_tables(
    index_root: &Path,
    floor: &std::collections::BTreeMap<&'static str, u64>,
) -> Result<Vec<PruneOutcome>, ConstellationError> {
    // The marker mirrors `facade::marker_for` — pinned reads and writers
    // agree on one protocol shape; GC names the same tables by role.
    let descriptor = read_current(index_root)?.ok_or_else(|| {
        ConstellationError::IndexGenerationUnavailable {
            detail: "GC needs a published CURRENT to name the active tables".to_string(),
        }
    })?;
    // GC reads CURRENT's manifest through the same pin a reader uses: the
    // owner lock already excludes publishers, and this keeps "which
    // generation is this" a single verified read in one place rather than a
    // second, looser copy of the same logic.
    let pinned = super::storage::pin_generation_read(index_root, &descriptor)?;
    let manifest = pinned.manifest();
    let marker = ProtocolMarker {
        protocol_version: super::FORMAT_VERSION,
        state: super::manifest::BootstrapState::Active,
        index_epoch: manifest.index_epoch,
        checkout_id: manifest.checkout_id.clone(),
        table_names: ProtocolMarker::active_table_names(),
        storage_scheme: super::manifest::STORAGE_SCHEME_FILE_OBJECT_STORE.to_string(),
        bootstrap_attempt: 1,
    };
    let connection = super::lance_tables::open_wrapped_connection(index_root).await?;
    let database = Arc::clone(connection.database());

    // Extra cushion, not the safety argument — see [`RECENCY_GUARD_MS`].
    let recency_cutoff =
        chrono::Utc::now() - chrono::Duration::milliseconds(RECENCY_GUARD_MS as i64);

    let mut out = Vec::new();
    for (role, floor_version) in floor {
        let uri = super::storage::table_uri_for_role(&marker, role, index_root)?;
        let native = match lancedb::table::NativeTable::open_with_params(
            &uri,
            role,
            Vec::new(),
            None,
            None,
            None,
            None,
            false,
            None,
        )
        .await
        {
            Ok(t) => t,
            Err(e) => {
                // A manifest-only or torn root has no tables to prune —
                // report the skip; do not fabricate a byte count.
                out.push(PruneOutcome {
                    table: role.to_string(),
                    versions_removed: 0,
                    bytes_removed: 0,
                    kept_after: String::new(),
                    skipped: Some(format!("cannot open: {e}")),
                });
                continue;
            }
        };
        let table = lancedb::table::Table::new(Arc::new(native), Arc::clone(&database));
        let versions = match table.list_versions().await {
            Ok(v) => v,
            Err(e) => {
                out.push(PruneOutcome {
                    table: role.to_string(),
                    versions_removed: 0,
                    bytes_removed: 0,
                    kept_after: String::new(),
                    skipped: Some(format!("cannot list versions: {e}")),
                });
                continue;
            }
        };
        let Some(floor_ts) = versions
            .iter()
            .find(|v| v.version == *floor_version)
            .map(|v| v.timestamp)
        else {
            // The pinned floor version is already absent — don't guess a
            // new floor; leave this table untouched and say so.
            out.push(PruneOutcome {
                table: role.to_string(),
                versions_removed: 0,
                bytes_removed: 0,
                kept_after: String::new(),
                skipped: Some("pinned floor version missing".to_string()),
            });
            continue;
        };
        // cutoff = min(floor_ts, recency): delete versions strictly older.
        let cutoff = std::cmp::min(floor_ts, recency_cutoff);
        let older_than = chrono::Utc::now() - cutoff;
        let stats = table
            .optimize(lancedb::table::OptimizeAction::Prune {
                older_than: Some(older_than),
                delete_unverified: Some(false),
                error_if_tagged_old_versions: Some(false),
            })
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("GC prune of {role} failed: {e}"),
            })?;
        let prune = stats.prune.unwrap_or_default();
        out.push(PruneOutcome {
            table: role.to_string(),
            versions_removed: prune.old_versions,
            bytes_removed: prune.bytes_removed,
            kept_after: cutoff.to_rfc3339(),
            skipped: None,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::manifest::CurrentDescriptor;
    use crate::constellation::storage::CURRENT_FILE_NAME;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn manifest_json(id: &str, artifacts_v: u64, relations_v: u64) -> String {
        serde_json::json!({
            "schema_version": 1,
            "generation_id": id,
            "checkout_id": "ck",
            "index_epoch": 1,
            "source_set_digest": "d",
            "config_digest": "c",
            "table_versions": {"artifacts": artifacts_v, "relations": relations_v},
            "source_entries": [],
            "tombstones": [],
            "pending_jobs": {},
            "coverage": {"expected":0,"read":0,"invalid":0,"unavailable":0,"excluded":0},
            "diagnostics": [],
            "evaluated_at": "2026-01-01T00:00:00Z",
        })
        .to_string()
    }

    /// An index root with N flat generation manifests + CURRENT.
    ///
    /// No `readers/` directory is created — pinning is a kernel lock on the
    /// manifest, so a fixture that made one would be testing a protocol that
    /// no longer exists.
    fn index_root_with(ids: &[(&str, u64)], current: &str) -> TempDir {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("lance");
        std::fs::create_dir_all(root.join(GENERATIONS_DIR)).unwrap();
        for (id, v) in ids {
            std::fs::write(
                root.join(GENERATIONS_DIR).join(format!("{id}.json")),
                manifest_json(id, *v, *v),
            )
            .unwrap();
        }
        let cv = ids
            .iter()
            .find(|(id, _)| *id == current)
            .map(|(_, v)| *v)
            .unwrap_or(1);
        std::fs::write(
            root.join(CURRENT_FILE_NAME),
            serde_json::json!({
                "format_version": 1,
                "index_epoch": 1,
                "checkout_id": "ck",
                "generation_id": current,
                "owner_epoch": 1,
                "source_set_digest": "d",
                "config_digest": "c",
                "profile_id": null,
                "evaluated_at": "now",
                "manifest_path": format!("generations/{current}.json"),
                "manifest_sha256": crate::constellation::sha256_hex(
                    manifest_json(current, cv, cv).as_bytes()),
            })
            .to_string(),
        )
        .unwrap();
        tmp
    }

    /// A `CURRENT`-shaped descriptor for a generation that is **not**
    /// current — the only way to reproduce "a reader pinned an older
    /// generation" without publishing one.
    fn descriptor_for(generation_id: &str, v: u64) -> CurrentDescriptor {
        serde_json::from_value(serde_json::json!({
            "format_version": 1,
            "index_epoch": 1,
            "checkout_id": "ck",
            "generation_id": generation_id,
            "owner_epoch": 1,
            "source_set_digest": "d",
            "config_digest": "c",
            "profile_id": null,
            "evaluated_at": "now",
            "manifest_path": format!("generations/{generation_id}.json"),
            "manifest_sha256": crate::constellation::sha256_hex(
                manifest_json(generation_id, v, v).as_bytes()),
        }))
        .unwrap()
    }

    /// Every path under `dir` with its size — the "nothing was written"
    /// witness a read-only claim needs.
    fn tree_snapshot(dir: &Path) -> Vec<(PathBuf, u64)> {
        let mut out: Vec<(PathBuf, u64)> = walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| (e.path().to_path_buf(), e.metadata().unwrap().len()))
            .collect();
        out.sort();
        out
    }

    #[test]
    fn index_lists_generations_with_current_flag_and_versions() {
        let tmp = index_root_with(&[("gen-1", 2), ("gen-2", 3), ("gen-3", 4)], "gen-3");
        let index = generation_index(&tmp.path().join("lance")).unwrap();
        assert_eq!(index.generations.len(), 3);
        assert_eq!(index.current.as_deref(), Some("gen-3"));
        let g3 = index
            .generations
            .iter()
            .find(|g| g.generation_id == "gen-3")
            .unwrap();
        assert!(g3.is_current);
        assert_eq!(g3.artifacts_version, Some(4));
        assert!(index.lance_bytes > 0);
    }

    #[tokio::test]
    async fn gc_removes_old_manifests_keeps_current_and_newest() {
        let tmp = index_root_with(
            &[("gen-1", 2), ("gen-2", 3), ("gen-3", 4), ("gen-4", 5)],
            "gen-4",
        );
        let report = gc_generations(&tmp.path().join("lance"), 1).await.unwrap();
        assert_eq!(report.removed_manifests, vec!["gen-1", "gen-2"]);
        assert!(
            report
                .protected
                .iter()
                .any(|(g, w)| g == "gen-4" && w == "current")
        );
        assert!(
            report
                .protected
                .iter()
                .any(|(g, w)| g == "gen-3" && w == "within_keep")
        );
        assert!(!tmp.path().join("lance/generations/gen-1.json").exists());
        assert!(tmp.path().join("lance/generations/gen-3.json").exists());
        assert!(tmp.path().join("lance/generations/gen-4.json").exists());
    }

    /// The C06 core behaviour, in one test: a reader that holds the manifest's
    /// shared lock keeps its generation through a GC pass, and the very same
    /// generation becomes collectable the moment the reader is gone — with no
    /// TTL, no lease file and no time passing.
    #[tokio::test]
    async fn a_live_kernel_pin_protects_its_generation_until_the_reader_lets_go() {
        let tmp = index_root_with(&[("gen-1", 2), ("gen-2", 3)], "gen-2");
        let root = tmp.path().join("lance");
        let pin =
            super::super::storage::pin_generation_read(&root, &descriptor_for("gen-1", 2)).unwrap();
        assert_eq!(pin.generation_id(), "gen-1");
        // The protection left no file behind — no `readers/` directory, no
        // heartbeat, nothing that has to expire.
        assert!(!root.join("readers").exists());

        let report = gc_generations(&root, 0).await.unwrap();
        assert!(
            report.removed_manifests.is_empty(),
            "GC removed {:?} while a reader held the manifest",
            report.removed_manifests
        );
        assert_eq!(
            report.protected,
            vec![
                ("gen-1".to_string(), format!("pinned:{OS_SHARED_LOCK_PIN}")),
                ("gen-2".to_string(), "current".to_string()),
            ],
        );
        assert!(root.join("generations/gen-1.json").exists());

        drop(pin);
        let report = gc_generations(&root, 0).await.unwrap();
        assert_eq!(report.removed_manifests, vec!["gen-1"]);
        assert!(!root.join("generations/gen-1.json").exists());
    }

    /// A pin is a read. It must not create, rewrite or extend anything under
    /// the index root — the property a read-only mount would otherwise be the
    /// only way to observe.
    #[test]
    fn pinning_a_generation_writes_nothing_under_the_index_root() {
        let tmp = index_root_with(&[("gen-1", 2), ("gen-2", 3)], "gen-2");
        let root = tmp.path().join("lance");
        let before = tree_snapshot(&root);
        {
            let pin =
                super::super::storage::pin_generation_read(&root, &descriptor_for("gen-1", 2))
                    .unwrap();
            assert_eq!(pin.manifest().generation_id, "gen-1");
        }
        assert_eq!(before, tree_snapshot(&root));
    }

    /// An unreadable manifest is protected, and because the floor it would
    /// contribute is unknown, GC refuses to prune *any* version — naming why.
    #[tokio::test]
    async fn unreadable_protected_manifest_skips_physical_prune() {
        let tmp = index_root_with(&[("gen-1", 2), ("gen-2", 3)], "gen-2");
        let root = tmp.path().join("lance");
        std::fs::write(root.join("generations/gen-1.json"), "not json").unwrap();
        let report = gc_generations(&root, 0).await.unwrap();
        assert!(
            report
                .protected
                .iter()
                .any(|(g, w)| g == "gen-1" && w == "manifest_unreadable")
        );
        assert!(root.join("generations/gen-1.json").exists());
        assert!(report.pruned.is_empty());
        assert_eq!(
            report.physical_prune_skipped.as_deref(),
            Some("protected generation `gen-1` has an unreadable manifest")
        );
    }

    /// A generations directory that cannot be enumerated stops the pass.
    ///
    /// It used to read as an empty one, and that is the worst shape this can
    /// take. GC saw no generations, concluded nothing was protected, computed an
    /// empty floor, and reported "no protected generation pins a table
    /// version" — the same sentence a genuinely clean run produces, while a
    /// reader was holding a generation whose versions were about to be pruned.
    /// A refusal that is indistinguishable from success is worse than a crash.
    #[tokio::test]
    async fn an_unenumerable_generations_directory_stops_gc_instead_of_reading_as_empty() {
        let tmp = index_root_with(&[("gen-1", 2), ("gen-2", 3)], "gen-2");
        let root = tmp.path().join("lance");

        // Replace the directory with a regular file: the same `NotADirectory`
        // a permission problem or a detached volume produces, and reachable on
        // every platform this suite runs on.
        std::fs::remove_dir_all(root.join("generations")).unwrap();
        std::fs::write(root.join("generations"), "not a directory").unwrap();
        // Snapshot **after** the damage: what matters is that GC itself changes
        // nothing, not that the fixture is untouched.
        let before = tree_snapshot(&root);

        let err = gc_generations(&root, 0).await.unwrap_err();
        assert!(
            err.to_string().contains("enumerate"),
            "GC must say it could not enumerate, not report a clean pass: {err}"
        );
        assert_eq!(
            before,
            tree_snapshot(&root),
            "a refused pass must not have removed or created anything under the index root"
        );
    }

    /// The same refusal, at the level of a single generation that exists but
    /// cannot be listed — the `entries.flatten()` shape.
    ///
    /// Dropping an unreadable entry would drop the only evidence that the
    /// generation exists, and a generation nobody knows about is a generation
    /// GC is free to delete.
    #[test]
    fn generation_index_reports_a_directory_it_cannot_read_rather_than_an_empty_one() {
        let tmp = index_root_with(&[("gen-1", 2)], "gen-1");
        let root = tmp.path().join("lance");
        std::fs::remove_dir_all(root.join("generations")).unwrap();

        let err = generation_index(&root).unwrap_err();
        assert!(
            err.to_string().contains("enumerate"),
            "an absent generations directory is a fact GC must not read as 'nothing is here': {err}"
        );
    }
}
