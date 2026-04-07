//! Discovery engine — protocol + session tracking for brownfield project analysis.
//!
//! Per PROB-022: ForgePlan provides structured protocol; AI agent parses code.
//!
//! # Overview
//!
//! - `protocol` — Phase enum + Protocol struct served to agents
//! - `session` — DiscoverSession state + file storage in .forgeplan/discovery/
//!
//! # Usage (from MCP tools / CLI in W2)
//!
//! ```ignore
//! use forgeplan_core::discover::{Protocol, DiscoverSession, session};
//!
//! let session = DiscoverSession::new("my-project");
//! let protocol = Protocol::default();
//! session::save_session(workspace, &session)?;
//! ```

pub mod protocol;
pub mod session;

pub use protocol::{Phase, PhaseInstruction, Protocol, SourceTierRules};
pub use session::{
    DiscoverSession, Finding, SessionId, SessionStatus, list_sessions, load_session, save_session,
    session_dir, session_file,
};
