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
