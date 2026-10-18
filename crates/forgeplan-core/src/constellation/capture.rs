//! Stable source capture — SPEC-008 "Формальный критерий stable capture".
//!
//! Capture is the only thing in this tree that touches the canonical
//! markdown. It reads, and it reads **twice**.
//!
//! # The criterion
//!
//! 1. Pass one enumerates every artifact path, reads **full bytes**, reads
//!    the config, computes content digests, and builds one immutable set of
//!    [`ParsedArtifact`] plus outgoing sets.
//! 2. Pass two re-enumerates and re-reads bytes and config. The path sets and
//!    **all** digests must match.
//! 3. mtime, size and HEAD are hints. They never participate in identity — a
//!    same-size, same-mtime edit is detected because the bytes differ.
//! 4. Any observable difference discards the attempt. At most
//!    [`CaptureLimits::max_attempts`] attempts inside
//!    [`CaptureLimits::budget`]; then `source_busy`. No old row is deleted on
//!    a busy source.
//! 5. Everything downstream — lexical, graph, pure score, vector eligibility
//!    — is computed from the **first confirmed immutable set**. Nothing
//!    re-reads the files afterwards.
//! 6. Unreadable or invalid source is a per-source diagnostic and incomplete
//!    coverage. It is never a confirmed delete and never a complete-negative.
//! 7. Duplicate canonical identity is `identity_collision`; conflicting rows
//!    are excluded and no automatic winner is chosen.
//!
//! # What this is not
//!
//! It is **not** a promise of an atomic snapshot of an arbitrary mutating
//! directory. External git and editors do not take `WorkspaceLock`, so two
//! agreeing passes do not prove every file existed simultaneously. What they
//! prove — and what `evaluated_at` means — is "these complete bytes matched
//! the control pass". A change after the final check belongs to the next
//! barrier.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::artifact::frontmatter::{self, Frontmatter};
use crate::artifact::types::ArtifactKind;
use crate::workspace::ARTIFACT_DIRS;

use super::durability::platform_barrier;
use super::jobs::{
    DEFAULT_BODY_PREFIX_CHARS, SemanticState, embedding_input_digest, embedding_input_text,
    section_input_digest, section_input_text, section_key,
};
use super::manifest::{GenerationManifest, TableVersions};
use super::owner::{IndexIdentity, IndexLocation};
use super::{
    ConstellationError, DiagnosticCode, DigestBuilder, SNAPSHOT_DOMAIN, SOURCE_SET_DOMAIN,
    SourceDiagnostic, checked_increment, rfc3339_now, sha256_hex,
};

/// Default attempt count before a moving source is reported as `source_busy`.
pub const DEFAULT_CAPTURE_ATTEMPTS: u32 = 2;

/// Default total barrier budget in milliseconds.
pub const DEFAULT_CAPTURE_BUDGET_MS: u64 = 5000;

/// Bounded envelope for one capture: attempts and total barrier budget.
///
/// Both numbers are project parameters, not an SLA. The budget bounds how
/// long a reader may block on a moving source; the attempt count bounds how
/// often it may look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureLimits {
    pub max_attempts: u32,
    pub budget: Duration,
}

impl Default for CaptureLimits {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_CAPTURE_ATTEMPTS,
            budget: Duration::from_millis(DEFAULT_CAPTURE_BUDGET_MS),
        }
    }
}

/// Whether the index root's filesystem provides a usable durability barrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemSupport {
    /// A supported local filesystem with a real directory barrier.
    Supported,
    /// No supported barrier — ADR-029 forbids acknowledging a write here.
    Unsupported,
}

/// One outgoing edge: `(target bytes, relation)`.
///
/// `target` keeps the **exact bytes** the file declared. Targets are not
/// lower-cased and are not required to resolve locally: a cross-workspace or
/// qualified reference stays an edge with a diagnostic. Lower-casing here
/// would merge `PRD-001` with `prd-001` and silently rewrite somebody's
/// reference.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OutgoingEdge {
    pub target: String,
    pub relation: String,
}

impl OutgoingEdge {
    pub fn new(target: impl Into<String>, relation: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            relation: relation.into(),
        }
    }
}

/// ADR-012 identity triple.
///
/// `slug` is canonical; `predicted_number` / `assigned_number` are the
/// display layer. `assigned_number` is write-once — capture never proposes
/// changing it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityTriple {
    pub slug: Option<String>,
    pub predicted_number: Option<u32>,
    pub assigned_number: Option<u32>,
}

/// Full typed projection of one canonical artifact file.
///
/// Carries every field the store's [`crate::db::store::ArtifactRecord`] has
/// **plus** the raw canonical bytes, the complete frontmatter map including
/// unknown keys, the identity triple, the exact outgoing set and the three
/// separately compared digests.
///
/// `body` and `raw_bytes` are `Arc`-shared: a job, a reader and a publisher
/// reference the same capture instead of each holding a copy of the corpus.
#[derive(Debug, Clone)]
pub struct ParsedArtifact {
    /// ADR-012 slug when valid, else the canonical legacy id, else the file
    /// stem (with a `missing_identity` diagnostic).
    pub canonical_key: String,
    /// Rendered reference form (`PRD-074`), kept separate from the key.
    pub display_id: String,
    pub kind: String,
    pub status: String,
    pub title: String,
    /// Exact captured body bytes, including the empty body.
    pub body: Arc<str>,
    pub depth: Option<String>,
    pub author: Option<String>,
    pub parent_epic: Option<String>,
    pub valid_until: Option<String>,
    /// Absent stays absent. Capture never substitutes `now` for a missing
    /// timestamp — that would make every scan look like a content change.
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub tags: Vec<String>,
    pub identity: IdentityTriple,
    /// Complete frontmatter map, unknown keys included.
    pub frontmatter: Frontmatter,
    pub relative_path: String,
    /// Sorted, de-duplicated exact outgoing set.
    pub outgoing: BTreeSet<OutgoingEdge>,
    pub raw_bytes: Arc<[u8]>,
    pub source_digest: String,
    pub input_digest: String,
    /// `None` until P3 reads a real local model snapshot. A guessed revision
    /// would let an unresolved profile publish vectors that later look ready.
    pub profile_id: Option<String>,
    pub semantic_state: SemanticState,
    pub incarnation: u64,
    pub row_revision: u64,
    pub diagnostics: Vec<SourceDiagnostic>,
}

impl ParsedArtifact {
    /// The captured input text, i.e. exactly what a tokenizer would receive.
    pub fn embedding_input(&self, body_prefix_chars: usize) -> String {
        embedding_input_text(&self.title, &self.body, body_prefix_chars)
    }
}

/// One deterministic section row derived from a captured artifact.
///
/// Sections exist so semantic coverage is not capped at the body prefix:
/// the artifact row keeps its `title + body-prefix` vector (one row per
/// canonical key — unchanged), and each markdown section additionally gets
/// its own row in `constellation_sections_v1`, keyed by
/// `"{parent_key}#s{ordinal}"`.
///
/// A section row borrows its parent's whole binding — `source_digest`,
/// `incarnation`, `row_revision`, checkout and epoch are read from the
/// parent's [`SourceEntry`] at publish time — and adds only what is its own:
/// the section key, ordinal, heading, level, and the digest of the exact
/// text the tokenizer sees (`title + heading + content prefix`). An empty
/// heading means the whole-body fallback section (`ordinal == 0`), which is
/// how a markdown file with no headings still gets exactly one section row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedSection {
    /// `{parent_key}#s{ordinal}` — positional and deterministic.
    pub canonical_key: String,
    /// Canonical key of the artifact this section was parsed from.
    pub parent_key: String,
    /// Position in `list_section_spans` order. Reordering sections changes which
    /// content a key binds; `input_digest` detects the change.
    pub ordinal: u32,
    /// Heading text, or `None` for the no-headings whole-body fallback.
    pub heading: Option<String>,
    /// Markdown heading level (1–6), `None` for the fallback section.
    pub level: Option<u8>,
    /// Exact section bytes as captured — the text between this heading and
    /// the next heading of **any** level (`list_section_spans` semantics, not
    /// `list_sections`' nested fold — a `###` gets its own row).
    /// `Arc`-shared into the artifact body like every other captured text.
    pub content: Arc<str>,
    /// SHA-256 of `section_input_text(parent title, heading, content)`.
    /// The section's own input digest — never the parent's.
    pub input_digest: String,
    /// Parent artifact's input digest, carried so the section row records
    /// which artifact-level input its parent was bound to at capture time.
    pub parent_input_digest: String,
}

/// Derive the deterministic section set for one captured artifact.
///
/// Derived from the **captured bytes** — `list_section_spans` parses
/// `artifact.body`, never the filesystem. Three cases:
///
/// * **No headings at all** — one whole-body section at ordinal 0 with no
///   heading. Every captured artifact always produces at least one section
///   row, so an empty `sections` vec can only mean an empty `artifacts` vec.
/// * **Preamble** — `list_section_spans` drops text before the first heading, so
///   a body that starts with prose gets one extra synthetic section for it
///   at ordinal 0 (heading `None`), with the heading sections shifted to
///   ordinals 1... Without this a long preamble would be the same
///   not-searchable tail the sections table exists to remove.
/// * **Headed sections** — one row each, in `list_section_spans` order. The
///   split is flat: every heading opens a new row, so a nested `###` is its
///   own unit rather than content inside its parent's — otherwise one long
///   `#` document would produce a single section whose tail hits the input
///   cap and stays invisible, the very gap this table exists to close.
///
/// Ordinal keys are positional on purpose: two sections that share a heading
/// are still distinct rows, and reordering sections changes which content a
/// key binds — `input_digest` detects that the way it does for any edit.
pub fn sections_for(artifact: &ParsedArtifact, body_prefix_chars: usize) -> Vec<CapturedSection> {
    let parsed = crate::artifact::sections::list_section_spans(&artifact.body);
    if parsed.is_empty() {
        return vec![CapturedSection {
            canonical_key: section_key(&artifact.canonical_key, 0),
            parent_key: artifact.canonical_key.clone(),
            ordinal: 0,
            heading: None,
            level: None,
            content: Arc::from(artifact.body.as_ref()),
            input_digest: section_input_digest(
                &artifact.title,
                None,
                &artifact.body,
                body_prefix_chars,
            ),
            parent_input_digest: artifact.input_digest.clone(),
        }];
    }

    // A preamble section is emitted only for real prose before the first
    // heading — whitespace or an empty run gets no row.
    let preamble = preamble_text(&artifact.body);
    let shift = u32::from(preamble.is_some());
    let mut sections = Vec::with_capacity(parsed.len() + shift as usize);
    if let Some(preamble) = preamble {
        let content: Arc<str> = Arc::from(preamble);
        sections.push(CapturedSection {
            canonical_key: section_key(&artifact.canonical_key, 0),
            parent_key: artifact.canonical_key.clone(),
            ordinal: 0,
            heading: None,
            level: None,
            input_digest: section_input_digest(&artifact.title, None, &content, body_prefix_chars),
            parent_input_digest: artifact.input_digest.clone(),
            content,
        });
    }
    for (ordinal, section) in parsed.iter().enumerate() {
        let ordinal = ordinal as u32 + shift;
        let heading = if section.heading.is_empty() {
            None
        } else {
            Some(section.heading.clone())
        };
        let content: Arc<str> = Arc::from(section.content.as_str());
        sections.push(CapturedSection {
            canonical_key: section_key(&artifact.canonical_key, ordinal),
            parent_key: artifact.canonical_key.clone(),
            ordinal,
            heading: heading.clone(),
            level: Some(section.level),
            input_digest: section_input_digest(
                &artifact.title,
                heading.as_deref(),
                &content,
                body_prefix_chars,
            ),
            parent_input_digest: artifact.input_digest.clone(),
            content,
        });
    }
    sections
}

/// The raw body bytes before the first markdown heading, if non-whitespace.
///
/// Byte offsets come from the same `pulldown-cmark` event stream
/// `list_section_spans` uses, so "before the first heading" means exactly what
/// the section parser calls a heading — fenced code blocks still do not
/// count. Returns `None` when there is no heading or the run is blank.
fn preamble_text(body: &str) -> Option<&str> {
    use pulldown_cmark::{Event, Options, Parser, Tag};

    for (event, range) in Parser::new_ext(body, Options::all()).into_offset_iter() {
        if let Event::Start(Tag::Heading { .. }) = event {
            return body
                .get(..range.start)
                .map(str::trim)
                .filter(|text| !text.is_empty());
        }
    }
    None
}

/// Per-artifact durable source record carried by a [`GenerationManifest`].
///
/// One entry per current artifact. The manifest never carries a second copy
/// of a body — `source_digest` plus the published table row are the record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEntry {
    pub canonical_key: String,
    pub relative_path: String,
    pub checkout_id: String,
    pub index_epoch: u64,
    pub incarnation: u64,
    pub row_revision: u64,
    pub source_digest: String,
    pub input_digest: Option<String>,
    pub profile_id: Option<String>,
    pub semantic_state: SemanticState,
    pub diagnostic_codes: Vec<String>,
    pub observed_at: String,
}

/// A path that was enumerated but could not become a current row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnusableSource {
    pub relative_path: String,
    pub canonical_key: Option<String>,
    pub code: DiagnosticCode,
    pub detail: String,
}

/// Coverage of one capture, by outcome.
///
/// `expected` counts every enumerated artifact path; the other four partition
/// it, so `read + invalid + unavailable + excluded == expected`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    pub expected: usize,
    pub read: usize,
    pub invalid: usize,
    pub unavailable: usize,
    pub excluded: usize,
}

impl Coverage {
    /// Whether this capture may be treated as complete coverage.
    ///
    /// A complete-negative result is only meaningful when this is true. An
    /// empty result under incomplete coverage means "we could not look", and
    /// reporting it as "nothing matched" is the silent-failure class this
    /// whole protocol exists to remove.
    pub fn is_complete(&self) -> bool {
        self.invalid == 0 && self.unavailable == 0 && self.excluded == 0
    }
}

/// One immutable, confirmed capture of the canonical source.
#[derive(Debug, Clone)]
pub struct CapturedSourceView {
    /// The `.forgeplan` directory that was read.
    pub workspace: PathBuf,
    pub checkout_id: String,
    pub index_epoch: u64,
    /// Identity of this capture. Two captures of identical bytes at
    /// different instants get different ids.
    pub snapshot_id: String,
    pub evaluated_at: String,
    /// SHA-256 over the versioned domain, the config digest, and every
    /// length-prefixed sorted `(relativePath, sourceDigest)` pair.
    pub source_set_digest: String,
    pub config_digest: String,
    /// Parsed artifacts, sorted by canonical key, sharing their bytes by
    /// `Arc`.
    pub artifacts: Vec<Arc<ParsedArtifact>>,
    /// Deterministic section rows derived from `artifacts` — parent key
    /// sorted, then ordinal. One row per section per captured artifact,
    /// always at least one per artifact; this is the sections table's
    /// projection of the same confirmed bytes.
    pub sections: Vec<CapturedSection>,
    /// Durable source records, sorted by relative path.
    pub entries: Vec<SourceEntry>,
    /// Enumerated paths that produced no current row.
    pub unusable: Vec<UnusableSource>,
    /// Every path the two passes saw, sorted. This is what tells "absent"
    /// from "could not read" — the difference between a delete and a
    /// diagnostic.
    pub enumerated_paths: Vec<String>,
    pub coverage: Coverage,
    pub diagnostics: Vec<SourceDiagnostic>,
    /// Whether the two passes agreed. `false` would mean the caller received
    /// a stale view that must not be treated as current.
    pub confirmed: bool,
    pub filesystem_support: FilesystemSupport,
    pub attempts: u32,
}

impl CapturedSourceView {
    /// Look up one parsed artifact by canonical key.
    pub fn artifact(&self, canonical_key: &str) -> Option<&ParsedArtifact> {
        // `artifacts` holds `Arc<ParsedArtifact>`; the public accessor hands
        // out a plain reference rather than leaking the `Arc`.
        self.artifacts
            .iter()
            .find(|a| a.canonical_key == canonical_key)
            .map(|artifact| artifact.as_ref())
    }

    /// Look up one captured section by its deterministic key
    /// (`"{parent}#s{ordinal}"`).
    pub fn section(&self, canonical_key: &str) -> Option<&CapturedSection> {
        self.sections
            .iter()
            .find(|s| s.canonical_key == canonical_key)
    }

    /// The captured sections of one artifact, in ordinal order.
    pub fn sections_of(&self, parent_key: &str) -> impl Iterator<Item = &CapturedSection> {
        self.sections
            .iter()
            .filter(move |s| s.parent_key == parent_key)
    }

    /// The captured input text a tokenizer would receive for one section —
    /// `title + heading + content prefix`, the same string its
    /// `input_digest` was hashed over.
    pub fn section_input(&self, section: &CapturedSection, body_prefix_chars: usize) -> String {
        let title = self
            .artifact(&section.parent_key)
            .map(|a| a.title.as_str())
            .unwrap_or_default();
        section_input_text(
            title,
            section.heading.as_deref(),
            &section.content,
            body_prefix_chars,
        )
    }

    /// Whether an enumerated path failed to become a current row.
    pub fn is_unusable(&self, relative_path: &str) -> bool {
        self.unusable
            .iter()
            .any(|u| u.relative_path == relative_path)
    }

    /// Whether this capture is a complete observation of the corpus.
    pub fn is_complete(&self) -> bool {
        self.coverage.is_complete()
    }

    /// Build the generation manifest this capture would publish, given the
    /// table versions its commits produced.
    ///
    /// Deliberately a *draft*: the descriptor still has to be built and the
    /// publication acknowledged. Nothing here is on disk yet.
    pub fn to_generation_draft(
        &self,
        generation_id: &str,
        table_versions: TableVersions,
    ) -> GenerationManifest {
        GenerationManifest::from_capture(self, generation_id, table_versions)
    }
}

/// Request for one capture.
#[derive(Debug, Clone)]
pub struct CaptureRequest {
    /// The `.forgeplan` directory of the workspace to read.
    pub workspace: PathBuf,
    /// Previous confirmed capture, used to derive `incarnation` and
    /// `row_revision`. Without it every artifact is observed for the first
    /// time.
    pub previous: Option<Arc<CapturedSourceView>>,
}

impl CaptureRequest {
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        Self {
            workspace: workspace.into(),
            previous: None,
        }
    }

    /// Attach the previous confirmed capture as the incarnation baseline.
    pub fn with_previous(mut self, previous: Arc<CapturedSourceView>) -> Self {
        self.previous = Some(previous);
        self
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse one artifact file into its full typed projection.
///
/// The bytes have already been read; this never touches the filesystem, which
/// is what lets the control pass skip parsing entirely.
pub fn parse_artifact_file(
    relative_path: &str,
    raw_bytes: Arc<[u8]>,
    body_prefix_chars: usize,
) -> Result<ParsedArtifact, SourceDiagnostic> {
    let text = std::str::from_utf8(&raw_bytes).map_err(|e| {
        SourceDiagnostic::for_source(
            DiagnosticCode::UnreadableSource,
            relative_path,
            format!("canonical bytes are not valid UTF-8: {e}"),
        )
    })?;

    let (fm, body) = frontmatter::parse_frontmatter(text).map_err(|e| {
        SourceDiagnostic::for_source(
            DiagnosticCode::UnreadableSource,
            relative_path,
            format!("frontmatter did not parse: {e}"),
        )
    })?;

    let mut diagnostics = Vec::new();
    let source_digest = sha256_hex(&raw_bytes);

    let kind_raw = string_field(&fm, "kind").unwrap_or_default();
    if kind_raw.is_empty() {
        diagnostics.push(SourceDiagnostic::for_source(
            DiagnosticCode::UnreadableSource,
            relative_path,
            "frontmatter has no `kind`",
        ));
    } else if kind_raw.parse::<ArtifactKind>().is_err() {
        diagnostics.push(SourceDiagnostic::for_source(
            DiagnosticCode::UnreadableSource,
            relative_path,
            format!("`kind: {kind_raw}` is not an artifact kind"),
        ));
    }

    // ADR-012: the slug is canonical when it validates, otherwise the legacy
    // id. Never guess a key from the title — two artifacts can share one.
    let identity = IdentityTriple {
        slug: frontmatter::slug_from_frontmatter(&fm)
            .filter(|s| crate::artifact::types::validate_slug(s).is_ok())
            .map(str::to_string),
        predicted_number: frontmatter::predicted_number_from_frontmatter(&fm),
        assigned_number: frontmatter::assigned_number_from_frontmatter(&fm),
    };
    let declared_id = string_field(&fm, "id");
    let canonical_key = match (&identity.slug, declared_id.as_ref()) {
        (Some(slug), _) => slug.clone(),
        (None, Some(id)) => id.clone(),
        (None, None) => {
            let stem = relative_path
                .rsplit('/')
                .next()
                .and_then(|f| f.strip_suffix(".md"))
                .unwrap_or(relative_path)
                .to_string();
            diagnostics.push(SourceDiagnostic::for_source(
                DiagnosticCode::MissingIdentity,
                relative_path,
                "frontmatter declares neither `slug` nor `id`; canonical key taken from the filename",
            ));
            stem
        }
    };
    let display_id = declared_id.unwrap_or_else(|| canonical_key.clone());

    let title = string_field(&fm, "title").unwrap_or_default();
    let status = string_field(&fm, "status").unwrap_or_default();
    let depth = string_field(&fm, "depth");
    let author = string_field(&fm, "author");
    let parent_epic = string_field(&fm, "parent_epic");
    let valid_until = string_field(&fm, "valid_until");
    let created_at = string_field(&fm, "created_at");
    let updated_at = string_field(&fm, "updated_at");

    let mut tags = frontmatter::tags_from_frontmatter(&fm);
    tags.sort();
    tags.dedup();

    let (outgoing, link_diagnostics) = extract_outgoing(&fm, relative_path);
    diagnostics.extend(link_diagnostics);

    let input_digest = embedding_input_digest(&title, &body, body_prefix_chars);
    let body: Arc<str> = Arc::from(body);

    Ok(ParsedArtifact {
        canonical_key,
        display_id,
        kind: kind_raw,
        status,
        title,
        body,
        depth,
        author,
        parent_epic,
        valid_until,
        created_at,
        updated_at,
        tags,
        identity,
        frontmatter: fm,
        relative_path: relative_path.to_string(),
        outgoing,
        raw_bytes,
        source_digest,
        input_digest,
        profile_id: None,
        semantic_state: SemanticState::Unavailable,
        incarnation: 0,
        row_revision: 0,
        diagnostics,
    })
}

/// Read a scalar frontmatter field. An explicit YAML `null` and an absent
/// key are both `None` — neither becomes an empty string, and neither is
/// back-filled with a current timestamp.
fn string_field(fm: &Frontmatter, key: &str) -> Option<String> {
    fm.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

/// Extract the exact outgoing set from a `links:` block.
///
/// A malformed entry produces a diagnostic and is **not** silently dropped:
/// the edges around it are kept and the source is reported, so a lost edge is
/// never dressed up as "this artifact has no relations".
fn extract_outgoing(
    fm: &Frontmatter,
    relative_path: &str,
) -> (BTreeSet<OutgoingEdge>, Vec<SourceDiagnostic>) {
    let mut edges = BTreeSet::new();
    let mut diagnostics = Vec::new();

    let Some(value) = fm.get("links") else {
        return (edges, diagnostics);
    };
    let serde_yaml::Value::Sequence(seq) = value else {
        diagnostics.push(SourceDiagnostic::for_source(
            DiagnosticCode::InvalidLinks,
            relative_path,
            "frontmatter `links:` is present but is not a sequence",
        ));
        return (edges, diagnostics);
    };

    for (index, item) in seq.iter().enumerate() {
        let serde_yaml::Value::Mapping(map) = item else {
            diagnostics.push(SourceDiagnostic::for_source(
                DiagnosticCode::InvalidLinks,
                relative_path,
                format!("links[{index}] is not a mapping"),
            ));
            continue;
        };
        let target = map
            .get(serde_yaml::Value::String("target".to_string()))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let relation = map
            .get(serde_yaml::Value::String("relation".to_string()))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        match (target, relation) {
            (Some(t), Some(r)) => {
                if !edges.insert(OutgoingEdge::new(t, r)) {
                    diagnostics.push(SourceDiagnostic::for_source(
                        DiagnosticCode::InvalidLinks,
                        relative_path,
                        format!("links[{index}] duplicates the edge {t} --{r}-->"),
                    ));
                }
            }
            (target, _relation) => {
                let missing = if target.is_none() {
                    "target"
                } else {
                    "relation"
                };
                diagnostics.push(SourceDiagnostic::for_source(
                    DiagnosticCode::InvalidLinks,
                    relative_path,
                    format!("links[{index}] is missing `{missing}`"),
                ));
            }
        }
    }

    (edges, diagnostics)
}

// ---------------------------------------------------------------------------
// Capture
// ---------------------------------------------------------------------------

/// One file's observation within a single pass.
struct PassFile {
    relative_path: String,
    source_digest: Option<String>,
    parsed: Option<ParsedArtifact>,
    diagnostic: Option<SourceDiagnostic>,
}

/// The result of one full enumerate + bytes + config pass.
struct Pass {
    files: Vec<PassFile>,
    unusable: Vec<UnusableSource>,
    config_digest: String,
    /// The `embedding.chunk_size` the config resolved to. Sections hash
    /// their input under exactly this bound, so it travels with the pass
    /// rather than being re-derived where it was never read.
    body_prefix_chars: usize,
}

impl Pass {
    /// Whether this pass observed exactly the same source set as `other`.
    ///
    /// Path sets and **all** digests must agree, plus the config. Parsed
    /// content is deliberately not compared: identical bytes parse
    /// identically, so a second parse would be pure cost.
    fn agrees_with(&self, other: &Pass) -> bool {
        if self.config_digest != other.config_digest || self.files.len() != other.files.len() {
            return false;
        }
        self.files.iter().zip(&other.files).all(|(a, b)| {
            a.relative_path == b.relative_path
                && a.source_digest == b.source_digest
                && a.diagnostic.as_ref().map(|d| d.code) == b.diagnostic.as_ref().map(|d| d.code)
        })
    }
}

/// Minimal config probe — capture needs `embedding.chunk_size` and nothing
/// else, and must not fail on the rest of the config surface.
#[derive(Debug, Default, Deserialize)]
struct ConfigProbe {
    #[serde(default)]
    embedding: EmbeddingProbe,
}

#[derive(Debug, Default, Deserialize)]
struct EmbeddingProbe {
    #[serde(default)]
    chunk_size: Option<usize>,
}

/// Capture the canonical source under the stable-capture criterion.
///
/// # Errors
///
/// [`ConstellationError::SourceBusy`] when the source is still moving after
/// the attempt and budget limits. That is not a defect of the workspace: the
/// old view simply is not current, and no row is deleted because of it.
///
/// [`ConstellationError::Io`] when the config or an artifact directory could
/// not be read at all.
pub async fn capture_source(
    request: &CaptureRequest,
    limits: CaptureLimits,
) -> Result<CapturedSourceView, ConstellationError> {
    let started = Instant::now();
    let workspace = request.workspace.as_path();

    for attempt in 1..=limits.max_attempts {
        let first = run_pass(workspace, true).await?;
        if started.elapsed() >= limits.budget {
            return Err(busy(attempt, started));
        }
        // Control pass: re-enumerate, re-read bytes and config. Bytes are
        // re-parsed for diagnostics, but the parsed artifact is not kept.
        let second = run_pass(workspace, false).await?;
        if first.agrees_with(&second) {
            return build_view(workspace, request, first, attempt);
        }
        if started.elapsed() >= limits.budget {
            return Err(busy(attempt, started));
        }
    }

    Err(busy(limits.max_attempts, started))
}

fn busy(attempts: u32, started: Instant) -> ConstellationError {
    ConstellationError::SourceBusy {
        attempts,
        elapsed_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
    }
}

/// One full enumerate / read-bytes / read-config pass.
///
/// `parse` is false for the control pass: it re-reads bytes and digests and
/// still parses for diagnostics, but drops the parsed artifact.
async fn run_pass(workspace: &Path, parse: bool) -> Result<Pass, ConstellationError> {
    // Confinement starts at the root: a symlinked workspace would carry every
    // read outside it. Read paths take no `WorkspaceLock`, so this
    // `symlink_metadata` is the only boundary check on that route.
    let root_meta =
        tokio::fs::symlink_metadata(workspace)
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!(
                    "cannot stat the workspace root {}: {e}",
                    workspace.display()
                ),
            })?;
    if root_meta.file_type().is_symlink() {
        return Err(ConstellationError::Io {
            detail: format!(
                "the workspace root {} is a symlink; capture refuses to follow an alias \
                 out of the source boundary",
                workspace.display()
            ),
        });
    }

    let (config_digest, body_prefix_chars) = read_config(workspace).await?;

    let mut files: Vec<PassFile> = Vec::new();
    for dir_name in ARTIFACT_DIRS {
        let dir = workspace.join(dir_name);
        // A symlinked artifact directory is the same escape one level down:
        // `read_dir` would follow it and every `.md` outside the root would
        // be read, hashed and indexed as if it were inside. Refuse to
        // descend; the directory reports as an unreadable source, which
        // marks coverage partial and suppresses deletes — never a silent
        // skip, because skipping it would read as "those files are gone".
        match tokio::fs::symlink_metadata(&dir).await {
            Ok(meta) if meta.file_type().is_symlink() => {
                files.push(PassFile {
                    source_digest: None,
                    parsed: None,
                    diagnostic: Some(SourceDiagnostic::for_source(
                        DiagnosticCode::UnreadableSource,
                        dir_name.to_string(),
                        "artifact directory is a symlink; aliases are refused at the filesystem boundary",
                    )),
                    relative_path: dir_name.to_string(),
                });
                continue;
            }
            Ok(_) => {}
            // A missing artifact directory is normal on a partial checkout.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(ConstellationError::Io {
                    detail: format!("cannot stat {}: {e}", dir.display()),
                });
            }
        }
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(entries) => entries,
            // A missing artifact directory is normal on a partial checkout.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(ConstellationError::Io {
                    detail: format!("cannot enumerate {}: {e}", dir.display()),
                });
            }
        };

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("cannot enumerate {}: {e}", dir.display()),
            })?
        {
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str() else {
                continue;
            };
            if !name.ends_with(".md") {
                continue;
            }
            // Boundary check at enumeration: a name that could escape the
            // directory is never opened, canonicalize or not.
            if name.contains('/') || name.contains('\\') || name == "." || name == ".." {
                let relative_path = format!("{dir_name}/{name}");
                files.push(PassFile {
                    source_digest: None,
                    parsed: None,
                    diagnostic: Some(SourceDiagnostic::for_source(
                        DiagnosticCode::UnreadableSource,
                        relative_path.clone(),
                        "file name escapes the artifact directory",
                    )),
                    relative_path,
                });
                continue;
            }

            let relative_path = format!("{dir_name}/{name}");
            let file_type = entry
                .file_type()
                .await
                .map_err(|e| ConstellationError::Io {
                    detail: format!("cannot stat {}: {e}", entry.path().display()),
                })?;
            if file_type.is_symlink() {
                files.push(PassFile {
                    source_digest: None,
                    parsed: None,
                    diagnostic: Some(SourceDiagnostic::for_source(
                        DiagnosticCode::UnreadableSource,
                        relative_path.clone(),
                        "artifact file is a symlink; aliases are refused at the filesystem boundary",
                    )),
                    relative_path,
                });
                continue;
            }
            if !file_type.is_file() {
                continue;
            }

            let bytes = match tokio::fs::read(entry.path()).await {
                Ok(bytes) => bytes,
                Err(e) => {
                    files.push(PassFile {
                        source_digest: None,
                        parsed: None,
                        diagnostic: Some(SourceDiagnostic::for_source(
                            DiagnosticCode::UnreadableSource,
                            relative_path.clone(),
                            format!("cannot read canonical bytes: {e}"),
                        )),
                        relative_path,
                    });
                    continue;
                }
            };

            let source_digest = sha256_hex(&bytes);
            // The control pass still parses — for the diagnostic, not the
            // parsed artifact. An invalid file produces `UnreadableSource` in
            // the first pass; if the second pass skipped parsing it would
            // report `None` here, `agrees_with` would compare Some(_) to None,
            // and a perfectly stable broken file would exhaust the capture
            // budget as `source_busy`. Parse twice, keep the artifact once.
            let (parsed, diagnostic) = match parse_artifact_file(
                &relative_path,
                Arc::<[u8]>::from(bytes),
                body_prefix_chars,
            ) {
                Ok(parsed) => (if parse { Some(parsed) } else { None }, None),
                Err(diagnostic) => (None, Some(diagnostic)),
            };

            files.push(PassFile {
                relative_path,
                source_digest: Some(source_digest),
                parsed,
                diagnostic,
            });
        }
    }

    // Byte-wise sort of the relative paths — the set must be comparable, not
    // dependent on directory iteration order.
    files.sort_by(|a, b| a.relative_path.as_bytes().cmp(b.relative_path.as_bytes()));

    let mut unusable: Vec<UnusableSource> = files
        .iter()
        .filter_map(|f| {
            f.diagnostic.as_ref().map(|d| UnusableSource {
                relative_path: f.relative_path.clone(),
                canonical_key: f.parsed.as_ref().map(|p| p.canonical_key.clone()),
                code: d.code,
                detail: d.detail.clone(),
            })
        })
        .collect();
    unusable.sort_by(|a, b| a.relative_path.as_bytes().cmp(b.relative_path.as_bytes()));

    Ok(Pass {
        files,
        unusable,
        config_digest,
        body_prefix_chars,
    })
}

/// Read the workspace config and derive both the config digest and the body
/// prefix length that feeds `inputDigest`.
async fn read_config(workspace: &Path) -> Result<(String, usize), ConstellationError> {
    let config_path = workspace.join("config.yaml");
    let bytes = match tokio::fs::read(&config_path).await {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => {
            return Err(ConstellationError::Io {
                detail: format!("cannot read {}: {e}", config_path.display()),
            });
        }
    };

    let mut digest = DigestBuilder::new(SOURCE_SET_DOMAIN);
    digest
        .field(b"config")
        .field(&bytes)
        .field(ARTIFACT_DIRS.join("\u{1}").as_bytes());
    let config_digest = digest.finish_hex();

    let body_prefix_chars = if bytes.is_empty() {
        DEFAULT_BODY_PREFIX_CHARS
    } else {
        serde_yaml::from_slice::<ConfigProbe>(&bytes)
            .ok()
            .and_then(|p| p.embedding.chunk_size)
            .unwrap_or(DEFAULT_BODY_PREFIX_CHARS)
    };

    Ok((config_digest, body_prefix_chars))
}

/// Assemble the confirmed capture from the first pass.
fn build_view(
    workspace: &Path,
    request: &CaptureRequest,
    pass: Pass,
    attempts: u32,
) -> Result<CapturedSourceView, ConstellationError> {
    let evaluated_at = rfc3339_now();

    // Identity collisions: two paths claiming one canonical key. Every
    // conflicting row is excluded and no winner is picked — choosing one
    // would publish one file's bytes under another file's identity.
    let mut by_key: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for file in &pass.files {
        if let Some(parsed) = &file.parsed {
            by_key
                .entry(parsed.canonical_key.as_str())
                .or_default()
                .push(file.relative_path.clone());
        }
    }
    let collisions: BTreeMap<&str, Vec<String>> = by_key
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .collect();

    let previous: BTreeMap<&str, &SourceEntry> = match &request.previous {
        Some(prev) => prev
            .entries
            .iter()
            .map(|e| (e.canonical_key.as_str(), e))
            .collect(),
        None => BTreeMap::new(),
    };

    let mut diagnostics: Vec<SourceDiagnostic> = Vec::new();
    let mut unusable: Vec<UnusableSource> = pass.unusable.clone();
    let mut artifacts: Vec<Arc<ParsedArtifact>> = Vec::new();
    let mut entries: Vec<SourceEntry> = Vec::new();

    for file in &pass.files {
        let Some(mut parsed) = file.parsed.clone() else {
            continue;
        };

        if let Some(paths) = collisions.get(parsed.canonical_key.as_str()) {
            unusable.push(UnusableSource {
                relative_path: file.relative_path.clone(),
                canonical_key: Some(parsed.canonical_key.clone()),
                code: DiagnosticCode::IdentityCollision,
                detail: format!(
                    "canonical key `{}` is claimed by {}",
                    parsed.canonical_key,
                    paths.join(", ")
                ),
            });
            continue;
        }

        // Incarnation changes on a confirmed delete/recreate or a change of
        // path; rowRevision grows when the bytes change. Neither is invented
        // when there is no previous observation to compare against.
        match previous.get(parsed.canonical_key.as_str()) {
            None => {
                parsed.incarnation = 1;
                parsed.row_revision = 1;
            }
            Some(prev) => {
                if prev.relative_path == parsed.relative_path {
                    parsed.incarnation = prev.incarnation;
                    parsed.row_revision = if prev.source_digest == parsed.source_digest {
                        prev.row_revision
                    } else {
                        checked_increment("row_revision", prev.row_revision)?
                    };
                } else {
                    parsed.incarnation = checked_increment("incarnation", prev.incarnation)?;
                    parsed.row_revision = 1;
                }
            }
        }

        diagnostics.extend(parsed.diagnostics.iter().cloned());
        let diagnostic_codes: Vec<String> = parsed
            .diagnostics
            .iter()
            .map(|d| d.code.as_str().to_string())
            .collect();

        entries.push(SourceEntry {
            canonical_key: parsed.canonical_key.clone(),
            relative_path: parsed.relative_path.clone(),
            checkout_id: String::new(),
            index_epoch: 0,
            incarnation: parsed.incarnation,
            row_revision: parsed.row_revision,
            source_digest: parsed.source_digest.clone(),
            input_digest: Some(parsed.input_digest.clone()),
            profile_id: parsed.profile_id.clone(),
            semantic_state: parsed.semantic_state,
            diagnostic_codes,
            observed_at: evaluated_at.clone(),
        });
        artifacts.push(Arc::new(parsed));
    }

    artifacts.sort_by(|a, b| a.canonical_key.cmp(&b.canonical_key));
    // Sections derive from the confirmed immutable set, never from a second
    // read. Parent-key order is the artifacts order (already sorted); the
    // ordinal keeps each parent's rows in document order.
    let mut sections = Vec::new();
    for artifact in &artifacts {
        sections.extend(sections_for(artifact, pass.body_prefix_chars));
    }
    entries.sort_by(|a, b| a.relative_path.as_bytes().cmp(b.relative_path.as_bytes()));
    unusable.sort_by(|a, b| a.relative_path.as_bytes().cmp(b.relative_path.as_bytes()));
    diagnostics.extend(
        unusable.iter().map(|u| {
            SourceDiagnostic::for_source(u.code, u.relative_path.clone(), u.detail.clone())
        }),
    );

    let enumerated_paths: Vec<String> =
        pass.files.iter().map(|f| f.relative_path.clone()).collect();

    let coverage = Coverage {
        expected: enumerated_paths.len(),
        read: entries.len(),
        invalid: unusable
            .iter()
            .filter(|u| u.code == DiagnosticCode::UnreadableSource)
            .count(),
        unavailable: unusable
            .iter()
            .filter(|u| u.code == DiagnosticCode::MissingIdentity)
            .count(),
        excluded: unusable
            .iter()
            .filter(|u| u.code == DiagnosticCode::IdentityCollision)
            .count(),
    };

    let source_set_digest = compute_source_set_digest(&pass, &entries);

    let index_root = workspace.join("lance");
    let (filesystem_support, _write_barrier, fs_diagnostic) = platform_barrier(&index_root);
    if let Some(diagnostic) = fs_diagnostic {
        diagnostics.push(diagnostic);
    }

    let identity = IndexIdentity::derive(&IndexLocation::new(&index_root));
    for entry in &mut entries {
        entry.checkout_id = identity.checkout_id().to_string();
        entry.index_epoch = identity.index_epoch();
    }

    let mut snapshot = DigestBuilder::new(SNAPSHOT_DOMAIN);
    snapshot
        .field(source_set_digest.as_bytes())
        .field(evaluated_at.as_bytes())
        .field(identity.checkout_id().as_bytes());
    let snapshot_id = snapshot.finish_hex();

    Ok(CapturedSourceView {
        workspace: workspace.to_path_buf(),
        checkout_id: identity.checkout_id().to_string(),
        index_epoch: identity.index_epoch(),
        snapshot_id,
        evaluated_at,
        source_set_digest,
        config_digest: pass.config_digest,
        artifacts,
        sections,
        entries,
        unusable,
        enumerated_paths,
        coverage,
        diagnostics,
        confirmed: true,
        filesystem_support,
        attempts,
    })
}

/// SHA-256 over the versioned domain, the config digest, every
/// length-prefixed sorted `(relativePath, sourceDigest)` pair, and every
/// enumerated-but-unusable path.
///
/// The unusable paths are inside the digest on purpose: a file that became
/// unreadable must change the source set, otherwise two passes would "agree"
/// across a disappearance only one of them observed.
fn compute_source_set_digest(pass: &Pass, entries: &[SourceEntry]) -> String {
    let mut digest = DigestBuilder::new(SOURCE_SET_DOMAIN);
    digest
        .field(b"source-set")
        .field(pass.config_digest.as_bytes());

    for file in &pass.files {
        digest.field(b"path").field(file.relative_path.as_bytes());
        // Statement, not tail expression: `DigestBuilder::field` returns
        // `&mut Self`, and a `match` used as a value would demand `()`.
        match &file.source_digest {
            Some(source_digest) => {
                digest.field(source_digest.as_bytes());
            }
            None => {
                digest.field(b"");
            }
        }
    }
    for unusable in &pass.unusable {
        digest
            .field(b"unusable")
            .field(unusable.relative_path.as_bytes())
            .field(unusable.code.as_str().as_bytes());
    }
    for entry in entries {
        digest
            .field(b"key")
            .field(entry.canonical_key.as_bytes())
            .field(entry.source_digest.as_bytes());
    }
    digest.finish_hex()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const PRD: &str = "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\ndepth: standard\n---\n\nBody A.\n";

    fn workspace() -> TempDir {
        let tmp = TempDir::new().unwrap();
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        tmp
    }

    fn write_artifact(root: &Path, dir: &str, name: &str, content: &str) -> PathBuf {
        let dir = root.join(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    async fn capture(root: &Path) -> CapturedSourceView {
        capture_source(&CaptureRequest::new(root), CaptureLimits::default())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn capture_reports_full_bytes_and_exact_digests() {
        let tmp = workspace();
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", PRD);
        let view = capture(tmp.path()).await;

        assert!(view.confirmed);
        assert_eq!(view.coverage.expected, 1);
        assert_eq!(view.coverage.read, 1);
        assert_eq!(view.entries.len(), 1);
        assert_eq!(view.entries[0].source_digest, sha256_hex(PRD.as_bytes()));
        let artifact = view.artifact("PRD-001").expect("parsed artifact");
        assert_eq!(artifact.status, "draft");
        assert_eq!(&*artifact.body, "Body A.\n");
    }

    #[tokio::test]
    async fn same_mtime_and_size_different_bytes_is_detected_by_content_digest() {
        let tmp = workspace();
        let path = write_artifact(tmp.path(), "prds", "PRD-001-auth.md", PRD);
        let first = capture(tmp.path()).await;

        let original_mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
        let altered = PRD.replace("Body A.", "Body B.");
        assert_eq!(altered.len(), PRD.len(), "fixture must keep its length");
        std::fs::write(&path, &altered).unwrap();
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(original_mtime)
            .unwrap();

        let second = capture(tmp.path()).await;
        assert_ne!(
            first.source_set_digest, second.source_set_digest,
            "content digest, not mtime, is the identity"
        );
    }

    #[tokio::test]
    async fn unreadable_source_is_diagnosed_and_never_a_current_row() {
        let tmp = workspace();
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", PRD);
        write_artifact(
            tmp.path(),
            "prds",
            "PRD-002-broken.md",
            "---\nid: [unclosed\n",
        );
        let view = capture(tmp.path()).await;

        assert_eq!(view.coverage.expected, 2);
        assert_eq!(view.coverage.read, 1);
        assert_eq!(view.coverage.invalid, 1);
        assert!(!view.is_complete());
        assert!(view.is_unusable("prds/PRD-002-broken.md"));
        // The good row is untouched: an unreadable sibling is not a delete.
        assert!(view.artifact("PRD-001").is_some());
        assert!(
            view.diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::UnreadableSource
                    && d.source.as_deref() == Some("prds/PRD-002-broken.md"))
        );
    }

    #[tokio::test]
    async fn duplicate_identity_excludes_every_conflicting_row() {
        let tmp = workspace();
        write_artifact(
            tmp.path(),
            "prds",
            "PRD-001-a.md",
            &PRD.replace("Body A.", "From A."),
        );
        write_artifact(
            tmp.path(),
            "rfcs",
            "PRD-001-b.md",
            &PRD.replace("Body A.", "From B."),
        );
        let view = capture(tmp.path()).await;

        assert_eq!(view.coverage.expected, 2);
        assert_eq!(view.coverage.read, 0);
        assert_eq!(view.coverage.excluded, 2);
        assert!(
            view.artifacts.is_empty(),
            "no automatic winner may be chosen"
        );
    }

    #[tokio::test]
    async fn outgoing_set_is_exact_sorted_and_case_sensitive() {
        let tmp = workspace();
        let content = "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\nlinks:\n  - target: RFC-002\n    relation: informs\n  - target: prd-001\n    relation: refines\n  - target: RFC-002\n    relation: informs\n  - target: ADR-003\n---\n\nBody.\n";
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", content);
        let view = capture(tmp.path()).await;
        let artifact = view.artifact("PRD-001").unwrap();

        let edges: Vec<(&str, &str)> = artifact
            .outgoing
            .iter()
            .map(|e| (e.target.as_str(), e.relation.as_str()))
            .collect();
        assert_eq!(
            edges,
            vec![("RFC-002", "informs"), ("prd-001", "refines")],
            "the duplicate collapses, case is preserved verbatim, and the \
             relation-less edge is diagnosed instead of stored"
        );
        assert!(
            artifact
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::InvalidLinks),
            "a malformed edge must be diagnosed rather than silently dropped"
        );
    }

    #[tokio::test]
    async fn row_revision_grows_on_byte_change_and_incarnation_on_path_change() {
        let tmp = workspace();
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", PRD);
        let baseline = capture(tmp.path()).await;
        assert_eq!(baseline.entries[0].row_revision, 1);
        assert_eq!(baseline.entries[0].incarnation, 1);

        write_artifact(
            tmp.path(),
            "prds",
            "PRD-001-auth.md",
            &PRD.replace("Body A.", "Changed."),
        );
        let changed = capture_source(
            &CaptureRequest::new(tmp.path()).with_previous(Arc::new(baseline)),
            CaptureLimits::default(),
        )
        .await
        .unwrap();
        assert_eq!(changed.entries[0].row_revision, 2);
        assert_eq!(changed.entries[0].incarnation, 1);
    }

    #[tokio::test]
    async fn a_renamed_path_starts_a_new_incarnation() {
        let tmp = workspace();
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", PRD);
        let baseline = capture(tmp.path()).await;

        std::fs::remove_file(tmp.path().join("prds").join("PRD-001-auth.md")).unwrap();
        write_artifact(tmp.path(), "prds", "PRD-001-renamed.md", PRD);

        let after = capture_source(
            &CaptureRequest::new(tmp.path()).with_previous(Arc::new(baseline)),
            CaptureLimits::default(),
        )
        .await
        .unwrap();
        assert_eq!(after.entries[0].incarnation, 2);
        assert_eq!(after.entries[0].row_revision, 1);
    }

    #[tokio::test]
    async fn config_digest_changes_when_config_changes() {
        let tmp = workspace();
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", PRD);
        let before = capture(tmp.path()).await;

        std::fs::write(
            tmp.path().join("config.yaml"),
            "embedding:\n  chunk_size: 10\n",
        )
        .unwrap();
        let after = capture(tmp.path()).await;
        assert_ne!(before.config_digest, after.config_digest);
        assert_ne!(before.source_set_digest, after.source_set_digest);
    }

    #[tokio::test]
    async fn a_body_without_headings_gets_exactly_one_whole_body_section() {
        let tmp = workspace();
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", PRD);
        let view = capture(tmp.path()).await;

        let sections: Vec<_> = view.sections_of("PRD-001").collect();
        assert_eq!(sections.len(), 1, "every artifact owns at least one row");
        let section = sections[0];
        assert_eq!(section.canonical_key, "PRD-001#s0");
        assert_eq!(section.parent_key, "PRD-001");
        assert_eq!(section.ordinal, 0);
        assert_eq!(section.heading, None);
        assert_eq!(section.level, None);
        assert_eq!(&*section.content, "Body A.\n");
        // The digest covers exactly the string the tokenizer receives. For
        // the fallback section that string is byte-identical to the
        // artifact's own input — `title + body prefix` — so the digests
        // legitimately agree; what keeps the rows distinct is the key
        // (`PRD-001` vs `PRD-001#s0`), which the binding predicate carries.
        assert_eq!(
            section.input_digest,
            section_input_digest("Auth", None, &section.content, DEFAULT_BODY_PREFIX_CHARS)
        );
        assert_eq!(
            section.input_digest, section.parent_input_digest,
            "a heading-less body embeds the same string twice — same digest, \
             different row keys"
        );
    }

    #[tokio::test]
    async fn a_preamble_and_headings_map_to_positional_sections() {
        let tmp = workspace();
        let content = "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\ndepth: standard\n---\n\nIntro prose before any heading.\n\n## Problem\nLogin is slow.\n\n### Detail\nSubheading content stays inside Problem.\n\n## Goals\nMake it fast.\n";
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", content);
        let view = capture(tmp.path()).await;

        let sections: Vec<_> = view.sections_of("PRD-001").collect();
        let keys: Vec<&str> = sections.iter().map(|s| s.canonical_key.as_str()).collect();
        assert_eq!(
            keys,
            ["PRD-001#s0", "PRD-001#s1", "PRD-001#s2", "PRD-001#s3"]
        );

        let preamble = sections[0];
        assert_eq!(preamble.heading, None);
        assert_eq!(preamble.level, None);
        assert!(preamble.content.starts_with("Intro prose"));

        let problem = sections[1];
        assert_eq!(problem.heading.as_deref(), Some("Problem"));
        assert_eq!(problem.level, Some(2));
        // A nested heading is its own row, not part of the parent's content.
        assert!(!problem.content.contains("### Detail"));
        assert!(problem.content.contains("Login is slow."));

        let detail = sections[2];
        assert_eq!(detail.heading.as_deref(), Some("Detail"));
        assert_eq!(detail.level, Some(3));
        assert!(detail.content.contains("Subheading content"));

        let goals = sections[3];
        assert_eq!(goals.heading.as_deref(), Some("Goals"));
        assert!(goals.content.contains("Make it fast."));

        // Every section hash covers its own distinct input — `title +
        // heading + content` — and headed sections hash a string the
        // artifact input never produced.
        let mut digests = std::collections::BTreeSet::new();
        for section in &sections {
            assert_eq!(
                section.input_digest,
                section_input_digest(
                    "Auth",
                    section.heading.as_deref(),
                    &section.content,
                    DEFAULT_BODY_PREFIX_CHARS
                )
            );
            assert!(
                digests.insert(section.input_digest.clone()),
                "sections of one artifact must not share an input digest"
            );
        }
        assert!(
            sections
                .iter()
                .skip(1)
                .all(|s| s.input_digest != s.parent_input_digest),
            "a headed section's input is a different string than the artifact input"
        );
    }

    #[tokio::test]
    async fn a_heading_only_body_gets_no_preamble_row() {
        let tmp = workspace();
        let content = "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\ndepth: standard\n---\n\n## Problem\nLogin is slow.\n";
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", content);
        let view = capture(tmp.path()).await;

        let sections: Vec<_> = view.sections_of("PRD-001").collect();
        assert_eq!(
            sections.len(),
            1,
            "whitespace before the first heading is not a section"
        );
        assert_eq!(sections[0].canonical_key, "PRD-001#s0");
        assert_eq!(sections[0].heading.as_deref(), Some("Problem"));
    }

    #[tokio::test]
    async fn section_sets_are_identical_across_two_captures() {
        let tmp = workspace();
        let content = "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\ndepth: standard\n---\n\nIntro.\n\n## Problem\nSlow.\n\n## Goals\nFast.\n";
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", content);

        let first = capture(tmp.path()).await;
        let second = capture(tmp.path()).await;
        assert_eq!(
            first.sections, second.sections,
            "derivation is a pure function of the bytes"
        );
    }

    #[tokio::test]
    async fn content_past_the_body_prefix_is_reachable_through_a_section() {
        let tmp = workspace();
        // The tail marker sits past the artifact row's prefix: an artifact
        // vector alone could never match it. A section vector can, because
        // its input is the section's own prefix, not the body's.
        let tail = "THE_UNIQUE_TAIL_TOKEN xyzzy";
        let body = format!(
            "Intro.\n\n## Overview\n{}\n\n## Appendix\n{}\n",
            "a".repeat(DEFAULT_BODY_PREFIX_CHARS + 500),
            tail
        );
        let content = format!(
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\ndepth: standard\n---\n\n{body}"
        );
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", &content);
        let view = capture(tmp.path()).await;

        let artifact = view.artifact("PRD-001").unwrap();
        assert!(
            !artifact
                .embedding_input(DEFAULT_BODY_PREFIX_CHARS)
                .contains(tail),
            "the artifact input must actually clip the tail for this test to prove anything"
        );
        // s0 = "Intro." preamble, s1 = Overview, s2 = Appendix.
        let appendix = view
            .section("PRD-001#s2")
            .expect("the Appendix section exists");
        assert_eq!(appendix.heading.as_deref(), Some("Appendix"));
        assert!(
            view.section_input(appendix, DEFAULT_BODY_PREFIX_CHARS)
                .contains(tail),
            "the section input carries what the artifact input clipped"
        );
    }

    #[tokio::test]
    async fn an_invalid_artifact_contributes_no_section_rows() {
        let tmp = workspace();
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", PRD);
        write_artifact(
            tmp.path(),
            "prds",
            "PRD-002-broken.md",
            "---\nid: [unclosed\n\n## Hidden\nNever parsed.\n",
        );
        let view = capture(tmp.path()).await;

        assert!(!view.is_complete());
        assert!(
            view.sections.iter().all(|s| s.parent_key == "PRD-001"),
            "an artifact that was never parsed cannot mint section rows"
        );
    }
}
