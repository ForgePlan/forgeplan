//! Claim protocol — soft coordination signal for multi-agent workspaces.
//!
//! A claim is a file in `.forgeplan/claims/<ID>.yaml` that says "agent X
//! is actively working on artifact ID until T". Claims are advisory: any
//! agent may forcibly take over with `--force`, and a TTL guarantees that
//! a crashed agent's claim self-expires (NFR-004).
//!
//! Scope is deliberately narrow: the claim mediates intent, not access —
//! write serialization is handled by `workspace::lock` (Inc 1), and the
//! dispatcher (Inc 4) is what converts claim state into parallel buckets.
//!
//! PRD-057 FR-004..006, FR-014, AC-2, AC-3.

use crate::artifact::identity::{self, MAX_FIELD_LEN as IDENTITY_MAX_FIELD_LEN};
use crate::workspace::lock::{WorkspaceLock, WorkspaceReadPermit};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

/// Maximum permitted agent-string length. Mirrors the per-field cap in
/// `artifact::identity::MAX_FIELD_LEN` (64) doubled to leave room for the
/// canonical `name/version` shape — even though we forbid the `/` delimiter
/// in the free-form claim agent string, we still allow other reasonable
/// composite forms (e.g. `worker-1-staging`) up to this length.
const MAX_AGENT_LEN: usize = IDENTITY_MAX_FIELD_LEN;

/// Upper bound on a single claim YAML file. Matches the 64 KB cap in
/// `artifact::frontmatter::parse_frontmatter` — defense against a
/// pathological claim file (billion-laughs attempt, oversized note) on
/// `get` / `list_active` deserialization. R1-audit security finding.
const MAX_CLAIM_FILE_BYTES: u64 = 65_536;

/// Upper bound on the canonical key a claim file may be named after.
///
/// The key is escaped into a file name, so the escaped form is what has to
/// fit a file system: 255 bytes on ext4, less on other file systems. The
/// cap leaves room for the escape expansion and the `.yaml` extension.
const MAX_CANONICAL_KEY_LEN: usize = 128;

/// Minimum permitted TTL — claims shorter than this would churn more than
/// they coordinate. Orchestrators can still force-release earlier.
pub const MIN_TTL: Duration = Duration::seconds(60);
/// Upper bound (R-3 mitigation). Longer-running work should renew instead
/// of setting a day-long claim — renewal is what gives the system a
/// periodic "agent still alive" signal.
pub const MAX_TTL: Duration = Duration::hours(24);
/// Default when no TTL supplied. Covers a typical sub-agent work window
/// without blocking the workspace for hours on a silent crash (R-4).
pub const DEFAULT_TTL: Duration = Duration::minutes(30);

/// Persisted shape of a claim file. One file per claimed artifact; absence
/// of file means "unclaimed". Expired claims are technically still on disk
/// but filtered out by every read path (`get`, `list_active`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    /// Artifact being claimed (e.g. `PRD-057`). Uppercased on disk.
    ///
    /// This is the document's **display alias** — the name a person reads.
    /// The alias is not the claim's address: two documents may share one
    /// alias, so the address is the canonical key below.
    pub id: String,
    /// The document's **canonical key** — the identity this claim belongs to.
    ///
    /// The sidecar file is named after this key, so two documents that share
    /// a display alias never share a claim. `None` for a claim written
    /// before the key was recorded; its legacy alias path is read only when
    /// the workspace scan proves that alias has a unique owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_key: Option<String>,
    /// Caller identity — typically `AgentIdentity::as_frontmatter_value()`
    /// ("name/version"), but kept as a free-form string so orchestrator
    /// tools can inject synthetic names like `worker-1`.
    pub agent_id: String,
    pub claimed_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// Optional free-form note — "working on FR-003", "spec writing", etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Claim {
    /// True iff `Utc::now() >= expires_at`. Centralized so both reads and
    /// writes use the same wall-clock comparison.
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    /// Exact-match agent check. Agents are identified by string; callers
    /// are responsible for normalizing (trim / lowercase if needed).
    pub fn is_held_by(&self, agent: &str) -> bool {
        self.agent_id == agent
    }
}

/// Errors surfaced by the claim protocol. Designed so the MCP layer can
/// translate into user-facing messages without leaking filesystem paths.
#[derive(Debug, thiserror::Error)]
pub enum ClaimError {
    #[error("claim for {id} already held by {agent_id}, expires at {expires_at}")]
    AlreadyHeld {
        id: String,
        agent_id: String,
        expires_at: DateTime<Utc>,
    },
    #[error("claim for {id} is held by {held_by}, not {requester}")]
    NotHeldByRequester {
        id: String,
        held_by: String,
        requester: String,
    },
    #[error("ttl out of permitted range (1 minute..=24 hours)")]
    TtlOutOfRange(Duration),
    #[error("artifact id must be non-empty")]
    EmptyId,
    #[error(
        "artifact id contains invalid characters (allowed: A-Z, a-z, 0-9, '-', '_'; must start with a letter): {0:?}"
    )]
    InvalidId(String),
    #[error(
        "the display alias {selector:?} names more than one document — address one by its canonical key"
    )]
    AmbiguousAlias {
        /// The alias the caller wrote.
        selector: String,
    },
    #[error("agent id must be non-empty")]
    EmptyAgent,
    #[error(
        "agent id contains invalid characters (allowed: printable non-path, no controls, no bidi/ZWJ, no `/`, `\\`, NUL): {0:?}"
    )]
    InvalidAgent(String),
    #[error("agent id exceeds {MAX_AGENT_LEN} bytes")]
    AgentTooLong(usize),
    #[error("claim file exceeds {MAX_CLAIM_FILE_BYTES} bytes — refusing to parse")]
    FileTooLarge,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("artifact selector could not be resolved: {0}")]
    Resolution(String),
    #[error("prepared claim no longer matches the workspace or its inspected state")]
    PreparedStateChanged,
}

/// Validate an artifact ID is safe for filesystem use.
///
/// Refuses path-traversal segments (`..`, `/`, `\\`, null bytes, leading
/// `.`) and any character outside `[A-Za-z0-9_-]`. Parallels
/// `db::store::validate_id_for_filter` but returns a typed ClaimError so
/// MCP error hints remain structured.
///
/// Security (R2 audit HIGH #1): without this, `id="../../etc/passwd"`
/// would escape `.forgeplan/claims/` via `Path::join`; `release` would
/// then `remove_file` arbitrary paths the process can write.
fn validate_id(id: &str) -> Result<(), ClaimError> {
    if id.is_empty() {
        return Err(ClaimError::EmptyId);
    }
    let first = id.chars().next().unwrap_or(' ');
    if !first.is_ascii_alphabetic() {
        return Err(ClaimError::InvalidId(id.to_string()));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ClaimError::InvalidId(id.to_string()));
    }
    // Belt-and-suspenders — should be unreachable after the charset check
    // but explicit rejection of traversal sequences is cheap insurance.
    if id.contains("..") || id.contains('/') || id.contains('\\') || id.contains('\0') {
        return Err(ClaimError::InvalidId(id.to_string()));
    }
    Ok(())
}

/// Validate a canonical key that is about to become a file name.
///
/// The key is escaped into exactly one path component (see
/// [`crate::artifact::sidecar::sidecar_file_stem`]), so whatever it holds
/// cannot escape the claims directory. The check only refuses what cannot
/// be a key at all: an empty name, a name too long for a file, and a name
/// with a NUL byte in it.
fn validate_canonical_key(key: &str) -> Result<(), ClaimError> {
    if key.is_empty() {
        return Err(ClaimError::EmptyId);
    }
    if key.len() > MAX_CANONICAL_KEY_LEN {
        return Err(ClaimError::InvalidId(key.to_string()));
    }
    if key.contains('\0') {
        return Err(ClaimError::InvalidId(key.to_string()));
    }
    Ok(())
}

/// Validate the free-form agent string accepted by `forgeplan claim --agent`.
///
/// Closes PROB-066: CLI surface previously only checked `is_empty()`, while
/// the MCP write-stamping path enforced a full character-class filter via
/// `AgentIdentity::new`. The asymmetry let a malicious or careless caller
/// land `\n`, `\u{202E}` (RLO), `\u{200B}` (ZWSP), ANSI escapes, or the `/`
/// delimiter into the YAML body — corrupting the file (newline) or spoofing
/// the operator's terminal (bidi/control chars) when `claim_id` was echoed
/// by `forgeplan claims`.
///
/// The character-class rejection delegates to `is_identity_char_forbidden`
/// so CLI and MCP identity surfaces stay symmetric — the single source of
/// truth lives in `artifact::identity`.
///
/// **CLI-strict variant** — rejects `/` outright so the operator cannot
/// enshrine the canonical MCP `name/version` shape as a CLI argument
/// (`smoke-test/v1` was the audit-flagged case). Use this from
/// `forgeplan claim --agent <STR>` and similar operator-supplied surfaces.
///
/// For the canonical `name/version` form emitted by `AgentIdentity::as_frontmatter_value`
/// (legitimate on the MCP write-stamping path), see
/// [`validate_agent_id_relaxed`] which keeps the same controls/bidi/NUL
/// filter but allows `/`.
///
/// Rejections:
/// - empty / whitespace-only string → `EmptyAgent`
/// - length > `MAX_AGENT_LEN` (64) bytes → `AgentTooLong`
/// - any character matching `is_identity_char_forbidden` (controls, bidi,
///   ZWJ, BOM, format chars, variation selectors, tag chars, `/`, `\`, NUL)
///   → `InvalidAgent`
pub fn validate_agent_id(agent: &str) -> Result<(), ClaimError> {
    let trimmed = agent.trim();
    if trimmed.is_empty() {
        return Err(ClaimError::EmptyAgent);
    }
    if trimmed.len() > MAX_AGENT_LEN {
        return Err(ClaimError::AgentTooLong(trimmed.len()));
    }
    if trimmed.chars().any(identity::is_identity_char_forbidden) {
        return Err(ClaimError::InvalidAgent(trimmed.to_string()));
    }
    Ok(())
}

/// Same defence class as [`validate_agent_id`] but keeps `/` and `\` legal.
///
/// Used inside `ClaimStore::claim`/`ClaimStore::release` so the MCP path
/// (which passes the canonical `AgentIdentity::as_frontmatter_value()` form
/// — e.g. `claude-code/1.0.50`) still works while the dangerous classes
/// (controls, bidi/ZWJ, BOM, format chars, NUL) remain rejected.
///
/// CLI-side callers should call the stricter [`validate_agent_id`] BEFORE
/// reaching the store — defense-in-depth from operator typo / pasted
/// payload, with this function as the final filter that catches anything
/// bypassing the CLI guard.
fn validate_agent_id_relaxed(agent: &str) -> Result<(), ClaimError> {
    let trimmed = agent.trim();
    if trimmed.is_empty() {
        return Err(ClaimError::EmptyAgent);
    }
    if trimmed.len() > MAX_AGENT_LEN {
        return Err(ClaimError::AgentTooLong(trimmed.len()));
    }
    // Same forbidden classes minus `/` and `\` — those are part of the
    // canonical `name/version` shape on the MCP path.
    if trimmed
        .chars()
        .any(|c| c != '/' && c != '\\' && identity::is_identity_char_forbidden(c))
    {
        return Err(ClaimError::InvalidAgent(trimmed.to_string()));
    }
    // NUL is still always banned — it would corrupt the YAML body even on
    // the MCP path.
    if trimmed.contains('\0') {
        return Err(ClaimError::InvalidAgent(trimmed.to_string()));
    }
    Ok(())
}

/// Atomic file write via `write to tempfile → rename` so a crash mid-write
/// never produces a truncated target. R2 audit MED (rust-pro + architect):
/// `tokio::fs::write` was a single non-atomic syscall; a SIGKILL between
/// truncate and write left a zero-byte YAML that blocked every subsequent
/// `get`.
async fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "atomic_write: path has no parent",
        )
    })?;
    tokio::fs::create_dir_all(parent).await?;
    let tmp = parent.join(format!(
        ".{}.tmp.{}",
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "anon".to_string()),
        std::process::id(),
    ));
    tokio::fs::write(&tmp, bytes).await?;
    // `tokio::fs::rename` is atomic on POSIX and "atomic on same volume" on
    // Windows (MoveFileEx with MOVEFILE_REPLACE_EXISTING). Good enough for
    // workspace-local writes.
    match tokio::fs::rename(&tmp, path).await {
        Ok(()) => Ok(()),
        Err(e) => {
            // Clean up tempfile on failure — otherwise repeated writes
            // would accumulate orphans.
            let _ = tokio::fs::remove_file(&tmp).await;
            Err(e)
        }
    }
}

/// On-disk claim store rooted at `<workspace>/claims/`.
///
/// Every claim file is named after the document's **canonical key** — the
/// identity — and never after its display alias. `PRD-001` is a name two
/// documents may share; a claim named after it would give those two
/// documents one row, and one agent's claim would silently block the
/// other's work. The canonical key is unique, so a claim named after the key
/// belongs to exactly one document.
///
/// An alias-named file is read without mutation only after the workspace scan
/// proves that the alias belongs to exactly one canonical key.
#[derive(Debug, Clone)]
#[must_use = "ClaimStore is a lightweight handle — bind it before calling methods"]
pub struct ClaimStore {
    /// `<workspace>/claims/` — where the claim files live.
    dir: PathBuf,
    /// The `.forgeplan/` directory the store belongs to. Read to learn the
    /// canonical key of a document a caller names by its alias.
    root: PathBuf,
}

/// A claim prepared under the borrowed exclusive workspace lock.
#[derive(Debug)]
pub struct PreparedClaim<'a> {
    state: PreparedState<'a>,
    claim: Claim,
    yaml: String,
    keyed_path: PathBuf,
    legacy_path: Option<PathBuf>,
}

/// A release prepared under the borrowed exclusive workspace lock.
#[derive(Debug)]
pub struct PreparedRelease<'a> {
    state: PreparedState<'a>,
    paths: Vec<PathBuf>,
}

#[derive(Debug)]
struct PreparedState<'a> {
    guard: Option<&'a WorkspaceLock>,
    workspace: PathBuf,
    canonical_key: String,
    display_alias: String,
    candidates: Vec<crate::artifact::canonical_key::ArtifactCandidate>,
    files: Vec<(PathBuf, Option<[u8; 32]>)>,
}

impl PreparedClaim<'_> {
    /// The resolved document key.
    pub fn canonical_key(&self) -> &str {
        &self.state.canonical_key
    }

    /// The resolved document display alias.
    pub fn display_alias(&self) -> &str {
        &self.state.display_alias
    }
}

impl PreparedRelease<'_> {
    /// Whether applying this release removes a claim file.
    pub fn is_mutation(&self) -> bool {
        !self.paths.is_empty()
    }

    /// The resolved document key.
    pub fn canonical_key(&self) -> &str {
        &self.state.canonical_key
    }

    /// The resolved document display alias.
    pub fn display_alias(&self) -> &str {
        &self.state.display_alias
    }
}


impl ClaimStore {
    /// `workspace` is the `.forgeplan/` directory. The store creates
    /// `<workspace>/claims/` lazily on first write.
    pub fn new(workspace: impl AsRef<Path>) -> Self {
        let root = workspace.as_ref().to_path_buf();
        Self {
            dir: root.join("claims"),
            root,
        }
    }

    fn guarded_store(&self, guard: &WorkspaceLock) -> Result<Self, ClaimError> {
        guard.matches_workspace(&self.root).map_err(|_| ClaimError::PreparedStateChanged)?;
        Ok(Self::new(self.root.canonicalize()?))
    }

    /// The path of a document's claim file, named after its canonical key.
    ///
    /// The name is the escaped key — see
    /// [`crate::artifact::sidecar::sidecar_file_stem`] — so a key that
    /// carries a separator still names exactly one file inside the claims
    /// directory.
    fn path_for(&self, canonical_key: &str) -> PathBuf {
        crate::artifact::sidecar::sidecar_path(&self.dir, canonical_key, "yaml")
    }

    /// Find a document's claim file without migrating a pre-key file.
    ///
    /// Alias-named legacy files are read only when a complete scan proves the
    /// alias has one owner. This read never migrates or renames sidecars.
    async fn sidecar_path_for(&self, canonical_key: &str, display_alias: &str) -> Result<PathBuf, ClaimError> {
        Ok(crate::artifact::sidecar::resolve_sidecar_path(
            &self.dir,
            canonical_key,
            display_alias,
            "yaml",
        )
        .await?)
    }

    /// The identity a selector names: the canonical key of the document, and
    /// the display alias it is known by.
    ///
    /// The selector may be a display alias (`PRD-001`) or a canonical key
    /// (`prd-auth-system`); both name the same document and therefore the
    /// same claim file. A selector that names no document keeps the pre-key
    /// naming — the alias upper-cased — so a forward reference lands where
    /// the old code put it and an old workspace stays readable.
    ///
    /// An alias several documents share is refused: the claim would belong
    /// to one of them, and which one is not the store's decision.
    /// Resolve a selector without failing open on an incomplete workspace.
    async fn resolve_identity(&self, selector: &str) -> Result<(String, String), ClaimError> {
        let fallback = || (selector.to_uppercase(), selector.to_string());
        let candidates = crate::artifact::documents::workspace_candidates(&self.root)
            .await
            .map_err(|error| ClaimError::Io(std::io::Error::other(error)))?;
        match crate::artifact::canonical_key::resolve_artifact(selector, &candidates) {
            Ok(Some(resolved)) => Ok((
                resolved.identity.canonical_key,
                resolved.identity.display_id,
            )),
            Ok(None) => Ok(fallback()),
            Err(crate::artifact::canonical_key::ResolveError::InvalidSelector { .. }) => {
                Ok(fallback())
            }
            Err(crate::artifact::canonical_key::ResolveError::Ambiguous { .. }) => {
                Err(ClaimError::AmbiguousAlias {
                    selector: selector.to_string(),
                })
            }
            Err(problem) => Err(ClaimError::Resolution(problem.to_string())),
        }
    }

    async fn prepare_state<'a>(
        &self,
        canonical_key: &str,
        display_alias: &str,
        guard: Option<&'a WorkspaceLock>,
    ) -> Result<PreparedState<'a>, ClaimError> {
        let workspace = if let Some(guard) = guard {
            guard.matches_workspace(&self.root).map_err(|_| ClaimError::PreparedStateChanged)?;
            self.root.canonicalize()?
        } else {
            self.root.clone()
        };
        let candidates = crate::artifact::documents::workspace_candidates(&self.root)
            .await.map_err(|error| ClaimError::Io(std::io::Error::other(error)))?;
        let display_alias = if display_alias == canonical_key {
            candidates.iter().find(|candidate| candidate.canonical_key == canonical_key)
                .map_or(display_alias, |candidate| candidate.display_id.as_str())
        } else {
            display_alias
        };
        let mut paths = crate::artifact::sidecar::legacy_sidecar_paths(
            &self.dir, canonical_key, display_alias, "yaml",
        );
        let keyed = self.path_for(canonical_key);
        if !paths.contains(&keyed) {
            paths.push(keyed);
        }
        let mut files = Vec::with_capacity(paths.len());
        for path in paths {
            let digest = file_digest(&path).await?;
            files.push((path, digest));
        }
        Ok(PreparedState {
            guard,
            workspace,
            canonical_key: canonical_key.to_string(),
            display_alias: display_alias.to_string(),
            candidates,
            files,
        })
    }

    async fn check_prepared_state(&self, state: &PreparedState<'_>) -> Result<(), ClaimError> {
        if let Some(guard) = state.guard {
            guard.matches_workspace(&self.root).map_err(|_| ClaimError::PreparedStateChanged)?;
            if self.root.canonicalize()? != state.workspace {
                return Err(ClaimError::PreparedStateChanged);
            }
        } else if self.root != state.workspace {
            return Err(ClaimError::PreparedStateChanged);
        }
        let candidates = crate::artifact::documents::workspace_candidates(&self.root)
            .await.map_err(|error| ClaimError::Io(std::io::Error::other(error)))?;
        if candidates != state.candidates {
            return Err(ClaimError::PreparedStateChanged);
        }
        for (path, digest) in &state.files {
            if file_digest(path).await? != *digest {
                return Err(ClaimError::PreparedStateChanged);
            }
        }
        Ok(())
    }

    #[cfg(test)]
    /// Ensure the claims directory exists. Called before any write.
    async fn ensure_dir(&self) -> Result<(), ClaimError> {
        tokio::fs::create_dir_all(&self.dir).await?;
        Ok(())
    }

    /// Read a claim if present AND not expired. Expired claims are treated
    /// as absent so downstream logic can acquire without a separate purge
    /// step (AC-3).
    ///
    /// The selector may be a display alias or a canonical key: both name
    /// the same document, so both read the same claim file.
    pub async fn get(&self, id: &str) -> Result<Option<Claim>, ClaimError> {
        validate_id(id)?;
        let (canonical_key, display_alias) = self.resolve_identity(id).await?;
        self.get_for(&canonical_key, &display_alias).await
    }

    /// Read the claim of a document named by its canonical key.
    ///
    /// The display alias is only used to find a claim written before the key
    /// existed; the file itself is addressed by the key.
    pub async fn get_for(
        &self,
        canonical_key: &str,
        display_alias: &str,
    ) -> Result<Option<Claim>, ClaimError> {
        validate_canonical_key(canonical_key)?;
        let path = self.sidecar_path_for(canonical_key, display_alias).await?;
        let raw = match read_bounded(&path).await? {
            Some(s) => s,
            None => return Ok(None),
        };
        let mut claim: Claim = serde_yaml::from_str(&raw)?;
        if claim.canonical_key.is_none() {
            // A claim written before the key was recorded still belongs to
            // the document the key names.
            claim.canonical_key = Some(canonical_key.to_string());
        }
        if claim.is_expired() {
            Ok(None)
        } else {
            Ok(Some(claim))
        }
    }

    /// Raw get without TTL filtering. Exposed for diagnostics (`--include-expired`).
    pub async fn get_including_expired(&self, id: &str) -> Result<Option<Claim>, ClaimError> {
        validate_id(id)?;
        let (canonical_key, display_alias) = self.resolve_identity(id).await?;
        self.get_including_expired_for(&canonical_key, &display_alias)
            .await
    }

    /// Raw get of a document named by its canonical key, without TTL
    /// filtering.
    pub async fn get_including_expired_for(
        &self,
        canonical_key: &str,
        display_alias: &str,
    ) -> Result<Option<Claim>, ClaimError> {
        validate_canonical_key(canonical_key)?;
        let path = self.sidecar_path_for(canonical_key, display_alias).await?;
        match read_bounded(&path).await? {
            Some(raw) => {
                let mut claim: Claim = serde_yaml::from_str(&raw)?;
                if claim.canonical_key.is_none() {
                    claim.canonical_key = Some(canonical_key.to_string());
                }
                Ok(Some(claim))
            }
            None => Ok(None),
        }
    }

    /// Attempt to take a claim. Succeeds if:
    /// - no file exists
    /// - file exists but is expired (overwritten — AC-3)
    /// - file exists, is live, and is held by `agent` (renewed)
    ///
    /// Fails with `AlreadyHeld` otherwise (AC-2).
    ///
    /// The selector may be a display alias or a canonical key: both name
    /// the same document, so both take the same claim.
    ///
    /// Callers should already hold the workspace lock (`workspace::lock`)
    /// to make the read-modify-write atomic across sub-agents sharing the
    /// filesystem — this function does no internal locking.
    pub async fn claim(
        &self,
        id: &str,
        agent: &str,
        ttl: Duration,
        note: Option<String>,
    ) -> Result<Claim, ClaimError> {
        let prepared = self.prepare_claim_impl(id, agent, ttl, note, None).await?;
        self.apply_prepared_claim(prepared).await
    }

    /// Resolve and check a claim without modifying the workspace. Hold the
    /// workspace lock until the plan is applied or discarded.
    pub async fn prepare_claim<'a>(
        &self,
        id: &str,
        agent: &str,
        ttl: Duration,
        note: Option<String>,
        guard: &'a WorkspaceLock,
    ) -> Result<PreparedClaim<'a>, ClaimError> {
        self.guarded_store(guard)?
            .prepare_claim_impl(id, agent, ttl, note, Some(guard)).await
    }

    async fn prepare_claim_impl<'a>(
        &self,
        id: &str,
        agent: &str,
        ttl: Duration,
        note: Option<String>,
        guard: Option<&'a WorkspaceLock>,
    ) -> Result<PreparedClaim<'a>, ClaimError> {
        validate_id(id)?;
        // PROB-066: full character-class filter (mirrors AgentIdentity::new).
        // Previously only is_empty() was checked — letting `\n`, `\u{202E}`,
        // ANSI escapes, NUL through. The store uses the relaxed variant
        // because the MCP path legitimately passes the `name/version` shape
        // (`claude-code/1.0.50`) via `AgentIdentity::as_frontmatter_value()`.
        // CLI surfaces apply the stricter `validate_agent_id` BEFORE
        // reaching this point.
        validate_agent_id_relaxed(agent)?;
        if ttl < MIN_TTL || ttl > MAX_TTL {
            return Err(ClaimError::TtlOutOfRange(ttl));
        }

        let (canonical_key, display_alias) = self.resolve_identity(id).await?;
        let plan = self.prepare_claim_for_impl(&canonical_key, &display_alias, agent, ttl, note, guard).await?;
        if self.resolve_identity(id).await? != (canonical_key, display_alias) {
            return Err(ClaimError::PreparedStateChanged);
        }
        Ok(plan)
    }

    /// Attempt to take a claim on a document named by its canonical key.
    ///
    /// The claim file is named after the key, so two documents that share a
    /// display alias never share a claim. The display alias is kept inside
    /// the file for the humans who read it, and is used to find a claim
    /// written before the key existed.
    pub async fn claim_for(
        &self,
        canonical_key: &str,
        display_alias: &str,
        agent: &str,
        ttl: Duration,
        note: Option<String>,
    ) -> Result<Claim, ClaimError> {
        let prepared = self
            .prepare_claim_for_impl(canonical_key, display_alias, agent, ttl, note, None)
            .await?;
        self.apply_prepared_claim(prepared).await
    }

    /// Check a claim without modifying the workspace.
    ///
    /// Hold the workspace lock until the returned plan has been applied or
    /// discarded. This keeps ownership and identity stable across the write.
    pub async fn prepare_claim_for<'a>(
        &self,
        canonical_key: &str,
        display_alias: &str,
        agent: &str,
        ttl: Duration,
        note: Option<String>,
        guard: &'a WorkspaceLock,
    ) -> Result<PreparedClaim<'a>, ClaimError> {
        self.guarded_store(guard)?
            .prepare_claim_for_impl(canonical_key, display_alias, agent, ttl, note, Some(guard)).await
    }

    async fn prepare_claim_for_impl<'a>(
        &self,
        canonical_key: &str,
        display_alias: &str,
        agent: &str,
        ttl: Duration,
        note: Option<String>,
        guard: Option<&'a WorkspaceLock>,
    ) -> Result<PreparedClaim<'a>, ClaimError> {
        validate_canonical_key(canonical_key)?;
        validate_agent_id_relaxed(agent)?;
        if ttl < MIN_TTL || ttl > MAX_TTL {
            return Err(ClaimError::TtlOutOfRange(ttl));
        }
        let agent = agent.trim();
        let state = self.prepare_state(canonical_key, display_alias, guard).await?;
        let existing_path = self.sidecar_path_for(canonical_key, display_alias).await?;
        if let Some(existing) = read_claim_at(&existing_path).await?
            && !existing.is_held_by(agent)
            && !existing.is_expired()
        {
            return Err(ClaimError::AlreadyHeld {
                id: existing.id,
                agent_id: existing.agent_id,
                expires_at: existing.expires_at,
            });
        }

        let now = Utc::now();
        let claim = Claim {
            id: display_alias.to_uppercase(),
            canonical_key: Some(canonical_key.to_string()),
            agent_id: agent.to_string(),
            claimed_at: now,
            expires_at: now + ttl,
            note,
        };
        let yaml = serde_yaml::to_string(&claim)?;
        let keyed_path = self.path_for(canonical_key);
        let legacy_path = (existing_path != keyed_path).then_some(existing_path);
        self.check_prepared_state(&state).await?;
        Ok(PreparedClaim {
            state,
            claim,
            yaml,
            keyed_path,
            legacy_path,
        })
    }

    /// Apply a prepared claim while still holding the preparation lock.
    pub async fn apply_prepared_claim(&self, prepared: PreparedClaim<'_>) -> Result<Claim, ClaimError> {
        self.check_prepared_state(&prepared.state).await?;
        atomic_write(&prepared.keyed_path, prepared.yaml.as_bytes()).await?;
        if let Some(path) = prepared.legacy_path {
            match tokio::fs::remove_file(path).await {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(ClaimError::Io(error)),
            }
        }
        Ok(prepared.claim)
    }

    /// Remove a claim.
    ///
    /// - `force = false`: refuse if held by a different agent (protects
    ///   agents from clobbering each other's work markers).
    /// - `force = true`: remove regardless — orchestrator escape hatch for
    ///   a confirmed-dead agent (R-4 mitigation).
    ///
    /// Missing claim is a no-op (not an error) to make release idempotent.
    ///
    /// The selector may be a display alias or a canonical key: both name
    /// the same document, so both release the same claim.
    pub async fn release(&self, id: &str, agent: &str, force: bool) -> Result<(), ClaimError> {
        let prepared = self.prepare_release_impl(id, agent, force, None).await?;
        self.apply_prepared_release(prepared).await
    }

    /// Resolve and check a release without modifying the workspace. Hold the
    /// workspace lock until the plan is applied or discarded.
    pub async fn prepare_release<'a>(
        &self,
        id: &str,
        agent: &str,
        force: bool,
        guard: &'a WorkspaceLock,
    ) -> Result<PreparedRelease<'a>, ClaimError> {
        self.guarded_store(guard)?
            .prepare_release_impl(id, agent, force, Some(guard)).await
    }

    async fn prepare_release_impl<'a>(
        &self,
        id: &str,
        agent: &str,
        force: bool,
        guard: Option<&'a WorkspaceLock>,
    ) -> Result<PreparedRelease<'a>, ClaimError> {
        validate_id(id)?;
        // R2 audit HIGH #2 (rust-pro): the empty-agent guard must run BEFORE
        // the filesystem read so that a bogus `release("X", "", false)` call
        // on a missing claim doesn't silently succeed. Only `force = true`
        // legitimately waives the agent requirement (orchestrator reaping
        // a dead holder without knowing whose it is).
        //
        // PROB-066: extend the guard from is_empty() to the full identity
        // char-class — release must reject the same agent strings as claim,
        // otherwise a hostile caller could trigger error-path `eprintln!`
        // with bidi-override / ANSI escape bytes already neutralised at the
        // write site (we don't want the inverse asymmetry either).
        if !force {
            validate_agent_id_relaxed(agent)?;
        }
        // Match the trim semantics applied at write time so callers can
        // pass `" foo "` against a stored `foo` consistently.
        let agent = agent.trim();

        let (canonical_key, display_alias) = self.resolve_identity(id).await?;
        let plan = self.prepare_release_for_impl(&canonical_key, &display_alias, agent, force, guard).await?;
        if self.resolve_identity(id).await? != (canonical_key, display_alias) {
            return Err(ClaimError::PreparedStateChanged);
        }
        Ok(plan)
    }

    /// Remove the claim of a document named by its canonical key.
    ///
    /// Keyed claims take priority. Any shadowed legacy claims are removed
    /// only when their unique ownership and holder can also be established.
    pub async fn release_for(
        &self,
        canonical_key: &str,
        display_alias: &str,
        agent: &str,
        force: bool,
    ) -> Result<(), ClaimError> {
        let prepared = self
            .prepare_release_for_impl(canonical_key, display_alias, agent, force, None)
            .await?;
        self.apply_prepared_release(prepared).await
    }

    /// Check a release without modifying the workspace.
    ///
    /// Hold the workspace lock until the returned plan has been applied or
    /// discarded. A missing claim produces a non-mutating plan.
    pub async fn prepare_release_for<'a>(
        &self,
        canonical_key: &str,
        display_alias: &str,
        agent: &str,
        force: bool,
        guard: &'a WorkspaceLock,
    ) -> Result<PreparedRelease<'a>, ClaimError> {
        self.guarded_store(guard)?
            .prepare_release_for_impl(canonical_key, display_alias, agent, force, Some(guard)).await
    }

    async fn prepare_release_for_impl<'a>(
        &self,
        canonical_key: &str,
        display_alias: &str,
        agent: &str,
        force: bool,
        guard: Option<&'a WorkspaceLock>,
    ) -> Result<PreparedRelease<'a>, ClaimError> {
        validate_canonical_key(canonical_key)?;
        if !force {
            validate_agent_id_relaxed(agent)?;
        }
        let agent = agent.trim();
        let state = self.prepare_state(canonical_key, display_alias, guard).await?;
        let path = self.sidecar_path_for(canonical_key, display_alias).await?;
        if !force
            && let Some(existing) = read_claim_at(&path).await?
            && !existing.is_held_by(agent)
            && !existing.is_expired()
        {
            return Err(ClaimError::NotHeldByRequester {
                id: existing.id,
                held_by: existing.agent_id,
                requester: agent.to_string(),
            });
        }
        let mut paths = Vec::new();
        if state.files.iter().any(|(candidate, digest)| candidate == &path && digest.is_some()) {
            paths.push(path.clone());
            for (candidate, digest) in &state.files {
                if candidate == &path || digest.is_none() {
                    continue;
                }
                let owner = crate::artifact::canonical_key::resolve_artifact(
                    &state.display_alias, &state.candidates,
                ).map_err(|error| ClaimError::Resolution(error.to_string()))?;
                if !owner.is_some_and(|owner| owner.identity.canonical_key == canonical_key) {
                    return Err(ClaimError::Resolution("legacy claim ownership is not unique".to_string()));
                }
                let shadow = read_claim_at(candidate).await?
                    .ok_or(ClaimError::PreparedStateChanged)?;
                if shadow.canonical_key.as_deref().is_some_and(|key| key != canonical_key) {
                    return Err(ClaimError::Resolution("legacy claim names a different canonical key".to_string()));
                }
                if !force && !shadow.is_held_by(agent) && !shadow.is_expired() {
                    return Err(ClaimError::NotHeldByRequester {
                        id: shadow.id,
                        held_by: shadow.agent_id,
                        requester: agent.to_string(),
                    });
                }
                paths.push(candidate.clone());
            }
        }
        self.check_prepared_state(&state).await?;
        Ok(PreparedRelease { state, paths })
    }

    /// Apply a prepared release while still holding the preparation lock.
    pub async fn apply_prepared_release(&self, prepared: PreparedRelease<'_>) -> Result<(), ClaimError> {
        self.check_prepared_state(&prepared.state).await?;
        for path in prepared.paths {
            match tokio::fs::remove_file(path).await {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(ClaimError::Io(error)),
            }
        }
        Ok(())
    }

    /// List every live claim in the workspace, sorted by `expires_at`
    /// ascending (earliest-expiring first). Expired claims are skipped
    /// (not returned, not removed — purging is a separate concern).
    pub async fn list_active(&self) -> Result<Vec<Claim>, ClaimError> {
        Ok(self.list_active_with_stats().await?.0)
    }

    /// Map view of `list_active`, keyed by canonical key for identity-safe
    /// joins. Legacy claims without a stored key use their historical ID.
    pub async fn list_active_map(&self) -> Result<BTreeMap<String, Claim>, ClaimError> {
        let mut out = BTreeMap::new();
        for c in self.list_active().await? {
            let key = c.canonical_key.clone().unwrap_or_else(|| c.id.clone());
            out.insert(key, c);
        }
        Ok(out)
    }

    /// Like `list_active` but also returns the count of YAML files that
    /// were skipped because they failed to parse or exceeded the size cap.
    /// R2 audit MED (rust-pro + security): previous behaviour silently
    /// dropped malformed files — an attacker could plant a truncated file
    /// at `claims/<id>.yaml` to make an artifact appear unclaimed to the
    /// dispatcher while still holding live-ish state. Callers can surface
    /// the skip count so audit listings don't lie by omission.
    pub async fn list_active_with_stats(&self) -> Result<(Vec<Claim>, usize), ClaimError> {
        if !self.dir.exists() {
            return Ok((Vec::new(), 0));
        }
        let mut out = Vec::new();
        let mut skipped = 0usize;
        let mut rd = tokio::fs::read_dir(&self.dir).await?;
        while let Some(entry) = rd.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
                continue;
            }
            match read_bounded(&path).await {
                Ok(Some(raw)) => match serde_yaml::from_str::<Claim>(&raw) {
                    Ok(claim) if !claim.is_expired() => out.push(claim),
                    Ok(_) => { /* expired — drop silently; expected */ }
                    Err(e) => {
                        skipped += 1;
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "claim file failed to parse — skipped from list_active"
                        );
                    }
                },
                Ok(None) => { /* disappeared mid-scan — ignore */ }
                Err(e) => {
                    skipped += 1;
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "claim file read failed — skipped from list_active"
                    );
                }
            }
        }
        // Listing claims is read-only. Legacy migration must be performed by
        // a locked owner-aware operation, never as a read side effect.
        out.sort_by_key(|c| c.expires_at);
        Ok((out, skipped))
    }

}

async fn file_digest(path: &Path) -> Result<Option<[u8; 32]>, ClaimError> {
    let mut file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if file.metadata().await?.len() > MAX_CLAIM_FILE_BYTES {
        return Err(ClaimError::FileTooLarge);
    }
    Ok(Some(digest_claim_reader(&mut file).await?))
}

async fn digest_claim_reader(
    reader: &mut (impl tokio::io::AsyncRead + Unpin),
) -> Result<[u8; 32], ClaimError> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    let mut total = 0u64;
    loop {
        let limit = (MAX_CLAIM_FILE_BYTES - total + 1).min(buffer.len() as u64) as usize;
        let count = reader.read(&mut buffer[..limit]).await?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > MAX_CLAIM_FILE_BYTES {
            return Err(ClaimError::FileTooLarge);
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize().into())
}

/// Read a file with the MAX_CLAIM_FILE_BYTES cap enforced. Returns
/// `Ok(None)` for missing file; otherwise errors on size-violation or I/O
/// failure. Centralized so every read path (`get`, `list_active`) applies
/// the same protection.
async fn read_bounded(path: &Path) -> Result<Option<String>, ClaimError> {
    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ClaimError::Io(e)),
    };
    let len = file.metadata().await?.len();
    if len > MAX_CLAIM_FILE_BYTES {
        return Err(ClaimError::FileTooLarge);
    }
    let mut bytes = Vec::with_capacity(len as usize);
    file.take(MAX_CLAIM_FILE_BYTES + 1).read_to_end(&mut bytes).await?;
    if bytes.len() as u64 > MAX_CLAIM_FILE_BYTES {
        return Err(ClaimError::FileTooLarge);
    }
    let raw = String::from_utf8(bytes)
        .map_err(|error| ClaimError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error)))?;
    Ok(Some(raw))
}

/// Read the claim stored at a path, expired or not.
///
/// A missing file is `None`; a file that does not parse is an error, the
/// same way [`read_bounded`] reports an oversized one. The caller decides
/// what an expired claim means.
async fn read_claim_at(path: &Path) -> Result<Option<Claim>, ClaimError> {
    match read_bounded(path).await? {
        Some(raw) => Ok(Some(serde_yaml::from_str::<Claim>(&raw)?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn ws(tmp: &TempDir) -> PathBuf {
        tmp.path().join(".forgeplan")
    }

    #[test]
    fn claim_is_expired_matches_now() {
        let past = Claim {
            id: "PRD-001".into(),
            canonical_key: None,
            agent_id: "a/1".into(),
            claimed_at: Utc::now() - Duration::minutes(10),
            expires_at: Utc::now() - Duration::minutes(1),
            note: None,
        };
        assert!(past.is_expired());

        let future = Claim {
            expires_at: Utc::now() + Duration::minutes(5),
            ..past.clone()
        };
        assert!(!future.is_expired());
    }

    #[test]
    fn is_held_by_exact_match() {
        let c = Claim {
            id: "PRD-001".into(),
            canonical_key: None,
            agent_id: "orchestrator/1.0".into(),
            claimed_at: Utc::now(),
            expires_at: Utc::now() + Duration::minutes(30),
            note: None,
        };
        assert!(c.is_held_by("orchestrator/1.0"));
        assert!(!c.is_held_by("orchestrator/2.0"));
        assert!(!c.is_held_by("worker-1"));
    }

    #[tokio::test]
    async fn claim_creates_file_and_returns_claim() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        let claim = store
            .claim("PRD-001", "agent/1.0", DEFAULT_TTL, None)
            .await
            .unwrap();
        assert_eq!(claim.id, "PRD-001");
        assert_eq!(claim.agent_id, "agent/1.0");
        assert!(ws(&tmp).join("claims/PRD-001.yaml").exists());
    }

    #[tokio::test]
    async fn claim_uppercases_id_on_disk() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        store
            .claim("prd-002", "a/1", DEFAULT_TTL, None)
            .await
            .unwrap();
        assert!(ws(&tmp).join("claims/PRD-002.yaml").exists());
    }

    #[tokio::test]
    async fn claim_rejects_active_different_agent() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        store
            .claim("PRD-003", "agent-a/1", DEFAULT_TTL, None)
            .await
            .unwrap();
        let err = store
            .claim("PRD-003", "agent-b/1", DEFAULT_TTL, None)
            .await
            .unwrap_err();
        match err {
            ClaimError::AlreadyHeld { agent_id, .. } => assert_eq!(agent_id, "agent-a/1"),
            other => panic!("expected AlreadyHeld, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn claim_renews_same_agent() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        let first = store
            .claim("PRD-004", "a/1", Duration::minutes(5), None)
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let second = store
            .claim("PRD-004", "a/1", Duration::minutes(10), None)
            .await
            .unwrap();
        assert!(second.claimed_at > first.claimed_at);
        assert!(second.expires_at > first.expires_at);
    }

    #[tokio::test]
    async fn claim_takes_over_expired_claim() {
        // AC-3: expired claim is ignored, new agent can take it.
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        store.ensure_dir().await.unwrap();

        let past = Claim {
            id: "PRD-005".into(),
            canonical_key: None,
            agent_id: "dead-agent".into(),
            claimed_at: Utc::now() - Duration::minutes(20),
            expires_at: Utc::now() - Duration::minutes(5),
            note: None,
        };
        tokio::fs::write(
            ws(&tmp).join("claims/PRD-005.yaml"),
            serde_yaml::to_string(&past).unwrap(),
        )
        .await
        .unwrap();

        let new_claim = store
            .claim("PRD-005", "fresh-agent/1", DEFAULT_TTL, None)
            .await
            .unwrap();
        assert_eq!(new_claim.agent_id, "fresh-agent/1");
    }

    #[tokio::test]
    async fn claim_rejects_ttl_out_of_range() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        assert!(matches!(
            store
                .claim("PRD-006", "a/1", Duration::seconds(10), None)
                .await,
            Err(ClaimError::TtlOutOfRange(_))
        ));
        assert!(matches!(
            store
                .claim("PRD-006", "a/1", Duration::hours(48), None)
                .await,
            Err(ClaimError::TtlOutOfRange(_))
        ));
    }

    #[tokio::test]
    async fn claim_rejects_empty_ids_and_agents() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        assert!(matches!(
            store.claim("", "a/1", DEFAULT_TTL, None).await,
            Err(ClaimError::EmptyId)
        ));
        assert!(matches!(
            store.claim("PRD-007", "", DEFAULT_TTL, None).await,
            Err(ClaimError::EmptyAgent)
        ));
    }

    #[tokio::test]
    async fn get_returns_none_for_missing_or_expired() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        assert!(store.get("PRD-404").await.unwrap().is_none());

        store.ensure_dir().await.unwrap();
        let past = Claim {
            id: "PRD-405".into(),
            canonical_key: None,
            agent_id: "x".into(),
            claimed_at: Utc::now() - Duration::hours(1),
            expires_at: Utc::now() - Duration::seconds(1),
            note: None,
        };
        tokio::fs::write(
            ws(&tmp).join("claims/PRD-405.yaml"),
            serde_yaml::to_string(&past).unwrap(),
        )
        .await
        .unwrap();
        assert!(store.get("PRD-405").await.unwrap().is_none());
        assert!(
            store
                .get_including_expired("PRD-405")
                .await
                .unwrap()
                .is_some(),
            "raw read should surface expired entries"
        );
    }

    #[tokio::test]
    async fn release_by_owner_succeeds_and_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        store
            .claim("PRD-010", "a/1", DEFAULT_TTL, None)
            .await
            .unwrap();
        store.release("PRD-010", "a/1", false).await.unwrap();
        assert!(store.get("PRD-010").await.unwrap().is_none());
        // Second release no-ops (idempotent).
        store.release("PRD-010", "a/1", false).await.unwrap();
    }

    #[tokio::test]
    async fn release_by_wrong_agent_rejects_without_force() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        store
            .claim("PRD-011", "owner/1", DEFAULT_TTL, None)
            .await
            .unwrap();
        let err = store
            .release("PRD-011", "stranger/1", false)
            .await
            .unwrap_err();
        assert!(matches!(err, ClaimError::NotHeldByRequester { .. }));
        assert!(
            store.get("PRD-011").await.unwrap().is_some(),
            "claim must still be on disk after refused release"
        );
    }

    #[tokio::test]
    async fn release_force_overrides_agent_check() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        store
            .claim("PRD-012", "owner/1", DEFAULT_TTL, None)
            .await
            .unwrap();
        // force=true lets the orchestrator reap a stuck claim.
        store
            .release("PRD-012", "orchestrator/1", true)
            .await
            .unwrap();
        assert!(store.get("PRD-012").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn release_expired_is_allowed_without_force() {
        // Expired claims are "practically released" already; any agent may
        // tidy them up without --force.
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        store.ensure_dir().await.unwrap();
        let past = Claim {
            id: "PRD-013".into(),
            canonical_key: None,
            agent_id: "ghost".into(),
            claimed_at: Utc::now() - Duration::hours(2),
            expires_at: Utc::now() - Duration::minutes(5),
            note: None,
        };
        tokio::fs::write(
            ws(&tmp).join("claims/PRD-013.yaml"),
            serde_yaml::to_string(&past).unwrap(),
        )
        .await
        .unwrap();
        store.release("PRD-013", "anyone/1", false).await.unwrap();
        assert!(
            store
                .get_including_expired("PRD-013")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn list_active_sorted_by_expiry_ascending() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        store
            .claim("PRD-020", "a/1", Duration::hours(2), None)
            .await
            .unwrap();
        store
            .claim("PRD-021", "b/1", Duration::minutes(30), None)
            .await
            .unwrap();
        store
            .claim("PRD-022", "c/1", Duration::hours(1), None)
            .await
            .unwrap();
        let active = store.list_active().await.unwrap();
        assert_eq!(active.len(), 3);
        assert_eq!(active[0].id, "PRD-021"); // earliest expiry
        assert_eq!(active[2].id, "PRD-020"); // latest expiry
    }

    #[tokio::test]
    async fn list_active_skips_expired() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        store.ensure_dir().await.unwrap();

        store
            .claim("PRD-030", "live/1", DEFAULT_TTL, None)
            .await
            .unwrap();

        let expired = Claim {
            id: "PRD-031".into(),
            canonical_key: None,
            agent_id: "stale/1".into(),
            claimed_at: Utc::now() - Duration::hours(2),
            expires_at: Utc::now() - Duration::seconds(1),
            note: None,
        };
        tokio::fs::write(
            ws(&tmp).join("claims/PRD-031.yaml"),
            serde_yaml::to_string(&expired).unwrap(),
        )
        .await
        .unwrap();

        let active = store.list_active().await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "PRD-030");
    }

    #[tokio::test]
    async fn list_active_handles_missing_dir() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        // Directory never created — must not error.
        assert!(store.list_active().await.unwrap().is_empty());
    }

    // ── R2 audit hardening ────────────────────────────────────────────

    #[test]
    fn validate_id_rejects_path_traversal() {
        // R2 audit HIGH #1 (security): before this guard, id="../../etc/passwd"
        // would escape the claims dir via Path::join.
        for bad in [
            "..",
            "../foo",
            "../../etc/passwd",
            "foo/bar",
            "foo\\bar",
            "foo\0bar",
            ".hidden",
            "",
            "123-leading-digit",
        ] {
            assert!(validate_id(bad).is_err(), "expected rejection of {bad:?}");
        }
    }

    #[test]
    fn validate_id_accepts_forgeplan_shapes() {
        for good in ["PRD-001", "EPIC-042", "note-slug", "PROB-036", "mem-abc"] {
            assert!(validate_id(good).is_ok(), "expected accept of {good:?}");
        }
    }

    #[tokio::test]
    async fn claim_rejects_traversal_id_before_write() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        let err = store
            .claim("../../etc/passwd", "a/1", DEFAULT_TTL, None)
            .await
            .unwrap_err();
        assert!(matches!(err, ClaimError::InvalidId(_)));
        // No file must have been written anywhere under workspace.
        let tmp_abs = tmp.path().to_path_buf();
        let root_entries: Vec<_> = std::fs::read_dir(&tmp_abs)
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        // The claims subdir might not even exist yet, and definitely no
        // "etc/passwd" artifact should appear at root.
        for e in root_entries {
            let name = e.file_name().to_string_lossy().to_string();
            assert!(
                !name.contains("passwd") && !name.contains("etc"),
                "traversal attempt leaked: {name}"
            );
        }
    }

    #[tokio::test]
    async fn release_rejects_traversal_id() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        let err = store
            .release("../sensitive-file", "a/1", false)
            .await
            .unwrap_err();
        assert!(matches!(err, ClaimError::InvalidId(_)));
    }

    #[tokio::test]
    async fn release_checks_empty_agent_before_filesystem() {
        // R2 audit HIGH #2 (rust-pro): empty agent + missing claim must
        // surface EmptyAgent, not a silent Ok.
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        let err = store.release("PRD-099", "", false).await.unwrap_err();
        assert!(matches!(err, ClaimError::EmptyAgent));
    }

    #[tokio::test]
    async fn get_rejects_oversized_file() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        store.ensure_dir().await.unwrap();
        let path = ws(&tmp).join("claims/PRD-999.yaml");
        // 1 MB of YAML-parseable junk — far above MAX_CLAIM_FILE_BYTES (64 KB).
        tokio::fs::write(&path, "# ".repeat(600_000)).await.unwrap();
        let err = store.get("PRD-999").await.unwrap_err();
        assert!(matches!(err, ClaimError::FileTooLarge));
    }

    #[tokio::test]
    async fn list_active_with_stats_counts_malformed() {
        // R2 audit MED (rust-pro + security): malformed files must not be
        // invisible — surface a count so orchestrators see the omission.
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        store
            .claim("PRD-200", "a/1", DEFAULT_TTL, None)
            .await
            .unwrap();
        tokio::fs::write(ws(&tmp).join("claims/PRD-201.yaml"), "{{{ not yaml")
            .await
            .unwrap();

        let (active, skipped) = store.list_active_with_stats().await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(skipped, 1);
    }

    #[tokio::test]
    async fn list_active_map_keys_by_id() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        store
            .claim("PRD-300", "a/1", DEFAULT_TTL, None)
            .await
            .unwrap();
        store
            .claim("PRD-301", "b/1", DEFAULT_TTL, None)
            .await
            .unwrap();

        let map = store.list_active_map().await.unwrap();
        assert!(map.contains_key("PRD-300"));
        assert!(map.contains_key("PRD-301"));
        assert_eq!(map["PRD-300"].agent_id, "a/1");
    }
    #[tokio::test]
    async fn conflicting_canonical_keys_refuse_claim_without_writing_sidecars() {
        let tmp = TempDir::new().unwrap();
        let workspace = ws(&tmp);
        let prds = workspace.join("prds");
        tokio::fs::create_dir_all(&prds).await.unwrap();
        for (name, id) in [("first.md", "PRD-001"), ("second.md", "PRD-002")] {
            tokio::fs::write(
                prds.join(name),
                format!(
                    "---\nid: {id}\nslug: prd-repeated-key\ntitle: {id}\nkind: prd\n---\nbody\n"
                ),
            )
            .await
            .unwrap();
        }
        let store = ClaimStore::new(workspace.clone());

        let error = store
            .claim("prd-repeated-key", "agent", DEFAULT_TTL, None)
            .await
            .unwrap_err();
        assert!(matches!(error, ClaimError::Resolution(_)));
        assert!(!workspace.join("claims").exists());
    }

    #[tokio::test]
    async fn incomplete_document_scan_refuses_claim_without_writing_sidecars() {
        let tmp = TempDir::new().unwrap();
        let workspace = ws(&tmp);
        let prds = workspace.join("prds");
        tokio::fs::create_dir_all(&prds).await.unwrap();
        tokio::fs::write(prds.join("broken.md"), "not frontmatter")
            .await
            .unwrap();
        let store = ClaimStore::new(workspace.clone());

        let error = store
            .claim("PRD-001", "agent", DEFAULT_TTL, None)
            .await
            .unwrap_err();
        assert!(matches!(error, ClaimError::Io(_)));
        assert!(!workspace.join("claims").exists());
    }

    #[tokio::test]
    async fn resolved_claim_cannot_bypass_an_incomplete_document_scan() {
        let tmp = TempDir::new().unwrap();
        let workspace = ws(&tmp);
        let store = ClaimStore::new(&workspace);
        let key = "prd-alpha";
        store.claim_for(key, "PRD-001", "agent", DEFAULT_TTL, None).await.unwrap();
        let claim_path = store.path_for(key);
        let before = tokio::fs::read(&claim_path).await.unwrap();
        tokio::fs::create_dir_all(workspace.join("prds")).await.unwrap();
        tokio::fs::write(workspace.join("prds/broken.md"), "not frontmatter").await.unwrap();

        assert!(store.get_for(key, "PRD-001").await.is_err());
        assert!(store.release_for(key, "PRD-001", "agent", false).await.is_err());
        assert_eq!(tokio::fs::read(claim_path).await.unwrap(), before);
    }

    #[tokio::test]
    async fn list_active_map_keeps_shared_alias_claims_by_canonical_key() {
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        let alpha = "prd-alpha-k11111111111111111111111111111111";
        let beta = "prd-beta-k22222222222222222222222222222222";
        store
            .claim_for(alpha, "PRD-001", "alpha-agent", DEFAULT_TTL, None)
            .await
            .unwrap();
        store
            .claim_for(beta, "PRD-001", "beta-agent", DEFAULT_TTL, None)
            .await
            .unwrap();

        let map = store.list_active_map().await.unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map[alpha].agent_id, "alpha-agent");
        assert_eq!(map[beta].agent_id, "beta-agent");
    }

    #[tokio::test]
    async fn atomic_write_leaves_no_temp_files_on_success() {
        // R2 audit MED: atomic rename pattern must not accumulate orphans.
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        for i in 0..5 {
            store
                .claim(&format!("PRD-40{i}"), "agent/1", DEFAULT_TTL, None)
                .await
                .unwrap();
        }

        let claims_dir = ws(&tmp).join("claims");
        let mut rd = tokio::fs::read_dir(&claims_dir).await.unwrap();
        while let Some(entry) = rd.next_entry().await.unwrap() {
            let name = entry.file_name();
            let s = name.to_string_lossy();
            assert!(
                !s.starts_with('.') || s.ends_with(".yaml"),
                "tempfile leaked: {s}"
            );
            assert!(!s.contains(".tmp."), "tempfile leaked: {s}");
        }
    }

    // ── PROB-066 hardening: agent-string validation parity with MCP ──────

    #[test]
    fn validate_agent_id_accepts_alphanumeric_hyphen() {
        // PROB-066 positive class — the operator-friendly form recommended
        // in the CLI Fix hint must round-trip cleanly.
        for good in [
            "smoke-test-v1",
            "worker-1",
            "agent_42",
            "Orchestrator",
            "ci_runner",
            "x", // single-char minimum
        ] {
            assert!(
                validate_agent_id(good).is_ok(),
                "expected accept of {good:?}"
            );
        }
    }

    #[test]
    fn validate_agent_id_rejects_slash() {
        // PROB-066 core: the `/` delimiter that AgentIdentity::new rejects
        // must also be rejected here. `smoke-test/v1` was the enshrined
        // CLI form that motivated this fix.
        let err = validate_agent_id("smoke-test/v1").unwrap_err();
        assert!(matches!(err, ClaimError::InvalidAgent(_)));
    }

    #[test]
    fn validate_agent_id_rejects_control_chars() {
        // YAML-injection vector: an unsanitized newline в agent_id
        // corrupted the on-disk .yaml body.
        for bad in [
            "foo\nbar",      // LF
            "tab\there",     // TAB
            "bell\u{0007}!", // BEL
            "cr\rlf",        // CR
        ] {
            let err = validate_agent_id(bad).unwrap_err();
            assert!(
                matches!(err, ClaimError::InvalidAgent(_)),
                "expected InvalidAgent for {bad:?}, got {err:?}"
            );
        }
    }

    #[test]
    fn validate_agent_id_rejects_bidi_override() {
        // Terminal-spoof vector: when `forgeplan claims` echoes agent_id,
        // a bidi-override sequence inverts the rendered string. Reject all
        // ZWJ / RTL / BOM / tag / variation-selector classes that
        // is_identity_char_forbidden covers.
        for bad in [
            "orch\u{202E}drawkcab", // RLO
            "agent\u{200B}zwsp",    // ZWSP
            "client\u{200D}zwj",    // ZWJ
            "bom\u{FEFF}prefix",    // BOM
            "tag\u{E0041}chars",    // TAG-A
        ] {
            let err = validate_agent_id(bad).unwrap_err();
            assert!(
                matches!(err, ClaimError::InvalidAgent(_)),
                "expected InvalidAgent for {bad:?}, got {err:?}"
            );
        }
    }

    #[test]
    fn validate_agent_id_caps_length_at_64() {
        // Length cap mirrors MAX_FIELD_LEN in artifact::identity. At the
        // boundary the value is accepted; one byte over → AgentTooLong.
        let at_boundary = "x".repeat(64);
        assert!(validate_agent_id(&at_boundary).is_ok());

        let over = "x".repeat(65);
        let err = validate_agent_id(&over).unwrap_err();
        assert!(matches!(err, ClaimError::AgentTooLong(65)));
    }

    #[test]
    fn validate_agent_id_rejects_empty_and_whitespace() {
        // Empty and whitespace-only must surface EmptyAgent (preserving the
        // existing typed-error contract that callers / hints depend on).
        for bad in ["", "   ", "\t\t", "\n  "] {
            let err = validate_agent_id(bad).unwrap_err();
            // Note: `\n` and `\t` are control chars, but trim() strips them
            // first so the empty branch fires.
            assert!(
                matches!(err, ClaimError::EmptyAgent),
                "expected EmptyAgent for {bad:?}, got {err:?}"
            );
        }
    }

    #[tokio::test]
    async fn claim_rejects_newline_agent_at_store_layer() {
        // PROB-066 defense-in-depth: even if a caller bypasses the CLI
        // strict guard, ClaimStore::claim still refuses control / bidi /
        // NUL classes (newline below would otherwise corrupt the YAML body).
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        let err = store
            .claim("PRD-066", "evil\nx: y", DEFAULT_TTL, None)
            .await
            .unwrap_err();
        assert!(matches!(err, ClaimError::InvalidAgent(_)));
        // No file must have been written.
        assert!(!ws(&tmp).join("claims/PRD-066.yaml").exists());
    }

    #[tokio::test]
    async fn claim_store_still_accepts_canonical_slash_form() {
        // MCP write-stamping path passes `AgentIdentity::as_frontmatter_value()`
        // — e.g. `claude-code/1.0.50`. The relaxed store-layer validator must
        // keep that legitimate shape working; the strict CLI-side filter is
        // what rejects user-typed `smoke-test/v1`.
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        let claim = store
            .claim("PRD-068", "claude-code/1.0.50", DEFAULT_TTL, None)
            .await
            .unwrap();
        assert_eq!(claim.agent_id, "claude-code/1.0.50");
    }

    #[tokio::test]
    async fn release_rejects_control_char_agent_without_force() {
        // PROB-066 A-3: validation must hit both claim() and release().
        let tmp = TempDir::new().unwrap();
        let store = ClaimStore::new(ws(&tmp));
        store
            .claim("PRD-067", "owner-1", DEFAULT_TTL, None)
            .await
            .unwrap();
        let err = store
            .release("PRD-067", "evil\nx: y", false)
            .await
            .unwrap_err();
        assert!(
            matches!(err, ClaimError::InvalidAgent(_)),
            "expected InvalidAgent, got {err:?}"
        );
        // The original claim is still on disk — release was refused before
        // any filesystem mutation.
        assert!(store.get("PRD-067").await.unwrap().is_some());
    }

    // ── ADR-030 — the claim is addressed by the canonical key ──────────────

    /// A workspace holding the given documents, and the temp directory that
    /// holds it.
    ///
    /// The documents are what the resolver reads, so a claim written against
    /// one of them is addressed by that document's canonical key.
    fn workspace_with(documents: &[(&str, &str)]) -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = ws.join("prds");
        std::fs::create_dir_all(&dir).unwrap();
        for (name, body) in documents {
            std::fs::write(dir.join(name), body).unwrap();
        }
        (tmp, ws)
    }

    /// One document, as the projection writes it.
    fn document(id: &str, slug: Option<&str>) -> String {
        let slug_line = slug.map(|s| format!("slug: {s}\n")).unwrap_or_default();
        format!("---\nid: {id}\n{slug_line}kind: prd\ntitle: Doc\nstatus: draft\n---\n\nBody.\n")
    }

    #[tokio::test]
    async fn legacy_claim_conflicts_leave_files_unchanged() {
        let (_tmp, ws) = workspace_with(&[("a.md", &document("PRD-001", Some("prd-auth-system")))]);
        let claims = ws.join("claims");
        std::fs::create_dir_all(&claims).unwrap();
        let legacy = claims.join("PRD-001.yaml");
        let contents = b"id: PRD-001\nagent_id: owner/1\nclaimed_at: 2026-01-01T00:00:00+00:00\nexpires_at: 2099-01-01T00:00:00+00:00\n";
        std::fs::write(&legacy, contents).unwrap();
        let store = ClaimStore::new(&ws);
        let guard = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();

        assert!(matches!(
            store.prepare_claim_for("prd-auth-system", "prd-001", "other/1", DEFAULT_TTL, None, &guard).await,
            Err(ClaimError::AlreadyHeld { .. })
        ));
        assert!(matches!(
            store.prepare_release_for("prd-auth-system", "prd-001", "other/1", false, &guard).await,
            Err(ClaimError::NotHeldByRequester { .. })
        ));
        assert!(matches!(
            store.claim("prd-001", "other/1", DEFAULT_TTL, None).await,
            Err(ClaimError::AlreadyHeld { .. })
        ));
        assert!(matches!(
            store.release("prd-001", "other/1", false).await,
            Err(ClaimError::NotHeldByRequester { .. })
        ));
        assert_eq!(std::fs::read(&legacy).unwrap().as_slice(), contents);
        assert_eq!(std::fs::read_dir(&claims).unwrap().count(), 1);
        assert!(!claims.join("prd-auth-system.yaml").exists());
    }

    #[tokio::test]
    async fn keyed_claim_takes_priority_over_legacy_claim_on_conflict() {
        let (_tmp, ws) = workspace_with(&[("a.md", &document("PRD-001", Some("prd-auth-system")))]);
        let claims = ws.join("claims");
        std::fs::create_dir_all(&claims).unwrap();
        let legacy = claims.join("PRD-001.yaml");
        let keyed = claims.join("prd-auth-system.yaml");
        let claim = |agent: &str| format!("id: PRD-001\nagent_id: {agent}\nclaimed_at: 2026-01-01T00:00:00+00:00\nexpires_at: 2099-01-01T00:00:00+00:00\n");
        let legacy_bytes = claim("legacy/1");
        let keyed_bytes = claim("keyed/1");
        std::fs::write(&legacy, &legacy_bytes).unwrap();
        std::fs::write(&keyed, &keyed_bytes).unwrap();
        let store = ClaimStore::new(&ws);
        let guard = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();

        assert!(matches!(
            store.prepare_claim_for("prd-auth-system", "prd-001", "legacy/1", DEFAULT_TTL, None, &guard).await,
            Err(ClaimError::AlreadyHeld { agent_id, .. }) if agent_id == "keyed/1"
        ));
        assert!(matches!(
            store.prepare_release_for("prd-auth-system", "prd-001", "legacy/1", false, &guard).await,
            Err(ClaimError::NotHeldByRequester { held_by, .. }) if held_by == "keyed/1"
        ));
        assert_eq!(std::fs::read_to_string(legacy).unwrap(), legacy_bytes);
        assert_eq!(std::fs::read_to_string(keyed).unwrap(), keyed_bytes);
        assert_eq!(std::fs::read_dir(claims).unwrap().count(), 2);
    }

    #[tokio::test]
    async fn prepared_claim_migrates_only_when_applied_and_release_noop_is_read_only() {
        let (_tmp, ws) = workspace_with(&[("a.md", &document("PRD-001", Some("prd-auth-system")))]);
        let claims = ws.join("claims");
        let store = ClaimStore::new(&ws);
        let guard = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let absent = store.prepare_release_for("prd-auth-system", "PRD-001", "owner/1", false, &guard).await.unwrap();
        assert!(!absent.is_mutation());
        assert!(!claims.exists());
        let plan = store.prepare_claim_for("prd-auth-system", "PRD-001", "owner/1", DEFAULT_TTL, None, &guard).await.unwrap();
        assert!(!claims.exists());
        drop(plan);
        std::fs::create_dir_all(&claims).unwrap();
        let legacy = claims.join("PRD-001.yaml");
        let bytes = b"id: PRD-001\nagent_id: owner/1\nclaimed_at: 2026-01-01T00:00:00+00:00\nexpires_at: 2099-01-01T00:00:00+00:00\n";
        std::fs::write(&legacy, bytes).unwrap();

        let plan = store.prepare_claim("prd-001", " owner/1 ", DEFAULT_TTL, None, &guard).await.unwrap();
        assert_eq!(std::fs::read(&legacy).unwrap().as_slice(), bytes);
        assert!(!claims.join("prd-auth-system.yaml").exists());
        let renewed = store.apply_prepared_claim(plan).await.unwrap();
        assert_eq!(renewed.agent_id, "owner/1");
        assert!(!legacy.exists());
        assert!(claims.join("prd-auth-system.yaml").exists());
        let release = store.prepare_release("prd-001", "owner/1", false, &guard).await.unwrap();
        assert!(release.is_mutation());
        store.apply_prepared_release(release).await.unwrap();
        assert!(!claims.join("prd-auth-system.yaml").exists());
    }

    #[tokio::test]
    async fn legacy_expiry_and_force_are_checked_before_mutation() {
        let (_tmp, ws) = workspace_with(&[("a.md", &document("PRD-001", Some("prd-auth-system")))]);
        let claims = ws.join("claims");
        std::fs::create_dir_all(&claims).unwrap();
        let legacy = claims.join("PRD-001.yaml");
        let expired = b"id: PRD-001\nagent_id: owner/1\nclaimed_at: 2000-01-01T00:00:00+00:00\nexpires_at: 2000-01-01T00:01:00+00:00\n";
        std::fs::write(&legacy, expired).unwrap();
        let store = ClaimStore::new(&ws);
        let guard = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let plan = store.prepare_claim_for("prd-auth-system", "PRD-001", "new/1", DEFAULT_TTL, None, &guard).await.unwrap();
        assert_eq!(std::fs::read(&legacy).unwrap().as_slice(), expired);
        store.apply_prepared_claim(plan).await.unwrap();
        assert!(!legacy.exists());

        let plan = store.prepare_release_for("prd-auth-system", "PRD-001", "", true, &guard).await.unwrap();
        assert!(plan.is_mutation());
        assert!(claims.join("prd-auth-system.yaml").exists());
        store.apply_prepared_release(plan).await.unwrap();
        assert!(!claims.join("prd-auth-system.yaml").exists());
    }

    #[tokio::test]
    async fn prepared_plans_reject_other_workspaces_and_stale_files_or_identity() {
        let (_a, ws_a) = workspace_with(&[("a.md", &document("PRD-001", Some("prd-auth-system")))]);
        let (_b, ws_b) = workspace_with(&[("a.md", &document("PRD-001", Some("prd-auth-system")))]);
        let a = ClaimStore::new(&ws_a);
        let b = ClaimStore::new(&ws_b);
        let guard = crate::workspace::lock::acquire_workspace_lock(&ws_a).await.unwrap();
        assert!(matches!(
            b.prepare_claim("PRD-001", "owner", DEFAULT_TTL, None, &guard).await,
            Err(ClaimError::PreparedStateChanged)
        ));
        let plan = a.prepare_claim("PRD-001", "owner", DEFAULT_TTL, None, &guard).await.unwrap();
        assert!(matches!(b.apply_prepared_claim(plan).await, Err(ClaimError::PreparedStateChanged)));
        assert!(!ws_a.join("claims").exists());
        assert!(!ws_b.join("claims").exists());

        let plan = a.prepare_claim("PRD-001", "owner", DEFAULT_TTL, None, &guard).await.unwrap();
        a.claim_for("prd-auth-system", "PRD-001", "other", DEFAULT_TTL, None).await.unwrap();
        assert!(matches!(a.apply_prepared_claim(plan).await, Err(ClaimError::PreparedStateChanged)));
        let plan = a.prepare_release("PRD-001", "other", false, &guard).await.unwrap();
        assert!(matches!(b.apply_prepared_release(plan).await, Err(ClaimError::PreparedStateChanged)));
        assert!(ws_a.join("claims/prd-auth-system.yaml").exists());
        let plan = a.prepare_release("PRD-001", "other", false, &guard).await.unwrap();
        a.claim_for("prd-auth-system", "PRD-001", "other", DEFAULT_TTL, Some("renewed".into())).await.unwrap();
        assert!(matches!(a.apply_prepared_release(plan).await, Err(ClaimError::PreparedStateChanged)));
        assert_eq!(a.get("PRD-001").await.unwrap().unwrap().note.as_deref(), Some("renewed"));

        let plan = a.prepare_release("PRD-001", "other", false, &guard).await.unwrap();
        std::fs::write(ws_a.join("prds/a.md"), document("PRD-002", Some("prd-auth-system"))).unwrap();
        assert!(matches!(a.apply_prepared_release(plan).await, Err(ClaimError::PreparedStateChanged)));
        assert!(ws_a.join("claims/prd-auth-system.yaml").exists());
    }

    #[tokio::test]
    async fn relative_store_prepares_only_canonical_workspace_paths() {
        let tmp = tempfile::Builder::new().prefix("claim-path-binding-")
            .tempdir_in(std::env::current_dir().unwrap()).unwrap();
        let workspace = tmp.path().join(".forgeplan");
        std::fs::create_dir_all(workspace.join("prds")).unwrap();
        std::fs::write(workspace.join("prds/a.md"), document("PRD-001", Some("prd-auth-system"))).unwrap();
        let relative = PathBuf::from(tmp.path().file_name().unwrap()).join(".forgeplan");
        let store = ClaimStore::new(relative);
        let absolute = ClaimStore::new(&workspace);
        let guard = crate::workspace::lock::acquire_workspace_lock(&workspace).await.unwrap();
        let plan = store.prepare_claim("PRD-001", "owner", DEFAULT_TTL, None, &guard).await.unwrap();
        let canonical = workspace.canonicalize().unwrap();
        assert!(plan.keyed_path.is_absolute() && plan.keyed_path.starts_with(&canonical));
        assert!(plan.state.files.iter().all(|(path, _)| path.is_absolute() && path.starts_with(&canonical)));
        absolute.apply_prepared_claim(plan).await.unwrap();
        let release = store.prepare_release("PRD-001", "owner", false, &guard).await.unwrap();
        assert!(release.paths.iter().all(|path| path.is_absolute() && path.starts_with(&canonical)));
        assert!(release.state.files.iter().all(|(path, _)| path.is_absolute() && path.starts_with(&canonical)));
        absolute.apply_prepared_release(release).await.unwrap();
        assert!(!workspace.join("claims/prd-auth-system.yaml").exists());
    }

    #[tokio::test]
    async fn prepared_operations_bound_sidecar_fingerprints_and_growing_streams() {
        let (_tmp, ws) = workspace_with(&[("a.md", &document("PRD-001", Some("prd-auth-system")))]);
        let claims = ws.join("claims");
        std::fs::create_dir_all(&claims).unwrap();
        let legacy = claims.join("PRD-001.yaml");
        std::fs::write(&legacy, vec![b'x'; MAX_CLAIM_FILE_BYTES as usize + 1]).unwrap();
        let store = ClaimStore::new(&ws);
        let guard = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        assert!(matches!(
            store.prepare_claim("PRD-001", "owner", DEFAULT_TTL, None, &guard).await,
            Err(ClaimError::FileTooLarge)
        ));
        assert!(matches!(
            store.prepare_release("PRD-001", "", true, &guard).await,
            Err(ClaimError::FileTooLarge)
        ));
        assert_eq!(std::fs::metadata(&legacy).unwrap().len(), MAX_CLAIM_FILE_BYTES + 1);
        assert!(!claims.join("prd-auth-system.yaml").exists());
        let mut stream = tokio::io::AsyncReadExt::take(tokio::io::repeat(0), MAX_CLAIM_FILE_BYTES + 1);
        assert!(matches!(digest_claim_reader(&mut stream).await, Err(ClaimError::FileTooLarge)));
    }

    #[tokio::test]
    async fn release_checks_and_removes_uniquely_owned_shadow_claims() {
        let (_tmp, ws) = workspace_with(&[("a.md", &document("PRD-001", Some("prd-auth-system")))]);
        let claims = ws.join("claims");
        std::fs::create_dir_all(&claims).unwrap();
        let keyed = claims.join("prd-auth-system.yaml");
        let legacy = claims.join("PRD-001.yaml");
        let claim = |agent: &str| format!("id: PRD-001\nagent_id: {agent}\nclaimed_at: 2026-01-01T00:00:00+00:00\nexpires_at: 2099-01-01T00:00:00+00:00\n");
        std::fs::write(&keyed, claim("owner")).unwrap();
        std::fs::write(&legacy, claim("other")).unwrap();
        let store = ClaimStore::new(&ws);
        let guard = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        assert!(matches!(
            store.prepare_release("PRD-001", "owner", false, &guard).await,
            Err(ClaimError::NotHeldByRequester { held_by, .. }) if held_by == "other"
        ));
        assert!(keyed.exists() && legacy.exists());
        let plan = store.prepare_release("PRD-001", "", true, &guard).await.unwrap();
        store.apply_prepared_release(plan).await.unwrap();
        assert!(!keyed.exists() && !legacy.exists());
        assert!(store.get("PRD-001").await.unwrap().is_none());
        std::fs::write(&keyed, claim("owner")).unwrap();
        std::fs::write(&legacy, claim("owner")).unwrap();
        let plan = store.prepare_release("PRD-001", "owner", false, &guard).await.unwrap();
        store.apply_prepared_release(plan).await.unwrap();
        assert!(!keyed.exists() && !legacy.exists());
        assert!(store.get("PRD-001").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn release_preserves_ambiguous_or_foreign_shadow_claims() {
        let (_tmp, ws) = workspace_with(&[("a.md", &document("PRD-001", Some("prd-auth-system")))]);
        let claims = ws.join("claims");
        std::fs::create_dir_all(&claims).unwrap();
        let keyed = claims.join("prd-auth-system.yaml");
        let legacy = claims.join("PRD-001.yaml");
        let base = "id: PRD-001\nagent_id: owner\nclaimed_at: 2026-01-01T00:00:00+00:00\nexpires_at: 2099-01-01T00:00:00+00:00\n";
        std::fs::write(&keyed, base).unwrap();
        std::fs::write(&legacy, format!("{base}canonical_key: prd-foreign\n")).unwrap();
        let store = ClaimStore::new(&ws);
        let guard = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        assert!(matches!(store.prepare_release("PRD-001", "", true, &guard).await, Err(ClaimError::Resolution(_))));
        assert!(keyed.exists() && legacy.exists());
        std::fs::write(&legacy, base).unwrap();
        std::fs::write(ws.join("prds/b.md"), document("PRD-001", Some("prd-other"))).unwrap();
        assert!(store.prepare_release_for("prd-auth-system", "PRD-001", "", true, &guard).await.is_err());
        assert!(keyed.exists() && legacy.exists());
    }

    #[tokio::test]
    async fn two_documents_sharing_an_alias_keep_their_own_claim() {
        // The pilot corpus holds documents that share one number across two
        // projects. A claim named after the alias gave those two documents
        // one row, so one agent's claim blocked the other's work.
        let (_tmp, ws) = workspace_with(&[
            ("a.md", &document("PRD-001", Some("prd-first-document"))),
            ("b.md", &document("PRD-001", Some("prd-second-document"))),
        ]);
        let store = ClaimStore::new(&ws);

        store
            .claim_for("prd-first-document", "PRD-001", "a/1", DEFAULT_TTL, None)
            .await
            .unwrap();
        store
            .claim_for("prd-second-document", "PRD-001", "b/1", DEFAULT_TTL, None)
            .await
            .unwrap();

        let claims = ws.join("claims");
        assert!(
            claims.join("prd-first-document.yaml").exists(),
            "the first document's claim is filed under its own key"
        );
        assert!(
            claims.join("prd-second-document.yaml").exists(),
            "the second document's claim is filed under its own key"
        );
        assert!(
            !claims.join("PRD-001.yaml").exists(),
            "the shared alias names no claim file"
        );

        // The two claims are independent: the second holder does not block
        // the first document.
        let err = store
            .claim_for("prd-second-document", "PRD-001", "c/1", DEFAULT_TTL, None)
            .await
            .unwrap_err();
        assert!(matches!(err, ClaimError::AlreadyHeld { .. }), "{err:?}");
    }

    #[tokio::test]
    async fn the_alias_and_the_key_address_one_claim() {
        let (_tmp, ws) = workspace_with(&[("a.md", &document("PRD-001", Some("prd-auth-system")))]);
        let store = ClaimStore::new(&ws);

        let by_alias = store
            .claim("PRD-001", "a/1", DEFAULT_TTL, None)
            .await
            .unwrap();
        let by_key = store
            .claim("prd-auth-system", "a/1", DEFAULT_TTL, None)
            .await
            .unwrap();

        assert_eq!(
            by_alias.canonical_key.as_deref(),
            Some("prd-auth-system"),
            "the claim records the document's key"
        );
        assert_eq!(by_alias.canonical_key, by_key.canonical_key);
        assert_eq!(by_alias.id, by_key.id);

        let claims = ws.join("claims");
        assert!(claims.join("prd-auth-system.yaml").exists());
        assert_eq!(
            std::fs::read_dir(&claims).unwrap().count(),
            1,
            "one document, one claim row"
        );
    }

    #[tokio::test]
    async fn a_legacy_claim_file_is_read_without_migration() {
        let (_tmp, ws) = workspace_with(&[("a.md", &document("PRD-001", Some("prd-auth-system")))]);
        let claims = ws.join("claims");
        std::fs::create_dir_all(&claims).unwrap();
        // A claim written by the old code: named after the display alias.
        std::fs::write(
            claims.join("PRD-001.yaml"),
            "id: PRD-001\nagent_id: old/1\nclaimed_at: 2026-01-01T00:00:00+00:00\n\
             expires_at: 2099-01-01T00:00:00+00:00\n",
        )
        .unwrap();

        let store = ClaimStore::new(&ws);
        let found = store
            .get("PRD-001")
            .await
            .unwrap()
            .expect("the legacy claim is still found");
        assert_eq!(found.agent_id, "old/1");
        assert_eq!(
            found.canonical_key.as_deref(),
            Some("prd-auth-system"),
            "the claim names the document it belongs to"
        );

        assert!(!claims.join("prd-auth-system.yaml").exists());
        assert!(claims.join("PRD-001.yaml").exists());
    }

    #[tokio::test]
    async fn a_legacy_claim_file_is_found_by_its_key_without_migration() {
        let (_tmp, ws) = workspace_with(&[("a.md", &document("PRD-001", Some("prd-auth-system")))]);
        let claims = ws.join("claims");
        std::fs::create_dir_all(&claims).unwrap();
        std::fs::write(
            claims.join("PRD-001.yaml"),
            "id: PRD-001\nagent_id: old/1\nclaimed_at: 2026-01-01T00:00:00+00:00\n\
             expires_at: 2099-01-01T00:00:00+00:00\n",
        )
        .unwrap();

        let store = ClaimStore::new(&ws);
        let found = store
            .get("prd-auth-system")
            .await
            .unwrap()
            .expect("the legacy claim is found under the key too");
        assert_eq!(found.agent_id, "old/1");
        assert!(!claims.join("prd-auth-system.yaml").exists());
        assert!(claims.join("PRD-001.yaml").exists());
    }

    #[tokio::test]
    async fn a_claim_file_name_is_never_built_from_the_alias_alone() {
        // The file name carries the key, escaped. A key that carries a
        // separator still names exactly one file inside the claims
        // directory.
        let (_tmp, ws) = workspace_with(&[("a.md", &document("PRD-001", Some("prd-auth-system")))]);
        let store = ClaimStore::new(&ws);
        store
            .claim_for(
                "forgeplan/plugins/x--PRD-074",
                "PRD-074",
                "a/1",
                DEFAULT_TTL,
                None,
            )
            .await
            .unwrap();

        let claims = ws.join("claims");
        assert!(
            claims
                .join("forgeplan%2Fplugins%2Fx--PRD-074.yaml")
                .exists(),
            "the key is escaped into one file name"
        );
        assert!(
            !claims.join("forgeplan").exists(),
            "no directory is created for the separator"
        );
    }
}
