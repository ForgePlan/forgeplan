//! Ingest engine — declarative mapping of plugin output to forge artifacts.
//!
//! See [`SPEC-004`](../../../.forgeplan/specs/SPEC-004-mapping-yaml-schema.md)
//! for the mapping YAML schema and [`PRD-066`](../../../.forgeplan/prds/PRD-066-ingest-engine-mapping-yaml-format-c4-to-forge-autoresearch-to-forge-git-to-forge-ddd-to-forge-spec-to-forge.md)
//! for goals and acceptance criteria.

pub mod engine;
pub mod idempotency;
pub mod sources;
pub mod template;
pub mod types;

pub use types::{
    ALLOWED_FILTERS, ArtifactTargetKind, CompatSpecVersion, ErrorAction, ErrorPolicy, Guards,
    IfExists, LinkSpec, Mapping, Parser, Rule, Selector, SourceKind, SourcePrecision, SourceSpec,
    SourcesSectionSpec, TargetKind, TargetSpec, Template,
};

pub use engine::{
    DraftLink, IngestArtifactDraft, IngestEngine, IngestError, IngestOptions, IngestReport,
    RuleError, SkipReason,
};
pub use idempotency::{
    UpdateDecision, artifact_needs_update, compute_source_hash, extract_existing_source_hash,
    render_source_hash_marker,
};
pub use sources::{
    FrontMatterPlusSections, JsonParser, LogWithBlame, MarkdownOnly, ParseError, ParsedSection,
    ParsedSource, SourceParser, YamlParser, parser_for,
};
pub use template::{TemplateEngine, TemplateError};
