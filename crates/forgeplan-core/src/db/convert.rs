use std::sync::Arc;

use arrow_array::{
    Array, Float64Array, Int32Array, ListArray, RecordBatch, StringArray,
    builder::{ListBuilder, StringBuilder},
};
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
    let tags_col = build_tags_list_array(std::iter::once(artifact.tags.as_slice()));

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(vec![artifact.id.as_str()])),
            Arc::new(StringArray::from(vec![artifact.kind.as_str()])),
            Arc::new(StringArray::from(vec![artifact.status.as_str()])),
            Arc::new(StringArray::from(vec![artifact.title.as_str()])),
            Arc::new(arrow_array::LargeStringArray::from(vec![
                artifact.body.as_str(),
            ])),
            Arc::new(StringArray::from(vec![artifact.depth.as_str()])),
            Arc::new(StringArray::from(vec![artifact.author.as_deref()])),
            Arc::new(StringArray::from(vec![artifact.parent_epic.as_deref()])),
            Arc::new(Float64Array::from(vec![0.0f64])),
            Arc::new(StringArray::from(vec![artifact.valid_until.as_deref()])),
            Arc::new(StringArray::from(vec![now])),
            Arc::new(StringArray::from(vec![now])),
            Arc::new(StringArray::from(vec![Option::<&str>::None])),
            embedding_col,
            Arc::new(tags_col),
        ],
    )?;
    Ok(batch)
}

/// Build a `ListArray` of `Utf8` tag rows from a sequence of tag slices.
///
/// One row per slice. Empty slices are written as empty (non-null) lists so
/// the column is always materialized with the declared inner nullability.
pub(crate) fn build_tags_list_array<'a, I>(rows: I) -> ListArray
where
    I: IntoIterator<Item = &'a [String]>,
{
    let mut builder = ListBuilder::new(StringBuilder::new());
    for row in rows {
        for tag in row {
            builder.values().append_value(tag);
        }
        builder.append(true);
    }
    builder.finish()
}

/// Extract tags as `Vec<String>` for a single row from a `ListArray` column.
pub(crate) fn extract_tags(batch: &RecordBatch, row: usize) -> Vec<String> {
    let Some(col) = batch.column_by_name("tags") else {
        return Vec::new();
    };
    let Some(list) = col.as_any().downcast_ref::<ListArray>() else {
        return Vec::new();
    };
    if list.is_null(row) {
        return Vec::new();
    }
    let values = list.value(row);
    let Some(strs) = values.as_any().downcast_ref::<StringArray>() else {
        return Vec::new();
    };
    (0..strs.len())
        .filter_map(|i| {
            if strs.is_null(i) {
                None
            } else {
                Some(strs.value(i).to_string())
            }
        })
        .collect()
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
    relation_to_batch_with_cl(source, target, relation, now, None)
}

/// Build a one-row RecordBatch for a relation record with explicit CL.
pub fn relation_to_batch_with_cl(
    source: &str,
    target: &str,
    relation: &str,
    now: &str,
    cl: Option<i32>,
) -> anyhow::Result<RecordBatch> {
    let schema = schema::relations_schema();
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(vec![source])),
            Arc::new(StringArray::from(vec![target])),
            Arc::new(StringArray::from(vec![relation])),
            Arc::new(StringArray::from(vec![now])),
            Arc::new(Int32Array::from(vec![cl])),
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
    Arc::new(FixedSizeListArray::new_null(
        item_field,
        schema::EMBEDDING_DIM,
        len,
    ))
}

/// Build a one-row embedding column from an optional vector.
///
/// If `embedding` is `Some`, builds a `FixedSizeListArray` with one non-null
/// row containing the values. If `None`, returns a one-row null embedding
/// column. Used by `replace_record` to preserve pre-computed embeddings
/// across full-row rewrites (PRD-035 / Sprint 13.3 C2 fix).
pub(crate) fn make_embedding_col_from_option(embedding: Option<&[f32]>) -> Arc<dyn Array> {
    use arrow_array::{FixedSizeListArray, Float32Array};
    use arrow_schema::Field;

    let dim = schema::EMBEDDING_DIM as usize;
    let item_field = Arc::new(Field::new("item", arrow_schema::DataType::Float32, true));

    match embedding {
        Some(vec) if vec.len() == dim => {
            let values = Arc::new(Float32Array::from(vec.to_vec()));
            Arc::new(FixedSizeListArray::new(
                item_field,
                schema::EMBEDDING_DIM,
                values,
                None,
            ))
        }
        // Wrong-length vectors fall back to a null row to avoid corrupting
        // the schema. Callers that care should re-embed.
        _ => make_null_embedding_col(1),
    }
}

/// Extract a single row's embedding vector from a `FixedSizeListArray` column.
///
/// Returns `None` when the column is missing, the row is null, or the inner
/// values cannot be downcast to `Float32Array`.
pub(crate) fn extract_embedding(batch: &RecordBatch, row: usize) -> Option<Vec<f32>> {
    use arrow_array::{FixedSizeListArray, Float32Array};
    let col = batch.column_by_name("embedding")?;
    let list = col.as_any().downcast_ref::<FixedSizeListArray>()?;
    if list.is_null(row) {
        return None;
    }
    let values = list.value(row);
    let arr = values.as_any().downcast_ref::<Float32Array>()?;
    Some((0..arr.len()).map(|i| arr.value(i)).collect())
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
            tags: Vec::new(),
        }
    }

    #[test]
    fn artifact_to_batch_persists_tags_list() {
        let mut artifact = sample_artifact();
        artifact.tags = vec!["source=code".to_string(), "layer=domain".to_string()];
        let batch = artifact_to_batch(&artifact, "2026-01-01T00:00:00Z").unwrap();
        let tags = extract_tags(&batch, 0);
        assert_eq!(tags, vec!["source=code", "layer=domain"]);
    }

    #[test]
    fn artifact_to_batch_empty_tags_extracts_empty() {
        let artifact = sample_artifact();
        let batch = artifact_to_batch(&artifact, "2026-01-01T00:00:00Z").unwrap();
        assert!(extract_tags(&batch, 0).is_empty());
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
