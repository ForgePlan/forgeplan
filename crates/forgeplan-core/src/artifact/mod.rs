pub mod canonical_key;
pub mod delta;
pub mod frontmatter;
pub mod id_alloc;
pub mod identity;
pub mod sanitize;
pub mod sections;
pub mod store;
pub mod types;
pub mod validation;

// Wave 9 SEC-C1+C2: centralised title validator — re-export at module level
// so call sites in `forgeplan-cli` and `forgeplan-mcp` can write
// `forgeplan_core::artifact::validate_title(...)` without reaching into the
// submodule path.
pub use validation::{MAX_TITLE_LEN, validate_title};

// ADR-030 — canonical artifact identity. The canonical key is a document's
// permanent identity; the display id (`PRD-074`) is an alias that may repeat.
// Re-exported at module level so a caller can write
// `forgeplan_core::artifact::mint_artifact_key(...)` the same way it writes
// `forgeplan_core::artifact::validate_title(...)`.
pub use canonical_key::{
    ArtifactCandidate, ArtifactIdentity, ResolveError, ResolvedArtifact, mint_artifact_key,
};
