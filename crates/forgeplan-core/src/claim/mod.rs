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

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
    #[error("ttl {0:?} outside permitted range ({MIN_TTL:?}..={MAX_TTL:?})")]
    TtlOutOfRange(Duration),
    #[error("artifact id must be non-empty")]
    EmptyId,
    #[error("agent id must be non-empty")]
    EmptyAgent,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// On-disk claim store rooted at `<workspace>/claims/`.
#[derive(Debug, Clone)]
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
        if id.is_empty() {
            return Err(ClaimError::EmptyId);
        }
        let path = self.path_for(id);
        let raw = match tokio::fs::read_to_string(&path).await {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(ClaimError::Io(e)),
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
        if id.is_empty() {
            return Err(ClaimError::EmptyId);
        }
        let path = self.path_for(id);
        match tokio::fs::read_to_string(&path).await {
            Ok(raw) => {
                let claim: Claim = serde_yaml::from_str(&raw)?;
                Ok(Some(claim))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ClaimError::Io(e)),
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
        if id.is_empty() {
            return Err(ClaimError::EmptyId);
        }
        if agent.is_empty() {
            return Err(ClaimError::EmptyAgent);
        }
        if ttl < MIN_TTL || ttl > MAX_TTL {
            return Err(ClaimError::TtlOutOfRange(ttl));
        }

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
        tokio::fs::write(self.path_for(id), yaml).await?;
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
        if id.is_empty() {
            return Err(ClaimError::EmptyId);
        }
        if !force && agent.is_empty() {
            return Err(ClaimError::EmptyAgent);
        }

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
        if !self.dir.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        let mut rd = tokio::fs::read_dir(&self.dir).await?;
        while let Some(entry) = rd.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
                continue;
            }
            let raw = match tokio::fs::read_to_string(&path).await {
                Ok(s) => s,
                Err(_) => continue, // skip unreadable files rather than fail the whole list
            };
            if let Ok(claim) = serde_yaml::from_str::<Claim>(&raw)
                && !claim.is_expired()
            {
                out.push(claim);
            }
        }
        out.sort_by_key(|c| c.expires_at);
        Ok(out)
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
}
