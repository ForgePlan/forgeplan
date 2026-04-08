//! Discovery session — tracks state of a discover run.
//!
//! Session files live in .forgeplan/discovery/<session-id>.json.
//! Per ADR-003, these are markdown-adjacent JSON state files.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::discover::protocol::Phase;

/// Unique ID for a discovery session (e.g., "disc-20260407-abc123")
pub type SessionId = String;

/// State of a discovery session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Just created, no findings yet
    Started,
    /// Findings being reported (at least one finding received)
    Active,
    /// Agent called discover_complete
    Completed,
    /// Session abandoned (timeout or manual close)
    Abandoned,
}

/// A single finding reported by an agent during discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub phase: Phase,
    pub tier: u8,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub source_files: Vec<String>,
    pub artifact_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Discovery session metadata + findings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverSession {
    pub id: SessionId,
    pub project_name: String,
    pub status: SessionStatus,
    pub current_phase: Phase,
    pub findings: Vec<Finding>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl DiscoverSession {
    /// Create new session with generated ID.
    pub fn new(project_name: impl Into<String>) -> Self {
        let now = Utc::now();
        let id = format!("disc-{}", now.format("%Y%m%d-%H%M%S"));
        Self {
            id,
            project_name: project_name.into(),
            status: SessionStatus::Started,
            current_phase: Phase::Detect,
            findings: Vec::new(),
            started_at: now,
            completed_at: None,
        }
    }

    /// Add a finding and mark session as Active.
    pub fn add_finding(&mut self, finding: Finding) {
        self.findings.push(finding);
        if self.status == SessionStatus::Started {
            self.status = SessionStatus::Active;
        }
    }

    /// Mark session as completed.
    pub fn complete(&mut self) {
        self.status = SessionStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Count findings per phase.
    pub fn phase_counts(&self) -> std::collections::HashMap<Phase, usize> {
        let mut counts = std::collections::HashMap::new();
        for f in &self.findings {
            *counts.entry(f.phase).or_insert(0) += 1;
        }
        counts
    }

    /// Count findings per tier.
    pub fn tier_counts(&self) -> std::collections::HashMap<u8, usize> {
        let mut counts = std::collections::HashMap::new();
        for f in &self.findings {
            *counts.entry(f.tier).or_insert(0) += 1;
        }
        counts
    }
}

/// Directory inside workspace where session files live.
pub fn session_dir(workspace: &Path) -> PathBuf {
    workspace.join("discovery")
}

/// Path to a specific session file.
pub fn session_file(workspace: &Path, id: &str) -> PathBuf {
    session_dir(workspace).join(format!("{}.json", id))
}

/// Save session to .forgeplan/discovery/<id>.json
pub fn save_session(workspace: &Path, session: &DiscoverSession) -> anyhow::Result<()> {
    let dir = session_dir(workspace);
    std::fs::create_dir_all(&dir)?;
    let path = session_file(workspace, &session.id);
    let json = serde_json::to_string_pretty(session)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// Load session from file.
pub fn load_session(workspace: &Path, id: &str) -> anyhow::Result<DiscoverSession> {
    let path = session_file(workspace, id);
    let json = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Session not found: {} ({})", id, e))?;
    let session: DiscoverSession = serde_json::from_str(&json)?;
    Ok(session)
}

/// List all sessions in workspace.
pub fn list_sessions(workspace: &Path) -> anyhow::Result<Vec<DiscoverSession>> {
    let dir = session_dir(workspace);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut sessions = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.path().extension().and_then(|s| s.to_str()) == Some("json")
            && let Ok(json) = std::fs::read_to_string(entry.path())
            && let Ok(s) = serde_json::from_str::<DiscoverSession>(&json)
        {
            sessions.push(s);
        }
    }
    sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(sessions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_finding(phase: Phase, tier: u8) -> Finding {
        Finding {
            phase,
            tier,
            kind: "note".to_string(),
            title: "test".to_string(),
            body: "body".to_string(),
            source_files: vec!["src/main.rs".to_string()],
            artifact_id: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn new_session_has_defaults() {
        let s = DiscoverSession::new("proj");
        assert_eq!(s.project_name, "proj");
        assert_eq!(s.status, SessionStatus::Started);
        assert_eq!(s.current_phase, Phase::Detect);
        assert!(s.findings.is_empty());
        assert!(s.completed_at.is_none());
        assert!(s.id.starts_with("disc-"));
    }

    #[test]
    fn add_finding_bumps_status_to_active() {
        let mut s = DiscoverSession::new("p");
        s.add_finding(sample_finding(Phase::Detect, 1));
        assert_eq!(s.status, SessionStatus::Active);
        assert_eq!(s.findings.len(), 1);
    }

    #[test]
    fn complete_marks_completed() {
        let mut s = DiscoverSession::new("p");
        s.complete();
        assert_eq!(s.status, SessionStatus::Completed);
        assert!(s.completed_at.is_some());
    }

    #[test]
    fn phase_counts_aggregates() {
        let mut s = DiscoverSession::new("p");
        s.add_finding(sample_finding(Phase::Detect, 1));
        s.add_finding(sample_finding(Phase::Detect, 1));
        s.add_finding(sample_finding(Phase::Code, 1));
        let counts = s.phase_counts();
        assert_eq!(counts.get(&Phase::Detect), Some(&2));
        assert_eq!(counts.get(&Phase::Code), Some(&1));
    }

    #[test]
    fn tier_counts_aggregates() {
        let mut s = DiscoverSession::new("p");
        s.add_finding(sample_finding(Phase::Detect, 1));
        s.add_finding(sample_finding(Phase::Tests, 2));
        s.add_finding(sample_finding(Phase::Docs, 3));
        s.add_finding(sample_finding(Phase::Docs, 3));
        let counts = s.tier_counts();
        assert_eq!(counts.get(&1), Some(&1));
        assert_eq!(counts.get(&2), Some(&1));
        assert_eq!(counts.get(&3), Some(&2));
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let mut s = DiscoverSession::new("proj");
        s.add_finding(sample_finding(Phase::Code, 1));
        save_session(tmp.path(), &s).unwrap();

        let loaded = load_session(tmp.path(), &s.id).unwrap();
        assert_eq!(loaded.id, s.id);
        assert_eq!(loaded.findings.len(), 1);
        assert_eq!(loaded.status, SessionStatus::Active);
    }

    #[test]
    fn load_missing_session_errors() {
        let tmp = TempDir::new().unwrap();
        assert!(load_session(tmp.path(), "disc-nope").is_err());
    }

    #[test]
    fn list_sessions_empty_when_no_dir() {
        let tmp = TempDir::new().unwrap();
        let sessions = list_sessions(tmp.path()).unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn list_sessions_returns_saved() {
        let tmp = TempDir::new().unwrap();
        let s1 = DiscoverSession::new("a");
        save_session(tmp.path(), &s1).unwrap();
        // ensure second session has distinct id
        let mut s2 = DiscoverSession::new("b");
        s2.id = format!("{}-x", s2.id);
        save_session(tmp.path(), &s2).unwrap();

        let sessions = list_sessions(tmp.path()).unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn session_dir_path() {
        let p = Path::new("/tmp/ws");
        assert_eq!(session_dir(p), Path::new("/tmp/ws/discovery"));
        assert_eq!(
            session_file(p, "disc-1"),
            Path::new("/tmp/ws/discovery/disc-1.json")
        );
    }
}
