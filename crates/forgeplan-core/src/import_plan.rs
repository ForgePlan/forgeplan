//! Preflight a complete import before migrating an index or changing any file.
//!
//! The plan borrows the actual workspace writer guard. It records the entire
//! canonical document set, including content digests, and resolves all bundle
//! selectors against one final document set before a caller may apply it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::artifact::canonical_key::{
    ArtifactCandidate, ResolveError, canonical_key_from_document, resolve_artifact,
};
use crate::artifact::frontmatter::{parse_frontmatter_borrowed, verify_import_canonical_key};
use crate::artifact::types::ArtifactKind;
use crate::db::store::{LanceStore, NewArtifact, validate_artifact_id, validate_link_target};
use crate::workspace::lock::{WorkspaceLock, WorkspaceReadPermit};

/// The exact canonical file that a plan expects before execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentPreimage {
    pub canonical_key: String,
    pub display_id: String,
    pub title: String,
    pub kind: String,
    pub relative_path: PathBuf,
    /// SHA-256 of the complete file bytes, not just its parsed body.
    pub content_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportAction {
    Create,
    Replace { expected: DocumentPreimage },
    Skip { expected: DocumentPreimage },
}

/// An input artifact with its identity and action already determined.
#[derive(Debug)]
pub struct PlannedArtifact {
    pub artifact: NewArtifact,
    pub action: ImportAction,
}

/// Identity conflicts and changes to the read set are distinct from I/O errors.
#[derive(Debug, thiserror::Error)]
pub enum ImportPlanError {
    #[error(transparent)]
    Resolve(#[from] ResolveError),
    #[error("import identity conflict for {key:?}: {reason}")]
    IdentityConflict { key: String, reason: String },
    #[error("import input is invalid: {0}")]
    InvalidInput(String),
    #[error("import preimage changed: {0}")]
    PreimageChanged(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    declared_edges: Vec<(String, String, String)>,
    parents: Vec<(String, String)>,
    documents: Vec<DocumentPreimage>,
    // Full non-vector record payload, normalized for v4 -> v5 parent keys.
    index_identities: Option<Vec<(String, String, String, [u8; 32])>>,
    index_relations: Option<Vec<(String, String, String)>>,
}

/// An immutable import bound to the writer lock used during its preflight.
pub struct ImportPlan<'guard> {
    guard: &'guard WorkspaceLock,
    workspace: PathBuf,
    before: Snapshot,
    actions: Vec<PlannedArtifact>,
    relations: Vec<(String, String, String)>,
    embedded_relations: Vec<(String, String, String)>,
    destinations: Vec<(PathBuf, Option<PathBuf>)>,
}

impl ImportPlan<'_> {
    pub fn actions(&self) -> &[PlannedArtifact] {
        &self.actions
    }

    /// Explicit bundle relations resolved once against the final document set.
    pub fn relations(&self) -> &[(String, String, String)] {
        &self.relations
    }

    /// Edges declared in imported canonical files, resolved against the same
    /// final set but kept separate from explicit bundle relations.
    pub fn embedded_relations(&self) -> &[(String, String, String)] {
        &self.embedded_relations
    }

    /// Recheck the complete preimage after reopening the store and before the
    /// first write. This never re-resolves an import selector.
    pub async fn verify(&self, workspace: &Path, store: Option<&LanceStore>) -> Result<()> {
        self.guard.matches_workspace(workspace)?;
        if std::fs::canonicalize(workspace)? != self.workspace {
            return Err(ImportPlanError::PreimageChanged("workspace changed".into()).into());
        }
        let current = capture_snapshot(workspace, self.guard, store).await?;
        if current != self.before {
            return Err(ImportPlanError::PreimageChanged(
                "canonical documents or indexed identities/relations changed".into(),
            ).into());
        }
        check_destinations(workspace, &self.destinations).await
    }
}

fn invalid(message: impl Into<String>) -> anyhow::Error {
    ImportPlanError::InvalidInput(message.into()).into()
}

fn conflict(key: &str, reason: impl Into<String>) -> anyhow::Error {
    ImportPlanError::IdentityConflict { key: key.to_string(), reason: reason.into() }.into()
}

fn field<'a>(value: &'a Value, name: &str, default: &'a str) -> Result<&'a str> {
    match value.get(name) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::String(text)) => Ok(text),
        Some(_) => Err(invalid(format!("field {name:?} must be a string"))),
    }
}

fn optional_field(value: &Value, name: &str) -> Result<Option<String>> {
    match value.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(invalid(format!("field {name:?} must be a string"))),
    }
}

fn candidates(documents: &[DocumentPreimage]) -> Vec<ArtifactCandidate> {
    documents.iter().map(|doc| ArtifactCandidate {
        canonical_key: doc.canonical_key.clone(),
        display_id: doc.display_id.clone(),
        title: doc.title.clone(),
        relative_path: doc.relative_path.clone(),
    }).collect()
}

fn require_unique_keys(docs: &[ArtifactCandidate]) -> Result<()> {
    let mut keys = BTreeSet::new();
    for doc in docs {
        if !keys.insert(&doc.canonical_key) {
            // The shared resolver supplies the complete typed conflict and candidates.
            resolve_artifact(&doc.canonical_key, docs)?;
            return Err(conflict(&doc.canonical_key, "canonical key has multiple claimants"));
        }
    }
    Ok(())
}

fn resolve_endpoint(input: &str, docs: &[ArtifactCandidate], source: bool) -> Result<String> {
    if !source && input.contains('@') {
        validate_link_target(input)?;
        return Ok(input.to_string());
    }
    validate_artifact_id(input)?;
    let resolved = if source {
        resolve_artifact(input, docs)?
    } else {
        crate::artifact::canonical_key::resolve_structural_target(input, docs)?
    };
    match resolved {
        Some(document) => {
            let key = document.identity.canonical_key;
            let check = if source {
                resolve_artifact(&key, docs)?
            } else {
                crate::artifact::canonical_key::resolve_structural_target(&key, docs)?
            };
            if check.as_ref().map(|row| row.identity.canonical_key.as_str()) != Some(key.as_str()) {
                return Err(conflict(&key, "resolved key does not select the same document"));
            }
            Ok(key)
        }
        None if source => Err(invalid(format!("relation source {input:?} does not exist"))),
        // Forward references are legitimate outgoing links.
        None => Ok(input.to_string()),
    }
}

fn bundle_endpoint(
    relation: &Value,
    key_field: &str,
    alias_field: &str,
    docs: &[ArtifactCandidate],
    source: bool,
) -> Result<String> {
    let key = field(relation, key_field, "")?;
    let alias = field(relation, alias_field, "")?;
    if key.is_empty() {
        return resolve_endpoint(alias, docs, source);
    }
    let selected = resolve_endpoint(key, docs, source)?;
    if selected != key {
        return Err(conflict(key, format!("{key_field} is an alias for {selected:?}, not a canonical key")));
    }
    if alias.is_empty() || alias == key {
        return Ok(selected);
    }
    if alias.contains('@') {
        validate_link_target(alias)?;
        return Err(conflict(key, format!("{alias_field} {alias:?} disagrees with {key_field}")));
    }
    validate_artifact_id(alias)?;
    match resolve_artifact(alias, docs) {
        Ok(Some(resolved)) if resolved.identity.canonical_key == selected => {}
        Err(ResolveError::Ambiguous { candidates, .. })
            if candidates.iter().any(|candidate| candidate.canonical_key == selected) => {}
        Err(error) => return Err(error.into()),
        _ => return Err(conflict(key, format!("{alias_field} {alias:?} disagrees with {key_field}"))),
    }
    Ok(selected)
}

async fn capture_snapshot(workspace: &Path, guard: &WorkspaceLock, store: Option<&LanceStore>) -> Result<Snapshot> {
    guard.matches_workspace(workspace)?;
    let listed = crate::db::store::workspace_documents(workspace).await?;
    let mut documents = Vec::with_capacity(listed.len());
    let mut declared_edges = Vec::new();
    let mut parents = Vec::new();
    for doc in listed {
        let relative_path = PathBuf::from(&doc.relative_path);
        let bytes = tokio::fs::read(workspace.join(&relative_path)).await
            .with_context(|| format!("read canonical document {}", relative_path.display()))?;
        let text = std::str::from_utf8(&bytes).context("canonical document is not UTF-8")?;
        let (fm, _) = parse_frontmatter_borrowed(text)?;
        let id = fm.get("id").and_then(|v| v.as_str()).ok_or_else(|| invalid("canonical document has no id"))?;
        let kind = fm.get("kind").and_then(|v| v.as_str()).ok_or_else(|| invalid(format!("{} has no kind", relative_path.display())))?;
        let title = fm.get("title").and_then(|v| v.as_str()).unwrap_or_default();
        let key = canonical_key_from_document(id, text);
        if id != doc.display_id || key != doc.canonical_key || title != doc.title {
            return Err(ImportPlanError::PreimageChanged(format!("{} changed while capturing", relative_path.display())).into());
        }
        for (target, relation) in crate::artifact::frontmatter::links_from_frontmatter(&fm) {
            declared_edges.push((key.clone(), target, relation));
        }
        if let Some(parent) = fm.get("parent_epic").and_then(|value| value.as_str()) {
            parents.push((key.clone(), parent.to_string()));
        }
        documents.push(DocumentPreimage {
            canonical_key: key,
            display_id: id.to_string(),
            title: title.to_string(),
            kind: kind.to_string(),
            relative_path,
            content_sha256: Sha256::digest(&bytes).into(),
        });
    }
    documents.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    let docs = candidates(&documents);
    require_unique_keys(&docs)?;

    let (index_identities, index_relations) = if let Some(store) = store {
        let records = store.list_records(guard, None).await?;
        let by_key: BTreeMap<&str, &DocumentPreimage> = documents.iter()
            .map(|doc| (doc.canonical_key.as_str(), doc))
            .collect();
        let mut indexed = Vec::with_capacity(records.len());
        for mut record in records {
            let matching = by_key.get(record.canonical_key.as_str());
            if !matching.is_some_and(|doc| doc.display_id == record.id && doc.kind == record.kind) {
                return Err(conflict(&record.canonical_key, "indexed row does not match exactly one canonical document"));
            }
            if let Some(parent) = record.parent_epic.as_deref() {
                record.parent_epic = Some(if parent.contains('@') {
                    validate_link_target(parent)?;
                    parent.to_string()
                } else {
                    resolve_endpoint(parent, &docs, false)?
                });
            }
            // Embeddings are deliberately excluded by ArtifactRecord's
            // Serialize contract: v4 recovery recomputes their provenance.
            let digest: [u8; 32] = Sha256::digest(serde_json::to_vec(&record)?).into();
            indexed.push((record.canonical_key, record.id, record.kind, digest));
        }
        indexed.sort();
        if indexed.len() != documents.len()
            || indexed.windows(2).any(|pair| pair[0].0 == pair[1].0)
        {
            return Err(invalid("index and canonical document set are incomplete or contain duplicate rows"));
        }
        let mut edges = Vec::new();
        for (source, target, relation) in store.get_all_relations(guard).await? {
            edges.push((resolve_endpoint(&source, &docs, true)?, resolve_endpoint(&target, &docs, false)?, relation));
        }
        edges.sort();
        if !store.uses_identity_v5() {
            // A v4 rebuild copies its existing edges and adds edges that the
            // canonical documents already declare. Compare the post-rebuild
            // table with that exact expected multiset, not the incomplete
            // pre-rebuild index.
            let mut expected = BTreeMap::new();
            for (source, target, relation) in &declared_edges {
                let edge = (source.clone(), resolve_endpoint(target, &docs, false)?, relation.clone());
                *expected.entry(edge).or_insert(0usize) += 1;
            }
            let mut missing = expected.clone();
            for edge in &edges {
                match missing.get_mut(edge) {
                    Some(count) if *count > 0 => *count -= 1,
                    _ => return Err(conflict(&edge.0, "legacy indexed relation is absent from the canonical document")),
                }
            }
            edges.clear();
            for (edge, count) in expected {
                for _ in 0..count {
                    edges.push(edge.clone());
                }
            }
        }
        (Some(indexed), Some(edges))
    } else {
        (None, None)
    };
    Ok(Snapshot { documents, declared_edges, parents, index_identities, index_relations })
}

async fn check_destinations(workspace: &Path, destinations: &[(PathBuf, Option<PathBuf>)]) -> Result<()> {
    let mut folded = BTreeSet::new();
    for (destination, replaced) in destinations {
        if !folded.insert(destination.to_string_lossy().to_ascii_lowercase()) {
            return Err(conflict(&destination.to_string_lossy(), "bundle destinations collide ignoring case"));
        }
        match tokio::fs::symlink_metadata(workspace.join(destination)).await {
            Ok(_) if replaced.as_ref() == Some(destination) => {}
            Ok(_) => return Err(conflict(&destination.to_string_lossy(), "destination is already occupied")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

/// Freeze the whole bundle and workspace before any migration or import write.
pub async fn plan_import<'guard>(
    workspace: &Path,
    guard: &'guard WorkspaceLock,
    store: Option<&LanceStore>,
    data: &Value,
    force: bool,
) -> Result<ImportPlan<'guard>> {
    guard.matches_workspace(workspace)?;
    let root = std::fs::canonicalize(workspace)?;
    let before = capture_snapshot(workspace, guard, store).await?;
    let existing = candidates(&before.documents);
    let artifacts = data.get("artifacts").and_then(Value::as_array)
        .ok_or_else(|| invalid("missing artifacts array"))?;
    let mut actions = Vec::with_capacity(artifacts.len());
    let mut seen = BTreeSet::new();
    let mut destinations = Vec::new();

    for (index, item) in artifacts.iter().enumerate() {
        if !item.is_object() { return Err(invalid(format!("artifact {index} is not an object"))); }
        let id = field(item, "id", "")?;
        validate_artifact_id(id)?;
        let kind = field(item, "kind", "note")?;
        let parsed_kind: ArtifactKind = kind.parse()?;
        let status = field(item, "status", "draft")?;
        if !matches!(status, "draft" | "active" | "superseded" | "deprecated" | "stale") {
            return Err(invalid(format!("invalid status {status:?} for {id}")));
        }
        let title = field(item, "title", "")?;
        let body = field(item, "body", "")?;
        let canonical = optional_field(item, "canonical_key")?.filter(|key| !key.is_empty());
        let slug = optional_field(item, "slug")?.filter(|key| !key.is_empty());
        if let (Some(canonical), Some(slug)) = (&canonical, &slug) {
            if canonical != slug {
                return Err(conflict(canonical, format!("bundle slug {slug:?} disagrees with canonical key")));
            }
        }
        let explicit = canonical.or(slug);
        let derived = canonical_key_from_document(id, body);
        let key = match explicit {
            Some(key) => key,
            None if derived == id => resolve_artifact(id, &existing)?.map(|r| r.identity.canonical_key).unwrap_or(derived),
            None => derived,
        };
        crate::artifact::store::canonical_filename(&key)?;
        if key != id && !crate::artifact::types::is_canonical_key(&key) {
            return Err(conflict(&key, "key cannot be declared by this document"));
        }
        verify_import_canonical_key(id, kind, body, &key)?;
        if let Ok((fm, _)) = parse_frontmatter_borrowed(body) {
            if let (Some(body_id), Some(body_kind)) = (fm.get("id").and_then(|v| v.as_str()), fm.get("kind").and_then(|v| v.as_str())) {
                if body_id != id || body_kind != kind {
                    return Err(conflict(&key, format!("complete body declares id {body_id:?}, kind {body_kind:?}, not {id:?}, {kind:?}")));
                }
                crate::artifact::frontmatter::refuse_canonical_key_change(&key, body)?;
                let mut layer = body;
                for _ in 0..8 {
                    let Ok((nested, rest)) = parse_frontmatter_borrowed(layer) else { break };
                    if nested.get("id").and_then(|value| value.as_str()).is_some_and(|value| value != id)
                        || nested.get("kind").and_then(|value| value.as_str()).is_some_and(|value| value != kind)
                    {
                        return Err(conflict(&key, "nested frontmatter declares a different id or kind"));
                    }
                    if !rest.trim_start().starts_with("---") { break; }
                    layer = rest;
                }
            }
        }
        if !seen.insert(key.clone()) { return Err(conflict(&key, "more than one bundle artifact claims this key")); }
        let old = before.documents.iter().find(|doc| doc.canonical_key == key);
        if let Some(old) = old {
            if old.display_id != id { return Err(conflict(&key, format!("existing display id {:?} differs from payload id {id:?}", old.display_id))); }
            if old.kind != kind { return Err(conflict(&key, format!("existing kind {:?} differs from payload kind {kind:?}", old.kind))); }
        }
        let action = match old {
            Some(old) if force => ImportAction::Replace { expected: old.clone() },
            Some(old) => ImportAction::Skip { expected: old.clone() },
            None => ImportAction::Create,
        };
        if !matches!(&action, ImportAction::Skip { .. }) {
            let dest = PathBuf::from(parsed_kind.dir_name()).join(crate::artifact::store::canonical_filename(&key)?);
            destinations.push((dest, old.map(|doc| doc.relative_path.clone())));
        }
        let tags = match item.get("tags") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(values)) => values.iter().map(|v| v.as_str().map(str::to_string).ok_or_else(|| invalid(format!("artifact {id} has a non-string tag")))).collect::<Result<Vec<_>>>()?,
            Some(_) => return Err(invalid(format!("artifact {id} tags must be an array"))),
        };
        actions.push(PlannedArtifact { artifact: NewArtifact {
            id: id.to_string(), canonical_key: key, kind: kind.to_string(), status: status.to_string(), title: title.to_string(), body: body.to_string(),
            depth: field(item, "depth", "standard")?.to_string(), author: optional_field(item, "author")?,
            parent_epic: optional_field(item, "parent_epic")?, valid_until: optional_field(item, "valid_until")?, tags,
        }, action });
    }

    // Projection and store helpers resolve canonical keys as they apply each
    // action. Prove those lookups succeed against every intermediate set,
    // not just the final set, before allowing the first destructive write.
    let mut intermediate = existing.clone();
    for item in &actions {
        let key = &item.artifact.canonical_key;
        match &item.action {
            ImportAction::Skip { .. } => continue,
            ImportAction::Replace { .. } => {
                let selected = resolve_artifact(key, &intermediate)?;
                if selected.as_ref().map(|row| row.identity.canonical_key.as_str()) != Some(key.as_str()) {
                    return Err(conflict(key, "replacement key does not select its preimage"));
                }
                intermediate.retain(|doc| doc.canonical_key != *key);
            }
            ImportAction::Create => {}
        }
        if resolve_artifact(key, &intermediate)?.is_some() {
            return Err(conflict(key, "creation key selects another document before insertion"));
        }
        intermediate.push(ArtifactCandidate {
            canonical_key: key.clone(),
            display_id: item.artifact.id.clone(),
            title: item.artifact.title.clone(),
            relative_path: PathBuf::new(),
        });
    }

    let mut final_docs = existing.clone();
    for action in &actions {
        if matches!(&action.action, ImportAction::Skip { .. }) { continue; }
        final_docs.retain(|doc| doc.canonical_key != action.artifact.canonical_key);
        final_docs.push(ArtifactCandidate { canonical_key: action.artifact.canonical_key.clone(), display_id: action.artifact.id.clone(), title: action.artifact.title.clone(), relative_path: PathBuf::new() });
    }
    require_unique_keys(&final_docs)?;
    for action in &actions {
        if !matches!(&action.action, ImportAction::Skip { .. }) {
            let resolved = resolve_artifact(&action.artifact.canonical_key, &final_docs)?;
            if resolved.as_ref().map(|r| r.identity.canonical_key.as_str()) != Some(action.artifact.canonical_key.as_str()) {
                return Err(conflict(&action.artifact.canonical_key, "final key does not select the planned document"));
            }
        }
    }
    let replaced: BTreeSet<&str> = actions.iter()
        .filter(|item| matches!(&item.action, ImportAction::Replace { .. }))
        .map(|item| item.artifact.canonical_key.as_str())
        .collect();
    let mut embedded_relations = Vec::new();
    for (source, target, _) in &before.declared_edges {
        if replaced.contains(source.as_str()) { continue; }
        let source = resolve_endpoint(source, &final_docs, true)?;
        let target = resolve_endpoint(target, &final_docs, false)?;
        if source.eq_ignore_ascii_case(&target) {
            return Err(conflict(&source, "self-link is not allowed"));
        }
    }
    for (source, parent) in &before.parents {
        if replaced.contains(source.as_str()) { continue; }
        if parent.contains('@') {
            validate_link_target(parent)?;
        } else {
            resolve_endpoint(parent, &final_docs, false)?;
        }
    }
    for action in &mut actions {
        if matches!(&action.action, ImportAction::Skip { .. }) { continue; }
        let complete_document = parse_frontmatter_borrowed(&action.artifact.body)
            .ok()
            .is_some_and(|(fm, _)| {
                fm.get("id").and_then(|value| value.as_str()) == Some(action.artifact.id.as_str())
                    && fm.get("kind").and_then(|value| value.as_str()) == Some(action.artifact.kind.as_str())
            });
        if let Some(parent) = &action.artifact.parent_epic {
            let resolved = resolve_endpoint(parent, &final_docs, false)?;
            if !complete_document {
                action.artifact.parent_epic = Some(resolved);
            }
        }
    }


    let relations_json = match data.get("relations") {
        None | Some(Value::Null) => &[][..],
        Some(Value::Array(values)) => values.as_slice(),
        Some(_) => return Err(invalid("relations must be an array")),
    };
    let mut relations = Vec::with_capacity(relations_json.len());
    for (index, rel) in relations_json.iter().enumerate() {
        if !rel.is_object() { return Err(invalid(format!("relation {index} is not an object"))); }
        let source = bundle_endpoint(rel, "source_key", "source", &final_docs, true)?;
        let target = bundle_endpoint(rel, "target_key", "target", &final_docs, false)?;
        let relation = field(rel, "relation", "informs")?;
        if relation.trim().is_empty() { return Err(invalid(format!("relation {index} has empty type"))); }
        if source.eq_ignore_ascii_case(&target) { return Err(conflict(&source, "self-link is not allowed")); }
        relations.push((source, target, relation.to_string()));
    }
    // Complete-file payloads carry additional edges that are not necessarily
    // repeated in the bundle relation array. Validate these before any write.
    for action in &actions {
        if matches!(&action.action, ImportAction::Skip { .. }) { continue; }
        if let Ok((fm, _)) = parse_frontmatter_borrowed(&action.artifact.body) {
            if fm.get("id").and_then(|v| v.as_str()) == Some(action.artifact.id.as_str())
                && fm.get("kind").and_then(|v| v.as_str()) == Some(action.artifact.kind.as_str())
            {
                if let Some(parent) = fm.get("parent_epic").and_then(|value| value.as_str()) {
                    if parent.contains('@') {
                        validate_link_target(parent)?;
                    } else {
                        resolve_endpoint(parent, &final_docs, false)?;
                    }
                }
                if let Some(links) = fm.get("links") {
                    let links = links.as_sequence().ok_or_else(|| invalid(format!("{} links must be a sequence", action.artifact.id)))?;
                    for link in links {
                        let target = link.get("target").and_then(|v| v.as_str()).ok_or_else(|| invalid("link target must be a string"))?;
                        let relation = link.get("relation").and_then(|v| v.as_str()).ok_or_else(|| invalid("link relation must be a string"))?;
                        if relation.trim().is_empty() { return Err(invalid("link relation cannot be empty")); }
                        let target = resolve_endpoint(target, &final_docs, false)?;
                        if target.eq_ignore_ascii_case(&action.artifact.canonical_key) { return Err(conflict(&target, "self-link is not allowed")); }
                        embedded_relations.push((action.artifact.canonical_key.clone(), target, relation.to_string()));
                    }
                }
            }
        }
    }
    check_destinations(workspace, &destinations).await?;
    Ok(ImportPlan { guard, workspace: root, before, actions, relations, embedded_relations, destinations })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    async fn workspace() -> (TempDir, PathBuf, WorkspaceLock) {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let guard = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        (tmp, ws, guard)
    }

    async fn document(ws: &Path, name: &str, id: &str, slug: &str, links: &str) -> Vec<u8> {
        let path = ws.join("prds").join(name);
        tokio::fs::create_dir_all(path.parent().unwrap()).await.unwrap();
        let bytes = format!(
            "---\nid: {id}\nkind: prd\nstatus: draft\ntitle: {slug}\nslug: {slug}\n{links}---\n\nOriginal content.\n"
        ).into_bytes();
        tokio::fs::write(path, &bytes).await.unwrap();
        bytes
    }

    async fn v4_store(ws: &Path) -> LanceStore {
        use arrow_array::RecordBatch;
        use arrow_schema::Schema;
        use std::sync::Arc;

        let lance = ws.join("lance");
        tokio::fs::create_dir_all(&lance).await.unwrap();
        let db = lancedb::connect(lance.to_str().unwrap()).execute().await.unwrap();
        let new = NewArtifact {
            id: "PRD-001".into(), canonical_key: "prd-original".into(),
            kind: "prd".into(), status: "draft".into(), title: "prd-original".into(),
            body: "Original content.\n".into(), depth: "standard".into(),
            ..Default::default()
        };
        let row = crate::db::convert::artifact_to_batch(&new, "2020-01-01T00:00:00Z").unwrap();
        let schema = Arc::new(Schema::new(
            row.schema().fields().iter().take(15).cloned().collect::<Vec<_>>(),
        ));
        let legacy = RecordBatch::try_new(schema, row.columns()[..15].to_vec()).unwrap();
        db.create_table("artifacts", vec![legacy]).execute().await.unwrap();
        db.create_table("relations", vec![RecordBatch::new_empty(crate::db::schema::relations_schema())])
            .execute().await.unwrap();
        db.create_table("evidence", vec![RecordBatch::new_empty(crate::db::schema::evidence_schema())])
            .execute().await.unwrap();
        LanceStore::open(ws).await.unwrap()
    }

    #[tokio::test]
    async fn late_duplicate_key_aborts_before_any_source_is_changed() {
        let (_tmp, ws, guard) = workspace().await;
        let original = document(&ws, "old.md", "PRD-001", "prd-original", "").await;
        let data = json!({"artifacts": [
            {"id":"PRD-001","kind":"prd","canonical_key":"prd-original","body":"first"},
            {"id":"PRD-001","kind":"prd","canonical_key":"prd-original","body":"second"}
        ]});
        let error = plan_import(&ws, &guard, None, &data, true).await.err().unwrap();
        assert!(matches!(error.downcast_ref::<ImportPlanError>(), Some(ImportPlanError::IdentityConflict { .. })));
        assert_eq!(tokio::fs::read(ws.join("prds/old.md")).await.unwrap(), original);
    }

    #[tokio::test]
    async fn same_key_with_different_id_or_kind_is_refused() {
        let (_tmp, ws, guard) = workspace().await;
        let original = document(&ws, "old.md", "PRD-001", "prd-original", "").await;
        for item in [
            json!({"id":"PRD-002","kind":"prd","canonical_key":"prd-original"}),
            json!({"id":"PRD-001","kind":"rfc","canonical_key":"prd-original"}),
        ] {
            let error = plan_import(&ws, &guard, None, &json!({"artifacts":[item]}), true).await.err().unwrap();
            assert!(matches!(error.downcast_ref::<ImportPlanError>(), Some(ImportPlanError::IdentityConflict { .. })));
        }
        assert_eq!(tokio::fs::read(ws.join("prds/old.md")).await.unwrap(), original);
    }

    #[tokio::test]
    async fn contradictory_complete_body_is_rejected_before_replacement() {
        let (_tmp, ws, guard) = workspace().await;
        let original = document(&ws, "old.md", "PRD-001", "prd-original", "").await;
        for body in [
            "---\nid: PRD-002\nkind: prd\nslug: prd-original\n---\nBody",
            "---\nid: PRD-001\nkind: prd\nslug: prd-different\n---\nBody",
            "---\nid: PRD-001\nkind: prd\nslug: prd-original\n---\n\n---\nid: PRD-002\nkind: prd\n---\nNested body",
        ] {
            let data = json!({"artifacts":[{"id":"PRD-001","kind":"prd","canonical_key":"prd-original","body":body}]});
            assert!(plan_import(&ws, &guard, None, &data, true).await.is_err());
        }
        assert_eq!(tokio::fs::read(ws.join("prds/old.md")).await.unwrap(), original);
    }

    #[tokio::test]
    async fn ambiguous_relation_reports_all_candidates_without_deleting_anything() {
        let (_tmp, ws, guard) = workspace().await;
        let one = document(&ws, "one.md", "PRD-002", "prd-one", "").await;
        let two = document(&ws, "two.md", "PRD-002", "prd-two", "").await;
        let data = json!({
            "artifacts":[{"id":"PRD-003","kind":"prd","canonical_key":"prd-three"}],
            "relations":[{"source":"prd-three","target":"PRD-002","relation":"based_on"}]
        });
        let error = plan_import(&ws, &guard, None, &data, true).await.err().unwrap();
        match error.downcast_ref::<ResolveError>() {
            Some(ResolveError::Ambiguous { input, candidates }) => {
                assert_eq!(input, "PRD-002");
                assert_eq!(candidates.len(), 2);
                assert_eq!(candidates[0].title, "prd-one");
                assert_eq!(candidates[0].relative_path, PathBuf::from("prds/one.md"));
                assert_eq!(candidates[1].title, "prd-two");
            }
            other => panic!("expected typed ambiguity, got {other:?}"),
        }
        assert_eq!(tokio::fs::read(ws.join("prds/one.md")).await.unwrap(), one);
        assert_eq!(tokio::fs::read(ws.join("prds/two.md")).await.unwrap(), two);
    }

    #[tokio::test]
    async fn new_alias_cannot_make_a_retained_document_link_ambiguous() {
        let (_tmp, ws, guard) = workspace().await;
        document(&ws, "source.md", "PRD-001", "prd-source",
            "links:\n  - target: PRD-002\n    relation: based_on\n").await;
        document(&ws, "target.md", "PRD-002", "prd-target", "").await;
        let data = json!({"artifacts":[
            {"id":"PRD-002","kind":"prd","canonical_key":"prd-other"}
        ]});
        let error = plan_import(&ws, &guard, None, &data, true).await.err().unwrap();
        assert!(matches!(error.downcast_ref::<ResolveError>(), Some(ResolveError::Ambiguous { .. })));
    }

    #[tokio::test]
    async fn preflight_preserves_missing_structural_links_and_parents() {
        let (_tmp, ws, guard) = workspace().await;
        let original = document(&ws, "source.md", "PRD-001", "prd-source",
            "parent_epic: EPIC-A01\nlinks:\n  - target: RFC-A01\n    relation: informs\n").await;
        let data = json!({
            "artifacts":[{"id":"PRD-002","kind":"prd","canonical_key":"prd-new","parent_epic":"EPIC-A01",
                "body":"---\nid: PRD-002\nkind: prd\nslug: prd-new\nparent_epic: EPIC-A01\nlinks:\n  - target: RFC-A01\n    relation: informs\n---\n\nBody"}],
            "relations":[{"source":"prd-source","target":"RFC-A01","relation":"refines"}]
        });
        let plan = plan_import(&ws, &guard, None, &data, false).await.unwrap();
        assert_eq!(plan.relations()[0].1, "RFC-A01");
        assert_eq!(plan.actions()[0].artifact.parent_epic.as_deref(), Some("EPIC-A01"));
        assert_eq!(tokio::fs::read(ws.join("prds/source.md")).await.unwrap(), original);
        for bad in ["---", "EPIC-4294967296"] {
            let bad_parent = json!({"artifacts":[{"id":"PRD-003","kind":"prd","canonical_key":"prd-third","parent_epic":bad}]});
            assert!(plan_import(&ws, &guard, None, &bad_parent, false).await.is_err());
            let bad_link = json!({"artifacts":[], "relations":[{"source":"prd-source","target":bad,"relation":"informs"}]});
            assert!(plan_import(&ws, &guard, None, &bad_link, false).await.is_err());
        }
    }

    #[tokio::test]
    async fn body_only_parent_is_canonical_but_complete_document_stays_literal() {
        let (_tmp, ws, guard) = workspace().await;
        let epic = ws.join("epics/epic-platform.md");
        tokio::fs::create_dir_all(epic.parent().unwrap()).await.unwrap();
        tokio::fs::write(&epic,
            "---\nid: EPIC-001\nkind: epic\nstatus: draft\ntitle: Platform\nslug: epic-platform\n---\n\nEpic.\n"
        ).await.unwrap();
        let full = "---\nid: PRD-004\nkind: prd\nstatus: draft\ntitle: Historical\nslug: prd-historical\nparent_epic: EPIC-001\n---\n\nHistorical body.\n";
        let data = json!({"artifacts":[
            {"id":"PRD-001","kind":"prd","title":"Child","canonical_key":"prd-child","parent_epic":"EPIC-001","body":"Child body."},
            {"id":"PRD-002","kind":"prd","title":"Forward","canonical_key":"prd-forward","parent_epic":"EPIC-A01","body":"Forward."},
            {"id":"PRD-003","kind":"prd","title":"External","canonical_key":"prd-external","parent_epic":"EPIC-001@remote","body":"External."},
            {"id":"PRD-004","kind":"prd","title":"Historical","canonical_key":"prd-historical","parent_epic":"EPIC-001","body":full}
        ]});
        let plan = plan_import(&ws, &guard, None, &data, false).await.unwrap();
        let parents: Vec<_> = plan.actions().iter()
            .map(|item| item.artifact.parent_epic.as_deref()).collect();
        assert_eq!(parents, [Some("epic-platform"), Some("EPIC-A01"),
            Some("EPIC-001@remote"), Some("EPIC-001")]);
        let ctx = crate::projection::MutationContext::new(&ws, &guard, None);
        for item in plan.actions() {
            let path = crate::projection::import_artifact_with_projection(&ctx, &item.artifact)
                .await.unwrap();
            let bytes = tokio::fs::read_to_string(path).await.unwrap();
            if item.artifact.id == "PRD-004" {
                assert_eq!(bytes, full);
            } else {
                let (fm, _) = parse_frontmatter_borrowed(&bytes).unwrap();
                assert_eq!(fm.get("parent_epic").and_then(|v| v.as_str()),
                    item.artifact.parent_epic.as_deref());
            }
        }
    }

    #[tokio::test]
    async fn relation_key_must_agree_with_the_supplied_display_alias() {
        let (_tmp, ws, guard) = workspace().await;
        document(&ws, "source.md", "PRD-001", "prd-source", "").await;
        document(&ws, "one.md", "PRD-002", "prd-one", "").await;
        document(&ws, "two.md", "PRD-003", "prd-two", "").await;
        let data = json!({"artifacts":[], "relations":[{
            "source_key":"prd-source", "source":"PRD-001",
            "target_key":"prd-one", "target":"PRD-003", "relation":"based_on"
        }]});
        let error = plan_import(&ws, &guard, None, &data, true).await.err().unwrap();
        assert!(matches!(error.downcast_ref::<ImportPlanError>(), Some(ImportPlanError::IdentityConflict { .. })));
    }

    #[tokio::test]
    async fn freezes_resolved_bundle_and_embedded_edges_and_rechecks_full_preimage() {
        let (_tmp, ws, guard) = workspace().await;
        document(&ws, "target.md", "PRD-002", "prd-target", "").await;
        let body = "---\nid: PRD-003\nkind: prd\nslug: prd-source\nlinks:\n  - target: PRD-002\n    relation: based_on\n---\n\nContent.";
        let data = json!({
            "artifacts":[{"id":"PRD-003","kind":"prd","canonical_key":"prd-source","body":body}],
            "relations":[{"source":"PRD-003","target":"PRD-002","relation":"informs"}]
        });
        let plan = plan_import(&ws, &guard, None, &data, false).await.unwrap();
        assert!(matches!(&plan.actions()[0].action, ImportAction::Create));
        assert_eq!(plan.actions()[0].artifact.id, "PRD-003");
        assert_eq!(plan.relations(), &[("prd-source".to_string(), "prd-target".to_string(), "informs".to_string())]);
        assert_eq!(plan.embedded_relations(), &[("prd-source".to_string(), "prd-target".to_string(), "based_on".to_string())]);
        plan.verify(&ws, None).await.unwrap();
        let path = ws.join("prds/target.md");
        let contents = tokio::fs::read_to_string(&path).await.unwrap();
        tokio::fs::write(&path, contents.replace("Original content.", "Changed content.")).await.unwrap();
        let error = plan.verify(&ws, None).await.unwrap_err();
        assert!(matches!(error.downcast_ref::<ImportPlanError>(), Some(ImportPlanError::PreimageChanged(_))));
    }

    #[tokio::test]
    async fn occupied_destination_without_an_identity_is_not_overwritten() {
        let (_tmp, ws, guard) = workspace().await;
        let path = ws.join("prds/prd-target.md");
        tokio::fs::create_dir_all(path.parent().unwrap()).await.unwrap();
        tokio::fs::write(&path, "---\ntitle: unowned\n---\nKeep this.").await.unwrap();
        let data = json!({"artifacts":[{"id":"PRD-004","kind":"prd","canonical_key":"prd-target"}]});
        assert!(plan_import(&ws, &guard, None, &data, true).await.is_err());
        assert_eq!(tokio::fs::read_to_string(path).await.unwrap(), "---\ntitle: unowned\n---\nKeep this.");
    }

    #[tokio::test]
    async fn indexed_workspace_revalidates_after_reopen_with_same_guard() {
        let (_tmp, ws, guard) = workspace().await;
        let store = LanceStore::init_new(&ws, &guard).await.unwrap();
        let artifact = NewArtifact {
            id: "PRD-005".into(), canonical_key: "prd-existing".into(),
            kind: "prd".into(), status: "draft".into(), title: "Existing".into(),
            body: "Body".into(), depth: "standard".into(), ..Default::default()
        };
        crate::projection::create_artifact_with_projection(
            &crate::projection::MutationContext::new(&ws, &guard, Some(&store)), &artifact,
        ).await.unwrap();
        let data = json!({"artifacts":[{"id":"PRD-005","kind":"prd","canonical_key":"prd-existing","body":"Updated"}]});
        let plan = plan_import(&ws, &guard, Some(&store), &data, true).await.unwrap();
        assert!(matches!(&plan.actions()[0].action, ImportAction::Replace { .. }));
        let reopened = LanceStore::open_for_write(&ws, &guard).await.unwrap();
        plan.verify(&ws, Some(&reopened)).await.unwrap();
        reopened.update_body(&guard, "prd-existing", "Changed indexed preimage").await.unwrap();
        let error = plan.verify(&ws, Some(&reopened)).await.unwrap_err();
        assert!(matches!(error.downcast_ref::<ImportPlanError>(), Some(ImportPlanError::PreimageChanged(_))));
    }

    #[tokio::test]
    async fn numeric_key_collision_is_rejected_before_an_earlier_replacement() {
        let (_tmp, ws, guard) = workspace().await;
        let original = document(&ws, "old.md", "PRD-001", "prd-original", "").await;
        let data = json!({"artifacts":[
            {"id":"PRD-001","kind":"prd","canonical_key":"prd-original","body":"Updated"},
            {"id":"PRD-003","kind":"prd","canonical_key":"PRD-001","body":"Other"}
        ]});
        assert!(plan_import(&ws, &guard, None, &data, true).await.is_err());
        assert_eq!(tokio::fs::read(ws.join("prds/old.md")).await.unwrap(), original);
    }

    #[tokio::test]
    async fn v4_conflict_refuses_before_migration_or_canonical_mutation() {
        let (_tmp, ws, guard) = workspace().await;
        let original = document(&ws, "old.md", "PRD-001", "prd-original", "").await;
        let store = v4_store(&ws).await;
        let data = json!({"artifacts":[
            {"id":"PRD-001","kind":"prd","canonical_key":"prd-original","body":"First"},
            {"id":"PRD-099","kind":"prd","canonical_key":"prd-original","body":"Conflict"}
        ]});
        assert!(plan_import(&ws, &guard, Some(&store), &data, true).await.is_err());
        assert!(!store.uses_identity_v5());
        assert!(crate::db::migrate::read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
        assert_eq!(tokio::fs::read(ws.join("prds/old.md")).await.unwrap(), original);
        assert_eq!(store.get_record(&guard, "prd-original").await.unwrap().unwrap().body, "Original content.\n");
    }

    #[tokio::test]
    async fn v4_plan_survives_guarded_migration_without_re_resolving_bundle() {
        let (_tmp, ws, guard) = workspace().await;
        document(&ws, "old.md", "PRD-001", "prd-original", "").await;
        let store = v4_store(&ws).await;
        let data = json!({"artifacts":[
            {"id":"PRD-001","kind":"prd","canonical_key":"prd-original","body":"New content"}
        ]});
        let plan = plan_import(&ws, &guard, Some(&store), &data, true).await.unwrap();
        assert!(matches!(&plan.actions()[0].action, ImportAction::Replace { .. }));
        let reopened = LanceStore::open_for_write(&ws, &guard).await.unwrap();
        assert!(reopened.uses_identity_v5());
        plan.verify(&ws, Some(&reopened)).await.unwrap();
    }
}
