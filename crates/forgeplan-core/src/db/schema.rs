use std::sync::Arc;

use arrow_schema::{DataType, Field, Schema};

/// Embedding vector dimension — BGE-M3 full-size (ADR-005).
/// Referenced from schema, store, and convert modules.
pub const EMBEDDING_DIM: i32 = 1024;

/// Arrow schema for the `artifacts` table.
///
/// Columns:
/// - id          Utf8 (not null) — PK: "PRD-001", "RFC-002"
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
    ]))
}

/// Arrow schema for the `evidence` table.
///
/// Columns:
/// - id               Utf8 (not null) — PK: "EVID-001"
/// - artifact_id      Utf8 (not null) — FK: linked artifact
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
/// Columns:
/// - source_id     Utf8 (not null) — FK: source artifact
/// - target_id     Utf8 (not null) — FK: target artifact
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

/// Arrow schema for the `fpf_spec` table — FPF knowledge base chunks.
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
    ]))
}

/// Arrow schema for the `change_log` table — audit trail of artifact changes.
///
/// Columns:
/// - timestamp     Utf8 (not null) — ISO datetime (RFC 3339)
/// - artifact_id   Utf8 (not null) — which artifact changed
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
        assert_eq!(nullable, vec!["parent_section"]);
    }
}
