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
use super::manifest::{GenerationManifest, TableVersions, Tombstone};
use super::owner::{IndexIdentity, IndexLocation};
use super::{
    ConstellationError, DiagnosticCode, DigestBuilder, SNAPSHOT_DOMAIN, SOURCE_SET_DOMAIN,
    SourceDiagnostic, checked_increment, rfc3339_now, sha256_hex,
};

// Interpreting nested legacy metadata changes edges even when source bytes
// and canonical keys do not change. Bind that interpretation to the capture.
const METADATA_PARSER_REVISION: &str = "legacy-projection-frontmatter-v1";

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
/// canonical key — unchanged), and the body additionally gets its own rows
/// in `constellation_sections_v2`, keyed by `"{parent_key}#s{ordinal}"`.
///
/// A section row borrows its parent's whole binding — `source_digest`,
/// `incarnation`, `row_revision`, checkout and epoch are read from the
/// parent's [`SourceEntry`] at publish time — and adds only what is its own:
/// the section key, ordinal, the UTF-8 byte range it covers, heading, level,
/// and the digest of the exact text the tokenizer sees. An empty heading means
/// the whole-body fallback section (`ordinal == 0`), which is how a markdown
/// file with no headings still gets exactly one section row.
///
/// The byte range is not a convenience: it is what makes "these windows
/// reproduce the body exactly" checkable rather than asserted, and it is what
/// the durable manifest records so a later pass can verify section membership
/// against the same immutable layout instead of against a sampling of hits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedSection {
    /// `{parent_key}#s{ordinal}` — positional and deterministic.
    pub canonical_key: String,
    /// Canonical key of the artifact this section was parsed from.
    pub parent_key: String,
    /// Position in `list_section_spans` order. Reordering sections changes which
    /// content a key binds; `input_digest` detects the change.
    pub ordinal: u32,
    /// Start of this window inside the **parent's body**, as a UTF-8 byte
    /// offset. Body-absolute, never relative to the section: combined with
    /// `byte_end` it names exactly the bytes `content` holds.
    pub byte_start: usize,
    /// End of this window, exclusive.
    pub byte_end: usize,
    /// Heading text, or `None` for the no-headings whole-body fallback.
    ///
    /// The **full** captured heading. It is what a reader displays and what
    /// the row records; only the copy that enters the embedding input is
    /// token-budgeted.
    pub heading: Option<String>,
    /// Markdown heading level (1–6), `None` for the fallback section.
    pub level: Option<u8>,
    /// Exact bytes of **one window** of this section, as captured.
    ///
    /// The unit a section is cut from is the raw run from this heading's `#` to
    /// the next heading of **any** level (`list_section_spans` semantics, not
    /// `list_sections`' nested fold — a `###` gets its own row), so the
    /// heading's own bytes are part of the section rather than sitting in front
    /// of it. A section longer than the body token budget is split across
    /// several rows, however, and so is a heading too long to fit one window:
    /// `content` holds a single window, not the whole section, and a reader
    /// that wants the section assembles the rows by `byte_start..byte_end`.
    /// `Arc`-shared into the artifact body like every other captured text.
    pub content: Arc<str>,
    /// SHA-256 of the exact bytes the shared builder handed to the tokenizer
    /// for this window. The section's own input digest — never the parent's.
    pub input_digest: String,
    /// Parent artifact's input digest, carried so the section row records
    /// which artifact-level input its parent was bound to at capture time.
    pub parent_input_digest: String,
}

/// Split one section's content into contiguous **token** windows.
///
/// Windows are cut at token boundaries and then extended so they tile the
/// content exactly: the end of one window is the start of the next, and the
/// last one runs to the end of the content. Every byte therefore lands in
/// exactly one window — including the whitespace and metaspaces between
/// tokens, which a "start here, stop at the token limit" reading would drop —
/// and no byte is embedded twice.
///
/// The previous implementation split by **characters** while the engine
/// truncated by **tokens**, so a 2000-character Russian, CJK or no-space
/// window routinely lost its tail at encode time while the capture believed
/// the whole document was covered. This is the one builder capture, digests,
/// admission and worker recovery all share, so a window's byte range and the
/// bytes actually handed to the tokenizer cannot drift apart.
fn section_windows(
    tokenizer: &crate::embed::tokenizer::MetadataTokenizer,
    budgets: &crate::embed::tokenizer::TokenBudgets,
    content: &str,
) -> Result<Vec<std::ops::Range<usize>>, super::ConstellationError> {
    tokenizer
        .token_windows(content, budgets)
        .map(|ranges| {
            ranges
                .into_iter()
                .map(|range| range.start..range.end)
                .collect::<Vec<_>>()
        })
        .map_err(|e| super::ConstellationError::Io {
            detail: format!("cannot measure section windows: {e}"),
        })
}

struct SectionInput<'a> {
    heading: Option<&'a str>,
    level: Option<u8>,
    content: &'a str,
    content_base: usize,
}

/// Emit one row per window of `content`, continuing the shared ordinal.
///
/// The heading and level repeat on every window so a hit deep inside a long
/// section still names the section it came from. The **display** heading is
/// the full captured text in every row: only the embedding input is
/// header-budgeted, and shortening the stored heading would change what a
/// reader sees rather than what a vector measured.
///
/// # `content_base`
///
/// `content_base` is where `content` begins inside `artifact.body`, and it is
/// what makes the recorded offsets **body-absolute** rather than relative to
/// whichever markdown unit produced the row. A range that only meant something
/// relative to its own unit could not be checked against the body at all, so
/// "the windows reproduce the body" would stay an assertion rather than
/// something [`body_is_fully_covered`] verifies on published data.
fn push_section_rows(
    out: &mut Vec<CapturedSection>,
    ordinal: &mut u32,
    artifact: &ParsedArtifact,
    SectionInput {
        heading,
        level,
        content,
        content_base,
    }: SectionInput<'_>,
    tokenizer: &crate::embed::tokenizer::MetadataTokenizer,
    budgets: &crate::embed::tokenizer::TokenBudgets,
) -> Result<(), super::ConstellationError> {
    for range in section_windows(tokenizer, budgets, content)? {
        // A range is a byte range into `content`; a tokenizer reporting a
        // non-boundary would make this `None`, and silently skipping the row
        // would leave part of the body uncovered.
        let window = content
            .get(range.clone())
            .ok_or_else(|| super::ConstellationError::Io {
                detail: format!(
                    "section window {}..{} is not a UTF-8 boundary of the captured content",
                    range.start, range.end
                ),
            })?;
        out.push(CapturedSection {
            canonical_key: section_key(&artifact.canonical_key, *ordinal),
            parent_key: artifact.canonical_key.clone(),
            ordinal: *ordinal,
            byte_start: content_base + range.start,
            byte_end: content_base + range.end,
            heading: heading.map(str::to_owned),
            level,
            content: Arc::from(window),
            // Mapped, not converted by a blanket `From`. `section_input_digest`
            // is `anyhow`-typed because the tokenizer library is, but the row
            // this lands in is durable: a capture that could not measure its
            // own input must name the section and the reason in a typed
            // `ConstellationError`, not lose it in an erased conversion.
            input_digest: section_input_digest(
                tokenizer,
                budgets,
                &artifact.title,
                heading,
                window,
            )
            .map_err(|error| super::ConstellationError::Io {
                detail: format!(
                    "cannot build the embedding input for section {} of `{}`: {error:#}",
                    section_key(&artifact.canonical_key, *ordinal),
                    artifact.canonical_key
                ),
            })?,
            parent_input_digest: artifact.input_digest.clone(),
        });
        *ordinal += 1;
    }
    Ok(())
}

/// The deterministic section set for one captured artifact, under `layout`.
///
/// Derived from the **captured bytes** — `list_section_spans` parses
/// `artifact.body`, never the filesystem — and measured by the one tokenizer
/// the whole capture shares.
///
/// Three cases:
///
/// * **No headings at all** — one whole-body section at ordinal 0 with no
///   heading.
/// * **Preamble** — the bytes before the first heading, published **raw** at
///   base `0` when they contain real prose. A whitespace-only prefix gets no
///   row of its own; it joins the first section, whose published range then
///   starts at `0` too.
/// * **Headed sections** — in `list_section_spans` order, flat: every heading
///   opens a new unit, **including its own heading bytes**. Each unit is then
///   windowed by **tokens**, so a section longer than the token budget becomes
///   several contiguous rows and none of its tail is invisible.
///
/// Every byte of the body belongs to exactly one unit, and every unit is
/// published at its body-absolute offset — the verifier
/// [`body_is_fully_covered`] requires that the rows tile the body with no gap
/// and no overlap, and a range derived from a copy of the text could not
/// satisfy it.
///
/// # Errors
///
/// [`super::ConstellationError::Io`] when the tokenizer cannot measure the
/// content or the budgets are unsatisfiable for this layout. That is a real
/// failure — a section set that cannot be measured must not be reported as an
/// empty one, which would read as "this document has no sections".
pub fn sections_for(
    artifact: &ParsedArtifact,
    tokenizer: &crate::embed::tokenizer::MetadataTokenizer,
    budgets: &crate::embed::tokenizer::TokenBudgets,
) -> Result<Vec<CapturedSection>, super::ConstellationError> {
    let parsed = crate::artifact::sections::list_section_spans(&artifact.body);
    let mut sections = Vec::new();
    let mut ordinal = 0u32;
    if parsed.is_empty() {
        push_section_rows(
            &mut sections,
            &mut ordinal,
            artifact,
            SectionInput {
                heading: None,
                level: None,
                content: &artifact.body,
                content_base: 0,
            },
            tokenizer,
            budgets,
        )?;
        return Ok(sections);
    }

    let first_heading_start = parsed.first().map(|section| section.raw_range.start);
    // A preamble section is emitted only for real prose before the first
    // heading. It is published **untrimmed** at base `0`, because the blank
    // lines in front of it are body bytes like any other and the windows must
    // tile the body from its very first byte.
    let has_preamble = preamble_span(&artifact.body, first_heading_start).is_some();
    if has_preamble {
        let start = first_heading_start.unwrap_or(0);
        push_section_rows(
            &mut sections,
            &mut ordinal,
            artifact,
            SectionInput {
                heading: None,
                level: None,
                content: &artifact.body[..start],
                content_base: 0,
            },
            tokenizer,
            budgets,
        )?;
    }

    for (index, section) in parsed.iter().enumerate() {
        let heading = (!section.heading.is_empty()).then_some(section.heading.as_str());
        // With no preamble row of its own, the leading blank run belongs to
        // the first section: its published start is `0`, not the offset of its
        // `#`, so nothing falls between the previous window and this one.
        let base = if index == 0 && !has_preamble {
            0
        } else {
            section.raw_range.start
        };
        push_section_rows(
            &mut sections,
            &mut ordinal,
            artifact,
            SectionInput {
                heading,
                level: Some(section.level),
                content: &artifact.body[base..section.raw_range.end],
                content_base: base,
            },
            tokenizer,
            budgets,
        )?;
    }
    Ok(sections)
}

/// Whether one artifact's section rows reproduce its body exactly.
///
/// Concatenating the rows' windows, in ordinal order, must give back the
/// whole body: every byte, including inter-window whitespace, metaspaces and a
/// long tail past any former character limit. This is the check ADR-029 §2A.3
/// makes load-bearing — a window scheme that merely *claims* whole-body
/// coverage while dropping a tail is the defect this slice exists to remove.
///
/// Rows are matched to their body by their recorded absolute range rather than
/// by re-parsing, so this verifies the ranges the manifest will publish rather
/// than a second, independent reconstruction.
///
/// Public because the claim it verifies is a claim about **published** rows:
/// the manifest carries the byte ranges, a later pass checks section membership
/// against them, and the section-window tests need to assert the same
/// property the publisher relies on rather than a re-implementation of it. An
/// in-crate-only verifier would leave the integration suite asserting its own
/// weaker version of the rule.
pub fn body_is_fully_covered(
    artifact: &ParsedArtifact,
    sections: &[CapturedSection],
) -> Result<(), super::ConstellationError> {
    let mut rows: Vec<&CapturedSection> = sections
        .iter()
        .filter(|row| row.parent_key == artifact.canonical_key)
        .collect();
    rows.sort_by_key(|row| row.ordinal);

    let mut cursor = 0usize;
    let mut rebuilt = String::with_capacity(artifact.body.len());
    for row in rows {
        if row.byte_start != cursor {
            return Err(super::ConstellationError::Io {
                detail: format!(
                    "section {} starts at body byte {} but the previous window ended at {cursor}; \
                     the windows do not tile the body",
                    row.canonical_key, row.byte_start
                ),
            });
        }
        let slice = artifact
            .body
            .get(row.byte_start..row.byte_end)
            .ok_or_else(|| super::ConstellationError::Io {
                detail: format!(
                    "section {} claims body bytes {}..{}, which are not a UTF-8 boundary of the \
                     captured body",
                    row.canonical_key, row.byte_start, row.byte_end
                ),
            })?;
        if slice != &*row.content {
            return Err(super::ConstellationError::Io {
                detail: format!(
                    "section {} stores text that is not the body slice it claims",
                    row.canonical_key
                ),
            });
        }
        rebuilt.push_str(slice);
        cursor = row.byte_end;
    }

    // `rebuilt` is a `String` and `artifact.body` an `Arc<str>`; there is no
    // `PartialEq<Arc<str>> for String`, so both sides are compared as `&str`.
    if cursor != artifact.body.len() || rebuilt.as_str() != artifact.body.as_ref() {
        return Err(super::ConstellationError::Io {
            detail: format!(
                "the section windows of `{}` cover {cursor} of the body's {} bytes and do not \
                 reproduce it exactly; a tail would be unsearchable",
                artifact.canonical_key,
                artifact.body.len()
            ),
        });
    }
    Ok(())
}

/// Every captured artifact's body is reproduced exactly by its section rows.
///
/// Run once over the whole set before the view is returned, so a tokenizer
/// reporting a bad offset fails the capture instead of publishing a generation
/// whose section membership silently drops content.
pub(crate) fn verify_bodies_are_fully_covered(
    artifacts: &[Arc<ParsedArtifact>],
    sections: &[CapturedSection],
) -> Result<(), super::ConstellationError> {
    for artifact in artifacts {
        body_is_fully_covered(artifact, sections)?;
    }
    Ok(())
}

/// The raw body bytes before the first markdown heading, if non-whitespace,
/// with the offset of their first real byte in the body.
///
/// The boundary comes from the caller — `list_section_spans` already knows
/// where the first heading starts, and reparsing the body here would mean a
/// second `pulldown-cmark` walk of every artifact on every capture. "Before
/// the first heading" therefore means exactly what the section parser calls a
/// heading: fenced code blocks still do not count.
///
/// The **trimmed** run is returned with the offset of its first real byte,
/// because that is the description a reader wants: where the preamble's prose
/// begins, not where the blank lines in front of it do. The section rows
/// publish the untrimmed prefix regardless — the blank lines are body bytes
/// too — so the two deliberately answer different questions.
///
/// Returns `None` when there is no heading or the whole prefix is blank.
fn preamble_span(body: &str, first_heading_start: Option<usize>) -> Option<(usize, &str)> {
    let prefix = body.get(..first_heading_start?)?;
    let leading = prefix.len() - prefix.trim_start().len();
    let trimmed = prefix.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some((leading, trimmed))
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
    /// The section layout this capture measured its windows under.
    ///
    /// Held for the capture's life alongside the tokenizer below, and never
    /// re-derived per artifact. Its `max_tokens == 0` form is the fingerprinted
    /// `semantic_unavailable` state, and in that state `sections` is exactly
    /// empty — never a character-window substitute under the previous layout's
    /// identity.
    pub layout: crate::embed::tokenizer::SectionLayout,
    /// The one tokenizer every window in this capture was measured with.
    ///
    /// `None` exactly when [`Self::layout`] is unavailable; the two are set
    /// together by construction, because a section set without a tokenizer
    /// could not have been produced in the first place.
    pub tokenizer: Option<Arc<crate::embed::tokenizer::MetadataTokenizer>>,
    /// Prefix budget read from the same confirmed configuration bytes.
    ///
    /// **Artifact rows only.** It never clips a section window: those are cut
    /// in tokens by the layout above. It stays a field because the artifact
    /// vector is still prefix-bounded and every consumer that builds one needs
    /// it.
    pub body_prefix_chars: usize,
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

    /// The token budgets this capture's windows obey.
    ///
    /// Resolved once and shared: capture, digests, admission and worker
    /// recovery all read the same numbers rather than each re-deriving them
    /// from the layout, which is how a window and the encoder could disagree
    /// about the limit.
    pub fn token_budgets(
        &self,
    ) -> Result<crate::embed::tokenizer::TokenBudgets, ConstellationError> {
        crate::embed::tokenizer::TokenBudgets::resolve(&self.layout).map_err(|e| {
            ConstellationError::Io {
                detail: format!(
                    "the section layout {} has no usable token budget: {e}",
                    self.layout.fingerprint()
                ),
            }
        })
    }

    /// The one tokenizer and the budgets together, or why neither exists.
    ///
    /// The single accessor every window and input call goes through, so no
    /// caller can accidentally build a section input against a different
    /// layout from the one that measured the window.
    pub fn section_layout(
        &self,
    ) -> Result<
        (
            &crate::embed::tokenizer::MetadataTokenizer,
            crate::embed::tokenizer::TokenBudgets,
        ),
        ConstellationError,
    > {
        let tokenizer = self.tokenizer.as_deref().ok_or_else(|| ConstellationError::Io {
            detail: format!(
                "this capture has no tokenizer; its layout is `{}` and it publishes no semantic \
                 sections",
                self.layout.fingerprint()
            ),
        })?;
        Ok((tokenizer, self.token_budgets()?))
    }

    /// The captured input text a tokenizer would receive for one section —
    /// exactly the string its `input_digest` was hashed over.
    ///
    /// **No `body_prefix_chars`.** The window arriving here was already cut in
    /// tokens by this capture's layout; clipping it again by characters would
    /// reintroduce the second, unrelated budget this contract removes, and the
    /// mismatch between a capture-time budget and a later re-read of the config
    /// is the shape of bug that loses a tail silently.
    pub fn section_input(&self, section: &CapturedSection) -> Result<String, ConstellationError> {
        let (tokenizer, budgets) = self.section_layout()?;
        let title = self
            .artifact(&section.parent_key)
            .map(|a| a.title.as_str())
            .unwrap_or_default();
        section_input_text(
            tokenizer,
            &budgets,
            title,
            section.heading.as_deref(),
            &section.content,
        )
        .map_err(|e| ConstellationError::Io {
            detail: format!(
                "cannot build the embedding input for section {}: {e}",
                section.canonical_key
            ),
        })
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

/// What a capture continues its durable identity from.
///
/// Two carriers, one meaning: the **identity records** of the previous
/// observation. Both spellings answer the same two questions — what did the
/// last row for this key look like, and was it retired by a confirmed delete?
///
/// The distinction is *who holds the record*, not what it means. A read-side
/// caller keeps the whole previous view in memory. Production has no such
/// view: the bytes are gone and only the published manifest survives, so its
/// `source_entries` and `tombstones` are the baseline. Reusing one derivation
/// for both is the point — two carriers must never become two identity
/// schemes.
#[derive(Debug, Clone)]
pub enum PreviousObservation {
    /// A previous confirmed capture, held whole.
    Capture(Arc<CapturedSourceView>),
    /// The generation manifest `CURRENT` selects.
    Manifest(Arc<GenerationManifest>),
}

impl PreviousObservation {
    /// The rows the previous observation published, by canonical key.
    pub fn entries(&self) -> &[SourceEntry] {
        match self {
            Self::Capture(view) => &view.entries,
            Self::Manifest(manifest) => &manifest.source_entries,
        }
    }

    /// The incarnations a confirmed delete retired, as `(canonical key,
    /// incarnation)` pairs.
    ///
    /// A capture view carries no tombstones of its own: absence is only ever
    /// *confirmed* by a reconcile that compared two published sets, and the
    /// manifest is where that verdict is durable. Empty for a view, by
    /// construction rather than by a default nobody set.
    pub fn retired(&self) -> impl Iterator<Item = (&str, u64)> {
        const NONE: &[Tombstone] = &[];
        let tombstones = match self {
            Self::Capture(_) => NONE,
            Self::Manifest(manifest) => manifest.tombstones.as_slice(),
        };
        tombstones
            .iter()
            .map(|tombstone| (tombstone.canonical_key.as_str(), tombstone.incarnation))
    }
}

/// Request for one capture.
#[derive(Debug, Clone)]
pub struct CaptureRequest {
    /// The `.forgeplan` directory of the workspace to read.
    pub workspace: PathBuf,
    /// Previous durable observation, used to derive `incarnation` and
    /// `row_revision`. Without it every artifact is observed for the first
    /// time.
    pub previous: Option<PreviousObservation>,
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
        self.previous = Some(PreviousObservation::Capture(previous));
        self
    }

    /// Attach the published manifest as the incarnation baseline.
    ///
    /// This is what production capture uses. Without it every production pass
    /// re-observed the corpus from nothing: `incarnation` and `row_revision`
    /// came back as `1` on every run, so a delete followed by a recreate with
    /// identical bytes reproduced the retired row's identity exactly and the
    /// splice preserved a vector bound to a document that had been deleted.
    pub fn with_published_manifest(mut self, manifest: Arc<GenerationManifest>) -> Self {
        self.previous = Some(PreviousObservation::Manifest(manifest));
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

    // ADR-030: the declared slug is the canonical key when it is one — a
    // minted key (SPEC-009) or a grandfathered slug (SPEC-005). Never guess
    // a key from the title — two artifacts can share one.
    let identity = IdentityTriple {
        slug: frontmatter::slug_from_frontmatter(&fm)
            .filter(|s| crate::artifact::types::is_canonical_key(s))
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
    // Old templates use `created` / `updated`, sometimes with date precision
    // only. Keep that exact source value; explicit current fields (even null)
    // win, and neither cache timestamps nor filesystem times fill gaps.
    let created_at = string_field(
        &fm,
        if fm.contains_key("created_at") {
            "created_at"
        } else {
            "created"
        },
    );
    let updated_at = string_field(
        &fm,
        if fm.contains_key("updated_at") {
            "updated_at"
        } else {
            "updated"
        },
    );

    let mut tags = frontmatter::tags_from_frontmatter(&fm);
    tags.sort();
    tags.dedup();

    // A `links:` block that is not a sequence cannot be read as an edge set.
    // The source is refused here, so it is excluded from the current set and
    // leaves the capture incomplete - never published as "no relations".
    let (outgoing, link_diagnostics) = extract_outgoing(&fm, relative_path)?;
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
///
/// A `links:` block that is not a sequence is a different failure. There is
/// no edge set to read at all, so there is nothing to keep: reporting an
/// empty outgoing set would publish the artifact as if it had no relations
/// and drop every relation the author wrote. That is an invalid source, not a
/// warning, and it is refused here so the caller reports it instead of
/// publishing a silent empty set.
fn extract_outgoing(
    fm: &Frontmatter,
    relative_path: &str,
) -> Result<(BTreeSet<OutgoingEdge>, Vec<SourceDiagnostic>), SourceDiagnostic> {
    let mut edges = BTreeSet::new();
    let mut diagnostics = Vec::new();

    let Some(value) = fm.get("links") else {
        return Ok((edges, diagnostics));
    };
    let serde_yaml::Value::Sequence(seq) = value else {
        return Err(SourceDiagnostic::for_source(
            DiagnosticCode::InvalidLinks,
            relative_path,
            "frontmatter `links:` is present but is not a sequence",
        ));
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

    Ok((edges, diagnostics))
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
    /// The `embedding.chunk_size` the config resolved to. **Artifact rows
    /// only** — section windows are cut in tokens by `layout_probe`, and a
    /// window re-clipped by this character budget is the double budget this
    /// contract removes.
    body_prefix_chars: usize,
    /// The resolved section layout, carried on the pass so the windows are
    /// measured against the layout the digest already names.
    layout_probe: SectionLayoutProbe,
}

impl Pass {
    /// Whether this pass observed exactly the same source set as `other`.
    ///
    /// Path sets and **all** digests must agree, plus the config and — now —
    /// the **section layout fingerprint**. The layout check is explicit rather
    /// than riding on `config_digest` alone so the reason a pass disagreed is
    /// visible here: a cache that gained or lost tokenizer metadata between
    /// the two passes is a different observation, not a stable one.
    ///
    /// Parsed content is deliberately not compared: identical bytes parse
    /// identically, so a second parse would be pure cost.
    fn agrees_with(&self, other: &Pass) -> bool {
        if self.config_digest != other.config_digest
            || self.files.len() != other.files.len()
            || self.layout_probe.layout.fingerprint() != other.layout_probe.layout.fingerprint()
            || self.body_prefix_chars != other.body_prefix_chars
        {
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

    let (config_digest, body_prefix_chars, layout_probe) = read_config(workspace).await?;

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
            // parsed artifact. An invalid file produces its diagnostic
            // (`UnreadableSource`, `InvalidLinks`) in the first pass; if the
            // second pass skipped parsing it would
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
        layout_probe,
    })
}

/// Read the workspace config and derive the config digest, the artifact-prefix
/// budget, and the section layout.
///
/// The digest binds raw config bytes, artifact directories, section layout,
/// and the compiled metadata parser revision. A tokenizer/layout change or a
/// newly recovered nested edge must not claim the old projection is current:
/// identical bytes can have a different interpretation after an upgrade.
///
/// The layout itself is resolved from the **process-wide model cache**, not
/// from the workspace, so both passes see the same answer as long as the cache
/// did not change underneath them — and if it did, the fingerprints differ and
/// the pass is discarded, which is the correct outcome rather than two passes
/// quietly agreeing on different layouts.
async fn read_config(
    workspace: &Path,
) -> Result<(String, usize, SectionLayoutProbe), ConstellationError> {
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

    let probe = SectionLayoutProbe::resolve();
    let layout_fingerprint = probe.layout.fingerprint();

    let mut digest = DigestBuilder::new(SOURCE_SET_DOMAIN);
    digest
        .field(b"config")
        .field(&bytes)
        .field(ARTIFACT_DIRS.join("\u{1}").as_bytes())
        .field(b"section_layout")
        .field(layout_fingerprint.as_bytes())
        .field(b"metadata_parser")
        .field(METADATA_PARSER_REVISION.as_bytes());
    let config_digest = digest.finish_hex();

    let body_prefix_chars = if bytes.is_empty() {
        DEFAULT_BODY_PREFIX_CHARS
    } else {
        serde_yaml::from_slice::<ConfigProbe>(&bytes)
            .ok()
            .and_then(|p| p.embedding.chunk_size)
            .unwrap_or(DEFAULT_BODY_PREFIX_CHARS)
    };

    Ok((config_digest, body_prefix_chars, probe))
}

/// The resolved section layout for one capture, kept as one value.
///
/// `tokenizer` is `None` exactly when the layout is unavailable. Holding both
/// in one struct is what stops a capture from recording a fingerprint it did
/// not measure under, or measuring under a layout it did not record.
#[derive(Debug, Clone)]
struct SectionLayoutProbe {
    layout: crate::embed::tokenizer::SectionLayout,
    tokenizer: Option<Arc<crate::embed::tokenizer::MetadataTokenizer>>,
}

impl SectionLayoutProbe {
    /// Resolve the layout from the local model cache. Never downloads.
    fn resolve() -> Self {
        let resolution =
            crate::embed::tokenizer::resolve_layout(crate::constellation::jobs::DEFAULT_MODEL_REPO);
        let layout = resolution.layout();
        let tokenizer = resolution.tokenizer().cloned();
        Self { layout, tokenizer }
    }

    /// The budgets, or why this layout has none.
    fn budgets(&self) -> Result<crate::embed::tokenizer::TokenBudgets, ConstellationError> {
        crate::embed::tokenizer::TokenBudgets::resolve(&self.layout).map_err(|e| {
            ConstellationError::Io {
                detail: format!(
                    "the section layout {} has no usable token budget: {e}",
                    self.layout.fingerprint()
                ),
            }
        })
    }
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
            .entries()
            .iter()
            .map(|e| (e.canonical_key.as_str(), e))
            .collect(),
        None => BTreeMap::new(),
    };

    // What a confirmed delete retired, by canonical key. Only present when a
    // published manifest is the baseline — a capture view cannot record a
    // confirmed absence, because absence is a verdict two published sets
    // produce, not one scan.
    let retired: BTreeMap<&str, u64> = match &request.previous {
        Some(prev) => prev.retired().collect(),
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
        //
        // A live previous entry is authoritative: it means the recreate has
        // already been published, carrying an incarnation that is above
        // whatever the tombstone retired. Honouring the tombstone then would
        // bump the identity again on *every* pass and nothing would ever
        // converge.
        //
        // The tombstone is consulted only when the key has no live entry —
        // which is exactly the state a confirmed delete leaves behind, and
        // exactly the state in which a byte-identical recreate at the same
        // path would otherwise reproduce the retired row's
        // `(incarnation, row_revision)`. `SemanticColumns::is_preserved_for`
        // compares those very fields, so reproducing them preserves the
        // deleted document's vector and satisfies its outstanding job. Skipping
        // past the retired incarnation is what makes the fence real.
        match previous.get(parsed.canonical_key.as_str()) {
            None => {
                parsed.incarnation = match retired.get(parsed.canonical_key.as_str()) {
                    Some(retired_incarnation) => {
                        checked_increment("incarnation", *retired_incarnation)?
                    }
                    None => 1,
                };
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
    //
    // With no usable metadata the set is **exactly empty**, and that emptiness
    // is the whole signal: the capture publishes zero semantic sections rather
    // than character windows carrying the previous layout's `#sN` keys. Old
    // vectors may remain as inert cache rows; nothing here makes them current.
    let mut sections = Vec::new();
    let tokenizer = pass.layout_probe.tokenizer.clone();
    if let Some(tokenizer) = tokenizer.as_deref() {
        let budgets = pass.layout_probe.budgets()?;
        for artifact in &artifacts {
            let mut rows = sections_for(artifact, tokenizer, &budgets)?;
            sections.append(&mut rows);
        }
        // The invariant that makes "the whole body is covered" a checked fact
        // rather than a claim about the windowing algorithm: every row's stored
        // text is exactly the body slice it names, and the ranges tile the
        // body with no gap.
        verify_bodies_are_fully_covered(&artifacts, &sections)?;
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
        // A file that could not be read at all and a file whose `links:`
        // block cannot be read as an edge set are the same outcome: the path
        // was seen, but it cannot become a current row. Both keep the capture
        // incomplete, so a read reports a named error instead of an empty
        // success, and a reconcile refuses to publish.
        invalid: unusable
            .iter()
            .filter(|u| {
                matches!(
                    u.code,
                    DiagnosticCode::UnreadableSource | DiagnosticCode::InvalidLinks
                )
            })
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
        layout: pass.layout_probe.layout.clone(),
        tokenizer,
        body_prefix_chars: pass.body_prefix_chars,
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

    #[tokio::test]
    async fn recovered_edges_do_not_reuse_a_byte_identical_pre_upgrade_projection() {
        let tmp = workspace();
        let source = "---\nid: NOTE-001\nkind: note\nstatus: draft\ntitle: Stable identity\nslug: note-stable\n---\n\n---\nlinks:\n  - target: NOTE-002\n    relation: informs\n---\n\nBody.\n";
        write_artifact(tmp.path(), "notes", "NOTE-001-stable.md", source);
        let current = capture_source(
            &CaptureRequest::new(tmp.path()),
            CaptureLimits {
                budget: Duration::from_secs(120),
                ..CaptureLimits::default()
            },
        )
        .await
        .unwrap();
        let artifact = current.artifact("note-stable").unwrap();
        assert_eq!(artifact.canonical_key, "note-stable");
        assert!(
            artifact
                .outgoing
                .contains(&OutgoingEdge::new("NOTE-002", "informs"))
        );
        assert_eq!(artifact.raw_bytes.as_ref(), source.as_bytes());

        // The old format did not bind the metadata interpreter. Its bytes,
        // paths and canonical key can all match while its edge table is old.
        let mut old_config = DigestBuilder::new(SOURCE_SET_DOMAIN);
        old_config
            .field(b"config")
            .field(b"")
            .field(ARTIFACT_DIRS.join("\u{1}").as_bytes())
            .field(b"section_layout")
            .field(current.layout.fingerprint().as_bytes());
        assert_ne!(current.config_digest, old_config.finish_hex());
    }

    #[test]
    fn old_projection_captures_the_original_identity_and_source_dates() {
        let content = "---\nid: NOTE-001\nkind: note\nstatus: draft\ntitle: Compatibility source\n---\n\n---\nassigned_number: 1\ncreated: 2026-10-24\npredicted_number: 1\nslug: note-compatibility-source\nupdated: 2026-10-24\n---\n\nKeep NOTE-002.\n";
        let raw: Arc<[u8]> = Arc::from(content.as_bytes());
        let parsed = parse_artifact_file("notes/NOTE-001-source.md", raw, 512).unwrap();

        assert_eq!(parsed.canonical_key, "note-compatibility-source");
        assert_eq!(parsed.display_id, "NOTE-001");
        assert_eq!(
            parsed.identity.slug.as_deref(),
            Some("note-compatibility-source")
        );
        assert_eq!(parsed.identity.assigned_number, Some(1));
        assert_eq!(parsed.identity.predicted_number, Some(1));
        assert_eq!(parsed.created_at.as_deref(), Some("2026-10-24"));
        assert_eq!(parsed.updated_at.as_deref(), Some("2026-10-24"));
        assert_eq!(parsed.raw_bytes.as_ref(), content.as_bytes());
        assert!(parsed.body.ends_with("Keep NOTE-002.\n"));
    }

    #[test]
    fn source_timestamp_aliases_keep_precision_and_do_not_invent_missing_values() {
        let content = "---\nid: mem-original\nkind: memory\nstatus: active\ntitle: Convention\n---\n\n---\nid: mem-original\nkind: memory\ncreated: 2026-10-24T08:07:25.802334+00:00\n---\n\nRemember this convention.\n";
        let parsed =
            parse_artifact_file("memory/mem-original.md", Arc::from(content.as_bytes()), 512)
                .unwrap();

        assert_eq!(
            parsed.created_at.as_deref(),
            Some("2026-10-24T08:07:25.802334+00:00")
        );
        assert_eq!(parsed.updated_at, None);
    }

    #[test]
    fn explicit_source_timestamps_and_nulls_take_precedence_over_legacy_aliases() {
        for (fields, expected_created, expected_updated) in [
            (
                "created_at: 2026-10-01T12:34:56Z\nupdated_at: 2026-10-02T01:02:03Z\n",
                Some("2026-10-01T12:34:56Z"),
                Some("2026-10-02T01:02:03Z"),
            ),
            ("created_at: null\nupdated_at: null\n", None, None),
        ] {
            let content = format!(
                "---\nid: NOTE-001\nkind: note\n{fields}created: 2026-10-24\nupdated: 2026-10-24\n---\n\nBody.\n"
            );
            let parsed = parse_artifact_file(
                "notes/NOTE-001-original.md",
                Arc::from(content.as_bytes()),
                512,
            )
            .unwrap();

            assert_eq!(parsed.created_at.as_deref(), expected_created);
            assert_eq!(parsed.updated_at.as_deref(), expected_updated);
        }
    }

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

    // -----------------------------------------------------------------------
    // Deterministic section layout.
    //
    // A capture resolves its layout from the process-wide model cache. That is
    // right in production and wrong in a unit test: without a downloaded
    // BGE-M3 the capture publishes **no** sections, so every section assertion
    // here would either panic or — worse — quietly pass vacuously on a machine
    // that happens to have one, and fail on one that does not. The same test
    // would then mean two different things depending on ambient state.
    //
    // So every test that asserts anything about sections installs its own
    // snapshot, built by [`crate::embed::tokenizer::testing`] and owned by the
    // test. The lock serialises the handful of tests that install one, because
    // the override is process-global: without it, one test's layout could be
    // swapped out from under another's capture mid-run.
    // -----------------------------------------------------------------------

    static LAYOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Installs a real tokenizer snapshot for the lifetime of the guard.
    struct FixtureLayout {
        _lock: std::sync::MutexGuard<'static, ()>,
        _cache: TempDir,
    }

    impl Drop for FixtureLayout {
        fn drop(&mut self) {
            // Before `_cache` is dropped: the snapshot has to outlive the
            // override that points at it, not the other way round.
            crate::embed::tokenizer::clear_test_layout_cache();
        }
    }

    /// A cache directory this test owns, holding one metadata-only snapshot.
    fn fixture_layout(limit: usize) -> FixtureLayout {
        // Poison-tolerant: a panicking test must not turn every later
        // layout-dependent test into a cascading failure.
        let lock = LAYOUT_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let cache = TempDir::new().unwrap();
        let snapshot = cache
            .path()
            .join(format!(
                "models--{}",
                super::super::jobs::DEFAULT_MODEL_REPO.replace('/', "--")
            ))
            .join("snapshots/rev-fixture");
        std::fs::create_dir_all(&snapshot).unwrap();
        crate::embed::tokenizer::testing::write_snapshot(&snapshot, limit).unwrap();
        crate::embed::tokenizer::set_test_layout_cache(Some(cache.path().to_path_buf()));
        FixtureLayout {
            _lock: lock,
            _cache: cache,
        }
    }

    /// The generation manifest a capture of this workspace would publish.
    ///
    /// This is the shape production capture continues from, so tests that
    /// exercise the identity baseline build one rather than holding a view —
    /// holding a view would test the `PreviousObservation::Capture` arm and
    /// leave the arm every production call site uses unexercised.
    fn published(captured: &CapturedSourceView) -> GenerationManifest {
        GenerationManifest::from_capture(captured, "gen-test", TableVersions::new(1, 1, 1))
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

    /// The published manifest is what production capture continues from, and
    /// it must carry identity forward exactly as a held capture view does.
    ///
    /// Without it every production pass re-derived `incarnation`/`row_revision`
    /// as `1`, so no edit ever moved `row_revision` and no rename ever moved
    /// `incarnation` — the two fences the semantic splice and the guarded
    /// completion both depend on.
    #[tokio::test]
    async fn a_published_manifest_baseline_carries_identity_forward() {
        let tmp = workspace();
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", PRD);
        let first = capture(tmp.path()).await;
        let manifest = published(&first);

        // An unchanged source reproduces the published identity exactly.
        let again = capture_source(
            &CaptureRequest::new(tmp.path()).with_published_manifest(Arc::new(manifest)),
            CaptureLimits::default(),
        )
        .await
        .unwrap();
        assert_eq!(again.entries[0].incarnation, 1);
        assert_eq!(again.entries[0].row_revision, 1);

        // A byte change moves `row_revision` and holds `incarnation` — the
        // metadata-only shape whose vector must survive.
        write_artifact(
            tmp.path(),
            "prds",
            "PRD-001-auth.md",
            &PRD.replace("status: draft", "status: active"),
        );
        let edited = capture_source(
            &CaptureRequest::new(tmp.path()).with_published_manifest(Arc::new(published(&again))),
            CaptureLimits::default(),
        )
        .await
        .unwrap();
        assert_eq!(
            edited.entries[0].row_revision, 2,
            "the bytes moved, so the row revision must too"
        );
        assert_eq!(
            edited.entries[0].incarnation, 1,
            "a metadata edit is the same document, not a new incarnation"
        );
        assert_eq!(
            edited.entries[0].input_digest, first.entries[0].input_digest,
            "this is what makes the edit preservable: the tokenizer's input \
             did not move"
        );

        // A rename moves `incarnation`, because the row is now at a path the
        // published generation never described.
        std::fs::rename(
            tmp.path().join("prds").join("PRD-001-auth.md"),
            tmp.path().join("prds").join("PRD-001-moved.md"),
        )
        .unwrap();
        let renamed = capture_source(
            &CaptureRequest::new(tmp.path()).with_published_manifest(Arc::new(published(&edited))),
            CaptureLimits::default(),
        )
        .await
        .unwrap();
        assert_eq!(renamed.entries[0].incarnation, 2);
        assert_eq!(renamed.entries[0].row_revision, 1);
    }

    /// The confirmed-delete/recreate fence: byte-identical content recreated
    /// at the same path must NOT reproduce the deleted row's identity.
    ///
    /// This is the case the missing manifest baseline left open. Without a
    /// baseline the capture minted `1` for everything, so the recreated
    /// document came back as `(incarnation 1, row_revision 1)` — the exact
    /// tuple `SemanticColumns::is_preserved_for` compares, and the exact tuple
    /// an outstanding `EmbeddingJob` binding carries. The deleted document's
    /// vector was therefore preserved onto its replacement and its stale
    /// completion was admitted.
    #[tokio::test]
    async fn a_byte_identical_recreate_after_a_confirmed_delete_gets_a_new_incarnation() {
        let tmp = workspace();
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", PRD);
        let first = capture(tmp.path()).await;

        // The delete is confirmed: the complete capture no longer enumerates
        // the path, so the reconciler records a tombstone naming the retired
        // incarnation. That verdict is what the manifest carries forward.
        std::fs::remove_file(tmp.path().join("prds").join("PRD-001-auth.md")).unwrap();
        let absent = capture(tmp.path()).await;
        assert!(
            absent.entries.is_empty(),
            "the delete must actually be observed as an absence"
        );
        let deleted_generation = {
            let mut manifest = published(&first);
            manifest.source_entries = Vec::new();
            manifest.tombstones = vec![Tombstone {
                canonical_key: "PRD-001".to_string(),
                relative_path: "prds/PRD-001-auth.md".to_string(),
                source_digest: first.entries[0].source_digest.clone(),
                incarnation: first.entries[0].incarnation,
                observed_at: absent.evaluated_at.clone(),
            }];
            manifest
        };

        // Byte-identical content, same path, same key. Nothing observable
        // distinguishes it from the deleted document — only the tombstone.
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", PRD);
        let recreated = capture_source(
            &CaptureRequest::new(tmp.path()).with_published_manifest(Arc::new(deleted_generation)),
            CaptureLimits::default(),
        )
        .await
        .unwrap();

        assert_eq!(recreated.entries.len(), 1);
        let entry = &recreated.entries[0];
        assert_eq!(
            entry.source_digest, first.entries[0].source_digest,
            "this test only means something if the bytes are identical"
        );
        assert_eq!(
            entry.incarnation, 2,
            "the recreate must skip past the incarnation the delete retired"
        );
        assert_eq!(
            entry.row_revision, 1,
            "a new incarnation starts at its first revision"
        );
    }

    /// A key that is still present must not be treated as retired, or every
    /// edit would be read as a delete/recreate.
    #[tokio::test]
    async fn a_live_key_ignores_a_tombstone_that_a_previous_generation_carried() {
        let tmp = workspace();
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", PRD);
        let first = capture(tmp.path()).await;

        // A manifest that still publishes the row AND carries a tombstone for
        // it — the state a generation is briefly in after the recreate is
        // published but before the tombstone is pruned. The live entry is the
        // truth; the tombstone must not resurrect a second bump.
        let carried = Tombstone {
            canonical_key: "PRD-001".to_string(),
            relative_path: "prds/PRD-001-auth.md".to_string(),
            source_digest: "an-older-digest".to_string(),
            incarnation: 1,
            observed_at: first.evaluated_at.clone(),
        };
        let mut manifest = published(&first);
        manifest.tombstones = vec![carried.clone()];

        let next = capture_source(
            &CaptureRequest::new(tmp.path()).with_published_manifest(Arc::new(manifest)),
            CaptureLimits::default(),
        )
        .await
        .unwrap();
        assert_eq!(
            next.entries[0].incarnation, 1,
            "an unchanged, still-published row keeps the identity it has"
        );

        // And the reason that matters: once the recreate itself is published,
        // a *further* pass must not bump it again. A tombstone that keeps
        // winning over the live entry would make identity climb forever and
        // the index would never converge.
        let mut republished = published(&next);
        republished.tombstones = vec![carried];
        let stable = capture_source(
            &CaptureRequest::new(tmp.path()).with_published_manifest(Arc::new(republished)),
            CaptureLimits::default(),
        )
        .await
        .unwrap();
        assert_eq!(
            stable.entries[0].incarnation, 1,
            "the published entry is authoritative once the recreate is \
             published; a surviving tombstone must not keep bumping it"
        );
        assert_eq!(stable.entries[0].row_revision, 1);
    }

    #[tokio::test]
    async fn config_digest_changes_when_config_changes() {
        // Pinned because `config_digest` now folds in the layout fingerprint:
        // with an ambient layout this comparison would depend on whether the
        // machine's cache changed between the two captures.
        let _layout = fixture_layout(crate::embed::tokenizer::MAX_TOKENS_CEILING);
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
        let _layout = fixture_layout(crate::embed::tokenizer::MAX_TOKENS_CEILING);
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
        // The digest covers exactly the string the tokenizer receives, measured
        // under **this capture's own** layout. For the fallback section that
        // string is byte-identical to the artifact's own input — `title + body
        // prefix` — so the digests legitimately agree; what keeps the rows
        // distinct is the key (`PRD-001` vs `PRD-001#s0`), which the binding
        // predicate carries.
        let (tokenizer, budgets) = view.section_layout().unwrap();
        assert_eq!(
            section.input_digest,
            section_input_digest(tokenizer, &budgets, "Auth", None, &section.content,).unwrap()
        );
        assert_eq!(
            section.input_digest, section.parent_input_digest,
            "a heading-less body embeds the same string twice — same digest, \
             different row keys"
        );
    }

    #[tokio::test]
    async fn a_preamble_and_headings_map_to_positional_sections() {
        let _layout = fixture_layout(crate::embed::tokenizer::MAX_TOKENS_CEILING);
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
        // heading + content` — measured under this capture's layout, and
        // headed sections hash a string the artifact input never produced.
        let (tokenizer, budgets) = view.section_layout().unwrap();
        let mut digests = std::collections::BTreeSet::new();
        for section in &sections {
            assert_eq!(
                section.input_digest,
                section_input_digest(
                    tokenizer,
                    &budgets,
                    "Auth",
                    section.heading.as_deref(),
                    &section.content,
                )
                .unwrap()
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
        let _layout = fixture_layout(crate::embed::tokenizer::MAX_TOKENS_CEILING);
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
        let _layout = fixture_layout(crate::embed::tokenizer::MAX_TOKENS_CEILING);
        let tmp = workspace();
        let content = "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\ndepth: standard\n---\n\nIntro.\n\n## Problem\nSlow.\n\n## Goals\nFast.\n";
        write_artifact(tmp.path(), "prds", "PRD-001-auth.md", content);

        let first = capture(tmp.path()).await;
        let second = capture(tmp.path()).await;
        // Not vacuous: with no tokenizer both captures would produce empty
        // sets and this would "pass" while proving nothing about derivation.
        assert!(
            !first.sections.is_empty(),
            "the fixture must resolve a real layout, or this comparison is empty == empty"
        );
        assert_eq!(
            first.sections, second.sections,
            "derivation is a pure function of the bytes"
        );
    }

    #[tokio::test]
    async fn content_past_the_body_prefix_is_reachable_through_a_section() {
        let _layout = fixture_layout(crate::embed::tokenizer::MAX_TOKENS_CEILING);
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
        // A long section is windowed, so ordinals shift; find the Appendix
        // row by what it says, not by where it landed.
        let appendix = view
            .sections_of("PRD-001")
            .find(|row| row.heading.as_deref() == Some("Appendix"))
            .expect("the Appendix section exists");
        assert!(
            view.sections_of("PRD-001")
                .filter(|row| row.heading.as_deref() == Some("Overview"))
                .count()
                > 1,
            "the long Overview section must span several rows"
        );
        assert!(
            view.section_input(appendix).unwrap().contains(tail),
            "the section input carries what the artifact input clipped"
        );
    }

    #[tokio::test]
    async fn an_invalid_artifact_contributes_no_section_rows() {
        let _layout = fixture_layout(crate::embed::tokenizer::MAX_TOKENS_CEILING);
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
            !view.sections.is_empty(),
            "the valid artifact must still own rows, or `all` below is vacuously true"
        );
        assert!(
            view.sections.iter().all(|s| s.parent_key == "PRD-001"),
            "an artifact that was never parsed cannot mint section rows"
        );
    }

    #[tokio::test]
    async fn a_section_longer_than_one_token_window_is_windowed_across_rows() {
        // A small declared limit, so the body budget is a few dozen tokens and
        // a ~2500-character section is guaranteed to window into many rows
        // rather than depending on where the 512-token default happens to
        // divide it.
        let _layout = fixture_layout(64);
        let tmp = workspace();
        let long = "x".repeat(DEFAULT_BODY_PREFIX_CHARS + 500);
        let content = format!(
            "---\nid: PRD-900\nkind: prd\nstatus: draft\ntitle: Long\n---\n\n## Body\n\n{long}\n"
        );
        write_artifact(tmp.path(), "prds", "PRD-900-long.md", &content);
        let view = capture(tmp.path()).await;

        let rows: Vec<_> = view.sections_of("PRD-900").collect();
        assert!(
            rows.len() >= 2,
            "a section longer than the prefix must become several rows, got {rows:?}"
        );
        let joined: String = rows.iter().map(|row| row.content.as_ref()).collect();
        // The whole artifact body, heading included: a window set that covered
        // only the section's prose would leave `## Body` uncovered, and
        // trimming here would hide exactly that loss.
        let expected = view
            .artifact("PRD-900")
            .expect("the captured artifact")
            .body
            .as_ref()
            .to_string();
        assert_eq!(
            joined, expected,
            "the windows must cover every byte of the artifact body exactly once, heading included"
        );
        // The window budget is in **tokens**, and it is the same budget the
        // encoder will apply. The old oracle here was
        // `content.chars().count() <= DEFAULT_BODY_PREFIX_CHARS`, which was
        // precisely the claim that no longer holds: `body_prefix_chars` is now
        // an artifact-row-only budget, and a character count said nothing about
        // what the tokenizer would see. Keeping it would have kept asserting a
        // property this layout does not promise, while missing the one it does.
        let (tokenizer, budgets) = view.section_layout().unwrap();
        for row in &rows {
            let input = view.section_input(row).unwrap();
            let counted = tokenizer
                .count_with_special_tokens(&input)
                .expect("tokenizing a section input must not fail");
            assert!(
                counted <= budgets.max_tokens,
                "section `{}` produced an input of {counted} tokens against the layout's actual \
                 limit of {}",
                row.canonical_key,
                budgets.max_tokens
            );
        }
        // Windows are distinguished by their **ranges**, not by their input
        // digests. A run of `x` cut into several windows produces windows whose
        // embedding inputs are byte-identical, so their digests are equal and
        // a pairwise-digest assertion would be asserting the opposite of the
        // point: it would pass only for a body with no repeated content, and
        // fail for every legitimately uniform one. What must hold instead is
        // that consecutive rows carry different ranges, different keys, and
        // abut exactly.
        assert!(
            rows.windows(2).all(|pair| {
                pair[0].byte_start != pair[1].byte_start
                    && pair[0].byte_end != pair[1].byte_end
                    && pair[0].canonical_key != pair[1].canonical_key
                    && pair[0].byte_end == pair[1].byte_start
            }),
            "each window must own its own contiguous byte range and key; got {:?}",
            rows.iter()
                .map(|row| (
                    row.canonical_key.as_str(),
                    row.byte_start,
                    row.byte_end,
                    row.input_digest.as_str()
                ))
                .collect::<Vec<_>>()
        );
    }

    // -----------------------------------------------------------------------
    // The preamble boundary.
    //
    // A preamble is the run before the first heading. Its boundary comes from
    // `list_section_spans` — the same parse that produces the sections — so a
    // disagreement between the two would compute ranges from two readings of
    // one body.
    // -----------------------------------------------------------------------

    /// The first heading's byte offset, read from the section parse that
    /// production also uses.
    ///
    /// Production takes this boundary from `list_section_spans` rather than
    /// from a second parse, so a test that computed it any other way would
    /// check a different code path than the one that ships.
    fn first_heading_start(body: &str) -> Option<usize> {
        crate::artifact::sections::list_section_spans(body)
            .first()
            .map(|section| section.raw_range.start)
    }

    /// The preamble is the run before the first heading, reported at the
    /// offset of its first **real** byte.
    ///
    /// The trimming matters for the *description*: a reader asking where the
    /// prose begins should not be answered with the offset of a blank line.
    /// The rows themselves publish the untrimmed prefix.
    #[test]
    fn a_preamble_is_reported_at_the_offset_of_its_first_real_byte() {
        // Leading blank lines, then prose, then a heading.
        let body = "\n\n  Intro prose before any heading.\n\n## Problem\nLogin is slow.\n";
        let (offset, text) =
            preamble_span(body, first_heading_start(body)).expect("there is a preamble");

        assert_eq!(text, "Intro prose before any heading.");
        assert_eq!(
            &body[offset..offset + text.len()],
            text,
            "the reported offset must index the bytes actually returned; a body that starts with \
             blank lines is exactly where an untrimmed offset goes wrong"
        );
        assert!(
            body[..offset].chars().all(char::is_whitespace),
            "everything before the offset is whitespace by construction"
        );
    }

    #[test]
    fn a_blank_or_absent_preamble_is_none_rather_than_an_empty_row() {
        // No heading at all: the whole-body fallback owns the body, so there is
        // no preamble to mint.
        assert_eq!(
            preamble_span("Just prose, no heading anywhere.\n", None),
            None,
            "without a heading there is no preamble; an empty row here would duplicate the \
             whole-body fallback under a second key"
        );
        // A heading with nothing but whitespace before it.
        let blank = "\n\n   \n## Problem\nbody\n";
        assert_eq!(
            preamble_span(blank, first_heading_start(blank)),
            None,
            "a whitespace-only run is not prose and must not become a row"
        );
        assert_eq!(preamble_span("", None), None);
    }

    /// A fenced code block is not a heading, so `#` inside it does not end the
    /// preamble. Both helpers read the same parse, so if they disagreed the two
    /// would compute ranges from different readings of one body.
    #[test]
    fn a_fenced_block_does_not_end_the_preamble() {
        let body = "Intro.\n\n```\n# not a heading\n```\n\n## Real\nbody\n";
        let (offset, text) =
            preamble_span(body, first_heading_start(body)).expect("the fenced block is prose");
        assert_eq!(
            text, "Intro.\n\n```\n# not a heading\n```",
            "the preamble runs to the first real ATX heading, fence or not"
        );
        assert_eq!(&body[offset..offset + text.len()], text);
    }

    /// A whitespace-only prefix belongs to the **first section**, not to a row
    /// of its own, and the first section's published range therefore starts at
    /// byte `0`.
    ///
    /// This is the ownership rule that makes the tiling verifier satisfiable:
    /// a prefix cannot belong to both a preamble row and the first section, or
    /// to neither.
    #[test]
    fn a_whitespace_only_prefix_joins_the_first_section() {
        let body = "\n \t\n## A\nx\n";
        assert_eq!(
            first_heading_start(body),
            Some(4),
            "the leading run is whitespace, so the preamble helper declines it"
        );
        assert_eq!(
            preamble_span(body, first_heading_start(body)),
            None,
            "no prose, no preamble row"
        );
    }
}
