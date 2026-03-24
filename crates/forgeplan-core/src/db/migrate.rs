//! LanceDB schema migration — add missing columns without data loss.
//!
//! Each migration is idempotent: checks if column exists before adding.
//! Version tracked in config.yaml as `schema_version`.

use lancedb::table::NewColumnTransform;
use lancedb::Table;

/// Current schema version. Increment when adding migrations.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

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
                    "CAST(NULL AS UTF8)".to_string(),
                )]),
                None,
            )
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
                    "CAST(3 AS INT32)".to_string(),
                )]),
                None,
            )
            .await?;
    }

    Ok(())
}

/// Run all migrations on all tables. Call from LanceStore::open().
pub async fn run_migrations(artifacts: &Table, relations: &Table) -> anyhow::Result<()> {
    migrate_artifacts(artifacts).await?;
    migrate_relations(relations).await?;
    Ok(())
}
