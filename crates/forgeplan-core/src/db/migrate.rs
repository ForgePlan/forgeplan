//! LanceDB schema migration — add missing columns without data loss.
//!
//! Each migration is idempotent: checks if column exists before adding.
//! Version tracked in config.yaml as `schema_version`.

use lancedb::Table;
use lancedb::table::NewColumnTransform;

/// Current schema version. Increment when adding migrations.
///
/// v4: add `tags` List(Utf8) column to artifacts (PRD-035 FR-001).
pub const CURRENT_SCHEMA_VERSION: u32 = 4;

/// Run all pending migrations on the artifacts table.
/// Idempotent — safe to run multiple times.
pub async fn migrate_artifacts(table: &Table) -> anyhow::Result<()> {
    let schema = table.schema().await?;
    let field_names: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();

    // Migration 1→2: add body_hash column (nullable Utf8)
    if !field_names.contains(&"body_hash".to_string()) {
        eprintln!("[migrate] Adding body_hash column to artifacts");
        table
            .add_columns(
                NewColumnTransform::SqlExpressions(vec![(
                    "body_hash".to_string(),
                    "CAST(NULL AS STRING)".to_string(),
                )]),
                None,
            )
            .await?;
    }

    // Migration 3→4: add tags List(Utf8) column (PRD-035 FR-001).
    // Existing rows get NULL (interpreted as empty tag list on read).
    if !field_names.contains(&"tags".to_string()) {
        eprintln!("[migrate] Adding tags column to artifacts");
        // The lance-datafusion SQL parser does not understand `LIST<...>`
        // syntax, so `CAST(NULL AS LIST<STRING>)` fails at runtime on real
        // v3 tables (release blocker). Use `AllNulls` instead — it accepts an
        // Arrow schema directly and fills existing rows with typed nulls.
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;
        let tags_schema = Arc::new(Schema::new(vec![Field::new(
            "tags",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            true,
        )]));
        table
            .add_columns(NewColumnTransform::AllNulls(tags_schema), None)
            .await?;
    }

    Ok(())
}

/// Run all pending migrations on the relations table.
/// Idempotent — safe to run multiple times.
pub async fn migrate_relations(table: &Table) -> anyhow::Result<()> {
    let schema = table.schema().await?;
    let field_names: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();

    // Migration 1→2: add congruence_level column (nullable Int32, default 3)
    if !field_names.contains(&"congruence_level".to_string()) {
        eprintln!("[migrate] Adding congruence_level column to relations");
        table
            .add_columns(
                NewColumnTransform::SqlExpressions(vec![(
                    "congruence_level".to_string(),
                    "CAST(3 AS INT)".to_string(),
                )]),
                None,
            )
            .await?;
    }

    Ok(())
}

/// Ensure the change_log table exists. Called from LanceStore::open().
/// Unlike other migrations, this creates the table if missing (since older workspaces
/// don't have it). The table handle is passed as Option — None means "not found".
pub async fn ensure_change_log(db: &lancedb::connection::Connection) -> anyhow::Result<()> {
    let tables = db.table_names().execute().await?;
    if !tables.contains(&"change_log".to_string()) {
        use arrow_array::{RecordBatch, StringArray};
        use arrow_schema::ArrowError;
        use std::sync::Arc;

        let schema = super::schema::change_log_schema();
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(Vec::<&str>::new())),
                Arc::new(StringArray::from(Vec::<&str>::new())),
                Arc::new(StringArray::from(Vec::<&str>::new())),
                Arc::new(StringArray::from(Vec::<Option<&str>>::new())),
                Arc::new(StringArray::from(Vec::<Option<&str>>::new())),
                Arc::new(StringArray::from(Vec::<Option<&str>>::new())),
                Arc::new(StringArray::from(Vec::<&str>::new())),
                Arc::new(StringArray::from(Vec::<Option<&str>>::new())), // commit_hash
            ],
        )
        .map_err(|e: ArrowError| anyhow::anyhow!("Failed to create change_log batch: {}", e))?;
        db.create_table("change_log", vec![batch]).execute().await?;
        eprintln!("[migrate] Created change_log table");
    }
    Ok(())
}

/// Migrate existing change_log table to add commit_hash column (v2→v3).
pub async fn migrate_change_log(table: &Table) -> anyhow::Result<()> {
    let schema = table.schema().await?;
    let field_names: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();

    if !field_names.contains(&"commit_hash".to_string()) {
        eprintln!("[migrate] Adding commit_hash column to change_log");
        table
            .add_columns(
                NewColumnTransform::SqlExpressions(vec![(
                    "commit_hash".to_string(),
                    "CAST(NULL AS STRING)".to_string(),
                )]),
                None,
            )
            .await?;
    }

    Ok(())
}

/// Run idempotent migrations on the `fpf_spec` table — currently adds the
/// `embedding` FixedSizeList<Float32, 1024> column for semantic search
/// (PRD-042). Pre-existing rows are filled with NULL via
/// `NewColumnTransform::AllNulls`.
pub async fn migrate_fpf_spec(table: &Table) -> anyhow::Result<()> {
    let schema = table.schema().await?;
    let field_names: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();

    if !field_names.contains(&"embedding".to_string()) {
        eprintln!("[migrate] Adding embedding column to fpf_spec");
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;
        let embedding_schema = Arc::new(Schema::new(vec![Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                super::schema::EMBEDDING_DIM,
            ),
            true,
        )]));
        table
            .add_columns(NewColumnTransform::AllNulls(embedding_schema), None)
            .await?;
    }

    Ok(())
}

/// Run all migrations on all tables. Call from LanceStore::open().
pub async fn run_migrations(
    artifacts: &Table,
    relations: &Table,
    change_log: Option<&Table>,
    fpf_spec: Option<&Table>,
) -> anyhow::Result<()> {
    migrate_artifacts(artifacts).await?;
    migrate_relations(relations).await?;
    if let Some(cl) = change_log {
        migrate_change_log(cl).await?;
    }
    if let Some(fs) = fpf_spec {
        migrate_fpf_spec(fs).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::convert::{extract_tags, make_null_embedding_col};
    use crate::db::schema::EMBEDDING_DIM;
    use arrow_array::{
        Array, Float64Array, LargeStringArray, ListArray, RecordBatch, StringArray,
        builder::{ListBuilder, StringBuilder},
    };
    use arrow_schema::{DataType, Field, Schema};
    use futures::TryStreamExt;
    use lancedb::query::ExecutableQuery;
    use std::sync::Arc;
    use tempfile::TempDir;

    /// Schema used by v3 workspaces (no `tags` column).
    fn v3_artifacts_schema() -> Arc<Schema> {
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
        ]))
    }

    /// Build a one-row v3 batch (no tags column) for a legacy artifact.
    fn v3_row(id: &str, kind: &str, status: &str, title: &str, body: &str) -> RecordBatch {
        let schema = v3_artifacts_schema();
        let now = "2026-01-01T00:00:00Z";
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec![id])),
                Arc::new(StringArray::from(vec![kind])),
                Arc::new(StringArray::from(vec![status])),
                Arc::new(StringArray::from(vec![title])),
                Arc::new(LargeStringArray::from(vec![body])),
                Arc::new(StringArray::from(vec!["standard"])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                Arc::new(Float64Array::from(vec![0.0f64])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                Arc::new(StringArray::from(vec![now])),
                Arc::new(StringArray::from(vec![now])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                make_null_embedding_col(1),
            ],
        )
        .unwrap()
    }

    /// Create a fresh LanceDB at `dir` with an `artifacts` table matching v3 schema
    /// (no tags column) and return the opened table handle.
    async fn create_v3_table(dir: &std::path::Path) -> Table {
        let conn = lancedb::connect(dir.to_str().unwrap())
            .execute()
            .await
            .unwrap();
        let schema = v3_artifacts_schema();
        let empty = RecordBatch::new_empty(schema);
        conn.create_table("artifacts", vec![empty])
            .execute()
            .await
            .unwrap()
    }

    async fn collect_rows(table: &Table) -> Vec<RecordBatch> {
        table
            .query()
            .execute()
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap()
    }

    /// H4 regression (Sprint 13.3 W5): proves the migration now succeeds on a
    /// real v3 workspace. Previously `CAST(NULL AS LIST<STRING>)` was rejected
    /// by the lance-datafusion SQL parser; the fix switches to
    /// `NewColumnTransform::AllNulls` with a typed Arrow schema.
    #[tokio::test]
    async fn migrate_v3_to_v4_succeeds_on_real_v3_workspace() {
        let tmp = TempDir::new().unwrap();
        let table = create_v3_table(tmp.path()).await;
        let row = v3_row("PRD-001", "prd", "draft", "Legacy", "body");
        table.add(vec![row]).execute().await.unwrap();

        migrate_artifacts(&table)
            .await
            .expect("v3→v4 migration must succeed on a real v3 row");

        let schema = table.schema().await.unwrap();
        assert!(schema.fields().iter().any(|f| f.name() == "tags"));
    }

    #[tokio::test]
    async fn migrate_v3_to_v4_adds_tags_column() {
        let tmp = TempDir::new().unwrap();
        let table = create_v3_table(tmp.path()).await;

        // Precondition: no tags column yet
        let schema = table.schema().await.unwrap();
        assert!(!schema.fields().iter().any(|f| f.name() == "tags"));

        migrate_artifacts(&table).await.unwrap();

        let schema = table.schema().await.unwrap();
        assert!(
            schema.fields().iter().any(|f| f.name() == "tags"),
            "tags column should exist after migration"
        );
    }

    #[tokio::test]
    async fn migrate_v3_to_v4_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let table = create_v3_table(tmp.path()).await;

        migrate_artifacts(&table).await.unwrap();
        // Second run must not error — tags column already present.
        migrate_artifacts(&table).await.unwrap();
        migrate_artifacts(&table).await.unwrap();

        let schema = table.schema().await.unwrap();
        let tags_fields: Vec<_> = schema
            .fields()
            .iter()
            .filter(|f| f.name() == "tags")
            .collect();
        assert_eq!(tags_fields.len(), 1, "exactly one tags column");
    }

    #[tokio::test]
    async fn migrate_v3_to_v4_preserves_existing_rows() {
        let tmp = TempDir::new().unwrap();
        let table = create_v3_table(tmp.path()).await;

        // Insert v3-style row (no tags column)
        let _v3_schema = v3_artifacts_schema();
        let row = v3_row("PRD-001", "prd", "draft", "Legacy Test", "# legacy body");
        table.add(vec![row]).execute().await.unwrap();

        // Sanity: row is present pre-migration
        let pre = collect_rows(&table).await;
        let pre_rows: usize = pre.iter().map(|b| b.num_rows()).sum();
        assert_eq!(pre_rows, 1);

        // Run migration
        migrate_artifacts(&table).await.unwrap();

        // Verify row is still there post-migration
        let post = collect_rows(&table).await;
        let post_rows: usize = post.iter().map(|b| b.num_rows()).sum();
        assert_eq!(post_rows, 1, "existing row must survive migration");

        // Verify tags column exists and reads as empty (null → Vec::new via extract_tags)
        let batch = &post[0];
        let tags_col = batch
            .column_by_name("tags")
            .expect("tags column must exist after migration");
        // Column should be a ListArray; the one existing row should have null tags
        // (because the CAST(NULL AS LIST<STRING>) fills old rows with null).
        let list = tags_col.as_any().downcast_ref::<ListArray>().unwrap();
        assert!(
            list.is_null(0) || extract_tags(batch, 0).is_empty(),
            "pre-existing row must have null or empty tags"
        );

        // extract_tags must return an empty Vec for null rows (not panic)
        let tags = extract_tags(batch, 0);
        assert_eq!(tags, Vec::<String>::new());

        // Verify id/title preserved
        let id_col = batch
            .column_by_name("id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .unwrap();
        assert_eq!(id_col.value(0), "PRD-001");
        let title_col = batch
            .column_by_name("title")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .unwrap();
        assert_eq!(title_col.value(0), "Legacy Test");
    }

    #[tokio::test]
    async fn migrate_v3_to_v4_allows_new_rows_with_tags() {
        let tmp = TempDir::new().unwrap();
        let table = create_v3_table(tmp.path()).await;

        // Insert a v3 row first
        let _v3_schema = v3_artifacts_schema();
        let legacy = v3_row("PRD-001", "prd", "draft", "Legacy", "old body");
        table.add(vec![legacy]).execute().await.unwrap();

        // Migrate
        migrate_artifacts(&table).await.unwrap();

        // Now insert a v4 row WITH tags using the current (post-migration) schema
        let v4_schema = table.schema().await.unwrap();
        let mut tag_builder = ListBuilder::new(StringBuilder::new());
        tag_builder.values().append_value("source=code");
        tag_builder.values().append_value("layer=domain");
        tag_builder.append(true);
        let tags_col: ListArray = tag_builder.finish();

        let now = "2026-01-02T00:00:00Z";
        let v4_row = RecordBatch::try_new(
            v4_schema.clone(),
            vec![
                Arc::new(StringArray::from(vec!["PRD-002"])),
                Arc::new(StringArray::from(vec!["prd"])),
                Arc::new(StringArray::from(vec!["draft"])),
                Arc::new(StringArray::from(vec!["New With Tags"])),
                Arc::new(LargeStringArray::from(vec!["# new body"])),
                Arc::new(StringArray::from(vec!["standard"])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                Arc::new(Float64Array::from(vec![0.0f64])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                Arc::new(StringArray::from(vec![now])),
                Arc::new(StringArray::from(vec![now])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                make_null_embedding_col(1),
                Arc::new(tags_col),
            ],
        )
        .unwrap();
        table.add(vec![v4_row]).execute().await.unwrap();

        // Read back: both rows present, new row has tags populated
        let batches = collect_rows(&table).await;
        let total: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total, 2);

        // Find PRD-002 row across batches and verify its tags
        let mut found_tags = None;
        for batch in &batches {
            let ids = batch
                .column_by_name("id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .unwrap();
            for row in 0..batch.num_rows() {
                if ids.value(row) == "PRD-002" {
                    found_tags = Some(extract_tags(batch, row));
                }
            }
        }
        let tags = found_tags.expect("PRD-002 must be present");
        assert_eq!(tags, vec!["source=code", "layer=domain"]);
    }

    /// Pre-PRD-042 fpf_spec schema (8 columns, no embedding).
    fn legacy_fpf_spec_schema() -> Arc<Schema> {
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

    async fn create_legacy_fpf_spec_table(dir: &std::path::Path) -> Table {
        let conn = lancedb::connect(dir.to_str().unwrap())
            .execute()
            .await
            .unwrap();
        let schema = legacy_fpf_spec_schema();
        let empty = RecordBatch::new_empty(schema);
        conn.create_table("fpf_spec", vec![empty])
            .execute()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn migrate_fpf_spec_adds_embedding_column() {
        let tmp = TempDir::new().unwrap();
        let table = create_legacy_fpf_spec_table(tmp.path()).await;

        let schema = table.schema().await.unwrap();
        assert!(!schema.fields().iter().any(|f| f.name() == "embedding"));

        migrate_fpf_spec(&table).await.unwrap();

        let schema = table.schema().await.unwrap();
        let emb = schema
            .fields()
            .iter()
            .find(|f| f.name() == "embedding")
            .expect("embedding column should exist after migration");
        assert!(emb.is_nullable());
        match emb.data_type() {
            DataType::FixedSizeList(inner, size) => {
                assert_eq!(*size, EMBEDDING_DIM);
                assert_eq!(*inner.data_type(), DataType::Float32);
            }
            _ => panic!("embedding should be FixedSizeList<Float32, 1024>"),
        }
    }

    #[tokio::test]
    async fn migrate_fpf_spec_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let table = create_legacy_fpf_spec_table(tmp.path()).await;

        migrate_fpf_spec(&table).await.unwrap();
        migrate_fpf_spec(&table).await.unwrap();
        migrate_fpf_spec(&table).await.unwrap();

        let schema = table.schema().await.unwrap();
        let emb_count = schema
            .fields()
            .iter()
            .filter(|f| f.name() == "embedding")
            .count();
        assert_eq!(emb_count, 1, "exactly one embedding column");
    }

    #[tokio::test]
    async fn migrate_v3_to_v4_null_tags_read_as_empty_vec() {
        // Multiple legacy rows → migrate → extract_tags must never panic and
        // must return empty Vec<String> for every pre-existing row.
        let tmp = TempDir::new().unwrap();
        let table = create_v3_table(tmp.path()).await;

        let _v3_schema = v3_artifacts_schema();
        for i in 1..=3 {
            let row = v3_row(
                &format!("PRD-{:03}", i),
                "prd",
                "draft",
                &format!("Legacy {}", i),
                "body",
            );
            table.add(vec![row]).execute().await.unwrap();
        }

        migrate_artifacts(&table).await.unwrap();

        let batches = collect_rows(&table).await;
        let mut total = 0;
        for batch in &batches {
            for row in 0..batch.num_rows() {
                let tags = extract_tags(batch, row);
                assert_eq!(
                    tags,
                    Vec::<String>::new(),
                    "legacy row {} must read as empty tags",
                    row
                );
                total += 1;
            }
        }
        assert_eq!(total, 3);
    }
}
