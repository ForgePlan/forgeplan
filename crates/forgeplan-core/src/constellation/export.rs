//! Stage 9 — portable export: captured working data without local
//! absolute paths, with version and provenance contracts that round-trip.
//!
//! Two shapes of truth leave a workspace:
//!
//! * **`jsonl` — `constellation-export-v1`**: one JSON object per record —
//!   `header`, `source`, `document`, `edge`, `footer`. Identity is
//!   `source_id` + `version_key` + `canonical_key`; an absolute local path
//!   never appears (a `label` derived from the path tail is as close as a
//!   human hint gets).
//! * **`scip` — SCIP-shaped JSON lines**: documents and relation edges
//!   expressed as SCIP-style symbol / relationship records for consumers
//!   that speak the code-intelligence vocabulary. Symbols use the
//!   `forgeplan` scheme: `forgeplan {source_id} {kind}#{canonical_key}`.
//!
//! Round-trip contract: an exported stream parses back through
//! [`parse_export`] and yields the same source ids, version keys, git
//! OIDs and content digests it was written with — provenance is data,
//! not commentary. Automatic writes and remote execution are out of
//! scope by spec.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::capture::{CaptureLimits, CaptureRequest, capture_source, parse_artifact_file};
use super::federation::{
    QueryMode, SourceChannel, SourceFailure, SourceRef, SourceSpec, failure_is_fatal,
    source_id_for, source_unavailable,
};
use super::manifest::ProtocolMarker;
use super::snapshot::ConstellationSnapshot;
use super::{ConstellationError, sha256_hex};

/// Format tag — checked on re-parse; a version bump is a new tag.
pub const EXPORT_FORMAT: &str = "constellation-export-v1";
/// Format tag for the SCIP-shaped stream.
pub const EXPORT_FORMAT_SCIP: &str = "constellation-scip-jsonl-v1";

/// One source line of the export — provenance, never an absolute path.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExportSource {
    pub source_id: String,
    /// Human hint: the last two path components, e.g. `projA/.forgeplan`.
    pub label: String,
    pub channel: String,
    /// The read contract: generation id, capture digest, or `oid:tree:digest`.
    pub version_key: String,
    /// Pinned-git sources carry their full OIDs for provenance.
    pub git_oid: Option<String>,
    pub tree_oid: Option<String>,
    pub bytes_digest: Option<String>,
    pub documents: usize,
}

/// A document record — the portable unit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExportDocument {
    pub source_id: String,
    pub canonical_key: String,
    pub display_id: Option<String>,
    pub kind: String,
    pub status: String,
    pub title: String,
    /// Path *inside* the source — never absolute.
    pub relative_path: String,
    pub body_sha256: String,
    pub body: String,
}

/// An exact edge as the source wrote it, plus the resolution inside the
/// export's own document set when it exists.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExportEdge {
    pub source_id: String,
    pub from: String,
    pub relation: String,
    /// As written — `RFC-002`, `prd-001`, whatever the source spelled.
    pub target: String,
    /// Canonical key inside the same source when the target resolves.
    pub resolved: Option<String>,
}

/// The header record — the first line of a `jsonl` export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportHeader {
    pub format: String,
    pub exported_at: String,
    /// `live:<constellation>` or `snapshot:<snapshot_id>`.
    pub scope: String,
}

/// A parsed export — what [`parse_export`] hands back.
#[derive(Debug)]
pub struct ParsedExport {
    pub header: ExportHeader,
    pub sources: Vec<ExportSource>,
    pub documents: Vec<ExportDocument>,
    pub edges: Vec<ExportEdge>,
    /// Source ids that could not answer during export.
    pub failures: Vec<SourceFailure>,
}

/// Last-two-components label — a hint, never an authority.
fn label_for(path: &str) -> String {
    let parts: Vec<&str> = path.rsplit('/').filter(|p| !p.is_empty()).take(2).collect();
    parts.into_iter().rev().collect::<Vec<_>>().join("/")
}

/// The export-side shape of an artifact — every channel fills this;
/// the full `ParsedArtifact` carries fields export never emits.
#[derive(Clone)]
struct ExportRow {
    canonical_key: String,
    display_id: String,
    kind: String,
    status: String,
    title: String,
    relative_path: String,
    body: String,
    /// (target-as-written, relation)
    edges: Vec<(String, String)>,
}

impl From<&super::capture::ParsedArtifact> for ExportRow {
    fn from(a: &super::capture::ParsedArtifact) -> Self {
        Self {
            canonical_key: a.canonical_key.clone(),
            display_id: a.display_id.clone(),
            kind: a.kind.clone(),
            status: a.status.clone(),
            title: a.title.clone(),
            relative_path: a.relative_path.clone(),
            body: a.body.to_string(),
            edges: a
                .outgoing
                .iter()
                .map(|e| (e.target.clone(), e.relation.clone()))
                .collect(),
        }
    }
}

/// Collect every document + edge of one live source.
async fn collect_live(spec: &SourceSpec) -> Result<(SourceRef, Vec<ExportRow>), SourceFailure> {
    let canonical = spec.path.canonicalize().map_err(|e| SourceFailure {
        path: spec.path.display().to_string(),
        reason: format!("cannot canonicalize: {e}"),
    })?;
    let source_id = source_id_for(&canonical);
    // Pinned channel when a generation is published — same rule as
    // federation: a broken index fails the source, not silently downgrades.
    let index_root = canonical.join("lance");
    if index_root.join(super::storage::CURRENT_FILE_NAME).is_file() {
        let descriptor = super::storage::read_current(&index_root)
            .and_then(|d| {
                d.ok_or_else(|| ConstellationError::IndexGenerationUnavailable {
                    detail: "CURRENT vanished between checks".into(),
                })
            })
            .map_err(|e| SourceFailure {
                path: spec.path.display().to_string(),
                reason: format!("{e}"),
            })?;
        let manifest =
            super::storage::read_manifest(&index_root, &descriptor).map_err(|e| SourceFailure {
                path: spec.path.display().to_string(),
                reason: format!("{e}"),
            })?;
        // The pinned read honours the descriptor's embedding head: the
        // manifest's version predates any drain that landed vectors, and
        // exporting it would ship `unavailable` rows that are already ready.
        let artifacts_version = descriptor
            .effective_artifacts_version(&manifest)
            .map_err(|e| SourceFailure {
                path: spec.path.display().to_string(),
                reason: format!("{e}"),
            })?;
        let docs = dump_pinned(&index_root, &manifest, artifacts_version)
            .await
            .map_err(|e| SourceFailure {
                path: spec.path.display().to_string(),
                reason: format!("{e}"),
            })?;
        let source = SourceRef {
            source_id,
            path: canonical.display().to_string(),
            channel: SourceChannel::Pinned,
            version_key: manifest.generation_id.clone(),
        };
        return Ok((source, docs));
    }
    let captured = capture_source(&CaptureRequest::new(&canonical), CaptureLimits::default())
        .await
        .map_err(|e| SourceFailure {
            path: spec.path.display().to_string(),
            reason: format!("{e}"),
        })?;
    let source = SourceRef {
        source_id,
        path: canonical.display().to_string(),
        channel: SourceChannel::Markdown,
        version_key: captured.source_set_digest.clone(),
    };
    Ok((
        source,
        captured
            .artifacts
            .iter()
            .map(|a| ExportRow::from(&**a))
            .collect(),
    ))
}

/// Read every artifact row (with bodies) + edges from the pinned tables.
async fn dump_pinned(
    index_root: &Path,
    manifest: &super::manifest::GenerationManifest,
    artifacts_version: u64,
) -> Result<Vec<ExportRow>, ConstellationError> {
    use arrow_array::{Array, StringArray};

    let marker = ProtocolMarker {
        protocol_version: super::FORMAT_VERSION,
        state: super::manifest::BootstrapState::Active,
        index_epoch: manifest.index_epoch,
        checkout_id: manifest.checkout_id.clone(),
        table_names: ProtocolMarker::active_table_names(),
        storage_scheme: super::manifest::STORAGE_SCHEME_FILE_OBJECT_STORE.to_string(),
        bootstrap_attempt: 1,
    };
    let artifacts = super::lance_tables::open_table_at_version(
        index_root,
        &marker,
        "artifacts",
        artifacts_version,
    )
    .await?;
    let relations = super::lance_tables::open_table_at_version(
        index_root,
        &marker,
        "relations",
        manifest.table_versions.relations,
    )
    .await?;
    // Edges grouped per canonical key — the relation set the artifacts
    // carry as `outgoing`.
    let mut edges: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for batch in &relations
        .select_columns(&["canonical_key", "target", "relation"])
        .await?
    {
        let value = |col: &str, row: usize| {
            batch
                .column_by_name(col)
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .filter(|c| !c.is_null(row))
                .map(|c| c.value(row).to_string())
        };
        for row in 0..batch.num_rows() {
            edges
                .entry(value("canonical_key", row).unwrap_or_default())
                .or_default()
                .push((
                    value("target", row).unwrap_or_default(),
                    value("relation", row).unwrap_or_default(),
                ));
        }
    }
    let bodies = artifacts
        .select_columns(&[
            "canonical_key",
            "id",
            "title",
            "kind",
            "status",
            "relative_path",
            "body",
        ])
        .await?;
    let mut docs = Vec::new();
    for batch in &bodies {
        let value = |col: &str, row: usize| {
            batch
                .column_by_name(col)
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .filter(|c| !c.is_null(row))
                .map(|c| c.value(row).to_string())
        };
        let big = |col: &str, row: usize| -> Option<String> {
            batch.column_by_name(col).and_then(|c| {
                c.as_any()
                    .downcast_ref::<arrow_array::LargeStringArray>()
                    .map(|a| a.value(row).to_string())
            })
        };
        for row in 0..batch.num_rows() {
            let key = value("canonical_key", row).unwrap_or_default();
            docs.push(ExportRow {
                canonical_key: key.clone(),
                display_id: value("id", row).unwrap_or_default(),
                kind: value("kind", row).unwrap_or_default(),
                status: value("status", row).unwrap_or_default(),
                title: value("title", row).unwrap_or_default(),
                body: big("body", row)
                    .or_else(|| value("body", row))
                    .unwrap_or_default(),
                relative_path: value("relative_path", row).unwrap_or_default(),
                edges: edges.get(&key).cloned().unwrap_or_default(),
            });
        }
    }
    Ok(docs)
}

/// Every document+edge of one snapshot source, from pinned git objects.
fn collect_snapshot(
    entry: &super::snapshot::SnapshotSourceEntry,
) -> Result<(SourceRef, Vec<ExportRow>), String> {
    let repo = PathBuf::from(&entry.checkout_root);
    let names =
        super::snapshot::list_files_at(&repo, &entry.git_oid).map_err(|e| format!("{e}"))?;
    let mut docs = Vec::new();
    for rel in &names {
        let bytes = super::snapshot::read_file_at(&repo, &entry.git_oid, rel)
            .map_err(|e| format!("{e}"))?;
        if let Ok(p) = parse_artifact_file(
            rel,
            Arc::from(&bytes[..]),
            super::jobs::DEFAULT_BODY_PREFIX_CHARS,
        ) {
            docs.push(ExportRow::from(&p));
        }
    }
    let source = SourceRef {
        source_id: format!(
            "src:{}",
            &sha256_hex(format!("{}@{}", entry.checkout_root, entry.git_oid).as_bytes())[..16]
        ),
        path: entry.source_path.clone(),
        channel: SourceChannel::PinnedGit,
        version_key: format!(
            "{}:{}:{}",
            entry.git_oid, entry.tree_oid, entry.bytes_digest
        ),
    };
    Ok((source, docs))
}

fn to_records(
    sources: &[(SourceRef, Vec<ExportRow>, Option<SnapshotOids>)],
    failures: Vec<SourceFailure>,
) -> (Vec<ExportSource>, Vec<ExportDocument>, Vec<ExportEdge>) {
    let _ = &failures;
    let mut out_sources = Vec::new();
    let mut out_docs = Vec::new();
    let mut out_edges = Vec::new();
    for (source, artifacts, oids) in sources {
        let mut key_to_canonical: BTreeMap<String, String> = BTreeMap::new();
        for a in artifacts.iter() {
            key_to_canonical
                .entry(a.canonical_key.clone())
                .or_insert_with(|| a.canonical_key.clone());
            key_to_canonical
                .entry(a.display_id.to_lowercase())
                .or_insert_with(|| a.canonical_key.clone());
        }
        for a in artifacts {
            for (target, relation) in &a.edges {
                out_edges.push(ExportEdge {
                    source_id: source.source_id.clone(),
                    from: a.canonical_key.clone(),
                    relation: relation.clone(),
                    target: target.clone(),
                    resolved: key_to_canonical
                        .get(target)
                        .or_else(|| key_to_canonical.get(&target.to_lowercase()))
                        .cloned(),
                });
            }
            out_docs.push(ExportDocument {
                source_id: source.source_id.clone(),
                canonical_key: a.canonical_key.clone(),
                display_id: Some(a.display_id.clone()),
                kind: a.kind.clone(),
                status: a.status.clone(),
                title: a.title.clone(),
                relative_path: a.relative_path.clone(),
                body_sha256: sha256_hex(a.body.as_bytes()),
                body: a.body.clone(),
            });
        }
        out_sources.push(ExportSource {
            source_id: source.source_id.clone(),
            label: label_for(&source.path),
            channel: source.channel.channel_str().to_string(),
            version_key: source.version_key.clone(),
            git_oid: oids.as_ref().map(|o| o.git_oid.clone()),
            tree_oid: oids.as_ref().map(|o| o.tree_oid.clone()),
            bytes_digest: oids.as_ref().map(|o| o.bytes_digest.clone()),
            documents: artifacts.len(),
        });
    }
    (out_sources, out_docs, out_edges)
}

struct SnapshotOids {
    git_oid: String,
    tree_oid: String,
    bytes_digest: String,
}

/// Serialize an export to the `jsonl` stream.
pub fn render_jsonl(
    scope: &str,
    sources: &[ExportSource],
    documents: &[ExportDocument],
    edges: &[ExportEdge],
    failures: &[SourceFailure],
) -> String {
    let mut out = String::new();
    let header = ExportHeader {
        format: EXPORT_FORMAT.to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        scope: scope.to_string(),
    };
    out.push_str(&serde_json::to_string(&serde_json::json!({"header": header})).unwrap());
    out.push('\n');
    for s in sources {
        out.push_str(&serde_json::to_string(&serde_json::json!({"source": s})).unwrap());
        out.push('\n');
    }
    for d in documents {
        out.push_str(&serde_json::to_string(&serde_json::json!({"document": d})).unwrap());
        out.push('\n');
    }
    for e in edges {
        out.push_str(&serde_json::to_string(&serde_json::json!({"edge": e})).unwrap());
        out.push('\n');
    }
    if !failures.is_empty() {
        out.push_str(&serde_json::to_string(&serde_json::json!({"failures": failures})).unwrap());
        out.push('\n');
    }
    out.push_str(
        &serde_json::to_string(&serde_json::json!({
            "footer": {"documents": documents.len(), "edges": edges.len()}
        }))
        .unwrap(),
    );
    out.push('\n');
    out
}

/// Serialize to the SCIP-shaped stream: a `meta` header, then `document`,
/// `symbol` and `relationship` records.
pub fn render_scip(
    scope: &str,
    sources: &[ExportSource],
    documents: &[ExportDocument],
    edges: &[ExportEdge],
) -> String {
    let symbol_for =
        |source_id: &str, kind: &str, key: &str| format!("forgeplan {source_id} {kind}#{key}");
    let mut out = String::new();
    out.push_str(
        &serde_json::to_string(&serde_json::json!({
            "meta": {
                "format": EXPORT_FORMAT_SCIP,
                "exported_at": chrono::Utc::now().to_rfc3339(),
                "scope": scope,
                "tool": concat!("forgeplan-", env!("CARGO_PKG_VERSION")),
            }
        }))
        .unwrap(),
    );
    out.push('\n');
    for s in sources {
        out.push_str(
            &serde_json::to_string(&serde_json::json!({
                "package": {
                    "manager": "forgeplan",
                    "name": s.label,
                    "version": s.version_key,
                    "source_id": s.source_id,
                    "channel": s.channel,
                    "git_oid": s.git_oid,
                    "tree_oid": s.tree_oid,
                    "bytes_digest": s.bytes_digest,
                }
            }))
            .unwrap(),
        );
        out.push('\n');
    }
    for d in documents {
        out.push_str(
            &serde_json::to_string(&serde_json::json!({
                "document": {
                    "relative_path": d.relative_path,
                    "language": "markdown",
                    "symbol": symbol_for(&d.source_id, &d.kind, &d.canonical_key),
                    "display_id": d.display_id,
                    "title": d.title,
                    "status": d.status,
                    "body_sha256": d.body_sha256,
                }
            }))
            .unwrap(),
        );
        out.push('\n');
    }
    for e in edges {
        out.push_str(
            &serde_json::to_string(&serde_json::json!({
                "relationship": {
                    "symbol": symbol_for(&e.source_id, "artifact", &e.from),
                    "relation": e.relation,
                    "target": e.target,
                    "resolved_symbol": e.resolved.as_ref().map(|k| symbol_for(&e.source_id, "artifact", k)),
                }
            }))
            .unwrap(),
        );
        out.push('\n');
    }
    out
}

/// Parse a `jsonl` export back — the round-trip contract's reader half.
pub fn parse_export(text: &str) -> Result<ParsedExport, ConstellationError> {
    let mut header = None;
    let mut sources = Vec::new();
    let mut documents = Vec::new();
    let mut edges = Vec::new();
    let mut failures = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|e| ConstellationError::Io {
                detail: format!("export line {} does not parse: {e}", line_no + 1),
            })?;
        if let Some(h) = value.get("header") {
            let h: ExportHeader =
                serde_json::from_value(h.clone()).map_err(|e| ConstellationError::Io {
                    detail: format!("export header does not parse: {e}"),
                })?;
            if h.format != EXPORT_FORMAT {
                return Err(ConstellationError::IndexProtocolMismatch {
                    detail: format!("export format {:?} is not {EXPORT_FORMAT}", h.format),
                });
            }
            header = Some(h);
        } else if let Some(s) = value.get("source") {
            sources.push(
                serde_json::from_value::<ExportSource>(s.clone()).map_err(|e| {
                    ConstellationError::Io {
                        detail: format!("export source line {} does not parse: {e}", line_no + 1),
                    }
                })?,
            );
        } else if let Some(d) = value.get("document") {
            documents.push(
                serde_json::from_value::<ExportDocument>(d.clone()).map_err(|e| {
                    ConstellationError::Io {
                        detail: format!("export document line {} does not parse: {e}", line_no + 1),
                    }
                })?,
            );
        } else if let Some(e) = value.get("edge") {
            edges.push(
                serde_json::from_value::<ExportEdge>(e.clone()).map_err(|e| {
                    ConstellationError::Io {
                        detail: format!("export edge line {} does not parse: {e}", line_no + 1),
                    }
                })?,
            );
        } else if let Some(f) = value.get("failures") {
            failures = serde_json::from_value::<Vec<SourceFailure>>(f.clone()).map_err(|e| {
                ConstellationError::Io {
                    detail: format!("export failures line does not parse: {e}"),
                }
            })?;
        }
    }
    let header = header.ok_or_else(|| ConstellationError::Io {
        detail: "export has no header record".to_string(),
    })?;
    Ok(ParsedExport {
        header,
        sources,
        documents,
        edges,
        failures,
    })
}

/// Export the live scope — every source on its honest channel.
///
/// `mode` applies the required/optional contract: a `required` source
/// that cannot answer aborts the strict export with `source_unavailable`
/// — a stream missing a required corpus would ship as if complete. An
/// `optional` source, or any source under [`QueryMode::AllowPartial`],
/// lands in `failures` and is recorded in the stream's failure line.
pub async fn export_live(
    specs: &[SourceSpec],
    scope_name: &str,
    mode: QueryMode,
) -> Result<(String, String, ParsedExport), ConstellationError> {
    let mut collected: Vec<(SourceRef, Vec<ExportRow>, Option<SnapshotOids>)> = Vec::new();
    let mut failures = Vec::new();
    for spec in specs {
        match collect_live(spec).await {
            Ok((source, docs)) => collected.push((source, docs, None)),
            Err(f) => {
                if failure_is_fatal(spec, mode) {
                    return Err(source_unavailable(&f));
                }
                failures.push(f);
            }
        }
    }
    let (sources, documents, edges) = to_records(&collected, failures.clone());
    let scope = format!("live:{scope_name}");
    let jsonl = render_jsonl(&scope, &sources, &documents, &edges, &failures);
    let parsed = ParsedExport {
        header: ExportHeader {
            format: EXPORT_FORMAT.to_string(),
            exported_at: String::new(),
            scope: scope.clone(),
        },
        sources,
        documents,
        edges,
        failures,
    };
    Ok((scope, jsonl, parsed))
}

/// Export a recorded snapshot — pinned revisions, same records.
pub fn export_snapshot(
    snapshot: &ConstellationSnapshot,
) -> Result<(String, String, ParsedExport), ConstellationError> {
    let mut collected: Vec<(SourceRef, Vec<ExportRow>, Option<SnapshotOids>)> = Vec::new();
    let mut failures: Vec<SourceFailure> = snapshot
        .skipped
        .iter()
        .map(|s| SourceFailure {
            path: s.path.clone(),
            reason: format!("source was not pinned: {}", s.reason),
        })
        .collect();
    for entry in &snapshot.sources {
        match collect_snapshot(entry) {
            Ok((source, docs)) => collected.push((
                source,
                docs,
                Some(SnapshotOids {
                    git_oid: entry.git_oid.clone(),
                    tree_oid: entry.tree_oid.clone(),
                    bytes_digest: entry.bytes_digest.clone(),
                }),
            )),
            Err(reason) => failures.push(SourceFailure {
                path: entry.source_path.clone(),
                reason,
            }),
        }
    }
    let (sources, documents, edges) = to_records(&collected, failures.clone());
    let scope = format!("snapshot:{}", snapshot.snapshot_id);
    let jsonl = render_jsonl(&scope, &sources, &documents, &edges, &failures);
    let parsed = ParsedExport {
        header: ExportHeader {
            format: EXPORT_FORMAT.to_string(),
            exported_at: String::new(),
            scope: scope.clone(),
        },
        sources,
        documents,
        edges,
        failures,
    };
    Ok((scope, jsonl, parsed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const DOC: &str = "---\nid: ADR-001\nkind: adr\nstatus: accepted\ntitle: Auth storage\nlinks:\n  - target: PRD-001\n    relation: refines\n---\n\nBody about auth.\n";
    const DOC2: &str =
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth prd\n---\n\nPrd body.\n";

    fn ws() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".forgeplan");
        std::fs::create_dir_all(root.join("adrs")).unwrap();
        std::fs::create_dir_all(root.join("prds")).unwrap();
        std::fs::write(root.join("adrs/ADR-001.md"), DOC).unwrap();
        std::fs::write(root.join("prds/PRD-001.md"), DOC2).unwrap();
        tmp
    }

    #[tokio::test]
    async fn export_round_trips_provenance_and_hides_absolute_paths() {
        let ws = ws();
        let specs = vec![SourceSpec::new(ws.path().join(".forgeplan"))];
        let (scope, jsonl, _) = export_live(&specs, "demo", QueryMode::Strict)
            .await
            .unwrap();

        // Round trip: parse back, compare contract fields.
        let parsed = parse_export(&jsonl).unwrap();
        assert_eq!(parsed.header.format, EXPORT_FORMAT);
        assert!(parsed.header.scope.starts_with("live:"));
        assert_eq!(parsed.sources.len(), 1);
        assert_eq!(parsed.documents.len(), 2);
        // Edge resolved inside the export's own document set.
        assert!(
            parsed
                .edges
                .iter()
                .any(|e| e.relation == "refines" && e.resolved.as_deref() == Some("PRD-001"))
        );

        // No absolute local path anywhere in the stream.
        assert!(
            !jsonl.contains(ws.path().to_str().unwrap()),
            "export leaks the absolute workspace path"
        );
        for line in jsonl.lines() {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            let text = serde_json::to_string(&v).unwrap();
            assert!(
                !text.contains("/tmp/") && !text.contains("/private/") && !text.contains("/Users/"),
                "line leaks an absolute path: {text}"
            );
        }
        let _ = scope;
    }

    #[tokio::test]
    async fn scip_records_carry_symbols_and_provenance() {
        let ws = ws();
        let specs = vec![SourceSpec::new(ws.path().join(".forgeplan"))];
        let (_, _, parsed) = export_live(&specs, "demo", QueryMode::Strict)
            .await
            .unwrap();
        let scip = render_scip(
            "live:demo",
            &parsed.sources,
            &parsed.documents,
            &parsed.edges,
        );
        assert!(scip.contains(EXPORT_FORMAT_SCIP));
        assert!(scip.contains("forgeplan src:"));
        assert!(scip.contains("\"relationship\""));
        assert!(scip.contains("refines"));
        // And it too hides absolute paths.
        assert!(!scip.contains(ws.path().to_str().unwrap()));
    }

    #[test]
    fn wrong_format_tag_is_a_protocol_error_not_empty() {
        let err = parse_export(
            "{\"header\":{\"format\":\"other-v9\",\"exported_at\":\"x\",\"scope\":\"s\"}}\n",
        )
        .unwrap_err();
        assert!(matches!(
            err,
            ConstellationError::IndexProtocolMismatch { .. }
        ));
    }

    #[tokio::test]
    async fn a_dead_required_source_refuses_the_export_stream() {
        // Exporting without a required source would ship a stream that
        // reads complete; strict mode refuses instead. An optional source
        // — or an AllowPartial call — lands in the failures line, which
        // `parse_export` reads back.
        let ws = ws();
        let specs = vec![
            SourceSpec::new(ws.path().join(".forgeplan")),
            SourceSpec::new("/definitely/not/a/workspace"),
        ];
        let err = export_live(&specs, "demo", QueryMode::Strict)
            .await
            .unwrap_err();
        assert!(matches!(err, ConstellationError::SourceUnavailable { .. }));

        let (_, jsonl, parsed) = export_live(&specs, "demo", QueryMode::AllowPartial)
            .await
            .unwrap();
        assert_eq!(parsed.failures.len(), 1);
        assert_eq!(parsed.documents.len(), 2);
        assert!(jsonl.contains("\"failures\""), "{jsonl}");

        let specs = vec![
            SourceSpec::new(ws.path().join(".forgeplan")),
            SourceSpec::new_optional("/definitely/not/a/workspace"),
        ];
        let (_, _, parsed) = export_live(&specs, "demo", QueryMode::Strict)
            .await
            .unwrap();
        assert_eq!(parsed.failures.len(), 1);
        assert_eq!(parsed.documents.len(), 2);
    }
}
