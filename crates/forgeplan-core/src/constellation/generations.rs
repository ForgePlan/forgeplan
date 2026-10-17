//! Stage 8 — the generation index, pinned-reader leases, and manual GC.
//!
//! A published generation is a *manifest file* (`generations/<id>.json`)
//! plus the exact Lance table versions it pins inside the shared tables —
//! generations are not directories. "Growing forever" therefore has two
//! parts: manifest files accumulate, and every committed version stays
//! inside `*.lance/` forever. This module makes deletion *safe* instead
//! of forbidden:
//!
//! * **Generation index** — every manifest listed with its pinned
//!   versions, and who protects it (`CURRENT` or a live reader lease).
//! * **Reader leases** — a pinned read registers a lease for the
//!   generation it is about to open, for the call's duration. A crashed
//!   reader's lease ages out at [`LEASE_TTL_MS`].
//! * **Manual GC** — deletes only manifests that are not CURRENT, not
//!   leased, and outside the newest `keep`; then prunes physical table
//!   versions, keeping everything a protected manifest pins *and*
//!   everything younger than the lease TTL — the recency window that
//!   makes a mid-read delete impossible to begin with (a reader can only
//!   pin `CURRENT`, so its version is always recent relative to any GC
//!   pass that could see it unprotected).
//!
//! Automatic retention stays off (`AUTOMATIC_RETENTION_ENABLED`): GC is a
//! verb the operator runs (`forgeplan index --gc <keep>`), not a surprise.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::ConstellationError;
use super::manifest::{GenerationManifest, ProtocolMarker};
use super::storage::{GENERATIONS_DIR, read_current};

/// How long a lease survives its writer. A crashed read's lease protects
/// its generation for this long, then GC treats it as gone — never honored
/// forever. Physical version cleanup also never touches versions younger
/// than this: any version a reader could pin is `CURRENT`'s, hence fresh.
pub const LEASE_TTL_MS: u64 = 30 * 60 * 1000;

/// The readers' lease directory inside the index root.
pub const READERS_DIR: &str = "readers";

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
    pub evaluated_at: Option<String>,
    pub is_current: bool,
    /// Lease ids pinning this generation right now.
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

/// A lease file's payload.
#[derive(Debug, Serialize, Deserialize)]
struct LeaseRecord {
    generation_id: String,
    pid: u32,
    heartbeat_ms: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Live leases — parseable and fresh. Unparseable or aged-out files are
/// skipped, not fatal.
fn live_leases(index_root: &Path) -> Vec<(String, LeaseRecord)> {
    let dir = index_root.join(READERS_DIR);
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    let now = now_ms();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        let Ok(record) = serde_json::from_str::<LeaseRecord>(&text) else {
            continue;
        };
        if now.saturating_sub(record.heartbeat_ms) <= LEASE_TTL_MS {
            out.push((name, record));
        }
    }
    out
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
pub fn generation_index(index_root: &Path) -> Result<GenerationIndex, ConstellationError> {
    let current = read_current(index_root)?.map(|d| d.generation_id);
    let leases = live_leases(index_root);
    let dir = index_root.join(GENERATIONS_DIR);
    let mut generations = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let generation_id = entry
                .file_name()
                .to_string_lossy()
                .trim_end_matches(".json")
                .to_string();
            let bytes = std::fs::read(&path).unwrap_or_default();
            let manifest = serde_json::from_slice::<GenerationManifest>(&bytes).ok();
            let leased_by = leases
                .iter()
                .filter(|(_, l)| l.generation_id == generation_id)
                .map(|(name, _)| name.clone())
                .collect();
            generations.push(GenerationEntry {
                is_current: current.as_deref() == Some(generation_id.as_str()),
                generation_id,
                manifest_ok: manifest.is_some(),
                manifest_bytes: bytes.len() as u64,
                artifacts_version: manifest.as_ref().map(|m| m.table_versions.artifacts),
                relations_version: manifest.as_ref().map(|m| m.table_versions.relations),
                evaluated_at: manifest.as_ref().map(|m| m.evaluated_at.clone()),
                leased_by,
            });
        }
    }
    generations.sort_by(|a, b| a.generation_id.cmp(&b.generation_id));
    Ok(GenerationIndex {
        current,
        generations,
        lance_bytes: dir_bytes(index_root),
    })
}

/// A live reader's claim on one generation. Dropping removes the file;
/// crashing leaves a lease that ages out at [`LEASE_TTL_MS`].
pub struct ReaderLease {
    file: PathBuf,
}

impl ReaderLease {
    /// Register this process as a reader of `generation_id`.
    ///
    /// Acquire after the manifest names the generation and **before** the
    /// pinned tables open — the window GC must not delete in.
    pub fn acquire(index_root: &Path, generation_id: &str) -> Result<Self, ConstellationError> {
        let dir = index_root.join(READERS_DIR);
        std::fs::create_dir_all(&dir).map_err(|e| ConstellationError::Io {
            detail: format!("cannot create the readers dir {}: {e}", dir.display()),
        })?;
        let name = format!("{}-{}.lease", std::process::id(), now_ms());
        let file = dir.join(name);
        let record = LeaseRecord {
            generation_id: generation_id.to_string(),
            pid: std::process::id(),
            heartbeat_ms: now_ms(),
        };
        let body = serde_json::to_string(&record).map_err(|e| ConstellationError::Io {
            detail: format!("cannot serialize a reader lease: {e}"),
        })?;
        std::fs::write(&file, body).map_err(|e| ConstellationError::Io {
            detail: format!("cannot write reader lease {}: {e}", file.display()),
        })?;
        Ok(Self { file })
    }
}

impl Drop for ReaderLease {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.file);
    }
}

/// What GC decided, line by line.
#[derive(Debug, Serialize)]
pub struct GcReport {
    /// Generation manifests physically removed.
    pub removed_manifests: Vec<String>,
    /// (generation_id, why it stayed) — `current` / `leased:<id>` /
    /// `within_keep` / `manifest_unreadable`.
    pub protected: Vec<(String, String)>,
    /// Per table: versions pruned and bytes freed (lance RemovalStats).
    pub pruned: Vec<PruneOutcome>,
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

/// Manual GC: remove manifests that are neither CURRENT, leased, nor in
/// the newest `keep`, then prune physical versions older than every
/// still-needed floor.
///
/// Takes the owner lock — a mid-publish manifest or a publisher restoring
/// a table must not race a delete; `IndexBusy` is reported, never stolen.
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
    for entry in ordered.iter() {
        let why = if entry.is_current {
            Some("current".to_string())
        } else if !entry.leased_by.is_empty() {
            Some(format!("leased:{}", entry.leased_by.join(",")))
        } else if !entry.manifest_ok {
            Some("manifest_unreadable".to_string())
        } else if kept < keep {
            kept += 1;
            Some("within_keep".to_string())
        } else {
            None
        };
        match why {
            Some(reason) => {
                protected_ids.insert(entry.generation_id.clone());
                protected.push((entry.generation_id.clone(), reason));
            }
            None => {
                let file = index_root
                    .join(GENERATIONS_DIR)
                    .join(format!("{}.json", entry.generation_id));
                std::fs::remove_file(&file).map_err(|e| ConstellationError::Io {
                    detail: format!("cannot remove manifest {}: {e}", file.display()),
                })?;
                removed_manifests.push(entry.generation_id.clone());
            }
        }
    }

    // Physical prune: floor per table = the minimum version any protected
    // manifest still pins. Cutoff is also clamped to `now - LEASE_TTL`:
    // versions younger than the lease window are always kept — that closes
    // the race where a lease lands between our scan and the delete call.
    let mut floor: std::collections::BTreeMap<&'static str, u64> = Default::default();
    for entry in ordered
        .iter()
        .filter(|e| protected_ids.contains(&e.generation_id))
    {
        if let (Some(a), Some(r)) = (entry.artifacts_version, entry.relations_version) {
            floor
                .entry("artifacts")
                .and_modify(|v| *v = (*v).min(a))
                .or_insert(a);
            floor
                .entry("relations")
                .and_modify(|v| *v = (*v).min(r))
                .or_insert(r);
        }
    }
    let mut pruned = Vec::new();
    if !floor.is_empty() {
        pruned = prune_tables(index_root, &floor).await?;
    }

    removed_manifests.sort();
    protected.sort();
    Ok(GcReport {
        removed_manifests,
        protected,
        pruned,
        keep,
    })
}

/// Open both active tables at head, list versions, and prune strictly
/// older than each table's floor — never younger than the recency window.
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
    let manifest = super::storage::read_manifest(index_root, &descriptor)?;
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

    // Recency guard: never touch versions younger than the lease TTL.
    let recency_cutoff = chrono::Utc::now() - chrono::Duration::milliseconds(LEASE_TTL_MS as i64);

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
    use crate::constellation::storage::CURRENT_FILE_NAME;
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
    fn index_root_with(ids: &[(&str, u64)], current: &str) -> TempDir {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("lance");
        std::fs::create_dir_all(root.join(GENERATIONS_DIR)).unwrap();
        std::fs::create_dir_all(root.join(READERS_DIR)).unwrap();
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

    #[tokio::test]
    async fn live_lease_protects_its_generation_manifest() {
        let tmp = index_root_with(&[("gen-1", 2), ("gen-2", 3)], "gen-2");
        let root = tmp.path().join("lance");
        let _lease = ReaderLease::acquire(&root, "gen-1").unwrap();
        let report = gc_generations(&root, 0).await.unwrap();
        assert!(
            report.removed_manifests.is_empty(),
            "{:?}",
            report.removed_manifests
        );
        assert!(
            report
                .protected
                .iter()
                .any(|(g, w)| g == "gen-1" && w.starts_with("leased:"))
        );
        assert!(root.join("generations/gen-1.json").exists());
    }

    #[tokio::test]
    async fn dropped_and_stale_leases_free_their_generation() {
        let tmp = index_root_with(&[("gen-1", 2), ("gen-2", 3)], "gen-2");
        let root = tmp.path().join("lance");
        {
            let _lease = ReaderLease::acquire(&root, "gen-1").unwrap();
        }
        std::fs::write(
            root.join(READERS_DIR).join("ancient.lease"),
            "{\"generation_id\":\"gen-1\",\"pid\":1,\"heartbeat_ms\":1}",
        )
        .unwrap();
        let report = gc_generations(&root, 0).await.unwrap();
        assert_eq!(report.removed_manifests, vec!["gen-1"]);
    }

    #[tokio::test]
    async fn unreadable_manifest_is_protected_not_deleted() {
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
    }
}
