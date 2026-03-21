use std::sync::Arc;

use arrow_array::{Array, Float64Array, RecordBatch, StringArray};
use arrow_schema::Schema;

use crate::artifact::store::ArtifactSummary;
use crate::db::schema;
use crate::db::store::NewArtifact;

/// Convert a NewArtifact into an Arrow RecordBatch using the artifacts schema.
///
/// The embedding column is set to null (no vector computed at creation time).
pub fn artifact_to_batch(artifact: &NewArtifact, now: &str) -> anyhow::Result<RecordBatch> {
    let schema = schema::artifacts_schema();
    artifact_to_batch_with_schema(artifact, now, &schema)
}

pub(crate) fn artifact_to_batch_with_schema(
    artifact: &NewArtifact,
    now: &str,
    schema: &Arc<Schema>,
) -> anyhow::Result<RecordBatch> {
    let embedding_col = make_null_embedding_col(1);

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(vec![artifact.id.as_str()])),
            Arc::new(StringArray::from(vec![artifact.kind.as_str()])),
            Arc::new(StringArray::from(vec![artifact.status.as_str()])),
            Arc::new(StringArray::from(vec![artifact.title.as_str()])),
            Arc::new(arrow_array::LargeStringArray::from(vec![
                artifact.body.as_str()
            ])),
            Arc::new(StringArray::from(vec![artifact.depth.as_str()])),
            Arc::new(StringArray::from(vec![artifact.author.as_deref()])),
            Arc::new(StringArray::from(vec![artifact.parent_epic.as_deref()])),
            Arc::new(Float64Array::from(vec![0.0f64])),
            Arc::new(StringArray::from(vec![artifact.valid_until.as_deref()])),
            Arc::new(StringArray::from(vec![now])),
            Arc::new(StringArray::from(vec![now])),
            embedding_col,
        ],
    )?;
    Ok(batch)
}

/// Extract ArtifactSummary values from all rows in a RecordBatch.
pub fn batch_to_artifacts(batch: &RecordBatch) -> anyhow::Result<Vec<ArtifactSummary>> {
    let mut results = Vec::new();
    for row in 0..batch.num_rows() {
        if let Some(summary) = extract_summary(batch, row) {
            results.push(summary);
        }
    }
    Ok(results)
}

/// Build a one-row RecordBatch for a relation record.
pub fn relation_to_batch(
    source: &str,
    target: &str,
    relation: &str,
    now: &str,
) -> anyhow::Result<RecordBatch> {
    let schema = schema::relations_schema();
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(vec![source])),
            Arc::new(StringArray::from(vec![target])),
            Arc::new(StringArray::from(vec![relation])),
            Arc::new(StringArray::from(vec![now])),
        ],
    )?;
    Ok(batch)
}

fn extract_summary(batch: &RecordBatch, row: usize) -> Option<ArtifactSummary> {
    let id = get_string(batch, "id", row)?;
    let title = get_string(batch, "title", row).unwrap_or_default();
    let kind = get_string(batch, "kind", row).unwrap_or_default();
    let status = get_string(batch, "status", row).unwrap_or_default();

    Some(ArtifactSummary {
        id,
        title,
        kind,
        status,
    })
}

fn get_string(batch: &RecordBatch, col: &str, row: usize) -> Option<String> {
    let array = batch.column_by_name(col)?;
    if let Some(arr) = array.as_any().downcast_ref::<StringArray>() {
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    } else {
        None
    }
}

pub(crate) fn make_null_embedding_col(len: usize) -> Arc<dyn Array> {
    use arrow_array::FixedSizeListArray;
    use arrow_schema::Field;

    let item_field = Arc::new(Field::new("item", arrow_schema::DataType::Float32, true));
    Arc::new(FixedSizeListArray::new_null(item_field, schema::EMBEDDING_DIM, len))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::store::NewArtifact;

    fn sample_artifact() -> NewArtifact {
        NewArtifact {
            id: "PRD-001".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Test PRD".to_string(),
            body: "## Summary\n\nTest.".to_string(),
            depth: "standard".to_string(),
            author: Some("alice".to_string()),
            parent_epic: None,
            valid_until: None,
        }
    }

    #[test]
    fn artifact_to_batch_has_correct_row_count() {
        let artifact = sample_artifact();
        let batch = artifact_to_batch(&artifact, "2026-01-01T00:00:00Z").unwrap();
        assert_eq!(batch.num_rows(), 1);
    }

    #[test]
    fn artifact_to_batch_contains_id_and_kind() {
        let artifact = sample_artifact();
        let batch = artifact_to_batch(&artifact, "2026-01-01T00:00:00Z").unwrap();
        let id_col = batch
            .column_by_name("id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .unwrap();
        assert_eq!(id_col.value(0), "PRD-001");

        let kind_col = batch
            .column_by_name("kind")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .unwrap();
        assert_eq!(kind_col.value(0), "prd");
    }

    #[test]
    fn batch_to_artifacts_round_trips_summary() {
        let artifact = sample_artifact();
        let batch = artifact_to_batch(&artifact, "2026-01-01T00:00:00Z").unwrap();
        let summaries = batch_to_artifacts(&batch).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, "PRD-001");
        assert_eq!(summaries[0].kind, "prd");
        assert_eq!(summaries[0].status, "draft");
        assert_eq!(summaries[0].title, "Test PRD");
    }

    #[test]
    fn relation_to_batch_has_correct_columns() {
        let batch =
            relation_to_batch("PRD-001", "RFC-001", "informs", "2026-01-01T00:00:00Z").unwrap();
        assert_eq!(batch.num_rows(), 1);

        let src_col = batch
            .column_by_name("source_id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .unwrap();
        assert_eq!(src_col.value(0), "PRD-001");

        let tgt_col = batch
            .column_by_name("target_id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .unwrap();
        assert_eq!(tgt_col.value(0), "RFC-001");
    }

    #[test]
    fn artifact_to_batch_nullable_author_is_null_when_none() {
        let mut artifact = sample_artifact();
        artifact.author = None;
        let batch = artifact_to_batch(&artifact, "2026-01-01T00:00:00Z").unwrap();
        let author_col = batch
            .column_by_name("author")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .unwrap();
        assert!(author_col.is_null(0));
    }
}
