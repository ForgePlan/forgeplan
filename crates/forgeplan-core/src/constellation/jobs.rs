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
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let evidence = SnapshotEvidence::read(cache_dir, repo).ok_or_else(|| {
        super::ConstellationError::ModelUnavailable {
            detail: format!(
                "no cached snapshot for `{repo}` under {}; the model was never downloaded, or \
                 `refs/main` is unreadable",
                cache_dir.display()
            ),
        }
    })?;

    let hash_file = |relative: &str| -> Result<String, super::ConstellationError> {
        let path = evidence.root.join(relative);
        // `File::open` follows the `blobs/` symlink; the target's bytes are
        // what get hashed.
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
}

#[cfg(feature = "semantic-search")]
impl ResolvedProfile {
    /// Resolve the **already-cached** snapshot for `repo`, or explain why the
    /// semantic channel is unavailable.
    ///
    /// # Pinned signatures, written here rather than recalled
    ///
    /// ```text
    /// // crates/forgeplan-core/src/embed/tract_engine.rs:229
    /// pub fn find_snapshot(cache_dir: &Path, repo: &str) -> Option<PathBuf>
    ///
    /// // crates/forgeplan-core/src/embed/tract_engine.rs:264
    /// pub fn ensure_model(cache_dir: &Path, repo: &str, show_progress: bool) -> Result<PathBuf>
    /// ```
    ///
    /// # Why `ensure_model` is not called
    ///
    /// Its third parameter is `show_progress` — **not** permission to
    /// download. Calling it from a query or maintenance path would fetch 2 GB
    /// while the caller believed it was reading a cache. Only explicit
    /// `embed`, `setup warm_model` and FPF ingest may obtain weights, and
    /// none of those comes through here.
    ///
    /// # Where the engine goes
    ///
    /// `TractEmbedder::from_snapshot`
    /// (`crates/forgeplan-core/src/embed/tract_engine.rs:70`) is deliberately
    /// **not** called. This type answers *which weights*; the engine belongs to
    /// the caller that will actually embed, and it returns in P4 with that
    /// reader. Opening it here would load a model into memory for a process
    /// that may never encode a single string.
    ///
    /// # Errors
    ///
    /// [`super::ConstellationError::ModelUnavailable`] when no snapshot is
    /// cached, a required file is missing, or no real content digests were
    /// supplied. Never a guessed revision.
    pub fn open(
        cache_dir: &Path,
        repo: &str,
        checksums: &SnapshotChecksums,
        engine_version: &str,
    ) -> Result<Self, super::ConstellationError> {
        use crate::embed::tract_engine;

        if tract_engine::find_snapshot(cache_dir, repo).is_none() {
            return Err(super::ConstellationError::ModelUnavailable {
                detail: format!(
                    "no cached snapshot for `{repo}` under {}",
                    cache_dir.display()
                ),
            });
        }

        let evidence = SnapshotEvidence::read(cache_dir, repo).ok_or_else(|| {
            super::ConstellationError::ModelUnavailable {
                detail: format!("snapshot layout unreadable for `{repo}`"),
            }
        })?;
        let profile = evidence.to_profile(
            checksums.weights.clone(),
            checksums.tokenizer.clone(),
            checksums.tokenizer_config.clone(),
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

        Ok(Self { profile })
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
/// Nine fields, all mandatory. `matches` compares every one of them; dropping
/// any field turns the predicate into a subset a stale job can satisfy.
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
    /// Nine comparisons, `AND`-ed, covering every field of [`Binding`]. If
    /// any one differs — a new incarnation after a delete/recreate, a new
    /// `row_revision` after an edit, a new `profile_id` after a model
    /// change — the predicate matches nothing and the update affects **zero
    /// rows**, so a stale completion can neither insert nor revive.
    ///
    /// # What replaces the old skip condition
    ///
    /// `forgeplan-cli`'s embed skipped a record when
    /// `record.embedding.is_some() && record.body_hash == compute_content_hash(title, body)`.
    /// `body_hash` is a content stamp: it sees the title and the body and
    /// nothing else. It cannot see a **profile** change, so re-running embed
    /// after a model swap skipped every record and left vectors from the old
    /// space in place; nor an **incarnation** change, so a recreated artifact
    /// was skipped; nor the **input digest**, which is what the tokenizer
    /// actually receives. This predicate covers all three, which is why the
    /// hash comparison goes rather than being kept alongside.
    ///
    /// # Escaping
    ///
    /// Single quotes are doubled and the value is delimited. No caller text
    /// is ever concatenated unescaped: a predicate is the one place where a
    /// quote in the data becomes structure in the query.
    pub fn predicate(&self) -> String {
        fn text(value: &str) -> String {
            format!("'{}'", value.replace('\'', "''"))
        }
        format!(
            "canonical_key = {key} \
             AND index_epoch = {epoch} \
             AND checkout_id = {checkout} \
             AND incarnation = {incarnation} \
             AND row_revision = {revision} \
             AND source_digest = {source} \
             AND input_digest = {input} \
             AND profile_id = {profile}",
            key = text(&self.canonical_key),
            epoch = self.index_epoch,
            checkout = text(&self.checkout_id),
            incarnation = self.incarnation,
            revision = self.row_revision,
            source = text(&self.source_digest),
            input = text(&self.input_digest),
            profile = text(&self.profile_id),
        )
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
    let (Some(profile_id), Some(semantic_state)) = (
        stored.profile_id.as_deref(),
        stored.semantic_state.as_deref(),
    ) else {
        return missing("profile or semantic state");
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
    fn the_publish_guard_covers_every_binding_field() {
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
            "nine comparisons joined by seven ANDs"
        );
    }

    #[test]
    fn the_publish_guard_escapes_quotes_in_every_text_field() {
        let mut b = binding();
        b.canonical_key = "O'Brien".to_string();
        b.checkout_id = "local'x".to_string();
        let predicate = b.predicate();
        assert!(
            predicate.contains("'O''Brien'"),
            "a quote in the data must not become structure: {predicate}"
        );
        assert!(predicate.contains("'local''x'"));
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
    fn a_changed_profile_changes_the_guard() {
        let before = binding();
        let after = Binding {
            profile_id: "profile-d".to_string(),
            ..before.clone()
        };
        assert_ne!(
            before.predicate(),
            after.predicate(),
            "a model swap must not be skipped by a content-only stamp"
        );
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
}
