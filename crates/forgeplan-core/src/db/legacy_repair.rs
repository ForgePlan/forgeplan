//! Owner-scoped, recoverable publication of a repaired legacy index.
//!
//! The identity-repair journal remains the reader/writer barrier throughout
//! staging and publication. This module never obtains an ordinary store permit.

#[cfg(not(windows))]
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::artifact::identity_repair::{self, JournalStatus, ValidatedLegacyRepair};
use crate::db::{legacy_repair_rows, migrate};

const NAMES: [&str; 3] = [
    migrate::IDENTITY_V5_ARTIFACTS_TABLE,
    migrate::IDENTITY_V5_RELATIONS_TABLE,
    migrate::IDENTITY_V5_EVIDENCE_TABLE,
];

#[cfg(test)]
static REPAIR_FAILPOINTS: std::sync::LazyLock<std::sync::Mutex<std::collections::BTreeMap<PathBuf, &'static str>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::BTreeMap::new()));

#[cfg(test)]
pub(crate) fn fail_repair_once(workspace: &Path, point: &'static str) {
    REPAIR_FAILPOINTS.lock().unwrap().insert(workspace.to_path_buf(), point);
}

#[cfg(test)]
pub(crate) fn repair_stage_path_for_test(workspace: &Path, journal_id: &str) -> Result<PathBuf> {
    let dir = current_plan_directory(workspace, journal_id)?;
    let plan: RepairIndexPlan = serde_json::from_slice(&read_regular(&dir.join("plan.json"))?)?;
    if !plan.stage_name.strip_prefix("stage-").is_some_and(|suffix|
        suffix.len() == 64 && suffix.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
    {
        bail!("invalid repair stage name");
    }
    Ok(dir.join(plan.stage_name))
}

#[cfg(test)]
fn failpoint(workspace: &Path, point: &str) -> Result<()> {
    let mut pending = REPAIR_FAILPOINTS.lock().unwrap();
    if pending.get(workspace).copied() == Some(point) {
        pending.remove(workspace);
        bail!("injected repair interruption at {point}");
    }
    Ok(())
}

#[cfg(not(test))]
fn failpoint(_workspace: &Path, _point: &str) -> Result<()> { Ok(()) }

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TablePlan {
    name: String,
    before_digest: Option<String>,
    after_digest: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepairIndexPlan {
    version: u32,
    workspace: PathBuf,
    journal_id: String,
    journal_digest: String,
    inventory_digest: String,
    source_digest: String,
    operation_time: String,
    stage_name: String,
    old_marker: Option<Vec<u8>>,
    original_table_digests: [String; 3],
    new_marker: migrate::IdentityV5Marker,
    tables: Vec<TablePlan>,
    digest: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepairHeader {
    version: u32,
    workspace: PathBuf,
    journal_id: String,
    journal_digest: String,
    inventory_digest: String,
    operation_time: String,
}


fn plan_digest(plan: &RepairIndexPlan) -> Result<String> {
    let mut copy = plan.clone();
    copy.digest.clear();
    Ok(identity_repair::sha256_hex(&serde_json::to_vec(&copy)?))
}
fn random_component(prefix: &str) -> Result<String> {
    use rand::TryRngCore;
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.try_fill_bytes(&mut bytes)
        .map_err(|error| anyhow::anyhow!("cannot generate repair staging name: {error}"))?;
    Ok(format!("{prefix}{}", identity_repair::sha256_hex(&bytes)))
}


fn plan_directory(workspace: &Path, journal_id: &str) -> PathBuf {
    // Journal IDs are data, never filesystem components.
    workspace.join("lance").join(format!(
        "legacy-repair-{}", identity_repair::sha256_hex(journal_id.as_bytes())
    ))
}

fn revision_directory(workspace: &Path, journal_id: &str, journal_digest: &str, inventory_digest: &str) -> PathBuf {
    let revision = identity_repair::sha256_hex(format!("{journal_digest}:{inventory_digest}").as_bytes());
    workspace.join("lance").join(format!(
        "legacy-repair-{}-{revision}", identity_repair::sha256_hex(journal_id.as_bytes())
    ))
}

fn select_plan_directory(
    workspace: &Path, journal_id: &str, journal_digest: &str, inventory_digest: &str,
    status: JournalStatus,
) -> Result<PathBuf> {
    // Continue a pre-revision forward attempt in place. Never repurpose it
    // for rollback: it may contain the only backup of the original index.
    let legacy = plan_directory(workspace, journal_id);
    if status == JournalStatus::CanonicalCommittedIndexPending {
        match std::fs::symlink_metadata(&legacy) {
            Ok(_) => return Ok(legacy),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(revision_directory(workspace, journal_id, journal_digest, inventory_digest))
}

#[cfg(test)]
fn current_plan_directory(workspace: &Path, journal_id: &str) -> Result<PathBuf> {
    let digest = identity_repair::pending_repair_digest(workspace, journal_id)?;
    let source = identity_repair::census_workspace(workspace)?;
    let journal = identity_repair::inspect_repair_journal(workspace, journal_id)?;
    select_plan_directory(workspace, journal_id, &digest, &source.inventory_digest, journal.status)
}
#[cfg(windows)]
fn ensure_repair_directory(path: &Path) -> Result<()> {
    let parent = path.parent().context("repair directory has no parent")?;
    let (_, barrier, diagnostic) = crate::constellation::durability::windows_ntfs_primitive_barrier(parent);
    let barrier = barrier.ok_or_else(|| anyhow::anyhow!("repair durability unavailable: {diagnostic:?}"))?;
    barrier.ensure_dir(path)?;
    real_directory(path)
}

#[cfg(windows)]
fn publish_repair_file(dir: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    let (_, barrier, diagnostic) = crate::constellation::durability::windows_ntfs_primitive_barrier(dir);
    let barrier = barrier.ok_or_else(|| anyhow::anyhow!("repair durability unavailable: {diagnostic:?}"))?;
    crate::constellation::durability::install_immutable_file(dir, name, bytes, barrier.as_ref())?;
    Ok(())
}

fn repair_operation_time(dir: &Path, proof: &ValidatedLegacyRepair<'_>) -> Result<String> {
    proof.revalidate(proof.workspace())?;
    let lance = proof.workspace().join("lance");
    real_directory(&lance)?;
    #[cfg(windows)]
    ensure_repair_directory(dir)?;
    #[cfg(not(windows))]
    match std::fs::create_dir(dir) {
        Ok(()) => std::fs::File::open(&lance)?.sync_all()?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => real_directory(dir)?,
        Err(error) => return Err(error.into()),
    }
    let path = dir.join("header.json");
    if let Err(error) = std::fs::symlink_metadata(&path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(error.into());
        }
        let header = RepairHeader {
            version: 1, workspace: proof.workspace().to_path_buf(),
            journal_id: proof.journal_id().to_owned(), journal_digest: proof.journal_digest().to_owned(),
            inventory_digest: proof.source_digest().to_owned(), operation_time: chrono::Utc::now().to_rfc3339(),
        };
        #[cfg(windows)]
        publish_repair_file(dir, "header.json", &serde_json::to_vec(&header)?)?;
        #[cfg(not(windows))]
        {
        let temporary = dir.join(random_component("header-tmp-")?);
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary)?;
        file.write_all(&serde_json::to_vec(&header)?)?;
        file.sync_all()?;
        std::fs::hard_link(&temporary, &path)?;
        std::fs::File::open(dir)?.sync_all()?;
        std::fs::remove_file(temporary)?;
        std::fs::File::open(dir)?.sync_all()?;
        }
    }
    let header: RepairHeader = serde_json::from_slice(&read_regular(&path)?)?;
    if header.version != 1 || header.workspace.as_path() != proof.workspace()
        || header.journal_id != proof.journal_id() || header.journal_digest != proof.journal_digest()
        || header.inventory_digest != proof.source_digest()
        || chrono::DateTime::parse_from_rfc3339(&header.operation_time).is_err()
    {
        bail!("repair staging header is damaged or belongs to another journal");
    }
    proof.revalidate(proof.workspace())?;
    Ok(header.operation_time)
}


fn real_directory(path: &Path) -> Result<()> {
    let meta = std::fs::symlink_metadata(path)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            bail!("repair transaction directory is a reparse point: {}", path.display());
        }
    }
    if !meta.is_dir() || meta.file_type().is_symlink() {
        bail!("repair transaction directory is not a real directory: {}", path.display());
    }
    Ok(())
}

fn read_regular(path: &Path) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path)?;
    let meta = file.metadata()?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            bail!("repair transaction file is a reparse point: {}", path.display());
        }
    }
    if !meta.is_file() || meta.len() > 4 * 1024 * 1024 {
        bail!("repair transaction file is not a bounded regular file: {}", path.display());
    }
    let mut bytes = Vec::new();
    file.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 4 * 1024 * 1024 {
        bail!("repair transaction file grew beyond the size limit");
    }
    Ok(bytes)
}

fn marker_bytes(workspace: &Path) -> Result<Option<Vec<u8>>> {
    let path = workspace.join("lance").join(migrate::IDENTITY_V5_MARKER_FILE);
    match std::fs::symlink_metadata(&path) {
        Ok(_) => Ok(Some(read_regular(&path)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn validate_plan(plan: &RepairIndexPlan, proof: &ValidatedLegacyRepair<'_>) -> Result<()> {
    proof.revalidate(proof.workspace())?;
    if plan.version != 1 || plan.workspace.as_path() != proof.workspace()
        || plan.journal_id != proof.journal_id()
        || plan.journal_digest != proof.journal_digest()
        || plan.inventory_digest != proof.source_digest()
        || plan.tables.len() != NAMES.len()
        || !plan.stage_name.strip_prefix("stage-").is_some_and(|suffix|
            suffix.len() == 64 && suffix.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
        || plan.tables.iter().zip(NAMES).any(|(table, name)| table.name != name)
        || plan.new_marker.version != migrate::IDENTITY_V5_VERSION
        || !plan.new_marker.completed
        || plan.new_marker.artifacts_table != NAMES[0]
        || plan.new_marker.relations_table != NAMES[1]
        || plan.new_marker.evidence_table != NAMES[2]
        || plan.new_marker.source_digest != plan.source_digest
        || plan.digest != plan_digest(plan)?
    {
        bail!("repair index plan is damaged or belongs to another repair");
    }
    let semantic = plan.new_marker.semantic.as_ref()
        .ok_or_else(|| anyhow::anyhow!("repaired index has no semantic protocol"))?;
    if semantic.protocol != 1 || semantic.active_profile_digest.as_deref()
        != plan.new_marker.embeddings.as_ref().map(|profile| profile.profile_digest.as_str())
    {
        bail!("repaired index has inconsistent semantic protocol and profile");
    }
    let expected = serde_json::to_vec_pretty(&plan.new_marker)?;
    let actual = marker_bytes(proof.workspace())?;
    if actual.as_deref() != plan.old_marker.as_deref() && actual.as_deref() != Some(expected.as_slice()) {
        bail!("identity marker changed during repair index publication");
    }
    Ok(())
}

fn load_plan(dir: &Path, proof: &ValidatedLegacyRepair<'_>) -> Result<RepairIndexPlan> {
    real_directory(dir)?;
    real_directory(&dir.join("backup"))?;
    let plan: RepairIndexPlan = serde_json::from_slice(&read_regular(&dir.join("plan.json"))?)?;
    validate_plan(&plan, proof)?;
    real_directory(&dir.join(&plan.stage_name))?;
    Ok(plan)
}

fn validate_unchanged_plan(dir: &Path, proof: &ValidatedLegacyRepair<'_>, digest: &str) -> Result<()> {
    let current = load_plan(dir, proof)?;
    if current.digest != digest {
        bail!("repair index plan changed during publication");
    }
    Ok(())
}

async fn connection(root: &Path) -> Result<lancedb::connection::Connection> {
    Ok(lancedb::connect(root.to_str().context("non-UTF-8 Lance path")?).execute().await?)
}

async fn old_rows(
    workspace: &Path,
    plan: Option<&RepairIndexPlan>,
    dir: &Path,
    marker: Option<&migrate::IdentityV5Marker>,
) -> Result<[Vec<arrow_array::RecordBatch>; 3]> {
    let live = workspace.join("lance");
    let backup = dir.join("backup");
    let mut rows: [Vec<arrow_array::RecordBatch>; 3] = Default::default();
    for (index, out) in rows.iter_mut().enumerate() {
        if marker.is_none() {
            let name = ["artifacts", "relations", "evidence"][index];
            if let Some(plan) = plan {
                let actual = migrate::semantic_rebuild_tree_digest(&live.join(format!("{name}.lance")))?;
                if actual != plan.original_table_digests[index] {
                    bail!("original {name} table changed during repair");
                }
            }
            let db = connection(&live).await?;
            *out = migrate::table_rows(&db, name).await?;
            continue;
        }
        let filename = format!("{}.lance", NAMES[index]);
        let saved = backup.join(&filename);
        let (root, path) = match std::fs::symlink_metadata(&saved) {
            Ok(_) => (&backup, saved),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (&live, live.join(&filename)),
            Err(error) => return Err(error.into()),
        };
        if let Some(plan) = plan {
            let actual = migrate::semantic_rebuild_tree_digest(&path)?;
            if Some(actual.as_str()) != plan.tables[index].before_digest.as_deref() {
                bail!("original {} table no longer matches repair plan", NAMES[index]);
            }
        }
        let db = connection(root).await?;
        *out = migrate::table_rows(&db, NAMES[index]).await?;
    }
    Ok(rows)
}

/// A capability created only after exact row and fixed-table verification.
/// Its fields are private so a caller cannot assert publication by constructing it.
pub(crate) struct VerifiedLegacyIndex {
    workspace: PathBuf,
    journal_id: String,
    journal_digest: String,
    inventory_digest: String,
    marker: Vec<u8>,
    table_digests: [String; 3],
}

impl VerifiedLegacyIndex {
    pub(crate) fn matches_repair(&self, proof: &ValidatedLegacyRepair<'_>) -> bool {
        if self.workspace.as_path() != proof.workspace() || self.journal_id != proof.journal_id()
            || self.journal_digest != proof.journal_digest()
            || self.inventory_digest != proof.source_digest()
            || proof.revalidate(proof.workspace()).is_err()
            || marker_bytes(proof.workspace()).ok().flatten().as_deref() != Some(self.marker.as_slice())
        {
            return false;
        }
        NAMES.iter().zip(&self.table_digests).all(|(name, expected)| {
            migrate::semantic_rebuild_tree_digest(
                &self.workspace.join("lance").join(format!("{name}.lance")),
            ).is_ok_and(|actual| &actual == expected)
        }) && proof.revalidate(proof.workspace()).is_ok()
    }
}

async fn stage_plan(
    workspace: &Path,
    dir: &Path,
    proof: &ValidatedLegacyRepair<'_>,
    source: &migrate::IdentityV5Source,
    operation_time: &str,
) -> Result<(RepairIndexPlan, Option<legacy_repair_rows::PreparedRepairRows>)> {
    let lance = workspace.join("lance");
    real_directory(&lance)?;
    // Pre-plan attempts stay intact; retries create fresh, exclusive stages.
    match std::fs::symlink_metadata(dir) {
        Ok(_) => {
            real_directory(dir)?;
            match std::fs::symlink_metadata(dir.join("plan.json")) {
                Ok(_) => return Ok((load_plan(dir, proof)?, None)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            let backup = dir.join("backup");
            match std::fs::symlink_metadata(&backup) {
                Ok(_) => {
                    real_directory(&backup)?;
                    if std::fs::read_dir(&backup)?.next().transpose()?.is_some() {
                        bail!("unpublished repair stage has a nonempty backup");
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    proof.revalidate(workspace)?;
    let original_marker = marker_bytes(workspace)?;
    let marker = migrate::read_identity_v5_marker(&lance)?;
    let original_names = if marker.is_some() { NAMES } else { ["artifacts", "relations", "evidence"] };
    let mut original_table_digests = Vec::with_capacity(3);
    for name in original_names {
        original_table_digests.push(migrate::semantic_rebuild_tree_digest(
            &lance.join(format!("{name}.lance")),
        )?);
    }
    let old = old_rows(workspace, None, dir, marker.as_ref()).await?;
    let mut prepared = legacy_repair_rows::prepare(proof, &old, marker.as_ref(), source, operation_time).await?;
    migrate::prepare_semantic_rows_for_repair(workspace, proof, prepared.had_legacy_vectors, &mut prepared.rows, &mut prepared.marker).await?;
    proof.revalidate(workspace)?;
    if marker_bytes(workspace)? != original_marker {
        bail!("identity marker changed during repair preparation");
    }
    #[cfg(windows)]
    ensure_repair_directory(dir)?;
    #[cfg(not(windows))]
    match std::fs::create_dir(dir) {
        Ok(()) => std::fs::File::open(&lance)?.sync_all()?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => real_directory(dir)?,
        Err(error) => return Err(error.into()),
    }
    let backup = dir.join("backup");
    #[cfg(windows)]
    ensure_repair_directory(&backup)?;
    #[cfg(not(windows))]
    match std::fs::create_dir(&backup) {
        Ok(()) => std::fs::File::open(dir)?.sync_all()?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => real_directory(&backup)?,
        Err(error) => return Err(error.into()),
    }
    let stage_name = random_component("stage-")?;
    let staged = dir.join(&stage_name);
    #[cfg(windows)]
    ensure_repair_directory(&staged)?;
    #[cfg(not(windows))]
    std::fs::create_dir(&staged)?;
    migrate::write_prepared_identity_v5_rows(&staged, &prepared.rows).await?;
    let stage_db = connection(&staged).await?;
    migrate::verify_prepared_identity_v5_rows(&stage_db, &prepared.rows).await?;
    drop(stage_db);
    migrate::sync_rebuild_tree(&staged)?;
    proof.revalidate(workspace)?;
    for (index, name) in original_names.iter().enumerate() {
        if migrate::semantic_rebuild_tree_digest(&lance.join(format!("{name}.lance")))?
            != original_table_digests[index]
        {
            bail!("original {name} table changed during repair staging");
        }
    }
    let mut tables = Vec::with_capacity(3);
    for name in NAMES {
        let filename = format!("{name}.lance");
        let live = lance.join(&filename);
        let before_digest = match std::fs::symlink_metadata(&live) {
            Ok(_) => Some(migrate::semantic_rebuild_tree_digest(&live)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        if marker.is_some() && before_digest.is_none() {
            bail!("published identity index is missing table '{name}'");
        }
        tables.push(TablePlan {
            name: name.to_string(), before_digest,
            after_digest: migrate::semantic_rebuild_tree_digest(&staged.join(filename))?,
        });
    }
    let mut plan = RepairIndexPlan {
        version: 1, workspace: proof.workspace().to_path_buf(),
        journal_id: proof.journal_id().to_owned(), journal_digest: proof.journal_digest().to_owned(),
        inventory_digest: proof.source_digest().to_owned(), source_digest: source.source_digest.clone(),
        operation_time: operation_time.to_owned(), stage_name,
        old_marker: original_marker,
        original_table_digests: original_table_digests.try_into().map_err(|_| anyhow::anyhow!("invalid original table count"))?,
        new_marker: prepared.marker.clone(), tables, digest: String::new(),
    };
    plan.digest = plan_digest(&plan)?;
    validate_plan(&plan, proof)?;
    #[cfg(windows)]
    publish_repair_file(dir, "plan.json", &serde_json::to_vec(&plan)?)?;
    #[cfg(not(windows))]
    {
    let temporary = dir.join(random_component("plan-tmp-")?);
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary)?;
    file.write_all(&serde_json::to_vec(&plan)?)?;
    file.sync_all()?;
    std::fs::File::open(&backup)?.sync_all()?;
    std::fs::File::open(dir)?.sync_all()?;
    // hard_link publishes a complete file without replacing an existing plan.
    std::fs::hard_link(&temporary, dir.join("plan.json"))?;
    std::fs::File::open(dir)?.sync_all()?;
    std::fs::remove_file(&temporary)?;
    std::fs::File::open(dir)?.sync_all()?;
    std::fs::File::open(&lance)?.sync_all()?;
    }
    Ok((plan, Some(prepared)))
}

async fn reconcile_locked(workspace: &Path, proof: &ValidatedLegacyRepair<'_>, status: JournalStatus) -> Result<VerifiedLegacyIndex> {
    proof.revalidate(workspace)?;
    let dir = select_plan_directory(workspace, proof.journal_id(), proof.journal_digest(), proof.source_digest(), status)?;
    let operation_time = repair_operation_time(&dir, proof)?;
    let source = migrate::read_identity_v5_source_at(workspace, &operation_time).await?;
    proof.revalidate(workspace)?;
    let report = migrate::verify_identity_v5_source(&source);
    if status == JournalStatus::RolledBackIndexPending && !report.duplicate_keys.is_empty() {
        let diagnostic = crate::artifact::legacy_diagnostic::write_legacy_diagnostic(proof).await?;
        identity_repair::finish_legacy_diagnostic(proof, workspace, &diagnostic).await?;
        bail!("rollback restored conflicting canonical keys; diagnostic recorded and legacy index remains blocked");
    }
    if report.is_blocked() {
        bail!("repair source contains conflicting or unmappable identities: {report:?}");
    }
    let (plan, initially_prepared) = stage_plan(workspace, &dir, proof, &source, &operation_time).await?;
    failpoint(workspace, "after_stage")?;
    if plan.operation_time != operation_time {
        bail!("repair staging header and index plan have different timestamps");
    }
    if source.source_digest != plan.source_digest {
        bail!("repair source changed since the index plan was staged");
    }
    let prepared = if let Some(prepared) = initially_prepared {
        prepared
    } else {
        let old_marker: Option<migrate::IdentityV5Marker> = plan.old_marker.as_deref()
            .map(serde_json::from_slice).transpose()?;
        let old = old_rows(workspace, Some(&plan), &dir, old_marker.as_ref()).await?;
        legacy_repair_rows::prepare(proof, &old, old_marker.as_ref(), &source, &plan.operation_time).await?
    };
    if prepared.marker.version != plan.new_marker.version
        || prepared.marker.artifacts_table != plan.new_marker.artifacts_table
        || prepared.marker.relations_table != plan.new_marker.relations_table
        || prepared.marker.evidence_table != plan.new_marker.evidence_table
        || prepared.marker.source_digest != plan.new_marker.source_digest
        || prepared.marker.document_count != plan.new_marker.document_count
        || prepared.marker.completed != plan.new_marker.completed
        || prepared.marker.document_count != source.documents.len()
    {
        bail!("reconstructed structural marker differs from durable plan");
    }
    // The semantic profile is finalized when the stage is first prepared.
    // Structural regeneration on replay must not start another embedding job.
    // On replay a table may already be live, so compare each staged-or-live
    // replacement with freshly reconstructed rows before any further swap.
    let mut replacement_rows: [Vec<arrow_array::RecordBatch>; 3] = Default::default();
    for (index, name) in NAMES.iter().enumerate() {
        let staged = dir.join(&plan.stage_name);
        let filename = format!("{name}.lance");
        let staged_table = staged.join(&filename);
        let root = match std::fs::symlink_metadata(&staged_table) {
            Ok(_) => staged,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => workspace.join("lance"),
            Err(error) => return Err(error.into()),
        };
        if migrate::semantic_rebuild_tree_digest(&root.join(&filename))? != plan.tables[index].after_digest {
            bail!("replacement {name} table changed since the durable plan");
        }
        let db = connection(&root).await?;
        replacement_rows[index] = migrate::table_rows(&db, name).await?;
    }
    migrate::verify_semantic_rows_for_repair(
        workspace, proof, prepared.had_legacy_vectors, &prepared.rows, &replacement_rows, &plan.new_marker,
    )?;
    validate_unchanged_plan(&dir, proof, &plan.digest)?;
    let entries: Vec<_> = plan.tables.iter().map(|table| migrate::FixedTableSwapEntry {
        name: table.name.clone(), before_digest: table.before_digest.clone(), after_digest: table.after_digest.clone(),
    }).collect();
    migrate::complete_fixed_table_swap(workspace, &dir.join(&plan.stage_name), &dir.join("backup"), &entries,
        || validate_unchanged_plan(&dir, proof, &plan.digest))?;
    failpoint(workspace, "after_swap")?;
    let live_db = connection(&workspace.join("lance")).await?;
    migrate::verify_prepared_identity_v5_rows(&live_db, &replacement_rows).await?;
    drop(live_db);
    proof.revalidate(workspace)?;
    if migrate::read_identity_v5_source_at(workspace, &operation_time).await?.source_digest != plan.source_digest {
        bail!("repair source changed before index verification");
    }
    validate_unchanged_plan(&dir, proof, &plan.digest)?;
    let marker = serde_json::to_vec_pretty(&plan.new_marker)?;
    if marker_bytes(workspace)?.as_deref() != Some(marker.as_slice()) {
        if plan.old_marker.is_none() {
            migrate::publish_identity_v5_marker_new(&workspace.join("lance"), &plan.new_marker)?;
        } else {
            migrate::publish_identity_v5_marker(&workspace.join("lance"), &plan.new_marker)?;
        }
    }
    failpoint(workspace, "after_marker")?;
    if marker_bytes(workspace)?.as_deref() != Some(marker.as_slice()) {
        bail!("repair marker failed read-back verification");
    }
    let table_digests: [String; 3] = std::array::from_fn(|i| plan.tables[i].after_digest.clone());
    let verified = VerifiedLegacyIndex {
        workspace: proof.workspace().to_path_buf(), journal_id: proof.journal_id().to_owned(),
        journal_digest: proof.journal_digest().to_owned(), inventory_digest: proof.source_digest().to_owned(),
        marker, table_digests,
    };
    if !verified.matches_repair(proof) {
        bail!("published index changed after exact verification");
    }
    Ok(verified)
}

pub async fn reconcile_legacy_repair(workspace: &Path, journal_id: &str) -> Result<()> {
    #[cfg(windows)]
    bail!("legacy identity repair on Windows requires durable swap replay acknowledgement");
    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    bail!("legacy identity repair index publication requires an atomic no-replace rename");
    let predecessor = identity_repair::legacy_repair_predecessor_hint(workspace, journal_id)?;
    let guard = if let Some(previous) = predecessor {
        crate::workspace::lock::acquire_linked_repair_workspace_lock(workspace, &previous, journal_id).await?
    } else {
        crate::workspace::lock::acquire_legacy_repair_workspace_lock(workspace, journal_id).await?
    };
    let proof = identity_repair::validate_legacy_repair(workspace, &guard, journal_id)?;
    let status = identity_repair::pending_recovery_status(workspace, journal_id, &guard)?;
    let verified = reconcile_locked(workspace, &proof, status).await?;
    identity_repair::finish_legacy_repair(proof, workspace, verified)?;
    drop(guard);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn windows_repair_manifest_reader_rejects_directories() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("manifest.json");
        std::fs::write(&file, b"manifest").unwrap();
        assert_eq!(read_regular(&file).unwrap(), b"manifest");
        assert!(read_regular(root.path()).is_err());
    }

    #[tokio::test]
    async fn missing_repair_journal_fails_without_creating_an_index() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join(".forgeplan");
        std::fs::create_dir(&workspace).unwrap();
        assert!(reconcile_legacy_repair(&workspace, "missing").await.is_err());
        assert!(!workspace.join("lance").exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn markerless_v4_stage_then_rollback_preserves_stage_and_blocks_ordinary_access() {
        let root = tempfile::tempdir().unwrap();
        let workspace = migrate::populated_identity_v4_workspace_for_test(root.path()).await.unwrap().canonicalize().unwrap();
        std::fs::write(workspace.join("config.yaml"), "{}\n").unwrap();
        for (name, title) in [("PRD-002-a.md", "Unindexed A"), ("PRD-002-b.md", "Unindexed B")] {
            std::fs::write(workspace.join("prds").join(name), format!("---\nid: PRD-002\nkind: prd\nstatus: draft\ntitle: {title}\n---\n\n{title} body.\n")).unwrap();
        }
        let census = identity_repair::census_workspace(&workspace).unwrap();
        let plan = identity_repair::build_repair_plan(&workspace, &census, &[]).unwrap();
        let applied = identity_repair::apply_repair_plan(&workspace, &plan).unwrap();
        let id = &applied.transaction_id;
        assert!(!workspace.join("lance").join(migrate::IDENTITY_V5_MARKER_FILE).exists());
        let evidence = workspace.join("lance/evidence.lance");
        let parked = workspace.join("lance/evidence.parked");
        std::fs::rename(&evidence, &parked).unwrap();
        assert!(reconcile_legacy_repair(&workspace, id).await.is_err());
        std::fs::rename(&parked, &evidence).unwrap();
        assert!(reconcile_legacy_repair(&workspace, "wrong-transaction").await.is_err());
        assert!(crate::workspace::lock::acquire_workspace_read_lock(&workspace).await.is_err());
        assert!(crate::workspace::lock::acquire_workspace_lock(&workspace).await.is_err());
        fail_repair_once(&workspace, "after_stage");
        assert!(reconcile_legacy_repair(&workspace, id).await.is_err());
        let forward = current_plan_directory(&workspace, id).unwrap();
        let saved_plan = std::fs::read(forward.join("plan.json")).unwrap();
        let stage = repair_stage_path_for_test(&workspace, id).unwrap();
        let saved_stage = migrate::semantic_rebuild_tree_digest(&stage).unwrap();
        identity_repair::rollback_repair(&workspace, id).unwrap();
        let rollback = current_plan_directory(&workspace, id).unwrap();
        assert_ne!(forward, rollback);
        let error = reconcile_legacy_repair(&workspace, id).await.unwrap_err();
        assert!(format!("{error:#}").contains("diagnostic recorded"));
        let journal = identity_repair::inspect_repair_journal(&workspace, id).unwrap();
        assert_eq!(journal.status, JournalStatus::RolledBackIndexBlocked);
        assert!(journal.legacy_diagnostic.is_some());
        assert_eq!(std::fs::read(forward.join("plan.json")).unwrap(), saved_plan);
        assert_eq!(migrate::semantic_rebuild_tree_digest(&stage).unwrap(), saved_stage);
        assert!(crate::workspace::lock::acquire_workspace_read_lock(&workspace).await.is_err());
        assert!(crate::workspace::lock::acquire_workspace_lock(&workspace).await.is_err());
    }


    #[test]
    fn journal_id_is_never_used_as_a_path_component() {
        let workspace = Path::new("/workspace");
        let path = plan_directory(workspace, "../../foreign/plan");
        assert_eq!(path.parent(), Some(workspace.join("lance").as_path()));
        assert!(path.file_name().unwrap().to_str().unwrap().starts_with("legacy-repair-"));
        assert!(!path.to_string_lossy().contains("foreign"));
    }

    #[test]
    fn revision_selection_keeps_legacy_staging_and_separates_rollback() {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path();
        std::fs::create_dir(workspace.join("lance")).unwrap();
        let old = plan_directory(workspace, "repair-1");
        std::fs::create_dir(&old).unwrap();
        std::fs::write(old.join("backup"), b"original index").unwrap();
        let forward = select_plan_directory(workspace, "repair-1", "forward", "source-a", JournalStatus::CanonicalCommittedIndexPending).unwrap();
        let rollback = select_plan_directory(workspace, "repair-1", "rollback", "source-b", JournalStatus::RolledBackIndexPending).unwrap();
        assert_eq!(forward, old);
        assert_ne!(forward, rollback);
        assert_eq!(rollback, select_plan_directory(workspace, "repair-1", "rollback", "source-b", JournalStatus::RolledBackIndexPending).unwrap());
        assert_ne!(rollback, select_plan_directory(workspace, "repair-1", "rollback-revised", "source-b", JournalStatus::RolledBackIndexPending).unwrap());
        assert_eq!(std::fs::read(old.join("backup")).unwrap(), b"original index");
    }

    #[cfg(unix)]
    #[test]
    fn partial_fixed_table_swap_resumes_without_losing_preimages() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path();
        let lance = workspace.join("lance");
        let staged = workspace.join("staged");
        let backup = workspace.join("backup");
        for path in [&lance, &staged, &backup] {
            std::fs::create_dir(path).unwrap();
        }
        let mut entries = Vec::new();
        for name in NAMES {
            let filename = format!("{name}.lance");
            let before = lance.join(&filename);
            let after = staged.join(&filename);
            std::fs::create_dir(&before).unwrap();
            std::fs::create_dir(&after).unwrap();
            std::fs::write(before.join("row"), format!("before-{name}")).unwrap();
            std::fs::write(after.join("row"), format!("after-{name}")).unwrap();
            entries.push(migrate::FixedTableSwapEntry {
                name: name.to_owned(),
                before_digest: Some(migrate::semantic_rebuild_tree_digest(&before).unwrap()),
                after_digest: migrate::semantic_rebuild_tree_digest(&after).unwrap(),
            });
        }
        let calls = std::cell::Cell::new(0);
        let error = migrate::complete_fixed_table_swap(workspace, &staged, &backup, &entries, || {
            calls.set(calls.get() + 1);
            if calls.get() == 4 { bail!("injected interruption"); }
            Ok(())
        }).unwrap_err();
        assert!(error.to_string().contains("injected interruption"));
        migrate::complete_fixed_table_swap(workspace, &staged, &backup, &entries, || Ok(())).unwrap();
        for name in NAMES {
            let filename = format!("{name}.lance");
            assert_eq!(std::fs::read_to_string(lance.join(&filename).join("row")).unwrap(), format!("after-{name}"));
            assert_eq!(std::fs::read_to_string(backup.join(&filename).join("row")).unwrap(), format!("before-{name}"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn transaction_manifest_symlink_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tmp.path().join("outside");
        std::fs::write(&outside, b"untouched").unwrap();
        let link = tmp.path().join("plan.json");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        assert!(read_regular(&link).is_err());
        assert_eq!(std::fs::read(&outside).unwrap(), b"untouched");
    }
}

#[cfg(all(test, feature = "semantic-search"))]
#[path = "legacy_repair_tests.rs"]
mod legacy_repair_tests;
