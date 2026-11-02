use std::sync::Arc;

use arrow_schema::{DataType, Field, Schema};

/// Embedding vector dimension — BGE-M3 full-size (ADR-005).
/// Referenced from schema, store, and convert modules.
pub const EMBEDDING_DIM: i32 = 1024;

/// Arrow schema for the `artifacts` table.
///
/// Schema version 5 (ADR-030): the row is keyed by its **canonical key**.
/// The `id` column stays the human-facing display alias; `canonical_key` is
/// the authoritative identity every filter, upsert and mutation uses.
///
/// Columns:
/// - id          Utf8 (not null) — display alias: "PRD-001", "RFC-002"
/// - kind        Utf8 (not null) — "prd", "rfc", "adr", "epic", etc.
/// - status      Utf8 (not null) — "draft", "active", "superseded"
/// - title       Utf8 (not null) — human-readable title
/// - body        LargeUtf8 (not null) — markdown content (after ---)
/// - depth       Utf8 (not null) — "tactical", "standard", "deep"
/// - author      Utf8 (nullable) — author name
/// - parent_epic Utf8 (nullable) — FK: parent epic ID
/// - r_eff_score Float64 (not null) — cached R_eff score
/// - valid_until Utf8 (nullable) — ISO date for evidence decay
/// - created_at  Utf8 (not null) — ISO datetime
/// - updated_at  Utf8 (not null) — ISO datetime
/// - body_hash   Utf8 (nullable) — hash of body for change detection
/// - embedding   FixedSizeList(1024, Float32) (nullable) — vector for semantic search
/// - tags        List(Utf8) (nullable) — string tags like "source=code", "layer=domain"
/// - canonical_key Utf8 (nullable) — ADR-030 authoritative identity: the frontmatter
///   slug when the document declares a valid one, otherwise the display id
/// - semantic_input_digest Utf8 (nullable) — digest of the semantic input
/// - semantic_profile_digest Utf8 (nullable) — digest of the embedding profile
/// - semantic_vector_digest Utf8 (nullable) — digest of the stored vector
/// - semantic_job_id Utf8 (nullable) — semantic job identifier
/// - semantic_attempt_token Utf8 (nullable) — semantic attempt identifier
/// - semantic_state Utf8 (nullable) — semantic binding state
///
/// `canonical_key` follows the 15 schema-4 columns. The semantic columns are
/// appended after it so fresh and upgraded tables share the same order.
///
/// The column is nullable in the Arrow schema because that is the only shape
/// an in-place upgrade can produce — `add_columns` fills existing rows with
/// nulls. Every schema-5 writer fills it, and a row that still has no key is
/// a pre-migration row the staged identity-v5 migration replaces.
pub fn artifacts_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("kind", DataType::Utf8, false),
        Field::new("status", DataType::Utf8, false),
        Field::new("title", DataType::Utf8, false),
        Field::new("body", DataType::LargeUtf8, false),
        Field::new("depth", DataType::Utf8, false),
        Field::new("author", DataType::Utf8, true),
        Field::new("parent_epic", DataType::Utf8, true),
        Field::new("r_eff_score", DataType::Float64, false),
        Field::new("valid_until", DataType::Utf8, true),
        Field::new("created_at", DataType::Utf8, false),
        Field::new("updated_at", DataType::Utf8, false),
        Field::new("body_hash", DataType::Utf8, true),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIM,
            ),
            true,
        ),
        Field::new(
            "tags",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            true,
        ),
        Field::new("canonical_key", DataType::Utf8, true),
        Field::new("semantic_input_digest", DataType::Utf8, true),
        Field::new("semantic_profile_digest", DataType::Utf8, true),
        Field::new("semantic_vector_digest", DataType::Utf8, true),
        Field::new("semantic_job_id", DataType::Utf8, true),
        Field::new("semantic_attempt_token", DataType::Utf8, true),
        Field::new("semantic_state", DataType::Utf8, true),
    ]))
}

/// Arrow schema for the `evidence` table.
///
/// Schema version 5 (ADR-030): `artifact_id` is the **canonical key** of the
/// linked artifact, not its display alias.
///
/// Columns:
/// - id               Utf8 (not null) — PK: "EVID-001"
/// - artifact_id      Utf8 (not null) — canonical key of the linked artifact
/// - evidence_type    Utf8 (not null) — "measurement", "test", etc.
/// - verdict          Utf8 (not null) — "supports", "weakens", "refutes"
/// - congruence_level Int32 (not null) — 0-3
/// - valid_until      Utf8 (nullable) — ISO date
/// - content          Utf8 (not null) — evidence description
/// - created_at       Utf8 (not null) — ISO datetime
pub fn evidence_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("artifact_id", DataType::Utf8, false),
        Field::new("evidence_type", DataType::Utf8, false),
        Field::new("verdict", DataType::Utf8, false),
        Field::new("congruence_level", DataType::Int32, false),
        Field::new("valid_until", DataType::Utf8, true),
        Field::new("content", DataType::Utf8, false),
        Field::new("created_at", DataType::Utf8, false),
    ]))
}

/// Arrow schema for the `relations` table.
///
/// Schema version 5 (ADR-030): `source_id` and `target_id` keep their names
/// but hold **canonical keys**. A target that names a document outside this
/// workspace (a forward reference, a qualified `KEY@qualifier` cross-source
/// ref) keeps its declared bytes — it is an edge with a diagnostic, not a
/// silently dropped row.
///
/// Columns:
/// - source_id     Utf8 (not null) — canonical key of the source artifact
/// - target_id     Utf8 (not null) — canonical key of the target artifact
/// - relation_type Utf8 (not null) — "informs", "based_on", "supersedes"
/// - created_at    Utf8 (not null) — ISO datetime
pub fn relations_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("source_id", DataType::Utf8, false),
        Field::new("target_id", DataType::Utf8, false),
        Field::new("relation_type", DataType::Utf8, false),
        Field::new("created_at", DataType::Utf8, false),
        Field::new("congruence_level", DataType::Int32, true),
    ]))
}

/// Arrow schema for the FPF Knowledge Base `fpf_spec` table.
///
/// # Feature-flag contract
///
/// The `embedding` column is present **unconditionally**, regardless of whether
/// the `semantic-search` feature is enabled. This is intentional:
/// - Binary compatibility: workspaces built with `semantic-search` can be read
///   by binaries without the feature (and vice versa).
/// - Migration safety: no conditional schema branches means no "semantic-enabled
///   vs disabled" workspace drift.
///
/// The *encoding* of embeddings (turning text into `Vec<f32>`) is feature-gated
/// in `crates/forgeplan-core/src/embed/mod.rs` and consumers in
/// `crates/forgeplan-cli/src/commands/fpf.rs::run_ingest`. When the feature is
/// disabled, the column stays null and keyword search continues to work.
///
/// Columns:
/// - id            Utf8 (not null) — unique chunk ID: "fpf-B.3-001"
/// - section_id    Utf8 (not null) — FPF section ID: "B.3", "C.2.2", "A.1"
/// - parent_section Utf8 (nullable) — parent section: "07-part-b"
/// - title         Utf8 (not null) — section title
/// - body          LargeUtf8 (not null) — full markdown content
/// - line_count    Int32 (not null) — number of lines
/// - file_path     Utf8 (not null) — original file path
/// - created_at    Utf8 (not null) — ISO datetime of ingestion
/// - embedding     FixedSizeList(1024, Float32) (nullable) — vector for semantic
///   search (PRD-042). Pre-semantic workspaces have null.
///
/// The dimension 1024 is tied to the default BGE-M3 model in
/// `forgeplan_core::embed::Embedder`. Swapping to a different model with a
/// different output dim requires a schema migration. See `run_ingest` in
/// `forgeplan-cli/src/commands/fpf.rs` for the runtime assertion that
/// catches mismatches early (Sprint 13.7 hotfix FIX-E).
pub fn fpf_spec_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("section_id", DataType::Utf8, false),
        Field::new("parent_section", DataType::Utf8, true),
        Field::new("title", DataType::Utf8, false),
        Field::new("body", DataType::LargeUtf8, false),
        Field::new("line_count", DataType::Int32, false),
        Field::new("file_path", DataType::Utf8, false),
        Field::new("created_at", DataType::Utf8, false),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIM,
            ),
            true,
        ),
    ]))
}

/// Arrow schema for the `change_log` table — audit trail of artifact changes.
///
/// Schema version 5 (ADR-030): `artifact_id` holds the **canonical key** of
/// the changed artifact, so the history is queryable by the same identity the
/// index uses. Entries written before the upgrade keep their display id.
///
/// Columns:
/// - timestamp     Utf8 (not null) — ISO datetime (RFC 3339)
/// - artifact_id   Utf8 (not null) — canonical key of the changed artifact
/// - action        Utf8 (not null) — create/update/delete/link/unlink
/// - field         Utf8 (nullable) — which field changed (status, body, title)
/// - old_value     Utf8 (nullable) — previous value (hash for body)
/// - new_value     Utf8 (nullable) — new value (hash for body)
/// - source        Utf8 (not null) — cli/file_edit/git_sync/reindex
/// - commit_hash   Utf8 (nullable) — git commit hash (short, 7 chars)
pub fn change_log_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("timestamp", DataType::Utf8, false),
        Field::new("artifact_id", DataType::Utf8, false),
        Field::new("action", DataType::Utf8, false),
        Field::new("field", DataType::Utf8, true),
        Field::new("old_value", DataType::Utf8, true),
        Field::new("new_value", DataType::Utf8, true),
        Field::new("source", DataType::Utf8, false),
        Field::new("commit_hash", DataType::Utf8, true),
        // The document's canonical key — the identity the entry is about.
        // The display id in `artifact_id` is an alias and may name two
        // documents; the key names exactly one.
        Field::new("canonical_key", DataType::Utf8, true),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifacts_schema_has_required_columns() {
        let schema = artifacts_schema();
        let names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert!(names.contains(&"id"));
        assert!(names.contains(&"kind"));
        assert!(names.contains(&"status"));
        assert!(names.contains(&"title"));
        assert!(names.contains(&"body"));
        assert!(names.contains(&"r_eff_score"));
        assert!(names.contains(&"embedding"));
        assert!(names.contains(&"created_at"));
        assert!(names.contains(&"updated_at"));
    }

    #[test]
    fn artifacts_schema_nullable_fields() {
        let schema = artifacts_schema();
        let nullable: Vec<&str> = schema
            .fields()
            .iter()
            .filter(|f| f.is_nullable())
            .map(|f| f.name().as_str())
            .collect();
        assert!(nullable.contains(&"author"));
        assert!(nullable.contains(&"parent_epic"));
        assert!(nullable.contains(&"valid_until"));
        assert!(nullable.contains(&"body_hash"));
        assert!(nullable.contains(&"embedding"));
    }

    #[test]
    fn artifacts_schema_embedding_type() {
        let schema = artifacts_schema();
        let emb = schema.field_with_name("embedding").unwrap();
        assert!(emb.is_nullable());
        match emb.data_type() {
            DataType::FixedSizeList(inner, size) => {
                assert_eq!(*size, EMBEDDING_DIM);
                assert_eq!(*inner.data_type(), DataType::Float32);
            }
            _ => panic!("embedding should be FixedSizeList"),
        }
    }

    #[test]
    fn artifacts_schema_tags_is_nullable_list_of_utf8() {
        let schema = artifacts_schema();
        let tags = schema.field_with_name("tags").unwrap();
        assert!(tags.is_nullable());
        match tags.data_type() {
            DataType::List(inner) => assert_eq!(*inner.data_type(), DataType::Utf8),
            _ => panic!("tags should be List(Utf8)"),
        }
    }

    #[test]
    fn evidence_schema_has_required_columns() {
        let schema = evidence_schema();
        let names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert!(names.contains(&"id"));
        assert!(names.contains(&"artifact_id"));
        assert!(names.contains(&"verdict"));
        assert!(names.contains(&"congruence_level"));
    }

    #[test]
    fn relations_schema_has_required_columns() {
        let schema = relations_schema();
        let names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert!(names.contains(&"source_id"));
        assert!(names.contains(&"target_id"));
        assert!(names.contains(&"relation_type"));
        assert!(names.contains(&"created_at"));
    }

    #[test]
    fn fpf_spec_schema_has_required_columns() {
        let schema = fpf_spec_schema();
        let names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert!(names.contains(&"id"));
        assert!(names.contains(&"section_id"));
        assert!(names.contains(&"parent_section"));
        assert!(names.contains(&"title"));
        assert!(names.contains(&"body"));
        assert!(names.contains(&"line_count"));
        assert!(names.contains(&"file_path"));
        assert!(names.contains(&"created_at"));
        assert!(names.contains(&"embedding"));
    }

    #[test]
    fn fpf_spec_schema_nullable_fields() {
        let schema = fpf_spec_schema();
        let nullable: Vec<&str> = schema
            .fields()
            .iter()
            .filter(|f| f.is_nullable())
            .map(|f| f.name().as_str())
            .collect();
        assert_eq!(nullable, vec!["parent_section", "embedding"]);
    }

    #[test]
    fn fpf_spec_schema_embedding_type() {
        let schema = fpf_spec_schema();
        let emb = schema.field_with_name("embedding").unwrap();
        assert!(emb.is_nullable());
        match emb.data_type() {
            DataType::FixedSizeList(inner, size) => {
                assert_eq!(*size, EMBEDDING_DIM);
                assert_eq!(*inner.data_type(), DataType::Float32);
            }
            _ => panic!("embedding should be FixedSizeList"),
        }
    }

    /// Preserve the schema-4 columns and the canonical key's appended position.
    #[test]
    fn artifacts_schema_carries_canonical_key_and_semantic_columns_in_order() {
        let schema = artifacts_schema();
        let names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert_eq!(
            names,
            vec![
                "id", "kind", "status", "title", "body", "depth", "author",
                "parent_epic", "r_eff_score", "valid_until", "created_at", "updated_at",
                "body_hash", "embedding", "tags", "canonical_key",
                "semantic_input_digest", "semantic_profile_digest", "semantic_vector_digest",
                "semantic_job_id", "semantic_attempt_token", "semantic_state",
            ]
        );
        for name in &names[16..] {
            let field = schema.field_with_name(name).unwrap();
            assert_eq!(field.data_type(), &DataType::Utf8);
            assert!(field.is_nullable());
        }
        let key = schema.field_with_name("canonical_key").unwrap();
        assert_eq!(key.data_type(), &DataType::Utf8);
        // Nullable in the Arrow schema: `add_columns` is the only way an
        // in-place upgrade can add a column, and it fills old rows with
        // nulls. Every schema-5 writer fills the column itself.
        assert!(key.is_nullable());
    }

    /// The public id stays the display alias: the column keeps its name, its
    /// position and its non-null contract.
    #[test]
    fn artifacts_schema_keeps_the_display_id_column() {
        let schema = artifacts_schema();
        let names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert_eq!(names.first(), Some(&"id"));
        let id = schema.field_with_name("id").unwrap();
        assert!(!id.is_nullable());
        assert_eq!(id.data_type(), &DataType::Utf8);
    }

    /// The relation endpoints keep their column names — only the value they
    /// hold changes (canonical key instead of display id).
    #[test]
    fn relations_schema_keeps_its_endpoint_columns() {
        let schema = relations_schema();
        let names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert!(names.contains(&"source_id"));
        assert!(names.contains(&"target_id"));
        for endpoint in ["source_id", "target_id"] {
            let field = schema.field_with_name(endpoint).unwrap();
            assert!(!field.is_nullable());
            assert_eq!(field.data_type(), &DataType::Utf8);
        }
    }
}
