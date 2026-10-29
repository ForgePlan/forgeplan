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
//! The repair is preserve-all: nothing is deleted, nothing is merged, and
//! no historical display number is rewritten. What changes is the canonical
//! key of every member of a colliding group, and the inbound references
//! that pointed at those members.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::artifact::canonical_key::{canonical_key_from_document, mint_artifact_key};
use crate::artifact::frontmatter::parse_frontmatter;
use crate::artifact::types::ArtifactKind;
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
    /// Digest over the whole document corpus of the workspace.
    pub inventory_digest: String,
    /// Documents whose canonical key changes.
    pub sources: Vec<PlannedSource>,
    /// Inbound references the repair rewrites.
    pub references: Vec<PlannedReference>,
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
    /// The rollback refused to overwrite an unrelated edit.
    #[serde(rename = "conflict")]
    Conflict,
}

/// The recovery journal of a repair transaction.
///
/// The journal is the durable record of a transaction: the preimage of every
/// file the transaction touches, the post-image it planned to publish, and
/// the state of every operation. It is written and fsynced before the first
/// mutation, so a crash always leaves a journal that can restore the
/// workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairJournal {
    /// Identifier of the transaction; also the name of its journal directory.
    pub transaction_id: String,
    /// Digest over every field of this journal except the digest itself.
    pub manifest_sha256: String,
    /// Workspace-relative path → SHA-256 of the preimage at that path.
    pub preimages: BTreeMap<String, String>,
    /// Workspace-relative path → SHA-256 of the post-image published there.
    pub planned_postimages: BTreeMap<String, String>,
    /// One entry per file operation.
    pub operations: Vec<JournalOperation>,
    /// Status of the transaction.
    pub status: JournalStatus,
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
        matches!(self.status, JournalStatus::InProgress)
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
    let blocks = consumed_blocks(content)?;
    let Some(index) = block_with_key(content, &blocks, "links") else {
        return Ok(Vec::new());
    };
    let block = &blocks[index];
    let lines = line_ranges(content);
    let Some(links_line) = top_level_key_line(content, &lines, block, "links") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (occurrence, item) in link_item_ranges(content, &lines, block, links_line)
        .into_iter()
        .enumerate()
    {
        let target = item_scalar(content, &lines, item, "target").unwrap_or_default();
        let relation = item_scalar(content, &lines, item, "relation").unwrap_or_default();
        out.push(LinkEntry {
            occurrence,
            target,
            relation,
        });
    }
    Ok(out)
}

/// The `parent_epic` value of a document's effective frontmatter.
pub fn parent_epic_target(content: &str) -> Result<Option<String>> {
    let blocks = consumed_blocks(content)?;
    let Some(index) = block_with_key(content, &blocks, "parent_epic") else {
        return Ok(None);
    };
    let block = &blocks[index];
    let lines = line_ranges(content);
    let Some(line) = top_level_key_line(content, &lines, block, "parent_epic") else {
        return Ok(None);
    };
    let (start, end) = scalar_range(content, &lines[line], "parent_epic".len())?;
    Ok(Some(content[start..end].to_string()))
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
    let mut out = String::with_capacity(content.len() + new_target.len());
    out.push_str(&content[..start]);
    out.push_str(new_target);
    out.push_str(&content[end..]);
    Ok(out)
}

/// Rewrite the `parent_epic` value of a document.
pub fn set_parent_epic(content: &str, new_target: &str) -> Result<String> {
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
    let mut out = String::with_capacity(content.len() + new_target.len());
    out.push_str(&content[..start]);
    out.push_str(new_target);
    out.push_str(&content[end..]);
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
    let bytes = content.as_bytes();
    let mut value_start = start + key_len + 1;
    while value_start < end && (bytes[value_start] == b' ' || bytes[value_start] == b'\t') {
        value_start += 1;
    }
    let mut value_end = end;
    while value_end > value_start
        && matches!(bytes[value_end - 1], b' ' | b'\t' | b'\r')
    {
        value_end -= 1;
    }
    Ok((value_start, value_end))
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
        let offset = if let Some(stripped) = text.strip_prefix("- ") {
            stripped.len()
        } else if text.starts_with('-') {
            text.len() - text.trim_start_matches('-').len()
        } else {
            0
        };
        let candidate = &text[offset..];
        if candidate.starts_with(&prefix) {
            let value_start = start + offset + prefix.len();
            let bytes = content.as_bytes();
            let mut vs = value_start;
            while vs < end && (bytes[vs] == b' ' || bytes[vs] == b'\t') {
                vs += 1;
            }
            let mut ve = end;
            while ve > vs && matches!(bytes[ve - 1], b' ' | b'\t' | b'\r') {
                ve -= 1;
            }
            return Some((vs, ve));
        }
    }
    None
}

/// The value of `key:` inside one `links` item.
fn item_scalar(
    content: &str,
    lines: &[(usize, usize)],
    item: (usize, usize),
    key: &str,
) -> Option<String> {
    let (start, end) = item_scalar_range(content, lines, item, key)?;
    Some(content[start..end].to_string())
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
/// The list is the one `reconcile-ids` scans: `Memory` is left out because
/// a memory document carries no display number and takes no part in the
/// lifecycle identity.
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

/// The census of a workspace.
#[derive(Debug, Clone)]
pub struct Census {
    /// Every document of the workspace, sorted by relative path.
    pub documents: Vec<CensusDocument>,
    /// Digest over the path and the file hash of every document.
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
    for kind in REPAIR_KINDS {
        let dir = forgeplan_dir.join(kind.dir_name());
        if !dir.is_dir() {
            continue;
        }
        let meta = fs::symlink_metadata(&dir)
            .with_context(|| format!("read kind directory {}", dir.display()))?;
        if meta.file_type().is_symlink() {
            bail!(
                "refusing to census a symlinked kind directory: {}",
                dir.display()
            );
        }
        let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
            .with_context(|| format!("read kind directory {}", dir.display()))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("read kind directory {}", dir.display()))?;
        paths.sort();
        for path in paths {
            if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
                continue;
            }
            let meta = fs::symlink_metadata(&path)
                .with_context(|| format!("read document {}", path.display()))?;
            if meta.file_type().is_symlink() {
                bail!(
                    "refusing to census a symlinked document: {}",
                    path.display()
                );
            }
            if !meta.is_file() {
                continue;
            }
            let relative_path = workspace_relative_path(forgeplan_dir, &path)?;
            let content = fs::read_to_string(&path)
                .with_context(|| format!("read document {relative_path}"))?;
            documents.push(census_document(relative_path, kind, &content)?);
        }
    }
    documents.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let inventory_digest = compute_inventory_digest(&documents);
    Ok(Census {
        documents,
        inventory_digest,
    })
}

/// The digest over the whole document corpus.
///
/// The digest covers the path and the file hash of every document, so any
/// change anywhere in the corpus — a new document, an edited body, a
/// removed file — changes the digest and makes the plan stale.
pub fn compute_inventory_digest(documents: &[CensusDocument]) -> String {
    let mut lines: Vec<String> = Vec::with_capacity(documents.len());
    for document in documents {
        lines.push(format!(
            "{}\t{}",
            document.relative_path, document.hashes.file_sha256
        ));
    }
    sha256_hex(lines.join("\n").as_bytes())
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
    Ok(file)
}

/// Merge reviewed decisions, the later ones winning.
///
/// A decision is identified by the document that holds the reference, the
/// field and the target text — not by an index — so a decision survives an
/// unrelated edit of the same document.
pub fn merge_decisions(
    base: &[ReferenceDecision],
    extra: &[ReferenceDecision],
) -> Vec<ReferenceDecision> {
    let mut merged: BTreeMap<(String, String, String), ReferenceDecision> = BTreeMap::new();
    for decision in base.iter().chain(extra.iter()) {
        let key = (
            decision.source_path.clone(),
            decision.field.clone(),
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
/// The one genuinely ambiguous reference, `NOTE-019` → `PRD-072` in the
/// Marketplace project, was decided by the owner on 2026-10-28: it points at
/// the **deterministic** PRD-072 — the one whose own subject is the
/// determinism gap the note describes. The decision is encoded here so the
/// repair never has to leave it open.
///
/// The paths are workspace-relative, so the same record serves a pilot copy
/// and any other workspace that carries the same documents.
///
/// A `LazyLock` rather than a `const`: the records own `String`s, and
/// `String::to_string` is not a const fn.
pub static PILOT_REFERENCE_DECISIONS: LazyLock<Vec<ReferenceDecision>> = LazyLock::new(|| {
    vec![
    // -- ForgePlanWeb ------------------------------------------------------
    ReferenceDecision {
        source_path: "evidence/EVID-004-claude-md-baseline-guides-skaffold-smoke-pass-all-5-sc-validated.md".to_string(),
        field: "links".to_string(),
        from: "PRD-001".to_string(),
        target_path: "prds/PRD-001-bootstrap-claude-md-baseline-and-add-methodological-guides.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-005-safety-hardening-init-force-hook-red-line-8-windows-ci-fully-green-for-the-first-time.md".to_string(),
        field: "links".to_string(),
        from: "PRD-001".to_string(),
        target_path: "prds/PRD-001-bootstrap-claude-md-baseline-and-add-methodological-guides.md".to_string(),
    },
    ReferenceDecision {
        source_path: "rfcs/RFC-015-mosaic-split-tree-layout-engine-for-multi-graph-canvas.md".to_string(),
        field: "links".to_string(),
        from: "PRD-016".to_string(),
        target_path: "prds/PRD-016-multi-graph-mosaic-dashboard-with-persistent-layout.md".to_string(),
    },
    ReferenceDecision {
        source_path: "rfcs/RFC-015-identity-aware-route-structured-snapshot-error-surface.md".to_string(),
        field: "links".to_string(),
        from: "PRD-016".to_string(),
        target_path: "prds/PRD-016-prob-060-slug-canonical-identity-in-web-snapshot-error-surfacing.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-020-mosaic-layout-engine-unit-tests-svelte-check-pass.md".to_string(),
        field: "links".to_string(),
        from: "PRD-016".to_string(),
        target_path: "prds/PRD-016-multi-graph-mosaic-dashboard-with-persistent-layout.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-020-mosaic-layout-engine-unit-tests-svelte-check-pass.md".to_string(),
        field: "links".to_string(),
        from: "RFC-015".to_string(),
        target_path: "rfcs/RFC-015-mosaic-split-tree-layout-engine-for-multi-graph-canvas.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-021-prd-016-verified-typecheck-clean-179-tests-pass-identifier-guard-correct.md".to_string(),
        field: "links".to_string(),
        from: "PRD-016".to_string(),
        target_path: "prds/PRD-016-prob-060-slug-canonical-identity-in-web-snapshot-error-surfacing.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-021-prd-016-verified-typecheck-clean-179-tests-pass-identifier-guard-correct.md".to_string(),
        field: "links".to_string(),
        from: "RFC-015".to_string(),
        target_path: "rfcs/RFC-015-identity-aware-route-structured-snapshot-error-surface.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-040-code-review-of-commit-0101560-rfc-015-d-4-pass.md".to_string(),
        field: "links".to_string(),
        from: "RFC-015".to_string(),
        target_path: "rfcs/RFC-015-identity-aware-route-structured-snapshot-error-surface.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-040-code-review-of-commit-0101560-rfc-015-d-4-pass.md".to_string(),
        field: "links".to_string(),
        from: "PRD-016".to_string(),
        target_path: "prds/PRD-016-prob-060-slug-canonical-identity-in-web-snapshot-error-surfacing.md".to_string(),
    },
    // -- ForgePlanMarketplace ----------------------------------------------
    ReferenceDecision {
        source_path: "adrs/ADR-007-determinism-is-behavioral-implementation-passes-the-frozen-conformance-corpus-sha-not-bitwise.md".to_string(),
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-deterministic-sdd-tdd-pipeline-frozen-language-neutral-conformance-corpus-drives-multi-language-tdd.md".to_string(),
    },
    ReferenceDecision {
        source_path: "adrs/ADR-008-spec-driven-development-on-the-spec-kind-is-the-pipeline-spine-frozen-corpus-conformance-is-an-optional-gate.md".to_string(),
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-spec-driven-pipeline-openspec-speckit-style-on-the-spec-kind-optional-conformance-vectors-for-pure-cores.md".to_string(),
    },
    // The owner decision of 2026-10-28: NOTE-019 points at the
    // deterministic PRD-072.
    ReferenceDecision {
        source_path: "notes/NOTE-019-research-context-deterministic-sdd-tdd-openspec-speckit-3-layer-orchestration-playbook-readiness.md".to_string(),
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-deterministic-sdd-tdd-pipeline-frozen-language-neutral-conformance-corpus-drives-multi-language-tdd.md".to_string(),
    },
    ReferenceDecision {
        source_path: "notes/NOTE-020-spec-authoring-template-light-path-requirements-scenario-behavioral-contract-contracts-optional-conformance-vectors.md".to_string(),
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-spec-driven-pipeline-openspec-speckit-style-on-the-spec-kind-optional-conformance-vectors-for-pure-cores.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-121-adi-decision-adopt-spec-driven-light-pipeline-demote-frozen-corpus-cross-language-gate-to-optional.md".to_string(),
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-spec-driven-pipeline-openspec-speckit-style-on-the-spec-kind-optional-conformance-vectors-for-pure-cores.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-123-artifact-health-audit-prd-072-concerns-related-artifacts-table-incomplete-premature-supersede-claim.md".to_string(),
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-spec-driven-pipeline-openspec-speckit-style-on-the-spec-kind-optional-conformance-vectors-for-pure-cores.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-127-code-review-of-prd-072-phase-2-feat-spec-author-skill-light-pipeline-concerns.md".to_string(),
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-spec-driven-pipeline-openspec-speckit-style-on-the-spec-kind-optional-conformance-vectors-for-pure-cores.md".to_string(),
    },
    ReferenceDecision {
        source_path: "specs/SPEC-001-semver-compare-behavioral-contract-embedded-conformance-corpus-first-deterministic-sdd-tdd-increment.md".to_string(),
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-deterministic-sdd-tdd-pipeline-frozen-language-neutral-conformance-corpus-drives-multi-language-tdd.md".to_string(),
    },
    ReferenceDecision {
        source_path: "specs/SPEC-001-semver-compare-requirements-scenarios-behavioral-contract-optional-conformance-vectors-light-path-demonstrator.md".to_string(),
        field: "links".to_string(),
        from: "PRD-072".to_string(),
        target_path: "prds/PRD-072-spec-driven-pipeline-openspec-speckit-style-on-the-spec-kind-optional-conformance-vectors-for-pure-cores.md".to_string(),
    },
    ReferenceDecision {
        source_path: "notes/NOTE-022-test-plan-for-spec-001-semver-compare-precedence-total-order-invariants-language-neutral.md".to_string(),
        field: "links".to_string(),
        from: "SPEC-001".to_string(),
        target_path: "specs/SPEC-001-semver-compare-requirements-scenarios-behavioral-contract-optional-conformance-vectors-light-path-demonstrator.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-119-spike-result-semver-compare-ts-py-18-18-vectors-4-4-invariants-10-10-determinism-negative-control-caught.md".to_string(),
        field: "links".to_string(),
        from: "SPEC-001".to_string(),
        target_path: "specs/SPEC-001-semver-compare-behavioral-contract-embedded-conformance-corpus-first-deterministic-sdd-tdd-increment.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-119-spike-result-semver-compare-on-ts-py-go-rust-20-20-vectors-4-language-equivalence-per-language-negative-control.md".to_string(),
        field: "links".to_string(),
        from: "SPEC-001".to_string(),
        target_path: "specs/SPEC-001-semver-compare-behavioral-contract-embedded-conformance-corpus-first-deterministic-sdd-tdd-increment.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-125-adi-spec-001-reframe-maintain-in-place-not-supersede-removed-0-corpus-retained-as-optional.md".to_string(),
        field: "links".to_string(),
        from: "SPEC-001".to_string(),
        target_path: "specs/SPEC-001-semver-compare-behavioral-contract-embedded-conformance-corpus-first-deterministic-sdd-tdd-increment.md".to_string(),
    },
    ReferenceDecision {
        source_path: "evidence/EVID-126-artifact-health-audit-spec-001-light-path-reframe-concerns-scenario-adequacy-gaps-maintain-in-place-agreed.md".to_string(),
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

    let mut plan = RepairPlan {
        version: REPAIR_PLAN_VERSION,
        workspace_path: workspace_path.to_string_lossy().to_string(),
        git: detect_git_identity(forgeplan_dir)?,
        inventory_digest: census.inventory_digest.clone(),
        sources,
        references,
        digest: String::new(),
    };
    plan.digest = compute_plan_digest(&plan);
    Ok(plan)
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
    if candidates.is_empty() {
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
            })
            .and_then(|decision| {
                candidates
                    .iter()
                    .find(|candidate| candidate.relative_path == decision.target_path)
            })
            .copied()
    };
    let to = attributed
        .and_then(|candidate| new_key_by_path.get(&candidate.relative_path).cloned())
        .unwrap_or_default();
    if to.is_empty() {
        // The occurrence only belongs in the plan when it points at a
        // document whose key changes. Otherwise it needs no rewrite.
        let touches_changed_key = candidates
            .iter()
            .any(|candidate| new_key_by_path.contains_key(&candidate.relative_path));
        if !touches_changed_key {
            return None;
        }
    }
    if !to.is_empty() && to == target {
        return None;
    }
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
/// Without a key match the display id is matched, case-insensitively, and
/// every match counts.
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
    if !by_key.is_empty() {
        return by_key;
    }
    census
        .documents
        .iter()
        .filter(|document| document.display_id.eq_ignore_ascii_case(trimmed))
        .collect()
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
    /// The bytes the document holds today.
    preimage: Vec<u8>,
    /// The bytes the transaction publishes there.
    postimage: Vec<u8>,
}

/// The directory the recovery journals live in.
pub fn repair_journal_root(forgeplan_dir: &Path) -> PathBuf {
    forgeplan_dir.join("identity-repair")
}

/// The directory of one transaction's journal.
pub fn repair_journal_dir(forgeplan_dir: &Path, transaction_id: &str) -> PathBuf {
    repair_journal_root(forgeplan_dir).join(transaction_id)
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

    let _guard = acquire_repair_lock(forgeplan_dir)?;

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

    // The sources, the references and the target paths.
    let operations = plan_operations(forgeplan_dir, plan, &census)?;
    if operations.is_empty() {
        bail!("the repair plan touches no document — there is nothing to repair");
    }

    // The journal is durable before the first mutation.
    let transaction_id = new_transaction_id();
    let journal_dir = repair_journal_dir(forgeplan_dir, &transaction_id);
    let mut journal = RepairJournal {
        transaction_id: transaction_id.clone(),
        manifest_sha256: String::new(),
        preimages: operations
            .iter()
            .map(|operation| (operation.path.clone(), sha256_hex(&operation.preimage)))
            .collect(),
        planned_postimages: operations
            .iter()
            .map(|operation| (operation.path.clone(), sha256_hex(&operation.postimage)))
            .collect(),
        operations: operations
            .iter()
            .map(|operation| JournalOperation {
                path: operation.path.clone(),
                state: OperationState::Pending,
            })
            .collect(),
        status: JournalStatus::InProgress,
    };
    write_preimages(&journal_dir, &operations)?;
    write_manifest(&journal_dir, &mut journal)?;

    // Publish every post-image atomically.
    let mut published: Vec<String> = Vec::new();
    for operation in &operations {
        if let Err(error) = publish(&operation.absolute, &operation.postimage) {
            let restored = restore_preimages(&operations, &published);
            for entry in &mut journal.operations {
                if published.contains(&entry.path) {
                    entry.state = OperationState::RolledBack;
                }
            }
            journal.status = JournalStatus::RolledBack;
            // The publish failure is the error worth reporting; a journal
            // that cannot be rewritten must not hide it.
            let _ = write_manifest(&journal_dir, &mut journal);
            bail!(
                "the repair failed while publishing {}: {error:#} (the published files were restored: {})",
                operation.path,
                if restored.is_empty() {
                    "nothing to restore".to_string()
                } else {
                    restored.join(", ")
                }
            );
        }
        published.push(operation.path.clone());
        if let Some(entry) = journal
            .operations
            .iter_mut()
            .find(|entry| entry.path == operation.path)
        {
            entry.state = OperationState::Published;
        }
        write_manifest(&journal_dir, &mut journal)?;
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
    let journal_dir = repair_journal_dir(forgeplan_dir, transaction_id);
    let manifest_path = journal_dir.join("manifest.json");
    let bytes = fs::read(&manifest_path)
        .with_context(|| format!("read the recovery journal {}", manifest_path.display()))?;
    let mut journal: RepairJournal = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse the recovery journal {}", manifest_path.display()))?;
    if !journal_digest_matches(&journal) {
        bail!(
            "the recovery journal {} does not match its digest — the journal was edited",
            manifest_path.display()
        );
    }

    let mut restored: Vec<String> = Vec::new();
    let mut conflicts: Vec<String> = Vec::new();
    // The two maps are read while the operations are written, so they are
    // taken out of the journal first.
    let planned_postimages = journal.planned_postimages.clone();
    let preimages = journal.preimages.clone();
    for operation in &mut journal.operations {
        if operation.state == OperationState::RolledBack {
            continue;
        }
        let Some(expected_post) = planned_postimages.get(&operation.path).cloned() else {
            conflicts.push(format!("{} (the journal names no post-image)", operation.path));
            operation.state = OperationState::Conflict;
            continue;
        };
        let Some(expected_pre) = preimages.get(&operation.path).cloned() else {
            conflicts.push(format!("{} (the journal names no preimage)", operation.path));
            operation.state = OperationState::Conflict;
            continue;
        };
        let absolute = resolve_document_path(forgeplan_dir, &operation.path)?;
        let current = match fs::read(&absolute) {
            Ok(bytes) => sha256_hex(&bytes),
            Err(error) => {
                conflicts.push(format!("{} (the file cannot be read: {error})", operation.path));
                operation.state = OperationState::Conflict;
                continue;
            }
        };
        if current == expected_post {
            let preimage = fs::read(preimage_file(&journal_dir, &operation.path)).with_context(|| {
                format!("read the preimage of {}", operation.path)
            })?;
            publish(&absolute, &preimage)?;
            operation.state = OperationState::RolledBack;
            restored.push(operation.path.clone());
        } else if current == expected_pre {
            // Already restored — nothing to do, and nothing to overwrite.
            operation.state = OperationState::RolledBack;
        } else {
            conflicts.push(format!(
                "{} (expected post-image {expected_post} or preimage {expected_pre}, found {current})",
                operation.path
            ));
            operation.state = OperationState::Conflict;
        }
    }

    if !conflicts.is_empty() {
        journal.status = JournalStatus::Conflict;
        write_manifest(&journal_dir, &mut journal)?;
        bail!(
            "the rollback refused to overwrite an unrelated edit: {}",
            conflicts.join("; ")
        );
    }

    journal.status = JournalStatus::RolledBack;
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
                preimage: content.into_bytes(),
                postimage: updated.into_bytes(),
            },
        );
    }

    // The references: the documents that hold a reference to a changed key.
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
        let absolute = resolve_document_path(forgeplan_dir, &reference.source_path)?;
        if !operations.contains_key(&reference.source_path) {
            let content = read_document(&absolute, &reference.source_path)?;
            operations.insert(
                reference.source_path.clone(),
                FileOperation {
                    path: reference.source_path.clone(),
                    absolute: absolute.clone(),
                    preimage: content.clone().into_bytes(),
                    postimage: content.into_bytes(),
                },
            );
        }
        let entry = operations
            .get_mut(&reference.source_path)
            .expect("the operation was just inserted");
        let current = String::from_utf8(entry.postimage.clone())
            .with_context(|| format!("read {}", reference.source_path))?;
        verify_reference_occurrence(&current, reference)?;
        let updated = match reference.field.as_str() {
            "links" => set_link_target(&current, reference.occurrence, &reference.to)
                .with_context(|| format!("rewrite a reference of {}", reference.source_path))?,
            "parent_epic" => set_parent_epic(&current, &reference.to)
                .with_context(|| format!("rewrite a reference of {}", reference.source_path))?,
            other => bail!("unsupported reference field {other:?}"),
        };
        entry.postimage = updated.into_bytes();
    }

    // Every occurrence that points at a changed key must be resolved by the
    // plan. An unresolved structural reference onto a changed key is
    // refused: the repair does not guess which document a relation meant.
    let new_key_by_path: BTreeMap<&str, &str> = plan
        .sources
        .iter()
        .map(|source| (source.relative_path.as_str(), source.new_key.as_str()))
        .collect();
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
                new_key_by_path
                    .get(candidate.relative_path.as_str())
                    .copied()
                    == Some(planned.to.as_str())
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

    Ok(operations.into_values().collect())
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
        if let Ok(meta) = fs::symlink_metadata(&current) {
            if meta.file_type().is_symlink() {
                bail!("refusing a document path that crosses a symlink: {relative_path}");
            }
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

/// Write the preimages of a transaction and fsync them.
fn write_preimages(journal_dir: &Path, operations: &[FileOperation]) -> Result<()> {
    let preimages_dir = journal_dir.join("preimages");
    fs::create_dir_all(&preimages_dir)
        .with_context(|| format!("create {}", preimages_dir.display()))?;
    for operation in operations {
        write_file_atomically(&preimage_file(journal_dir, &operation.path), &operation.preimage)?;
    }
    sync_dir(&preimages_dir)?;
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
fn restore_preimages(operations: &[FileOperation], published: &[String]) -> Vec<String> {
    let mut restored: Vec<String> = Vec::new();
    for operation in operations {
        if !published.contains(&operation.path) {
            continue;
        }
        if publish(&operation.absolute, &operation.preimage).is_ok() {
            restored.push(operation.path.clone());
        }
    }
    restored
}

/// Write bytes to a path atomically: a temporary file in the same
/// directory, fsynced, then renamed over the target.
fn write_file_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("create {}", parent.display()))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document");
    let temp = parent.join(format!(".{name}.repair-tmp"));
    {
        let mut file = fs::File::create(&temp)
            .with_context(|| format!("create {}", temp.display()))?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&temp, path).with_context(|| format!("write {}", path.display()))?;
    sync_dir(parent)?;
    Ok(())
}

/// Publish one post-image atomically.
///
/// The write goes to a temporary file in the same directory, is fsynced,
/// and is then renamed over the target. No `git mv`, no staging, no
/// in-place truncation: a reader either sees the old bytes or the new ones.
fn publish(absolute: &Path, bytes: &[u8]) -> Result<()> {
    let parent = absolute.parent().unwrap_or_else(|| Path::new("."));
    let name = absolute
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document");
    let temp = parent.join(format!(".{name}.repair-tmp"));
    {
        let mut file = fs::File::create(&temp)
            .with_context(|| format!("create {}", temp.display()))?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&temp, absolute).with_context(|| format!("publish {}", absolute.display()))?;
    sync_dir(parent)?;
    Ok(())
}

/// fsync a directory so that a rename inside it survives a crash.
///
/// A directory that cannot be opened for reading — a Windows directory, for
/// instance — is not an error: the file itself is already durable.
fn sync_dir(dir: &Path) -> Result<()> {
    match fs::File::open(dir) {
        Ok(handle) => {
            if let Err(error) = handle.sync_all() {
                tracing::warn!(
                    error = %error,
                    path = %dir.display(),
                    "could not fsync a directory — the file itself is durable"
                );
            }
            Ok(())
        }
        Err(error) => {
            tracing::warn!(
                error = %error,
                path = %dir.display(),
                "could not open a directory for fsync — the file itself is durable"
            );
            Ok(())
        }
    }
}

/// The Git identity of the workspace, when it sits inside a repository.
///
/// The identity is the commit `HEAD` points at and the path of the Git
/// common directory — the shared state of linked worktrees. A plan made
/// outside Git carries no identity, and the apply then refuses to run
/// against a workspace that has become a Git repository (or the other way
/// round).
pub fn detect_git_identity(forgeplan_dir: &Path) -> Result<Option<GitIdentity>> {
    let Some(head) = git_head(forgeplan_dir) else {
        return Ok(None);
    };
    let Some(common_dir) = crate::workspace::git::git_common_dir(forgeplan_dir) else {
        return Ok(None);
    };
    let common_dir = fs::canonicalize(&common_dir).unwrap_or(common_dir);
    Ok(Some(GitIdentity {
        head,
        common_dir: common_dir.to_string_lossy().to_string(),
    }))
}

/// The commit `HEAD` points at, when the workspace is inside a repository.
fn git_head(forgeplan_dir: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(forgeplan_dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// The lock the repair takes on the workspace.
///
/// It is the same `.forgeplan/.lock` the rest of the workspace uses, so a
/// repair never runs beside another writer.
fn acquire_repair_lock(forgeplan_dir: &Path) -> Result<WorkspaceLock> {
    const LOCK_TIMEOUT: Duration = Duration::from_secs(30);
    const MAX_DELAY: Duration = Duration::from_millis(1000);

    let path = lock_path(forgeplan_dir);
    if let Ok(meta) = fs::symlink_metadata(&path) {
        if meta.file_type().is_symlink() {
            bail!(
                "the workspace lock {} is a symlink — refusing to open it",
                path.display()
            );
        }
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .with_context(|| format!("open the workspace lock {}", path.display()))?;
    let start = std::time::Instant::now();
    let mut delay = Duration::from_millis(10);
    loop {
        match fs2::FileExt::try_lock_exclusive(&file) {
            Ok(()) => return Ok(WorkspaceLock::from_file(file)),
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
    fn journal_digest_covers_the_operation_state() {
        let mut journal = RepairJournal {
            transaction_id: "txn-1".to_string(),
            manifest_sha256: String::new(),
            preimages: BTreeMap::from([("prds/a.md".to_string(), "aa".to_string())]),
            planned_postimages: BTreeMap::from([("prds/a.md".to_string(), "bb".to_string())]),
            operations: vec![JournalOperation {
                path: "prds/a.md".to_string(),
                state: OperationState::Pending,
            }],
            status: JournalStatus::InProgress,
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
            field: "links".to_string(),
            from: "PRD-001".to_string(),
            target_path: "prds/PRD-001-b.md".to_string(),
        }]
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
            field: "links".to_string(),
            from: "PRD-001".to_string(),
            target_path: "prds/PRD-001-a.md".to_string(),
        }];
        let extra = vec![ReferenceDecision {
            source_path: "evidence/EVID-001.md".to_string(),
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
        assert_eq!(rollback.status, JournalStatus::RolledBack);
        for (path, bytes) in &before {
            assert_eq!(
                &fs::read(forgeplan.join(path)).expect("read"),
                bytes,
                "{path} was not restored"
            );
        }
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
}
