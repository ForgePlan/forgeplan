//! SPEC-009 — identity repair: the version-1 plan, the recovery journal, and
//! the byte-level rewriting the repair performs.
//!
//! The repair is **preserve-all** (SPEC-009 "Preserve-all repair"): a
//! document is never deleted, never merged, and its historical display
//! number is never rewritten. What changes is the *canonical key* of every
//! member of a colliding group. The canonical key lives in the document's
//! `slug:` frontmatter field (ADR-030), so the rewrite is a surgical edit of
//! that one field: the body, the second frontmatter block, and every other
//! byte of the file stay exactly as they were.
//!
//! Three things live here, and the first two are pure:
//!
//! 1. the plan and journal data models plus their digests;
//! 2. the document splitter that says which bytes are frontmatter and which
//!    are body (the same rule the core frontmatter parser uses, including
//!    the nested-wrapper rule);
//! 3. the surgical setters — canonical key, one `links` target,
//!    `parent_epic` — that the repair applies to a document.
//!
//! On top of those sit the four moving parts of the repair itself:
//!
//! 4. the **census** — a read-only walk of the workspace that hashes every
//!    document, derives its canonical key, and collects the inbound
//!    structural references;
//! 5. the **plan writer** — the version-1 plan as JSON, with a digest over
//!    every field it carries;
//! 6. the **apply** — the transaction that rewrites the documents under the
//!    workspace lock and publishes every post-image atomically;
//! 7. the **recovery journal** — the preimages, the planned post-images and
//!    the per-operation state that make the transaction reversible.
//!
//! The repair never deletes or merges a document or rewrites a historical
//! display number. An attributed claim or state sidecar is copied to its new
//! canonical-key path before its old path is removed; both paths are journaled.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::artifact::canonical_key::{canonical_key_from_document, mint_artifact_key};
use crate::artifact::frontmatter::parse_frontmatter;
use crate::artifact::types::{ArtifactKind, is_minted_canonical_key};
use crate::workspace::lock::{WorkspaceLock, lock_path};

/// Version of the repair plan format this module reads and writes.
pub const REPAIR_PLAN_VERSION: u32 = 1;

/// SHA-256 of `bytes`, lowercase hex.
///
/// The repair plan and the recovery journal both carry checksums; they are
/// computed here so the plan writer, the plan verifier and the journal
/// writer cannot disagree about how a digest is spelled.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in hash.iter() {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Git identity of the workspace at plan time.
///
/// `None` when the workspace is not inside a Git repository — a plan made
/// outside Git carries no Git identity, and the apply then refuses to run
/// against a workspace that has become a Git repository (or the other way
/// round).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitIdentity {
    /// Full commit hash of `HEAD` at plan time.
    pub head: String,
    /// Absolute path of the Git common directory (`git rev-parse
    /// --git-common-dir`). It is the shared state of linked worktrees, so it
    /// is part of the workspace identity.
    pub common_dir: String,
}

/// One document the repair rewrites.
///
/// The three hashes are of the *same* file at three granularities: the whole
/// file, the body, and the frontmatter region. The apply re-checks all
/// three, so a change anywhere in the document — not only in the field the
/// repair touches — makes the plan stale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedSource {
    /// Workspace-relative path of the document (forward slashes).
    pub relative_path: String,
    /// SHA-256 of the whole file.
    pub file_sha256: String,
    /// SHA-256 of the body (everything after the frontmatter region).
    pub body_sha256: String,
    /// SHA-256 of the frontmatter region (all leading YAML blocks).
    pub frontmatter_sha256: String,
    /// The canonical key the document carries today.
    pub old_key: String,
    /// The canonical key the document receives. Reserved in the plan: the
    /// apply never mints a different one.
    pub new_key: String,
    /// Where the document will live after the repair. The repair never moves
    /// a file (RFC-019 R-1: existing paths stay), so this equals
    /// `relative_path` for every plan this module writes — but the plan
    /// carries it explicitly so the apply can refuse a destination that is
    /// already occupied by a different document.
    pub proposed_path: String,
}

/// One inbound structural reference the repair rewrites.
///
/// `from` is the target as the frontmatter says it today; `to` is the
/// canonical key it carries after the repair. `mapping` names every
/// candidate the reference could have denoted and the key each candidate
/// receives, so a reviewer can see *why* a reference points where it points.
/// An occurrence with more than one candidate is unresolved, and the apply
/// refuses it: an unknown relation is the owner's to clarify, not the tool's
/// to guess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedReference {
    /// Workspace-relative path of the document that holds the reference.
    pub source_path: String,
    /// The field the reference lives in: `links`, `parent_epic`, or the name
    /// of a sidecar file.
    pub field: String,
    /// Zero-based index of the occurrence inside its field.
    pub occurrence: usize,
    /// The reference target as written today.
    pub from: String,
    /// The reference target after the repair. Empty when the occurrence is
    /// unresolved.
    pub to: String,
    /// Candidate document path → the canonical key that candidate receives.
    pub mapping: BTreeMap<String, String>,
}

impl PlannedReference {
    /// Whether the occurrence names a decided target.
    ///
    /// An occurrence is resolved when it carries a target *and* that target
    /// is one of the keys the candidates receive. A reference that matched
    /// several candidates stays unresolved until a review decides which one
    /// it denotes; the mapping then holds every candidate together with the
    /// key it receives, so the decision is visible instead of implied.
    pub fn is_resolved(&self) -> bool {
        !self.to.is_empty() && self.mapping.values().any(|key| key == &self.to)
    }
}

/// An attributed move of a per-document claim or phase state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedSidecarMove {
    pub source_path: String,
    pub destination_path: String,
    pub owner_path: String,
    pub display_id: String,
    pub display_id_field: String,
    pub canonical_field_present: bool,
    pub canonical_field_before: Option<String>,
    pub old_key: String,
    pub new_key: String,
    pub source_sha256: String,
    pub postimage_sha256: String,
}

/// The version-1 repair plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairPlan {
    /// Plan format version. Always [`REPAIR_PLAN_VERSION`] today.
    pub version: u32,
    /// Absolute, canonicalized path of the `.forgeplan` directory the plan
    /// was made against.
    pub workspace_path: String,
    /// Git identity at plan time, absent outside a Git repository.
    pub git: Option<GitIdentity>,
    /// Versioned `v2:<sha256>` digest of document and sidecar paths and bytes.
    pub inventory_digest: String,
    /// Documents whose canonical key changes.
    pub sources: Vec<PlannedSource>,
    /// Inbound references the repair rewrites.
    pub references: Vec<PlannedReference>,
    /// Digest-bound sidecar moves; absent from legacy plans.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sidecar_moves: Vec<PlannedSidecarMove>,
    /// Unresolved or unsupported sidecars block application.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sidecar_conflicts: Vec<String>,
    /// Digest over every field above.
    pub digest: String,
}

/// Compute the digest of a plan over every field it carries.
///
/// The digest is computed over the plan with its own `digest` field
/// emptied, so it covers the version, the workspace path, the Git identity,
/// the inventory digest, every source and every reference — and nothing
/// else. It is an integrity check over the file, not an authorization: the
/// apply re-checks the workspace itself before it touches anything.
pub fn compute_plan_digest(plan: &RepairPlan) -> String {
    let mut probe = plan.clone();
    probe.digest.clear();
    let bytes = serde_json::to_vec(&probe).expect("repair plan is serializable");
    sha256_hex(&bytes)
}

/// Whether the plan's stored digest still matches its content.
pub fn plan_digest_matches(plan: &RepairPlan) -> bool {
    !plan.digest.is_empty() && compute_plan_digest(plan) == plan.digest
}

/// State of one file operation inside a repair transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationState {
    /// The operation has not been published yet.
    #[serde(rename = "pending")]
    Pending,
    /// The post-image was published at this path.
    #[serde(rename = "published")]
    Published,
    /// The preimage was restored at this path.
    #[serde(rename = "rolled_back")]
    RolledBack,
    /// The rollback refused to overwrite an unrelated edit at this path.
    #[serde(rename = "conflict")]
    Conflict,
}

/// One file operation of a repair transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalOperation {
    /// Workspace-relative path the operation touches.
    pub path: String,
    /// State of the operation.
    pub state: OperationState,
}

/// Status of a repair transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalStatus {
    /// The transaction is running; the workspace may be half-mutated.
    #[serde(rename = "in_progress")]
    InProgress,
    /// Every planned post-image is published.
    #[serde(rename = "canonical_committed")]
    CanonicalCommitted,
    /// Every planned post-image is published, but the index refresh failed.
    #[serde(rename = "canonical_committed/index_pending")]
    CanonicalCommittedIndexPending,
    /// Every published post-image was restored from its preimage.
    #[serde(rename = "rolled_back")]
    RolledBack,
    /// Canonical preimages are restored, but derived state still needs rebuilding.
    #[serde(rename = "rolled_back/index_pending")]
    RolledBackIndexPending,
    /// The rollback has a verified diagnostic snapshot, but the legacy index
    /// is not addressable and ordinary reads and writes remain blocked.
    #[serde(rename = "rolled_back/index_blocked")]
    RolledBackIndexBlocked,
    /// The rollback refused to overwrite an unrelated edit.
    #[serde(rename = "conflict")]
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairPhase {
    Applying,
    RollingBack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalMove {
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyDiagnosticReceipt {
    pub prior_manifest_sha256: String,
    /// Versioned inventory digest; unversioned historical receipts fail closed.
    pub source_digest: String,
    pub snapshot_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub successor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyRepairPredecessor {
    pub transaction_id: String,
    pub blocked_manifest_sha256: String,
}

/// The recovery journal of a repair transaction.
///
/// The journal is the durable record of a transaction: the preimage of every
/// file the transaction touches, the post-image it planned to publish, and
/// the state of every operation. It is written and fsynced before the first
/// mutation, so a crash always leaves a journal that can restore the
/// workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairJournal {
    /// Absent for legacy manifests; version 2 permits absent file images.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    /// Identifier of the transaction; also the name of its journal directory.
    pub transaction_id: String,
    /// Digest over every field of this journal except the digest itself.
    pub manifest_sha256: String,
    /// Workspace-relative path → SHA-256 of the preimage at that path.
    #[serde(deserialize_with = "deserialize_unique_hashes")]
    pub preimages: BTreeMap<String, String>,
    /// Workspace-relative path → SHA-256 of the post-image published there.
    #[serde(deserialize_with = "deserialize_unique_hashes")]
    pub planned_postimages: BTreeMap<String, String>,
    /// One entry per file operation.
    pub operations: Vec<JournalOperation>,
    /// Status of the transaction.
    pub status: JournalStatus,
    /// Direction is durable before the first rollback mutation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<RepairPhase>,
    /// Pairs source removal with its preceding destination creation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub moves: Vec<JournalMove>,
    /// Durable evidence that a rollback was published as a diagnostic view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_diagnostic: Option<LegacyDiagnosticReceipt>,
    /// Earlier blocked repairs explicitly superseded by this transaction.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub legacy_predecessors: Vec<LegacyRepairPredecessor>,
}

fn deserialize_unique_hashes<'de, D>(deserializer: D) -> std::result::Result<BTreeMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct UniqueHashes;
    impl<'de> serde::de::Visitor<'de> for UniqueHashes {
        type Value = BTreeMap<String, String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a map of unique paths to hashes")
        }

        fn visit_map<M>(self, mut map: M) -> std::result::Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            let mut values = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, String>()? {
                if values.insert(key.clone(), value).is_some() {
                    return Err(serde::de::Error::custom(format!("duplicate journal path: {key}")));
                }
            }
            Ok(values)
        }
    }
    deserializer.deserialize_map(UniqueHashes)
}

impl RepairJournal {
    /// Whether the transaction has published every planned post-image.
    pub fn is_committed(&self) -> bool {
        matches!(
            self.status,
            JournalStatus::CanonicalCommitted | JournalStatus::CanonicalCommittedIndexPending
        )
    }

    /// Whether the transaction is unfinished and therefore blocks the next
    /// mutation until it is recovered.
    pub fn is_unfinished(&self) -> bool {
        matches!(self.status, JournalStatus::InProgress | JournalStatus::Conflict | JournalStatus::CanonicalCommittedIndexPending | JournalStatus::RolledBackIndexPending | JournalStatus::RolledBackIndexBlocked)
    }
}

/// Compute the digest of a journal over every field it carries.
pub fn compute_journal_digest(journal: &RepairJournal) -> String {
    let mut probe = journal.clone();
    probe.manifest_sha256.clear();
    let bytes = serde_json::to_vec(&probe).expect("repair journal is serializable");
    sha256_hex(&bytes)
}

/// Whether the journal's stored digest still matches its content.
pub fn journal_digest_matches(journal: &RepairJournal) -> bool {
    !journal.manifest_sha256.is_empty() && compute_journal_digest(journal) == journal.manifest_sha256
}

/// The three hashes of one document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentHashes {
    /// SHA-256 of the whole file.
    pub file_sha256: String,
    /// SHA-256 of the body.
    pub body_sha256: String,
    /// SHA-256 of the frontmatter region.
    pub frontmatter_sha256: String,
}

/// Split a document into its frontmatter region and its body.
///
/// The frontmatter region is every leading YAML block the core frontmatter
/// parser would read — including the nested wrapper block the projection
/// layer writes around an already-rendered artifact. The body is everything
/// after it, byte for byte.
pub fn split_document(content: &str) -> Result<DocumentParts> {
    let blocks = consumed_blocks(content)?;
    let last = blocks.last().expect("consumed_blocks returns at least one block");
    let lines = line_ranges(content);
    let close_end = lines[last.close_line].1;
    let frontmatter_end = close_end;
    let body_start = next_non_blank_start(content, &lines, last.close_line);
    Ok(DocumentParts {
        frontmatter: content[..frontmatter_end].to_string(),
        body: content[body_start..].to_string(),
    })
}

/// The two halves of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentParts {
    /// The leading frontmatter region, verbatim.
    pub frontmatter: String,
    /// Everything after the frontmatter region, verbatim.
    pub body: String,
}

/// Hash a document at file, body and frontmatter granularity.
pub fn hash_document(content: &str) -> Result<DocumentHashes> {
    let parts = split_document(content)?;
    Ok(DocumentHashes {
        file_sha256: sha256_hex(content.as_bytes()),
        body_sha256: sha256_hex(parts.body.as_bytes()),
        frontmatter_sha256: sha256_hex(parts.frontmatter.as_bytes()),
    })
}

/// One entry of the `links` frontmatter sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkEntry {
    /// Zero-based index of the entry inside the `links` sequence. This is
    /// the identity a plan uses to name an occurrence, and it is stable
    /// because the sequence is never reordered by the repair.
    pub occurrence: usize,
    /// The `target` value of the entry, verbatim.
    pub target: String,
    /// The `relation` value of the entry, empty when the entry has none.
    pub relation: String,
}

/// The `links` entries of a document's effective frontmatter, in order.
///
/// The effective frontmatter is the merged view the core parser produces:
/// the first leading block that declares `links` wins, exactly as
/// `parse_frontmatter` merges nested wrappers.
pub fn link_entries(content: &str) -> Result<Vec<LinkEntry>> {
    let frontmatter = effective_frontmatter(content)?;
    let Some(value) = frontmatter.get("links") else {
        return Ok(Vec::new());
    };
    let sequence = value.as_sequence().ok_or_else(|| anyhow::anyhow!("links must be a sequence"))?;
    sequence.iter().enumerate().map(|(occurrence, item)| {
        let map = item.as_mapping().ok_or_else(|| anyhow::anyhow!("links[{occurrence}] must be a mapping"))?;
        let target = map.get(serde_yaml::Value::String("target".to_string()))
            .and_then(serde_yaml::Value::as_str)
            .map(str::trim)
            .filter(|target| !target.is_empty())
            .ok_or_else(|| anyhow::anyhow!("links[{occurrence}] has no string target"))?;
        let relation = match map.get(serde_yaml::Value::String("relation".to_string())) {
            None | Some(serde_yaml::Value::Null) => "",
            Some(value) => value.as_str().ok_or_else(|| anyhow::anyhow!("links[{occurrence}] has a non-string relation"))?.trim(),
        };
        Ok(LinkEntry { occurrence, target: target.to_string(), relation: relation.to_string() })
    }).collect()
}

/// The `parent_epic` value of a document's effective frontmatter.
pub fn parent_epic_target(content: &str) -> Result<Option<String>> {
    let frontmatter = effective_frontmatter(content)?;
    match frontmatter.get("parent_epic") {
        None | Some(serde_yaml::Value::Null) => Ok(None),
        Some(value) => {
            let target = value.as_str().ok_or_else(|| anyhow::anyhow!("parent_epic must be a string"))?.trim();
            if target.is_empty() { bail!("parent_epic must not be empty"); }
            Ok(Some(target.to_string()))
        }
    }
}

/// Set the canonical key of a document by rewriting its `slug:` field.
///
/// The field is replaced in place when the document already declares one —
/// including when the declaration sits in the nested wrapper block, which
/// is where the projection layer puts it. A document that declares no slug
/// gets one, appended to the outermost frontmatter block so the merged view
/// reads it back.
///
/// The body and every other byte of the file are returned unchanged.
pub fn set_frontmatter_slug(content: &str, new_key: &str) -> Result<String> {
    if new_key.trim().is_empty() {
        bail!("canonical key must not be empty");
    }
    let blocks = consumed_blocks(content)?;
    let lines = line_ranges(content);
    match block_with_key(content, &blocks, "slug") {
        Some(index) => {
            let block = &blocks[index];
            let Some(line) = top_level_key_line(content, &lines, block, "slug") else {
                bail!("document declares a slug key but no slug line was found");
            };
            let (start, end) = scalar_range(content, &lines[line], "slug".len())?;
            let mut out = String::with_capacity(content.len() + new_key.len());
            out.push_str(&content[..start]);
            out.push_str(new_key);
            out.push_str(&content[end..]);
            Ok(out)
        }
        None => {
            let block = &blocks[0];
            let insert_at = block.yaml.1;
            let yaml = &content[block.yaml.0..block.yaml.1];
            let insertion = if yaml.trim().is_empty() {
                format!("slug: {new_key}")
            } else {
                format!("\nslug: {new_key}")
            };
            let mut out = String::with_capacity(content.len() + insertion.len());
            out.push_str(&content[..insert_at]);
            out.push_str(&insertion);
            out.push_str(&content[insert_at..]);
            Ok(out)
        }
    }
}

/// Rewrite the target of one `links` entry.
///
/// `occurrence` is the index of the entry inside the `links` sequence, as
/// reported by [`link_entries`]. The relation, the entry order and every
/// other byte of the document stay untouched.
pub fn set_link_target(content: &str, occurrence: usize, new_target: &str) -> Result<String> {
    let mut expected = link_entries(content)?;
    let old = expected.get(occurrence)
        .ok_or_else(|| anyhow::anyhow!("links occurrence {occurrence} does not exist"))?
        .target.clone();
    validate_rewrite_target(new_target)?;
    let blocks = consumed_blocks(content)?;
    let Some(index) = block_with_key(content, &blocks, "links") else {
        bail!("document has no links frontmatter");
    };
    let block = &blocks[index];
    let lines = line_ranges(content);
    let Some(links_line) = top_level_key_line(content, &lines, block, "links") else {
        bail!("document has no links frontmatter");
    };
    let items = link_item_ranges(content, &lines, block, links_line);
    let Some(item) = items.get(occurrence).copied() else {
        bail!(
            "links occurrence {occurrence} does not exist (document has {})",
            items.len()
        );
    };
    let Some((start, end)) = item_scalar_range(content, &lines, item, "target") else {
        bail!("links occurrence {occurrence} has no target field");
    };
    let raw: String = serde_yaml::from_str(&content[start..end])
        .with_context(|| format!("links occurrence {occurrence} is not an inline string"))?;
    if raw.trim() != old {
        bail!("links occurrence {occurrence} cannot be safely located in the source");
    }
    let mut out = String::with_capacity(content.len() + new_target.len());
    out.push_str(&content[..start]);
    out.push_str(new_target);
    out.push_str(&content[end..]);
    expected[occurrence].target = new_target.to_string();
    if link_entries(&out)? != expected {
        bail!("rewriting links changed another reference");
    }
    Ok(out)
}

/// Rewrite the `parent_epic` value of a document.
pub fn set_parent_epic(content: &str, new_target: &str) -> Result<String> {
    let old = parent_epic_target(content)?.ok_or_else(|| anyhow::anyhow!("document has no parent_epic"))?;
    validate_rewrite_target(new_target)?;
    let blocks = consumed_blocks(content)?;
    let Some(index) = block_with_key(content, &blocks, "parent_epic") else {
        bail!("document has no parent_epic frontmatter");
    };
    let block = &blocks[index];
    let lines = line_ranges(content);
    let Some(line) = top_level_key_line(content, &lines, block, "parent_epic") else {
        bail!("document has no parent_epic frontmatter");
    };
    let (start, end) = scalar_range(content, &lines[line], "parent_epic".len())?;
    let raw: String = serde_yaml::from_str(&content[start..end])
        .context("parent_epic is not an inline string")?;
    if raw.trim() != old {
        bail!("parent_epic cannot be safely located in the source");
    }
    let mut out = String::with_capacity(content.len() + new_target.len());
    out.push_str(&content[..start]);
    out.push_str(new_target);
    out.push_str(&content[end..]);
    if parent_epic_target(&out)?.as_deref() != Some(new_target) {
        bail!("rewriting parent_epic did not preserve the reference");
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// One leading frontmatter block of a document.
#[derive(Debug, Clone)]
struct FrontmatterBlock {
    /// Line index of the opening `---` delimiter.
    open_line: usize,
    /// Line index of the closing `---` delimiter.
    close_line: usize,
    /// Byte range of the YAML text between the two delimiters.
    yaml: (usize, usize),
}

/// Byte ranges of every line of `content`, excluding the line terminators.
fn line_ranges(content: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let bytes = content.as_bytes();
    let mut start = 0usize;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            out.push((start, index));
            start = index + 1;
        }
    }
    if start < bytes.len() {
        out.push((start, bytes.len()));
    }
    out
}

/// Whether a line is a frontmatter delimiter.
fn is_delimiter(text: &str) -> bool {
    text.trim() == "---"
}

/// The YAML mapping a frontmatter block declares.
///
/// A block that does not parse as a mapping declares nothing; the core
/// parser would reject the whole document in that case, and the caller
/// treats the block as key-less.
fn yaml_map(text: &str) -> BTreeMap<String, serde_yaml::Value> {
    serde_yaml::from_str::<BTreeMap<String, serde_yaml::Value>>(text).unwrap_or_default()
}

/// The leading frontmatter blocks a document carries, in document order.
///
/// The rule mirrors the core frontmatter parser exactly: the first block is
/// always read; a following block is read only while the previous one
/// declared both `id` and `kind` and the following block declares the same
/// `id`/`kind` (or declares neither). A block that declares `id` and `kind`
/// continues the chain; a block that does not ends it.
fn consumed_blocks(content: &str) -> Result<Vec<FrontmatterBlock>> {
    let lines = line_ranges(content);
    let mut open: Option<usize> = None;
    for index in 0..lines.len() {
        let text = &content[lines[index].0..lines[index].1];
        if text.trim().is_empty() {
            continue;
        }
        if is_delimiter(text) {
            open = Some(index);
        }
        break;
    }
    let Some(open) = open else {
        bail!("document has no frontmatter block");
    };

    let mut blocks: Vec<FrontmatterBlock> = Vec::new();
    let mut current_open = open;
    loop {
        let close = ((current_open + 1)..lines.len())
            .find(|index| is_delimiter(&content[lines[*index].0..lines[*index].1]))
            .ok_or_else(|| anyhow::anyhow!("frontmatter block is not closed"))?;
        let block = FrontmatterBlock {
            open_line: current_open,
            close_line: close,
            yaml: yaml_range(content, &lines, current_open, close),
        };
        let outer = yaml_map(&content[block.yaml.0..block.yaml.1]);
        blocks.push(block);

        if !(outer.contains_key("id") && outer.contains_key("kind")) {
            break;
        }
        let Some(next) = next_non_blank_line(content, &lines, close + 1) else {
            break;
        };
        if !is_delimiter(&content[lines[next].0..lines[next].1]) {
            break;
        }
        let Some(inner_close) = ((next + 1)..lines.len())
            .find(|index| is_delimiter(&content[lines[*index].0..lines[*index].1]))
        else {
            break;
        };
        let inner_start = (lines[next].1 + 1).min(content.len());
        let mut inner_end = lines[inner_close].0;
        if inner_end > inner_start && content.as_bytes()[inner_end - 1] == b'\n' {
            inner_end -= 1;
        }
        let inner = yaml_map(&content[inner_start..inner_end]);
        // A nested block that declares a different identity is a different
        // document, not a wrapper: the chain ends before it.
        if let Some(id) = inner.get("id")
            && let Some(outer_id) = outer.get("id")
            && id != outer_id
        {
            break;
        }
        if let Some(kind) = inner.get("kind")
            && let Some(outer_kind) = outer.get("kind")
            && kind != outer_kind
        {
            break;
        }
        current_open = next;
    }
    Ok(blocks)
}

/// The byte range of the YAML text between the delimiters of a block.
fn yaml_range(
    content: &str,
    lines: &[(usize, usize)],
    open_line: usize,
    close_line: usize,
) -> (usize, usize) {
    let yaml_start = (lines[open_line].1 + 1).min(content.len());
    let mut yaml_end = lines[close_line].0;
    if yaml_end > yaml_start && content.as_bytes()[yaml_end - 1] == b'\n' {
        yaml_end -= 1;
    }
    (yaml_start.min(yaml_end), yaml_end)
}

/// The first non-blank line at or after `from`.
fn next_non_blank_line(content: &str, lines: &[(usize, usize)], from: usize) -> Option<usize> {
    (from..lines.len()).find(|index| {
        let (start, end) = lines[*index];
        !content[start..end].trim().is_empty()
    })
}

/// The byte offset where the body starts after the block that ends at
/// `close_line`.
fn next_non_blank_start(content: &str, lines: &[(usize, usize)], close_line: usize) -> usize {
    for index in (close_line + 1)..lines.len() {
        let (start, end) = lines[index];
        if !content[start..end].trim().is_empty() {
            return start;
        }
    }
    content.len()
}

/// The first leading block that declares `key`.
///
/// The merged view takes the value from the first block that declares the
/// key, so this is the block whose bytes the effective value comes from.
fn block_with_key(
    content: &str,
    blocks: &[FrontmatterBlock],
    key: &str,
) -> Option<usize> {
    blocks
        .iter()
        .position(|block| yaml_map(&content[block.yaml.0..block.yaml.1]).contains_key(key))
}

/// The line of a top-level `key:` declaration inside a block.
///
/// Only a line that starts at column zero counts: an indented `key:` belongs
/// to a nested mapping and is not the document's key.
fn top_level_key_line(
    content: &str,
    lines: &[(usize, usize)],
    block: &FrontmatterBlock,
    key: &str,
) -> Option<usize> {
    let prefix = format!("{key}:");
    ((block.open_line + 1)..block.close_line).find(|index| {
        let (start, end) = lines[*index];
        content[start..end].starts_with(&prefix)
    })
}

/// The byte range of the scalar value that follows `key:` on a line.
fn scalar_range(
    content: &str,
    line: &(usize, usize),
    key_len: usize,
) -> Result<(usize, usize)> {
    let (start, end) = *line;
    inline_scalar_range(content, start + key_len + 1, end)
}

fn validate_rewrite_target(target: &str) -> Result<()> {
    if target.is_empty() || !target.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_') {
        bail!("unsafe replacement reference {target:?}");
    }
    Ok(())
}

fn inline_scalar_range(content: &str, start: usize, end: usize) -> Result<(usize, usize)> {
    let bytes = content.as_bytes();
    let mut start = start;
    while start < end && matches!(bytes[start], b' ' | b'\t') {
        start += 1;
    }
    if start == end { bail!("missing inline scalar"); }
    let mut cursor = start;
    let value_end;
    if matches!(bytes[start], b'\'' | b'"') {
        let quote = bytes[start];
        cursor += 1;
        loop {
            if cursor >= end { bail!("unterminated quoted scalar"); }
            if quote == b'"' && bytes[cursor] == b'\\' {
                cursor += 2;
                continue;
            }
            if bytes[cursor] == quote {
                if quote == b'\'' && cursor + 1 < end && bytes[cursor + 1] == quote {
                    cursor += 2;
                    continue;
                }
                value_end = cursor + 1;
                break;
            }
            cursor += 1;
        }
        let rest = content[value_end..end].trim();
        if !rest.is_empty() && !rest.starts_with('#') {
            bail!("unsupported trailing YAML after scalar");
        }
    } else {
        while cursor < end {
            if bytes[cursor] == b'#' && cursor > start && matches!(bytes[cursor - 1], b' ' | b'\t') {
                break;
            }
            cursor += 1;
        }
        let mut stop = cursor;
        while stop > start && matches!(bytes[stop - 1], b' ' | b'\t' | b'\r') { stop -= 1; }
        value_end = stop;
    }
    Ok((start, value_end))
}

/// The line ranges of every item of the `links` sequence of a block.
fn link_item_ranges(
    content: &str,
    lines: &[(usize, usize)],
    block: &FrontmatterBlock,
    links_line: usize,
) -> Vec<(usize, usize)> {
    let mut items: Vec<(usize, usize)> = Vec::new();
    let mut current: Option<usize> = None;
    for index in (links_line + 1)..block.close_line {
        let (start, end) = lines[index];
        let text = &content[start..end];
        if text.trim().is_empty() {
            continue;
        }
        let trimmed = text.trim_start();
        if trimmed.starts_with('-') {
            if let Some(open) = current.take() {
                items.push((open, index));
            }
            current = Some(index);
            continue;
        }
        // A column-zero line that is not an item ends the sequence.
        if !text.starts_with(' ') && !text.starts_with('\t') {
            if let Some(open) = current.take() {
                items.push((open, index));
            }
            break;
        }
    }
    if let Some(open) = current {
        items.push((open, block.close_line));
    }
    items
}

/// The byte range of `key:`'s value inside one `links` item.
///
/// The key may sit on the item's first line (`- target: X`) or on one of
/// its continuation lines.
fn item_scalar_range(
    content: &str,
    lines: &[(usize, usize)],
    item: (usize, usize),
    key: &str,
) -> Option<(usize, usize)> {
    let prefix = format!("{key}:");
    for index in item.0..item.1 {
        let (start, end) = lines[index];
        let text = &content[start..end];
        // The key may sit on the item's first line, after the `- ` marker, or
        // on a continuation line, after the indentation. The offset is the
        // position of the key **in the original line**: the length of the
        // stripped text is not that offset, and the indentation has to be
        // counted too.
        let indent = text.len() - text.trim_start().len();
        let body = &text[indent..];
        let offset = if let Some(stripped) = body.strip_prefix("- ") {
            indent + (body.len() - stripped.len())
        } else if body.starts_with('-') {
            indent + (body.len() - body.trim_start_matches('-').len())
        } else {
            indent
        };
        let candidate = &text[offset..];
        if candidate.starts_with(&prefix) {
            return inline_scalar_range(content, start + offset + prefix.len(), end).ok();
        }
    }
    None
}


/// The merged frontmatter of a document, as the core parser reads it.
pub fn effective_frontmatter(content: &str) -> Result<BTreeMap<String, serde_yaml::Value>> {
    let (fm, _body) = parse_frontmatter(content)?;
    Ok(fm)
}

// ---------------------------------------------------------------------------
// The census
// ---------------------------------------------------------------------------

/// The kinds the identity census walks.
///
/// Includes memory and brownfield documents, which can also hold references.
pub const REPAIR_KINDS: &[ArtifactKind] = &[
    ArtifactKind::Prd,
    ArtifactKind::Rfc,
    ArtifactKind::Adr,
    ArtifactKind::Epic,
    ArtifactKind::Spec,
    ArtifactKind::ProblemCard,
    ArtifactKind::SolutionPortfolio,
    ArtifactKind::EvidencePack,
    ArtifactKind::Note,
    ArtifactKind::RefreshReport,
    ArtifactKind::Memory,
    ArtifactKind::UseCase,
    ArtifactKind::Glossary,
    ArtifactKind::Invariant,
    ArtifactKind::Scenario,
    ArtifactKind::Hypothesis,
    ArtifactKind::DomainModel,
];

/// One document of the workspace, as the census saw it.
#[derive(Debug, Clone)]
pub struct CensusDocument {
    /// Workspace-relative path of the document (forward slashes).
    pub relative_path: String,
    /// The display alias the document carries (`PRD-074`).
    pub display_id: String,
    /// The kind the document belongs to.
    pub kind: ArtifactKind,
    /// The document's title; empty when the document declares none.
    pub title: String,
    /// The canonical key the document claims today.
    pub canonical_key: String,
    /// The three hashes of the file.
    pub hashes: DocumentHashes,
    /// The `links` entries of the effective frontmatter, in order.
    pub links: Vec<LinkEntry>,
    /// The `parent_epic` value of the effective frontmatter.
    pub parent_epic: Option<String>,
}

/// An identity-bearing sidecar included in the inventory.
#[derive(Debug, Clone)]
pub struct CensusSidecar {
    pub relative_path: String,
    pub file_sha256: String,
    pub references: Vec<String>,
}

/// The census of a workspace.
#[derive(Debug, Clone)]
pub struct Census {
    /// Every document of the workspace, sorted by relative path.
    pub documents: Vec<CensusDocument>,
    /// Identity-bearing claims, phase states and receipts.
    pub sidecars: Vec<CensusSidecar>,
    /// Versioned digest over every document and identity-bearing sidecar.
    pub inventory_digest: String,
}

/// Walk the workspace and census every document.
///
/// The census reads and nothing else: it never writes, and it refuses to
/// follow a symlink. A document whose frontmatter cannot be parsed is an
/// error rather than a gap — a partial document set would make every
/// decision taken over it partial too.
pub fn census_workspace(forgeplan_dir: &Path) -> Result<Census> {
    let mut documents: Vec<CensusDocument> = Vec::new();
    for name in crate::workspace::ARTIFACT_DIRS {
        let kind = REPAIR_KINDS.iter().find(|kind| kind.dir_name() == *name);
        let dir = forgeplan_dir.join(name);
        let meta = match fs::symlink_metadata(&dir) {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error).with_context(|| format!("inspect {}", dir.display())),
        };
        if meta.file_type().is_symlink() {
            bail!(
                "refusing to census a symlinked kind directory: {}",
                dir.display()
            );
        }
        if !meta.is_dir() {
            bail!("refusing non-directory artifact kind path {}", dir.display());
        }
        for item in walkdir::WalkDir::new(&dir).follow_links(false).sort_by_file_name() {
            let item = item.with_context(|| format!("walk kind directory {}", dir.display()))?;
            if item.file_type().is_symlink() {
                bail!("refusing to census symlinked artifact path {}", item.path().display());
            }
            if item.file_type().is_dir() {
                continue;
            }
            if !item.file_type().is_file() {
                bail!("refusing non-regular artifact path {}", item.path().display());
            }
            let path = item.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
                continue;
            }
            let relative_path = workspace_relative_path(forgeplan_dir, path)?;
            let content = String::from_utf8(read_regular_file(path)?)
                .with_context(|| format!("read document {relative_path}"))?;
            let parsed_kind = if let Some(kind) = kind {
                kind.clone()
            } else {
                let fm = effective_frontmatter(&content)
                    .with_context(|| format!("parse frontmatter {relative_path}"))?;
                fm.get("id").and_then(|value| value.as_str())
                    .and_then(ArtifactKind::from_display_id)
                    .or_else(|| fm.get("kind").and_then(|value| value.as_str())
                        .and_then(|value| value.parse().ok()))
                    .ok_or_else(|| anyhow::anyhow!("unsupported document kind in {relative_path}"))?
            };
            documents.push(census_document(relative_path, &parsed_kind, &content)?);
        }
    }
    documents.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let sidecars = census_sidecars(forgeplan_dir)?;
    let inventory_digest = inventory_digest(&documents, &sidecars);
    Ok(Census {
        documents,
        sidecars,
        inventory_digest,
    })
}

fn collect_sidecar_strings(value: &serde_yaml::Value, output: &mut Vec<String>) {
    match value {
        serde_yaml::Value::String(text) => output.push(text.trim().to_string()),
        serde_yaml::Value::Sequence(items) => {
            for item in items { collect_sidecar_strings(item, output); }
        }
        serde_yaml::Value::Mapping(items) => {
            for (key, item) in items {
                collect_sidecar_strings(key, output);
                collect_sidecar_strings(item, output);
            }
        }
        serde_yaml::Value::Tagged(tagged) => collect_sidecar_strings(&tagged.value, output),
        _ => {}
    }
}

fn census_sidecars(forgeplan_dir: &Path) -> Result<Vec<CensusSidecar>> {
    let mut sidecars = Vec::new();
    for name in ["claims", "state", "trash"] {
        let root = forgeplan_dir.join(name);
        match fs::symlink_metadata(&root) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error).with_context(|| format!("inspect {}", root.display())),
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() =>
                bail!("refusing unsafe sidecar directory {}", root.display()),
            Ok(_) => {}
        }
        for item in walkdir::WalkDir::new(&root).follow_links(false) {
            let item = item.with_context(|| format!("walk {}", root.display()))?;
            if item.file_type().is_symlink() {
                bail!("refusing symlinked sidecar {}", item.path().display());
            }
            if item.file_type().is_dir() { continue; }
            if !item.file_type().is_file() {
                bail!("refusing non-regular sidecar {}", item.path().display());
            }
            let path = item.path();
            let extension = path.extension().and_then(|value| value.to_str()).unwrap_or("");
            if !matches!(extension, "yaml" | "yml" | "json" | "md") { continue; }
            let relative_path = workspace_relative_path(forgeplan_dir, path)?;
            let bytes = read_regular_file(path)?;
            let mut references = Vec::new();
            if let Some(stem) = path.file_stem().and_then(|value| value.to_str()) {
                references.push(stem.to_string());
            }
            if extension == "md" {
                let text = std::str::from_utf8(&bytes)?;
                let frontmatter = effective_frontmatter(text)
                    .with_context(|| format!("parse historical sidecar {relative_path}"))?;
                for value in frontmatter.values() { collect_sidecar_strings(value, &mut references); }
            } else {
                let value: serde_yaml::Value = serde_yaml::from_slice(&bytes)
                    .with_context(|| format!("parse sidecar {relative_path}"))?;
                collect_sidecar_strings(&value, &mut references);
            }
            sidecars.push(CensusSidecar {
                relative_path,
                file_sha256: sha256_hex(&bytes),
                references,
            });
        }
    }
    sidecars.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(sidecars)
}

/// A versioned, domain-separated digest over length-framed paths and hashes.
/// Document and sidecar counts and record tags separate the two namespaces.
pub fn compute_inventory_digest(documents: &[CensusDocument]) -> String {
    inventory_digest(documents, &[])
}

fn inventory_digest(documents: &[CensusDocument], sidecars: &[CensusSidecar]) -> String {
    fn field(hasher: &mut Sha256, bytes: &[u8]) {
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    let mut hasher = Sha256::new();
    hasher.update(b"forgeplan.identity-repair.inventory.v2\0");
    hasher.update((documents.len() as u64).to_be_bytes());
    for document in documents {
        hasher.update([0]);
        field(&mut hasher, document.relative_path.as_bytes());
        field(&mut hasher, document.hashes.file_sha256.as_bytes());
    }
    hasher.update((sidecars.len() as u64).to_be_bytes());
    for sidecar in sidecars {
        hasher.update([1]);
        field(&mut hasher, sidecar.relative_path.as_bytes());
        field(&mut hasher, sidecar.file_sha256.as_bytes());
    }
    format!("v2:{}", crate::constellation::hex_encode(&hasher.finalize()))
}

/// The documents that claim the same canonical key.
///
/// A group of two or more documents is a collision: several files answer to
/// one key. The key is the group's name.
pub fn collision_groups(census: &Census) -> BTreeMap<String, Vec<String>> {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for document in &census.documents {
        groups
            .entry(document.canonical_key.clone())
            .or_default()
            .push(document.relative_path.clone());
    }
    groups.retain(|_, members| members.len() > 1);
    for members in groups.values_mut() {
        members.sort();
    }
    groups
}

/// Read one document of the census.
fn census_document(
    relative_path: String,
    kind: &ArtifactKind,
    content: &str,
) -> Result<CensusDocument> {
    let frontmatter =
        effective_frontmatter(content).with_context(|| format!("parse frontmatter {relative_path}"))?;
    let display_id = frontmatter
        .get("id")
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| file_stem(&relative_path));
    let title = frontmatter
        .get("title")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();
    // The kind follows the display id: the frontmatter spells a kind in
    // more than one way ("evidence", "evid"), and the display id is the
    // spelling every corpus agrees on.
    let kind = ArtifactKind::from_display_id(&display_id).unwrap_or_else(|| kind.clone());
    let canonical_key = canonical_key_from_document(&display_id, content);
    let hashes = hash_document(content)?;
    let links = link_entries(content)?;
    let parent_epic = parent_epic_target(content)?;
    Ok(CensusDocument {
        relative_path,
        display_id,
        kind,
        title,
        canonical_key,
        hashes,
        links,
        parent_epic,
    })
}

/// The file name of a workspace-relative path, without its extension.
fn file_stem(relative_path: &str) -> String {
    relative_path
        .rsplit('/')
        .next()
        .and_then(|name| name.strip_suffix(".md"))
        .unwrap_or_default()
        .to_string()
}

/// The workspace-relative path of a document.
///
/// A path that leaves the workspace, or that climbs out of it, is refused:
/// the repair only ever touches documents inside the workspace it was
/// planned against.
fn workspace_relative_path(forgeplan_dir: &Path, path: &Path) -> Result<String> {
    let relative = path.strip_prefix(forgeplan_dir).map_err(|_| {
        anyhow::anyhow!(
            "refusing a document outside the workspace: {}",
            path.display()
        )
    })?;
    let mut parts: Vec<String> = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().to_string()),
            _ => bail!(
                "refusing a document path that leaves the workspace: {}",
                path.display()
            ),
        }
    }
    if parts.is_empty() {
        bail!("refusing a document path without a file name");
    }
    Ok(parts.join("/"))
}

// ---------------------------------------------------------------------------
// Reviewed decisions
// ---------------------------------------------------------------------------

/// A reviewed decision about one inbound reference.
///
/// A decision names the document that holds the reference, the field and the
/// target text it carries, and the document the reference denotes. It is the
/// only way a reference that matches several candidates becomes resolved:
/// the census never guesses which of two documents a relation meant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceDecision {
    /// Workspace-relative path of the document that holds the reference.
    pub source_path: String,
    /// Index within the field, bound to the reviewed source bytes.
    #[serde(default)]
    pub occurrence: Option<usize>,
    /// SHA-256 of the complete reviewed source document.
    #[serde(default)]
    pub source_sha256: Option<String>,
    /// The field the reference lives in: `links` or `parent_epic`.
    pub field: String,
    /// The reference target as the document writes it today.
    pub from: String,
    /// Workspace-relative path of the document the reference denotes.
    pub target_path: String,
}

/// A file of reviewed decisions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionFile {
    /// Version of the decision file format. Always `1` today.
    #[serde(default)]
    pub version: u32,
    /// The decisions the file carries.
    #[serde(default)]
    pub decisions: Vec<ReferenceDecision>,
}

/// Read a file of reviewed decisions.
pub fn read_decision_file(path: &Path) -> Result<DecisionFile> {
    let bytes =
        fs::read(path).with_context(|| format!("read decisions file {}", path.display()))?;
    let file: DecisionFile = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse decisions file {}", path.display()))?;
    if file.version != 1 {
        bail!("unsupported decision file version {}", file.version);
    }
    for decision in &file.decisions {
        if decision.occurrence.is_none() || decision.source_sha256.as_deref().is_none_or(str::is_empty) {
            bail!("decision for {} requires occurrence and source_sha256", decision.source_path);
        }
    }
    Ok(file)
}

/// Merge reviewed decisions, the later ones winning.
///
/// Decisions are keyed by source, field, occurrence and reviewed source hash.
pub fn merge_decisions(
    base: &[ReferenceDecision],
    extra: &[ReferenceDecision],
) -> Vec<ReferenceDecision> {
    let mut merged: BTreeMap<(String, String, Option<usize>, Option<String>, String), ReferenceDecision> = BTreeMap::new();
    for decision in base.iter().chain(extra.iter()) {
        let key = (
            decision.source_path.clone(),
            decision.field.clone(),
            decision.occurrence,
            decision.source_sha256.clone(),
            decision.from.clone(),
        );
        merged.insert(key, decision.clone());
    }
    merged.into_values().collect()
}

/// The pilot corpus review record.
///
/// The record is the inbound reference mapping measured on the pilot copies
/// on 2026-10-28 and reviewed by the owner. Every entry is justified by the
/// content or the stated history of that specific relation — never by a
/// number, a title alone, or a modification time.
///
/// These historical records predate occurrence and source hashes. They are
/// retained as review context but cannot resolve a reference until re-reviewed
/// with both fields populated for the current document.
///
/// A `LazyLock` rather than a `const`: the records own `String`s, and
/// `String::to_string` is not a const fn.
pub static PILOT_REFERENCE_DECISIONS: LazyLock<Vec<ReferenceDecision>> = LazyLock::new(|| {
    vec![
    // -- ForgePlanWeb ------------------------------------------------------
    ReferenceDecision {
        source_path: "evidence/EVID-004-claude-md-baseline-guides-skaffold-smoke-pass-all-5-sc-validated.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-001".to_string(),
        target_path: "prds/PRD-001-bootstrap-claude-md-baseline-and-add-methodological-guides.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-005-safety-hardening-init-force-hook-red-line-8-windows-ci-fully-green-for-the-first-time.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-001".to_string(),
        target_path: "prds/PRD-001-bootstrap-claude-md-baseline-and-add-methodological-guides.md".to_string(),
    },
    ReferenceDecision {
        source_path: "rfcs/RFC-015-mosaic-split-tree-layout-engine-for-multi-graph-canvas.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-016".to_string(),
        target_path: "prds/PRD-016-multi-graph-mosaic-dashboard-with-persistent-layout.md".to_string(),
    },
    ReferenceDecision {
        source_path: "rfcs/RFC-015-identity-aware-route-structured-snapshot-error-surface.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-016".to_string(),
        target_path: "prds/PRD-016-prob-060-slug-canonical-identity-in-web-snapshot-error-surfacing.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-020-mosaic-layout-engine-unit-tests-svelte-check-pass.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-016".to_string(),
        target_path: "prds/PRD-016-multi-graph-mosaic-dashboard-with-persistent-layout.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-020-mosaic-layout-engine-unit-tests-svelte-check-pass.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "RFC-015".to_string(),
        target_path: "rfcs/RFC-015-mosaic-split-tree-layout-engine-for-multi-graph-canvas.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-021-prd-016-verified-typecheck-clean-179-tests-pass-identifier-guard-correct.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-016".to_string(),
        target_path: "prds/PRD-016-prob-060-slug-canonical-identity-in-web-snapshot-error-surfacing.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-021-prd-016-verified-typecheck-clean-179-tests-pass-identifier-guard-correct.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "RFC-015".to_string(),
        target_path: "rfcs/RFC-015-identity-aware-route-structured-snapshot-error-surface.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-040-code-review-of-commit-0101560-rfc-015-d-4-pass.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "RFC-015".to_string(),
        target_path: "rfcs/RFC-015-identity-aware-route-structured-snapshot-error-surface.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-040-code-review-of-commit-0101560-rfc-015-d-4-pass.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-016".to_string(),
        target_path: "prds/PRD-016-prob-060-slug-canonical-identity-in-web-snapshot-error-surfacing.md".to_string(),
    },
    // -- ForgePlanMarketplace ----------------------------------------------
    ReferenceDecision {
        source_path: "adrs/ADR-007-determinism-is-behavioral-implementation-passes-the-frozen-conformance-corpus-sha-not-bitwise.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-deterministic-sdd-tdd-pipeline-frozen-language-neutral-conformance-corpus-drives-multi-language-tdd.md".to_string(),
    },
    ReferenceDecision {
        source_path: "adrs/ADR-008-spec-driven-development-on-the-spec-kind-is-the-pipeline-spine-frozen-corpus-conformance-is-an-optional-gate.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-spec-driven-pipeline-openspec-speckit-style-on-the-spec-kind-optional-conformance-vectors-for-pure-cores.md".to_string(),
    },
    // The owner decision of 2026-10-28: NOTE-019 points at the
    // deterministic PRD-072.
    ReferenceDecision {
        source_path: "notes/NOTE-019-research-context-deterministic-sdd-tdd-openspec-speckit-3-layer-orchestration-playbook-readiness.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-deterministic-sdd-tdd-pipeline-frozen-language-neutral-conformance-corpus-drives-multi-language-tdd.md".to_string(),
    },
    ReferenceDecision {
        source_path: "notes/NOTE-020-spec-authoring-template-light-path-requirements-scenario-behavioral-contract-contracts-optional-conformance-vectors.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-spec-driven-pipeline-openspec-speckit-style-on-the-spec-kind-optional-conformance-vectors-for-pure-cores.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-121-adi-decision-adopt-spec-driven-light-pipeline-demote-frozen-corpus-cross-language-gate-to-optional.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-spec-driven-pipeline-openspec-speckit-style-on-the-spec-kind-optional-conformance-vectors-for-pure-cores.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-123-artifact-health-audit-prd-072-concerns-related-artifacts-table-incomplete-premature-supersede-claim.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-spec-driven-pipeline-openspec-speckit-style-on-the-spec-kind-optional-conformance-vectors-for-pure-cores.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-127-code-review-of-prd-072-phase-2-feat-spec-author-skill-light-pipeline-concerns.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-spec-driven-pipeline-openspec-speckit-style-on-the-spec-kind-optional-conformance-vectors-for-pure-cores.md".to_string(),
    },
    ReferenceDecision {
        source_path: "specs/SPEC-001-semver-compare-behavioral-contract-embedded-conformance-corpus-first-deterministic-sdd-tdd-increment.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-deterministic-sdd-tdd-pipeline-frozen-language-neutral-conformance-corpus-drives-multi-language-tdd.md".to_string(),
    },
    ReferenceDecision {
        source_path: "specs/SPEC-001-semver-compare-requirements-scenarios-behavioral-contract-optional-conformance-vectors-light-path-demonstrator.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-spec-driven-pipeline-openspec-speckit-style-on-the-spec-kind-optional-conformance-vectors-for-pure-cores.md".to_string(),
    },
    ReferenceDecision {
        source_path: "notes/NOTE-022-test-plan-for-spec-001-semver-compare-precedence-total-order-invariants-language-neutral.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "SPEC-001".to_string(),
        target_path: "specs/SPEC-001-semver-compare-requirements-scenarios-behavioral-contract-optional-conformance-vectors-light-path-demonstrator.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-119-spike-result-semver-compare-ts-py-18-18-vectors-4-4-invariants-10-10-determinism-negative-control-caught.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "SPEC-001".to_string(),
        target_path: "specs/SPEC-001-semver-compare-behavioral-contract-embedded-conformance-corpus-first-deterministic-sdd-tdd-increment.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-119-spike-result-semver-compare-on-ts-py-go-rust-20-20-vectors-4-language-equivalence-per-language-negative-control.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "SPEC-001".to_string(),
        target_path: "specs/SPEC-001-semver-compare-behavioral-contract-embedded-conformance-corpus-first-deterministic-sdd-tdd-increment.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-125-adi-spec-001-reframe-maintain-in-place-not-supersede-removed-0-corpus-retained-as-optional.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "SPEC-001".to_string(),
        target_path: "specs/SPEC-001-semver-compare-behavioral-contract-embedded-conformance-corpus-first-deterministic-sdd-tdd-increment.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-126-artifact-health-audit-spec-001-light-path-reframe-concerns-scenario-adequacy-gaps-maintain-in-place-agreed.md".to_string(),
        occurrence: None, source_sha256: None,
        field: "links".to_string(),
        from: "SPEC-001".to_string(),
        target_path: "specs/SPEC-001-semver-compare-requirements-scenarios-behavioral-contract-optional-conformance-vectors-light-path-demonstrator.md".to_string(),
    },
    ]
});

// ---------------------------------------------------------------------------
// The plan
// ---------------------------------------------------------------------------

/// Build the version-1 repair plan of a workspace.
///
/// The plan names every document whose canonical key changes, the key it
/// receives, and every inbound reference that points at one of those
/// documents. A reference that matches several candidates stays unresolved
/// unless a reviewed decision names the document it denotes — the plan
/// never decides that on its own.
///
/// The plan is a *plan*: building it changes nothing in the workspace.
pub fn build_repair_plan(
    forgeplan_dir: &Path,
    census: &Census,
    decisions: &[ReferenceDecision],
) -> Result<RepairPlan> {
    let workspace_path = fs::canonicalize(forgeplan_dir)
        .with_context(|| format!("resolve workspace {}", forgeplan_dir.display()))?;

    let groups = collision_groups(census);

    // Every member of every group receives its own new key. The key is
    // minted here and reserved in the plan: the apply never mints another.
    let mut new_key_by_path: BTreeMap<String, String> = BTreeMap::new();
    let mut old_key_by_path: BTreeMap<String, String> = BTreeMap::new();
    let taken: BTreeSet<String> = census
        .documents
        .iter()
        .map(|document| document.canonical_key.clone())
        .collect();
    for (old_key, members) in &groups {
        for member in members {
            let document = census
                .documents
                .iter()
                .find(|document| document.relative_path == *member)
                .ok_or_else(|| {
                    anyhow::anyhow!("the collision group names an unknown document: {member}")
                })?;
            let mut new_key = mint_artifact_key(&document.kind, &document.title)
                .with_context(|| format!("mint a canonical key for {member}"))?;
            while taken.contains(&new_key)
                || new_key_by_path.values().any(|key| key == &new_key)
            {
                new_key = mint_artifact_key(&document.kind, &document.title)
                    .with_context(|| format!("mint a canonical key for {member}"))?;
            }
            new_key_by_path.insert(member.clone(), new_key);
            old_key_by_path.insert(member.clone(), old_key.clone());
        }
    }

    // The sources: the documents whose canonical key changes.
    let mut sources: Vec<PlannedSource> = Vec::new();
    for document in &census.documents {
        let Some(new_key) = new_key_by_path.get(&document.relative_path) else {
            continue;
        };
        sources.push(PlannedSource {
            relative_path: document.relative_path.clone(),
            file_sha256: document.hashes.file_sha256.clone(),
            body_sha256: document.hashes.body_sha256.clone(),
            frontmatter_sha256: document.hashes.frontmatter_sha256.clone(),
            old_key: old_key_by_path
                .get(&document.relative_path)
                .cloned()
                .unwrap_or_default(),
            new_key: new_key.clone(),
            // The repair rewrites a document in place: an existing file
            // keeps its path (SPEC-009 "New file naming"), and the body,
            // the display number and every other byte stay as they were.
            proposed_path: document.relative_path.clone(),
        });
    }

    // The references: the inbound references that point at a changed key.
    let mut references: Vec<PlannedReference> = Vec::new();
    for document in &census.documents {
        for entry in &document.links {
            if let Some(reference) = plan_reference(
                census,
                &new_key_by_path,
                decisions,
                &document.relative_path,
                "links",
                entry.occurrence,
                &entry.target,
            ) {
                references.push(reference);
            }
        }
        if let Some(target) = &document.parent_epic {
            if let Some(reference) = plan_reference(
                census,
                &new_key_by_path,
                decisions,
                &document.relative_path,
                "parent_epic",
                0,
                target,
            ) {
                references.push(reference);
            }
        }
    }
    let (sidecar_moves, sidecar_conflicts) = match plan_sidecar_moves(forgeplan_dir, census, &sources) {
        Ok(moves) => (moves, Vec::new()),
        Err(error) => (Vec::new(), vec![format!("{error:#}")]),
    };

    let mut plan = RepairPlan {
        version: REPAIR_PLAN_VERSION,
        workspace_path: workspace_path.to_string_lossy().to_string(),
        git: detect_git_identity(forgeplan_dir)?,
        inventory_digest: census.inventory_digest.clone(),
        sources,
        references,
        sidecar_moves,
        sidecar_conflicts,
        digest: String::new(),
    };
    plan.digest = compute_plan_digest(&plan);
    Ok(plan)
}

fn sidecar_postimage(content: &str, old_key: &str, new_key: &str) -> Result<Vec<u8>> {
    validate_rewrite_target(new_key)?;
    let value: serde_yaml::Value = serde_yaml::from_str(content)?;
    let mapping = value.as_mapping().ok_or_else(|| anyhow::anyhow!("sidecar is not a YAML mapping"))?;
    let key = serde_yaml::Value::String("canonical_key".into());
    match mapping.get(&key) {
        None | Some(serde_yaml::Value::Null) => {}
        Some(serde_yaml::Value::String(current)) if current == old_key => {}
        _ => bail!("sidecar canonical_key does not match its owner"),
    }
    let lines = line_ranges(content);
    let matching: Vec<_> = lines.iter().filter(|(start, end)| {
        content[*start..*end].starts_with("canonical_key:")
    }).collect();
    let updated = if mapping.contains_key(&key) {
        if matching.len() != 1 {
            bail!("cannot safely locate canonical_key scalar");
        }
        let line = matching[0];
        let (start, end) = scalar_range(content, line, "canonical_key".len())?;
        format!("{}{new_key}{}", &content[..start], &content[end..])
    } else {
        if !matching.is_empty() || content.lines().any(|line| line.trim() == "...") {
            bail!("cannot safely append canonical_key");
        }
        let separator = if content.ends_with('\n') { "" } else { "\n" };
        format!("{content}{separator}canonical_key: {new_key}\n")
    };
    let mut expected = mapping.clone();
    expected.insert(key, serde_yaml::Value::String(new_key.to_string()));
    let actual: serde_yaml::Value = serde_yaml::from_str(&updated)?;
    if actual != serde_yaml::Value::Mapping(expected) {
        bail!("sidecar rewrite changed fields other than canonical_key");
    }
    Ok(updated.into_bytes())
}

fn plan_sidecar_moves(forgeplan_dir: &Path, census: &Census, sources: &[PlannedSource]) -> Result<Vec<PlannedSidecarMove>> {
    let changed: BTreeSet<&str> = sources.iter().map(|source| source.relative_path.as_str()).collect();
    let touches = |reference: &str| {
        sources.iter().any(|source| crate::artifact::sidecar::sidecar_file_stem(&source.old_key) == reference)
            || resolve_reference(census, reference).iter().any(|document| changed.contains(document.relative_path.as_str()))
    };
    let mut moves = Vec::new();
    let mut destinations = BTreeSet::new();
    for sidecar in &census.sidecars {
        if !sidecar.references.iter().any(|reference| touches(reference)) {
            continue;
        }
        let path = Path::new(&sidecar.relative_path);
        let mut components = path.components();
        let directory = components.next().and_then(|part| match part {
            Component::Normal(value) => value.to_str(),
            _ => None,
        });
        if !matches!(directory, Some("claims" | "state"))
            || !matches!(components.next(), Some(Component::Normal(_)))
            || components.next().is_some()
            || path.extension().and_then(|ext| ext.to_str()) != Some("yaml")
        {
            bail!("identity-bearing sidecar {} cannot be attributed safely", sidecar.relative_path);
        }
        let absolute = resolve_document_path(forgeplan_dir, &sidecar.relative_path)?;
        let bytes = read_regular_file(&absolute)?;
        if sha256_hex(&bytes) != sidecar.file_sha256 {
            bail!("sidecar {} changed during planning", sidecar.relative_path);
        }
        let content = std::str::from_utf8(&bytes)?;
        let value: serde_yaml::Value = serde_yaml::from_str(content)?;
        let mapping = value.as_mapping().ok_or_else(|| anyhow::anyhow!("sidecar {} is not a mapping", sidecar.relative_path))?;
        let id_field = if directory == Some("claims") { "id" } else { "artifact_id" };
        let id_key = serde_yaml::Value::String(id_field.to_string());
        let canonical_key = serde_yaml::Value::String("canonical_key".to_string());
        let id = mapping.get(&id_key).and_then(|value| value.as_str())
            .ok_or_else(|| anyhow::anyhow!("sidecar {} has no {id_field}", sidecar.relative_path))?;
        let owners: Vec<_> = census.documents.iter().filter(|document| {
            normalized_display_alias(&document.display_id) == normalized_display_alias(id)
                && normalized_display_alias(id).is_some()
        }).collect();
        if owners.len() != 1 {
            bail!("sidecar {} has ambiguous owner {id:?}", sidecar.relative_path);
        }
        let owner = owners[0];
        let source = sources.iter().find(|source| source.relative_path == owner.relative_path)
            .ok_or_else(|| anyhow::anyhow!("sidecar {} references another changed identity", sidecar.relative_path))?;
        let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or("");
        let valid_stems = crate::artifact::sidecar::legacy_sidecar_file_stems(&source.old_key, &owner.display_id);
        if stem != crate::artifact::sidecar::sidecar_file_stem(&source.old_key)
            && !valid_stems.iter().any(|candidate| candidate == stem)
        {
            bail!("sidecar {} has an unexpected identity filename", sidecar.relative_path);
        }
        let mut other = mapping.clone();
        other.remove(&id_key);
        other.remove(&canonical_key);
        let mut other_strings = Vec::new();
        collect_sidecar_strings(&serde_yaml::Value::Mapping(other), &mut other_strings);
        if other_strings.iter().any(|reference| touches(reference)) {
            bail!("sidecar {} has an additional identity reference requiring review", sidecar.relative_path);
        }
        let postimage = sidecar_postimage(content, &source.old_key, &source.new_key)
            .with_context(|| format!("rewrite sidecar {}", sidecar.relative_path))?;
        let destination_path = format!("{}/{}.yaml", directory.unwrap(), crate::artifact::sidecar::sidecar_file_stem(&source.new_key));
        let destination = resolve_document_path(forgeplan_dir, &destination_path)?;
        if read_optional_file(&destination)?.is_some() || !destinations.insert(destination_path.clone()) {
            bail!("sidecar destination {destination_path} is already occupied");
        }
        moves.push(PlannedSidecarMove {
            source_path: sidecar.relative_path.clone(),
            destination_path,
            owner_path: owner.relative_path.clone(),
            display_id: id.to_string(),
            display_id_field: id_field.to_string(),
            canonical_field_present: mapping.contains_key(&canonical_key),
            canonical_field_before: mapping.get(&canonical_key).and_then(|value| value.as_str()).map(str::to_string),
            old_key: source.old_key.clone(),
            new_key: source.new_key.clone(),
            source_sha256: sidecar.file_sha256.clone(),
            postimage_sha256: sha256_hex(&postimage),
        });
    }
    Ok(moves)
}

/// The plan entry for one inbound reference, when the reference points at a
/// document whose canonical key changes.
///
/// A reference that no document answers to is not a reference to a changed
/// key and stays out of the plan. A reference that several documents answer
/// to is listed with every candidate and an empty target: it stays
/// unresolved until a review decides, and the apply refuses it.
fn plan_reference(
    census: &Census,
    new_key_by_path: &BTreeMap<String, String>,
    decisions: &[ReferenceDecision],
    source_path: &str,
    field: &str,
    occurrence: usize,
    target: &str,
) -> Option<PlannedReference> {
    let candidates = resolve_reference(census, target);
    if !candidates.iter().any(|candidate| new_key_by_path.contains_key(&candidate.relative_path)) {
        return None;
    }
    let mut mapping: BTreeMap<String, String> = BTreeMap::new();
    for candidate in &candidates {
        let key = new_key_by_path
            .get(&candidate.relative_path)
            .cloned()
            .unwrap_or_else(|| candidate.canonical_key.clone());
        mapping.insert(candidate.relative_path.clone(), key);
    }
    // The attributed document: the only candidate, or the one a reviewed
    // decision names.
    let attributed: Option<&CensusDocument> = if candidates.len() == 1 {
        Some(candidates[0])
    } else {
        decisions
            .iter()
            .find(|decision| {
                decision.source_path == source_path
                    && decision.field == field
                    && decision.from == target
                    && decision.occurrence == Some(occurrence)
                    && census.documents.iter().any(|document|
                        document.relative_path == source_path
                        && decision.source_sha256.as_deref() == Some(document.hashes.file_sha256.as_str()))
            })
            .and_then(|decision| {
                candidates
                    .iter()
                    .find(|candidate| candidate.relative_path == decision.target_path)
            })
            .copied()
    };
    let to = attributed
        .and_then(|candidate| mapping.get(&candidate.relative_path).cloned())
        .unwrap_or_default();
    Some(PlannedReference {
        source_path: source_path.to_string(),
        field: field.to_string(),
        occurrence,
        from: target.to_string(),
        to,
        mapping,
    })
}

/// The documents a reference target may denote.
///
/// ADR-030's resolution order: an exact canonical-key match wins, and
/// several documents claiming one key are a conflict rather than a winner.
/// Numeric display aliases are normalized in the same way as canonical
/// resolution. A numeric legacy key cannot mask another document's alias.
fn resolve_reference<'a>(census: &'a Census, target: &str) -> Vec<&'a CensusDocument> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let by_key: Vec<&CensusDocument> = census
        .documents
        .iter()
        .filter(|document| document.canonical_key == trimmed)
        .collect();
    let normalized = normalized_display_alias(trimmed);
    let aliases: Vec<&CensusDocument> = census.documents.iter().filter(|document| {
        document.display_id == trimmed || match (&normalized, normalized_display_alias(&document.display_id)) {
            (Some(expected), Some(actual)) => *expected == actual,
            _ => false,
        }
    }).collect();
    if by_key.len() > 1 || (!by_key.is_empty() && normalized.is_none()) {
        return by_key;
    }
    if let Some(key_match) = by_key.first() {
        let mut all = by_key.clone();
        all.extend(aliases.into_iter().filter(|candidate| candidate.relative_path != key_match.relative_path));
        return all;
    }
    aliases
}

fn normalized_display_alias(value: &str) -> Option<(String, u32)> {
    let (prefix, number) = value.split_once('-')?;
    if prefix.is_empty() || number.is_empty()
        || !prefix.bytes().all(|byte| byte.is_ascii_alphabetic())
        || !number.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let kind = ArtifactKind::from_slug_prefix(&prefix.to_ascii_lowercase())?;
    Some((kind.prefix().to_ascii_uppercase(), number.parse().ok()?))
}

/// Write a repair plan as JSON.
///
/// The plan is written atomically: a temporary file in the same directory,
/// fsynced, then renamed over the target. A plan file a reader finds
/// half-written would be a plan nobody can trust.
pub fn write_plan_json(plan: &RepairPlan, path: &Path) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(plan)?;
    bytes.push(b'\n');
    write_file_atomically(path, &bytes)
}

/// Read a repair plan from JSON.
///
/// A plan whose digest does not match its content was edited after it was
/// written, and is refused: the apply trusts the plan as a whole or not at
/// all.
pub fn read_plan_json(path: &Path) -> Result<RepairPlan> {
    let bytes = fs::read(path).with_context(|| format!("read repair plan {}", path.display()))?;
    let plan: RepairPlan = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse repair plan {}", path.display()))?;
    if plan.version != REPAIR_PLAN_VERSION {
        bail!("unsupported repair plan version {}", plan.version);
    }
    if !valid_inventory_digest(&plan.inventory_digest) {
        bail!("unsupported repair inventory digest version");
    }
    if !plan_digest_matches(&plan) {
        bail!(
            "the repair plan {} does not match its digest — the file was edited after it was written",
            path.display()
        );
    }
    Ok(plan)
}

// ---------------------------------------------------------------------------
// The transaction
// ---------------------------------------------------------------------------

/// The outcome of a successful apply.
#[derive(Debug, Clone)]
pub struct ApplyOutcome {
    /// The transaction identifier; also the name of its journal directory.
    pub transaction_id: String,
    /// The journal directory, absolute.
    pub journal_dir: PathBuf,
    /// The workspace-relative paths the transaction published.
    pub published: Vec<String>,
    /// The status the journal carries at the end of the transaction.
    pub status: JournalStatus,
}

/// The outcome of a rollback.
#[derive(Debug, Clone)]
pub struct RollbackOutcome {
    /// The transaction identifier.
    pub transaction_id: String,
    /// The journal directory, absolute.
    pub journal_dir: PathBuf,
    /// The paths whose preimage this rollback restored.
    pub restored: Vec<String>,
    /// The paths the rollback refused to overwrite.
    pub conflicts: Vec<String>,
    /// The status the journal carries at the end of the rollback.
    pub status: JournalStatus,
}

/// One file a transaction rewrites.
struct FileOperation {
    /// Workspace-relative path of the document.
    path: String,
    /// The absolute path of the document on disk.
    absolute: PathBuf,
    /// None when the path did not exist before this transaction.
    preimage: Option<Vec<u8>>,
    /// None when this transaction removes the path.
    postimage: Option<Vec<u8>>,
}

/// The directory the recovery journals live in.
pub fn repair_journal_root(forgeplan_dir: &Path) -> PathBuf {
    forgeplan_dir.join("identity-repair")
}

/// The directory of one transaction's journal.
pub fn repair_journal_dir(forgeplan_dir: &Path, transaction_id: &str) -> PathBuf {
    repair_journal_root(forgeplan_dir).join(transaction_id)
}

fn repair_staging_root(forgeplan_dir: &Path) -> PathBuf {
    forgeplan_dir.join("identity-repair-staging")
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedRepairReceipt {
    version: u32,
    transaction_id: String,
    workspace_path: String,
    plan_digest: String,
    manifest_digest: String,
}

fn validate_transaction_id(id: &str) -> Result<()> {
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
        bail!("invalid repair transaction identifier: {id:?}");
    }
    Ok(())
}

fn require_directory(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path).with_context(|| format!("inspect {}", path.display()))?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        bail!("expected a non-symlink directory: {}", path.display());
    }
    Ok(())
}

fn read_regular_file(path: &Path) -> Result<Vec<u8>> {
    let meta = fs::symlink_metadata(path).with_context(|| format!("inspect {}", path.display()))?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        bail!("expected a non-symlink regular file: {}", path.display());
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(path).with_context(|| format!("open {}", path.display()))?;
    if !file.metadata()?.is_file() {
        bail!("expected a regular file: {}", path.display());
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).with_context(|| format!("read {}", path.display()))?;
    Ok(bytes)
}

fn valid_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn valid_inventory_digest(digest: &str) -> bool {
    digest.strip_prefix("v2:").is_some_and(valid_hash)
}

fn validate_journal(forgeplan_dir: &Path, id: &str, journal: &RepairJournal) -> Result<()> {
    validate_transaction_id(id)?;
    if journal.transaction_id != id || !journal_digest_matches(journal) {
        bail!("repair journal {id} has an invalid identity or digest");
    }
    if !matches!(journal.version, None | Some(2)) {
        bail!("repair journal {id} has an unsupported version");
    }
    if (journal.version.is_none() && (journal.phase.is_some() || !journal.moves.is_empty()))
        || (journal.version == Some(2) && journal.phase.is_none())
    {
        bail!("repair journal {id} has an invalid phase or version");
    }
    if journal.status == JournalStatus::RolledBackIndexBlocked && journal.legacy_diagnostic.is_none() {
        bail!("repair journal {id} has no diagnostic receipt");
    }
    if let Some(receipt) = &journal.legacy_diagnostic {
        if !valid_hash(&receipt.prior_manifest_sha256)
            || !valid_inventory_digest(&receipt.source_digest)
            || !valid_hash(&receipt.snapshot_digest)
            || receipt.successor.as_ref().is_some_and(|successor| successor == id || validate_transaction_id(successor).is_err())
        {
            bail!("repair journal {id} has an invalid diagnostic receipt");
        }
    }
    let mut predecessor_ids = BTreeSet::new();
    for predecessor in &journal.legacy_predecessors {
        if predecessor.transaction_id == id
            || validate_transaction_id(&predecessor.transaction_id).is_err()
            || !valid_hash(&predecessor.blocked_manifest_sha256)
            || !predecessor_ids.insert(&predecessor.transaction_id)
        {
            bail!("repair journal {id} has an invalid predecessor");
        }
    }
    if journal.version == Some(2) {
        let valid_phase = match journal.status {
            JournalStatus::CanonicalCommitted | JournalStatus::CanonicalCommittedIndexPending =>
                journal.phase == Some(RepairPhase::Applying),
            JournalStatus::RolledBack | JournalStatus::RolledBackIndexPending | JournalStatus::RolledBackIndexBlocked =>
                journal.phase == Some(RepairPhase::RollingBack),
            JournalStatus::InProgress | JournalStatus::Conflict => true,
        };
        if !valid_phase {
            bail!("repair journal {id} has an inconsistent phase");
        }
    }
    if journal.operations.is_empty() {
        bail!("repair journal {id} has no operations");
    }
    let mut paths = BTreeSet::new();
    for operation in &journal.operations {
        let path = Path::new(&operation.path);
        let mut components = path.components();
        let kind = components.next().and_then(|c| match c {
            Component::Normal(name) => name.to_str(),
            _ => None,
        });
        let remaining: Vec<_> = components.collect();
        let normal_parts = !remaining.is_empty()
            && remaining.iter().all(|part| matches!(part, Component::Normal(_)));
        let document_path = normal_parts
            && kind.is_some_and(|name| crate::workspace::ARTIFACT_DIRS.contains(&name))
            && path.extension().and_then(|ext| ext.to_str()) == Some("md");
        let sidecar_path = journal.version == Some(2)
            && matches!(kind, Some("state" | "claims"))
            && normal_parts && remaining.len() == 1
            && path.extension().and_then(|ext| ext.to_str()) == Some("yaml");
        if !(document_path || sidecar_path)
            || operation.path.contains('\\')
            || !paths.insert(operation.path.as_str())
        {
            bail!("repair journal {id} contains an unsafe or duplicate path: {}", operation.path);
        }
        resolve_document_path(forgeplan_dir, &operation.path)?;
        let pre = journal.preimages.get(&operation.path);
        let post = journal.planned_postimages.get(&operation.path);
        if pre.is_some_and(|value| !valid_hash(value))
            || post.is_some_and(|value| !valid_hash(value))
            || (pre.is_none() && post.is_none())
            || (journal.version.is_none() && (pre.is_none() || post.is_none()))
            || (document_path && (pre.is_none() || post.is_none()))
            || (sidecar_path && pre.is_some() && post.is_some())
        {
            bail!("repair journal {id} has missing or invalid hashes for {}", operation.path);
        }
        let valid_state = match journal.status {
            JournalStatus::CanonicalCommitted | JournalStatus::CanonicalCommittedIndexPending =>
                operation.state == OperationState::Published,
            JournalStatus::RolledBack | JournalStatus::RolledBackIndexPending | JournalStatus::RolledBackIndexBlocked =>
                operation.state == OperationState::RolledBack,
            JournalStatus::InProgress | JournalStatus::Conflict => true,
        };
        if !valid_state {
            bail!("repair journal {id} has inconsistent operation state for {}", operation.path);
        }
    }
    if journal.preimages.keys().chain(journal.planned_postimages.keys())
        .any(|path| !paths.contains(path.as_str()))
    {
        bail!("repair journal {id} has inconsistent operation and hash sets");
    }
    let mut endpoints = BTreeSet::new();
    for pair in &journal.moves {
        let source = journal.operations.iter().position(|op| op.path == pair.source);
        let destination = journal.operations.iter().position(|op| op.path == pair.destination);
        if pair.source == pair.destination
            || !endpoints.insert(pair.source.as_str())
            || !endpoints.insert(pair.destination.as_str())
            || !matches!((source, destination), (Some(s), Some(d)) if d + 1 == s)
            || Path::new(&pair.source).parent() != Path::new(&pair.destination).parent()
            || !journal.preimages.contains_key(&pair.source)
            || journal.planned_postimages.contains_key(&pair.source)
            || journal.preimages.contains_key(&pair.destination)
            || !journal.planned_postimages.contains_key(&pair.destination)
        {
            bail!("repair journal {id} has an invalid move pair");
        }
    }
    for path in &paths {
        if journal.preimages.contains_key(*path) != journal.planned_postimages.contains_key(*path)
            && !endpoints.contains(path)
        {
            bail!("repair journal {id} has an unpaired create or remove: {path}");
        }
    }
    Ok(())
}

fn read_valid_journal(forgeplan_dir: &Path, id: &str) -> Result<RepairJournal> {
    validate_transaction_id(id)?;
    let root = repair_journal_root(forgeplan_dir);
    require_directory(&root)?;
    let dir = repair_journal_dir(forgeplan_dir, id);
    require_directory(&dir)?;
    let bytes = read_regular_file(&dir.join("manifest.json"))?;
    let journal: RepairJournal = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse repair journal {id}"))?;
    validate_journal(forgeplan_dir, id, &journal)?;
    Ok(journal)
}
/// Inspect a named recovery journal under the workspace lock.
pub fn inspect_repair_journal(forgeplan_dir: &Path, id: &str) -> Result<RepairJournal> {
    validate_transaction_id(id)?;
    let _guard = acquire_repair_lock(forgeplan_dir)?;
    check_repair_journals(forgeplan_dir, Some(id))?;
    let journal = read_valid_journal(forgeplan_dir, id)?;
    let dir = repair_journal_dir(forgeplan_dir, id);
    require_directory(&dir.join("preimages"))?;
    for (path, expected) in &journal.preimages {
        let bytes = read_regular_file(&preimage_file(&dir, path))?;
        if sha256_hex(&bytes) != *expected {
            bail!("repair journal {id} has a damaged preimage for {path}");
        }
    }
    validate_postimage_blobs(&dir, &journal)?;
    if let Some(successor) = journal.legacy_diagnostic.as_ref().and_then(|receipt| receipt.successor.as_deref()) {
        validate_linked_pair(forgeplan_dir, id, successor)?;
    }
    Ok(journal)
}


fn validate_linked_pair(
    forgeplan_dir: &Path,
    old_id: &str,
    new_id: &str,
) -> Result<(RepairJournal, RepairJournal)> {
    let workspace = fs::canonicalize(forgeplan_dir)
        .with_context(|| format!("resolve workspace {}", forgeplan_dir.display()))?;
    validate_linked_pair_inner(&workspace, old_id, new_id, &mut BTreeSet::new())
}

fn validate_linked_pair_inner(
    forgeplan_dir: &Path,
    old_id: &str,
    new_id: &str,
    visited: &mut BTreeSet<String>,
) -> Result<(RepairJournal, RepairJournal)> {
    if visited.len() >= 256 || !visited.insert(old_id.to_string()) || visited.contains(new_id) {
        bail!("repair predecessor chain is cyclic or exceeds its limit");
    }
    if old_id == new_id {
        bail!("a repair transaction cannot precede itself");
    }
    let old = read_valid_journal(forgeplan_dir, old_id)?;
    let new = read_valid_journal(forgeplan_dir, new_id)?;
    let receipt = old.legacy_diagnostic.as_ref()
        .ok_or_else(|| anyhow::anyhow!("linked rollback {old_id} has no diagnostic receipt"))?;
    if !matches!(old.status, JournalStatus::RolledBackIndexBlocked | JournalStatus::RolledBack)
        || new.legacy_predecessors.len() != 1
        || new.legacy_predecessors[0].transaction_id != old_id
        || receipt.successor.as_deref().is_some_and(|id| id != new_id)
    {
        bail!("invalid linked repair journals {old_id} and {new_id}");
    }
    let mut origin = old.clone();
    origin.status = JournalStatus::RolledBackIndexBlocked;
    origin.legacy_diagnostic.as_mut().expect("receipt checked").successor = None;
    if compute_journal_digest(&origin) != new.legacy_predecessors[0].blocked_manifest_sha256 {
        bail!("linked rollback {old_id} does not match the predecessor digest");
    }
    crate::artifact::legacy_diagnostic::verify_stored_snapshot_integrity(
        forgeplan_dir, old_id, &receipt.snapshot_digest, &receipt.prior_manifest_sha256, &receipt.source_digest,
    )?;
    for journal in [&old, &new] {
        let dir = repair_journal_dir(forgeplan_dir, &journal.transaction_id);
        require_directory(&dir.join("preimages"))?;
        for (path, expected) in &journal.preimages {
            if sha256_hex(&read_regular_file(&preimage_file(&dir, path))?) != *expected {
                bail!("linked repair {} has damaged preimage {path}", journal.transaction_id);
            }
        }
        validate_postimage_blobs(&dir, journal)?;
    }
    if receipt.successor.is_none() {
        if old.status != JournalStatus::RolledBackIndexBlocked
            || !matches!(new.status, JournalStatus::InProgress | JournalStatus::RolledBack)
        {
            bail!("incomplete repair chain is not recoverable in this state");
        }
        for operation in &new.operations {
            let absolute = resolve_document_path(forgeplan_dir, &operation.path)?;
            let current = read_optional_file(&absolute)?.map(|bytes| sha256_hex(&bytes));
            if current.as_ref() != new.preimages.get(&operation.path) {
                bail!("unbound successor {new_id} has already changed {}", operation.path);
            }
        }
    } else if new.status == JournalStatus::RolledBackIndexBlocked
        && new.legacy_diagnostic.as_ref().and_then(|receipt| receipt.successor.as_ref()).is_some()
    {
        let successor = new.legacy_diagnostic.as_ref().and_then(|receipt| receipt.successor.as_deref()).unwrap();
        validate_linked_pair_inner(forgeplan_dir, new_id, successor, visited)?;
    } else if !matches!(new.status, JournalStatus::CanonicalCommitted | JournalStatus::RolledBack) {
        let dir = repair_journal_dir(forgeplan_dir, new_id);
        for operation in &new.operations {
            let absolute = resolve_document_path(forgeplan_dir, &operation.path)?;
            let current = read_optional_file(&absolute)?.map(|bytes| sha256_hex(&bytes));
            let before = new.preimages.get(&operation.path);
            let after = new.planned_postimages.get(&operation.path);
            if current.as_ref() != before && current.as_ref() != after {
                bail!("linked successor {new_id} has an unrelated image at {}", operation.path);
            }
            if before.is_none() && current.is_some() && current.as_ref() == after {
                require_owned_create(&dir, &operation.path, &absolute)?;
            }
        }
    }
    Ok((old, new))
}

/// Validate a single linked pair under the workspace lock; no other unfinished
/// journal may be bypassed by this admission.
pub(crate) fn validate_linked_repair_journals(
    forgeplan_dir: &Path,
    old_id: &str,
    new_id: &str,
) -> Result<(String, String)> {
    let (old, new) = validate_linked_pair(forgeplan_dir, old_id, new_id)?;
    check_repair_journals(forgeplan_dir, Some(new_id))?;
    Ok((old.manifest_sha256, new.manifest_sha256))
}

/// Inspect both journals while holding their exact linked repair guard.
pub async fn inspect_linked_repair_journals(
    workspace: &Path,
    old_id: &str,
    new_id: &str,
) -> Result<(RepairJournal, RepairJournal)> {
    let guard = crate::workspace::lock::acquire_linked_repair_workspace_lock(workspace, old_id, new_id).await?;
    let workspace = fs::canonicalize(workspace)?;
    let (old, new) = validate_linked_pair(&workspace, old_id, new_id)?;
    if guard.linked_legacy_repair_digests_for(&workspace, old_id, new_id)
        != Some((old.manifest_sha256.as_str(), new.manifest_sha256.as_str()))
    {
        bail!("linked repair changed during inspection");
    }
    Ok((old, new))
}
/// Called only while holding the workspace lock.
pub(crate) fn check_repair_journals(forgeplan_dir: &Path, recovering: Option<&str>) -> Result<()> {
    check_repair_journals_inner(forgeplan_dir, recovering, None)
}

fn check_repair_journals_inner(forgeplan_dir: &Path, recovering: Option<&str>, staged_id: Option<&str>) -> Result<()> {
    let staging = repair_staging_root(forgeplan_dir);
    match fs::symlink_metadata(&staging) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).with_context(|| format!("inspect {}", staging.display())),
        Ok(_) => {
            require_directory(&staging)?;
            for entry in fs::read_dir(&staging)? {
                let entry = entry?;
                let name = entry.file_name().into_string()
                    .map_err(|_| anyhow::anyhow!("non-UTF-8 repair staging entry"))?;
                if staged_id != Some(name.as_str()) {
                    bail!("unresolved repair staging entry {name}; explicit preparation recovery or diagnosis is required");
                }
                validate_transaction_id(&name)?;
                require_directory(&entry.path())?;
            }
        }
    }
    let root = repair_journal_root(forgeplan_dir);
    match fs::symlink_metadata(&root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).with_context(|| format!("inspect {}", root.display())),
        Ok(_) => require_directory(&root)?,
    }
    for entry in fs::read_dir(&root).with_context(|| format!("read {}", root.display()))? {
        let entry = entry?;
        let name = entry.file_name().into_string().map_err(|_| anyhow::anyhow!("non-UTF-8 repair journal entry"))?;
        if name == "decisions.json" {
            read_regular_file(&entry.path())?;
            continue;
        }
        validate_transaction_id(&name)?;
        let journal = read_valid_journal(forgeplan_dir, &name)?;
        let dir = entry.path();
        require_directory(&dir.join("preimages"))?;
        if recovering == Some(name.as_str()) {
            continue;
        }
        for (path, expected) in &journal.preimages {
            let bytes = read_regular_file(&preimage_file(&dir, path))?;
            if sha256_hex(&bytes) != *expected {
                bail!("repair journal {name} has a damaged preimage for {path}");
            }
        }
        validate_postimage_blobs(&dir, &journal)?;
        if journal.status == JournalStatus::RolledBack {
            if let Some(successor) = journal.legacy_diagnostic.as_ref().and_then(|receipt| receipt.successor.as_deref()) {
                validate_linked_pair(forgeplan_dir, &name, successor)?;
            }
        }
        // A previous rename may have succeeded even if fsync reported failure.
        sync_dir(&dir)?;
        if journal.is_unfinished() {
            let linked = if journal.status == JournalStatus::RolledBackIndexBlocked {
                match recovering {
                    Some(successor) => validate_linked_pair(forgeplan_dir, &name, successor).is_ok(),
                    None => false,
                }
            } else {
                false
            };
            if !linked {
                bail!("repair transaction {name} requires recovery or index reconciliation ({:?})", journal.status);
            }
        }
    }
    Ok(())
}

/// Validate one pending transaction while its workspace lock is held.
pub(crate) fn validate_pending_recovery(forgeplan_dir: &Path, id: &str) -> Result<()> {
    check_repair_journals(forgeplan_dir, Some(id))?;
    let journal = read_valid_journal(forgeplan_dir, id)?;
    let expected = match journal.status {
        JournalStatus::CanonicalCommittedIndexPending => &journal.planned_postimages,
        JournalStatus::RolledBackIndexPending => &journal.preimages,
        _ => bail!("repair transaction {id} is not awaiting index reconciliation"),
    };
    let dir = repair_journal_dir(forgeplan_dir, id);
    require_directory(&dir.join("preimages"))?;
    for (path, pre_hash) in &journal.preimages {
        if sha256_hex(&read_regular_file(&preimage_file(&dir, path))?) != *pre_hash {
            bail!("repair journal {id} has a damaged preimage for {path}");
        }
    }
    validate_postimage_blobs(&dir, &journal)?;
    for operation in &journal.operations {
        let absolute = resolve_document_path(forgeplan_dir, &operation.path)?;
        let actual = read_optional_file(&absolute)?.map(|bytes| sha256_hex(&bytes));
        if actual.as_ref() != expected.get(&operation.path) {
            bail!("repair transaction {id} has an unexpected file image at {}", operation.path);
        }
        if journal.status == JournalStatus::CanonicalCommittedIndexPending
            && !journal.preimages.contains_key(&operation.path)
        {
            require_owned_create(&dir, &operation.path, &absolute)?;
        }
    }
    Ok(())
}

/// An additive receipt retains the legacy manifest as it existed before
/// rollback; the mutable manifest keeps its original serialization and digest.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyUpgradeReceipt {
    version: u32,
    original: RepairJournal,
}

fn validate_legacy_upgrade_receipt(dir: &Path, journal: &RepairJournal) -> Result<bool> {
    let receipt_dir = dir.join("legacy-upgrade-v1");
    match fs::symlink_metadata(&receipt_dir) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error).context("inspect legacy upgrade receipt"),
        Ok(_) => require_directory(&receipt_dir)?,
    }
    let receipt: LegacyUpgradeReceipt = serde_json::from_slice(
        &read_regular_file(&receipt_dir.join("receipt.json"))?,
    ).context("parse legacy upgrade receipt")?;
    let original = &receipt.original;
    if receipt.version != 1
        || original.version.is_some()
        || !journal_digest_matches(original)
        || original.transaction_id != journal.transaction_id
        || original.preimages != journal.preimages
        || original.planned_postimages != journal.planned_postimages
        || !original.operations.iter().map(|op| &op.path).eq(journal.operations.iter().map(|op| &op.path))
    {
        bail!("legacy upgrade receipt does not match journal {}", journal.transaction_id);
    }
    require_directory(&receipt_dir.join("postimages"))?;
    for (path, expected) in &journal.planned_postimages {
        if sha256_hex(&read_regular_file(&postimage_file(&receipt_dir, path))?) != *expected {
            bail!("legacy upgrade receipt has a damaged postimage for {path}");
        }
    }
    Ok(true)
}

/// Prepare all available verified postimages before the first rollback write.
/// Publishing the complete directory is the only point at which the receipt
/// becomes visible; an interrupted preparation never authorizes restoration.
fn prepare_legacy_upgrade_receipt(
    workspace: &Path,
    dir: &Path,
    journal: &RepairJournal,
) -> Result<()> {
    if validate_legacy_upgrade_receipt(dir, journal)? {
        sync_dir(dir)?;
        return Ok(());
    }
    let stage = tempfile::Builder::new().prefix(".legacy-upgrade-").tempdir_in(dir)?;
    fs::create_dir(stage.path().join("postimages"))?;
    for (path, expected) in &journal.planned_postimages {
        let absolute = resolve_document_path(workspace, path)?;
        let Some(bytes) = read_optional_file(&absolute)? else {
            return Ok(());
        };
        if sha256_hex(&bytes) != *expected {
            // An interrupted apply or rollback may already expose a preimage.
            // Preserve the original rollback behavior; no receipt is published.
            return Ok(());
        }
        let preimage = read_regular_file(&preimage_file(dir, path))?;
        if journal.preimages.get(path) != Some(&sha256_hex(&preimage)) {
            bail!("legacy preimage {path} does not match the journal");
        }
        write_file_atomically(&postimage_file(stage.path(), path), &bytes)?;
    }
    let receipt = LegacyUpgradeReceipt { version: 1, original: journal.clone() };
    write_file_atomically(&stage.path().join("receipt.json"), &serde_json::to_vec(&receipt)?)?;
    sync_dir(&stage.path().join("postimages"))?;
    sync_dir(stage.path())?;
    publish_directory_noclobber(stage.path(), &dir.join("legacy-upgrade-v1"))?;
    sync_dir(dir)?;
    validate_legacy_upgrade_receipt(dir, journal)?;
    Ok(())
}

fn validate_postimage_blobs(dir: &Path, journal: &RepairJournal) -> Result<()> {
    if journal.version.is_none() {
        validate_legacy_upgrade_receipt(dir, journal)?;
        return Ok(());
    }
    require_directory(&dir.join("postimages"))?;
    require_directory(&dir.join("staged"))?;
    for (path, expected) in &journal.planned_postimages {
        if sha256_hex(&read_regular_file(&postimage_file(dir, path))?) != *expected {
            bail!("repair journal {} has a damaged postimage for {path}", journal.transaction_id);
        }
    }
    Ok(())
}

fn validate_publication_journal(forgeplan_dir: &Path, dir: &Path, journal: &RepairJournal, operation: &FileOperation) -> Result<()> {
    if read_valid_journal(forgeplan_dir, &journal.transaction_id)? != *journal {
        bail!("repair journal changed before publication");
    }
    let path = &operation.path;
    if let Some(expected) = journal.preimages.get(path) {
        require_directory(&dir.join("preimages"))?;
        if sha256_hex(&read_regular_file(&preimage_file(dir, path))?) != *expected {
            bail!("repair journal has a damaged preimage for {path}");
        }
    }
    if let Some(expected) = journal.planned_postimages.get(path) {
        require_directory(&dir.join("postimages"))?;
        if sha256_hex(&read_regular_file(&postimage_file(dir, path))?) != *expected {
            bail!("repair journal has a damaged postimage for {path}");
        }
        if !journal.preimages.contains_key(path) {
            require_directory(&dir.join("staged"))?;
            if sha256_hex(&read_regular_file(&staged_file(dir, path))?) != *expected {
                bail!("repair journal has a damaged staged create image for {path}");
            }
        }
    }
    Ok(())
}

pub(crate) fn pending_repair_digest(forgeplan_dir: &Path, id: &str) -> Result<String> {
    validate_pending_recovery(forgeplan_dir, id)?;
    Ok(read_valid_journal(forgeplan_dir, id)?.manifest_sha256)
}

/// Check the sole blocked diagnostic transaction while the workspace lock is
/// held. The returned digest binds the blocked guard to this exact manifest.
pub(crate) fn validate_blocked_repair_admission(forgeplan_dir: &Path, id: &str) -> Result<String> {
    validate_blocked_repair_admission_inner(forgeplan_dir, id, None)
}

fn validate_blocked_repair_admission_inner(forgeplan_dir: &Path, id: &str, staged_id: Option<&str>) -> Result<String> {
    check_repair_journals_inner(forgeplan_dir, Some(id), staged_id)?;
    let journal = read_valid_journal(forgeplan_dir, id)?;
    let receipt = journal.legacy_diagnostic.as_ref()
        .ok_or_else(|| anyhow::anyhow!("repair transaction {id} has no diagnostic receipt"))?;
    if journal.status != JournalStatus::RolledBackIndexBlocked || receipt.successor.is_some() {
        bail!("repair transaction {id} is not an unlinked blocked rollback");
    }
    let dir = repair_journal_dir(forgeplan_dir, id);
    require_directory(&dir.join("preimages"))?;
    for (path, hash) in &journal.preimages {
        if sha256_hex(&read_regular_file(&preimage_file(&dir, path))?) != *hash {
            bail!("repair journal {id} has a damaged preimage for {path}");
        }
    }
    validate_postimage_blobs(&dir, &journal)?;
    for operation in &journal.operations {
        let path = resolve_document_path(forgeplan_dir, &operation.path)?;
        let hash = read_optional_file(&path)?.map(|bytes| sha256_hex(&bytes));
        if hash.as_ref() != journal.preimages.get(&operation.path) {
            bail!("blocked repair {id} has an unexpected file image at {}", operation.path);
        }
    }
    let census = census_workspace(forgeplan_dir)?;
    if census.inventory_digest != receipt.source_digest || collision_groups(&census).is_empty() {
        bail!("blocked repair {id} no longer matches its diagnostic corpus");
    }
    Ok(journal.manifest_sha256)
}

/// Read the validated pending status while the caller retains its scoped guard.
pub(crate) fn pending_recovery_status(
    forgeplan_dir: &Path,
    id: &str,
    guard: &WorkspaceLock,
) -> Result<JournalStatus> {
    let workspace = fs::canonicalize(forgeplan_dir)
        .with_context(|| format!("resolve workspace {}", forgeplan_dir.display()))?;
    if !guard.is_repair_guard_for(&workspace, id) {
        bail!("repair status requires the scoped workspace guard for transaction {id}");
    }
    validate_pending_recovery(forgeplan_dir, id)?;
    Ok(read_valid_journal(forgeplan_dir, id)?.status)
}

/// A document's exact identity transition derived from journaled bytes.
pub(crate) struct LegacyIdentityMapping {
    pub(crate) path: String,
    pub(crate) display_id: String,
    pub(crate) old_key: String,
    pub(crate) new_key: String,
    pub(crate) preimage: Vec<u8>,
    pub(crate) postimage: Vec<u8>,
}

/// A legacy-index recovery bound to one journal and a held legacy repair lock.
pub(crate) struct ValidatedLegacyRepair<'guard> {
    workspace: PathBuf,
    journal_id: String,
    journal_digest: String,
    current_manifest_digest: String,
    blocked: bool,
    source_digest: String,
    guard: &'guard WorkspaceLock,
}

impl ValidatedLegacyRepair<'_> {
    pub(crate) fn workspace(&self) -> &Path {
        &self.workspace
    }

    pub(crate) fn journal_id(&self) -> &str {
        &self.journal_id
    }

    pub(crate) fn journal_digest(&self) -> &str {
        &self.journal_digest
    }

    pub(crate) fn source_digest(&self) -> &str {
        &self.source_digest
    }

    pub(crate) fn identity_mappings(&self) -> Result<(JournalStatus, Vec<LegacyIdentityMapping>)> {
        self.revalidate(&self.workspace)?;
        let journal = read_valid_journal(&self.workspace, &self.journal_id)?;
        let dir = repair_journal_dir(&self.workspace, &self.journal_id);
        let mut mappings = Vec::new();
        for operation in &journal.operations {
            let path = Path::new(&operation.path);
            let kind = path.components().next().and_then(|component| match component {
                Component::Normal(name) => name.to_str(),
                _ => None,
            });
            if !kind.is_some_and(|name| crate::workspace::ARTIFACT_DIRS.contains(&name)) {
                continue;
            }
            let preimage = read_regular_file(&preimage_file(&dir, &operation.path))?;
            let postimage = if journal.version == Some(2) {
                read_regular_file(&postimage_file(&dir, &operation.path))?
            } else if validate_legacy_upgrade_receipt(&dir, &journal)? {
                read_regular_file(&postimage_file(&dir.join("legacy-upgrade-v1"), &operation.path))?
            } else if journal.status == JournalStatus::CanonicalCommittedIndexPending {
                read_regular_file(&resolve_document_path(&self.workspace, &operation.path)?)?
            } else {
                bail!("legacy journal has no verified postimage for {}; restore it from a backup", operation.path);
            };
            if journal.preimages.get(&operation.path) != Some(&sha256_hex(&preimage))
                || journal.planned_postimages.get(&operation.path) != Some(&sha256_hex(&postimage))
            {
                bail!("journaled identity images do not match for {}", operation.path);
            }
            let pre = std::str::from_utf8(&preimage)?;
            let post = std::str::from_utf8(&postimage)?;
            let pre_fm = effective_frontmatter(pre)?;
            let post_fm = effective_frontmatter(post)?;
            let old_id = pre_fm.get("id").and_then(|value| value.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing preimage id at {}", operation.path))?;
            let new_id = post_fm.get("id").and_then(|value| value.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing postimage id at {}", operation.path))?;
            if old_id != new_id {
                bail!("display id changed at {}", operation.path);
            }
            let old_key = canonical_key_from_document(old_id, pre);
            let new_key = canonical_key_from_document(new_id, post);
            mappings.push(LegacyIdentityMapping {
                path: operation.path.clone(),
                display_id: old_id.to_string(),
                old_key,
                new_key,
                preimage,
                postimage,
            });
        }
        self.revalidate(&self.workspace)?;
        Ok((journal.status, mappings))
    }

    /// Revalidate the complete source and the particular journal under the
    /// original guard before each index mutation and before the terminal write.
    pub(crate) fn revalidate(&self, workspace: &Path) -> Result<()> {
        let canonical = fs::canonicalize(workspace)
            .with_context(|| format!("resolve workspace {}", workspace.display()))?;
        if canonical != self.workspace {
            bail!("legacy repair capability is not bound to this workspace");
        }
        let current_digest = if self.blocked {
            if self.guard.blocked_legacy_repair_digest_for(&canonical, &self.journal_id)
                != Some(self.current_manifest_digest.as_str())
            {
                bail!("legacy blocked guard does not match the admitted journal");
            }
            validate_blocked_repair_admission(&canonical, &self.journal_id)?
        } else {
            if self.guard.legacy_repair_digest_for(&canonical, &self.journal_id)
                != Some(self.current_manifest_digest.as_str())
            {
                bail!("legacy repair guard does not match the admitted journal");
            }
            require_linked_legacy_guard(&canonical, &self.journal_id, self.guard)?;
            pending_repair_digest(&canonical, &self.journal_id)?
        };
        if current_digest != self.current_manifest_digest {
            bail!("legacy repair journal changed after admission");
        }
        let source = census_workspace(&canonical)?;
        if source.inventory_digest != self.source_digest {
            bail!("legacy repair source changed after admission");
        }
        let after = if self.blocked {
            validate_blocked_repair_admission(&canonical, &self.journal_id)?
        } else {
            pending_repair_digest(&canonical, &self.journal_id)?
        };
        if after != self.current_manifest_digest {
            bail!("legacy repair journal changed during source capture");
        }
        Ok(())
    }
}

/// A routing hint only; linked-lock acquisition revalidates the complete pair.
pub(crate) fn legacy_repair_predecessor_hint(workspace: &Path, id: &str) -> Result<Option<String>> {
    let journal = read_valid_journal(workspace, id)?;
    match journal.legacy_predecessors.as_slice() {
        [] => Ok(None),
        [previous] => Ok(Some(previous.transaction_id.clone())),
        _ => bail!("repair transaction {id} has an unsupported predecessor chain"),
    }
}

fn require_linked_legacy_guard(workspace: &Path, id: &str, guard: &WorkspaceLock) -> Result<()> {
    let Some(old_id) = legacy_repair_predecessor_hint(workspace, id)? else {
        return Ok(());
    };
    let (old, new) = validate_linked_pair(workspace, &old_id, id)?;
    if old.legacy_diagnostic.as_ref().and_then(|receipt| receipt.successor.as_deref()) != Some(id) {
        bail!("repair chain has not been durably bound");
    }
    if guard.linked_legacy_repair_digests_for(workspace, &old_id, id)
        != Some((old.manifest_sha256.as_str(), new.manifest_sha256.as_str()))
    {
        bail!("repair chain requires its exact linked legacy guard");
    }
    Ok(())
}

pub(crate) fn validate_legacy_repair<'guard>(
    workspace: &Path,
    guard: &'guard WorkspaceLock,
    journal_id: &str,
) -> Result<ValidatedLegacyRepair<'guard>> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_mutation()?;
    let workspace = fs::canonicalize(workspace)
        .with_context(|| format!("resolve workspace {}", workspace.display()))?;
    if !guard.is_legacy_repair_guard_for(&workspace, journal_id) {
        bail!("legacy repair requires the scoped legacy workspace guard for transaction {journal_id}");
    }
    let journal_digest = pending_repair_digest(&workspace, journal_id)?;
    if guard.legacy_repair_digest_for(&workspace, journal_id) != Some(journal_digest.as_str()) {
        bail!("legacy repair journal changed after acquiring its guard");
    }
    require_linked_legacy_guard(&workspace, journal_id, guard)?;
    let source_digest = census_workspace(&workspace)?.inventory_digest;
    let proof = ValidatedLegacyRepair {
        workspace,
        journal_id: journal_id.to_string(),
        current_manifest_digest: journal_digest.clone(),
        journal_digest,
        blocked: false,
        source_digest,
        guard,
    };
    proof.revalidate(&proof.workspace)?;
    Ok(proof)
}

pub(crate) fn validate_blocked_legacy_repair<'guard>(
    workspace: &Path,
    guard: &'guard WorkspaceLock,
    journal_id: &str,
) -> Result<ValidatedLegacyRepair<'guard>> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_mutation()?;
    let workspace = fs::canonicalize(workspace)
        .with_context(|| format!("resolve workspace {}", workspace.display()))?;
    let current_manifest_digest = validate_blocked_repair_admission(&workspace, journal_id)?;
    if guard.blocked_legacy_repair_digest_for(&workspace, journal_id)
        != Some(current_manifest_digest.as_str())
    {
        bail!("blocked repair requires the scoped guard for this exact journal");
    }
    let journal = read_valid_journal(&workspace, journal_id)?;
    let receipt = journal.legacy_diagnostic
        .ok_or_else(|| anyhow::anyhow!("blocked repair has no diagnostic receipt"))?;
    let proof = ValidatedLegacyRepair {
        workspace,
        journal_id: journal_id.to_string(),
        journal_digest: receipt.prior_manifest_sha256,
        current_manifest_digest,
        blocked: true,
        source_digest: receipt.source_digest,
        guard,
    };
    proof.revalidate(&proof.workspace)?;
    Ok(proof)
}

pub(crate) async fn finish_legacy_diagnostic(
    proof: &ValidatedLegacyRepair<'_>,
    workspace: &Path,
    verified: &crate::artifact::legacy_diagnostic::VerifiedLegacyDiagnostic<'_>,
) -> Result<()> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_mutation()?;
    proof.revalidate(workspace)?;
    if !verified.matches_repair(proof) || !valid_hash(verified.snapshot_digest()) {
        bail!("legacy diagnostic proof does not match the admitted repair");
    }
    let confirmed = crate::artifact::legacy_diagnostic::read_legacy_diagnostic(proof).await?;
    if !confirmed.matches_repair(proof) || confirmed.snapshot_digest() != verified.snapshot_digest() {
        bail!("legacy diagnostic snapshot changed after verification");
    }
    proof.revalidate(workspace)?;
    if !confirmed.has_identity_collisions() {
        bail!("legacy diagnostic completion requires unresolved identity collisions");
    }
    let mut journal = read_valid_journal(&proof.workspace, &proof.journal_id)?;
    if journal.status != JournalStatus::RolledBackIndexPending
        || journal.manifest_sha256 != proof.journal_digest
    {
        bail!("legacy diagnostic completion requires the admitted pending rollback");
    }
    journal.legacy_diagnostic = Some(LegacyDiagnosticReceipt {
        prior_manifest_sha256: proof.journal_digest.clone(),
        source_digest: proof.source_digest.clone(),
        snapshot_digest: confirmed.snapshot_digest().to_string(),
        successor: None,
    });
    journal.status = JournalStatus::RolledBackIndexBlocked;
    write_manifest(&repair_journal_dir(&proof.workspace, &proof.journal_id), &mut journal)?;
    finish_linked_predecessor(proof, true)
}

pub(crate) fn finish_legacy_repair(
    proof: ValidatedLegacyRepair<'_>,
    workspace: &Path,
    verified: crate::db::store::VerifiedLegacyIndex,
) -> Result<()> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_mutation()?;
    proof.revalidate(workspace)?;
    if !verified.matches_repair(&proof) {
        bail!("legacy index proof does not match the admitted repair");
    }
    finish_linked_predecessor(&proof, false)?;
    finish_pending_recovery_inner(&proof.workspace, &proof.journal_id, &proof.journal_digest, proof.guard)
}

fn finish_linked_predecessor(proof: &ValidatedLegacyRepair<'_>, diagnostic: bool) -> Result<()> {
    let Some(old_id) = legacy_repair_predecessor_hint(&proof.workspace, &proof.journal_id)? else {
        return Ok(());
    };
    let (mut old, new) = validate_linked_pair(&proof.workspace, &old_id, &proof.journal_id)?;
    if proof.guard.linked_legacy_repair_digests_for(&proof.workspace, &old_id, &proof.journal_id)
        != Some((old.manifest_sha256.as_str(), proof.current_manifest_digest.as_str()))
    {
        bail!("repair completion requires the exact linked guard");
    }
    if diagnostic {
        let receipt = new.legacy_diagnostic.as_ref()
            .ok_or_else(|| anyhow::anyhow!("successor has no diagnostic receipt"))?;
        if new.status != JournalStatus::RolledBackIndexBlocked
            || receipt.prior_manifest_sha256 != proof.journal_digest
        {
            bail!("successor diagnostic was not durably recorded");
        }
    } else if new.manifest_sha256 != proof.current_manifest_digest
        || !matches!(new.status, JournalStatus::CanonicalCommittedIndexPending | JournalStatus::RolledBackIndexPending)
    {
        bail!("successor changed before its verified completion");
    }
    if old.status == JournalStatus::RolledBackIndexBlocked {
        old.status = JournalStatus::RolledBack;
        write_manifest(&repair_journal_dir(&proof.workspace, &old_id), &mut old)?;
    }
    Ok(())
}

/// Finish the old half of a diagnostic chain after an interrupted completion.
/// The new blocked journal remains the ordinary-read/write barrier.
pub async fn finish_blocked_linked_repair(workspace: &Path, old_id: &str, new_id: &str) -> Result<()> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_mutation()?;
    let guard = crate::workspace::lock::acquire_linked_repair_workspace_lock(workspace, old_id, new_id).await?;
    let workspace = fs::canonicalize(workspace)?;
    let (mut old, new) = validate_linked_pair(&workspace, old_id, new_id)?;
    if guard.linked_legacy_repair_digests_for(&workspace, old_id, new_id)
        != Some((old.manifest_sha256.as_str(), new.manifest_sha256.as_str()))
    {
        bail!("diagnostic chain guard does not match both journals");
    }
    if old.legacy_diagnostic.as_ref().and_then(|receipt| receipt.successor.as_deref()) != Some(new_id) {
        bail!("diagnostic chain was not durably bound");
    }
    if validate_blocked_repair_admission(&workspace, new_id)? != new.manifest_sha256 {
        bail!("successor diagnostic journal changed after admission");
    }
    let receipt = new.legacy_diagnostic.as_ref()
        .ok_or_else(|| anyhow::anyhow!("successor diagnostic receipt is missing"))?;
    crate::artifact::legacy_diagnostic::verify_stored_snapshot_integrity(
        &workspace, new_id, &receipt.snapshot_digest, &receipt.prior_manifest_sha256, &receipt.source_digest,
    )?;
    if old.status == JournalStatus::RolledBackIndexBlocked {
        old.status = JournalStatus::RolledBack;
        write_manifest(&repair_journal_dir(&workspace, old_id), &mut old)?;
    }
    Ok(())
}

/// Finish only after the owner has verified its published index and backlog
/// against the current capture while retaining this workspace guard.
pub(crate) fn finish_pending_recovery(forgeplan_dir: &Path, id: &str, expected_digest: &str, guard: &WorkspaceLock) -> Result<()> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_mutation()?;
    let workspace = fs::canonicalize(forgeplan_dir)
        .with_context(|| format!("resolve workspace {}", forgeplan_dir.display()))?;
    if guard.is_legacy_repair_guard_for(&workspace, id) {
        bail!("legacy repair completion requires a verified legacy index proof");
    }
    if !read_valid_journal(&workspace, id)?.legacy_predecessors.is_empty() {
        bail!("linked legacy repair requires its verified legacy index proof");
    }
    finish_pending_recovery_inner(&workspace, id, expected_digest, guard)
}

fn finish_pending_recovery_inner(forgeplan_dir: &Path, id: &str, expected_digest: &str, guard: &WorkspaceLock) -> Result<()> {
    let workspace = fs::canonicalize(forgeplan_dir)
        .with_context(|| format!("resolve workspace {}", forgeplan_dir.display()))?;
    if !guard.is_repair_guard_for(&workspace, id) {
        bail!("repair completion requires the scoped workspace guard for transaction {id}");
    }
    if pending_repair_digest(forgeplan_dir, id)? != expected_digest {
        bail!("repair transaction {id} changed after index admission");
    }
    let mut journal = read_valid_journal(forgeplan_dir, id)?;
    journal.status = match journal.status {
        JournalStatus::CanonicalCommittedIndexPending => JournalStatus::CanonicalCommitted,
        JournalStatus::RolledBackIndexPending => JournalStatus::RolledBack,
        _ => bail!("repair transaction {id} is not awaiting index reconciliation"),
    };
    write_manifest(&repair_journal_dir(forgeplan_dir, id), &mut journal)
}

/// A fresh transaction identifier: the UTC time plus a random suffix.
pub fn new_transaction_id() -> String {
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let suffix: String = (0..4)
        .map(|_| format!("{:02x}", rand::random::<u8>()))
        .collect();
    format!("repair-{stamp}-{suffix}")
}

/// Apply a repair plan.
///
/// The apply refuses to touch anything until it has re-checked, under the
/// workspace lock, every fact the plan was built on: the workspace, the Git
/// identity, the whole document inventory, the group membership, the hash
/// of every source and every reference, and every target path. A plan that
/// no longer matches the workspace is stale, and a stale plan is refused.
///
/// The repair is preserve-all: no document is deleted, no body is merged,
/// and no historical display number is rewritten. Only the canonical key of
/// a colliding document and the references that pointed at it change.
pub fn apply_repair_plan(forgeplan_dir: &Path, plan: &RepairPlan) -> Result<ApplyOutcome> {
    apply_repair_plan_with_publisher(forgeplan_dir, plan, publish)
}

fn apply_repair_plan_with_publisher(
    forgeplan_dir: &Path,
    plan: &RepairPlan,
    publish_postimage: impl FnMut(&Path, &[u8]) -> Result<()>,
) -> Result<ApplyOutcome> {
    let guard = acquire_repair_lock(forgeplan_dir)?;
    apply_repair_plan_locked(forgeplan_dir, plan, publish_postimage, &guard, None)
}

/// Apply a new plan only after verifying the exact blocked rollback snapshot.
pub async fn apply_repair_plan_after_blocked(
    forgeplan_dir: &Path,
    plan: &RepairPlan,
    old_id: &str,
) -> Result<ApplyOutcome> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_mutation()?;
    let guard = crate::workspace::lock::acquire_blocked_legacy_workspace_lock(forgeplan_dir, old_id).await?;
    let proof = validate_blocked_legacy_repair(forgeplan_dir, &guard, old_id)?;
    let diagnostic = crate::artifact::legacy_diagnostic::read_legacy_diagnostic(&proof).await?;
    let old = read_valid_journal(&proof.workspace, old_id)?;
    let receipt = old.legacy_diagnostic.as_ref()
        .ok_or_else(|| anyhow::anyhow!("blocked journal lacks diagnostic receipt"))?;
    if !diagnostic.matches_repair(&proof) || diagnostic.snapshot_digest() != receipt.snapshot_digest {
        bail!("blocked diagnostic snapshot does not match its receipt");
    }
    proof.revalidate(forgeplan_dir)?;
    let predecessor = LegacyRepairPredecessor {
        transaction_id: old_id.to_string(),
        blocked_manifest_sha256: proof.current_manifest_digest.clone(),
    };
    drop(diagnostic);
    drop(proof);
    apply_repair_plan_locked(forgeplan_dir, plan, publish, &guard, Some(predecessor))
}

fn apply_repair_plan_locked(
    forgeplan_dir: &Path,
    plan: &RepairPlan,
    mut publish_postimage: impl FnMut(&Path, &[u8]) -> Result<()>,
    _guard: &WorkspaceLock,
    predecessor: Option<LegacyRepairPredecessor>,
) -> Result<ApplyOutcome> {
    if plan.version != REPAIR_PLAN_VERSION {
        bail!("unsupported repair plan version {}", plan.version);
    }
    if !valid_inventory_digest(&plan.inventory_digest) {
        bail!("unsupported repair inventory digest version");
    }
    if !plan_digest_matches(plan) {
        bail!("the repair plan does not match its digest — the plan file was edited after it was written");
    }
    let workspace = fs::canonicalize(forgeplan_dir)
        .with_context(|| format!("resolve workspace {}", forgeplan_dir.display()))?;
    if plan.workspace_path != workspace.to_string_lossy() {
        bail!(
            "the repair plan was made against a different workspace: {} (plan) vs {} (now)",
            plan.workspace_path,
            workspace.display()
        );
    }

    if let Some(previous) = &predecessor {
        if validate_blocked_repair_admission(forgeplan_dir, &previous.transaction_id)?
            != previous.blocked_manifest_sha256
        {
            bail!("blocked repair changed after snapshot verification");
        }
    } else {
        check_repair_journals(forgeplan_dir, None)?;
    }

    // The plan may only name documents inside the workspace. A path that
    // climbs out of it, or that crosses a symlink, is refused before
    // anything else is looked at.
    for source in &plan.sources {
        resolve_document_path(forgeplan_dir, &source.relative_path)?;
        resolve_document_path(forgeplan_dir, &source.proposed_path)?;
    }
    for reference in &plan.references {
        resolve_document_path(forgeplan_dir, &reference.source_path)?;
    }

    // The Git identity.
    let git_now = detect_git_identity(forgeplan_dir)?;
    if git_now != plan.git {
        bail!("the Git identity of the workspace changed since the plan was made — the plan is stale");
    }

    // The whole inventory.
    let census = census_workspace(forgeplan_dir)?;
    if census.inventory_digest != plan.inventory_digest {
        bail!("the workspace changed since the plan was made — the plan is stale");
    }

    // The group membership.
    if collision_groups(&census) != planned_groups(plan) {
        bail!("the collision groups changed since the plan was made — the plan is stale");
    }
    validate_plan_sources(plan, &census)?;
    if !plan.sidecar_conflicts.is_empty() {
        bail!("unresolved identity-bearing sidecar: {}", plan.sidecar_conflicts.join("; "));
    }
    if plan_sidecar_moves(forgeplan_dir, &census, &plan.sources)? != plan.sidecar_moves {
        bail!("the sidecar attribution or destination changed since the plan was made");
    }

    // The sources, the references and the target paths.
    let operations = plan_operations(forgeplan_dir, plan, &census)?;
    if operations.is_empty() {
        bail!("the repair plan touches no document — there is nothing to repair");
    }

    // The journal is durable before the first mutation.
    let transaction_id = new_transaction_id();
    let journal_dir = repair_journal_dir(forgeplan_dir, &transaction_id);
    let root = repair_journal_root(forgeplan_dir);
    match fs::create_dir(&root) {
        Ok(()) => sync_dir(forgeplan_dir)?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => require_directory(&root)?,
        Err(error) => return Err(error).with_context(|| format!("create {}", root.display())),
    }
    preparation_checkpoint("journal_root")?;
    let staging_root = repair_staging_root(forgeplan_dir);
    match fs::create_dir(&staging_root) {
        Ok(()) => sync_dir(forgeplan_dir)?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => require_directory(&staging_root)?,
        Err(error) => return Err(error).with_context(|| format!("create {}", staging_root.display())),
    }
    preparation_checkpoint("staging_root")?;
    let staging_dir = staging_root.join(&transaction_id);
    fs::create_dir(&staging_dir).with_context(|| format!("create {}", staging_dir.display()))?;
    sync_dir(&staging_root)?;
    preparation_checkpoint("staging_directory")?;
    let mut journal = RepairJournal {
        version: Some(2),
        transaction_id: transaction_id.clone(),
        manifest_sha256: String::new(),
        preimages: operations
            .iter()
            .filter_map(|operation| operation.preimage.as_ref().map(|bytes| (operation.path.clone(), sha256_hex(bytes))))
            .collect(),
        planned_postimages: operations
            .iter()
            .filter_map(|operation| operation.postimage.as_ref().map(|bytes| (operation.path.clone(), sha256_hex(bytes))))
            .collect(),
        operations: operations
            .iter()
            .map(|operation| JournalOperation {
                path: operation.path.clone(),
                state: OperationState::Pending,
            })
            .collect(),
        status: JournalStatus::InProgress,
        phase: Some(RepairPhase::Applying),
        moves: plan.sidecar_moves.iter().map(|sidecar| JournalMove {
            source: sidecar.source_path.clone(),
            destination: sidecar.destination_path.clone(),
        }).collect(),
        legacy_diagnostic: None,
        legacy_predecessors: predecessor.iter().cloned().collect(),
    };
    write_preimages(&staging_dir, &operations)?;
    write_manifest(&staging_dir, &mut journal)?;
    preparation_checkpoint("manifest")?;
    let prepared = PreparedRepairReceipt {
        version: 1,
        transaction_id: transaction_id.clone(),
        workspace_path: workspace.to_string_lossy().to_string(),
        plan_digest: plan.digest.clone(),
        manifest_digest: journal.manifest_sha256.clone(),
    };
    write_file_atomically(&staging_dir.join("READY.json"), &serde_json::to_vec(&prepared)?)?;
    sync_dir(&staging_dir)?;
    preparation_checkpoint("ready")?;
    publish_directory_noclobber(&staging_dir, &journal_dir)
        .with_context(|| format!("publish prepared transaction {transaction_id}"))?;
    preparation_checkpoint("directory_publish")?;
    sync_dir(&staging_root)?;
    preparation_checkpoint("staging_parent_sync")?;
    sync_dir(&root)?;
    preparation_checkpoint("journal_parent_sync")?;
    if let Some(previous) = &predecessor {
        let mut old = read_valid_journal(forgeplan_dir, &previous.transaction_id)?;
        if old.manifest_sha256 != previous.blocked_manifest_sha256 {
            bail!("blocked journal changed before chain publication");
        }
        let receipt = old.legacy_diagnostic.as_mut()
            .ok_or_else(|| anyhow::anyhow!("blocked journal has no diagnostic receipt"))?;
        if receipt.successor.is_some() {
            bail!("blocked journal is already linked to a successor");
        }
        receipt.successor = Some(transaction_id.clone());
        write_manifest(&repair_journal_dir(forgeplan_dir, &previous.transaction_id), &mut old)
            .with_context(|| format!("link blocked repair {} to successor {transaction_id}; recover this exact pair", previous.transaction_id))?;
    }

    // Publish every post-image atomically.
    let mut published: Vec<String> = Vec::new();
    for operation in &operations {
        let result = (|| -> Result<()> {
            validate_publication_journal(forgeplan_dir, &journal_dir, &journal, operation)?;
            let absolute = resolve_document_path(forgeplan_dir, &operation.path)?;
            if read_optional_file(&absolute)? != operation.preimage {
                bail!("{} changed before publication", operation.path);
            }
            match (&operation.preimage, &operation.postimage) {
                (None, Some(bytes)) => publish_staged(&journal_dir, &operation.path, &absolute, bytes),
                (Some(_), Some(bytes)) => publish_postimage(&absolute, bytes),
                (Some(_), None) => remove_file_and_sync(&absolute),
                (None, None) => bail!("empty operation {}", operation.path),
            }
        })().and_then(|()| {
            published.push(operation.path.clone());
            let entry = journal.operations.iter_mut()
                .find(|entry| entry.path == operation.path)
                .expect("operation was constructed from the same list");
            entry.state = OperationState::Published;
            write_manifest(&journal_dir, &mut journal)
        });
        if let Err(error) = result {
            // A rename may have happened before fsync failed. Treat every
            // operation, including this one, as potentially published.
            journal.status = JournalStatus::InProgress;
            journal.phase = Some(RepairPhase::RollingBack);
            write_manifest(&journal_dir, &mut journal)
                .with_context(|| format!("record recovery before restoring after {error:#}"))?;
            let all_paths: Vec<String> = operations.iter().map(|entry| entry.path.clone()).collect();
            let (restored, failures) = restore_preimages(&journal_dir, &operations, &all_paths);
            for entry in &mut journal.operations {
                entry.state = if restored.contains(&entry.path) {
                    OperationState::RolledBack
                } else {
                    OperationState::Conflict
                };
            }
            journal.status = if failures.is_empty() { JournalStatus::RolledBackIndexPending } else { JournalStatus::Conflict };
            write_manifest(&journal_dir, &mut journal)
                .with_context(|| format!("record failed repair after publishing {}: {error:#}", operation.path))?;
            bail!(
                "the repair failed while publishing {}: {error:#}; restored: {}; restoration failures: {}",
                operation.path, restored.join(", "), failures.join("; ")
            );
        }
    }

    for operation in &operations {
        let observed = resolve_document_path(forgeplan_dir, &operation.path).and_then(|path| {
            let current = read_optional_file(&path)?;
            if operation.preimage.is_none() && current.is_some() {
                require_owned_create(&journal_dir, &operation.path, &path)?;
            }
            Ok(current)
        });
        match observed {
            Ok(current) if current == operation.postimage => {}
            _ => {
                journal.status = JournalStatus::Conflict;
                write_manifest(&journal_dir, &mut journal)?;
                bail!("{} changed before commit", operation.path);
            }
        }
    }

    // The canonical commit is done. The derived index is refreshed by the
    // owner (this command never touches the index), so the journal says so.
    journal.status = JournalStatus::CanonicalCommittedIndexPending;
    write_manifest(&journal_dir, &mut journal)?;

    Ok(ApplyOutcome {
        transaction_id,
        journal_dir,
        published,
        status: journal.status,
    })
}

/// Roll a repair transaction back.
///
/// The rollback restores the preimage of every file the transaction
/// published, and only when the file still holds the post-image the
/// transaction planned. A file that was edited after the repair is left
/// alone and reported as a conflict: the journal is preserved, and the
/// operator decides what to do with it.
pub fn rollback_repair(forgeplan_dir: &Path, transaction_id: &str) -> Result<RollbackOutcome> {
    validate_transaction_id(transaction_id)?;
    let guard = acquire_repair_lock(forgeplan_dir)?;
    rollback_repair_locked(forgeplan_dir, transaction_id, &guard, None)
}

/// Explicitly recover a successor while retaining the guard for the exact pair.
pub async fn rollback_linked_repair(forgeplan_dir: &Path, old_id: &str, new_id: &str) -> Result<RollbackOutcome> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_mutation()?;
    let guard = crate::workspace::lock::acquire_linked_repair_workspace_lock(forgeplan_dir, old_id, new_id).await?;
    rollback_repair_locked(forgeplan_dir, new_id, &guard, Some(old_id))
}

fn rollback_repair_locked(
    forgeplan_dir: &Path,
    transaction_id: &str,
    guard: &WorkspaceLock,
    old_id: Option<&str>,
) -> Result<RollbackOutcome> {
    check_repair_journals(forgeplan_dir, Some(transaction_id))?;
    let journal_dir = repair_journal_dir(forgeplan_dir, transaction_id);
    let mut journal = read_valid_journal(forgeplan_dir, transaction_id)?;
    match (old_id, journal.legacy_predecessors.as_slice()) {
        (None, []) => {}
        (Some(old_id), [previous]) if previous.transaction_id == old_id => {
            let workspace = fs::canonicalize(forgeplan_dir)?;
            let (old, new) = validate_linked_pair(&workspace, old_id, transaction_id)?;
            if guard.linked_legacy_repair_digests_for(&workspace, old_id, transaction_id)
                != Some((old.manifest_sha256.as_str(), new.manifest_sha256.as_str()))
            {
                bail!("rollback requires the exact linked repair guard");
            }
        }
        _ => bail!("linked repair rollback requires its explicit predecessor and scoped guard"),
    }
    require_directory(&journal_dir.join("preimages"))?;
    if journal.version == Some(2) {
        require_directory(&journal_dir.join("staged"))?;
        require_directory(&journal_dir.join("postimages"))?;
    }

    if let [previous] = journal.legacy_predecessors.as_slice() {
        let (old, _) = validate_linked_pair(forgeplan_dir, &previous.transaction_id, transaction_id)?;
        if old.legacy_diagnostic.as_ref().and_then(|receipt| receipt.successor.as_deref()).is_none() {
            // The durable successor was prepared but never linked. The pair
            // validator proved that no canonical operation ran.
            for operation in &mut journal.operations {
                operation.state = OperationState::RolledBack;
            }
            journal.status = JournalStatus::RolledBack;
            if journal.version == Some(2) {
                journal.phase = Some(RepairPhase::RollingBack);
            }
            write_manifest(&journal_dir, &mut journal)?;
            return Ok(RollbackOutcome {
                transaction_id: transaction_id.to_string(),
                journal_dir,
                restored: Vec::new(),
                conflicts: Vec::new(),
                status: journal.status,
            });
        }
    }
    if journal.version.is_none() {
        prepare_legacy_upgrade_receipt(forgeplan_dir, &journal_dir, &journal)?;
    }


    // A committed journal must no longer admit writers before the first
    // rollback rename, even if the process crashes during restoration.
    journal.status = JournalStatus::InProgress;
    if journal.version == Some(2) {
        journal.phase = Some(RepairPhase::RollingBack);
    }
    write_manifest(&journal_dir, &mut journal)?;

    let mut restored = Vec::new();
    let mut conflicts = Vec::new();
    let planned_postimages = journal.planned_postimages.clone();
    let preimages = journal.preimages.clone();
    for index in (0..journal.operations.len()).rev() {
        let operation_path = journal.operations[index].path.clone();
        let expected_post = planned_postimages.get(&operation_path);
        let expected_pre = preimages.get(&operation_path);
        let result = (|| -> Result<bool> {
            let preimage = match expected_pre {
                Some(hash) => {
                    let bytes = read_regular_file(&preimage_file(&journal_dir, &operation_path))
                        .with_context(|| format!("read saved preimage of {operation_path}"))?;
                    if sha256_hex(&bytes) != *hash {
                        bail!("saved preimage does not match the journal");
                    }
                    Some(bytes)
                }
                None => None,
            };
            let absolute = resolve_document_path(forgeplan_dir, &operation_path)?;
            let current = read_optional_file(&absolute)?;
            let current_hash = current.as_ref().map(|bytes| sha256_hex(bytes));
            if current_hash.as_ref() == expected_pre {
                sync_dir(absolute.parent().unwrap_or_else(|| Path::new(".")))?;
                return Ok(false);
            }
            if current_hash.as_ref() != expected_post {
                bail!("unrelated edit: expected preimage {expected_pre:?} or post-image {expected_post:?}, found {current_hash:?}");
            }
            match (current, preimage) {
                (None, Some(bytes)) => publish_new(&absolute, &bytes)?,
                (Some(_), Some(bytes)) => publish(&absolute, &bytes)?,
                (Some(_), None) => {
                    require_owned_create(&journal_dir, &operation_path, &absolute)?;
                    remove_file_and_sync(&absolute)?
                }
                (None, None) => unreachable!(),
            }
            Ok(true)
        })();
        match result {
            Ok(was_restored) => {
                journal.operations[index].state = OperationState::RolledBack;
                if was_restored {
                    restored.push(operation_path.clone());
                }
            }
            Err(error) => {
                journal.operations[index].state = OperationState::Conflict;
                conflicts.push(format!("{operation_path} ({error:#})"));
                break;
            }
        }
        write_manifest(&journal_dir, &mut journal)?;
    }

    if !conflicts.is_empty() {
        journal.status = JournalStatus::Conflict;
        write_manifest(&journal_dir, &mut journal)?;
        bail!(
            "the rollback refused to overwrite an unrelated edit: {}",
            conflicts.join("; ")
        );
    }

    let mut changed_path = None;
    for operation in &journal.operations {
        let expected = journal.preimages.get(&operation.path);
        let observed = resolve_document_path(forgeplan_dir, &operation.path)
            .and_then(|path| read_optional_file(&path))
            .map(|bytes| bytes.map(|bytes| sha256_hex(&bytes)));
        if !matches!(&observed, Ok(current) if current.as_ref() == expected) {
            changed_path = Some(operation.path.clone());
            break;
        }
    }
    if let Some(path) = changed_path {
        journal.status = JournalStatus::Conflict;
        write_manifest(&journal_dir, &mut journal)?;
        bail!("{path} changed before rollback completion");
    }

    journal.status = JournalStatus::RolledBackIndexPending;
    write_manifest(&journal_dir, &mut journal)?;
    Ok(RollbackOutcome {
        transaction_id: transaction_id.to_string(),
        journal_dir,
        restored,
        conflicts,
        status: journal.status,
    })
}

/// The file that holds the preimage of one path.
fn preimage_file(journal_dir: &Path, relative_path: &str) -> PathBuf {
    journal_dir
        .join("preimages")
        .join(format!("{}.pre", sha256_hex(relative_path.as_bytes())))
}

fn postimage_file(journal_dir: &Path, relative_path: &str) -> PathBuf {
    journal_dir.join("postimages").join(format!("{}.post", sha256_hex(relative_path.as_bytes())))
}

fn staged_file(journal_dir: &Path, relative_path: &str) -> PathBuf {
    journal_dir.join("staged").join(format!("{}.stage", sha256_hex(relative_path.as_bytes())))
}

fn publish_staged(journal_dir: &Path, relative_path: &str, destination: &Path, expected: &[u8]) -> Result<()> {
    let staged = staged_file(journal_dir, relative_path);
    let bytes = read_regular_file(&staged)?;
    let saved = read_regular_file(&postimage_file(journal_dir, relative_path))?;
    if bytes.as_slice() != expected || saved.as_slice() != expected {
        bail!("damaged staged postimage for {relative_path}");
    }
    fs::hard_link(&staged, destination)
        .with_context(|| format!("create without replacement {}", destination.display()))?;
    sync_dir(destination.parent().unwrap_or_else(|| Path::new(".")))
}

#[cfg(unix)]
fn require_owned_create(journal_dir: &Path, relative_path: &str, destination: &Path) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let staged = staged_file(journal_dir, relative_path);
    let stage_meta = fs::symlink_metadata(&staged)?;
    let dest_meta = fs::symlink_metadata(destination)?;
    if !stage_meta.is_file() || !dest_meta.is_file()
        || stage_meta.dev() != dest_meta.dev() || stage_meta.ino() != dest_meta.ino()
    {
        bail!("unrelated destination at {relative_path}");
    }
    Ok(())
}

#[cfg(not(unix))]
fn require_owned_create(_journal_dir: &Path, relative_path: &str, _destination: &Path) -> Result<()> {
    bail!("cannot verify ownership of created file {relative_path} on this platform")
}

/// Validate the proposed identities against the complete, possibly colliding census.
fn validate_plan_sources(plan: &RepairPlan, census: &Census) -> Result<()> {
    let documents: BTreeMap<&str, &CensusDocument> = census.documents.iter()
        .map(|document| (document.relative_path.as_str(), document))
        .collect();
    let existing: BTreeSet<&str> = census.documents.iter()
        .map(|document| document.canonical_key.as_str())
        .collect();
    let mut paths = BTreeSet::new();
    let mut proposed = BTreeSet::new();
    for source in &plan.sources {
        if !paths.insert(source.relative_path.as_str()) {
            bail!("duplicate repair source path {}", source.relative_path);
        }
        let document = documents.get(source.relative_path.as_str())
            .ok_or_else(|| anyhow::anyhow!("repair source {} is not in the census", source.relative_path))?;
        if source.old_key != document.canonical_key {
            bail!("repair source {} no longer has the planned old key", source.relative_path);
        }
        if !is_minted_canonical_key(&source.new_key)
            || source.new_key.split('-').any(str::is_empty)
            || source.new_key.split_once('-').map(|(kind, _)| kind)
                != Some(document.kind.canonical_key_kind())
        {
            bail!("invalid proposed canonical key {:?} for {}", source.new_key, source.relative_path);
        }
        if existing.contains(source.new_key.as_str()) || !proposed.insert(source.new_key.as_str()) {
            bail!("proposed canonical key {:?} is already occupied or duplicated", source.new_key);
        }
    }
    Ok(())
}

/// Reparse every document postimage and allow only the planned identity and
/// structural-reference changes. This also verifies the effective key.
fn validate_document_postimages(
    operations: &BTreeMap<String, FileOperation>,
    plan: &RepairPlan,
    census: &Census,
) -> Result<()> {
    for (path, operation) in operations {
        let document = census.documents.iter().find(|document| document.relative_path == *path)
            .ok_or_else(|| anyhow::anyhow!("postimage path {path} is not a census document"))?;
        let before = std::str::from_utf8(operation.preimage.as_deref()
            .ok_or_else(|| anyhow::anyhow!("missing document preimage for {path}"))?)?;
        let after = std::str::from_utf8(operation.postimage.as_deref()
            .ok_or_else(|| anyhow::anyhow!("missing document postimage for {path}"))?)?;
        let mut expected = effective_frontmatter(before)?;
        let source = plan.sources.iter().find(|source| source.relative_path == *path);
        let expected_key = if let Some(source) = source {
            expected.insert("slug".to_string(), serde_yaml::Value::String(source.new_key.clone()));
            source.new_key.as_str()
        } else {
            document.canonical_key.as_str()
        };
        for reference in plan.references.iter().filter(|reference| reference.source_path == *path && reference.from != reference.to) {
            let target = serde_yaml::Value::String(reference.to.clone());
            match reference.field.as_str() {
                "links" => {
                    let item = expected.get_mut("links")
                        .and_then(serde_yaml::Value::as_sequence_mut)
                        .and_then(|links| links.get_mut(reference.occurrence))
                        .and_then(serde_yaml::Value::as_mapping_mut)
                        .ok_or_else(|| anyhow::anyhow!("invalid links occurrence in {path}"))?;
                    item.insert(serde_yaml::Value::String("target".to_string()), target);
                }
                "parent_epic" if reference.occurrence == 0 => {
                    expected.insert("parent_epic".to_string(), target);
                }
                _ => bail!("invalid reference occurrence in {path}"),
            }
        }
        if split_document(before)?.body != split_document(after)?.body
            || effective_frontmatter(after)? != expected
            || canonical_key_from_document(&document.display_id, after) != expected_key
        {
            bail!("document postimage {path} does not preserve its content and planned identity");
        }
    }
    Ok(())
}

/// The file operations a plan implies, with their preimages and postimages.
///
/// This is where the plan is checked against the workspace: every source
/// hash, every reference occurrence, every unresolved reference and every
/// target path is verified here, before anything is written.
fn plan_operations(
    forgeplan_dir: &Path,
    plan: &RepairPlan,
    census: &Census,
) -> Result<Vec<FileOperation>> {
    let mut operations: BTreeMap<String, FileOperation> = BTreeMap::new();

    // The sources: the documents whose canonical key changes.
    for source in &plan.sources {
        // The destination of a source must be free, or hold the very
        // document the plan writes there. The repair never overwrites a
        // document it did not plan for.
        if source.proposed_path != source.relative_path {
            let destination = resolve_document_path(forgeplan_dir, &source.proposed_path)?;
            if destination.exists() {
                bail!(
                    "the target path {} is already occupied — the repair refuses to overwrite a document it did not plan for",
                    source.proposed_path
                );
            }
            bail!(
                "the plan proposes to move {} to {} — the repair rewrites documents in place",
                source.relative_path,
                source.proposed_path
            );
        }
        let absolute = resolve_document_path(forgeplan_dir, &source.relative_path)?;
        let content = read_document(&absolute, &source.relative_path)?;
        let hashes = hash_document(&content)?;
        if hashes.file_sha256 != source.file_sha256
            || hashes.body_sha256 != source.body_sha256
            || hashes.frontmatter_sha256 != source.frontmatter_sha256
        {
            bail!(
                "the document {} changed since the plan was made — the plan is stale",
                source.relative_path
            );
        }
        let updated = set_frontmatter_slug(&content, &source.new_key)
            .with_context(|| format!("rewrite the canonical key of {}", source.relative_path))?;
        operations.insert(
            source.relative_path.clone(),
            FileOperation {
                path: source.relative_path.clone(),
                absolute,
                preimage: Some(content.into_bytes()),
                postimage: Some(updated.into_bytes()),
            },
        );
    }

    // The references: the documents that hold a reference to a changed key.
    let new_key_by_path: BTreeMap<&str, &str> = plan.sources.iter()
        .map(|source| (source.relative_path.as_str(), source.new_key.as_str()))
        .collect();
    let mut post_census = census.clone();
    for document in &mut post_census.documents {
        if let Some(key) = new_key_by_path.get(document.relative_path.as_str()) {
            document.canonical_key = (*key).to_string();
        }
    }
    let mut seen_references = BTreeSet::new();
    for reference in &plan.references {
        if reference.to.is_empty() {
            bail!(
                "the reference {} {} #{} ({}) is unresolved — a review must decide which document it denotes",
                reference.source_path,
                reference.field,
                reference.occurrence,
                reference.from
            );
        }
        if !seen_references.insert((&reference.source_path, &reference.field, reference.occurrence)) {
            bail!("duplicate reference occurrence in {}", reference.source_path);
        }
        let document = census.documents.iter().find(|document| document.relative_path == reference.source_path)
            .ok_or_else(|| anyhow::anyhow!("reference source {} is not in the census", reference.source_path))?;
        if !occurrences_of(document).iter().any(|(field, occurrence, target)| {
            *field == reference.field && *occurrence == reference.occurrence && *target == reference.from
        }) {
            bail!("reference occurrence in {} is stale", reference.source_path);
        }
        let candidates = resolve_reference(census, &reference.from);
        let expected_mapping: BTreeMap<String, String> = candidates.iter().map(|candidate| {
            let key = new_key_by_path.get(candidate.relative_path.as_str()).copied()
                .unwrap_or(&candidate.canonical_key);
            (candidate.relative_path.clone(), key.to_string())
        }).collect();
        if reference.mapping != expected_mapping
            || !candidates.iter().any(|candidate| new_key_by_path.contains_key(candidate.relative_path.as_str()))
            || !expected_mapping.values().any(|key| key == &reference.to)
        {
            bail!("reference mapping in {} does not match its candidates", reference.source_path);
        }
        let resolved = resolve_reference(&post_census, &reference.to);
        if resolved.len() != 1
            || expected_mapping.get(&resolved[0].relative_path).map(String::as_str)
                != Some(reference.to.as_str())
        {
            bail!("reference {} {} #{} remains ambiguous after repair: {:?}",
                reference.source_path, reference.field, reference.occurrence, reference.to);
        }
        let absolute = resolve_document_path(forgeplan_dir, &reference.source_path)?;
        if reference.from == reference.to {
            // Retain the reviewed occurrence in the plan without rewriting its bytes.
            let content = read_document(&absolute, &reference.source_path)?;
            verify_reference_occurrence(&content, reference)?;
            continue;
        }
        if !operations.contains_key(&reference.source_path) {
            let content = read_document(&absolute, &reference.source_path)?;
            operations.insert(
                reference.source_path.clone(),
                FileOperation {
                    path: reference.source_path.clone(),
                    absolute: absolute.clone(),
                    preimage: Some(content.clone().into_bytes()),
                    postimage: Some(content.into_bytes()),
                },
            );
        }
        let entry = operations
            .get_mut(&reference.source_path)
            .expect("the operation was just inserted");
        let current = String::from_utf8(entry.postimage.clone().expect("document postimage"))
            .with_context(|| format!("read {}", reference.source_path))?;
        verify_reference_occurrence(&current, reference)?;
        let updated = match reference.field.as_str() {
            "links" => set_link_target(&current, reference.occurrence, &reference.to)
                .with_context(|| format!("rewrite a reference of {}", reference.source_path))?,
            "parent_epic" => set_parent_epic(&current, &reference.to)
                .with_context(|| format!("rewrite a reference of {}", reference.source_path))?,
            other => bail!("unsupported reference field {other:?}"),
        };
        entry.postimage = Some(updated.into_bytes());
    }

    // Every occurrence that points at a changed key must be resolved by the
    // plan. An unresolved structural reference onto a changed key is
    // refused: the repair does not guess which document a relation meant.
    // `new_key_by_path` was checked against every reference above.
    for document in &census.documents {
        for (field, occurrence, target) in occurrences_of(document) {
            let candidates = resolve_reference(census, target);
            let touches_changed_key = candidates
                .iter()
                .any(|candidate| new_key_by_path.contains_key(candidate.relative_path.as_str()));
            if !touches_changed_key {
                continue;
            }
            let planned = plan.references.iter().find(|reference| {
                reference.source_path == document.relative_path
                    && reference.field == field
                    && reference.occurrence == occurrence
            });
            let Some(planned) = planned else {
                bail!(
                    "the reference {} {} #{} ({}) points at a document whose key changes and the plan does not resolve it",
                    document.relative_path,
                    field,
                    occurrence,
                    target
                );
            };
            if planned.to.is_empty() {
                bail!(
                    "the reference {} {} #{} ({}) points at a document whose key changes and stays unresolved",
                    document.relative_path,
                    field,
                    occurrence,
                    target
                );
            }
            let named = candidates.iter().any(|candidate| {
                new_key_by_path.get(candidate.relative_path.as_str()).copied()
                    .unwrap_or(candidate.canonical_key.as_str()) == planned.to
            });
            if !named {
                bail!(
                    "the plan resolves the reference {} {} #{} to a key no candidate receives",
                    document.relative_path,
                    field,
                    occurrence
                );
            }
        }
    }
    validate_document_postimages(&operations, plan, census)?;


    let mut result: Vec<_> = operations.into_values().collect();
    for sidecar in &plan.sidecar_moves {
        let source = resolve_document_path(forgeplan_dir, &sidecar.source_path)?;
        let destination = resolve_document_path(forgeplan_dir, &sidecar.destination_path)?;
        validate_staging_device(forgeplan_dir, &destination)?;
        let preimage = read_regular_file(&source)?;
        if sha256_hex(&preimage) != sidecar.source_sha256 || read_optional_file(&destination)?.is_some() {
            bail!("sidecar {} changed or destination is occupied", sidecar.source_path);
        }
        let postimage = sidecar_postimage(std::str::from_utf8(&preimage)?, &sidecar.old_key, &sidecar.new_key)?;
        if sha256_hex(&postimage) != sidecar.postimage_sha256 {
            bail!("sidecar {} postimage differs from the plan", sidecar.source_path);
        }
        result.push(FileOperation {
            path: sidecar.destination_path.clone(),
            absolute: destination,
            preimage: None,
            postimage: Some(postimage),
        });
        result.push(FileOperation {
            path: sidecar.source_path.clone(),
            absolute: source,
            preimage: Some(preimage),
            postimage: None,
        });
    }
    Ok(result)
}

/// The collision groups the plan was built on.
fn planned_groups(plan: &RepairPlan) -> BTreeMap<String, Vec<String>> {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for source in &plan.sources {
        groups
            .entry(source.old_key.clone())
            .or_default()
            .push(source.relative_path.clone());
    }
    for members in groups.values_mut() {
        members.sort();
    }
    groups
}

/// The structural occurrences of a document: its `links` entries and its
/// `parent_epic` value.
fn occurrences_of(document: &CensusDocument) -> Vec<(&'static str, usize, &str)> {
    let mut out: Vec<(&'static str, usize, &str)> = Vec::new();
    for entry in &document.links {
        out.push(("links", entry.occurrence, entry.target.as_str()));
    }
    if let Some(target) = &document.parent_epic {
        out.push(("parent_epic", 0, target.as_str()));
    }
    out
}

/// Verify that a reference still says what the plan says it says.
fn verify_reference_occurrence(content: &str, reference: &PlannedReference) -> Result<()> {
    let actual = match reference.field.as_str() {
        "links" => link_entries(content)?
            .get(reference.occurrence)
            .map(|entry| entry.target.clone()),
        "parent_epic" => parent_epic_target(content)?,
        other => bail!("unsupported reference field {other:?}"),
    };
    match actual {
        Some(target) if target == reference.from => Ok(()),
        Some(target) => bail!(
            "the reference {} {} #{} now says {target:?} instead of {:?} — the plan is stale",
            reference.source_path,
            reference.field,
            reference.occurrence,
            reference.from
        ),
        None => bail!(
            "the reference {} {} #{} no longer exists — the plan is stale",
            reference.source_path,
            reference.field,
            reference.occurrence
        ),
    }
}

/// The absolute path of a workspace-relative document path.
///
/// The path must stay inside the workspace: a path that climbs out of it,
/// or that crosses a symlink, is refused.
fn resolve_document_path(forgeplan_dir: &Path, relative_path: &str) -> Result<PathBuf> {
    if relative_path.is_empty() {
        bail!("a document path must not be empty");
    }
    let mut absolute = forgeplan_dir.to_path_buf();
    let mut current = forgeplan_dir.to_path_buf();
    for part in relative_path.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            bail!(
                "refusing a document path that leaves the workspace: {relative_path}"
            );
        }
        absolute.push(part);
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() =>
                bail!("refusing a document path that crosses a symlink: {relative_path}"),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).with_context(|| format!("inspect {}", current.display())),
        }
    }
    Ok(absolute)
}

/// Read a document and refuse to read anything but a regular file.
fn read_document(absolute: &Path, relative_path: &str) -> Result<String> {
    let meta = fs::symlink_metadata(absolute)
        .with_context(|| format!("read document {relative_path}"))?;
    if meta.file_type().is_symlink() {
        bail!("refusing to read a symlinked document: {relative_path}");
    }
    if !meta.is_file() {
        bail!("refusing to read a document that is not a regular file: {relative_path}");
    }
    fs::read_to_string(absolute).with_context(|| format!("read document {relative_path}"))
}

#[cfg(test)]
thread_local! {
    static PREPARATION_FAILURE: std::cell::RefCell<Option<&'static str>> = std::cell::RefCell::new(None);
}

#[cfg(test)]
fn preparation_checkpoint(name: &str) -> Result<()> {
    let fail = PREPARATION_FAILURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.as_deref() == Some(name) {
            *slot = None;
            true
        } else {
            false
        }
    });
    if fail {
        bail!("injected preparation interruption at {name}");
    }
    Ok(())
}

#[cfg(not(test))]
#[inline(always)]
fn preparation_checkpoint(_name: &str) -> Result<()> {
    Ok(())
}

/// Write the preimages of a transaction and fsync them.
fn write_preimages(journal_dir: &Path, operations: &[FileOperation]) -> Result<()> {
    let preimages_dir = journal_dir.join("preimages");
    let postimages_dir = journal_dir.join("postimages");
    fs::create_dir(&postimages_dir)?;
    preparation_checkpoint("postimages_directory")?;
    let staged_dir = journal_dir.join("staged");
    fs::create_dir(&staged_dir)?;
    preparation_checkpoint("staged_directory")?;
    fs::create_dir(&preimages_dir)
        .with_context(|| format!("create {}", preimages_dir.display()))?;
    preparation_checkpoint("preimages_directory")?;
    sync_dir(journal_dir)?;
    preparation_checkpoint("blob_parent_sync")?;
    for operation in operations {
        if let Some(preimage) = &operation.preimage {
            write_file_atomically(&preimage_file(journal_dir, &operation.path), preimage)?;
            preparation_checkpoint("preimage_blob")?;
        }
        if let Some(postimage) = &operation.postimage {
            write_file_atomically(&postimage_file(journal_dir, &operation.path), postimage)?;
            preparation_checkpoint("postimage_blob")?;
            if operation.preimage.is_none() {
                write_file_atomically(&staged_file(journal_dir, &operation.path), postimage)?;
                preparation_checkpoint("create_blob")?;
            }
        }
    }
    sync_dir(&preimages_dir)?;
    preparation_checkpoint("preimages_sync")?;
    sync_dir(&postimages_dir)?;
    preparation_checkpoint("postimages_sync")?;
    sync_dir(&staged_dir)?;
    preparation_checkpoint("staged_sync")?;
    Ok(())
}

/// Write the manifest of a transaction and fsync it.
fn write_manifest(journal_dir: &Path, journal: &mut RepairJournal) -> Result<()> {
    journal.manifest_sha256 = compute_journal_digest(journal);
    let mut bytes = serde_json::to_vec_pretty(journal)?;
    bytes.push(b'\n');
    write_file_atomically(&journal_dir.join("manifest.json"), &bytes)?;
    sync_dir(journal_dir)?;
    Ok(())
}

/// Restore the preimages of the files a failed transaction published.
fn restore_preimages(journal_dir: &Path, operations: &[FileOperation], published: &[String]) -> (Vec<String>, Vec<String>) {
    let mut restored = Vec::new();
    let mut failures = Vec::new();
    for operation in operations.iter().rev() {
        if !published.contains(&operation.path) {
            continue;
        }
        let result = (|| -> Result<()> {
            let current = read_optional_file(&operation.absolute)?;
            if current == operation.preimage {
                sync_dir(operation.absolute.parent().unwrap_or_else(|| Path::new(".")))?;
                return Ok(());
            }
            if current != operation.postimage {
                bail!("{} has an unrelated edit", operation.path);
            }
            match (&current, &operation.preimage) {
                (None, Some(bytes)) => publish_new(&operation.absolute, bytes),
                (Some(_), Some(bytes)) => publish(&operation.absolute, bytes),
                (Some(_), None) => {
                    require_owned_create(journal_dir, &operation.path, &operation.absolute)?;
                    remove_file_and_sync(&operation.absolute)
                }
                (None, None) => unreachable!(),
            }
        })();
        match result {
            Ok(()) => restored.push(operation.path.clone()),
            Err(error) => {
                failures.push(format!("{}: {error:#}", operation.path));
                break;
            }
        }
    }
    (restored, failures)
}

fn read_optional_file(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            require_directory(path.parent().unwrap_or_else(|| Path::new(".")))?;
            Ok(None)
        }
        Err(error) => Err(error).with_context(|| format!("inspect {}", path.display())),
        Ok(_) => read_regular_file(path).map(Some),
    }
}

#[cfg(unix)]
fn validate_staging_device(forgeplan_dir: &Path, destination: &Path) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let root = repair_journal_root(forgeplan_dir);
    let journal_parent = match fs::symlink_metadata(&root) {
        Ok(_) => {
            require_directory(&root)?;
            root.as_path()
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => forgeplan_dir,
        Err(error) => return Err(error.into()),
    };
    let destination_parent = destination.parent().ok_or_else(|| anyhow::anyhow!("destination has no parent"))?;
    require_directory(destination_parent)?;
    if fs::metadata(journal_parent)?.dev() != fs::metadata(destination_parent)?.dev() {
        bail!("sidecar destination is on a different filesystem from the repair journal");
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_staging_device(_forgeplan_dir: &Path, _destination: &Path) -> Result<()> {
    bail!("sidecar repair requires filesystem ownership verification on this platform")
}

fn publish_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist_noclobber(path)
        .map_err(|error| error.error)
        .with_context(|| format!("create {}", path.display()))?;
    sync_dir(parent)
}

fn remove_file_and_sync(path: &Path) -> Result<()> {
    fs::remove_file(path).with_context(|| format!("remove {}", path.display()))?;
    sync_dir(path.parent().unwrap_or_else(|| Path::new(".")))
}

/// Write bytes to a path atomically: a temporary file in the same
/// directory, fsynced, then renamed over the target.
fn write_file_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("create {}", parent.display()))?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("create temporary file in {}", parent.display()))?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() =>
            bail!("refusing to replace non-regular file {}", path.display()),
        Ok(metadata) => temp.as_file().set_permissions(metadata.permissions())?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).with_context(|| format!("inspect {}", path.display())),
    }
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|error| error.error)
        .with_context(|| format!("write {}", path.display()))?;
    sync_dir(parent)?;
    Ok(())
}

/// Publish one post-image atomically.
///
/// The write goes to a temporary file in the same directory, is fsynced,
/// and is then renamed over the target. No `git mv`, no staging, no
/// in-place truncation: a reader either sees the old bytes or the new ones.
fn publish(absolute: &Path, bytes: &[u8]) -> Result<()> {
    write_file_atomically(absolute, bytes)
}

/// Publish a complete journal directory without replacing an existing name.
#[cfg(any(target_os = "macos", target_os = "ios", target_os = "linux"))]
fn publish_directory_noclobber(source: &Path, destination: &Path) -> Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let source = CString::new(source.as_os_str().as_bytes())?;
    let destination = CString::new(destination.as_os_str().as_bytes())?;
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    // SAFETY: both C strings are NUL-terminated and remain alive for the call.
    let result = unsafe {
        libc::renameatx_np(libc::AT_FDCWD, source.as_ptr(), libc::AT_FDCWD, destination.as_ptr(), libc::RENAME_EXCL)
    };
    #[cfg(target_os = "linux")]
    // SAFETY: both C strings are NUL-terminated and remain alive for the call.
    let result = unsafe {
        libc::renameat2(libc::AT_FDCWD, source.as_ptr(), libc::AT_FDCWD, destination.as_ptr(), libc::RENAME_NOREPLACE)
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error()).context("publish prepared repair directory without replacement");
    }
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "linux")))]
fn publish_directory_noclobber(_source: &Path, _destination: &Path) -> Result<()> {
    bail!("atomic no-replace directory publication is unsupported on this platform")
}

/// Explicitly publish a fully prepared, unmutated transaction after a crash.
/// Incomplete or foreign staging is retained and reported, never removed.
pub fn recover_prepared_repair(forgeplan_dir: &Path, id: &str) -> Result<()> {
    validate_transaction_id(id)?;
    let _guard = acquire_repair_lock(forgeplan_dir)?;
    let workspace = fs::canonicalize(forgeplan_dir)?;
    let staging_root = repair_staging_root(&workspace);
    let stage = staging_root.join(id);
    let journal_root = repair_journal_root(&workspace);
    let published = repair_journal_dir(&workspace, id);
    require_directory(&staging_root)?;
    require_directory(&journal_root)?;
    let staged_exists = match fs::symlink_metadata(&stage) {
        Ok(_) => { require_directory(&stage)?; true }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error.into()),
    };
    let published_exists = match fs::symlink_metadata(&published) {
        Ok(_) => { require_directory(&published)?; true }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error.into()),
    };
    if staged_exists == published_exists {
        bail!("prepared transaction {id} has missing or conflicting staging and journal paths");
    }
    let dir = if staged_exists { &stage } else { &published };
    let receipt: PreparedRepairReceipt = serde_json::from_slice(&read_regular_file(&dir.join("READY.json"))?)
        .with_context(|| format!("transaction {id} has incomplete or unverified staging"))?;
    let journal: RepairJournal = serde_json::from_slice(&read_regular_file(&dir.join("manifest.json"))?)?;
    validate_journal(&workspace, id, &journal)?;
    if receipt.version != 1 || receipt.transaction_id != id
        || receipt.workspace_path != workspace.to_string_lossy()
        || !valid_hash(&receipt.plan_digest)
        || receipt.manifest_digest != journal.manifest_sha256
        || journal.status != JournalStatus::InProgress
        || journal.phase != Some(RepairPhase::Applying)
        || journal.operations.iter().any(|operation| operation.state != OperationState::Pending)
    {
        bail!("prepared transaction {id} has an invalid or progressed manifest");
    }
    require_directory(&dir.join("preimages"))?;
    validate_postimage_blobs(dir, &journal)?;
    for operation in &journal.operations {
        if let Some(expected) = journal.preimages.get(&operation.path) {
            let saved = preimage_file(dir, &operation.path);
            if sha256_hex(&read_regular_file(&saved)?) != *expected {
                bail!("prepared transaction {id} has a damaged preimage for {}", operation.path);
            }
            fs::File::open(saved)?.sync_all()?;
        }
        if let Some(expected) = journal.planned_postimages.get(&operation.path) {
            let saved = postimage_file(dir, &operation.path);
            if sha256_hex(&read_regular_file(&saved)?) != *expected {
                bail!("prepared transaction {id} has a damaged postimage for {}", operation.path);
            }
            fs::File::open(saved)?.sync_all()?;
            if !journal.preimages.contains_key(&operation.path) {
                let stage_file = staged_file(dir, &operation.path);
                if sha256_hex(&read_regular_file(&stage_file)?) != *expected {
                    bail!("prepared transaction {id} has a damaged create image for {}", operation.path);
                }
                fs::File::open(stage_file)?.sync_all()?;
            }
        }
        let absolute = resolve_document_path(&workspace, &operation.path)?;
        let current = read_optional_file(&absolute)?.map(|bytes| sha256_hex(&bytes));
        if current.as_ref() != journal.preimages.get(&operation.path) {
            bail!("prepared transaction {id} source or destination changed at {}", operation.path);
        }
    }
    if staged_exists {
        match journal.legacy_predecessors.as_slice() {
            [] => check_repair_journals_inner(&workspace, None, Some(id))?,
            [previous] => {
                if validate_blocked_repair_admission_inner(&workspace, &previous.transaction_id, Some(id))?
                    != previous.blocked_manifest_sha256
                {
                    bail!("prepared transaction {id} predecessor changed");
                }
                let old = read_valid_journal(&workspace, &previous.transaction_id)?;
                let diagnostic = old.legacy_diagnostic.as_ref().ok_or_else(|| anyhow::anyhow!("missing predecessor receipt"))?;
                crate::artifact::legacy_diagnostic::verify_stored_snapshot_integrity(
                    &workspace, &previous.transaction_id, &diagnostic.snapshot_digest,
                    &diagnostic.prior_manifest_sha256, &diagnostic.source_digest,
                )?;
            }
            _ => bail!("prepared transaction {id} has an unsupported predecessor chain"),
        }
        for name in ["preimages", "postimages", "staged"] {
            sync_dir(&stage.join(name))?;
        }
        fs::File::open(stage.join("manifest.json"))?.sync_all()?;
        fs::File::open(stage.join("READY.json"))?.sync_all()?;
        sync_dir(&stage)?;
        publish_directory_noclobber(&stage, &published)?;
    } else {
        check_repair_journals_inner(&workspace, Some(id), None)?;
    }
    sync_dir(&staging_root)?;
    sync_dir(&journal_root)?;
    Ok(())
}

/// fsync a directory so that a rename inside it survives a crash.
#[cfg(unix)]
fn sync_dir(dir: &Path) -> Result<()> {
    fs::File::open(dir)
        .with_context(|| format!("open directory {} for fsync", dir.display()))?
        .sync_all()
        .with_context(|| format!("fsync directory {}", dir.display()))
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> Result<()> {
    refuse_unpinned_non_unix_mutation()
}

/// The Git identity of the workspace, when it sits inside a repository.
///
/// The identity is the commit `HEAD` points at and the path of the Git
/// common directory — the shared state of linked worktrees. A plan made
/// outside Git carries no identity, and the apply then refuses to run
/// against a workspace that has become a Git repository (or the other way
/// round).
pub fn detect_git_identity(forgeplan_dir: &Path) -> Result<Option<GitIdentity>> {
    detect_git_identity_with(forgeplan_dir, std::ffi::OsStr::new("git"))
}

fn detect_git_identity_with(forgeplan_dir: &Path, git: &std::ffi::OsStr) -> Result<Option<GitIdentity>> {
    let workspace = fs::canonicalize(forgeplan_dir)
        .with_context(|| format!("resolve workspace {}", forgeplan_dir.display()))?;
    let run = |args: &[&str]| -> Result<std::process::Output> {
        std::process::Command::new(git)
            .arg("-C")
            .arg(&workspace)
            .env("LC_ALL", "C")
            .args(args)
            .output()
            .with_context(|| format!("run Git in {}", workspace.display()))
    };
    let probe = run(&["rev-parse", "--git-dir"])?;
    if !probe.status.success() {
        let stderr = String::from_utf8_lossy(&probe.stderr);
        // Git uses the same diagnostic for some broken repositories. Only
        // accept it as non-Git when no repository marker or explicit Git
        // directory can have been involved in discovery.
        let no_explicit_git = ["GIT_DIR", "GIT_COMMON_DIR", "GIT_WORK_TREE"]
            .iter()
            .all(|name| std::env::var_os(name).is_none());
        if no_explicit_git
            && stderr.trim() == "fatal: not a git repository (or any of the parent directories): .git"
        {
            let mut marker_found = false;
            for ancestor in workspace.ancestors() {
                match fs::symlink_metadata(ancestor.join(".git")) {
                    Ok(_) => {
                        marker_found = true;
                        break;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error).context("inspect Git repository marker"),
                }
            }
            if !marker_found {
                return Ok(None);
            }
        }
        bail!("Git repository discovery failed: {}", stderr.trim());
    }

    let common = run(&["rev-parse", "--path-format=absolute", "--git-common-dir"])?;
    if !common.status.success() {
        bail!("Git common directory lookup failed: {}", String::from_utf8_lossy(&common.stderr).trim());
    }
    let common_output = std::str::from_utf8(&common.stdout)
        .context("Git common directory is not UTF-8")?;
    let common_text = common_output.strip_suffix('\n').unwrap_or(common_output);
    let common_text = common_text.strip_suffix('\r').unwrap_or(common_text);
    if common_text.is_empty() {
        bail!("Git returned an empty common directory");
    }
    let common_dir = fs::canonicalize(common_text)
        .with_context(|| format!("resolve Git common directory {common_text}"))?;

    let head = run(&["rev-parse", "--verify", "HEAD^{commit}"])?;
    if !head.status.success() {
        bail!("cannot resolve Git HEAD (unborn or invalid); identity repair is refused: {}",
            String::from_utf8_lossy(&head.stderr).trim());
    }
    let head = std::str::from_utf8(&head.stdout).context("Git HEAD is not UTF-8")?.trim();
    if head.is_empty() {
        bail!("Git returned an empty HEAD");
    }
    Ok(Some(GitIdentity {
        head: head.to_string(),
        common_dir: common_dir.to_str().ok_or_else(|| anyhow::anyhow!("Git common directory is not UTF-8"))?.to_string(),
    }))
}

#[cfg(not(unix))]
fn refuse_unpinned_non_unix_mutation() -> Result<()> {
    bail!("identity repair is unavailable on non-Unix platforms until journal and index operations use pinned, no-reparse handles with durable namespace barriers")
}

/// The lock the repair takes on the workspace.
///
/// It is the same `.forgeplan/.lock` the rest of the workspace uses, so a
/// repair never runs beside another writer.
fn acquire_repair_lock(forgeplan_dir: &Path) -> Result<WorkspaceLock> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_mutation()?;
    const LOCK_TIMEOUT: Duration = Duration::from_secs(30);
    const MAX_DELAY: Duration = Duration::from_millis(1000);
    require_directory(forgeplan_dir)?;

    let path = lock_path(forgeplan_dir);
    if let Ok(meta) = fs::symlink_metadata(&path) {
        if meta.file_type().is_symlink() {
            bail!(
                "the workspace lock {} is a symlink — refusing to open it",
                path.display()
            );
        }
    }
    let mut options = fs::OpenOptions::new();
    options.create(true).write(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options
        .open(&path)
        .with_context(|| format!("open the workspace lock {}", path.display()))?;
    let start = std::time::Instant::now();
    let mut delay = Duration::from_millis(10);
    loop {
        match fs2::FileExt::try_lock_exclusive(&file) {
            Ok(()) => {
                let guard = WorkspaceLock::from_file(file);
                let workspace = forgeplan_dir.canonicalize()?;
                crate::db::migrate::check_legacy_semantic_rebuild(&workspace)?;
                return Ok(guard);
            }
            Err(error) => {
                if error.kind() != std::io::ErrorKind::WouldBlock {
                    bail!(
                        "could not acquire the workspace lock {}: {error}",
                        path.display()
                    );
                }
                if start.elapsed() >= LOCK_TIMEOUT {
                    bail!(
                        "the workspace lock {} is held by another writer; timed out after {:?}",
                        path.display(),
                        LOCK_TIMEOUT
                    );
                }
                std::thread::sleep(delay);
                delay = std::cmp::min(delay * 2, MAX_DELAY);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(unix))]
    #[test]
    fn non_unix_repair_refuses_before_creating_lock_or_journal() {
        let workspace = tempfile::tempdir().unwrap();
        assert!(acquire_repair_lock(workspace.path()).is_err());
        assert!(recover_prepared_repair(workspace.path(), "repair-1").is_err());
        assert!(rollback_repair(workspace.path(), "repair-1").is_err());
        assert_eq!(fs::read_dir(workspace.path()).unwrap().count(), 0);
    }

    #[cfg(not(unix))]
    #[test]
    fn non_unix_rejects_both_legacy_proofs_before_inspecting_paths() {
        let workspace = tempfile::tempdir().unwrap();
        let guard = WorkspaceLock::from_file(fs::File::create(workspace.path().join("test.lock")).unwrap());
        let missing = workspace.path().join("missing");
        let pending = validate_legacy_repair(&missing, &guard, "repair-1").err().unwrap();
        let blocked = validate_blocked_legacy_repair(&missing, &guard, "repair-1").err().unwrap();
        assert!(pending.to_string().contains("identity repair is unavailable on non-Unix platforms"));
        assert!(blocked.to_string().contains("identity repair is unavailable on non-Unix platforms"));
    }

    const TWO_BLOCKS: &str = "---\nid: PRD-001\nkind: prd\ntitle: First\n---\n---\nid: PRD-001\nkind: prd\nslug: prd-first\nassigned_number: 1\n---\n\n# Body\n\nText with PRD-001 inside.\n";

    #[test]
    fn plan_digest_covers_every_field() {
        let mut plan = RepairPlan {
            version: REPAIR_PLAN_VERSION,
            workspace_path: "/tmp/ws/.forgeplan".to_string(),
            git: Some(GitIdentity {
                head: "abc".to_string(),
                common_dir: "/tmp/ws/.git".to_string(),
            }),
            inventory_digest: "inv".to_string(),
            sources: vec![PlannedSource {
                relative_path: "prds/PRD-001-a.md".to_string(),
                file_sha256: "f".to_string(),
                body_sha256: "b".to_string(),
                frontmatter_sha256: "fm".to_string(),
                old_key: "PRD-001".to_string(),
                new_key: "prd-a-k00".to_string(),
                proposed_path: "prds/PRD-001-a.md".to_string(),
            }],
            references: vec![PlannedReference {
                source_path: "evidence/EVID-004.md".to_string(),
                field: "links".to_string(),
                occurrence: 0,
                from: "PRD-001".to_string(),
                to: "prd-a-k00".to_string(),
                mapping: BTreeMap::from([(
                    "prds/PRD-001-a.md".to_string(),
                    "prd-a-k00".to_string(),
                )]),
            }],
            sidecar_moves: Vec::new(),
            sidecar_conflicts: Vec::new(),
            digest: String::new(),
        };
        plan.digest = compute_plan_digest(&plan);
        assert!(plan_digest_matches(&plan));

        // Every field participates in the digest.
        let baseline = plan.digest.clone();
        let mut changed = plan.clone();
        changed.workspace_path = "/tmp/other/.forgeplan".to_string();
        assert_ne!(compute_plan_digest(&changed), baseline);
        changed = plan.clone();
        changed.git = None;
        assert_ne!(compute_plan_digest(&changed), baseline);
        changed = plan.clone();
        changed.inventory_digest = "other".to_string();
        assert_ne!(compute_plan_digest(&changed), baseline);
        changed = plan.clone();
        changed.sources[0].new_key = "prd-b-k00".to_string();
        assert_ne!(compute_plan_digest(&changed), baseline);
        changed = plan.clone();
        changed.references[0].to = "prd-b-k00".to_string();
        assert_ne!(compute_plan_digest(&changed), baseline);
        changed = plan.clone();
        changed.references[0].occurrence = 3;
        assert_ne!(compute_plan_digest(&changed), baseline);
    }

    #[test]
    fn split_document_separates_nested_frontmatter_from_body() {
        let parts = split_document(TWO_BLOCKS).unwrap();
        assert!(parts.frontmatter.contains("slug: prd-first"));
        assert!(!parts.body.contains("slug: prd-first"));
        assert!(parts.body.starts_with("# Body"));
        assert!(parts.body.contains("Text with PRD-001 inside."));
    }

    #[test]
    fn hash_document_detects_a_change_in_any_part() {
        let hashes = hash_document(TWO_BLOCKS).unwrap();
        assert_eq!(hashes.file_sha256.len(), 64);
        assert_eq!(hashes.body_sha256.len(), 64);
        assert_eq!(hashes.frontmatter_sha256.len(), 64);

        let body_changed = TWO_BLOCKS.replace("Text with PRD-001 inside.", "Text changed.");
        assert_ne!(
            hash_document(&body_changed).unwrap().body_sha256,
            hashes.body_sha256
        );
        assert_ne!(
            hash_document(&body_changed).unwrap().file_sha256,
            hashes.file_sha256
        );

        let frontmatter_changed = TWO_BLOCKS.replace("slug: prd-first", "slug: prd-second");
        assert_ne!(
            hash_document(&frontmatter_changed).unwrap().frontmatter_sha256,
            hashes.frontmatter_sha256
        );
    }

    #[test]
    fn set_frontmatter_slug_rewrites_the_nested_block_in_place() {
        let updated = set_frontmatter_slug(TWO_BLOCKS, "prd-first-k00").unwrap();
        let fm = effective_frontmatter(&updated).unwrap();
        assert_eq!(
            fm.get("slug").and_then(|v| v.as_str()),
            Some("prd-first-k00")
        );
        // The body and every other byte survive.
        let before = split_document(TWO_BLOCKS).unwrap();
        let after = split_document(&updated).unwrap();
        assert_eq!(before.body, after.body);
        assert!(updated.contains("id: PRD-001"));
        assert!(updated.contains("assigned_number: 1"));
        assert!(updated.contains("title: First"));
    }

    #[test]
    fn set_frontmatter_slug_adds_a_slug_when_the_document_has_none() {
        let plain = "---\nid: PRD-002\nkind: prd\n---\n\nBody\n";
        let updated = set_frontmatter_slug(plain, "prd-two-k00").unwrap();
        let fm = effective_frontmatter(&updated).unwrap();
        assert_eq!(fm.get("slug").and_then(|v| v.as_str()), Some("prd-two-k00"));
        assert_eq!(
            split_document(&updated).unwrap().body,
            split_document(plain).unwrap().body
        );
    }

    #[test]
    fn link_entries_are_numbered_in_document_order() {
        let doc = "---\nid: EVID-004\nkind: evid\nlinks:\n- target: PRD-001\n  relation: informs\n- target: RFC-002\n  relation: based_on\n---\n\nBody\n";
        let entries = link_entries(doc).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].occurrence, 0);
        assert_eq!(entries[0].target, "PRD-001");
        assert_eq!(entries[0].relation, "informs");
        assert_eq!(entries[1].occurrence, 1);
        assert_eq!(entries[1].target, "RFC-002");
    }

    #[test]
    fn set_link_target_rewrites_only_the_named_occurrence() {
        let doc = "---\nid: EVID-004\nkind: evid\nlinks:\n- target: PRD-001\n  relation: informs\n- target: RFC-002\n  relation: based_on\n---\n\nBody\n";
        let updated = set_link_target(doc, 1, "rfc-two-k00").unwrap();
        let entries = link_entries(&updated).unwrap();
        assert_eq!(entries[0].target, "PRD-001");
        assert_eq!(entries[1].target, "rfc-two-k00");
        assert_eq!(entries[1].relation, "based_on");
        assert_eq!(
            split_document(&updated).unwrap().body,
            split_document(doc).unwrap().body
        );
    }

    #[test]
    fn parent_epic_is_read_and_rewritten_in_place() {
        let doc = "---\nid: RFC-001\nkind: rfc\nparent_epic: PRD-009\n---\n\nBody\n";
        assert_eq!(parent_epic_target(doc).unwrap().as_deref(), Some("PRD-009"));
        let updated = set_parent_epic(doc, "prd-nine-k00").unwrap();
        assert_eq!(
            parent_epic_target(&updated).unwrap().as_deref(),
            Some("prd-nine-k00")
        );
        assert!(updated.contains("id: RFC-001"));
    }

    #[test]
    fn quoted_references_use_yaml_values_and_preserve_comments() {
        let doc = "---\nid: RFC-001\nkind: rfc\nparent_epic: ' EPIC-7 ' # parent comment\nlinks:\n  - target: \" PRD-1 \" # target comment\n    relation: informs\n---\n\nBody\n";
        assert_eq!(parent_epic_target(doc).unwrap().as_deref(), Some("EPIC-7"));
        assert_eq!(link_entries(doc).unwrap()[0].target, "PRD-1");
        let changed = set_link_target(doc, 0, "prd-new-k00").unwrap();
        let changed = set_parent_epic(&changed, "epic-new-k00").unwrap();
        assert!(changed.contains("target: prd-new-k00 # target comment"));
        assert!(changed.contains("parent_epic: epic-new-k00 # parent comment"));
        assert_eq!(split_document(&changed).unwrap().body, "Body\n");
    }

    #[test]
    fn flow_style_links_are_censused_but_never_rewritten_by_guessing_offsets() {
        let doc = "---\nid: RFC-001\nkind: rfc\nlinks: [{target: 'PRD-1', relation: informs}]\n---\n\nBody\n";
        assert_eq!(link_entries(doc).unwrap()[0].target, "PRD-1");
        assert!(set_link_target(doc, 0, "prd-new-k00").is_err());
    }
    #[test]
    fn journal_digest_covers_the_operation_state() {
        let mut journal = RepairJournal {
            version: None,
            transaction_id: "txn-1".to_string(),
            manifest_sha256: String::new(),
            preimages: BTreeMap::from([("prds/a.md".to_string(), "aa".to_string())]),
            planned_postimages: BTreeMap::from([("prds/a.md".to_string(), "bb".to_string())]),
            operations: vec![JournalOperation {
                path: "prds/a.md".to_string(),
                state: OperationState::Pending,
            }],
            status: JournalStatus::InProgress,
            phase: None,
            moves: Vec::new(),
            legacy_diagnostic: None,
            legacy_predecessors: Vec::new(),
        };
        journal.manifest_sha256 = compute_journal_digest(&journal);
        assert!(journal_digest_matches(&journal));
        let baseline = journal.manifest_sha256.clone();
        journal.operations[0].state = OperationState::Published;
        assert_ne!(compute_journal_digest(&journal), baseline);
        assert!(!journal.is_committed());
        journal.status = JournalStatus::CanonicalCommitted;
        assert!(journal.is_committed());
    }

    #[test]
    fn an_unresolved_reference_is_not_resolved() {
        let mut reference = PlannedReference {
            source_path: "evidence/EVID-004.md".to_string(),
            field: "links".to_string(),
            occurrence: 0,
            from: "PRD-001".to_string(),
            to: String::new(),
            mapping: BTreeMap::from([
                ("prds/a.md".to_string(), "prd-a-k00".to_string()),
                ("prds/b.md".to_string(), "prd-b-k00".to_string()),
            ]),
        };
        // No decision: the occurrence stays unresolved.
        assert!(!reference.is_resolved());
        // A decision that names a key no candidate receives is no decision.
        reference.to = "prd-c-k00".to_string();
        assert!(!reference.is_resolved());
        // A reviewed decision that names one of the candidates resolves it.
        reference.to = "prd-a-k00".to_string();
        assert!(reference.is_resolved());
        // A single candidate resolves mechanically.
        reference.mapping.remove("prds/b.md");
        assert!(reference.is_resolved());
    }

    // -----------------------------------------------------------------
    // The census
    // -----------------------------------------------------------------

    /// A minimal workspace skeleton. Returns the temp directory; the
    /// workspace itself is `<temp>/.forgeplan`.
    fn temp_workspace(files: &[(&str, &str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let forgeplan = dir.path().join(".forgeplan");
        for kind in REPAIR_KINDS {
            fs::create_dir_all(forgeplan.join(kind.dir_name())).expect("kind dir");
        }
        for (subdir, name, content) in files {
            fs::write(forgeplan.join(subdir).join(name), content).expect("write document");
        }
        dir
    }

    /// A document with a display id and a title, and no slug.
    fn document(id: &str, kind: &str, title: &str) -> String {
        format!("---\nid: {id}\nkind: {kind}\ntitle: {title}\n---\n\n# {id}\n\nBody of {id}.\n")
    }

    /// A document that links to one target.
    fn linking_document(id: &str, kind: &str, target: &str) -> String {
        format!(
            "---\nid: {id}\nkind: {kind}\ntitle: {id}\nlinks:\n- target: {target}\n  relation: informs\n---\n\n# {id}\n\nBody of {id}.\n"
        )
    }

    #[test]
    fn census_reads_every_document_and_derives_its_key() {
        let workspace = temp_workspace(&[
            ("prds", "PRD-001-a.md", &document("PRD-001", "prd", "First")),
            ("prds", "PRD-001-b.md", &document("PRD-001", "prd", "Second")),
            (
                "evidence",
                "EVID-001.md",
                &linking_document("EVID-001", "evidence", "PRD-001"),
            ),
        ]);
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).expect("census");

        assert_eq!(census.documents.len(), 3);
        let first = census
            .documents
            .iter()
            .find(|document| document.relative_path == "prds/PRD-001-a.md")
            .expect("first prd");
        assert_eq!(first.display_id, "PRD-001");
        // A document without a slug keeps its display id as its key.
        assert_eq!(first.canonical_key, "PRD-001");
        assert_eq!(first.kind, ArtifactKind::Prd);
        assert_eq!(first.title, "First");
        assert_eq!(first.hashes.file_sha256.len(), 64);

        let evidence = census
            .documents
            .iter()
            .find(|document| document.relative_path == "evidence/EVID-001.md")
            .expect("evidence");
        assert_eq!(evidence.links.len(), 1);
        assert_eq!(evidence.links[0].target, "PRD-001");
        assert_eq!(evidence.links[0].occurrence, 0);
        assert_eq!(evidence.kind, ArtifactKind::EvidencePack);

        // Two documents claim one key: one collision group of two members.
        let groups = collision_groups(&census);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups.get("PRD-001").expect("group").len(), 2);
    }

    #[test]
    fn census_reads_the_slug_of_a_nested_frontmatter_block() {
        let nested = "---\nid: PRD-077\nkind: prd\ntitle: Nested\n---\n\n---\nslug: prd-nested\nassigned_number: 77\n---\n\n# Body\n";
        let workspace = temp_workspace(&[("prds", "PRD-077.md", nested)]);
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).expect("census");
        assert_eq!(census.documents.len(), 1);
        assert_eq!(census.documents[0].canonical_key, "prd-nested");
    }

    #[cfg(unix)]
    #[test]
    fn census_refuses_a_symlinked_document() {
        let workspace = temp_workspace(&[]);
        let forgeplan = workspace.path().join(".forgeplan");
        let real = workspace.path().join("real.md");
        fs::write(&real, document("PRD-009", "prd", "Real")).expect("write");
        std::os::unix::fs::symlink(&real, forgeplan.join("prds/PRD-009.md")).expect("symlink");
        let error = census_workspace(&forgeplan).expect_err("a symlink must be refused");
        assert!(
            error.to_string().contains("symlink"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn the_inventory_digest_covers_every_document() {
        let workspace = temp_workspace(&[(
            "prds",
            "PRD-001.md",
            &document("PRD-001", "prd", "First"),
        )]);
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).expect("census");
        let before = census.inventory_digest.clone();
        assert_eq!(
            before,
            compute_inventory_digest(&census.documents),
            "the census digest is the digest over its documents"
        );
        fs::write(
            forgeplan.join("prds/PRD-001.md"),
            document("PRD-001", "prd", "Second"),
        )
        .expect("write");
        let after = census_workspace(&forgeplan).expect("census").inventory_digest;
        assert_ne!(before, after, "an edited document must change the digest");
    }

    #[cfg(unix)]
    #[test]
    fn inventory_framing_distinguishes_the_two_corpus_path_collision() {
        let first = document("NOTE-001", "note", "First");
        let second = document("NOTE-002", "note", "Second");
        let a = temp_workspace(&[("notes", "a.md", &first), ("notes", "b.md", &second)]);
        let b = temp_workspace(&[]);
        let b_root = b.path().join(".forgeplan");
        let a_hash = sha256_hex(first.as_bytes());
        let b_hash = sha256_hex(second.as_bytes());
        let crafted = format!("notes/a.md\t{a_hash}\nnotes/b.md");
        let path = b_root.join(&crafted);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, &second).unwrap();
        let old_a = format!("notes/a.md\t{a_hash}\nnotes/b.md\t{b_hash}");
        let old_b = format!("{crafted}\t{b_hash}");
        assert_eq!(old_a.as_bytes(), old_b.as_bytes());
        let census_a = census_workspace(&a.path().join(".forgeplan")).unwrap();
        let census_b = census_workspace(&b_root).unwrap();
        assert_ne!(census_a.inventory_digest, census_b.inventory_digest);
    }

    #[test]
    fn inventory_framing_distinguishes_sidecar_path_collision() {
        let first_hash = sha256_hex(b"first");
        let second_hash = sha256_hex(b"second");
        let sidecar = |path: String, hash: String| CensusSidecar {
            relative_path: path, file_sha256: hash, references: Vec::new(),
        };
        let a = vec![
            sidecar("claims/a.yaml".into(), first_hash.clone()),
            sidecar("claims/b.yaml".into(), second_hash.clone()),
        ];
        let path = format!("claims/a.yaml\t{first_hash}\nclaims/b.yaml");
        let b = vec![sidecar(path.clone(), second_hash.clone())];
        assert_eq!(
            format!("claims/a.yaml\t{first_hash}\nclaims/b.yaml\t{second_hash}\n"),
            format!("{path}\t{second_hash}\n")
        );
        assert_ne!(inventory_digest(&[], &a), inventory_digest(&[], &b));
    }

    #[test]
    fn old_plan_and_unversioned_receipt_digest_are_rejected() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let mut plan = build_repair_plan(&forgeplan, &census, &[]).unwrap();
        plan.inventory_digest = "a".repeat(64);
        plan.digest = compute_plan_digest(&plan);
        let path = workspace.path().join("old-plan.json");
        write_plan_json(&plan, &path).unwrap();
        assert!(read_plan_json(&path).unwrap_err().to_string().contains("unsupported repair inventory digest version"));
        assert!(apply_repair_plan(&forgeplan, &plan).unwrap_err().to_string().contains("unsupported repair inventory digest version"));
        assert!(!valid_inventory_digest(&"a".repeat(64)));
        assert!(valid_inventory_digest(&census.inventory_digest));
    }

    #[test]
    fn legacy_journal_receipt_with_unversioned_inventory_fails_closed() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let mut journal = read_valid_journal(&forgeplan, &outcome.transaction_id).unwrap();
        journal.legacy_diagnostic = Some(LegacyDiagnosticReceipt {
            prior_manifest_sha256: "a".repeat(64),
            source_digest: "b".repeat(64),
            snapshot_digest: "c".repeat(64),
            successor: None,
        });
        journal.manifest_sha256 = compute_journal_digest(&journal);
        let error = validate_journal(&forgeplan, &outcome.transaction_id, &journal).unwrap_err();
        assert!(error.to_string().contains("invalid diagnostic receipt"));
        journal.legacy_diagnostic.as_mut().unwrap().source_digest = census.inventory_digest;
        journal.manifest_sha256 = compute_journal_digest(&journal);
        validate_journal(&forgeplan, &outcome.transaction_id, &journal).unwrap();
    }

    // -----------------------------------------------------------------
    // The plan
    // -----------------------------------------------------------------

    /// Two PRDs that share a display id, plus one evidence document that
    /// links to it.
    fn colliding_workspace() -> tempfile::TempDir {
        temp_workspace(&[
            ("prds", "PRD-001-a.md", &document("PRD-001", "prd", "First")),
            ("prds", "PRD-001-b.md", &document("PRD-001", "prd", "Second")),
            ("prds", "PRD-002.md", &document("PRD-002", "prd", "Other")),
            (
                "evidence",
                "EVID-001.md",
                &linking_document("EVID-001", "evidence", "PRD-001"),
            ),
        ])
    }

    /// The review decision that attributes the link to the second PRD.
    fn decision_for_second_prd() -> Vec<ReferenceDecision> {
        vec![ReferenceDecision {
            source_path: "evidence/EVID-001.md".to_string(),
            occurrence: Some(0), source_sha256: Some(sha256_hex(linking_document("EVID-001", "evidence", "PRD-001").as_bytes())),
            field: "links".to_string(),
            from: "PRD-001".to_string(),
            target_path: "prds/PRD-001-b.md".to_string(),
        }]
    }
    #[test]
    fn census_includes_brownfield_references_and_normalizes_display_aliases() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        fs::write(
            forgeplan.join("use_cases/UC-001.md"),
            "---\nid: UC-001\nkind: use_case\ntitle: Example\nlinks:\n  - target: ' prd-1 '\n    relation: informs\n---\n\nBody\n",
        ).unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &[]).unwrap();
        let reference = plan.references.iter().find(|entry| entry.source_path == "use_cases/UC-001.md").unwrap();
        assert_eq!(reference.from, "prd-1");
        assert_eq!(reference.mapping.len(), 2);
        assert!(!reference.is_resolved());
    }
    #[test]
    fn discovery_claimants_and_inbound_references_are_in_the_plan() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        fs::create_dir_all(forgeplan.join("discovery")).unwrap();
        fs::write(forgeplan.join("discovery/claim.md"), document("PRD-001", "prd", "Third")).unwrap();
        fs::write(forgeplan.join("discovery/reference.md"), linking_document("EVID-002", "evidence", "PRD-001")).unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &[]).unwrap();
        assert!(plan.sources.iter().any(|source| source.relative_path == "discovery/claim.md"));
        let inbound = plan.references.iter().find(|reference| reference.source_path == "discovery/reference.md").unwrap();
        assert_eq!(inbound.mapping.len(), 3);
        assert!(!inbound.is_resolved());
    }

    #[test]
    fn unsupported_discovery_document_is_not_silently_skipped() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        fs::create_dir_all(forgeplan.join("discovery")).unwrap();
        fs::write(forgeplan.join("discovery/unknown.md"), "---\nid: unknown\nkind: unknown\n---\n").unwrap();
        assert!(census_workspace(&forgeplan).unwrap_err().to_string().contains("discovery/unknown.md"));
    }

    #[test]
    fn decisions_bind_each_occurrence_and_the_reviewed_bytes() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let path = forgeplan.join("evidence/EVID-001.md");
        let content = "---\nid: EVID-001\nkind: evidence\ntitle: Evidence\nlinks:\n- target: PRD-001\n  relation: informs\n- target: PRD-001\n  relation: based_on\n---\n\nBody\n";
        fs::write(&path, content).unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let first = ReferenceDecision {
            source_path: "evidence/EVID-001.md".into(), field: "links".into(),
            occurrence: Some(0), source_sha256: Some(sha256_hex(content.as_bytes())),
            from: "PRD-001".into(), target_path: "prds/PRD-001-a.md".into(),
        };
        let mut second = first.clone();
        second.occurrence = Some(1);
        second.target_path = "prds/PRD-001-b.md".into();
        let decisions = merge_decisions(&[first], &[second]);
        assert_eq!(decisions.len(), 2);
        let plan = build_repair_plan(&forgeplan, &census, &decisions).unwrap();
        for reference in &plan.references {
            assert_eq!(reference.to, reference.mapping[&decisions[reference.occurrence].target_path]);
        }
        let operations = plan_operations(&forgeplan, &plan, &census).unwrap();
        let inbound = operations.iter().find(|operation| operation.path == "evidence/EVID-001.md").unwrap();
        let rewritten = std::str::from_utf8(inbound.postimage.as_ref().unwrap()).unwrap();
        let links = link_entries(rewritten).unwrap();
        assert_eq!(links[0].target, plan.references[0].to);
        assert_eq!(links[1].target, plan.references[1].to);
        fs::write(&path, format!("{content}changed\n")).unwrap();
        let changed = census_workspace(&forgeplan).unwrap();
        let stale = build_repair_plan(&forgeplan, &changed, &decisions).unwrap();
        assert!(stale.references.iter().all(|reference| !reference.is_resolved()));
    }

    #[test]
    fn ambiguous_sidecar_blocks_repair_before_any_document_changes() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        fs::create_dir(forgeplan.join("claims")).unwrap();
        fs::write(forgeplan.join("claims/PRD-001.yaml"), "id: PRD-001\nagent_id: test\n").unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        assert_eq!(census.sidecars.len(), 1);
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let before = fs::read(forgeplan.join("prds/PRD-001-a.md")).unwrap();
        let error = apply_repair_plan(&forgeplan, &plan).unwrap_err();
        assert!(error.to_string().contains("sidecar"));
        assert_eq!(fs::read(forgeplan.join("prds/PRD-001-a.md")).unwrap(), before);
        assert!(!repair_journal_root(&forgeplan).exists());
    }

    #[test]
    fn sidecar_change_invalidates_a_previously_reviewed_plan() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        fs::create_dir(forgeplan.join("state")).unwrap();
        let state = forgeplan.join("state/PRD-002.yaml");
        fs::write(&state, "artifact_id: PRD-002\nphase: shape\n").unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        fs::write(&state, "artifact_id: PRD-002\nphase: code\n").unwrap();
        assert!(apply_repair_plan(&forgeplan, &plan).unwrap_err().to_string().contains("stale"));
    }


    #[test]
    fn the_plan_gives_every_collision_member_its_own_key() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).expect("census");
        let plan = build_repair_plan(&forgeplan, &census, &[]).expect("plan");

        assert_eq!(plan.version, REPAIR_PLAN_VERSION);
        assert!(plan_digest_matches(&plan));
        // The two members of the group get two different keys; the unique
        // document is not a source at all.
        assert_eq!(plan.sources.len(), 2);
        let mut keys: Vec<&str> = plan.sources.iter().map(|source| source.new_key.as_str()).collect();
        keys.sort_unstable();
        assert_ne!(keys[0], keys[1]);
        for source in &plan.sources {
            assert_eq!(source.old_key, "PRD-001");
            assert_eq!(source.proposed_path, source.relative_path);
            assert_eq!(source.file_sha256.len(), 64);
            assert_eq!(source.body_sha256.len(), 64);
            assert_eq!(source.frontmatter_sha256.len(), 64);
        }
        // The workspace is untouched by planning.
        assert_eq!(
            fs::read_to_string(forgeplan.join("prds/PRD-001-a.md")).expect("read"),
            document("PRD-001", "prd", "First")
        );
    }

    #[test]
    fn a_conflicting_reference_stays_unresolved_until_a_review_decides() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).expect("census");

        let plan = build_repair_plan(&forgeplan, &census, &[]).expect("plan");
        assert_eq!(plan.references.len(), 1);
        let reference = &plan.references[0];
        assert_eq!(reference.source_path, "evidence/EVID-001.md");
        assert_eq!(reference.field, "links");
        assert_eq!(reference.occurrence, 0);
        assert_eq!(reference.from, "PRD-001");
        assert!(reference.to.is_empty());
        assert!(!reference.is_resolved());
        // The mapping names both candidates and the key each one receives.
        assert_eq!(reference.mapping.len(), 2);
        for key in reference.mapping.values() {
            assert!(key.starts_with("prd-"));
        }

        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).expect("plan");
        let reference = &plan.references[0];
        assert!(reference.is_resolved());
        let expected = plan
            .sources
            .iter()
            .find(|source| source.relative_path == "prds/PRD-001-b.md")
            .expect("source")
            .new_key
            .clone();
        assert_eq!(reference.to, expected);
    }

    #[test]
    fn a_reference_to_an_unchanged_key_is_not_in_the_plan() {
        let workspace = temp_workspace(&[
            ("prds", "PRD-002.md", &document("PRD-002", "prd", "Other")),
            (
                "evidence",
                "EVID-002.md",
                &linking_document("EVID-002", "evidence", "PRD-002"),
            ),
        ]);
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).expect("census");
        let plan = build_repair_plan(&forgeplan, &census, &[]).expect("plan");
        assert!(plan.sources.is_empty());
        assert!(plan.references.is_empty());
    }

    #[test]
    fn reviewed_reference_can_select_unchanged_candidate_with_shared_alias() {
        let first = set_frontmatter_slug(&document("PRD-001", "prd", "First"), "prd-shared").unwrap();
        let second = set_frontmatter_slug(&document("PRD-001", "prd", "Second"), "prd-shared").unwrap();
        let stable = set_frontmatter_slug(&document("PRD-001", "prd", "Stable"), "prd-stable").unwrap();
        let evidence = linking_document("EVID-001", "evidence", "PRD-001");
        let workspace = temp_workspace(&[
            ("prds", "PRD-001-a.md", &first),
            ("prds", "PRD-001-b.md", &second),
            ("prds", "PRD-001-c.md", &stable),
            ("evidence", "EVID-001.md", &evidence),
        ]);
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let decision = [ReferenceDecision {
            source_path: "evidence/EVID-001.md".to_string(),
            occurrence: Some(0), source_sha256: Some(sha256_hex(evidence.as_bytes())),
            field: "links".to_string(),
            from: "PRD-001".to_string(),
            target_path: "prds/PRD-001-c.md".to_string(),
        }];
        let plan = build_repair_plan(&forgeplan, &census, &decision).unwrap();
        assert_eq!(plan.sources.len(), 2);
        assert!(plan.sources.iter().all(|source| source.relative_path != "prds/PRD-001-c.md"));
        assert_eq!(plan.references.len(), 1);
        assert_eq!(plan.references[0].mapping.len(), 3);
        assert_eq!(plan.references[0].to, "prd-stable");

        let stable_before = fs::read(forgeplan.join("prds/PRD-001-c.md")).unwrap();
        apply_repair_plan(&forgeplan, &plan).unwrap();
        assert_eq!(fs::read(forgeplan.join("prds/PRD-001-c.md")).unwrap(), stable_before);
        let rewritten = fs::read_to_string(forgeplan.join("evidence/EVID-001.md")).unwrap();
        assert_eq!(link_entries(&rewritten).unwrap()[0].target, "prd-stable");
        let after = census_workspace(&forgeplan).unwrap();
        assert!(collision_groups(&after).is_empty());
        for source in &plan.sources {
            let doc = after.documents.iter().find(|doc| doc.relative_path == source.relative_path).unwrap();
            assert_eq!(doc.canonical_key, source.new_key);
        }
    }

    #[test]
    fn reviewed_noop_reference_that_remains_ambiguous_is_refused_without_changes() {
        let first = set_frontmatter_slug(&document("PRD-001", "prd", "First"), "prd-shared").unwrap();
        let second = set_frontmatter_slug(&document("PRD-001", "prd", "Second"), "prd-shared").unwrap();
        let stable = document("PRD-001", "prd", "Stable");
        let evidence = linking_document("EVID-001", "evidence", "PRD-001");
        let workspace = temp_workspace(&[
            ("prds", "PRD-001-a.md", &first),
            ("prds", "PRD-001-b.md", &second),
            ("prds", "PRD-001-c.md", &stable),
            ("evidence", "EVID-001.md", &evidence),
        ]);
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let undecided = build_repair_plan(&forgeplan, &census, &[]).unwrap();
        assert!(undecided.references[0].to.is_empty());
        assert!(plan_operations(&forgeplan, &undecided, &census).is_err());
        let decision = [ReferenceDecision {
            source_path: "evidence/EVID-001.md".to_string(),
            occurrence: Some(0), source_sha256: Some(sha256_hex(evidence.as_bytes())),
            field: "links".to_string(),
            from: "PRD-001".to_string(),
            target_path: "prds/PRD-001-c.md".to_string(),
        }];
        let mut plan = build_repair_plan(&forgeplan, &census, &decision).unwrap();
        assert_eq!(plan.references.len(), 1);
        let reference = &plan.references[0];
        assert_eq!(reference.from, "PRD-001");
        assert_eq!(reference.to, "PRD-001");
        assert_eq!(reference.mapping.get("prds/PRD-001-c.md").map(String::as_str), Some("PRD-001"));
        assert_rejected_without_workspace_changes(&forgeplan, &mut plan, "remains ambiguous after repair");
    }

    #[test]
    fn a_plan_round_trips_through_json_and_an_edited_plan_is_refused() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).expect("census");
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).expect("plan");

        let path = workspace.path().join("plan.json");
        write_plan_json(&plan, &path).expect("write plan");
        let read = read_plan_json(&path).expect("read plan");
        assert_eq!(read, plan);

        // An edited plan no longer matches its digest and is refused.
        let mut edited = read.clone();
        edited.sources[0].new_key = "prd-edited-k00".to_string();
        let mut bytes = serde_json::to_vec_pretty(&edited).expect("serialize");
        bytes.push(b'\n');
        fs::write(&path, &bytes).expect("write");
        let error = read_plan_json(&path).expect_err("an edited plan must be refused");
        assert!(
            error.to_string().contains("digest"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn the_later_decision_wins() {
        let base = vec![ReferenceDecision {
            source_path: "evidence/EVID-001.md".to_string(),
            occurrence: Some(0), source_sha256: Some("test".into()),
            field: "links".to_string(),
            from: "PRD-001".to_string(),
            target_path: "prds/PRD-001-a.md".to_string(),
        }];
        let extra = vec![ReferenceDecision {
            source_path: "evidence/EVID-001.md".to_string(),
            occurrence: Some(0), source_sha256: Some("test".into()),
            field: "links".to_string(),
            from: "PRD-001".to_string(),
            target_path: "prds/PRD-001-b.md".to_string(),
        }];
        let merged = merge_decisions(&base, &extra);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].target_path, "prds/PRD-001-b.md");
    }

    #[test]
    fn a_decision_file_is_read() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let path = workspace.path().join("decisions.json");
        let file = DecisionFile {
            version: 1,
            decisions: decision_for_second_prd(),
        };
        fs::write(&path, serde_json::to_vec_pretty(&file).expect("serialize")).expect("write");
        let read = read_decision_file(&path).expect("read");
        assert_eq!(read.decisions.len(), 1);
        assert_eq!(read.decisions[0].target_path, "prds/PRD-001-b.md");
    }

    #[test]
    fn the_pilot_record_covers_the_measured_census() {
        assert_eq!(PILOT_REFERENCE_DECISIONS.len(), 24);
        // Every decision names a distinct occurrence.
        let mut keys: Vec<(String, String, String)> = PILOT_REFERENCE_DECISIONS
            .iter()
            .map(|decision| {
                (
                    decision.source_path.clone(),
                    decision.field.clone(),
                    decision.from.clone(),
                )
            })
            .collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), 24);
        // The owner decision: NOTE-019 points at the deterministic PRD-072.
        let decision = PILOT_REFERENCE_DECISIONS
            .iter()
            .find(|decision| decision.source_path.starts_with("notes/NOTE-019-"))
            .expect("the NOTE-019 decision");
        assert_eq!(decision.from, "PRD-072");
        assert!(
            decision
                .target_path
                .contains("PRD-072-deterministic-sdd-tdd-pipeline"),
            "unexpected target: {}",
            decision.target_path
        );
    }

    // -----------------------------------------------------------------
    // The transaction
    // -----------------------------------------------------------------

    fn workspace_snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
        fn visit(root: &Path, dir: &Path, snapshot: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                let relative = path.strip_prefix(root).unwrap().to_path_buf();
                if path.is_dir() {
                    snapshot.insert(relative, None);
                    visit(root, &path, snapshot);
                } else {
                    snapshot.insert(relative, Some(fs::read(path).unwrap()));
                }
            }
        }
        let mut snapshot = BTreeMap::new();
        visit(root, root, &mut snapshot);
        snapshot
    }

    fn assert_rejected_without_workspace_changes(forgeplan: &Path, plan: &mut RepairPlan, expected: &str) {
        // The ordinary workspace lock is not part of the repair transaction.
        fs::write(lock_path(forgeplan), b"existing lock").unwrap();
        plan.digest = compute_plan_digest(plan);
        let before = workspace_snapshot(forgeplan);
        let error = apply_repair_plan(forgeplan, plan).expect_err("unsafe plan must be refused");
        assert!(format!("{error:#}").contains(expected), "unexpected error: {error:#}");
        assert_eq!(workspace_snapshot(forgeplan), before);
        assert!(!repair_journal_root(forgeplan).exists());
        assert!(!repair_staging_root(forgeplan).exists());
    }

    fn replace_planned_key(plan: &mut RepairPlan, index: usize, key: &str) {
        let source = &mut plan.sources[index];
        let old = std::mem::replace(&mut source.new_key, key.to_string());
        for reference in &mut plan.references {
            if let Some(mapped) = reference.mapping.get_mut(&source.relative_path) {
                *mapped = key.to_string();
            }
            if reference.to == old {
                reference.to = key.to_string();
            }
        }
    }

    #[test]
    fn apply_refuses_duplicate_proposed_keys_even_with_a_matching_digest() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let mut plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let duplicate = plan.sources[0].new_key.clone();
        replace_planned_key(&mut plan, 1, &duplicate);
        assert_rejected_without_workspace_changes(&forgeplan, &mut plan, "occupied or duplicated");
    }

    #[test]
    fn apply_refuses_a_key_already_claimed_by_an_unchanged_document() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let occupied = "prd-existing-k0123456789abcdef0123456789abcdef";
        let path = forgeplan.join("prds/PRD-002.md");
        let original = fs::read_to_string(&path).unwrap();
        fs::write(&path, set_frontmatter_slug(&original, occupied).unwrap()).unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let mut plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        replace_planned_key(&mut plan, 0, occupied);
        assert_rejected_without_workspace_changes(&forgeplan, &mut plan, "occupied or duplicated");
    }

    #[test]
    fn apply_refuses_malformed_or_wrong_kind_proposed_keys() {
        for key in [
            "prd-invalid-k123",
            "prd-invalid-k0123456789ABCDEF0123456789ABCDEF",
            "prd--invalid-k0123456789abcdef0123456789abcdef",
            "prd-invalid-k0123456789abcdef0123456789abcde\nslug: hijacked",
            "rfc-invalid-k0123456789abcdef0123456789abcdef",
        ] {
            let workspace = colliding_workspace();
            let forgeplan = workspace.path().join(".forgeplan");
            let census = census_workspace(&forgeplan).unwrap();
            let mut plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
            replace_planned_key(&mut plan, 0, key);
            assert_rejected_without_workspace_changes(&forgeplan, &mut plan, "invalid proposed canonical key");
        }
    }

    #[test]
    fn apply_refuses_stale_source_paths_and_false_reference_mappings() {
        for change in 0..3 {
            let workspace = colliding_workspace();
            let forgeplan = workspace.path().join(".forgeplan");
            let census = census_workspace(&forgeplan).unwrap();
            let mut plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
            match change {
                0 => plan.sources[0].relative_path = "prds/PRD-002.md".to_string(),
                1 => plan.sources[0].old_key = "PRD-002".to_string(),
                _ => {
                    plan.references[0].mapping.insert(
                        "prds/PRD-001-a.md".to_string(), "prd-wrong".to_string(),
                    );
                }
            }
            let expected = if change < 2 { "collision groups changed" } else { "reference mapping" };
            assert_rejected_without_workspace_changes(&forgeplan, &mut plan, expected);
        }
    }

    #[test]
    fn postimage_validation_rejects_unplanned_content_and_identity_changes() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let operations = plan_operations(&forgeplan, &plan, &census).unwrap();
        let mut operations: BTreeMap<String, FileOperation> = operations.into_iter()
            .map(|operation| (operation.path.clone(), operation))
            .collect();
        let source = &plan.sources[0];
        let operation = operations.get_mut(&source.relative_path).unwrap();
        let original = String::from_utf8(operation.postimage.clone().unwrap()).unwrap();
        operation.postimage = Some(original.replace("Body of PRD-001.", "Lost body.").into_bytes());
        assert!(validate_document_postimages(&operations, &plan, &census).is_err());
        operations.get_mut(&source.relative_path).unwrap().postimage =
            Some(original.replace(&source.new_key, "invalid-key").into_bytes());
        assert!(validate_document_postimages(&operations, &plan, &census).is_err());
    }

    #[test]
    fn the_apply_rewrites_the_key_and_the_reference_and_leaves_a_journal() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).expect("census");
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).expect("plan");

        let outcome = apply_repair_plan(&forgeplan, &plan).expect("apply");
        assert_eq!(outcome.published.len(), 3);
        assert_eq!(outcome.status, JournalStatus::CanonicalCommittedIndexPending);

        let new_key = plan
            .sources
            .iter()
            .find(|source| source.relative_path == "prds/PRD-001-b.md")
            .expect("source")
            .new_key
            .clone();
        let rewritten = fs::read_to_string(forgeplan.join("prds/PRD-001-b.md")).expect("read");
        let frontmatter = effective_frontmatter(&rewritten).expect("frontmatter");
        assert_eq!(
            frontmatter.get("slug").and_then(|value| value.as_str()),
            Some(new_key.as_str())
        );
        // The historical display number and the body survive.
        assert_eq!(
            frontmatter.get("id").and_then(|value| value.as_str()),
            Some("PRD-001")
        );
        assert!(rewritten.contains("Body of PRD-001."));
        // Nothing was deleted: every document is still there.
        assert!(forgeplan.join("prds/PRD-001-a.md").exists());
        assert!(forgeplan.join("prds/PRD-002.md").exists());

        // The reference now points at the key its target received.
        let evidence = fs::read_to_string(forgeplan.join("evidence/EVID-001.md")).expect("read");
        assert_eq!(link_entries(&evidence).expect("links")[0].target, new_key);

        // The journal holds the manifest, the preimages and the post-image
        // hashes.
        let journal_dir = repair_journal_dir(&forgeplan, &outcome.transaction_id);
        let manifest_bytes = fs::read(journal_dir.join("manifest.json")).expect("manifest");
        let manifest: RepairJournal = serde_json::from_slice(&manifest_bytes).expect("parse");
        assert_eq!(manifest.status, JournalStatus::CanonicalCommittedIndexPending);
        assert_eq!(manifest.operations.len(), 3);
        assert!(
            manifest
                .operations
                .iter()
                .all(|operation| operation.state == OperationState::Published)
        );
        assert_eq!(manifest.planned_postimages.len(), 3);
        assert_eq!(manifest.preimages.len(), 3);
        assert_eq!(
            fs::read_dir(journal_dir.join("preimages"))
                .expect("preimages")
                .count(),
            3
        );
        // The preimage is the file as it was before the repair.
        let preimage = fs::read(
            preimage_file(&journal_dir, "prds/PRD-001-b.md"),
        )
        .expect("preimage");
        assert_eq!(sha256_hex(&preimage), manifest.preimages["prds/PRD-001-b.md"]);
    }

    #[test]
    fn the_apply_refuses_a_plan_the_workspace_has_outgrown() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).expect("census");
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).expect("plan");

        // A document changed after the plan was made.
        fs::write(
            forgeplan.join("prds/PRD-001-a.md"),
            document("PRD-001", "prd", "First, edited"),
        )
        .expect("write");
        let error = apply_repair_plan(&forgeplan, &plan).expect_err("a stale plan must be refused");
        assert!(
            error.to_string().contains("stale"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn the_apply_refuses_a_plan_the_head_moved_past() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let root = workspace.path();
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .expect("git")
        };
        git(&["init", "-b", "main"]);
        git(&["-c", "user.email=repair@example.invalid", "-c", "user.name=repair", "add", "-A"]);
        git(&[
            "-c",
            "user.email=repair@example.invalid",
            "-c",
            "user.name=repair",
            "commit",
            "-m",
            "root",
        ]);

        let census = census_workspace(&forgeplan).expect("census");
        let plan =
            build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).expect("plan");
        assert!(plan.git.is_some(), "the workspace is inside a repository");

        // A new commit moves HEAD.
        fs::write(
            forgeplan.join("prds/PRD-003.md"),
            document("PRD-003", "prd", "Third"),
        )
        .expect("write");
        git(&["-c", "user.email=repair@example.invalid", "-c", "user.name=repair", "add", "-A"]);
        git(&[
            "-c",
            "user.email=repair@example.invalid",
            "-c",
            "user.name=repair",
            "commit",
            "-m",
            "second",
        ]);

        let error = apply_repair_plan(&forgeplan, &plan).expect_err("a moved HEAD must be refused");
        assert!(
            error.to_string().contains("Git identity"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn the_apply_refuses_a_reference_that_changed() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).expect("census");
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).expect("plan");

        // The reference now points somewhere else.
        fs::write(
            forgeplan.join("evidence/EVID-001.md"),
            linking_document("EVID-001", "evidence", "PRD-002"),
        )
        .expect("write");
        let error = apply_repair_plan(&forgeplan, &plan).expect_err("a changed reference must be refused");
        assert!(
            error.to_string().contains("stale"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn the_apply_refuses_an_unresolved_reference_onto_a_changed_key() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).expect("census");
        // No review decision: the reference stays unresolved.
        let plan = build_repair_plan(&forgeplan, &census, &[]).expect("plan");
        let error = apply_repair_plan(&forgeplan, &plan).expect_err("an unresolved reference must be refused");
        assert!(
            error.to_string().contains("unresolved"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn the_apply_refuses_a_destination_that_is_already_occupied() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).expect("census");
        let mut plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).expect("plan");
        // A plan that would publish over another document. The digest is
        // recomputed, so the refusal is about the destination and not about
        // the plan's integrity.
        plan.sources[0].proposed_path = "prds/PRD-002.md".to_string();
        plan.digest = compute_plan_digest(&plan);
        let error = apply_repair_plan(&forgeplan, &plan).expect_err("an occupied destination must be refused");
        assert!(
            error.to_string().contains("occupied"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn the_apply_refuses_a_document_path_that_leaves_the_workspace() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).expect("census");
        let mut plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).expect("plan");
        plan.sources[0].relative_path = "../outside.md".to_string();
        plan.digest = compute_plan_digest(&plan);
        let error = apply_repair_plan(&forgeplan, &plan).expect_err("a traversal must be refused");
        assert!(
            error.to_string().contains("leaves the workspace"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn a_rollback_restores_every_preimage() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let before: BTreeMap<String, Vec<u8>> = ["prds/PRD-001-a.md", "prds/PRD-001-b.md", "evidence/EVID-001.md"]
            .iter()
            .map(|path| {
                (
                    (*path).to_string(),
                    fs::read(forgeplan.join(*path)).expect("read"),
                )
            })
            .collect();
        let census = census_workspace(&forgeplan).expect("census");
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).expect("plan");
        let outcome = apply_repair_plan(&forgeplan, &plan).expect("apply");

        let rollback = rollback_repair(&forgeplan, &outcome.transaction_id).expect("rollback");
        assert_eq!(rollback.restored.len(), 3);
        assert_eq!(rollback.status, JournalStatus::RolledBackIndexPending);
        for (path, bytes) in &before {
            assert_eq!(
                &fs::read(forgeplan.join(path)).expect("read"),
                bytes,
                "{path} was not restored"
            );
        }
    }

    #[test]
    fn rollback_refuses_a_corrupted_preimage() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).expect("census");
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).expect("plan");
        let outcome = apply_repair_plan(&forgeplan, &plan).expect("apply");
        let path = "prds/PRD-001-a.md";
        let published = fs::read(forgeplan.join(path)).expect("published document");
        let journal_dir = repair_journal_dir(&forgeplan, &outcome.transaction_id);
        fs::write(preimage_file(&journal_dir, path), b"corrupted").expect("corrupt preimage");

        let error = rollback_repair(&forgeplan, &outcome.transaction_id).expect_err("reject corruption");
        assert!(error.to_string().contains("saved preimage"));
        assert_eq!(fs::read(forgeplan.join(path)).unwrap(), published);
        let journal: RepairJournal = serde_json::from_slice(&fs::read(journal_dir.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(journal.status, JournalStatus::Conflict);
    }

    #[test]
    fn automatic_restoration_preserves_an_unrelated_edit() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("document.md");
        fs::write(&path, b"external edit").unwrap();
        let operation = FileOperation {
            path: "document.md".to_string(),
            absolute: path.clone(),
            preimage: Some(b"before".to_vec()),
            postimage: Some(b"after".to_vec()),
        };
        let (restored, failures) = restore_preimages(tmp.path(), &[operation], &["document.md".to_string()]);
        assert!(restored.is_empty());
        assert!(failures.iter().any(|failure| failure.contains("unrelated edit")));
        assert_eq!(fs::read(path).unwrap(), b"external edit");
    }

    #[test]
    fn a_rollback_refuses_to_overwrite_an_unrelated_edit() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).expect("census");
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).expect("plan");
        let outcome = apply_repair_plan(&forgeplan, &plan).expect("apply");

        // Somebody edited a published file after the repair.
        fs::write(
            forgeplan.join("prds/PRD-001-a.md"),
            "edited by somebody else\n",
        )
        .expect("write");
        let error = rollback_repair(&forgeplan, &outcome.transaction_id)
            .expect_err("an unrelated edit must not be overwritten");
        assert!(
            error.to_string().contains("unrelated edit"),
            "unexpected error: {error:#}"
        );
        // The edit survives, and the journal keeps the conflict.
        assert_eq!(
            fs::read_to_string(forgeplan.join("prds/PRD-001-a.md")).expect("read"),
            "edited by somebody else\n"
        );
        let journal_dir = repair_journal_dir(&forgeplan, &outcome.transaction_id);
        let manifest_bytes = fs::read(journal_dir.join("manifest.json")).expect("manifest");
        let manifest: RepairJournal = serde_json::from_slice(&manifest_bytes).expect("parse");
        assert_eq!(manifest.status, JournalStatus::Conflict);
        assert!(
            manifest
                .operations
                .iter()
                .any(|operation| operation.state == OperationState::Conflict),
            "the conflicting operation must be recorded"
        );
    }

    #[test]
    fn the_repair_publishes_with_plain_file_operations_and_never_stages() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let root = workspace.path();
        // A repository, so the staging area can be inspected afterwards.
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .expect("git")
        };
        git(&["init", "-b", "main"]);
        git(&[
            "-c",
            "user.email=repair@example.invalid",
            "-c",
            "user.name=repair",
            "add",
            "-A",
        ]);
        git(&[
            "-c",
            "user.email=repair@example.invalid",
            "-c",
            "user.name=repair",
            "commit",
            "-m",
            "root",
        ]);

        let census = census_workspace(&forgeplan).expect("census");
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).expect("plan");
        apply_repair_plan(&forgeplan, &plan).expect("apply");

        // The change is in the working tree, not in the index: the repair
        // never stages and never renames through git.
        let status = String::from_utf8_lossy(&git(&["status", "--porcelain"]).stdout).to_string();
        assert!(
            status.contains(" M .forgeplan/prds/PRD-001-a.md"),
            "unexpected git status: {status}"
        );
        assert!(
            !status.contains("A  "),
            "the repair must not stage anything: {status}"
        );
    }

    #[tokio::test]
    async fn failed_apply_after_rename_restores_files_and_requires_index_reconcile() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let before = census.inventory_digest.clone();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let mut calls = 0;
        let result = apply_repair_plan_with_publisher(&forgeplan, &plan, |path, bytes| {
            calls += 1;
            publish(path, bytes)?;
            if calls == 2 {
                bail!("injected failure after rename");
            }
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(calls, 2);
        assert_eq!(census_workspace(&forgeplan).unwrap().inventory_digest, before);
        let entry = fs::read_dir(repair_journal_root(&forgeplan)).unwrap().next().unwrap().unwrap();
        let id = entry.file_name().into_string().unwrap();
        let journal = read_valid_journal(&forgeplan, &id).unwrap();
        assert_eq!(journal.status, JournalStatus::RolledBackIndexPending);
        assert!(journal.operations.iter().all(|op| op.state == OperationState::RolledBack));
        assert!(crate::workspace::lock::acquire_workspace_lock(&forgeplan).await.is_err());
    }

    #[tokio::test]
    async fn repair_admission_binds_owner_journal_and_current_bytes() {
        use crate::constellation::publisher::IndexPublisher;
        use crate::workspace::lock::{acquire_repair_workspace_lock, begin_repair_index_admission, finish_repair_index_admission};
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan").canonicalize().unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let owner = IndexPublisher::acquire(&forgeplan.join("lance"), std::sync::Arc::new(object_store::memory::InMemory::new())).unwrap();
        let admission = begin_repair_index_admission(&forgeplan, &outcome.transaction_id, &owner).await.unwrap();
        assert!(crate::workspace::lock::acquire_workspace_lock(&forgeplan).await.is_err());
        let guard = acquire_repair_workspace_lock(&admission).await.unwrap();
        assert_eq!(
            pending_recovery_status(&forgeplan, &outcome.transaction_id, &guard).unwrap(),
            JournalStatus::CanonicalCommittedIndexPending
        );
        assert!(validate_legacy_repair(&forgeplan, &guard, &outcome.transaction_id).is_err());
        drop(guard);
        let unrelated = fs::File::create(workspace.path().join("other.lock")).unwrap();
        let wrong_guard = WorkspaceLock::from_file(unrelated);
        assert!(pending_recovery_status(&forgeplan, &outcome.transaction_id, &wrong_guard).is_err());
        assert!(validate_legacy_repair(&forgeplan, &wrong_guard, &outcome.transaction_id).is_err());
        let digest = pending_repair_digest(&forgeplan, &outcome.transaction_id).unwrap();
        assert!(finish_pending_recovery(&forgeplan, &outcome.transaction_id, &digest, &wrong_guard).is_err());
        assert!(finish_repair_index_admission(&admission, &wrong_guard).is_err());
        drop(wrong_guard);
        fs::write(forgeplan.join("prds/PRD-001-a.md"), b"intervening edit").unwrap();
        assert!(acquire_repair_workspace_lock(&admission).await.is_err());
        assert!(crate::workspace::lock::acquire_workspace_lock(&forgeplan).await.is_err());
    }

    #[tokio::test]
    async fn scoped_pending_status_distinguishes_rollback_and_revalidates_bytes() {
        use crate::constellation::publisher::IndexPublisher;
        use crate::workspace::lock::{acquire_repair_workspace_lock, begin_repair_index_admission};
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan").canonicalize().unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        rollback_repair(&forgeplan, &outcome.transaction_id).unwrap();
        let owner = IndexPublisher::acquire(
            &forgeplan.join("lance"),
            std::sync::Arc::new(object_store::memory::InMemory::new()),
        ).unwrap();
        let admission = begin_repair_index_admission(&forgeplan, &outcome.transaction_id, &owner).await.unwrap();
        let guard = acquire_repair_workspace_lock(&admission).await.unwrap();
        assert_eq!(
            pending_recovery_status(&forgeplan, &outcome.transaction_id, &guard).unwrap(),
            JournalStatus::RolledBackIndexPending
        );
        fs::write(forgeplan.join("prds/PRD-001-a.md"), b"intervening edit").unwrap();
        assert!(pending_recovery_status(&forgeplan, &outcome.transaction_id, &guard).is_err());
    }

    #[tokio::test]
    async fn legacy_capability_binds_guard_journal_and_full_census() {
        use crate::db::migrate::{
            identity_v5_marker_path, IDENTITY_V5_ARTIFACTS_TABLE, IDENTITY_V5_EVIDENCE_TABLE,
            IDENTITY_V5_RELATIONS_TABLE, IDENTITY_V5_VERSION,
        };
        use crate::workspace::lock::acquire_legacy_repair_workspace_lock;
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan").canonicalize().unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let lance = forgeplan.join("lance");
        fs::create_dir(&lance).unwrap();
        let marker = serde_json::json!({
            "version": IDENTITY_V5_VERSION,
            "artifacts_table": IDENTITY_V5_ARTIFACTS_TABLE,
            "relations_table": IDENTITY_V5_RELATIONS_TABLE,
            "evidence_table": IDENTITY_V5_EVIDENCE_TABLE,
            "source_digest": "a".repeat(64),
            "document_count": 0,
            "completed": true
        });
        fs::write(identity_v5_marker_path(&lance), serde_json::to_vec(&marker).unwrap()).unwrap();
        let guard = acquire_legacy_repair_workspace_lock(&forgeplan, &outcome.transaction_id).await.unwrap();
        let proof = validate_legacy_repair(&forgeplan, &guard, &outcome.transaction_id).unwrap();
        assert_eq!(proof.workspace(), forgeplan);
        assert_eq!(proof.journal_id(), outcome.transaction_id);
        assert!(proof.revalidate(&forgeplan).is_ok());
        let (status, mappings) = proof.identity_mappings().unwrap();
        assert_eq!(status, JournalStatus::CanonicalCommittedIndexPending);
        assert_eq!(mappings.len(), 3);
        let changed: Vec<_> = mappings.iter().filter(|entry| entry.old_key != entry.new_key).collect();
        assert_eq!(changed.len(), 2);
        assert!(changed.iter().all(|entry| entry.display_id == "PRD-001" && entry.old_key == "PRD-001"));
        assert_ne!(changed[0].new_key, changed[1].new_key);
        let reference = mappings.iter().find(|entry| entry.path == "evidence/EVID-001.md").unwrap();
        assert_eq!(reference.old_key, reference.new_key);
        assert_ne!(reference.preimage, reference.postimage);
        for entry in &mappings {
            assert_eq!(entry.preimage, fs::read(preimage_file(&outcome.journal_dir, &entry.path)).unwrap());
            assert_eq!(entry.postimage, fs::read(forgeplan.join(&entry.path)).unwrap());
        }
        assert!(finish_pending_recovery(&forgeplan, &outcome.transaction_id, &pending_repair_digest(&forgeplan, &outcome.transaction_id).unwrap(), &guard).is_err());
        fs::write(forgeplan.join("prds/PRD-002.md"), b"unrelated source edit").unwrap();
        assert!(proof.revalidate(&forgeplan).is_err());
    }

    async fn real_v4_collision(ambiguous_evidence: bool) -> (tempfile::TempDir, ApplyOutcome) {
        use std::sync::Arc;
        use arrow_array::{Int32Array, RecordBatch, StringArray};
        use crate::db::{convert, migrate, schema, store::NewArtifact};

        let first = "---\nid: PRD-001\nkind: prd\ntitle: First\n---\n\nFirst body.\n";
        let second = "---\nid: PRD-001\nkind: prd\ntitle: Second\n---\n\nSecond body.\n";
        let workspace = temp_workspace(&[("prds", "a.md", first), ("prds", "b.md", second)]);
        let forgeplan = workspace.path().join(".forgeplan").canonicalize().unwrap();
        let source = migrate::read_identity_v5_source(&forgeplan).await.unwrap();
        let document = source.documents.iter().find(|doc| doc.relative_path == "prds/a.md").unwrap();
        let artifact = NewArtifact {
            canonical_key: document.canonical_key.clone(), id: document.display_id.clone(),
            kind: document.kind.clone(), status: document.status.clone(), title: document.title.clone(),
            body: document.body.clone(), depth: document.depth.clone(), author: document.author.clone(),
            parent_epic: document.parent_epic.clone(), valid_until: document.valid_until.clone(), tags: document.tags.clone(),
        };
        let modern = convert::artifact_to_batch(&artifact, "2024-01-01T00:00:00Z").unwrap();
        let v4 = Arc::new(arrow_schema::Schema::new(schema::artifacts_schema().fields().iter().take(15).cloned().collect::<Vec<_>>()));
        let row = RecordBatch::try_new(v4, modern.columns()[..15].to_vec()).unwrap();
        let lance = forgeplan.join("lance");
        fs::create_dir_all(&lance).unwrap();
        let db = lancedb::connect(lance.to_str().unwrap()).execute().await.unwrap();
        db.create_table("artifacts", vec![row]).execute().await.unwrap();
        db.create_table("relations", vec![RecordBatch::new_empty(schema::relations_schema())]).execute().await.unwrap();
        let evidence = if ambiguous_evidence {
            RecordBatch::try_new(schema::evidence_schema(), vec![
                Arc::new(StringArray::from(vec!["EVID-001"])),
                Arc::new(StringArray::from(vec!["PRD-001"])),
                Arc::new(StringArray::from(vec!["test"])),
                Arc::new(StringArray::from(vec!["supports"])),
                Arc::new(Int32Array::from(vec![3])),
                Arc::new(StringArray::from(vec![None::<&str>])),
                Arc::new(StringArray::from(vec!["historical evidence"])),
                Arc::new(StringArray::from(vec!["2024-01-01"])),
            ]).unwrap()
        } else { RecordBatch::new_empty(schema::evidence_schema()) };
        db.create_table("evidence", vec![evidence]).execute().await.unwrap();
        drop(db);
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &[]).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        (workspace, outcome)
    }

    #[tokio::test]
    async fn public_legacy_reconcile_preserves_v4_collision_documents() {
        let (workspace, outcome) = real_v4_collision(false).await;
        let forgeplan = workspace.path().join(".forgeplan").canonicalize().unwrap();
        crate::db::store::LanceStore::reconcile_legacy_repair(&forgeplan, &outcome.transaction_id).await.unwrap();
        let source = crate::db::migrate::read_identity_v5_source(&forgeplan).await.unwrap();
        assert_eq!(source.documents.len(), 2);
        assert_ne!(source.documents[0].canonical_key, source.documents[1].canonical_key);
        assert!(source.documents.iter().any(|doc| doc.body.contains("First body.")));
        assert!(source.documents.iter().any(|doc| doc.body.contains("Second body.")));
        assert!(crate::db::migrate::read_identity_v5_marker(&forgeplan.join("lance")).unwrap().is_some());
    }

    #[tokio::test]
    async fn public_legacy_reconcile_refuses_ambiguous_evidence_and_preserves_v4() {
        let (workspace, outcome) = real_v4_collision(true).await;
        let forgeplan = workspace.path().join(".forgeplan").canonicalize().unwrap();
        let result = crate::db::store::LanceStore::reconcile_legacy_repair(&forgeplan, &outcome.transaction_id).await;
        assert!(result.is_err());
        assert!(crate::db::migrate::read_identity_v5_marker(&forgeplan.join("lance")).unwrap().is_none());
        let db = lancedb::connect(forgeplan.join("lance").to_str().unwrap()).execute().await.unwrap();
        let evidence = crate::db::migrate::table_rows(&db, "evidence").await.unwrap();
        assert_eq!(evidence.len(), 1);
        assert_eq!(crate::db::migrate::required_string(&evidence[0], "artifact_id").unwrap(), "PRD-001");
        assert_eq!(crate::db::migrate::required_string(&evidence[0], "content").unwrap(), "historical evidence");
    }

    #[tokio::test]
    async fn pending_index_and_conflicts_block_writers_until_rollback() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        assert!(crate::workspace::lock::acquire_workspace_lock(&forgeplan).await.is_err());
        let edited = forgeplan.join("prds/PRD-001-a.md");
        let postimage = fs::read(&edited).unwrap();
        fs::write(&edited, b"unrelated edit").unwrap();
        assert!(rollback_repair(&forgeplan, &outcome.transaction_id).is_err());
        assert_eq!(fs::read(&edited).unwrap(), b"unrelated edit");
        assert!(crate::workspace::lock::acquire_workspace_lock(&forgeplan).await.is_err());
        fs::write(&edited, postimage).unwrap();
        rollback_repair(&forgeplan, &outcome.transaction_id).unwrap();
        assert!(crate::workspace::lock::acquire_workspace_lock(&forgeplan).await.is_err());
    }
    #[tokio::test]
    async fn rollback_of_committed_journal_blocks_writers_on_conflict() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let mut journal = read_valid_journal(&forgeplan, &outcome.transaction_id).unwrap();
        journal.status = JournalStatus::CanonicalCommitted;
        write_manifest(&outcome.journal_dir, &mut journal).unwrap();
        drop(crate::workspace::lock::acquire_workspace_lock(&forgeplan).await.unwrap());
        let target = forgeplan.join("prds/PRD-001-a.md");
        fs::write(&target, b"unrelated edit").unwrap();
        assert!(rollback_repair(&forgeplan, &outcome.transaction_id).is_err());
        let journal = read_valid_journal(&forgeplan, &outcome.transaction_id).unwrap();
        assert_eq!(journal.status, JournalStatus::Conflict);
        assert_eq!(fs::read(target).unwrap(), b"unrelated edit");
        assert!(crate::workspace::lock::acquire_workspace_lock(&forgeplan).await.is_err());
    }

    #[test]
    fn directory_sync_failure_is_not_ignored() {
        let workspace = tempfile::tempdir().unwrap();
        assert!(sync_dir(&workspace.path().join("missing-directory")).is_err());
    }

    #[test]
    fn duplicated_hash_map_keys_are_rejected_even_with_matching_digest() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let journal = read_valid_journal(&forgeplan, &outcome.transaction_id).unwrap();
        let (path, hash) = journal.preimages.iter().next().unwrap();
        let duplicate = format!(
            "\"preimages\":{{{}:{},",
            serde_json::to_string(path).unwrap(),
            serde_json::to_string(hash).unwrap()
        );
        let json = serde_json::to_string(&journal).unwrap()
            .replacen("\"preimages\":{", &duplicate, 1);
        fs::write(outcome.journal_dir.join("manifest.json"), json).unwrap();
        assert!(check_repair_journals(&forgeplan, None).is_err());
        assert!(rollback_repair(&forgeplan, &outcome.transaction_id).is_err());
    }


    #[test]
    fn rollback_recovers_rename_even_if_journal_says_pending_or_rolled_back() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let before = fs::read(forgeplan.join("prds/PRD-001-a.md")).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let mut journal = read_valid_journal(&forgeplan, &outcome.transaction_id).unwrap();
        journal.status = JournalStatus::InProgress;
        for (index, operation) in journal.operations.iter_mut().enumerate() {
            operation.state = if index % 2 == 0 { OperationState::Pending } else { OperationState::RolledBack };
        }
        write_manifest(&outcome.journal_dir, &mut journal).unwrap();
        rollback_repair(&forgeplan, &outcome.transaction_id).unwrap();
        assert_eq!(fs::read(forgeplan.join("prds/PRD-001-a.md")).unwrap(), before);
    }

    #[test]
    fn rollback_refuses_a_consistently_hashed_but_incomplete_journal() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let original = fs::read(forgeplan.join("prds/PRD-001-a.md")).unwrap();
        let mut journal = read_valid_journal(&forgeplan, &outcome.transaction_id).unwrap();
        journal.operations.pop();
        write_manifest(&outcome.journal_dir, &mut journal).unwrap();
        assert!(rollback_repair(&forgeplan, &outcome.transaction_id).is_err());
        assert_eq!(fs::read(forgeplan.join("prds/PRD-001-a.md")).unwrap(), original);
        assert!(check_repair_journals(&forgeplan, None).is_err());
    }

    #[test]
    fn rollback_rejects_untrusted_transaction_id_and_paths() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        assert!(rollback_repair(&forgeplan, "../outside").is_err());
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let mut journal = read_valid_journal(&forgeplan, &outcome.transaction_id).unwrap();
        let old_path = journal.operations[0].path.clone();
        let bad_path = "../outside.md".to_string();
        journal.operations[0].path = bad_path.clone();
        let pre = journal.preimages.remove(&old_path).unwrap();
        let post = journal.planned_postimages.remove(&old_path).unwrap();
        journal.preimages.insert(bad_path.clone(), pre);
        journal.planned_postimages.insert(bad_path, post);
        write_manifest(&outcome.journal_dir, &mut journal).unwrap();
        assert!(rollback_repair(&forgeplan, &outcome.transaction_id).is_err());
        assert!(check_repair_journals(&forgeplan, None).is_err());
    }

    #[test]
    fn a_transaction_id_names_its_own_journal() {
        let first = new_transaction_id();
        let second = new_transaction_id();
        assert_ne!(first, second);
        assert!(first.starts_with("repair-"));
        let root = repair_journal_root(Path::new("/tmp/ws/.forgeplan"));
        assert_eq!(root, Path::new("/tmp/ws/.forgeplan/identity-repair"));
        assert_eq!(
            repair_journal_dir(Path::new("/tmp/ws/.forgeplan"), "txn-1"),
            Path::new("/tmp/ws/.forgeplan/identity-repair/txn-1")
        );
    }

    #[test]
    fn the_git_identity_is_absent_outside_a_repository() {
        let workspace = temp_workspace(&[]);
        let forgeplan = workspace.path().join(".forgeplan");
        assert!(detect_git_identity(&forgeplan).expect("detect").is_none());
    }

    #[test]
    fn unborn_head_refuses_plan_and_apply_instead_of_becoming_non_git() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        assert!(plan.git.is_none());
        assert!(std::process::Command::new("git").args(["init", "-q"])
            .current_dir(workspace.path()).status().unwrap().success());
        let before = fs::read(forgeplan.join("prds/PRD-001-a.md")).unwrap();
        let error = detect_git_identity(&forgeplan).unwrap_err().to_string();
        assert!(error.contains("unborn or invalid"));
        let census = census_workspace(&forgeplan).unwrap();
        assert!(build_repair_plan(&forgeplan, &census, &decision_for_second_prd())
            .unwrap_err().to_string().contains("unborn or invalid"));
        assert!(apply_repair_plan(&forgeplan, &plan)
            .unwrap_err().to_string().contains("unborn or invalid"));
        assert_eq!(fs::read(forgeplan.join("prds/PRD-001-a.md")).unwrap(), before);
    }

    #[test]
    fn broken_git_repository_is_not_reported_as_non_git() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        fs::write(workspace.path().join(".git"), "gitdir: /missing/git-directory\n").unwrap();
        assert!(detect_git_identity(&forgeplan).is_err());
        let census = census_workspace(&forgeplan).unwrap();
        assert!(build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).is_err());
        assert!(detect_git_identity_with(&forgeplan, std::ffi::OsStr::new("/nonexistent/git")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn git_common_dir_and_head_failures_are_not_reported_as_non_git() {
        use std::os::unix::fs::PermissionsExt;
        let workspace = temp_workspace(&[]);
        let forgeplan = workspace.path().join(".forgeplan");
        let git = workspace.path().join("fake-git");
        for (script, expected) in [
            ("#!/bin/sh\ncase \"$*\" in *--git-dir) echo .git;; *) echo broken >&2; exit 1;; esac\n", "common directory"),
            ("#!/bin/sh\ncase \"$*\" in *--git-dir) echo .git;; *--git-common-dir) pwd;; *) echo broken >&2; exit 1;; esac\n", "HEAD"),
        ] {
            fs::write(&git, script).unwrap();
            fs::set_permissions(&git, fs::Permissions::from_mode(0o700)).unwrap();
            let error = detect_git_identity_with(&forgeplan, git.as_os_str()).unwrap_err().to_string();
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn committed_git_identity_is_bound_and_broken_head_blocks_apply() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let git = |args: &[&str]| {
            assert!(std::process::Command::new("git").args(args)
                .current_dir(workspace.path()).status().unwrap().success());
        };
        git(&["init", "-q"]);
        git(&["-c", "user.name=Test", "-c", "user.email=test@example.com",
            "commit", "--allow-empty", "-qm", "initial"]);
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let identity = plan.git.as_ref().expect("Git identity");
        assert!(!identity.head.is_empty());
        assert_eq!(Path::new(&identity.common_dir), fs::canonicalize(workspace.path().join(".git")).unwrap());
        let before = fs::read(forgeplan.join("prds/PRD-001-a.md")).unwrap();
        fs::write(workspace.path().join(".git/HEAD"), "broken HEAD\n").unwrap();
        assert!(detect_git_identity(&forgeplan).is_err());
        assert!(apply_repair_plan(&forgeplan, &plan).is_err());
        assert_eq!(fs::read(forgeplan.join("prds/PRD-001-a.md")).unwrap(), before);
    }
    #[test]
    fn nested_collision_reference_apply_and_rollback_preserve_exact_preimages() {
        let workspace = temp_workspace(&[("prds", "first.md", &document("PRD-001", "prd", "First"))]);
        let forgeplan = workspace.path().join(".forgeplan");
        fs::create_dir_all(forgeplan.join("prds/nested")).unwrap();
        fs::create_dir_all(forgeplan.join("evidence/deep")).unwrap();
        let nested = forgeplan.join("prds/nested/second.md");
        let inbound = forgeplan.join("evidence/deep/ref.md");
        fs::write(&nested, document("PRD-001", "prd", "Second")).unwrap();
        fs::write(&inbound, linking_document("EVID-001", "evidence", "PRD-001")).unwrap();
        let before_nested = fs::read(&nested).unwrap();
        let before_inbound = fs::read(&inbound).unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let decision = ReferenceDecision {
            source_path: "evidence/deep/ref.md".into(),
            occurrence: Some(0), source_sha256: Some(sha256_hex(&before_inbound)),
            field: "links".into(),
            from: "PRD-001".into(),
            target_path: "prds/nested/second.md".into(),
        };
        let plan = build_repair_plan(&forgeplan, &census, &[decision]).unwrap();
        let expected_key = plan.sources.iter()
            .find(|source| source.relative_path == "prds/nested/second.md")
            .unwrap().new_key.clone();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        assert_eq!(link_entries(&fs::read_to_string(&inbound).unwrap()).unwrap()[0].target, expected_key);
        assert!(fs::read_to_string(&nested).unwrap().contains(&format!("slug: {expected_key}")));
        rollback_repair(&forgeplan, &outcome.transaction_id).unwrap();
        assert_eq!(fs::read(&nested).unwrap(), before_nested);
        assert_eq!(fs::read(&inbound).unwrap(), before_inbound);
    }

    #[cfg(unix)]
    #[test]
    fn nested_symlink_is_rejected_before_planning() {
        let workspace = temp_workspace(&[]);
        let forgeplan = workspace.path().join(".forgeplan");
        std::os::unix::fs::symlink(workspace.path(), forgeplan.join("prds/linked")).unwrap();
        assert!(census_workspace(&forgeplan).is_err());
    }

    fn sidecar_workspace() -> tempfile::TempDir {
        let first = "---\nid: PRD-001\nkind: prd\ntitle: First\nslug: prd-shared-key\n---\n\nFirst body\n";
        let second = "---\nid: PRD-002\nkind: prd\ntitle: Second\nslug: prd-shared-key\n---\n\nSecond body\n";
        let workspace = temp_workspace(&[("prds", "one.md", first), ("prds", "two.md", second)]);
        let forgeplan = workspace.path().join(".forgeplan");
        fs::create_dir(forgeplan.join("state")).unwrap();
        fs::create_dir(forgeplan.join("claims")).unwrap();
        fs::write(forgeplan.join("state/PRD-001.yaml"),
            "artifact_id: PRD-001\ncanonical_key: 'prd-shared-key' # preserve\ncurrent_phase: shape\n").unwrap();
        fs::write(forgeplan.join("claims/PRD-002.yaml"),
            "id: PRD-002\nagent_id: worker\nnote: retained\n").unwrap();
        workspace
    }

    #[test]
    fn sidecar_move_is_planned_and_rollback_restores_exact_bytes() {
        let workspace = sidecar_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let before_state = fs::read(forgeplan.join("state/PRD-001.yaml")).unwrap();
        let before_claim = fs::read(forgeplan.join("claims/PRD-002.yaml")).unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &[]).unwrap();
        assert!(plan.sidecar_conflicts.is_empty());
        assert_eq!(plan.sidecar_moves.len(), 2);
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let journal = inspect_repair_journal(&forgeplan, &outcome.transaction_id).unwrap();
        assert_eq!(journal.version, Some(2));
        assert_eq!(journal.moves.len(), 2);
        for pair in &plan.sidecar_moves {
            assert!(!forgeplan.join(&pair.source_path).exists());
            let bytes = fs::read(forgeplan.join(&pair.destination_path)).unwrap();
            assert_eq!(sha256_hex(&bytes), pair.postimage_sha256);
        }
        let state = plan.sidecar_moves.iter().find(|entry| entry.source_path.starts_with("state/")).unwrap();
        let state_text = fs::read_to_string(forgeplan.join(&state.destination_path)).unwrap();
        assert!(state_text.contains("# preserve"));
        validate_pending_recovery(&forgeplan, &outcome.transaction_id).unwrap();
        rollback_repair(&forgeplan, &outcome.transaction_id).unwrap();
        assert_eq!(fs::read(forgeplan.join("state/PRD-001.yaml")).unwrap(), before_state);
        assert_eq!(fs::read(forgeplan.join("claims/PRD-002.yaml")).unwrap(), before_claim);
        for pair in &plan.sidecar_moves {
            assert!(!forgeplan.join(&pair.destination_path).exists());
        }
        validate_pending_recovery(&forgeplan, &outcome.transaction_id).unwrap();
    }

    #[test]
    fn occupied_sidecar_destination_after_planning_is_never_overwritten() {
        let workspace = sidecar_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &[]).unwrap();
        let destination = forgeplan.join(&plan.sidecar_moves[0].destination_path);
        let mut injected = false;
        let result = apply_repair_plan_with_publisher(&forgeplan, &plan, |path, bytes| {
            if !injected {
                fs::write(&destination, b"unrelated destination").unwrap();
                injected = true;
            }
            publish(path, bytes)
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"unrelated destination");
        for pair in &plan.sidecar_moves {
            assert!(forgeplan.join(&pair.source_path).exists());
        }
    }

    #[test]
    fn rollback_keeps_destination_if_source_was_replaced() {
        let workspace = sidecar_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &[]).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let pair = plan.sidecar_moves.last().unwrap();
        let source = forgeplan.join(&pair.source_path);
        let destination = forgeplan.join(&pair.destination_path);
        let postimage = fs::read(&destination).unwrap();
        fs::write(&source, b"unrelated source").unwrap();
        assert!(rollback_repair(&forgeplan, &outcome.transaction_id).is_err());
        assert_eq!(fs::read(&source).unwrap(), b"unrelated source");
        assert_eq!(fs::read(&destination).unwrap(), postimage);
    }

    #[test]
    fn restart_recovers_create_before_remove_despite_pending_checkpoint() {
        let workspace = sidecar_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &[]).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let pair = plan.sidecar_moves.last().unwrap();
        let pre = fs::read(preimage_file(&outcome.journal_dir, &pair.source_path)).unwrap();
        publish_new(&forgeplan.join(&pair.source_path), &pre).unwrap();
        let mut journal = read_valid_journal(&forgeplan, &outcome.transaction_id).unwrap();
        journal.status = JournalStatus::InProgress;
        journal.phase = Some(RepairPhase::Applying);
        for operation in &mut journal.operations {
            operation.state = OperationState::Pending;
        }
        write_manifest(&outcome.journal_dir, &mut journal).unwrap();
        rollback_repair(&forgeplan, &outcome.transaction_id).unwrap();
        assert_eq!(fs::read(forgeplan.join(&pair.source_path)).unwrap(), pre);
        assert!(!forgeplan.join(&pair.destination_path).exists());
    }

    #[test]
    fn admission_checks_absent_and_present_sidecar_images() {
        let workspace = sidecar_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &[]).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let pair = &plan.sidecar_moves[0];
        validate_pending_recovery(&forgeplan, &outcome.transaction_id).unwrap();
        fs::write(forgeplan.join(&pair.source_path), b"unexpected").unwrap();
        assert!(validate_pending_recovery(&forgeplan, &outcome.transaction_id).is_err());
    }

    #[test]
    fn legacy_journal_serializes_without_v2_fields_or_digest_change() {
        let json = r#"{"transaction_id":"old","manifest_sha256":"","preimages":{"prds/a.md":"a"},"planned_postimages":{"prds/a.md":"b"},"operations":[{"path":"prds/a.md","state":"pending"}],"status":"in_progress"}"#;
        let journal: RepairJournal = serde_json::from_str(json).unwrap();
        assert_eq!(journal.version, None);
        assert_eq!(serde_json::to_string(&journal).unwrap(), json);
        assert_eq!(compute_journal_digest(&journal), sha256_hex(json.as_bytes()));
    }

    #[test]
    fn legacy_manifest_can_be_rolled_back() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let before = fs::read(forgeplan.join("prds/PRD-001-a.md")).unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let mut journal = read_valid_journal(&forgeplan, &outcome.transaction_id).unwrap();
        journal.version = None;
        journal.phase = None;
        journal.moves.clear();
        write_manifest(&outcome.journal_dir, &mut journal).unwrap();
        rollback_repair(&forgeplan, &outcome.transaction_id).unwrap();
        assert_eq!(fs::read(forgeplan.join("prds/PRD-001-a.md")).unwrap(), before);
    }

    #[test]
    fn legacy_upgrade_is_durable_before_rollback_and_detects_corruption() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let mut journal = read_valid_journal(&forgeplan, &outcome.transaction_id).unwrap();
        journal.version = None;
        journal.phase = None;
        journal.moves.clear();
        write_manifest(&outcome.journal_dir, &mut journal).unwrap();
        let manifest = fs::read(outcome.journal_dir.join("manifest.json")).unwrap();
        let postimages: BTreeMap<_, _> = journal.operations.iter()
            .map(|op| (op.path.clone(), fs::read(forgeplan.join(&op.path)).unwrap()))
            .collect();
        // A leftover incomplete staging directory is not a published receipt.
        let interrupted = outcome.journal_dir.join(".legacy-upgrade-interrupted");
        fs::create_dir(&interrupted).unwrap();
        fs::write(interrupted.join("partial"), b"untrusted").unwrap();
        prepare_legacy_upgrade_receipt(&forgeplan, &outcome.journal_dir, &journal).unwrap();
        // A crash after publication but before the rollback must be retryable.
        prepare_legacy_upgrade_receipt(&forgeplan, &outcome.journal_dir, &journal).unwrap();
        assert_eq!(fs::read(interrupted.join("partial")).unwrap(), b"untrusted");
        assert_eq!(fs::read(outcome.journal_dir.join("manifest.json")).unwrap(), manifest);
        rollback_repair(&forgeplan, &outcome.transaction_id).unwrap();
        let rolled = read_valid_journal(&forgeplan, &outcome.transaction_id).unwrap();
        assert_eq!(rolled.status, JournalStatus::RolledBackIndexPending);
        assert!(validate_legacy_upgrade_receipt(&outcome.journal_dir, &rolled).unwrap());
        for (path, bytes) in &postimages {
            assert_eq!(fs::read(postimage_file(&outcome.journal_dir.join("legacy-upgrade-v1"), path)).unwrap(), *bytes);
        }
        let first = &journal.operations[0].path;
        fs::write(postimage_file(&outcome.journal_dir.join("legacy-upgrade-v1"), first), b"damaged").unwrap();
        assert!(validate_pending_recovery(&forgeplan, &outcome.transaction_id).is_err());
    }

    #[test]
    fn legacy_partial_apply_and_rollback_remain_recoverable_without_forging_postimages() {
        for state in [OperationState::Pending, OperationState::RolledBack] {
            let workspace = colliding_workspace();
            let forgeplan = workspace.path().join(".forgeplan");
            let census = census_workspace(&forgeplan).unwrap();
            let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
            let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
            let mut journal = read_valid_journal(&forgeplan, &outcome.transaction_id).unwrap();
            journal.version = None;
            journal.phase = None;
            journal.moves.clear();
            journal.status = JournalStatus::InProgress;
            journal.operations[0].state = state;
            let path = journal.operations[0].path.clone();
            let preimage = fs::read(preimage_file(&outcome.journal_dir, &path)).unwrap();
            assert_ne!(sha256_hex(&preimage), journal.planned_postimages[&path]);
            fs::write(forgeplan.join(&path), &preimage).unwrap();
            write_manifest(&outcome.journal_dir, &mut journal).unwrap();
            let outcome = rollback_repair(&forgeplan, &outcome.transaction_id).unwrap();
            assert_eq!(outcome.status, JournalStatus::RolledBackIndexPending);
            assert_eq!(fs::read(forgeplan.join(&path)).unwrap(), preimage);
            assert!(!outcome.journal_dir.join("legacy-upgrade-v1").exists());
            // Admission remains available for the diagnostic-blocked collision
            // route; identity mapping itself fails explicitly without backup.
            validate_pending_recovery(&forgeplan, &outcome.transaction_id).unwrap();
        }
    }

    #[tokio::test]
    async fn upgraded_legacy_rollback_retains_identity_mappings() {
        use crate::db::migrate::{
            identity_v5_marker_path, IDENTITY_V5_ARTIFACTS_TABLE, IDENTITY_V5_EVIDENCE_TABLE,
            IDENTITY_V5_RELATIONS_TABLE, IDENTITY_V5_VERSION,
        };
        use crate::workspace::lock::acquire_legacy_repair_workspace_lock;
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan").canonicalize().unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let mut journal = read_valid_journal(&forgeplan, &outcome.transaction_id).unwrap();
        journal.version = None;
        journal.phase = None;
        journal.moves.clear();
        write_manifest(&outcome.journal_dir, &mut journal).unwrap();
        let posts: BTreeMap<_, _> = journal.operations.iter()
            .map(|op| (op.path.clone(), fs::read(forgeplan.join(&op.path)).unwrap()))
            .collect();
        rollback_repair(&forgeplan, &outcome.transaction_id).unwrap();
        let lance = forgeplan.join("lance");
        fs::create_dir(&lance).unwrap();
        let marker = serde_json::json!({
            "version": IDENTITY_V5_VERSION, "artifacts_table": IDENTITY_V5_ARTIFACTS_TABLE,
            "relations_table": IDENTITY_V5_RELATIONS_TABLE, "evidence_table": IDENTITY_V5_EVIDENCE_TABLE,
            "source_digest": "a".repeat(64), "document_count": 0, "completed": true
        });
        fs::write(identity_v5_marker_path(&lance), serde_json::to_vec(&marker).unwrap()).unwrap();
        let guard = acquire_legacy_repair_workspace_lock(&forgeplan, &outcome.transaction_id).await.unwrap();
        let proof = validate_legacy_repair(&forgeplan, &guard, &outcome.transaction_id).unwrap();
        let (status, mappings) = proof.identity_mappings().unwrap();
        assert_eq!(status, JournalStatus::RolledBackIndexPending);
        assert_eq!(mappings.len(), posts.len());
        for mapping in mappings {
            assert_eq!(mapping.postimage, posts[&mapping.path]);
            assert_eq!(mapping.preimage, fs::read(forgeplan.join(&mapping.path)).unwrap());
        }

        // A partially rolled-back legacy journal remains diagnostically
        // admissible but cannot fabricate an identity mapping for lost bytes.
        let partial = colliding_workspace();
        let root = partial.path().join(".forgeplan").canonicalize().unwrap();
        let census = census_workspace(&root).unwrap();
        let plan = build_repair_plan(&root, &census, &decision_for_second_prd()).unwrap();
        let applied = apply_repair_plan(&root, &plan).unwrap();
        let mut old = read_valid_journal(&root, &applied.transaction_id).unwrap();
        old.version = None;
        old.phase = None;
        old.moves.clear();
        old.status = JournalStatus::InProgress;
        old.operations[0].state = OperationState::RolledBack;
        let path = &old.operations[0].path;
        fs::write(root.join(path), fs::read(preimage_file(&applied.journal_dir, path)).unwrap()).unwrap();
        write_manifest(&applied.journal_dir, &mut old).unwrap();
        rollback_repair(&root, &applied.transaction_id).unwrap();
        let lance = root.join("lance");
        fs::create_dir(&lance).unwrap();
        fs::write(identity_v5_marker_path(&lance), serde_json::to_vec(&marker).unwrap()).unwrap();
        let guard = acquire_legacy_repair_workspace_lock(&root, &applied.transaction_id).await.unwrap();
        let proof = validate_legacy_repair(&root, &guard, &applied.transaction_id).unwrap();
        let error = proof.identity_mappings().err().unwrap();
        assert!(format!("{error:#}").contains("restore it from a backup"));
    }

    #[test]
    fn rollback_does_not_delete_identical_but_foreign_destination() {
        let workspace = sidecar_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &[]).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        let pair = plan.sidecar_moves.last().unwrap();
        let destination = forgeplan.join(&pair.destination_path);
        let bytes = fs::read(&destination).unwrap();
        fs::remove_file(&destination).unwrap();
        fs::write(&destination, &bytes).unwrap();
        assert!(validate_pending_recovery(&forgeplan, &outcome.transaction_id).is_err());
        assert!(rollback_repair(&forgeplan, &outcome.transaction_id).is_err());
        assert_eq!(fs::read(&destination).unwrap(), bytes);
        assert!(forgeplan.join(&pair.source_path).exists());
    }

    #[test]
    fn destination_race_with_identical_bytes_preserves_foreign_file() {
        let workspace = sidecar_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &[]).unwrap();
        let pair = &plan.sidecar_moves[0];
        let source = fs::read_to_string(forgeplan.join(&pair.source_path)).unwrap();
        let foreign = sidecar_postimage(&source, &pair.old_key, &pair.new_key).unwrap();
        let destination = forgeplan.join(&pair.destination_path);
        let mut injected = false;
        let result = apply_repair_plan_with_publisher(&forgeplan, &plan, |path, bytes| {
            if !injected {
                fs::write(&destination, &foreign).unwrap();
                injected = true;
            }
            publish(path, bytes)
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&destination).unwrap(), foreign);
        assert!(forgeplan.join(&pair.source_path).exists());
    }

    #[test]
    fn source_edit_during_apply_preserves_both_sidecar_images() {
        let workspace = sidecar_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &[]).unwrap();
        let pair = &plan.sidecar_moves[0];
        let source = forgeplan.join(&pair.source_path);
        let destination = forgeplan.join(&pair.destination_path);
        let mut injected = false;
        let result = apply_repair_plan_with_publisher(&forgeplan, &plan, |path, bytes| {
            if !injected {
                fs::write(&source, b"unrelated source edit").unwrap();
                injected = true;
            }
            publish(path, bytes)
        });
        assert!(result.is_err());
        assert_eq!(fs::read(source).unwrap(), b"unrelated source edit");
        assert_eq!(sha256_hex(&fs::read(destination).unwrap()), pair.postimage_sha256);
    }

    #[test]
    fn pending_admission_refuses_another_unfinished_journal() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        validate_pending_recovery(&forgeplan, &outcome.transaction_id).unwrap();
        let other_dir = repair_journal_dir(&forgeplan, "other");
        fs::create_dir(&other_dir).unwrap();
        fs::create_dir(other_dir.join("preimages")).unwrap();
        let path = "prds/PRD-001-a.md".to_string();
        let bytes = fs::read(forgeplan.join(&path)).unwrap();
        let hash = sha256_hex(&bytes);
        fs::write(preimage_file(&other_dir, &path), &bytes).unwrap();
        let mut other = RepairJournal {
            version: None,
            transaction_id: "other".into(),
            manifest_sha256: String::new(),
            preimages: BTreeMap::from([(path.clone(), hash.clone())]),
            planned_postimages: BTreeMap::from([(path.clone(), hash)]),
            operations: vec![JournalOperation { path, state: OperationState::Pending }],
            status: JournalStatus::InProgress,
            phase: None,
            moves: Vec::new(),
            legacy_diagnostic: None,
            legacy_predecessors: Vec::new(),
        };
        write_manifest(&other_dir, &mut other).unwrap();
        assert!(validate_pending_recovery(&forgeplan, &outcome.transaction_id).is_err());
    }

    async fn blocked_legacy_workspace() -> (tempfile::TempDir, String) {
        use crate::db::migrate::{
            identity_v5_marker_path, IDENTITY_V5_ARTIFACTS_TABLE, IDENTITY_V5_EVIDENCE_TABLE,
            IDENTITY_V5_RELATIONS_TABLE, IDENTITY_V5_VERSION,
        };
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan").canonicalize().unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let outcome = apply_repair_plan(&forgeplan, &plan).unwrap();
        rollback_repair(&forgeplan, &outcome.transaction_id).unwrap();
        let lance = forgeplan.join("lance");
        fs::create_dir(&lance).unwrap();
        let marker = serde_json::json!({
            "version": IDENTITY_V5_VERSION,
            "artifacts_table": IDENTITY_V5_ARTIFACTS_TABLE,
            "relations_table": IDENTITY_V5_RELATIONS_TABLE,
            "evidence_table": IDENTITY_V5_EVIDENCE_TABLE,
            "source_digest": "a".repeat(64),
            "document_count": 0,
            "completed": true
        });
        fs::write(identity_v5_marker_path(&lance), serde_json::to_vec(&marker).unwrap()).unwrap();
        let guard = crate::workspace::lock::acquire_legacy_repair_workspace_lock(&forgeplan, &outcome.transaction_id).await.unwrap();
        let proof = validate_legacy_repair(&forgeplan, &guard, &outcome.transaction_id).unwrap();
        let verified = crate::artifact::legacy_diagnostic::write_legacy_diagnostic(&proof).await.unwrap();
        finish_legacy_diagnostic(&proof, &forgeplan, &verified).await.unwrap();
        drop(verified);
        drop(proof);
        drop(guard);
        (workspace, outcome.transaction_id)
    }

    #[tokio::test]
    async fn blocked_successor_requires_exact_pair_for_explicit_rollback() {
        let (workspace, old_id) = blocked_legacy_workspace().await;
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        assert!(apply_repair_plan(&forgeplan, &plan).is_err());
        let result = apply_repair_plan_after_blocked(&forgeplan, &plan, &old_id).await.unwrap();
        assert!(check_repair_journals(&forgeplan, None).is_err());
        assert!(rollback_repair(&forgeplan, &result.transaction_id).is_err());
        assert!(rollback_linked_repair(&forgeplan, "wrong", &result.transaction_id).await.is_err());
        let rollback = rollback_linked_repair(&forgeplan, &old_id, &result.transaction_id).await.unwrap();
        assert_eq!(rollback.status, JournalStatus::RolledBackIndexPending);
        assert_eq!(read_valid_journal(&forgeplan, &old_id).unwrap().status, JournalStatus::RolledBackIndexBlocked);
        assert!(check_repair_journals(&forgeplan, None).is_err());
    }

    #[tokio::test]
    async fn diagnostic_successor_keeps_gate_and_can_chain_another_repair() {
        let (workspace, old_id) = blocked_legacy_workspace().await;
        let forgeplan = workspace.path().join(".forgeplan").canonicalize().unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let successor = apply_repair_plan_after_blocked(&forgeplan, &plan, &old_id).await.unwrap();
        rollback_linked_repair(&forgeplan, &old_id, &successor.transaction_id).await.unwrap();
        let guard = crate::workspace::lock::acquire_linked_repair_workspace_lock(
            &forgeplan, &old_id, &successor.transaction_id,
        ).await.unwrap();
        let proof = validate_legacy_repair(&forgeplan, &guard, &successor.transaction_id).unwrap();
        let diagnostic = crate::artifact::legacy_diagnostic::write_legacy_diagnostic(&proof).await.unwrap();
        finish_legacy_diagnostic(&proof, &forgeplan, &diagnostic).await.unwrap();
        drop(diagnostic);
        drop(proof);
        drop(guard);
        assert_eq!(read_valid_journal(&forgeplan, &old_id).unwrap().status, JournalStatus::RolledBack);
        assert_eq!(read_valid_journal(&forgeplan, &successor.transaction_id).unwrap().status, JournalStatus::RolledBackIndexBlocked);
        assert!(check_repair_journals(&forgeplan, None).is_err());
        let census = census_workspace(&forgeplan).unwrap();
        let next_plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let next = apply_repair_plan_after_blocked(&forgeplan, &next_plan, &successor.transaction_id).await.unwrap();
        let (middle, latest) = inspect_linked_repair_journals(&forgeplan, &successor.transaction_id, &next.transaction_id).await.unwrap();
        assert_eq!(middle.status, JournalStatus::RolledBackIndexBlocked);
        assert_eq!(latest.status, JournalStatus::CanonicalCommittedIndexPending);
        assert!(check_repair_journals(&forgeplan, None).is_err());
    }

    #[tokio::test]
    async fn old_terminal_before_successor_terminal_never_opens_the_gate() {
        let (workspace, old_id) = blocked_legacy_workspace().await;
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let result = apply_repair_plan_after_blocked(&forgeplan, &plan, &old_id).await.unwrap();
        let mut old = read_valid_journal(&forgeplan, &old_id).unwrap();
        old.status = JournalStatus::RolledBack;
        write_manifest(&repair_journal_dir(&forgeplan, &old_id), &mut old).unwrap();
        assert!(check_repair_journals(&forgeplan, None).is_err());
        let (seen_old, seen_new) = inspect_linked_repair_journals(&forgeplan, &old_id, &result.transaction_id).await.unwrap();
        assert_eq!(seen_old.status, JournalStatus::RolledBack);
        assert_eq!(seen_new.status, JournalStatus::CanonicalCommittedIndexPending);
        rollback_linked_repair(&forgeplan, &old_id, &result.transaction_id).await.unwrap();
        assert!(check_repair_journals(&forgeplan, None).is_err());
    }

    #[tokio::test]
    async fn corrupted_blocked_snapshot_refuses_successor_before_mutation() {
        let (workspace, old_id) = blocked_legacy_workspace().await;
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let before = census.inventory_digest.clone();
        fs::write(repair_journal_dir(&forgeplan, &old_id).join("legacy-diagnostic.json"), b"corrupted").unwrap();
        assert!(apply_repair_plan_after_blocked(&forgeplan, &plan, &old_id).await.is_err());
        assert_eq!(census_workspace(&forgeplan).unwrap().inventory_digest, before);
        assert!(check_repair_journals(&forgeplan, None).is_err());
    }

    #[tokio::test]
    async fn unbound_successor_recovers_only_if_every_path_is_still_preimage() {
        let (workspace, old_id) = blocked_legacy_workspace().await;
        let forgeplan = workspace.path().join(".forgeplan");
        let census = census_workspace(&forgeplan).unwrap();
        let before = census.inventory_digest.clone();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let operations = plan_operations(&forgeplan, &plan, &census).unwrap();
        let old = read_valid_journal(&forgeplan, &old_id).unwrap();
        let new_id = "repair-unbound";
        let journal_dir = repair_journal_dir(&forgeplan, new_id);
        fs::create_dir(&journal_dir).unwrap();
        let mut new = RepairJournal {
            version: Some(2),
            transaction_id: new_id.into(),
            manifest_sha256: String::new(),
            preimages: operations.iter().filter_map(|op| op.preimage.as_ref().map(|bytes| (op.path.clone(), sha256_hex(bytes)))).collect(),
            planned_postimages: operations.iter().filter_map(|op| op.postimage.as_ref().map(|bytes| (op.path.clone(), sha256_hex(bytes)))).collect(),
            operations: operations.iter().map(|op| JournalOperation { path: op.path.clone(), state: OperationState::Pending }).collect(),
            status: JournalStatus::InProgress,
            phase: Some(RepairPhase::Applying),
            moves: Vec::new(),
            legacy_diagnostic: None,
            legacy_predecessors: vec![LegacyRepairPredecessor {
                transaction_id: old_id.clone(),
                blocked_manifest_sha256: old.manifest_sha256,
            }],
        };
        write_preimages(&journal_dir, &operations).unwrap();
        write_manifest(&journal_dir, &mut new).unwrap();
        assert!(check_repair_journals(&forgeplan, None).is_err());
        let outcome = rollback_linked_repair(&forgeplan, &old_id, new_id).await.unwrap();
        assert_eq!(outcome.status, JournalStatus::RolledBack);
        assert_eq!(census_workspace(&forgeplan).unwrap().inventory_digest, before);
        assert_eq!(read_valid_journal(&forgeplan, &old_id).unwrap().status, JournalStatus::RolledBackIndexBlocked);
        assert!(check_repair_journals(&forgeplan, None).is_err());
    }

    #[test]
    fn prepared_directory_publication_never_replaces_a_foreign_directory() {
        let root = tempfile::tempdir().unwrap();
        let stage = root.path().join("stage");
        let target = root.path().join("target");
        fs::create_dir(&stage).unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(stage.join("ours"), b"ours").unwrap();
        fs::write(target.join("foreign"), b"foreign").unwrap();
        assert!(publish_directory_noclobber(&stage, &target).is_err());
        assert_eq!(fs::read(stage.join("ours")).unwrap(), b"ours");
        assert_eq!(fs::read(target.join("foreign")).unwrap(), b"foreign");
    }

    #[test]
    fn every_preparation_checkpoint_preserves_canonical_bytes_and_supports_safe_retry() {
        for point in [
            "journal_root", "staging_root", "staging_directory", "postimages_directory",
            "staged_directory", "preimages_directory", "blob_parent_sync", "preimage_blob",
            "postimage_blob", "create_blob", "preimages_sync", "postimages_sync",
            "staged_sync", "manifest", "ready", "directory_publish",
            "staging_parent_sync", "journal_parent_sync",
        ] {
            let workspace = sidecar_workspace();
            let forgeplan = workspace.path().join(".forgeplan").canonicalize().unwrap();
            let census = census_workspace(&forgeplan).unwrap();
            let before = census.inventory_digest.clone();
            let plan = build_repair_plan(&forgeplan, &census, &[]).unwrap();
            PREPARATION_FAILURE.with(|slot| *slot.borrow_mut() = Some(point));
            let error = apply_repair_plan(&forgeplan, &plan).unwrap_err();
            assert!(format!("{error:#}").contains(point), "checkpoint {point}: {error:#}");
            assert_eq!(census_workspace(&forgeplan).unwrap().inventory_digest, before);
            if matches!(point, "journal_root" | "staging_root") {
                let retried = apply_repair_plan(&forgeplan, &plan).unwrap();
                rollback_repair(&forgeplan, &retried.transaction_id).unwrap();
            } else {
                let published = matches!(point, "directory_publish" | "staging_parent_sync" | "journal_parent_sync");
                let dir = if published { repair_journal_root(&forgeplan) } else { repair_staging_root(&forgeplan) };
                let entry = fs::read_dir(dir).unwrap().next().unwrap().unwrap();
                let id = entry.file_name().into_string().unwrap();
                assert!(check_repair_journals(&forgeplan, None).is_err());
                if published || point == "ready" {
                    recover_prepared_repair(&forgeplan, &id).unwrap();
                    recover_prepared_repair(&forgeplan, &id).unwrap();
                    rollback_repair(&forgeplan, &id).unwrap();
                } else {
                    // No READY means explicit recovery diagnoses but never
                    // removes or assumes ownership of incomplete staging.
                    assert!(recover_prepared_repair(&forgeplan, &id).is_err());
                    assert!(entry.path().exists());
                    assert!(recover_prepared_repair(&forgeplan, &id).is_err());
                }
            }
            assert_eq!(census_workspace(&forgeplan).unwrap().inventory_digest, before);
        }
    }

    #[test]
    fn prepared_journal_crash_boundaries_are_fail_closed_and_retryable() {
        let workspace = colliding_workspace();
        let forgeplan = workspace.path().join(".forgeplan").canonicalize().unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let before = census.inventory_digest.clone();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let operations = plan_operations(&forgeplan, &plan, &census).unwrap();
        let id = "repair-staged";
        let root = repair_journal_root(&forgeplan);
        let staging_root = repair_staging_root(&forgeplan);
        fs::create_dir(&root).unwrap();
        fs::create_dir(&staging_root).unwrap();
        let stage = staging_root.join(id);
        fs::create_dir(&stage).unwrap();
        // Crash after directory creation: no manifest is assumed safe.
        assert!(check_repair_journals(&forgeplan, None).is_err());
        assert!(recover_prepared_repair(&forgeplan, id).is_err());
        assert!(stage.exists());
        // Each partially constructed blob directory remains blocked and intact.
        for name in ["postimages", "staged", "preimages"] {
            fs::create_dir(stage.join(name)).unwrap();
            assert!(recover_prepared_repair(&forgeplan, id).is_err());
            assert!(stage.join(name).is_dir());
        }
        let first_blob = preimage_file(&stage, &operations[0].path);
        fs::write(&first_blob, operations[0].preimage.as_ref().unwrap()).unwrap();
        assert!(recover_prepared_repair(&forgeplan, id).is_err());
        assert!(first_blob.exists());
        fs::remove_file(&first_blob).unwrap();
        for blob in [postimage_file(&stage, &operations[0].path), staged_file(&stage, &operations[0].path)] {
            fs::write(&blob, operations[0].postimage.as_ref().unwrap()).unwrap();
            assert!(recover_prepared_repair(&forgeplan, id).is_err());
            assert!(blob.exists());
            fs::remove_file(blob).unwrap();
        }
        for name in ["postimages", "staged", "preimages"] {
            fs::remove_dir(stage.join(name)).unwrap();
        }
        write_preimages(&stage, &operations).unwrap();
        // Crash after blob writes, before the manifest.
        assert!(recover_prepared_repair(&forgeplan, id).is_err());
        let mut journal = RepairJournal {
            version: Some(2),
            transaction_id: id.into(),
            manifest_sha256: String::new(),
            preimages: operations.iter().filter_map(|op| op.preimage.as_ref().map(|bytes| (op.path.clone(), sha256_hex(bytes)))).collect(),
            planned_postimages: operations.iter().filter_map(|op| op.postimage.as_ref().map(|bytes| (op.path.clone(), sha256_hex(bytes)))).collect(),
            operations: operations.iter().map(|op| JournalOperation { path: op.path.clone(), state: OperationState::Pending }).collect(),
            status: JournalStatus::InProgress,
            phase: Some(RepairPhase::Applying),
            moves: Vec::new(),
            legacy_diagnostic: None,
            legacy_predecessors: Vec::new(),
        };
        write_manifest(&stage, &mut journal).unwrap();
        // A manifest without the durable READY receipt is still incomplete.
        assert!(recover_prepared_repair(&forgeplan, id).is_err());
        let receipt = PreparedRepairReceipt {
            version: 1,
            transaction_id: id.into(),
            workspace_path: forgeplan.to_string_lossy().to_string(),
            plan_digest: plan.digest.clone(),
            manifest_digest: journal.manifest_sha256.clone(),
        };
        write_file_atomically(&stage.join("READY.json"), &serde_json::to_vec(&receipt).unwrap()).unwrap();
        let victim = preimage_file(&stage, &operations[0].path);
        let original = fs::read(&victim).unwrap();
        fs::write(&victim, b"corrupted").unwrap();
        assert!(recover_prepared_repair(&forgeplan, id).is_err());
        assert!(stage.exists());
        fs::write(&victim, original).unwrap();
        recover_prepared_repair(&forgeplan, id).unwrap();
        assert!(!stage.exists());
        // Retry after rename/fsync uncertainty is idempotent.
        recover_prepared_repair(&forgeplan, id).unwrap();
        assert_eq!(census_workspace(&forgeplan).unwrap().inventory_digest, before);
        assert!(check_repair_journals(&forgeplan, None).is_err());
        rollback_repair(&forgeplan, id).unwrap();
        assert_eq!(census_workspace(&forgeplan).unwrap().inventory_digest, before);
    }

    #[tokio::test]
    async fn prepared_linked_successor_is_published_then_rolled_back_without_opening_gate() {
        let (workspace, old_id) = blocked_legacy_workspace().await;
        let forgeplan = workspace.path().join(".forgeplan").canonicalize().unwrap();
        let census = census_workspace(&forgeplan).unwrap();
        let before = census.inventory_digest.clone();
        let plan = build_repair_plan(&forgeplan, &census, &decision_for_second_prd()).unwrap();
        let operations = plan_operations(&forgeplan, &plan, &census).unwrap();
        let old = read_valid_journal(&forgeplan, &old_id).unwrap();
        let id = "repair-prepared-linked";
        let stage = repair_staging_root(&forgeplan).join(id);
        fs::create_dir(&stage).unwrap();
        let mut journal = RepairJournal {
            version: Some(2),
            transaction_id: id.into(),
            manifest_sha256: String::new(),
            preimages: operations.iter().filter_map(|op| op.preimage.as_ref().map(|bytes| (op.path.clone(), sha256_hex(bytes)))).collect(),
            planned_postimages: operations.iter().filter_map(|op| op.postimage.as_ref().map(|bytes| (op.path.clone(), sha256_hex(bytes)))).collect(),
            operations: operations.iter().map(|op| JournalOperation { path: op.path.clone(), state: OperationState::Pending }).collect(),
            status: JournalStatus::InProgress,
            phase: Some(RepairPhase::Applying),
            moves: Vec::new(),
            legacy_diagnostic: None,
            legacy_predecessors: vec![LegacyRepairPredecessor {
                transaction_id: old_id.clone(),
                blocked_manifest_sha256: old.manifest_sha256,
            }],
        };
        write_preimages(&stage, &operations).unwrap();
        write_manifest(&stage, &mut journal).unwrap();
        let receipt = PreparedRepairReceipt {
            version: 1,
            transaction_id: id.into(),
            workspace_path: forgeplan.to_string_lossy().to_string(),
            plan_digest: plan.digest,
            manifest_digest: journal.manifest_sha256.clone(),
        };
        write_file_atomically(&stage.join("READY.json"), &serde_json::to_vec(&receipt).unwrap()).unwrap();
        assert!(check_repair_journals(&forgeplan, None).is_err());
        recover_prepared_repair(&forgeplan, id).unwrap();
        assert!(check_repair_journals(&forgeplan, None).is_err());
        let result = rollback_linked_repair(&forgeplan, &old_id, id).await.unwrap();
        assert_eq!(result.status, JournalStatus::RolledBack);
        assert_eq!(census_workspace(&forgeplan).unwrap().inventory_digest, before);
        assert_eq!(read_valid_journal(&forgeplan, &old_id).unwrap().status, JournalStatus::RolledBackIndexBlocked);
    }
}

#[cfg(test)]
mod link_entry_tests {
    use super::*;

    /// A `links:` list whose items start at column zero is the shape the
    /// projection writes; the item's key sits after the `- ` marker, and a
    /// continuation key sits after the indentation.
    #[test]
    fn link_entries_reads_a_column_zero_list() {
        let content = "---\nid: EVID-001\nkind: evidence\ntitle: EVID-001\nlinks:\n- target: PRD-001\n  relation: informs\n---\n\n# EVID-001\n\nBody.\n";
        let entries = link_entries(content).expect("links parse");
        assert_eq!(entries.len(), 1, "entries: {entries:?}");
        assert_eq!(entries[0].target, "PRD-001");
        assert_eq!(entries[0].relation, "informs");
    }
}

#[cfg(test)]
mod plan_reference_tests {
    use super::*;
    use std::fs;

    /// A reference that names two documents stays unresolved until a review
    /// attributes it: the plan carries it with an empty target and the
    /// mapping of every candidate, so the apply cannot pick a winner.
    #[test]
    fn an_ambiguous_reference_stays_unresolved_until_a_review_decides_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let forgeplan = dir.path().join(".forgeplan");
        for kind in crate::workspace::ARTIFACT_DIRS {
            fs::create_dir_all(forgeplan.join(kind)).expect("dir");
        }
        fs::write(
            forgeplan.join("prds/PRD-001-a.md"),
            "---\nid: PRD-001\nkind: prd\ntitle: First\n---\n\n# PRD-001\n\nBody of PRD-001.\n",
        )
        .expect("write");
        fs::write(
            forgeplan.join("prds/PRD-001-b.md"),
            "---\nid: PRD-001\nkind: prd\ntitle: Second\n---\n\n# PRD-001\n\nBody of PRD-001.\n",
        )
        .expect("write");
        fs::write(
            forgeplan.join("evidence/EVID-001.md"),
            "---\nid: EVID-001\nkind: evidence\ntitle: EVID-001\nlinks:\n- target: PRD-001\n  relation: informs\n---\n\n# EVID-001\n\nBody.\n",
        )
        .expect("write");
        let census = census_workspace(&forgeplan).expect("census");
        let plan = build_repair_plan(&forgeplan, &census, &[]).expect("plan");
        for reference in &plan.references {
            println!("REF from={:?} to={:?} resolved={} mapping={:?}", reference.from, reference.to, reference.is_resolved(), reference.mapping);
        }
        assert_eq!(plan.references.len(), 1, "references: {:?}", plan.references);
        assert!(!plan.references[0].is_resolved(), "the ambiguous reference must stay unresolved: {:?}", plan.references[0]);
    }
}
