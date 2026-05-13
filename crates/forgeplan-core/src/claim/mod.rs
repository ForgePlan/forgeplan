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
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    pub id: String,
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
#[derive(Debug, Clone)]
#[must_use = "ClaimStore is a lightweight handle — bind it before calling methods"]
pub struct ClaimStore {
    dir: PathBuf,
}

impl ClaimStore {
    /// `workspace` is the `.forgeplan/` directory. The store creates
    /// `<workspace>/claims/` lazily on first write.
    pub fn new(workspace: impl AsRef<Path>) -> Self {
        Self {
            dir: workspace.as_ref().join("claims"),
        }
    }

    fn path_for(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{}.yaml", id.to_uppercase()))
    }

    /// Ensure the claims directory exists. Called before any write.
    async fn ensure_dir(&self) -> Result<(), ClaimError> {
        tokio::fs::create_dir_all(&self.dir).await?;
        Ok(())
    }

    /// Read a claim if present AND not expired. Expired claims are treated
    /// as absent so downstream logic can acquire without a separate purge
    /// step (AC-3).
    pub async fn get(&self, id: &str) -> Result<Option<Claim>, ClaimError> {
        validate_id(id)?;
        let path = self.path_for(id);
        let raw = match read_bounded(&path).await? {
            Some(s) => s,
            None => return Ok(None),
        };
        let claim: Claim = serde_yaml::from_str(&raw)?;
        if claim.is_expired() {
            Ok(None)
        } else {
            Ok(Some(claim))
        }
    }

    /// Raw get without TTL filtering. Exposed for diagnostics (`--include-expired`).
    pub async fn get_including_expired(&self, id: &str) -> Result<Option<Claim>, ClaimError> {
        validate_id(id)?;
        let path = self.path_for(id);
        match read_bounded(&path).await? {
            Some(raw) => {
                let claim: Claim = serde_yaml::from_str(&raw)?;
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

        // Persist the trimmed form — validate_agent_id accepted ` foo ` as
        // equivalent to `foo`, so the on-disk value must reflect that
        // canonicalisation. Otherwise `is_held_by(agent)` round-trips break
        // (caller passes `foo`, file says ` foo `).
        let agent = agent.trim();

        self.ensure_dir().await?;

        // Check for live claim by a different agent.
        if let Some(existing) = self.get(id).await?
            && !existing.is_held_by(agent)
        {
            return Err(ClaimError::AlreadyHeld {
                id: existing.id,
                agent_id: existing.agent_id,
                expires_at: existing.expires_at,
            });
        }

        let now = Utc::now();
        let claim = Claim {
            id: id.to_uppercase(),
            agent_id: agent.to_string(),
            claimed_at: now,
            expires_at: now + ttl,
            note,
        };

        let yaml = serde_yaml::to_string(&claim)?;
        atomic_write(&self.path_for(id), yaml.as_bytes()).await?;
        Ok(claim)
    }

    /// Remove a claim.
    ///
    /// - `force = false`: refuse if held by a different agent (protects
    ///   agents from clobbering each other's work markers).
    /// - `force = true`: remove regardless — orchestrator escape hatch for
    ///   a confirmed-dead agent (R-4 mitigation).
    ///
    /// Missing claim is a no-op (not an error) to make release idempotent.
    pub async fn release(&self, id: &str, agent: &str, force: bool) -> Result<(), ClaimError> {
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

        let path = self.path_for(id);
        if !force
            && let Some(existing) = self.get_including_expired(id).await?
            && !existing.is_held_by(agent)
            && !existing.is_expired()
        {
            return Err(ClaimError::NotHeldByRequester {
                id: existing.id,
                held_by: existing.agent_id,
                requester: agent.to_string(),
            });
        }

        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(ClaimError::Io(e)),
        }
    }

    /// List every live claim in the workspace, sorted by `expires_at`
    /// ascending (earliest-expiring first). Expired claims are skipped
    /// (not returned, not removed — purging is a separate concern).
    pub async fn list_active(&self) -> Result<Vec<Claim>, ClaimError> {
        Ok(self.list_active_with_stats().await?.0)
    }

    /// Map view of `list_active` — keyed by claim ID uppercased. O(1)
    /// lookups when joining against a large artifact graph (Inc 4
    /// dispatcher needs `claims.get(id)` per artifact; iterating a Vec
    /// would be `O(artifacts × claims)`).
    pub async fn list_active_map(&self) -> Result<BTreeMap<String, Claim>, ClaimError> {
        let mut out = BTreeMap::new();
        for c in self.list_active().await? {
            out.insert(c.id.clone(), c);
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
        out.sort_by_key(|c| c.expires_at);
        Ok((out, skipped))
    }
}

/// Read a file with the MAX_CLAIM_FILE_BYTES cap enforced. Returns
/// `Ok(None)` for missing file; otherwise errors on size-violation or I/O
/// failure. Centralized so every read path (`get`, `list_active`) applies
/// the same protection.
async fn read_bounded(path: &Path) -> Result<Option<String>, ClaimError> {
    let meta = match tokio::fs::metadata(path).await {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ClaimError::Io(e)),
    };
    if meta.len() > MAX_CLAIM_FILE_BYTES {
        return Err(ClaimError::FileTooLarge);
    }
    match tokio::fs::read_to_string(path).await {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(ClaimError::Io(e)),
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
}
