//! Embedding profile, job binding and semantic state (SPEC-008 "Data Models").
//!
//! The rule this module exists to enforce: **`source`, `input` and `profile`
//! are three separately compared things.** `sourceDigest` says the markdown
//! bytes moved; `inputDigest` says the bytes handed to the tokenizer moved;
//! `profileId` says the model/weights/tokenizer/pooling space is the same one.
//! Collapsing them is how a vector computed from a superseded body keeps
//! being served as current.
//!
//! Nothing here downloads weights. `EmbeddingProfile` is a *description*; the
//! engine that realises it (and the cache-only construction rule) is P3's
//! subject, and a profile whose checksums have not been read from a local
//! snapshot stays `None` — never a guessed revision.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{CheckedCounter, ConstellationError, DigestBuilder, PROFILE_DOMAIN, sha256_hex};

/// Default number of body characters that feed the tokenizer.
///
/// Matches the historical `embedding.chunk_size` default so a vector built
/// by the old embedder and one built under this contract agree on the input
/// they hashed. Changing it changes `inputDigest`, which is the point.
pub const DEFAULT_BODY_PREFIX_CHARS: usize = 2000;

/// Default tokenizer token budget. BGE-M3 caps at 512.
pub const DEFAULT_MAX_TOKENS: usize = 512;

/// Semantic state of one artifact row, relative to the active profile.
///
/// `Unavailable` is **not** a failure. It is what a build without the
/// `semantic-search` feature — or with no cached model — reports. Folding it
/// into `Failed` would make "this binary has no vectors" indistinguishable
/// from "embedding the model broke", and both would look like a defect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticState {
    /// Vector published for the exact current input + profile + incarnation.
    Ready,
    /// Durably recorded as work to do; not yet published.
    Pending,
    /// Attempted and failed; `retryAfter` says when it may be retried.
    Failed,
    /// No semantic capability in this build or no cached model.
    Unavailable,
}

impl SemanticState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Pending => "pending",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
        }
    }

    /// Whether a vector for this row may take part in a query result.
    ///
    /// `Pending` and `Failed` are not eligible: an empty eligible set must
    /// surface as incomplete semantic coverage, never as "nothing matched".
    pub fn is_query_eligible(self) -> bool {
        matches!(self, Self::Ready)
    }
}

impl std::fmt::Display for SemanticState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Immutable description of one embedding space.
///
/// Every field participates in `profile_id`. Changing any one of them makes
/// the new space incompatible with the old vectors — which is the correct
/// outcome: mixing two spaces silently produces confident nonsense.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingProfile {
    pub model_repo: String,
    pub weights_revision: String,
    /// Checksums of `model.onnx` plus any external weight files.
    pub weights_checksums: Vec<String>,
    pub tokenizer_checksum: String,
    pub tokenizer_config_checksum: String,
    pub dimensions: u32,
    pub engine_version: String,
    pub pooling: String,
    pub normalization: String,
    pub query_prefix: String,
    pub document_prefix: String,
    pub body_prefix_chars: usize,
    pub max_tokens: usize,
    pub padding: String,
    pub truncation: String,
}

/// Stable identity of an [`EmbeddingProfile`].
///
/// Domain-separated, length-prefixed SHA-256 over every field in declaration
/// order, so the encoding cannot drift silently.
impl EmbeddingProfile {
    /// `profileId` for this profile.
    pub fn profile_id(&self) -> String {
        let mut d = DigestBuilder::new(PROFILE_DOMAIN);
        d.field(self.model_repo.as_bytes())
            .field(self.weights_revision.as_bytes());
        // Checksum lists are sorted so two profiles that declare the same
        // weight files in a different order hash identically.
        let mut checksums = self.weights_checksums.clone();
        checksums.sort();
        d.field(checksums.join("\u{1}").as_bytes())
            .field(self.tokenizer_checksum.as_bytes())
            .field(self.tokenizer_config_checksum.as_bytes())
            .field(self.dimensions.to_string().as_bytes())
            .field(self.engine_version.as_bytes())
            .field(self.pooling.as_bytes())
            .field(self.normalization.as_bytes())
            .field(self.query_prefix.as_bytes())
            .field(self.document_prefix.as_bytes())
            .field(self.body_prefix_chars.to_string().as_bytes())
            .field(self.max_tokens.to_string().as_bytes())
            .field(self.padding.as_bytes())
            .field(self.truncation.as_bytes());
        d.finish_hex()
    }
}

/// The exact text handed to the tokenizer for one artifact.
///
/// `title + " " + body.chars().take(body_prefix_chars)` — identical in shape
/// to the historical `ArtifactRecord::embedding_text`, deliberately. The ID
/// is excluded (no semantics, only noise), and the scope is a **prefix**, not
/// a full document: `semantic_scope=prefix` is reported to callers rather
/// than sold as whole-document matching.
pub fn embedding_input_text(title: &str, body: &str, body_prefix_chars: usize) -> String {
    let prefix: String = body.chars().take(body_prefix_chars).collect();
    format!("{title} {prefix}")
}

/// SHA-256 of the exact bytes handed to the tokenizer.
///
/// Hashing the bytes *before* tokenization is what makes "did the vector's
/// input change?" answerable without re-running the model.
pub fn embedding_input_digest(title: &str, body: &str, body_prefix_chars: usize) -> String {
    sha256_hex(embedding_input_text(title, body, body_prefix_chars).as_bytes())
}

/// Deterministic identity of one section row.
///
/// `<parent canonical key>#s<ordinal>` — the ordinal is the section's
/// position in [`crate::artifact::sections::list_sections`] output, so the
/// same captured body always produces the same key. Key uniqueness inside a
/// parent is positional on purpose: two sections that happen to share a
/// heading are still different rows, and a body edit that reorders sections
/// changes which content a key binds, which is exactly what `input_digest`
/// then detects.
pub fn section_key(parent_key: &str, ordinal: u32) -> String {
    format!("{parent_key}#s{ordinal}")
}

/// The exact text handed to the tokenizer for one section.
///
/// Same shape as the artifact input — `title + heading + content prefix` —
/// so a section vector lives in the same embedding space as its parent's
/// vector and the two ANN result sets can be merged on one score scale.
/// `heading` is `None` only for the body-with-no-headings fallback section
/// (ordinal 0), which embeds as `title + " " + content prefix`.
pub fn section_input_text(
    title: &str,
    heading: Option<&str>,
    content: &str,
    body_prefix_chars: usize,
) -> String {
    let prefix: String = content.chars().take(body_prefix_chars).collect();
    match heading {
        Some(heading) if !heading.is_empty() => format!("{title} {heading} {prefix}"),
        _ => format!("{title} {prefix}"),
    }
}

/// SHA-256 of the exact bytes handed to the tokenizer for one section.
///
/// Separate from the parent's `input_digest` on purpose: the artifact row's
/// digest describes the title+body-prefix input, the section row's digest
/// describes the title+heading+section input. Folding them would let a
/// section vector publish under a binding its bytes never satisfied.
pub fn section_input_digest(
    title: &str,
    heading: Option<&str>,
    content: &str,
    body_prefix_chars: usize,
) -> String {
    sha256_hex(section_input_text(title, heading, content, body_prefix_chars).as_bytes())
}

/// One unit of embedding work, bound to the exact state it was computed for.
///
/// Every field is mandatory. A job missing `incarnation` could apply to a
/// file that was deleted and recreated; a job missing `profileId` could
/// publish into a space its vector was not built in. Both are silent
/// resurrection vectors, so the binding is closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingJob {
    pub index_epoch: u64,
    pub checkout_id: String,
    pub canonical_key: String,
    pub incarnation: u64,
    pub row_revision: u64,
    pub source_digest: String,
    pub input_digest: String,
    pub profile_id: String,
    /// Reference to the captured input bytes. The bytes themselves live in the
    /// captured source view; a manifest never carries a second copy of a
    /// body.
    pub captured_input_ref: String,
    pub attempt: u32,
    pub retry_after: Option<String>,
}

impl EmbeddingJob {
    /// Advance the attempt counter, refusing to wrap.
    pub fn next_attempt(&self) -> Result<Self, ConstellationError> {
        let mut next = self.clone();
        next.attempt = CheckedCounter::u32_inc("attempt", self.attempt)?;
        Ok(next)
    }

    /// The job a **failed** attempt produces: attempt advanced, `retry_after`
    /// set to the moment this key may be admitted again.
    ///
    /// Every other field is carried unchanged. A failure changes how hard the
    /// worker has tried and when it may try again — never *what* the work is,
    /// because a job that re-binds its digest on failure is a job that could
    /// publish against bytes it never computed from.
    ///
    /// The attempt counter advances on failure, never on admission, so a
    /// reconcile between two attempts cannot reset how many times this has
    /// already been tried.
    pub fn failed_at(&self, retry_after: String) -> Result<Self, ConstellationError> {
        let mut next = self.next_attempt()?;
        next.retry_after = Some(retry_after);
        Ok(next)
    }

    /// Whether this job is inside its retry delay at `now`.
    ///
    /// `None` means no failure was recorded, so the job is due now. A
    /// `retry_after` that cannot be parsed is treated as **due**: a clock this
    /// build cannot read is not a reason to park a key forever, and the
    /// alternative — never retrying — is the failure that actually loses work.
    pub fn retry_is_due(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        retry_is_due(self.retry_after.as_deref(), now)
    }

    /// Whether this job may still be published against the given binding.
    ///
    /// All nine components are compared. This is the predicate the publisher
    /// evaluates before its conditional row update; a worker never gets to
    /// insert, so a job that fails this simply produces no row.
    #[allow(clippy::too_many_arguments)]
    pub fn matches_binding(
        &self,
        index_epoch: u64,
        checkout_id: &str,
        canonical_key: &str,
        incarnation: u64,
        row_revision: u64,
        source_digest: &str,
        input_digest: &str,
        profile_id: &str,
    ) -> bool {
        self.index_epoch == index_epoch
            && self.checkout_id == checkout_id
            && self.canonical_key == canonical_key
            && self.incarnation == incarnation
            && self.row_revision == row_revision
            && self.source_digest == source_digest
            && self.input_digest == input_digest
            && self.profile_id == profile_id
    }

    /// The binding this job publishes against, rebuilt from the job itself.
    ///
    /// From the **job**, never from a capture taken after the compute: that is
    /// the whole point of carrying the job into the completion. A binding read
    /// off a fresh capture would let a vector computed from superseded bytes
    /// be relabelled onto the bytes that replaced them.
    pub fn binding(&self) -> Binding {
        Binding {
            index_epoch: self.index_epoch,
            checkout_id: self.checkout_id.clone(),
            canonical_key: self.canonical_key.clone(),
            incarnation: self.incarnation,
            row_revision: self.row_revision,
            source_digest: self.source_digest.clone(),
            input_digest: self.input_digest.clone(),
            profile_id: self.profile_id.clone(),
        }
    }

    /// Which table this job's row lives in.
    pub fn row_role(&self) -> EmbeddingRowRole {
        if section_parent_key(&self.canonical_key).is_some() {
            EmbeddingRowRole::Section
        } else {
            EmbeddingRowRole::Artifact
        }
    }

    /// The canonical `captured_input_ref` for one admission.
    ///
    /// `"<admission_generation>:<canonical_key>"`. It is an **identity of the
    /// right to publish**, never a path to read: the bytes were captured once,
    /// under the owner, and the compute reads them from the prepared batch.
    /// Making it a path would invite a completion to re-read the file and bind
    /// a vector to whatever is there now.
    pub fn captured_input_ref(admission_generation: &str, canonical_key: &str) -> String {
        format!("{admission_generation}:{canonical_key}")
    }

    /// Build a job for one admitted binding.
    ///
    /// `attempt` starts at `0` and `retry_after` is absent: a job that has
    /// never run has neither failed nor been asked to wait.
    pub fn for_binding(
        binding: &Binding,
        admission_generation: &str,
    ) -> Result<Self, ConstellationError> {
        if binding.canonical_key.is_empty() {
            return Err(ConstellationError::Io {
                detail: "an embedding job needs a canonical key; an empty one binds no row"
                    .to_string(),
            });
        }
        if binding.input_digest.is_empty() {
            return Err(ConstellationError::Io {
                detail: format!(
                    "refusing to admit `{}`: it carries no input digest, so no binding can \
                     fence the result",
                    binding.canonical_key
                ),
            });
        }
        Ok(Self {
            index_epoch: binding.index_epoch,
            checkout_id: binding.checkout_id.clone(),
            canonical_key: binding.canonical_key.clone(),
            incarnation: binding.incarnation,
            row_revision: binding.row_revision,
            source_digest: binding.source_digest.clone(),
            input_digest: binding.input_digest.clone(),
            profile_id: binding.profile_id.clone(),
            captured_input_ref: Self::captured_input_ref(
                admission_generation,
                &binding.canonical_key,
            ),
            attempt: 0,
            retry_after: None,
        })
    }

    /// Whether this job describes **the same work** as `binding`.
    ///
    /// Compared without `captured_input_ref`, `attempt` and `retry_after`:
    /// those record *which admission* asked and *how often it has been tried*,
    /// not what the vector must be computed from. An admission that re-derives
    /// an identical binding must therefore keep the job already pending —
    /// minting a fresh one would reset its attempt counter on every reconcile
    /// and make a permanently failing key look brand new each pass.
    pub fn describes(&self, binding: &Binding) -> bool {
        self.matches_binding(
            binding.index_epoch,
            &binding.checkout_id,
            &binding.canonical_key,
            binding.incarnation,
            binding.row_revision,
            &binding.source_digest,
            &binding.input_digest,
            &binding.profile_id,
        )
    }
}

/// The first retry delay, in seconds. One minute.
pub const RETRY_BASE_SECONDS: i64 = 60;

/// The longest a failing key is ever parked, in seconds. One hour.
pub const RETRY_CEILING_SECONDS: i64 = 3_600;

/// The delay before attempt `attempt + 1`, doubling from
/// [`RETRY_BASE_SECONDS`] and capped at [`RETRY_CEILING_SECONDS`].
///
/// `attempt` is the count already recorded, so the first failure waits one
/// base interval and a key that has failed thirty times still comes back.
pub fn retry_delay_seconds(attempt: u32) -> i64 {
    let shift = attempt.min(16) as i64;
    RETRY_BASE_SECONDS
        .saturating_mul(1i64 << shift)
        .min(RETRY_CEILING_SECONDS)
}

/// The RFC 3339 instant at which a job with `attempt` recorded may be tried
/// again.
///
/// `now` is a parameter so a caller — and a test — controls the clock.
pub fn retry_after_at(attempt: u32, now: chrono::DateTime<chrono::Utc>) -> String {
    (now + chrono::Duration::seconds(retry_delay_seconds(attempt)))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Whether `retry_after` permits another attempt at `now`.
///
/// `None` is due. An unparseable stamp is **due**: this is the one place where
/// being wrong in the permissive direction loses a bounded amount of work
/// (one extra inference) and being wrong the other way loses it permanently.
pub fn retry_is_due(retry_after: Option<&str>, now: chrono::DateTime<chrono::Utc>) -> bool {
    let Some(stamp) = retry_after else {
        return true;
    };
    match chrono::DateTime::parse_from_rfc3339(stamp) {
        Ok(until) => now >= until.with_timezone(&chrono::Utc),
        // Unreadable: due rather than parked forever.
        Err(_) => true,
    }
}

/// The `model_repo` the initial configuration embeds. It names a snapshot;
/// it does **not** identify one.
pub const DEFAULT_MODEL_REPO: &str = "BAAI/bge-m3";

/// Files an initial profile must find in a snapshot, relative to its root.
///
/// Their *presence* is checked here; their *content* is checked by
/// [`SnapshotEvidence::checksums_resolved`], which cannot be satisfied
/// without real digests.
const REQUIRED_SNAPSHOT_FILES: &[&str] = &[
    "config.json",
    "tokenizer.json",
    "tokenizer_config.json",
    "onnx/model.onnx",
    "onnx/model.onnx_data",
];

/// What is actually on disk at a model cache path.
///
/// This is deliberately a thin reading of the HF cache layout rather than a
/// parse of the model: `refs/main` holds the resolved commit, and
/// `snapshots/<commit>/` holds the files. Nothing here infers a revision
/// from a repo name or a default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotEvidence {
    /// Contents of `refs/main` — the resolved commit, read from disk.
    pub revision: String,
    pub root: PathBuf,
    /// Relative paths that exist under the snapshot root.
    pub present_files: Vec<String>,
    /// Relative paths from [`REQUIRED_SNAPSHOT_FILES`] that are missing.
    pub missing_files: Vec<String>,
}

impl SnapshotEvidence {
    /// Read the evidence at `cache_dir` for `repo`.
    ///
    /// Returns `None` when the cache directory or `refs/main` is absent —
    /// that is "no cached model", not an error.
    pub fn read(cache_dir: &Path, repo: &str) -> Option<Self> {
        let repo_dir = cache_dir.join(format!("models--{}", repo.replace('/', "--")));
        let revision = std::fs::read_to_string(repo_dir.join("refs").join("main"))
            .ok()?
            .trim()
            .to_string();
        if revision.is_empty() {
            return None;
        }
        let root = repo_dir.join("snapshots").join(&revision);

        let mut present_files = Vec::new();
        let mut missing_files = Vec::new();
        for relative in REQUIRED_SNAPSHOT_FILES {
            if root.join(relative).exists() {
                present_files.push((*relative).to_string());
            } else {
                missing_files.push((*relative).to_string());
            }
        }
        Some(Self {
            revision,
            root,
            present_files,
            missing_files,
        })
    }

    /// Whether every file an initial profile needs is present.
    pub fn is_complete(&self) -> bool {
        self.missing_files.is_empty()
    }

    /// Whether real content digests were supplied for this snapshot.
    ///
    /// **False means the profile is unresolved.** A profile without real
    /// weights and tokenizer checksums cannot be published against: the
    /// vectors it would produce describe weights nobody has identified, and
    /// they would later present as `ready`. Returning `false` is the correct
    /// outcome, not a failure.
    pub fn checksums_resolved(
        &self,
        weights: &[String],
        tokenizer: &str,
        tokenizer_config: &str,
    ) -> bool {
        self.is_complete()
            && !weights.is_empty()
            && !tokenizer.is_empty()
            && !tokenizer_config.is_empty()
    }

    /// Build the initial profile, or explain why it cannot be built.
    ///
    /// # Errors
    ///
    /// [`super::ConstellationError::ModelUnavailable`] when a required file
    /// is missing. Callers treat that as "rows stay `Unavailable`", never as
    /// a reason to fall back to a guessed revision.
    pub fn to_profile(
        &self,
        weights_checksums: Vec<String>,
        tokenizer_checksum: String,
        tokenizer_config_checksum: String,
        engine_version: &str,
    ) -> Result<EmbeddingProfile, super::ConstellationError> {
        if !self.is_complete() {
            return Err(super::ConstellationError::ModelUnavailable {
                detail: format!(
                    "snapshot {} is missing {:?}; no profile can be resolved from it",
                    self.revision, self.missing_files
                ),
            });
        }
        Ok(EmbeddingProfile {
            model_repo: DEFAULT_MODEL_REPO.to_string(),
            // Read from `refs/main`, not derived from the repo name.
            weights_revision: self.revision.clone(),
            weights_checksums,
            tokenizer_checksum,
            tokenizer_config_checksum,
            dimensions: DEFAULT_DIMENSIONS,
            engine_version: engine_version.to_string(),
            pooling: "cls".to_string(),
            normalization: "l2".to_string(),
            query_prefix: String::new(),
            document_prefix: String::new(),
            body_prefix_chars: DEFAULT_BODY_PREFIX_CHARS,
            max_tokens: DEFAULT_MAX_TOKENS,
            padding: "none".to_string(),
            truncation: "longest_first".to_string(),
        })
    }
}

/// `hidden_size` in the snapshot's `config.json`. Verified on this machine:
/// `"hidden_size": 1024`.
pub const DEFAULT_DIMENSIONS: u32 = 1024;

/// Scope of what a vector actually covers.
///
/// `prefix`, honestly. It is **not** whole-document and **not** section
/// chunking: only `title + " " + body.chars().take(body_prefix_chars)` is
/// embedded, so a match says nothing about the rest of the document. The
/// value travels with every result so a caller cannot mistake it for full
/// coverage.
pub const SEMANTIC_SCOPE: &str = "prefix";

/// Scope reported when the pinned generation also carries a sections table.
///
/// The artifact vector still covers only its prefix — that half does not
/// change — but the generation additionally embeds every section, so a hit
/// may come from content beyond the prefix. `"prefix+sections"` rather than
/// `"document"`: coverage is still chunked, not whole-body, and the name
/// says exactly which two input shapes produced the candidate set.
pub const SEMANTIC_SCOPE_WITH_SECTIONS: &str = "prefix+sections";

/// What the semantic channel did, and what it could not do.
///
/// The counts are populated from work that happened. `pending == 0` because
/// nothing ran is a different fact from `pending == 0` because everything ran
/// and none were eligible, and `staged` is what tells them apart.
/// `retry_after` is an owned `String`, so this type is `Clone` but not `Copy`.
/// A cheap shared copy would be `Arc<SemanticOutcome>` at the call site, not
/// a re-derived `Copy`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticOutcome {
    pub ready: usize,
    pub pending: usize,
    pub failed: usize,
    pub unavailable: usize,
    /// Work that actually ran this pass.
    pub staged: usize,
    pub retry_after: Option<String>,
}

impl SemanticOutcome {
    /// Total rows accounted for.
    pub fn total(&self) -> usize {
        self.ready + self.pending + self.failed + self.unavailable
    }

    /// Whether a result drawn from this outcome can claim semantic coverage.
    ///
    /// An empty eligible set under pending, failed or incomplete coverage
    /// does **not** mean "nothing matched" — it means the index could not see.
    pub fn is_semantically_covered(&self) -> bool {
        self.pending == 0 && self.failed == 0 && self.unavailable == 0
    }
}

/// Bounded read buffer for content hashing.
///
/// 64 KiB. `model.onnx_data` is 2,266,820,608 bytes: a function that
/// allocated a `Vec` the size of the model would appear to work on the four
/// small files and exhaust memory on the one that matters.
const HASH_CHUNK: usize = 64 * 1024;

/// The two ONNX files whose digests make up the weights binding.
const WEIGHT_FILES: &[&str] = &["onnx/model.onnx", "onnx/model.onnx_data"];

/// Compute SHA-256 over the **content** of every file in a cached snapshot.
///
/// This is what makes [`ResolvedProfile`] able to resolve at all. Until it
/// existed the profile could only ever be `unavailable`, and the alternative —
/// carrying a table of known digests — would be a constant claiming to verify
/// weights it never hashed.
///
/// # Symlinks
///
/// Every file in an HF cache snapshot is a symlink into `blobs/`.
/// `File::open` **follows** them, which is what is wanted; `symlink_metadata`
/// does not. Nothing here calls `symlink_metadata` on a file it hashes, and
/// no size is recorded from a link rather than its target.
///
/// # Errors
///
/// [`super::ConstellationError::ModelUnavailable`] when the snapshot is not
/// cached or any required file is missing. A partial snapshot must not
/// resolve: a digest of nothing is not a digest.
///
/// # Pinned API, at the point of use
///
/// ```text
/// // sha2-0.11.0/src/lib.rs:36 — Sha256; update/finalize come from `Digest`.
/// // The identical pattern already lives at constellation/mod.rs:44-45,371-373.
/// ```
pub fn snapshot_checksums(
    cache_dir: &Path,
    repo: &str,
) -> Result<SnapshotChecksums, super::ConstellationError> {
    let evidence = SnapshotEvidence::read(cache_dir, repo).ok_or_else(|| {
        super::ConstellationError::ModelUnavailable {
            detail: format!(
                "no cached snapshot for `{repo}` under {}; the model was never downloaded, or \
                 `refs/main` is unreadable",
                cache_dir.display()
            ),
        }
    })?;
    snapshot_checksums_for_evidence(&evidence)
}

fn snapshot_checksums_for_evidence(
    evidence: &SnapshotEvidence,
) -> Result<SnapshotChecksums, super::ConstellationError> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let hash_file = |relative: &str| -> Result<String, super::ConstellationError> {
        let path = evidence.root.join(relative);
        let mut file = std::fs::File::open(&path).map_err(|e| {
            super::ConstellationError::ModelUnavailable {
                detail: format!("cannot open snapshot file {}: {e}", path.display()),
            }
        })?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; HASH_CHUNK];
        loop {
            let read = file.read(&mut buffer).map_err(|e| {
                super::ConstellationError::ModelUnavailable {
                    detail: format!("cannot read snapshot file {}: {e}", path.display()),
                }
            })?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok(super::hex_encode(&hasher.finalize()))
    };

    let mut weights = Vec::with_capacity(WEIGHT_FILES.len());
    for relative in WEIGHT_FILES {
        weights.push(hash_file(relative)?);
    }

    Ok(SnapshotChecksums {
        weights,
        tokenizer: hash_file("tokenizer.json")?,
        tokenizer_config: hash_file("tokenizer_config.json")?,
        config: hash_file("config.json")?,
    })
}

/// A snapshot on this machine whose identity has been **verified**.
///
/// It holds the profile and nothing else — no engine. That is deliberate and
/// it is the difference between a gate and a lie:
///
/// * An inference engine is resident memory, opened through a path whose whole
///   purpose is to avoid a 2 GB download. Holding one in a process that has
///   nothing to embed is the most expensive no-op available, and a field no
///   code reads is the P2d-4 defect with a memory bill attached.
/// * Nothing can call this yet. The embedding caller arrives with P4's
///   runtime, and **the engine field returns with that reader**, not before.
///   Adding a method now would only move the same warning one level up.
///
/// What this type *does* do is answer the question every semantic path must
/// ask first: may I use semantics at all, and against exactly which weights?
/// `open` resolves that from the files on disk, never downloads, and returns
/// `model_unavailable` when it cannot — which is what keeps rows
/// `Unavailable` instead of publishing a guessed revision.
#[cfg(feature = "semantic-search")]
pub struct ResolvedProfile {
    pub profile: EmbeddingProfile,
    pub snapshot_root: PathBuf,
    pub snapshot_id: String,
}

#[cfg(feature = "semantic-search")]
impl ResolvedProfile {
    /// Resolve the already-cached model snapshot and its content digests.
    ///
    /// This never downloads a model. Query and maintenance paths use the
    /// returned snapshot identity for both profile construction and engine
    /// loading, so a changed `refs/main` cannot silently select different
    /// weights between the two operations.
    ///
    /// # Argument order
    ///
    /// `(cache_dir, repo, engine_version, body_prefix_chars)` — the RFC-015
    /// order, and it is fixed: two `&str`/`usize` neighbours that are easy to
    /// swap at a call site, and swapping them compiles. `engine_version` is the
    /// *build's* identity and belongs with the other identity arguments;
    /// `body_prefix_chars` is the *workspace's* chunk size, read from that
    /// workspace's config, and belongs last where a caller cannot plausibly
    /// reach for it by mistake.
    pub fn open(
        cache_dir: &Path,
        repo: &str,
        engine_version: &str,
        body_prefix_chars: usize,
    ) -> Result<Self, super::ConstellationError> {
        // Same absence, same wording as `snapshot_checksums`: no `refs/main`
        // under the cache dir means the model was never downloaded. Calling it
        // an "unreadable layout" pointed an operator at a corrupt cache they
        // did not have, when the truthful reading was "nothing is there".
        let evidence = SnapshotEvidence::read(cache_dir, repo).ok_or_else(|| {
            super::ConstellationError::ModelUnavailable {
                detail: format!(
                    "no cached snapshot for `{repo}` under {}; the model was never downloaded, or \
                     `refs/main` is unreadable",
                    cache_dir.display()
                ),
            }
        })?;
        let checksums = snapshot_checksums_for_evidence(&evidence)?;
        let profile = evidence.to_profile(
            checksums.weights,
            checksums.tokenizer,
            checksums.tokenizer_config,
            engine_version,
        )?;
        if !evidence.checksums_resolved(
            &profile.weights_checksums,
            &profile.tokenizer_checksum,
            &profile.tokenizer_config_checksum,
        ) {
            return Err(super::ConstellationError::ModelUnavailable {
                detail: format!(
                    "snapshot {} has no real content digests; refusing to resolve a profile whose \
                     weights are unidentified",
                    evidence.revision
                ),
            });
        }
        let mut profile = profile;
        profile.body_prefix_chars = body_prefix_chars;

        Ok(Self {
            profile,
            snapshot_root: evidence.root,
            snapshot_id: evidence.revision,
        })
    }
}

/// Hand-written rather than derived, because the profile is what an operator
/// needs and every field of it is diagnostic.
///
/// * `model_repo`, `weights_revision`, `dimensions`, `engine_version` — which
///   weights were resolved;
/// * `weights_checksums` / `tokenizer_checksum` — so their *absence* is
///   itself the signal that a profile stayed unresolved;
/// * `profile_id` — the single digest every `Binding` is compared against, so
///   seeing it is how a caller checks the row it holds was built in this space.
#[cfg(feature = "semantic-search")]
impl std::fmt::Debug for ResolvedProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedProfile")
            .field("model_repo", &self.profile.model_repo)
            .field("weights_revision", &self.profile.weights_revision)
            .field("weights_checksums", &self.profile.weights_checksums)
            .field("tokenizer_checksum", &self.profile.tokenizer_checksum)
            .field("dimensions", &self.profile.dimensions)
            .field("engine_version", &self.profile.engine_version)
            .field("profile_id", &self.profile.profile_id())
            .finish_non_exhaustive()
    }
}

/// The configured model name the profile above belongs to.
///
/// `resolve_repo(DEFAULT_MODEL_NAME) == DEFAULT_MODEL_REPO`; the two are kept
/// apart because one is a user-facing config value and the other is an identity
/// recorded in an index. Every engine constructor takes the **name**, and every
/// profile records the **repo**, so neither has to be derived from the other at
/// a call site.
pub const DEFAULT_MODEL_NAME: &str = "bge-m3";

/// A query engine opened from the snapshot this machine already has, under the
/// profile identity the index pinned.
///
/// This is the one constructor every **read** path uses. It pairs the two facts
/// a semantic query needs and that nothing else ties together:
///
/// * the engine, opened from a snapshot verified complete
///   ([`crate::embed::CachedSnapshot`]) — never a download; and
/// * the profile whose weights those bytes hash to, resolved from the same
///   cache by content ([`ResolvedProfile::open`]).
///
/// Comparing the resolved `profile_id` against the one `CURRENT` pins is what
/// makes a query answerable rather than merely possible: vectors from two
/// revisions are both 1024 floats and both normalize to unit length, so
/// nothing downstream would notice that the comparison was meaningless.
#[cfg(feature = "semantic-search")]
pub struct CachedQueryEngine {
    pub profile_id: String,
    model: crate::embed::CachedSnapshot,
}

#[cfg(feature = "semantic-search")]
impl CachedQueryEngine {
    /// Embed one query string.
    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>, super::ConstellationError> {
        self.model
            .embedder
            .embed(text)
            .map_err(|e| super::ConstellationError::ModelUnavailable {
                detail: format!(
                    "the query could not be embedded by the cached model at {}: {e}",
                    self.model.snapshot_root.display()
                ),
            })
    }
}

/// Open a cached-only query engine for the captured profile parameters.
/// A different profile is refused before constructing the inference engine.
#[cfg(feature = "semantic-search")]
pub fn open_cached_query_engine_in(
    cache_dir: &Path,
    model_name: &str,
    expected_profile_id: Option<&str>,
    body_prefix_chars: usize,
) -> Result<CachedQueryEngine, super::ConstellationError> {
    let repo = crate::embed::resolve_repo(model_name).ok_or_else(|| {
        super::ConstellationError::ModelUnavailable {
            detail: format!("unsupported embedding model: {model_name}"),
        }
    })?;
    let resolved = ResolvedProfile::open(
        cache_dir,
        repo,
        super::facade::ENGINE_VERSION,
        body_prefix_chars,
    )?;
    let profile_id = resolved.profile.profile_id();

    if let Some(expected) = expected_profile_id {
        if expected != profile_id {
            return Err(super::ConstellationError::ModelUnavailable {
                detail: format!(
                    "the index pinned profile `{expected}` but the cached snapshot resolves to \
                     `{profile_id}`. Vectors from two revisions are both unit-length float \
                     vectors, so querying across them returns plausible nonsense. Fix: point \
                     FORGEPLAN_MODEL_CACHE at the snapshot the index was built from, or re-embed \
                     under the cached one."
                ),
            });
        }
    }

    let embedder = crate::embed::Embedder::from_snapshot(&resolved.snapshot_root, model_name)
        .map_err(|e| super::ConstellationError::ModelUnavailable {
            detail: e.to_string(),
        })?;
    let model = crate::embed::CachedSnapshot {
        embedder,
        model_repo: repo.to_string(),
        snapshot_id: resolved.snapshot_id,
        snapshot_root: resolved.snapshot_root,
    };

    Ok(CachedQueryEngine { profile_id, model })
}

/// SHA-256 digests measured over snapshot **file content**.
///
/// Separate from [`SnapshotEvidence`] on purpose: evidence says what is on
/// disk, this says what those bytes hash to. In this HF cache every file is a
/// symlink into `../../blobs/`, so a blob *name* is not a checksum — for the
/// three large files it happens to equal the SHA-256, and for `config.json`
/// and `tokenizer_config.json` it matches neither SHA-256 nor git SHA-1. The
/// only sound source is the content.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SnapshotChecksums {
    pub weights: Vec<String>,
    pub tokenizer: String,
    pub tokenizer_config: String,
    /// `config.json`'s digest. It fixes the model's declared architecture and
    /// hidden size, so it is hashed even though it is not part of
    /// `profile_id` — recording it means a mismatch is diagnosable rather
    /// than invisible.
    pub config: String,
}

impl SnapshotChecksums {
    /// Whether every field is a plausible lowercase hex SHA-256.
    ///
    /// Format-checked, not merely non-empty: a truncated or uppercase digest
    /// would produce a different `profileId` for identical weights, splitting
    /// one embedding space into two.
    pub fn are_well_formed(&self) -> bool {
        let is_sha256 = |value: &str| {
            value.len() == 64
                && value
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        };
        !self.weights.is_empty()
            && self.weights.iter().all(|w| is_sha256(w))
            && is_sha256(&self.tokenizer)
            && is_sha256(&self.tokenizer_config)
    }
}

/// The full identity a vector is published against.
///
/// Nine fields, all mandatory. `matches` compares every one of them — the
/// profile is part of a job's identity. [`Self::predicate`] compares the
/// seven freshness fields **and** the exact profile: admission assigns the
/// target profile to the row before any compute exists, so a completion can
/// and must require it. [`Self::freshness_predicate`] is the profile-free
/// form, and admission alone uses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub index_epoch: u64,
    pub checkout_id: String,
    pub canonical_key: String,
    pub incarnation: u64,
    pub row_revision: u64,
    pub source_digest: String,
    pub input_digest: String,
    pub profile_id: String,
}

impl Binding {
    /// Whether `job` was created for exactly this binding.
    ///
    /// All nine components. A display id is deliberately absent: two
    /// incarnations of the same artifact share it, which is the case the
    /// whole mechanism exists to isolate.
    pub fn matches(&self, job: &EmbeddingJob) -> bool {
        job.index_epoch == self.index_epoch
            && job.checkout_id == self.checkout_id
            && job.canonical_key == self.canonical_key
            && job.incarnation == self.incarnation
            && job.row_revision == self.row_revision
            && job.source_digest == self.source_digest
            && job.input_digest == self.input_digest
            && job.profile_id == self.profile_id
    }

    /// The conditional guard for a vector publish, as a bounded predicate.
    ///
    /// This is the whole mechanism, and it is deliberately a predicate on the
    /// **row's own stored binding** rather than a check the caller performs
    /// beforehand. A caller-supplied guard is not conditional: it is a
    /// suggestion, and the one thing a stale completion needs is for someone
    /// else to have decided the row still matches. Here the row decides.
    ///
    /// Eight comparisons, `AND`-ed — the freshness identity of the row plus
    /// the **exact** profile. If any one differs — a new incarnation after a
    /// delete/recreate, a new `row_revision` after an edit, a different
    /// `source_digest` or `input_digest`, or a profile that is not the one
    /// this job was admitted under — the predicate matches nothing and the
    /// update affects **zero rows**, so a stale completion can neither insert
    /// nor revive.
    ///
    /// # Why `profile_id` is compared exactly, with no `OR … IS NULL`
    ///
    /// The profile used to be the one field left out of the guard, because a
    /// row bound to an older space has to be able to adopt the current one.
    /// That concession required an `OR profile_id IS NULL`-shaped hole, and a
    /// hole in a guard is a hole a late result walks through: a vector
    /// computed under profile A could land on a row admission had already
    /// re-pointed at profile B.
    ///
    /// It is no longer needed. **Admission now assigns the target profile to
    /// the row first**, atomically, under the owner, using
    /// [`Self::freshness_predicate`] — the same seven freshness comparisons
    /// with no profile term. By the time any compute exists, the row already
    /// stores the profile its job names, so an exact equality is both correct
    /// and sufficient. A late A after a B admission matches zero rows.
    ///
    /// # Escaping
    ///
    /// Single quotes are doubled and the value is delimited. No caller text
    /// is ever concatenated unescaped: a predicate is the one place where a
    /// quote in the data becomes structure in the query.
    pub fn predicate(&self) -> String {
        format!(
            "{} AND profile_id = {profile}",
            self.freshness_predicate(),
            profile = Self::quote(&self.profile_id),
        )
    }

    /// The guard **without** the profile term — the freshness identity alone.
    ///
    /// Used by admission, which is the one write that must *change* a row's
    /// profile: it matches the row on the seven freshness fields and then
    /// assigns the target profile in the same statement. Refusing to use this
    /// anywhere else is deliberate — every completion goes through
    /// [`Self::predicate`].
    pub fn freshness_predicate(&self) -> String {
        format!(
            "canonical_key = {key} \
             AND index_epoch = {epoch} \
             AND checkout_id = {checkout} \
             AND incarnation = {incarnation} \
             AND row_revision = {revision} \
             AND source_digest = {source} \
             AND input_digest = {input}",
            key = Self::quote(&self.canonical_key),
            epoch = self.index_epoch,
            checkout = Self::quote(&self.checkout_id),
            incarnation = self.incarnation,
            revision = self.row_revision,
            source = Self::quote(&self.source_digest),
            input = Self::quote(&self.input_digest),
        )
    }

    /// A single-quoted SQL literal with embedded quotes doubled.
    fn quote(value: &str) -> String {
        format!("'{}'", value.replace('\'', "''"))
    }
}

/// The binding a row currently *stores*.
///
/// A row that has never been embedded has no binding, so the fields are
/// optional. That is not the same as a row whose binding is empty or
/// unparseable, and [`Eligibility`] keeps the two apart.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoredBinding {
    pub index_epoch: Option<u64>,
    pub checkout_id: Option<String>,
    pub incarnation: Option<u64>,
    pub row_revision: Option<u64>,
    pub source_digest: Option<String>,
    pub input_digest: Option<String>,
    pub profile_id: Option<String>,
    pub semantic_state: Option<String>,
}

/// Whether a row's vector is current, needs computing, or cannot be judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Eligibility {
    /// The stored binding equals the capture's and the state is `ready`.
    Current,
    /// The binding differs, or the state is not `ready`. This row's vector
    /// needs computing.
    Eligible,
    /// A required field is absent or unreadable, so eligibility is **unknown**.
    ///
    /// Distinct from [`Eligibility::Eligible`] and it must stay distinct:
    /// collapsing "cannot judge" into "needs work" turns a partial source into
    /// a work list that looks complete, and collapsing it into "current" turns
    /// it into a result that looks answered.
    CannotJudge { reason: String },
}

/// Why this question is answered with a **scalar scan**, not a vector search.
///
/// Eligibility is equality against stored values — nine binding fields plus
/// `semantic_state`. It is not similarity. The vector API would answer the
/// wrong question twice over:
///
/// * `bypass_vector_index` (`lancedb-0.27.2/src/query.rs:1184`) controls
///   whether an *ANN index* is consulted during a nearest-neighbour query.
///   There is no index to bypass here, because nothing is being searched for.
/// * A nearest-neighbour query returns the K **closest** rows. Asked "which
///   rows match this binding exactly", it returns rows that merely resemble
///   it — and under a limit, an eligible row could be crowded out by stale
///   ones that happen to be nearer. That is L15's failure mode, introduced
///   into the component meant to prevent it.
///
/// So eligibility is a scalar prefilter over stored columns, evaluated
/// before any vector work and before any limit. `postfilter` and
/// `fast_search` are excluded for the same reason the SPEC excludes them.
pub fn eligibility_for(stored: &StoredBinding, captured: &Binding) -> Eligibility {
    let missing = |field: &str| Eligibility::CannotJudge {
        reason: format!("the row stores no `{field}`"),
    };

    let (Some(index_epoch), Some(checkout_id), Some(incarnation), Some(row_revision)) = (
        stored.index_epoch,
        stored.checkout_id.as_deref(),
        stored.incarnation,
        stored.row_revision,
    ) else {
        return missing("binding");
    };
    let (Some(source_digest), Some(input_digest)) = (
        stored.source_digest.as_deref(),
        stored.input_digest.as_deref(),
    ) else {
        return missing("digest");
    };
    let (profile_id, semantic_state) = (
        stored.profile_id.as_deref(),
        stored.semantic_state.as_deref(),
    );
    let (Some(profile_id), Some(semantic_state)) = (profile_id, semantic_state) else {
        // The publisher's documented pre-model shape: `profile_id` NULL +
        // `semantic_state = "unavailable"` is written for every row while no
        // model exists (artifacts_batch comment, publisher.rs). There is no
        // stored binding to disagree with, so the row needs a vector — it is
        // work, not an unknown. Folding it into CannotJudge would park
        // pre-model rows forever, which is exactly what happened in the live
        // drain check. Every other missing combination is a torn write —
        // a `ready` row without a profile, or a profile without a state —
        // and stays CannotJudge.
        return if stored.profile_id.is_none() && semantic_state == Some("unavailable") {
            Eligibility::Eligible
        } else {
            missing("profile or semantic state")
        };
    };

    // Every one of the nine, or the row's vector describes a different
    // binding than the one we captured.
    if index_epoch != captured.index_epoch
        || checkout_id != captured.checkout_id
        || incarnation != captured.incarnation
        || row_revision != captured.row_revision
        || source_digest != captured.source_digest
        || input_digest != captured.input_digest
        || profile_id != captured.profile_id
    {
        return Eligibility::Eligible;
    }

    match semantic_state {
        "ready" => Eligibility::Current,
        "pending" | "failed" | "unavailable" => Eligibility::Eligible,
        other => Eligibility::CannotJudge {
            reason: format!("unknown semantic state `{other}`"),
        },
    }
}

/// The eligibility of every captured row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EligibilityReport {
    rows: std::collections::BTreeMap<String, Eligibility>,
}

impl EligibilityReport {
    pub fn record(&mut self, canonical_key: impl Into<String>, eligibility: Eligibility) {
        self.rows.insert(canonical_key.into(), eligibility);
    }

    /// Fold another report's rows into this one.
    ///
    /// The artifact and section scans are separate tables judged under one
    /// key namespace — artifact keys are bare, section keys are
    /// `"{parent}#s{ordinal}"` — so the merge is disjoint by construction,
    /// not by convention: a collision would mean a section row was minted
    /// under a bare artifact key, which `section_key` cannot produce.
    pub fn merge_from(&mut self, other: &Self) {
        for (key, eligibility) in &other.rows {
            self.rows.insert(key.clone(), eligibility.clone());
        }
    }

    /// Keys whose vector is current. Safe to skip.
    pub fn current_keys(&self) -> Vec<&str> {
        self.keys_matching(|e| matches!(e, Eligibility::Current))
    }

    /// Keys that need a vector computed. **This is the work list.**
    pub fn eligible_keys(&self) -> Vec<&str> {
        self.keys_matching(|e| matches!(e, Eligibility::Eligible))
    }

    /// Keys whose eligibility could not be judged, with the reason.
    ///
    /// Reported separately and never folded into either list above: an
    /// unreadable row is not work to do, and it is not a row that is current.
    pub fn cannot_judge(&self) -> Vec<(&str, &str)> {
        self.rows
            .iter()
            .filter_map(|(key, e)| match e {
                Eligibility::CannotJudge { reason } => Some((key.as_str(), reason.as_str())),
                _ => None,
            })
            .collect()
    }

    /// Whether every row's eligibility was decided.
    pub fn is_complete(&self) -> bool {
        self.cannot_judge().is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    fn keys_matching(&self, predicate: impl Fn(&Eligibility) -> bool) -> Vec<&str> {
        self.rows
            .iter()
            .filter(|(_, e)| predicate(e))
            .map(|(key, _)| key.as_str())
            .collect()
    }
}

/// What one conditional vector publish did.
///
/// `stale` is a **result, not a failure**. A completion whose row moved
/// matches nothing and updates zero rows; reporting that as an error would
/// train callers to treat the normal, expected case as a fault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingPublish {
    /// The binding the publish was attempted against.
    pub binding: Binding,
    /// Rows the guarded update actually affected.
    pub rows_updated: u64,
    /// The version after the operation. Never `0`.
    pub version: u64,
    /// True when the guard matched nothing and nothing was written.
    pub stale: bool,
}

impl EmbeddingPublish {
    /// Whether a vector is now current for this binding.
    pub fn published(&self) -> bool {
        !self.stale && self.rows_updated > 0
    }
}

/// Which table one prepared job's row lives in.
///
/// Decided from the key's shape, never from a caller: the two namespaces are
/// disjoint **by construction** — [`section_key`] is the only producer of a
/// `"{parent}#s{ordinal}"` key, and capture mints it only for a section that
/// exists. [`EmbeddingJob::row_role`] is the single place that decision is
/// made, so a result cannot be routed onto the wrong table by a call site
/// guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingRowRole {
    /// One row of `constellation_artifacts_v1`.
    Artifact,
    /// One row of `constellation_sections_v1`.
    Section,
}

/// The parent canonical key a section job's row belongs to, when `key` is one.
///
/// `None` for a bare artifact key. Only the exact `"#s"` separator followed by
/// a decimal ordinal counts: `"X#notes"` and `"X#s"` are not section keys and
/// must not be routed to the sections table.
pub fn section_parent_key(canonical_key: &str) -> Option<&str> {
    let (parent, suffix) = canonical_key.rsplit_once("#s")?;
    if parent.is_empty() || suffix.is_empty() || !suffix.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(parent)
}

/// One prepared unit of embedding work.
///
/// The pair is a unit because they were minted together under one owner from
/// one capture: the job names the exact binding the vector may be published
/// against, and `input_text` is the exact string that binding's `input_digest`
/// was computed over. Separating them is what lets a completion rebind a
/// vector to bytes a later capture happened to contain.
///
/// `Arc<str>`, not `String`: a prepared batch is handed to an inference thread
/// and comes back, and the whole corpus is never copied into one job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedEmbeddingInput {
    pub job: EmbeddingJob,
    pub input_text: std::sync::Arc<str>,
}

/// Everything one admission pass froze, and nothing that could be recomputed
/// differently later.
///
/// `admission_generation` and `config_digest` are the two facts a completion
/// re-checks before writing anything: they are what says *this* batch belongs
/// to *that* capture of *that* configuration. `profile` is the verified space
/// the vectors are being computed in — never a bare id, so a completion cannot
/// be handed a profile it did not resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedEmbeddingBatch {
    /// The published generation this batch was admitted under.
    pub admission_generation: String,
    /// The config digest the capture that produced these inputs saw.
    pub config_digest: String,
    /// The verified profile every vector in this batch is computed in.
    pub profile: EmbeddingProfile,
    pub inputs: Vec<PreparedEmbeddingInput>,
}

/// One computed vector, still carrying the job it was computed for.
///
/// The job travels with the vector rather than a key, because the key alone
/// cannot tell a late result from a current one: two admissions for the same
/// canonical key under different profiles produce two vectors with one name,
/// and only the job says which one this is.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedEmbedding {
    pub job: EmbeddingJob,
    pub vector: Vec<f32>,
}

impl PreparedEmbeddingBatch {
    /// The job admitted for `canonical_key`, if this batch holds one.
    pub fn job_for(&self, canonical_key: &str) -> Option<&EmbeddingJob> {
        self.inputs
            .iter()
            .find(|input| input.job.canonical_key == canonical_key)
            .map(|input| &input.job)
    }

    /// Whether `job` is exactly the job this batch prepared for `key`.
    ///
    /// Field equality, not key equality. A result whose job names a different
    /// incarnation, row revision, source or input digest — or a different
    /// `captured_input_ref` — was computed for a state this batch never
    /// admitted, and publishing it would rebind a row.
    pub fn accepts(&self, key: &str, job: &EmbeddingJob) -> bool {
        self.job_for(key).is_some_and(|prepared| prepared == job)
    }
}

impl ComputedEmbedding {
    /// Which table this result's row lives in.
    pub fn row_role(&self) -> EmbeddingRowRole {
        if section_parent_key(&self.job.canonical_key).is_some() {
            EmbeddingRowRole::Section
        } else {
            EmbeddingRowRole::Artifact
        }
    }

    /// The binding this vector is published against — from the job, never from
    /// a capture taken after the compute.
    pub fn binding(&self) -> Binding {
        self.job.binding()
    }
}

#[cfg(test)]
mod profile_tests {
    use super::*;

    fn evidence(root: PathBuf, revision: &str) -> SnapshotEvidence {
        SnapshotEvidence {
            revision: revision.to_string(),
            root,
            present_files: REQUIRED_SNAPSHOT_FILES
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            missing_files: Vec::new(),
        }
    }

    fn profile_with(revision: &str) -> EmbeddingProfile {
        evidence(PathBuf::from("/cache"), revision)
            .to_profile(
                vec!["weights-sha".to_string()],
                "tok-sha".to_string(),
                "tokcfg-sha".to_string(),
                "tract-onnx-0.23",
            )
            .unwrap()
    }

    #[test]
    fn a_profile_carries_the_revision_read_from_disk_not_the_repo_name() {
        let profile = profile_with("5617a9f61b028005a4858fdac845db406aefb181");
        assert_eq!(
            profile.weights_revision, "5617a9f61b028005a4858fdac845db406aefb181",
            "the revision comes from `refs/main`, not from `BAAI/bge-m3`"
        );
        assert_eq!(profile.model_repo, DEFAULT_MODEL_REPO);
        assert_ne!(
            profile.weights_revision, profile.model_repo,
            "a repo name must never stand in for a snapshot revision"
        );
    }

    #[test]
    fn the_initial_profile_declares_the_verified_prefix_configuration() {
        let profile = profile_with("rev");
        assert_eq!(profile.dimensions, DEFAULT_DIMENSIONS);
        assert_eq!(profile.max_tokens, DEFAULT_MAX_TOKENS);
        assert_eq!(profile.pooling, "cls");
        assert_eq!(profile.normalization, "l2");
        assert_eq!(profile.query_prefix, "");
        assert_eq!(profile.document_prefix, "");
        assert_eq!(profile.body_prefix_chars, DEFAULT_BODY_PREFIX_CHARS);
    }

    #[test]
    fn two_revisions_of_the_same_repo_are_different_profiles() {
        assert_ne!(
            profile_with("rev-a").profile_id(),
            profile_with("rev-b").profile_id(),
            "vectors built from different weights are different spaces"
        );
    }

    #[test]
    fn a_profile_is_unresolved_without_real_checksums() {
        let evidence = evidence(PathBuf::from("/cache"), "rev");
        assert!(
            !evidence.checksums_resolved(&[], "", ""),
            "with no weights or tokenizer digests the profile cannot identify \
             what it would load, so it stays unresolved"
        );
        assert!(evidence.checksums_resolved(&["weights-sha".to_string()], "tok", "tokcfg"));
    }

    #[test]
    fn an_incomplete_snapshot_is_model_unavailable_not_a_guessed_profile() {
        let mut incomplete = evidence(PathBuf::from("/cache"), "rev");
        incomplete.missing_files = vec!["onnx/model.onnx".to_string()];
        assert!(!incomplete.is_complete());
        let err = incomplete
            .to_profile(vec!["w".into()], "t".into(), "c".into(), "engine")
            .unwrap_err();
        assert_eq!(err.code(), super::super::DiagnosticCode::ModelUnavailable);
    }

    #[test]
    fn a_missing_cache_directory_is_no_model_not_an_error() {
        assert!(
            SnapshotEvidence::read(Path::new("/definitely/not/here"), DEFAULT_MODEL_REPO).is_none(),
            "an absent cache is `model_unavailable`, not a failure"
        );
    }

    #[test]
    fn semantic_scope_is_reported_as_prefix_not_whole_document() {
        assert_eq!(SEMANTIC_SCOPE, "prefix");
        assert_ne!(SEMANTIC_SCOPE, "document");
        assert_ne!(SEMANTIC_SCOPE, "section");
    }

    #[test]
    fn zero_pending_after_work_is_not_the_same_as_zero_pending_without_work() {
        let ran = SemanticOutcome {
            staged: 12,
            pending: 0,
            ready: 12,
            ..Default::default()
        };
        let never_ran = SemanticOutcome::default();
        assert_eq!(ran.pending, never_ran.pending, "both are zero");
        assert_ne!(
            ran.staged, never_ran.staged,
            "`staged` is what distinguishes nothing-ran from nothing-eligible"
        );
        assert!(ran.is_semantically_covered());
    }

    #[test]
    fn pending_or_failed_means_the_result_is_not_semantically_covered() {
        let pending = SemanticOutcome {
            pending: 1,
            staged: 3,
            ..Default::default()
        };
        let failed = SemanticOutcome {
            failed: 1,
            staged: 3,
            ..Default::default()
        };
        assert!(!pending.is_semantically_covered());
        assert!(!failed.is_semantically_covered());
    }

    fn job_for(binding: &Binding) -> EmbeddingJob {
        EmbeddingJob {
            index_epoch: binding.index_epoch,
            checkout_id: binding.checkout_id.clone(),
            canonical_key: binding.canonical_key.clone(),
            incarnation: binding.incarnation,
            row_revision: binding.row_revision,
            source_digest: binding.source_digest.clone(),
            input_digest: binding.input_digest.clone(),
            profile_id: binding.profile_id.clone(),
            captured_input_ref: "prds/PRD-001-auth.md".to_string(),
            attempt: 0,
            retry_after: None,
        }
    }

    fn binding() -> Binding {
        Binding {
            index_epoch: 3,
            checkout_id: "local-a".to_string(),
            canonical_key: "PRD-001".to_string(),
            incarnation: 2,
            row_revision: 7,
            source_digest: "sd".to_string(),
            input_digest: "id".to_string(),
            profile_id: "pid".to_string(),
        }
    }

    #[test]
    fn hashing_the_real_snapshot_matches_the_measured_digests() {
        // Ground truth, measured independently by the coordinator on this
        // machine and recorded at
        // `.local/constellation-s1/reports/model-checksums.json`. Asserting
        // against it is a stronger claim than any other test here: it fails
        // if the streaming is wrong, if a `blobs/` symlink is not followed,
        // or if a chunk boundary is mishandled.
        let cache_dir = crate::embed::resolve_cache_dir();
        let Ok(checksums) = snapshot_checksums(&cache_dir, DEFAULT_MODEL_REPO) else {
            // No cached model on this machine. Skipped, not deleted, and no
            // fake digest is asserted in its place.
            eprintln!(
                "skipping: no cached snapshot for `{DEFAULT_MODEL_REPO}` under {}",
                cache_dir.display()
            );
            return;
        };
        assert_eq!(
            checksums.weights,
            vec![
                "f84251230831afb359ab26d9fd37d5936d4d9bb5d1d5410e66442f630f24435b",
                "1eebfb28493f67bba03ce0ef64bfdc7fc5a3bd9d7493f818bb1d78cd798416b4",
            ],
            "onnx/model.onnx and onnx/model.onnx_data, hashed through their blob symlinks"
        );
        assert_eq!(
            checksums.tokenizer,
            "21106b6d7dab2952c1d496fb21d5dc9db75c28ed361a05f5020bbba27810dd08"
        );
        assert_eq!(
            checksums.tokenizer_config,
            "a62b2b6784f990259fddef5f16388693a8043be4f69179e6a5257eeb3f9abac4"
        );
        assert_eq!(
            checksums.config,
            "26159e7ad065073448460117eb24b7a4572f6f4e78eadff65dc0a11c052449fa"
        );
        assert!(checksums.are_well_formed());
    }

    #[test]
    fn an_absent_cache_is_model_unavailable_not_a_digest_of_nothing() {
        let err =
            snapshot_checksums(Path::new("/definitely/not/here"), DEFAULT_MODEL_REPO).unwrap_err();
        assert_eq!(err.code(), super::super::DiagnosticCode::ModelUnavailable);
    }

    /// A fixture cache with every file a profile needs, whose *contents* are
    /// fixture bytes — no weights, no network, and no dependency on what this
    /// machine happens to have downloaded.
    #[cfg(feature = "semantic-search")]
    fn fixture_cache(root: &Path) -> PathBuf {
        let repo_dir = root.join("models--BAAI--bge-m3");
        std::fs::create_dir_all(repo_dir.join("refs")).unwrap();
        std::fs::write(repo_dir.join("refs").join("main"), "rev1\n").unwrap();
        let snapshot = repo_dir.join("snapshots").join("rev1");
        std::fs::create_dir_all(snapshot.join("onnx")).unwrap();
        for relative in [
            "config.json",
            "tokenizer.json",
            "tokenizer_config.json",
            "onnx/model.onnx",
            "onnx/model.onnx_data",
        ] {
            std::fs::write(snapshot.join(relative), format!("fixture:{relative}")).unwrap();
        }
        root.to_path_buf()
    }

    /// A query against an index that pinned a different profile must refuse,
    /// not answer.
    ///
    /// Both revisions produce 1024-float unit vectors, so nothing downstream can
    /// detect the mismatch — which is why the refusal has to happen here. Breaks
    /// if the comparison is dropped, or if it happens *after* the engine is
    /// built (the error would then be an ONNX parse failure that says nothing
    /// about profiles).
    #[cfg(feature = "semantic-search")]
    #[test]
    fn a_query_refuses_when_the_cached_snapshot_is_not_the_pinned_profile() {
        let cache = tempfile::TempDir::new().unwrap();
        let root = fixture_cache(cache.path());

        // The id the fixture cache really resolves to, so the only thing wrong
        // with this call is the expectation it is given.
        let real = ResolvedProfile::open(
            &root,
            DEFAULT_MODEL_REPO,
            crate::constellation::facade::ENGINE_VERSION,
            512,
        )
        .unwrap()
        .profile
        .profile_id();

        let err =
            open_cached_query_engine_in(&root, DEFAULT_MODEL_NAME, Some("pinned-elsewhere"), 512)
                .err()
                .expect("a mismatched profile must refuse");
        let detail = err.to_string();
        assert!(
            detail.contains("pinned-elsewhere") && detail.contains(&real),
            "the refusal must name both profiles, was: {detail}"
        );

        // And the same call with the profile the cache actually resolves to
        // gets past this gate — it fails later, on engine construction, which
        // proves the mismatch check is what refused above.
        let later = open_cached_query_engine_in(&root, DEFAULT_MODEL_NAME, Some(&real), 512)
            .err()
            .expect("fixture bytes are not a loadable model");
        assert!(
            !later.to_string().contains("pinned"),
            "a matching profile must not fail on the profile comparison: {}",
            later
        );
    }

    /// The resolved snapshot identity and the profile come from **one** read of the
    /// cache, and the profile's own prefix budget is the one it was opened with.
    ///
    /// This is the property `open`'s signature exists for. Before it, a caller
    /// read the checksums, then handed them back to `open`, which read the
    /// snapshot again: two lookups of `refs/main`, and a profile assembled from
    /// a `revision` read at a different moment than the digests.
    ///
    /// Breaks if `snapshot_root`/`snapshot_id` stop being the root the
    /// checksums were measured from, or if `body_prefix_chars` is ignored — the
    /// second failure being the one that publishes vectors from a 2000-char
    /// prefix under a `profile_id` that claims 512.
    #[cfg(feature = "semantic-search")]
    #[test]
    fn a_resolved_profile_names_the_exact_snapshot_its_digests_were_read_from() {
        let cache = tempfile::TempDir::new().unwrap();
        let root = fixture_cache(cache.path());
        let repo_dir = root.join("models--BAAI--bge-m3");

        let resolved = ResolvedProfile::open(
            &root,
            DEFAULT_MODEL_REPO,
            crate::constellation::facade::ENGINE_VERSION,
            512,
        )
        .unwrap();

        // The reported root is the one `refs/main` points at, by construction
        // rather than by a second lookup that could disagree.
        assert_eq!(resolved.snapshot_id, "rev1");
        assert_eq!(
            resolved.snapshot_root,
            repo_dir.join("snapshots").join("rev1"),
            "the engine must open the directory the digests were measured from"
        );
        assert!(
            resolved.snapshot_root.join("onnx/model.onnx_data").exists(),
            "the reported root is a real directory in this cache, not a name"
        );

        // The checksums the profile carries are the checksums of those files.
        let checksums = snapshot_checksums(&root, DEFAULT_MODEL_REPO).unwrap();
        assert_eq!(
            resolved.profile.weights_checksums, checksums.weights,
            "resolved weights must hash the snapshot `open` reported"
        );
        assert_eq!(resolved.profile.tokenizer_checksum, checksums.tokenizer);
        assert_eq!(
            resolved.profile.tokenizer_config_checksum,
            checksums.tokenizer_config
        );
        assert_eq!(
            resolved.profile.weights_revision, "rev1",
            "the revision comes from the same read, not from the repo name"
        );

        // The prefix budget is the caller's, and it is part of the identity.
        assert_eq!(resolved.profile.body_prefix_chars, 512);
        let wide = ResolvedProfile::open(
            &root,
            DEFAULT_MODEL_REPO,
            crate::constellation::facade::ENGINE_VERSION,
            4096,
        )
        .unwrap();
        assert_eq!(wide.profile.body_prefix_chars, 4096);
        assert_ne!(
            wide.profile.profile_id(),
            resolved.profile.profile_id(),
            "a changed chunk size is a different embedding space and must not share its id"
        );
    }

    /// A resolved profile is a **snapshot of one moment**, and the worker built
    /// from it stays on that snapshot.
    ///
    /// `spawn_resolved` is the only automatic constructor, and it exists to
    /// avoid a second `find_snapshot`. If it went back to resolving the repo
    /// itself, a `refs/main` that moved between resolution and spawn would load
    /// different weights under the advertised profile — and nothing downstream
    /// could tell, because both are 1024 unit-length float vectors.
    ///
    /// The engine is built lazily on the first job, so the observable is what
    /// that job does with bytes that are not a model: it **fails**, loudly and
    /// on its own thread. It never returns a zero vector, never starts a
    /// download, and never publishes — the three ways an automatic path could
    /// turn a moved cache into a corrupted index.
    #[cfg(feature = "semantic-search")]
    #[test]
    fn a_worker_built_from_a_resolved_profile_stays_on_that_snapshot_and_never_answers_with_a_vector()
     {
        use crate::constellation::runtime::{InferenceJob, InferenceWorker, Poll};

        let cache = tempfile::TempDir::new().unwrap();
        let root = fixture_cache(cache.path());
        let resolved = ResolvedProfile::open(
            &root,
            DEFAULT_MODEL_REPO,
            crate::constellation::facade::ENGINE_VERSION,
            DEFAULT_BODY_PREFIX_CHARS,
        )
        .unwrap();
        let profile_id = resolved.profile.profile_id();
        let resolved_root = resolved.snapshot_root.clone();

        // Move `refs/main` after resolution, exactly as a background download
        // would. A constructor that re-resolved would now be pointed at a
        // revision that was never resolved, profiled or checksummed.
        let repo_dir = root.join("models--BAAI--bge-m3");
        std::fs::write(repo_dir.join("refs").join("main"), "rev2\n").unwrap();
        assert!(
            !repo_dir.join("snapshots").join("rev2").exists(),
            "rev2 has no files: a re-resolved worker would fail on a missing file, not on bad bytes"
        );

        let worker = InferenceWorker::spawn_resolved(DEFAULT_MODEL_NAME, &resolved)
            .expect("thread creation is not model loading");

        let submitted = worker.submit(InferenceJob {
            job: crate::constellation::jobs::EmbeddingJob {
                index_epoch: 1,
                checkout_id: "local-test".to_string(),
                canonical_key: "PRD-001".to_string(),
                incarnation: 1,
                row_revision: 1,
                source_digest: "s".to_string(),
                input_digest: "i".to_string(),
                profile_id,
                captured_input_ref: "rev1:PRD-001".to_string(),
                // First attempt, no backoff — the same shape
                // `EmbeddingJob::from_binding` produces for a fresh admission.
                attempt: 0,
                retry_after: None,
            },
            input_text: "title body".to_string(),
        });
        assert_eq!(
            submitted,
            crate::constellation::runtime::SubmitOutcome::Queued,
            "the job must reach the worker; a refusal here would hide the engine failure"
        );

        let mut deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let reason = loop {
            match worker.try_failure() {
                Poll::Ready(detail) => break detail,
                Poll::Disconnected => {
                    break "the worker disconnected without saying why".to_string();
                }
                Poll::Empty => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "the worker neither produced a vector nor reported a failure"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
            }
        };

        assert!(
            reason.contains("could not be constructed"),
            "fixture bytes are not a model and the worker must say so, not answer: {reason}"
        );

        // Nothing may have come back as a result. A zero vector here would be
        // published onto a row and read as "this document has no semantic
        // content", which is a false negative nothing downstream can detect.
        assert!(
            matches!(worker.try_result(), Poll::Empty | Poll::Disconnected),
            "no vector may be produced from bytes that are not a model"
        );

        // The profile still names the revision it resolved, whatever the cache
        // has done since — it is a moment, not a live view of `refs/main`.
        assert_eq!(resolved.snapshot_id, "rev1");
        assert!(
            resolved_root.ends_with("snapshots/rev1"),
            "the resolved root is the revision directory, not the repo: {resolved_root:?}"
        );
        assert!(
            resolved.profile.weights_checksums
                != snapshot_checksums(&root, DEFAULT_MODEL_REPO)
                    .unwrap_or_default()
                    .weights,
            "rev2 has no files, so a fresh read of the cache cannot reproduce rev1's digests; \
             the profile was not re-read"
        );

        let _ = worker.shutdown();
    }

    /// An absent or unsupported model is `unavailable` with a detail that says
    /// which — never a download, never a panic.
    #[cfg(feature = "semantic-search")]
    #[test]
    fn a_query_with_no_usable_cache_is_model_unavailable_and_says_why() {
        let empty = tempfile::TempDir::new().unwrap();
        let err = open_cached_query_engine_in(
            empty.path(),
            DEFAULT_MODEL_NAME,
            None,
            DEFAULT_BODY_PREFIX_CHARS,
        )
        .err()
        .expect("an empty cache must not open a query engine");
        assert_eq!(err.code(), super::super::DiagnosticCode::ModelUnavailable);
        assert!(
            err.to_string().contains("no cached snapshot"),
            "absence must read as absence, was: {err}"
        );

        let err = open_cached_query_engine_in(
            empty.path(),
            "not-a-model",
            None,
            DEFAULT_BODY_PREFIX_CHARS,
        )
        .err()
        .expect("an unsupported model must not open");
        assert!(
            err.to_string().contains("unsupported embedding model"),
            "was: {err}"
        );
    }

    #[test]
    fn a_row_published_before_the_model_is_eligible_not_stuck() {
        // The publisher's pre-model shape: binding + digests set,
        // profile_id NULL, semantic_state "unavailable". Such a row has no
        // vector at all — it must enter the work list on the next drain,
        // not park in CannotJudge forever.
        let b = binding();
        let stored = StoredBinding {
            index_epoch: Some(b.index_epoch),
            checkout_id: Some(b.checkout_id.clone()),
            incarnation: Some(b.incarnation),
            row_revision: Some(b.row_revision),
            source_digest: Some(b.source_digest.clone()),
            input_digest: Some(b.input_digest.clone()),
            profile_id: None,
            semantic_state: Some("unavailable".to_string()),
        };
        assert_eq!(eligibility_for(&stored, &b), Eligibility::Eligible);
    }

    #[test]
    fn a_torn_profile_state_pair_stays_cannot_judge() {
        let b = binding();
        for semantic_state in [Some("ready"), None] {
            let stored = StoredBinding {
                index_epoch: Some(b.index_epoch),
                checkout_id: Some(b.checkout_id.clone()),
                incarnation: Some(b.incarnation),
                row_revision: Some(b.row_revision),
                source_digest: Some(b.source_digest.clone()),
                input_digest: Some(b.input_digest.clone()),
                profile_id: None,
                semantic_state: semantic_state.map(String::from),
            };
            assert!(
                matches!(
                    eligibility_for(&stored, &b),
                    Eligibility::CannotJudge { .. }
                ),
                "profile NULL + state {semantic_state:?} is a torn write, not work"
            );
        }
    }

    #[test]
    fn a_stored_binding_equal_to_the_capture_is_current() {
        let b = binding();
        let stored = StoredBinding {
            index_epoch: Some(b.index_epoch),
            checkout_id: Some(b.checkout_id.clone()),
            incarnation: Some(b.incarnation),
            row_revision: Some(b.row_revision),
            source_digest: Some(b.source_digest.clone()),
            input_digest: Some(b.input_digest.clone()),
            profile_id: Some(b.profile_id.clone()),
            semantic_state: Some("ready".to_string()),
        };
        assert_eq!(eligibility_for(&stored, &b), Eligibility::Current);
    }

    #[test]
    fn each_differing_binding_field_makes_a_row_eligible() {
        let b = binding();
        let ready = StoredBinding {
            index_epoch: Some(b.index_epoch),
            checkout_id: Some(b.checkout_id.clone()),
            incarnation: Some(b.incarnation),
            row_revision: Some(b.row_revision),
            source_digest: Some(b.source_digest.clone()),
            input_digest: Some(b.input_digest.clone()),
            profile_id: Some(b.profile_id.clone()),
            semantic_state: Some("ready".to_string()),
        };
        let variants = [
            StoredBinding {
                index_epoch: Some(99),
                ..ready.clone()
            },
            StoredBinding {
                checkout_id: Some("other".into()),
                ..ready.clone()
            },
            StoredBinding {
                incarnation: Some(99),
                ..ready.clone()
            },
            StoredBinding {
                row_revision: Some(99),
                ..ready.clone()
            },
            StoredBinding {
                source_digest: Some("other".into()),
                ..ready.clone()
            },
            StoredBinding {
                input_digest: Some("other".into()),
                ..ready.clone()
            },
            StoredBinding {
                profile_id: Some("other".into()),
                ..ready
            },
        ];
        for (index, stored) in variants.iter().enumerate() {
            assert_eq!(
                eligibility_for(stored, &b),
                Eligibility::Eligible,
                "binding field {index} differs, so the vector describes another binding"
            );
        }
    }

    #[test]
    fn a_matching_binding_that_is_not_ready_still_needs_a_vector() {
        let b = binding();
        for state in ["pending", "failed", "unavailable"] {
            let stored = StoredBinding {
                index_epoch: Some(b.index_epoch),
                checkout_id: Some(b.checkout_id.clone()),
                incarnation: Some(b.incarnation),
                row_revision: Some(b.row_revision),
                source_digest: Some(b.source_digest.clone()),
                input_digest: Some(b.input_digest.clone()),
                profile_id: Some(b.profile_id.clone()),
                semantic_state: Some(state.to_string()),
            };
            assert_eq!(
                eligibility_for(&stored, &b),
                Eligibility::Eligible,
                "`{state}` is not a current vector"
            );
        }
    }

    #[test]
    fn an_unreadable_row_is_cannot_judge_not_eligible_and_not_current() {
        let b = binding();
        assert!(matches!(
            eligibility_for(&StoredBinding::default(), &b),
            Eligibility::CannotJudge { .. }
        ));
        let stored = StoredBinding {
            index_epoch: Some(b.index_epoch),
            checkout_id: Some(b.checkout_id.clone()),
            incarnation: Some(b.incarnation),
            row_revision: Some(b.row_revision),
            source_digest: Some(b.source_digest.clone()),
            input_digest: Some(b.input_digest.clone()),
            profile_id: Some(b.profile_id.clone()),
            semantic_state: Some("banana".to_string()),
        };
        assert!(matches!(
            eligibility_for(&stored, &b),
            Eligibility::CannotJudge { .. }
        ));
    }

    #[test]
    fn the_report_separates_current_eligible_and_unjudgeable_keys() {
        let mut report = EligibilityReport::default();
        report.record("PRD-001", Eligibility::Current);
        report.record("PRD-002", Eligibility::Eligible);
        report.record(
            "PRD-003",
            Eligibility::CannotJudge {
                reason: "unreadable".to_string(),
            },
        );

        assert_eq!(report.current_keys(), vec!["PRD-001"]);
        assert_eq!(report.eligible_keys(), vec!["PRD-002"]);
        assert_eq!(report.cannot_judge(), vec![("PRD-003", "unreadable")]);
        assert!(!report.is_complete());
        assert_eq!(report.len(), 3);
    }

    #[test]
    fn an_unjudgeable_row_never_appears_in_the_work_list() {
        let mut report = EligibilityReport::default();
        report.record(
            "PRD-001",
            Eligibility::CannotJudge {
                reason: "no input digest".to_string(),
            },
        );
        assert!(
            report.eligible_keys().is_empty(),
            "an unreadable row is not work to do; folding it into the work list \
             makes a partial source look like a complete one"
        );
        assert!(!report.is_complete());
    }

    #[test]
    fn a_job_matches_its_binding_and_nothing_else() {
        let b = binding();
        assert!(b.matches(&job_for(&b)));

        // Each field, changed alone, must break the match — this is the whole
        // point of carrying all nine.
        for mutate in [
            (|j: &mut EmbeddingJob| j.index_epoch += 1) as fn(&mut EmbeddingJob),
            |j: &mut EmbeddingJob| j.checkout_id = "local-b".into(),
            |j: &mut EmbeddingJob| j.canonical_key = "PRD-999".into(),
            |j: &mut EmbeddingJob| j.incarnation += 1,
            |j: &mut EmbeddingJob| j.row_revision += 1,
            |j: &mut EmbeddingJob| j.source_digest = "other".into(),
            |j: &mut EmbeddingJob| j.input_digest = "other".into(),
            |j: &mut EmbeddingJob| j.profile_id = "other".into(),
        ] {
            let mut job = job_for(&b);
            mutate(&mut job);
            assert!(
                !b.matches(&job),
                "a job differing in one binding field must not match: {job:?}"
            );
        }
    }

    #[test]
    fn a_zero_row_publish_is_stale_not_failed() {
        let b = binding();
        let stale = EmbeddingPublish {
            binding: b.clone(),
            rows_updated: 0,
            version: 4,
            stale: true,
        };
        assert!(!stale.published());
        assert!(stale.stale, "an untouched row is the answer, not an error");

        let written = EmbeddingPublish {
            rows_updated: 1,
            stale: false,
            ..stale
        };
        assert!(written.published());
    }

    #[test]
    fn the_publish_guard_covers_every_freshness_field_and_the_profile() {
        let predicate = binding().predicate();
        for column in [
            "canonical_key",
            "index_epoch",
            "checkout_id",
            "incarnation",
            "row_revision",
            "source_digest",
            "input_digest",
            "profile_id",
        ] {
            assert!(
                predicate.contains(&format!("{column} = ")),
                "the guard must compare `{column}`: {predicate}"
            );
        }
        assert_eq!(
            predicate.matches(" AND ").count(),
            7,
            "eight comparisons joined by seven ANDs"
        );
        assert!(
            !predicate.contains(" OR "),
            "no disjunct may soften the guard; an OR is a hole a late result \
             walks through: {predicate}"
        );
        assert!(
            !predicate.contains("IS NULL"),
            "a NULL branch is the same hole spelled differently: {predicate}"
        );
    }

    #[test]
    fn the_freshness_guard_is_the_publish_guard_without_the_profile_term() {
        let b = binding();
        let freshness = b.freshness_predicate();
        assert!(!freshness.contains("profile_id"), "{freshness}");
        assert_eq!(
            b.predicate(),
            format!("{freshness} AND profile_id = '{}'", b.profile_id),
            "the publish guard is exactly the freshness guard plus the profile \
             term, so the two can never drift apart"
        );
    }

    #[test]
    fn the_publish_guard_escapes_quotes_in_every_text_field() {
        let mut b = binding();
        b.canonical_key = "O'Brien".to_string();
        b.checkout_id = "local'x".to_string();
        b.profile_id = "pro'file".to_string();
        let predicate = b.predicate();
        assert!(
            predicate.contains("'O''Brien'"),
            "a quote in the data must not become structure: {predicate}"
        );
        assert!(predicate.contains("'local''x'"));
        assert!(
            predicate.contains("'pro''file'"),
            "the profile term is data too: {predicate}"
        );
    }

    #[test]
    fn a_changed_incarnation_changes_the_guard_so_a_stale_publish_matches_nothing() {
        let before = binding();
        let after = Binding {
            incarnation: before.incarnation + 1,
            ..before.clone()
        };
        assert_ne!(
            before.predicate(),
            after.predicate(),
            "the guard is built from the binding, so a recreate cannot match \
             the pre-delete row"
        );
    }

    #[test]
    fn a_late_result_from_another_profile_matches_nothing() {
        // A and B both admitted the same key. B's admission re-pointed the row
        // at B, so A's completion — which still carries A's profile — must be
        // refused outright rather than migrating the row back.
        let before = binding();
        let after = Binding {
            profile_id: "profile-d".to_string(),
            ..before.clone()
        };
        assert_ne!(
            before.predicate(),
            after.predicate(),
            "a profile change must change the guard, or a late result under the \
             old profile lands on a row that has since moved"
        );
        assert_eq!(
            before.freshness_predicate(),
            after.freshness_predicate(),
            "the freshness half is genuinely unchanged — that is what lets \
             admission re-point a row without touching its freshness identity"
        );
        // A job minted under profile A is not the job a profile-B admission
        // issued, even when every freshness field agrees.
        assert!(!before.matches(&job_for(&after)));
    }

    #[test]
    fn a_recreated_artifact_is_isolated_by_incarnation_not_by_display_id() {
        let b = binding();
        // Same display id, new incarnation: the old job must not apply.
        let old = job_for(&b);
        let mut recreated = b.clone();
        recreated.incarnation = 3;
        assert_eq!(old.canonical_key, recreated.canonical_key);
        assert!(
            !recreated.matches(&old),
            "a recreate must isolate the old job"
        );
    }
}

/// The original `jobs` tests, kept as their own module so the profile
/// bindings above and the primitive contracts below stay separable.
#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> EmbeddingProfile {
        EmbeddingProfile {
            model_repo: "BAAI/bge-m3".to_string(),
            weights_revision: "unresolved".to_string(),
            weights_checksums: vec!["aaa".to_string(), "bbb".to_string()],
            tokenizer_checksum: "tok".to_string(),
            tokenizer_config_checksum: "tokcfg".to_string(),
            dimensions: 1024,
            engine_version: "tract-onnx-0.23".to_string(),
            pooling: "cls".to_string(),
            normalization: "l2".to_string(),
            query_prefix: String::new(),
            document_prefix: String::new(),
            body_prefix_chars: DEFAULT_BODY_PREFIX_CHARS,
            max_tokens: DEFAULT_MAX_TOKENS,
            padding: "none".to_string(),
            truncation: "longest_first".to_string(),
        }
    }

    #[test]
    fn profile_id_is_64_lowercase_hex() {
        let id = profile().profile_id();
        assert_eq!(id.len(), 64);
        assert!(
            id.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn profile_id_ignores_weights_checksum_order() {
        let mut p = profile();
        let a = p.profile_id();
        p.weights_checksums = vec!["bbb".to_string(), "aaa".to_string()];
        assert_eq!(a, p.profile_id());
    }

    #[test]
    fn profile_id_changes_when_any_field_changes() {
        let base = profile().profile_id();
        type ProfileMutator = Box<dyn Fn(&mut EmbeddingProfile)>;
        let mutators: Vec<ProfileMutator> = vec![
            Box::new(|p: &mut EmbeddingProfile| p.model_repo = "other".into()),
            Box::new(|p: &mut EmbeddingProfile| p.weights_revision = "v2".into()),
            Box::new(|p: &mut EmbeddingProfile| p.dimensions = 768),
            Box::new(|p: &mut EmbeddingProfile| p.pooling = "mean".into()),
            Box::new(|p: &mut EmbeddingProfile| p.normalization = "none".into()),
            Box::new(|p: &mut EmbeddingProfile| p.document_prefix = "passage: ".into()),
            Box::new(|p: &mut EmbeddingProfile| p.body_prefix_chars = 500),
            Box::new(|p: &mut EmbeddingProfile| p.max_tokens = 256),
            Box::new(|p: &mut EmbeddingProfile| p.tokenizer_checksum = "other".into()),
        ];
        for m in mutators {
            let mut p = profile();
            m(&mut p);
            assert_ne!(
                base,
                p.profile_id(),
                "profile change must change profile_id"
            );
        }
    }

    #[test]
    fn input_digest_ignores_body_beyond_the_prefix() {
        let long = "b".repeat(DEFAULT_BODY_PREFIX_CHARS + 500);
        let a = embedding_input_digest("T", &long, DEFAULT_BODY_PREFIX_CHARS);
        let b = embedding_input_digest("T", &long, DEFAULT_BODY_PREFIX_CHARS);
        assert_eq!(a, b);

        let short = "b".repeat(DEFAULT_BODY_PREFIX_CHARS - 1);
        let c = embedding_input_digest("T", &short, DEFAULT_BODY_PREFIX_CHARS);
        assert_ne!(a, c, "a shorter body is a different input");
    }

    #[test]
    fn input_digest_includes_the_title() {
        let a = embedding_input_digest("T1", "same body", 100);
        let b = embedding_input_digest("T2", "same body", 100);
        assert_ne!(a, b);
    }

    #[test]
    fn input_text_uses_title_space_prefix() {
        assert_eq!(embedding_input_text("Title", "body", 2000), "Title body");
        assert_eq!(embedding_input_text("Title", "", 2000), "Title ");
    }

    #[test]
    fn only_ready_is_query_eligible() {
        assert!(SemanticState::Ready.is_query_eligible());
        assert!(!SemanticState::Pending.is_query_eligible());
        assert!(!SemanticState::Failed.is_query_eligible());
        assert!(!SemanticState::Unavailable.is_query_eligible());
    }

    #[test]
    fn semantic_state_wire_strings_are_snake_case() {
        for (state, text) in [
            (SemanticState::Ready, "ready"),
            (SemanticState::Pending, "pending"),
            (SemanticState::Failed, "failed"),
            (SemanticState::Unavailable, "unavailable"),
        ] {
            assert_eq!(state.as_str(), text);
            assert_eq!(
                serde_json::to_string(&state).unwrap(),
                format!("\"{text}\"")
            );
        }
    }

    fn job() -> EmbeddingJob {
        EmbeddingJob {
            index_epoch: 3,
            checkout_id: "checkout-a".to_string(),
            canonical_key: "prd-auth".to_string(),
            incarnation: 1,
            row_revision: 7,
            source_digest: "src".to_string(),
            input_digest: "in".to_string(),
            profile_id: "prof".to_string(),
            captured_input_ref: "prds/PRD-001-auth.md".to_string(),
            attempt: 0,
            retry_after: None,
        }
    }

    #[test]
    fn job_matches_binding_only_when_every_component_agrees() {
        let j = job();
        assert!(j.matches_binding(3, "checkout-a", "prd-auth", 1, 7, "src", "in", "prof"));
        // One component off is a stale job.
        assert!(!j.matches_binding(4, "checkout-a", "prd-auth", 1, 7, "src", "in", "prof"));
        assert!(!j.matches_binding(3, "checkout-b", "prd-auth", 1, 7, "src", "in", "prof"));
        assert!(!j.matches_binding(3, "checkout-a", "prd-auth", 2, 7, "src", "in", "prof"));
        assert!(!j.matches_binding(3, "checkout-a", "prd-auth", 1, 8, "src", "in", "prof"));
        assert!(!j.matches_binding(3, "checkout-a", "prd-auth", 1, 7, "src2", "in", "prof"));
        assert!(!j.matches_binding(3, "checkout-a", "prd-auth", 1, 7, "src", "in2", "prof"));
        assert!(!j.matches_binding(3, "checkout-a", "prd-auth", 1, 7, "src", "in", "prof2"));
    }

    #[test]
    fn next_attempt_increments_and_refuses_to_wrap() {
        let mut j = job();
        j.attempt = 1;
        assert_eq!(j.next_attempt().unwrap().attempt, 2);

        let mut j = job();
        j.attempt = u32::MAX;
        assert!(j.next_attempt().is_err());
    }

    #[test]
    fn a_failure_advances_the_attempt_and_records_when_to_try_again() {
        let mut j = job();
        j.attempt = 2;
        let failed = j.failed_at("2026-10-19T00:01:00.000Z".to_string()).unwrap();
        assert_eq!(failed.attempt, 3, "a failure counts as an attempt");
        assert_eq!(
            failed.retry_after.as_deref(),
            Some("2026-10-19T00:01:00.000Z")
        );
        // The work itself is untouched: a failure changes how often the key
        // has been tried, never what a vector would be computed from.
        assert_eq!(failed.binding(), j.binding());
        assert_eq!(failed.captured_input_ref, j.captured_input_ref);
    }

    #[test]
    fn a_failure_at_the_attempt_ceiling_is_refused_rather_than_wrapping() {
        let mut j = job();
        j.attempt = u32::MAX;
        assert!(j.failed_at("2026-10-19T00:01:00.000Z".to_string()).is_err());
    }

    #[test]
    fn the_retry_schedule_backs_off_and_is_capped() {
        assert_eq!(retry_delay_seconds(0), RETRY_BASE_SECONDS);
        assert_eq!(retry_delay_seconds(1), RETRY_BASE_SECONDS * 2);
        assert_eq!(retry_delay_seconds(3), RETRY_BASE_SECONDS * 8);
        assert_eq!(
            retry_delay_seconds(64),
            RETRY_CEILING_SECONDS,
            "a key that keeps failing is parked, not scheduled past the ceiling"
        );
        assert!(retry_delay_seconds(u32::MAX) <= RETRY_CEILING_SECONDS);
    }

    #[test]
    fn a_job_with_no_failure_is_due_immediately() {
        let j = job();
        assert!(j.retry_is_due(chrono::Utc::now()));
        assert!(j.retry_after.is_none());
    }

    #[test]
    fn a_job_inside_its_retry_delay_is_not_due_and_a_past_one_is() {
        let now = chrono::Utc::now();
        let mut j = job();
        j.retry_after = Some(retry_after_at(0, now));
        assert!(
            !j.retry_is_due(now),
            "the base delay must actually park the key"
        );
        assert!(
            j.retry_is_due(now + chrono::Duration::seconds(RETRY_BASE_SECONDS + 2)),
            "once the delay has passed the key is due again"
        );
    }

    #[test]
    fn an_unreadable_retry_stamp_is_due_rather_than_parked_forever() {
        let mut j = job();
        j.retry_after = Some("not-a-timestamp".to_string());
        assert!(j.retry_is_due(chrono::Utc::now()));
    }

    #[test]
    fn describes_ignores_the_admission_identity_and_the_attempt_counter() {
        let binding = job().binding();
        let mut admitted = EmbeddingJob::for_binding(&binding, "gen-7").unwrap();
        admitted.attempt = 5;
        admitted.retry_after = Some("2026-10-19T00:01:00.000Z".to_string());

        // A later admission over the same bytes describes the same work, so it
        // must not replace the pending job and reset the count.
        assert!(admitted.describes(&binding));

        // Any change to what the vector is computed from is different work.
        assert!(!admitted.describes(&Binding {
            input_digest: "other".to_string(),
            ..binding.clone()
        }));
        assert!(!admitted.describes(&Binding {
            profile_id: "other-space".to_string(),
            ..binding.clone()
        }));
        assert!(!admitted.describes(&Binding {
            row_revision: binding.row_revision + 1,
            ..binding
        }));
    }

    // -------------------------------------------------------------------
    // Admission → compute → completion: the prepared types
    // -------------------------------------------------------------------

    fn prepared_binding(key: &str) -> Binding {
        Binding {
            canonical_key: key.to_string(),
            ..job().binding()
        }
    }

    fn prepared_batch() -> PreparedEmbeddingBatch {
        PreparedEmbeddingBatch {
            admission_generation: "gen-2026-10-19T00-00-00Z".to_string(),
            config_digest: "cfg-a".to_string(),
            profile: profile(),
            inputs: vec![
                PreparedEmbeddingInput {
                    job: EmbeddingJob::for_binding(
                        &prepared_binding("prd-auth"),
                        "gen-2026-10-19T00-00-00Z",
                    )
                    .unwrap(),
                    input_text: std::sync::Arc::from("Auth body"),
                },
                PreparedEmbeddingInput {
                    job: EmbeddingJob::for_binding(
                        &prepared_binding("prd-auth#s0"),
                        "gen-2026-10-19T00-00-00Z",
                    )
                    .unwrap(),
                    input_text: std::sync::Arc::from("Auth section"),
                },
            ],
        }
    }

    #[test]
    fn captured_input_ref_names_the_admission_and_the_key_and_nothing_else() {
        let b = prepared_binding("prd-auth");
        let job = EmbeddingJob::for_binding(&b, "gen-7").unwrap();
        assert_eq!(job.captured_input_ref, "gen-7:prd-auth");
        assert_eq!(job.attempt, 0, "a job that has never run has no attempts");
        assert_eq!(job.retry_after, None);
        // The ref is an identity, not a path: it must not name a file, so a
        // completion can never be tempted to re-read one.
        assert!(
            !job.captured_input_ref.contains(".md"),
            "a captured_input_ref is the right to publish, not a path to read: {}",
            job.captured_input_ref
        );
    }

    #[test]
    fn a_binding_with_no_input_digest_is_refused_at_admission() {
        let mut b = prepared_binding("prd-auth");
        b.input_digest = String::new();
        let err = EmbeddingJob::for_binding(&b, "gen-7").unwrap_err();
        assert!(
            matches!(err, ConstellationError::Io { .. }),
            "a job with no input digest binds nothing: {err}"
        );
    }

    #[test]
    fn a_section_key_is_recognised_only_by_the_exact_suffix() {
        assert_eq!(section_parent_key("prd-auth#s0"), Some("prd-auth"));
        assert_eq!(section_parent_key("prd-auth#s12"), Some("prd-auth"));
        for not_a_section in [
            "prd-auth",
            "prd-auth#notes",
            "prd-auth#s",
            "#s0",
            "prd-auth#s0x",
            "prd-auth#s-1",
        ] {
            assert_eq!(
                section_parent_key(not_a_section),
                None,
                "`{not_a_section}` is not a section key"
            );
        }
    }

    #[test]
    fn row_role_follows_the_key_namespace() {
        let artifact = job();
        assert_eq!(artifact.row_role(), EmbeddingRowRole::Artifact);
        let section = EmbeddingJob::for_binding(&prepared_binding("prd-auth#s3"), "gen-7").unwrap();
        assert_eq!(section.row_role(), EmbeddingRowRole::Section);

        let computed = ComputedEmbedding {
            job: section,
            vector: vec![0.1; 4],
        };
        assert_eq!(computed.row_role(), EmbeddingRowRole::Section);
    }

    #[test]
    fn a_prepared_batch_accepts_only_the_exact_job_it_admitted() {
        let batch = prepared_batch();
        let admitted = batch.job_for("prd-auth").unwrap().clone();
        assert!(batch.accepts("prd-auth", &admitted));

        // Every field that identifies the state the vector was computed for
        // must break acceptance on its own.
        type JobMutator = Box<dyn Fn(&mut EmbeddingJob)>;
        let mutators: Vec<JobMutator> = vec![
            Box::new(|j: &mut EmbeddingJob| j.incarnation += 1),
            Box::new(|j: &mut EmbeddingJob| j.row_revision += 1),
            Box::new(|j: &mut EmbeddingJob| j.source_digest = "other".into()),
            Box::new(|j: &mut EmbeddingJob| j.input_digest = "other".into()),
            Box::new(|j: &mut EmbeddingJob| j.profile_id = "other".into()),
            Box::new(|j: &mut EmbeddingJob| j.captured_input_ref = "gen-other:prd-auth".into()),
            Box::new(|j: &mut EmbeddingJob| j.index_epoch += 1),
            Box::new(|j: &mut EmbeddingJob| j.checkout_id = "other".into()),
        ];
        for m in mutators {
            let mut drifted = admitted.clone();
            m(&mut drifted);
            assert!(
                !batch.accepts("prd-auth", &drifted),
                "a result whose job drifted in one field must not be accepted: \
                 {drifted:?}"
            );
        }
    }

    #[test]
    fn a_batch_never_accepts_a_key_it_did_not_admit() {
        let batch = prepared_batch();
        let stranger = EmbeddingJob::for_binding(&prepared_binding("prd-other"), "gen-7").unwrap();
        assert!(!batch.accepts("prd-other", &stranger));
        assert_eq!(batch.job_for("prd-other"), None);
        // And an admitted key with a stranger's job is refused too.
        assert!(!batch.accepts("prd-auth", &stranger));
    }

    #[test]
    fn prepared_input_texts_are_the_exact_strings_a_worker_embeds() {
        let batch = prepared_batch();
        let texts: Vec<&str> = batch
            .inputs
            .iter()
            .map(|input| input.input_text.as_ref())
            .collect();
        assert_eq!(texts, vec!["Auth body", "Auth section"]);
        // Ordering follows admission, so a worker can pair results positionally
        // without re-deriving anything — and pairing by position is safe only
        // because the batch never reorders.
        let keys: Vec<&str> = batch
            .inputs
            .iter()
            .map(|i| i.job.canonical_key.as_str())
            .collect();
        assert_eq!(keys, vec!["prd-auth", "prd-auth#s0"]);
    }

    #[test]
    fn a_prepared_batch_survives_a_round_trip_through_json() {
        // The batch travels to a caller and back; a job that lost a field on
        // the way would stop being the exact job completion has to match.
        let batch = prepared_batch();
        let encoded = serde_json::to_string(&batch.inputs[0].job).unwrap();
        let decoded: EmbeddingJob = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, batch.inputs[0].job);
        assert!(batch.accepts("prd-auth", &decoded));
    }
}
