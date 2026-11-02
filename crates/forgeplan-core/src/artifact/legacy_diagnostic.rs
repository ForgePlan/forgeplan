//! Durable, path-keyed diagnostics for a rolled-back identity repair.
//!
//! This is not a v5 marker or an index. Every read reconstructs the inventory
//! under the transaction's borrowed workspace lock before returning a proof.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{ErrorKind, Write};
use std::marker::PhantomData;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

use crate::artifact::canonical_key::{ArtifactCandidate, ResolveError, resolve_artifact};
use crate::artifact::identity_repair::{
    ValidatedLegacyRepair, census_workspace, effective_frontmatter, link_entries,
    parent_epic_target, repair_journal_root, sha256_hex,
};
use crate::db::migrate::{IDENTITY_V5_SOURCE_DIRS, read_identity_v5_source};
use sha2::{Digest, Sha256};

const SNAPSHOT_NAME: &str = "legacy-diagnostic.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Occurrence {
    field: String,
    occurrence: usize,
    target: String,
    relation: String,
    candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    canonical_key: String,
    display_id: String,
    title: String,
    sha256: String,
    references: Vec<Occurrence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sidecar {
    sha256: String,
    occurrences: Vec<Occurrence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    version: u32,
    workspace: PathBuf,
    transaction: String,
    journal_digest: String,
    inventory_digest: String,
    source_digest: String,
    documents: BTreeMap<String, Document>,
    sidecars: BTreeMap<String, Sidecar>,
}

/// Evidence of an on-disk snapshot revalidated against the locked workspace.
/// Private fields prevent callers from manufacturing this capability.
#[derive(Debug)]
pub struct VerifiedLegacyDiagnostic<'g> {
    workspace: PathBuf,
    transaction: String,
    journal_digest: String,
    inventory_digest: String,
    snapshot_digest: String,
    _guard: PhantomData<&'g ()>,
}

impl VerifiedLegacyDiagnostic<'_> {
    /// The SHA-256 of the exact published snapshot bytes.
    pub fn snapshot_digest(&self) -> &str { &self.snapshot_digest }

    /// A verified diagnostic is issued only after detecting a collision in
    /// the complete recursive source inventory.
    pub fn has_identity_collisions(&self) -> bool { true }

    /// Check that this proof belongs to the same guarded transaction.
    pub fn matches_repair(&self, proof: &ValidatedLegacyRepair<'_>) -> bool {
        self.workspace == proof.workspace()
            && self.transaction == proof.journal_id()
            && self.journal_digest == proof.journal_digest()
            && self.inventory_digest == proof.source_digest()
    }
}

fn safe_relative(value: &str) -> Result<()> {
    ensure!(!value.is_empty() && !value.contains('\\') && !value.contains('\0'), "unsafe relative path");
    ensure!(Path::new(value).components().all(|c| matches!(c, Component::Normal(_))), "unsafe relative path: {value:?}");
    ensure!(!value.split('/').any(|part| part.is_empty() || part == "." || part == ".."), "unsafe relative path: {value:?}");
    Ok(())
}

fn transaction_dir(workspace: &Path, transaction: &str) -> Result<PathBuf> {
    safe_relative(transaction)?;
    ensure!(Path::new(transaction).components().count() == 1, "invalid transaction id");
    let root = repair_journal_root(workspace);
    let dir = root.join(transaction);
    for path in [workspace, root.as_path(), dir.as_path()] {
        let metadata = fs::symlink_metadata(path).with_context(|| format!("inspect {}", path.display()))?;
        ensure!(metadata.is_dir() && !metadata.file_type().is_symlink(), "unsafe directory {}", path.display());
    }
    Ok(dir)
}

fn occurrence(field: &str, index: usize, target: &str, relation: &str, candidates: &[ArtifactCandidate]) -> Result<Occurrence> {
    let paths = if target.contains('@') {
        Vec::new()
    } else {
        match crate::artifact::canonical_key::resolve_structural_target(target, candidates) {
            Ok(Some(found)) => vec![found.relative_path.to_string_lossy().into_owned()],
            Ok(None) => Vec::new(), // Forward references are legal in the v5 source.
            Err(ResolveError::ConflictingCanonicalKey { candidates, .. }) => candidates.into_iter().map(|c| c.relative_path.to_string_lossy().into_owned()).collect(),
            Err(ResolveError::Ambiguous { candidates, .. }) if candidates.first().is_some_and(|first| candidates.iter().all(|c| c.canonical_key == first.canonical_key)) => candidates.into_iter().map(|c| c.relative_path.to_string_lossy().into_owned()).collect(),
            Err(error) => bail!("unrelated structural reference damage: {error}"),
        }
    };
    for path in &paths { safe_relative(path)?; }
    Ok(Occurrence { field: field.to_owned(), occurrence: index, target: target.to_owned(), relation: relation.to_owned(), candidates: paths })
}

fn inventory_files(workspace: &Path, roots: &[&str], extensions: &[&str]) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    for name in roots {
        let root = workspace.join(name);
        match fs::symlink_metadata(&root) {
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => return Err(e.into()),
            Ok(m) => ensure!(m.is_dir() && !m.file_type().is_symlink(), "unsafe inventory root {}", root.display()),
        }
        for item in walkdir::WalkDir::new(&root).follow_links(false).min_depth(1) {
            let item = item?;
            let ty = item.file_type();
            ensure!(!ty.is_symlink(), "symlink in inventory: {}", item.path().display());
            if ty.is_dir() { continue; }
            ensure!(ty.is_file(), "non-regular inventory entry: {}", item.path().display());
            if !extensions.contains(&item.path().extension().and_then(|s| s.to_str()).unwrap_or("")) { continue; }
            let relative = item.path().strip_prefix(workspace)?.to_str().context("non-UTF8 inventory path")?.to_owned();
            safe_relative(&relative)?;
            ensure!(files.insert(relative, read_regular(item.path())?).is_none(), "duplicate inventory path");
        }
    }
    Ok(files)
}

// Preserve every string-valued field, not just known identity field names:
// historical receipts have evolved and unknown nested references must not vanish.
fn sidecar_values(value: &serde_yaml::Value, path: &str, candidates: &[ArtifactCandidate], out: &mut Vec<Occurrence>) -> Result<()> {
    match value {
        serde_yaml::Value::String(text) => {
            let found = match resolve_artifact(text, candidates) {
                Ok(Some(item)) => vec![item.relative_path.to_string_lossy().into_owned()],
                Err(ResolveError::Ambiguous { candidates, .. } | ResolveError::ConflictingCanonicalKey { candidates, .. }) =>
                    candidates.into_iter().map(|c| c.relative_path.to_string_lossy().into_owned()).collect(),
                Ok(None) | Err(ResolveError::InvalidSelector { .. }) => Vec::new(),
                Err(error) => bail!("invalid sidecar reference at {path}: {error}"),
            };
            for candidate in &found { safe_relative(candidate)?; }
            out.push(Occurrence { field: path.to_owned(), occurrence: out.len(), target: text.clone(), relation: String::new(), candidates: found });
        }
        serde_yaml::Value::Sequence(items) => for (i, item) in items.iter().enumerate() {
            sidecar_values(item, &format!("{path}/{i}"), candidates, out)?;
        },
        serde_yaml::Value::Mapping(items) => for (key, item) in items {
            let raw_key = key.as_str().context("non-string sidecar mapping key")?;
            let key = raw_key.replace('~', "~0").replace('/', "~1");
            sidecar_values(&serde_yaml::Value::String(raw_key.to_owned()), &format!("{path}/{key}#key"), candidates, out)?;
            sidecar_values(item, &format!("{path}/{key}"), candidates, out)?;
        },
        serde_yaml::Value::Tagged(_) => bail!("tagged sidecar value at {path}"),
        _ => {}
    }
    Ok(())
}

async fn capture(proof: &ValidatedLegacyRepair<'_>) -> Result<Snapshot> {
    let workspace = proof.workspace();
    ensure!(fs::canonicalize(workspace)? == workspace, "workspace is not canonical");
    proof.revalidate(workspace)?;
    let census = census_workspace(workspace)?;
    ensure!(census.inventory_digest == proof.source_digest(), "workspace inventory changed");
    let files = inventory_files(workspace, IDENTITY_V5_SOURCE_DIRS, &["md"])?;
    let sidecar_files = inventory_files(workspace, &["claims", "state", "trash"], &["yaml", "yml", "json", "md"])?;
    let source = read_identity_v5_source(workspace).await?;
    let mut by_path = BTreeMap::new();
    for item in &source.documents {
        safe_relative(&item.relative_path)?;
        ensure!(by_path.insert(item.relative_path.as_str(), item).is_none(), "duplicate source path");
    }
    ensure!(by_path.len() == files.len(), "source and recursive census coverage differ");
    ensure!(
        census.documents.len() == files.len()
            && census.documents.iter().all(|document| files.contains_key(&document.relative_path)),
        "repair and diagnostic source coverage differ"
    );
    let candidates: Vec<_> = source.documents.iter().map(|d| ArtifactCandidate {
        canonical_key: d.canonical_key.clone(), display_id: d.display_id.clone(),
        title: d.title.clone(), relative_path: PathBuf::from(&d.relative_path),
    }).collect();
    let mut groups = BTreeMap::<&str, usize>::new();
    for candidate in &candidates { *groups.entry(&candidate.canonical_key).or_default() += 1; }
    ensure!(groups.values().any(|count| *count > 1), "diagnostic requires duplicate canonical keys");
    let mut digest = Sha256::new();
    let mut documents = BTreeMap::new();
    for (path, bytes) in &files {
        let parsed = by_path.get(path.as_str()).context("source missing from v5 inventory")?;
        let text = std::str::from_utf8(bytes)?;
        let links = link_entries(text).with_context(|| format!("parse links in {path}"))?;
        let parent = parent_epic_target(text).with_context(|| format!("parse parent in {path}"))?;
        ensure!(links.iter().map(|l| (l.target.as_str(), l.relation.as_str())).eq(parsed.links.iter().map(|(t, r)| (t.as_str(), r.as_str()))) && parent == parsed.parent_epic, "source reference parsers disagree: {path}");
        let mut references = Vec::new();
        for link in &links { references.push(occurrence("links", link.occurrence, &link.target, &link.relation, &candidates)?); }
        if let Some(parent) = parent { references.push(occurrence("parent_epic", 0, &parent, "", &candidates)?); }
        digest.update(path.as_bytes());
        digest.update([0u8]);
        digest.update(Sha256::digest(bytes));
        documents.insert(path.clone(), Document { canonical_key: parsed.canonical_key.clone(), display_id: parsed.display_id.clone(), title: parsed.title.clone(), sha256: sha256_hex(bytes), references });
    }
    let digest_hex: String = digest.finalize().iter().map(|byte| format!("{byte:02x}")).collect();
    ensure!(digest_hex == source.source_digest, "v5 source bytes changed during census");
    let mut sidecars = BTreeMap::new();
    for (path, bytes) in &sidecar_files {
        let value = if path.ends_with(".md") {
            let frontmatter = effective_frontmatter(std::str::from_utf8(bytes)?)?;
            serde_yaml::to_value(frontmatter)?
        } else {
            serde_yaml::from_slice(bytes).with_context(|| format!("parse sidecar {path}"))?
        };
        if path.starts_with("claims/") || path.starts_with("state/") {
            let mapping = value.as_mapping().with_context(|| format!("sidecar {path} is not a mapping"))?;
            let identity_field = if path.starts_with("claims/") { "id" } else { "artifact_id" };
            let identity = mapping.get(serde_yaml::Value::String(identity_field.to_owned()))
                .and_then(serde_yaml::Value::as_str).filter(|s| !s.trim().is_empty());
            ensure!(identity.is_some(), "sidecar {path} has invalid {identity_field}");
            if let Some(key) = mapping.get(serde_yaml::Value::String("canonical_key".to_owned())) {
                ensure!(key.is_null() || key.as_str().is_some_and(|s| !s.trim().is_empty()), "sidecar {path} has invalid canonical_key");
            }
        }
        let mut occurrences = Vec::new();
        let stem = Path::new(path).file_stem().and_then(|s| s.to_str()).context("invalid sidecar filename")?;
        sidecar_values(&serde_yaml::Value::String(stem.to_owned()), "$filename", &candidates, &mut occurrences)?;
        sidecar_values(&value, "", &candidates, &mut occurrences)?;
        sidecars.insert(path.clone(), Sidecar { sha256: sha256_hex(bytes), occurrences });
    }
    let all_paths: BTreeSet<_> = documents.keys().chain(sidecars.keys()).collect();
    ensure!(all_paths.len() == documents.len() + sidecars.len(), "overlapping inventory paths");
    proof.revalidate(workspace)?;
    ensure!(inventory_files(workspace, IDENTITY_V5_SOURCE_DIRS, &["md"])? == files && inventory_files(workspace, &["claims", "state", "trash"], &["yaml", "yml", "json", "md"])? == sidecar_files, "workspace changed during capture");
    Ok(Snapshot { version: 1, workspace: workspace.to_path_buf(), transaction: proof.journal_id().to_owned(), journal_digest: proof.journal_digest().to_owned(), inventory_digest: census.inventory_digest, source_digest: source.source_digest, documents, sidecars })
}

fn read_regular(path: &Path) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(metadata.is_file() && !metadata.file_type().is_symlink(), "snapshot is not a regular file");
    Ok(fs::read(path)?)
}

fn sync_dirs(dir: &Path) -> Result<()> {
    File::open(dir)?.sync_all()?;
    let parent = dir.parent().context("journal directory has no parent")?;
    File::open(parent)?.sync_all()?;
    if let Some(workspace) = parent.parent() { File::open(workspace)?.sync_all()?; }
    Ok(())
}

fn publish(dir: &Path, bytes: &[u8]) -> Result<()> {
    let target = dir.join(SNAPSHOT_NAME);
    let mut staging = tempfile::NamedTempFile::new_in(dir)?;
    staging.write_all(bytes)?;
    staging.as_file().sync_all()?;
    sync_dirs(dir)?;
    match staging.persist_noclobber(&target) {
        Ok(file) => { file.sync_all()?; },
        Err(error) if error.error.kind() == ErrorKind::AlreadyExists => {
            ensure!(read_regular(&target)? == bytes, "foreign or inconsistent diagnostic target");
            File::open(&target)?.sync_all()?;
        }
        Err(error) => return Err(error.error.into()),
    }
    sync_dirs(dir)?;
    ensure!(read_regular(&target)? == bytes, "diagnostic read-back mismatch");
    Ok(())
}

async fn verified<'g>(proof: &'g ValidatedLegacyRepair<'_>, write: bool) -> Result<VerifiedLegacyDiagnostic<'g>> {
    let dir = transaction_dir(proof.workspace(), proof.journal_id())?;
    let expected = capture(proof).await?;
    let bytes = serde_json::to_vec(&expected)?;
    if write { publish(&dir, &bytes)?; }
    let actual = read_regular(&dir.join(SNAPSHOT_NAME))?;
    let decoded: Snapshot = serde_json::from_slice(&actual).context("invalid diagnostic snapshot")?;
    ensure!(decoded == expected && actual == bytes, "diagnostic snapshot does not match current transaction and source");
    // A crash may have occurred after publication but before its directory
    // sync. Re-establish durability even when this is a read-only retry.
    File::open(dir.join(SNAPSHOT_NAME))?.sync_all()?;
    sync_dirs(&dir)?;
    // Recheck after I/O; no proof escapes if the source or journal changed.
    ensure!(capture(proof).await? == expected, "workspace changed during diagnostic verification");
    Ok(VerifiedLegacyDiagnostic { workspace: expected.workspace, transaction: expected.transaction, journal_digest: expected.journal_digest, inventory_digest: expected.inventory_digest, snapshot_digest: sha256_hex(&actual), _guard: PhantomData })
}

/// Check platform admission before acquiring any repair lock or proof.
pub fn ensure_legacy_diagnostic_write_supported() -> Result<()> {
    #[cfg(not(unix))]
    {
        bail!("identity diagnostic write is unsupported until no-reparse handle-bound durable namespace operations are available");
    }
    #[cfg(unix)]
    {
        Ok(())
    }
}

/// Publish without replacing any existing target, or verify an identical retry.
pub async fn write_legacy_diagnostic<'g>(proof: &'g ValidatedLegacyRepair<'_>) -> Result<VerifiedLegacyDiagnostic<'g>> {
    ensure_legacy_diagnostic_write_supported()?;
    verified(proof, true).await
}

/// Revalidate an existing diagnostic against its guarded journal and all sources.
pub async fn read_legacy_diagnostic<'g>(proof: &'g ValidatedLegacyRepair<'_>) -> Result<VerifiedLegacyDiagnostic<'g>> {
    verified(proof, false).await
}

/// Verify a previously bound receipt after its successor changed the corpus.
/// This checks stored bytes and bindings only; it does not grant read access.
pub(crate) fn verify_stored_snapshot_integrity(
    workspace: &Path,
    transaction: &str,
    snapshot_digest: &str,
    journal_digest: &str,
    inventory_digest: &str,
) -> Result<()> {
    ensure!(fs::canonicalize(workspace)? == workspace, "workspace is not canonical");
    let dir = transaction_dir(workspace, transaction)?;
    let bytes = read_regular(&dir.join(SNAPSHOT_NAME))?;
    ensure!(sha256_hex(&bytes) == snapshot_digest, "stored diagnostic digest mismatch");
    let snapshot: Snapshot = serde_json::from_slice(&bytes)?;
    ensure!(serde_json::to_vec(&snapshot)? == bytes, "noncanonical diagnostic encoding");
    ensure!(snapshot.version == 1 && snapshot.workspace == workspace && snapshot.transaction == transaction
        && snapshot.journal_digest == journal_digest && snapshot.inventory_digest == inventory_digest,
        "stored diagnostic bindings mismatch");
    File::open(dir.join(SNAPSHOT_NAME))?.sync_all()?;
    sync_dirs(&dir)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(unix))]
    #[test]
    fn non_unix_diagnostic_write_admission_fails_without_a_workspace() {
        let error = ensure_legacy_diagnostic_write_supported().unwrap_err();
        assert!(error.to_string().contains("identity diagnostic write is unsupported"));
    }

    #[cfg(unix)]
    #[test]
    fn unix_diagnostic_write_admission_succeeds() {
        ensure_legacy_diagnostic_write_supported().unwrap();
    }

    fn candidate(key: &str, path: &str) -> ArtifactCandidate {
        ArtifactCandidate { canonical_key: key.into(), display_id: "PRD-001".into(), title: path.into(), relative_path: path.into() }
    }

    #[tokio::test]
    async fn real_rollback_retries_and_refuses_changed_source() {
        use crate::artifact::identity_repair::{apply_repair_plan, build_repair_plan, rollback_repair, validate_legacy_repair};
        use crate::db::migrate::{identity_v5_marker_path, IDENTITY_V5_ARTIFACTS_TABLE, IDENTITY_V5_EVIDENCE_TABLE, IDENTITY_V5_RELATIONS_TABLE, IDENTITY_V5_VERSION};
        use crate::workspace::lock::acquire_legacy_repair_workspace_lock;

        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join(".forgeplan");
        fs::create_dir_all(workspace.join("prds")).unwrap();
        let first = "---\nid: PRD-001\nkind: prd\ntitle: First\nslug: prd-shared-key\nlinks:\n  - target: RFC-A01\n    relation: informs\n---\n\nFirst\n";
        let second = "---\nid: PRD-002\nkind: prd\ntitle: Second\nslug: prd-shared-key\n---\n\nSecond\n";
        fs::write(workspace.join("prds/one.md"), first).unwrap();
        fs::write(workspace.join("prds/two.md"), second).unwrap();
        let workspace = workspace.canonicalize().unwrap();
        let census = census_workspace(&workspace).unwrap();
        let plan = build_repair_plan(&workspace, &census, &[]).unwrap();
        let applied = apply_repair_plan(&workspace, &plan).unwrap();
        rollback_repair(&workspace, &applied.transaction_id).unwrap();
        fs::create_dir(workspace.join("lance")).unwrap();
        let marker = serde_json::json!({
            "version": IDENTITY_V5_VERSION, "artifacts_table": IDENTITY_V5_ARTIFACTS_TABLE,
            "relations_table": IDENTITY_V5_RELATIONS_TABLE, "evidence_table": IDENTITY_V5_EVIDENCE_TABLE,
            "source_digest": "a".repeat(64), "document_count": 0, "completed": true
        });
        fs::write(identity_v5_marker_path(&workspace.join("lance")), serde_json::to_vec(&marker).unwrap()).unwrap();
        let guard = acquire_legacy_repair_workspace_lock(&workspace, &applied.transaction_id).await.unwrap();
        let proof = validate_legacy_repair(&workspace, &guard, &applied.transaction_id).unwrap();
        let dir = transaction_dir(&workspace, &applied.transaction_id).unwrap();
        fs::write(dir.join(".interrupted"), b"incomplete").unwrap();
        let verified = write_legacy_diagnostic(&proof).await.unwrap();
        let digest = verified.snapshot_digest().to_owned();
        assert_eq!(write_legacy_diagnostic(&proof).await.unwrap().snapshot_digest(), digest);
        assert_eq!(read_legacy_diagnostic(&proof).await.unwrap().snapshot_digest(), digest);
        let snapshot: Snapshot = serde_json::from_slice(&read_regular(&dir.join(SNAPSHOT_NAME)).unwrap()).unwrap();
        assert_eq!(snapshot.documents.len(), 2);
        assert_eq!(snapshot.documents["prds/one.md"].canonical_key, snapshot.documents["prds/two.md"].canonical_key);
        let references = &snapshot.documents["prds/one.md"].references;
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].field, "links");
        assert_eq!(references[0].target, "RFC-A01");
        assert_eq!(references[0].relation, "informs");
        assert!(references[0].candidates.is_empty());
        fs::write(workspace.join("prds/one.md"), format!("{first}changed")).unwrap();
        assert!(read_legacy_diagnostic(&proof).await.is_err());
        assert!(write_legacy_diagnostic(&proof).await.is_err());
        fs::write(workspace.join("prds/one.md"), first).unwrap();
        assert_eq!(read_legacy_diagnostic(&proof).await.unwrap().snapshot_digest(), digest);
    }

    #[test]
    fn sidecar_nested_fields_and_recursive_inventory_are_preserved() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("state/nested")).unwrap();
        fs::write(temp.path().join("state/nested/a.yaml"), "artifact_id: PRD-001\nlinks:\n  - target: prd-same\n  - target: prd-same\n").unwrap();
        let files = inventory_files(temp.path(), &["state"], &["yaml"]).unwrap();
        let value: serde_yaml::Value = serde_yaml::from_slice(&files["state/nested/a.yaml"]).unwrap();
        let candidates = vec![candidate("prd-same", "prds/a.md"), candidate("prd-same", "prds/b.md")];
        let mut occurrences = Vec::new();
        sidecar_values(&value, "", &candidates, &mut occurrences).unwrap();
        let links: Vec<_> = occurrences.iter().filter(|item| item.target == "prd-same").collect();
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].field, "/links/0/target");
        assert_eq!(links[1].field, "/links/1/target");
        assert_eq!(links[0].candidates, vec!["prds/a.md", "prds/b.md"]);
    }

    #[test]
    fn duplicate_keys_keep_every_candidate_and_occurrence() {
        let candidates = vec![candidate("prd-same", "prds/a.md"), candidate("prd-same", "prds/b.md")];
        let a = occurrence("links", 0, "prd-same", "informs", &candidates).unwrap();
        let b = occurrence("links", 1, "prd-same", "refines", &candidates).unwrap();
        assert_eq!(a.candidates, vec!["prds/a.md", "prds/b.md"]);
        assert_eq!(b.candidates, a.candidates);
        assert_ne!(a.occurrence, b.occurrence);
        assert_ne!(a.relation, b.relation);
    }

    #[test]
    fn structural_occurrence_rejects_malformed_targets() {
        for target in ["---", "PRD-4294967296"] {
            let error = occurrence("links", 0, target, "informs", &[]).unwrap_err();
            assert!(error.to_string().contains("unrelated structural reference damage"));
        }
    }

    #[test]
    fn rejects_traversal_and_absolute_paths() {
        for path in ["../x", "a/../x", "/x", "a//b", "a/./b", "a\\b", ""] {
            assert!(safe_relative(path).is_err(), "{path:?}");
        }
        assert!(safe_relative("prds/a.md").is_ok());
    }

    #[test]
    fn interrupted_staging_and_identical_retry_do_not_replace_target() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".interrupted"), b"incomplete").unwrap();
        publish(dir.path(), b"snapshot").unwrap();
        publish(dir.path(), b"snapshot").unwrap();
        assert_eq!(read_regular(&dir.path().join(SNAPSHOT_NAME)).unwrap(), b"snapshot");
        assert_eq!(fs::read(dir.path().join(".interrupted")).unwrap(), b"incomplete");
    }

    #[test]
    fn foreign_target_and_changed_bytes_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(SNAPSHOT_NAME), b"foreign").unwrap();
        assert!(publish(dir.path(), b"snapshot").is_err());
        assert_eq!(fs::read(dir.path().join(SNAPSHOT_NAME)).unwrap(), b"foreign");
        assert!(publish(dir.path(), b"changed source").is_err());
    }
}
