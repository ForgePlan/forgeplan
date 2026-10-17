//! The two active Lance tables: schemas, wrapped creation, restore, pinned reads.
//!
//! These are **derived** tables. They exist so a reader can answer from one
//! immutable generation; they never become a second source of truth, and
//! nothing here writes canonical markdown.
//!
//! # Why the creation call carries the wrapper
//!
//! `NativeTable::create_empty` takes a `write_store_wrapper`. That argument is
//! the only supported way to make Lance's writes observable — `lance_io` wraps
//! the object store the dataset then opens for every operation. A constructor
//! that accepted a wrapper and dropped it would make the store look consumed
//! while it observed nothing, which is worse than not having one: the closure
//! barrier would then publish a generation over an empty write set.
//!
//! # Why the URI scheme is not negotiable
//!
//! Every URI here comes from [`storage::database_uri`] /
//! [`storage::table_uri_for_role`], i.e. from the versioned protocol marker's
//! `storageScheme = file-object-store`. The plain `file` scheme reaches
//! `LocalWriter`/`std::fs` directly for creation and copy, which bypasses the
//! wrapped inner store entirely. Nothing in this module normalises a scheme
//! back to `file://`.

use std::sync::Arc;

use arrow_schema::{DataType, Field, Schema, SchemaRef};

use super::ConstellationError;
use super::durability::DurabilityTrackingStore;
use super::storage;

/// Vector dimension, reused from the existing `db` layer so a workspace built
/// by either path agrees on the embedding column.
pub use crate::db::schema::EMBEDDING_DIM;

/// Column names the binding contract requires on the artifacts table.
///
/// Named once so the schema, the row builder and the tests cannot drift.
pub const ARTIFACT_COLUMNS: &[&str] = &[
    // Display and content — the same vocabulary the existing `artifacts`
    // table uses, so a migration between the two is a projection, not a
    // translation.
    "id",
    "kind",
    "status",
    "title",
    "body",
    "depth",
    "author",
    "parent_epic",
    "r_eff_score",
    "valid_until",
    "created_at",
    "updated_at",
    "body_hash",
    "embedding",
    "tags",
    // Derived identity and binding.
    "canonical_key",
    "relative_path",
    "source_digest",
    "input_digest",
    "profile_id",
    "incarnation",
    "row_revision",
    "semantic_state",
    "retry_after",
    "diagnostics",
];

/// Column names the binding contract requires on the relations table.
pub const RELATION_COLUMNS: &[&str] = &[
    "canonical_key",
    "target",
    "relation",
    "source_digest",
    "row_revision",
    "incarnation",
];

/// Arrow schema for `constellation_artifacts_v1`.
///
/// The legacy fields (`body_hash`, `embedding`) are present because removing
/// them would break every workspace written by an older build — **not**
/// because they prove readiness. A row with a non-null `embedding` and a null
/// `profile_id` has no vector this protocol would serve.
pub fn artifacts_v1_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("kind", DataType::Utf8, false),
        Field::new("status", DataType::Utf8, false),
        Field::new("title", DataType::Utf8, false),
        Field::new("body", DataType::LargeUtf8, false),
        Field::new("depth", DataType::Utf8, true),
        Field::new("author", DataType::Utf8, true),
        Field::new("parent_epic", DataType::Utf8, true),
        Field::new("r_eff_score", DataType::Float64, true),
        Field::new("valid_until", DataType::Utf8, true),
        Field::new("created_at", DataType::Utf8, true),
        Field::new("updated_at", DataType::Utf8, true),
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
        Field::new("canonical_key", DataType::Utf8, false),
        Field::new("relative_path", DataType::Utf8, false),
        Field::new("source_digest", DataType::Utf8, false),
        Field::new("input_digest", DataType::Utf8, true),
        Field::new("profile_id", DataType::Utf8, true),
        Field::new("incarnation", DataType::UInt64, false),
        Field::new("row_revision", DataType::UInt64, false),
        // Generation identity, stamped on every row at publish time: the
        // guarded vector update predicates on these, and `read_binding`
        // reads them back. Without them a stale job from a previous
        // checkout or epoch could not be told apart from a live one.
        Field::new("index_epoch", DataType::UInt64, false),
        Field::new("checkout_id", DataType::Utf8, false),
        Field::new("semantic_state", DataType::Utf8, false),
        Field::new("retry_after", DataType::Utf8, true),
        Field::new("diagnostics", DataType::Utf8, true),
    ]))
}

/// Arrow schema for `constellation_relations_v1`.
///
/// `target` holds the **exact** bytes the canonical file declared. No
/// normalisation, no lower-casing: the graph's identity is case-sensitive and
/// the store's own `delete_relation` filter is too.
pub fn relations_v1_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("canonical_key", DataType::Utf8, false),
        Field::new("target", DataType::Utf8, false),
        Field::new("relation", DataType::Utf8, false),
        Field::new("source_digest", DataType::Utf8, false),
        Field::new("row_revision", DataType::UInt64, false),
        Field::new("incarnation", DataType::UInt64, false),
    ]))
}

/// Schema for one active table role.
pub fn schema_for_role(role: &str) -> Result<SchemaRef, ConstellationError> {
    match role {
        "artifacts" => Ok(artifacts_v1_schema()),
        "relations" => Ok(relations_v1_schema()),
        other => Err(ConstellationError::IndexProtocolMismatch {
            detail: format!("unknown active table role `{other}`"),
        }),
    }
}

/// The SQL literal for a fixed-size-list vector column.
///
/// # Why a literal at all
///
/// The conditional write goes through `UpdateBuilder::column(name, expr)`,
/// whose second argument is a SQL *expression*
/// (`lancedb-0.27.2/src/table/update.rs:57-63`), and an expression cannot
/// carry an in-memory buffer. A list literal is accepted for a
/// `FixedSizeList` column because Lance carries an explicit
/// `ScalarValue::List` → `FixedSizeList` coercion branch
/// (`lance-4.0.0/src/dataset/write/update.rs:134-147`) — that branch exists
/// precisely because DataFusion's generic cast does not do it.
///
/// # Length is load-bearing
///
/// `safe_coerce_scalar` returns `None` on a failed cast and Lance turns that
/// into an `ArrowError::CastError`, so a wrong-length literal is an **error**,
/// never a silent truncation or pad. The count therefore comes from
/// [`EMBEDDING_DIM`] — the dimension the schema declares — and a vector of
/// any other length is refused here, before it becomes ~12 KB of SQL text.
///
/// # Numeric formatting
///
/// `f32`'s `Display` never emits exponent notation, so every element renders
/// as plain decimals and the parser never sees `1e-7`. NaN and infinity have
/// no SQL literal form and are refused: emitting them would produce a vector
/// that parses unpredictably rather than one rejected where the bad value
/// entered.
///
/// # Cost
///
/// An element is roughly 12 bytes of text, so a 1024-dimension literal is
/// about 12 KB. It is built **once per publish call**, never once per row of a
/// batch — a caller looping it per row is doing something wrong.
pub(crate) fn embedding_literal(vector: &[f32], dim: usize) -> Result<String, ConstellationError> {
    if vector.len() != dim {
        return Err(ConstellationError::ModelUnavailable {
            detail: format!(
                "refusing to build a literal for a {dim}-dimension column from {} values",
                vector.len()
            ),
        });
    }
    let mut out = String::with_capacity(vector.len() * 12 + 2);
    out.push('[');
    for (index, value) in vector.iter().enumerate() {
        if !value.is_finite() {
            return Err(ConstellationError::ModelUnavailable {
                detail: format!(
                    "embedding element {index} is {value}; NaN and infinity have no SQL literal \
                     form and are refused rather than written"
                ),
            });
        }
        if index > 0 {
            out.push(',');
        }
        // `f32: Display` never uses exponent notation.
        out.push_str(&value.to_string());
    }
    out.push(']');
    Ok(out)
}

/// The active pair of tables, together with the connection that owns them.
///
/// Same shape decision as [`OpenedTable`], and the same reason. A `Table`
/// holds an `Arc<dyn Database>` (`table.rs:369-371`), so a table that outlives
/// the connection it came from points at a database nothing holds. On the
/// write side that is worse than a failed read: it is a commit whose
/// durability accounting is detached from the store that performed it.
///
/// So no accessor yields a bare `Table` or a bare `Connection`. The write path
/// needs *operations*, not handles, so it gets operations: `merge_into_*`,
/// `delete_from_*`, `update_*_columns`, `restore_*_to`, `*_version`. A future
/// caller who needs a raw handle must change this shape then, with the same
/// reasoning — not route around it now.
pub struct IndexTables {
    /// **Held, never read** — and that is the point.
    ///
    /// A `Table` holds an `Arc<dyn Database>` (`table.rs:369-371`), so it must
    /// never outlive the `Connection` that owns that database. Deleting this
    /// field would reintroduce the dropped-connection bug P2e-7 closed: a
    /// writer whose storage options were released while its handle lived.
    #[allow(dead_code)]
    connection: lancedb::Connection,
    artifacts: lancedb::Table,
    relations: lancedb::Table,
}

impl std::fmt::Debug for IndexTables {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexTables").finish_non_exhaustive()
    }
}

impl IndexTables {
    /// Private by construction: only the operations below may reach the table.
    fn artifacts_table(&self) -> &lancedb::Table {
        &self.artifacts
    }

    fn relations_table(&self) -> &lancedb::Table {
        &self.relations
    }

    /// The artifacts table's physical head version.
    pub async fn artifacts_version(&self) -> Result<u64, ConstellationError> {
        self.artifacts
            .version()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("cannot read the artifacts version: {e}"),
            })
    }

    /// The relations table's physical head version.
    pub async fn relations_version(&self) -> Result<u64, ConstellationError> {
        self.relations
            .version()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("cannot read the relations version: {e}"),
            })
    }

    /// Both head versions, as one pair for `compare_heads`.
    pub async fn head_versions(
        &self,
    ) -> Result<super::manifest::TableVersions, ConstellationError> {
        Ok(super::manifest::TableVersions::new(
            self.artifacts_version().await?,
            self.relations_version().await?,
        ))
    }

    /// `merge_insert` against the artifacts table.
    pub async fn merge_into_artifacts(
        &self,
        batch: Arc<arrow_array::RecordBatch>,
        full_source_batch: bool,
    ) -> Result<super::publisher::MergeOutcome, ConstellationError> {
        super::publisher::execute_merge(
            self.artifacts_table(),
            batch,
            full_source_batch,
            &super::publisher::MERGE_KEY,
        )
        .await
    }

    /// A bounded delete against the artifacts table.
    pub async fn delete_from_artifacts(
        &self,
        keys: &[String],
    ) -> Result<super::publisher::DeleteOutcome, ConstellationError> {
        let op = super::publisher::CommitOp::DeleteByPredicate {
            keys: keys.to_vec(),
        };
        super::publisher::execute_delete(self.artifacts_table(), &op).await
    }

    /// A bounded column update against the artifacts table.
    pub async fn update_artifacts_columns(
        &self,
        keys: &[String],
        assignments: &[super::publisher::ColumnAssignment],
    ) -> Result<super::publisher::UpdateOutcome, ConstellationError> {
        super::publisher::execute_column_update(self.artifacts_table(), keys, assignments).await
    }

    /// `merge_insert` against the relations table.
    ///
    /// Committed after the artifact upserts, so a row always exists before an
    /// edge points at it.
    pub async fn merge_into_relations(
        &self,
        batch: Arc<arrow_array::RecordBatch>,
        full_source_batch: bool,
    ) -> Result<super::publisher::MergeOutcome, ConstellationError> {
        super::publisher::execute_merge(
            self.relations_table(),
            batch,
            full_source_batch,
            &super::publisher::RELATIONS_MERGE_KEY,
        )
        .await
    }

    /// A bounded delete against the relations table, for replacing one
    /// artifact's outgoing set before its new edges are merged.
    pub async fn delete_from_relations(
        &self,
        keys: &[String],
    ) -> Result<super::publisher::DeleteOutcome, ConstellationError> {
        let op = super::publisher::CommitOp::DeleteByPredicate {
            keys: keys.to_vec(),
        };
        super::publisher::execute_delete(self.relations_table(), &op).await
    }

    /// Read every artifacts row and decide its eligibility.
    ///
    /// Returns an [`EligibilityReport`] and **no** `Table`, no `&Table` and no
    /// handle a caller could mutate through. That is the point of the operation:
    /// a caller learns *which* keys need work, and cannot then write a vector
    /// by another route.
    ///
    /// # Why this is a full scalar scan, with no `only_if` filter
    ///
    /// `QueryBase::only_if` (`lancedb-0.27.2/src/query.rs:402`, impl at
    /// `:530`, taking `impl AsRef<str>` — **not** `impl Into<String>` like
    /// `UpdateBuilder::only_if` at `table/update.rs:43`) exists and is not
    /// used here. Filtering would be the wrong instinct twice over:
    ///
    /// * A filter that drops rows we cannot evaluate makes them *disappear*
    ///   instead of becoming `Eligibility::CannotJudge`. A partial source
    ///   would then present as a short but complete work list.
    /// * There is no cheap predicate that separates "current" from "eligible":
    ///   current rows are defined by nine fields all matching, which is the
    ///   whole scan.
    ///
    /// # Two error types, mapped separately
    ///
    /// `ExecutableQuery::execute()` returns `lancedb::Result` — error type
    /// `lancedb::Error` (`error.rs:95`; `query.rs:634`). The stream it yields is a
    /// `SendableRecordBatchStream` (`lancedb-0.27.2/src/arrow.rs:58`) whose items
    /// are `lance::Result` via `RecordBatchStream` (`lance-io-4.0.0/src/stream.rs:24`).
    /// **The two do not unify**, so each is mapped at its own site and the
    /// messages distinguish *executing the query* (planning or I/O) from
    /// *collecting the stream* (mid-scan) — different problems.
    pub async fn artifacts_eligibility(
        &self,
        captured: &super::capture::CapturedSourceView,
        profile_id: &str,
    ) -> Result<super::jobs::EligibilityReport, ConstellationError> {
        use futures::TryStreamExt;
        use lancedb::query::ExecutableQuery;

        let stream = self
            .artifacts_table()
            .query()
            .execute()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("executing the eligibility query failed: {e}"),
            })?;

        let batches: Vec<arrow_array::RecordBatch> =
            stream
                .try_collect()
                .await
                .map_err(|e| ConstellationError::Io {
                    detail: format!("collecting the eligibility scan failed: {e}"),
                })?;

        let mut report = super::jobs::EligibilityReport::default();
        for batch in &batches {
            // Row indices come from `num_rows()` and column access from
            // `column_by_name`, so `row < num_rows()` holds for every column by
            // construction. `value()` **asserts** rather than returning an error
            // (`byte_array.rs:342-346`, `primitive_array.rs:772-776`), so the
            // bound is enforced here and every column is checked for presence
            // before it is indexed.
            let row_count = batch.num_rows();
            for row in 0..row_count {
                let Some(key) = batch
                    .column_by_name("canonical_key")
                    .and_then(|c| Self::read_str_from(c, row))
                else {
                    // No key means no way to name the row in a report.
                    continue;
                };
                let stored = Self::read_binding(batch, row);
                let captured_binding = match captured
                    .entries
                    .iter()
                    .find(|entry| entry.canonical_key == key)
                    .and_then(|entry| super::publisher::binding_for(entry, profile_id).ok())
                {
                    Some(binding) => binding,
                    None => {
                        // The row exists in the index but the capture does not
                        // describe it: it cannot be judged against a binding it
                        // has no counterpart for.
                        report.record(
                            key,
                            super::jobs::Eligibility::CannotJudge {
                                reason: "the row is not present in the capture".to_string(),
                            },
                        );
                        continue;
                    }
                };
                report.record(
                    key,
                    super::jobs::eligibility_for(&stored, &captured_binding),
                );
            }
        }
        Ok(report)
    }

    /// Project one row's stored binding.
    ///
    /// Each field is read independently, so a row missing one column is
    /// `CannotJudge` **for that field only** rather than being discarded.
    fn read_binding(batch: &arrow_array::RecordBatch, row: usize) -> super::jobs::StoredBinding {
        // Associated functions on `IndexTables`, called as `Self::…`. They take a
        // `RecordBatch` and a row and touch no `IndexTables` state, so they take
        // no `self` — but they sit inside the impl block, which requires the
        // qualified call form.
        let text = |name: &str| {
            batch
                .column_by_name(name)
                .and_then(|column| Self::read_str_from(column, row))
        };
        let number = |name: &str| {
            batch
                .column_by_name(name)
                .and_then(|column| Self::read_u64_from(column, row))
        };
        super::jobs::StoredBinding {
            index_epoch: number("index_epoch"),
            checkout_id: text("checkout_id"),
            incarnation: number("incarnation"),
            row_revision: number("row_revision"),
            source_digest: text("source_digest"),
            input_digest: text("input_digest"),
            profile_id: text("profile_id"),
            semantic_state: text("semantic_state"),
        }
    }

    fn read_u64_from(column: &arrow_array::ArrayRef, row: usize) -> Option<u64> {
        use arrow_array::Array;
        if row >= column.len() || column.is_null(row) {
            return None;
        }
        column
            .as_any()
            .downcast_ref::<arrow_array::UInt64Array>()
            .map(|array| array.value(row))
    }

    fn read_str_from(column: &arrow_array::ArrayRef, row: usize) -> Option<String> {
        use arrow_array::Array;
        if row >= column.len() || column.is_null(row) {
            return None;
        }
        column
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .map(|array| array.value(row).to_string())
    }

    /// Publish a vector onto an **existing** row, conditionally.
    ///
    /// This is a fifth operation on the sealed handle, and it is the only way
    /// in the tree that writes an `embedding` column. There is deliberately no
    /// unguarded alternative on `IndexTables`.
    ///
    /// # How it is conditional
    ///
    /// The guard is [`Binding::predicate`] — all nine fields, evaluated by the
    /// engine against **the row's own stored binding**, inside the write
    /// itself. A check performed by the caller before calling would be TOCTOU
    /// by construction: it can pass, and the row can change between the check
    /// and the update. Here there is no window, because the predicate and the
    /// update are one statement.
    ///
    /// A stale completion — an old `incarnation` after a delete/recreate, an
    /// old `row_revision` after an edit, an old `profile_id` after a model
    /// swap — matches nothing and affects **zero rows**. It cannot insert,
    /// because the operation is an update, and it cannot revive, because
    /// reviving would require matching a row that is gone.
    ///
    /// Note what the guard deliberately makes impossible: re-binding a row to
    /// a *different* profile. A profile change means the row is not current,
    /// and moving it into the new space is a merge on the publisher's
    /// schedule, never a late completion's privilege.
    ///
    /// # Pinned signatures, at the point of use
    ///
    /// ```text
    /// // lancedb-0.27.2/src/table.rs:598
    /// pub fn update(&self) -> UpdateBuilder
    ///
    /// // lancedb-0.27.2/src/table/update.rs:43-46
    /// pub fn only_if(mut self, filter: impl Into<String>) -> Self
    ///
    /// // lancedb-0.27.2/src/table/update.rs:57-63
    /// pub fn column(mut self, column_name: impl Into<String>,
    ///                update_expr: impl Into<String>) -> Self
    ///
    /// // lancedb-0.27.2/src/table/update.rs:65-73
    /// pub async fn execute(self) -> Result<UpdateResult>
    ///
    /// // lancedb-0.27.2/src/table/update.rs:14-21
    /// pub struct UpdateResult { pub rows_updated: u64, pub version: u64 }
    /// ```
    ///
    /// # Version rule
    ///
    /// Reused from P2, not re-decided: the reported version is used when
    /// non-zero — `UpdateResult.version` is documented `0` for backends that
    /// do not return one — otherwise `Table::version()` is read, and a read
    /// of `0` is an error rather than a recorded value.
    ///
    /// # Errors
    ///
    /// [`ConstellationError::ModelUnavailable`] when the vector's length does
    /// not match [`EMBEDDING_DIM`] or contains a non-finite value.
    /// [`ConstellationError::Io`] when the update or the version read fails.
    pub async fn publish_embedding_into_artifacts(
        &self,
        binding: &super::jobs::Binding,
        vector: &[f32],
    ) -> Result<super::jobs::EmbeddingPublish, ConstellationError> {
        let literal = embedding_literal(vector, EMBEDDING_DIM as usize)?;
        let guard = binding.predicate();

        // Vector, binding and `ready` in ONE operation: three separate
        // writes would let a reader observe a vector without the binding that
        // authorises it, which is the exact state a pinned reader must never
        // be handed.
        let updated = self
            .update_artifacts_columns_where(
                &guard,
                &[
                    ("embedding", literal),
                    ("semantic_state", "'ready'".to_string()),
                    (
                        "profile_id",
                        super::publisher::escape_predicate_literal(&binding.profile_id),
                    ),
                    (
                        "input_digest",
                        super::publisher::escape_predicate_literal(&binding.input_digest),
                    ),
                ],
            )
            .await?;

        // Zero rows is not an error. It is the answer: the row moved, and the
        // completion is stale.
        Ok(super::jobs::EmbeddingPublish {
            binding: binding.clone(),
            rows_updated: updated.rows_updated,
            version: updated.version,
            stale: updated.rows_updated == 0,
        })
    }

    /// A bounded column update under a caller-independent guard.
    ///
    /// Private: `only_if` takes an arbitrary predicate, and exposing it on a
    /// sealed handle would hand back the unguarded write this phase exists to
    /// remove. The only public conditional write is
    /// [`Self::publish_embedding_into_artifacts`], which builds its own guard
    /// from a [`Binding`](super::jobs::Binding).
    async fn update_artifacts_columns_where(
        &self,
        predicate: &str,
        columns: &[(&str, String)],
    ) -> Result<super::publisher::UpdateOutcome, ConstellationError> {
        if columns.is_empty() {
            return Err(ConstellationError::Io {
                detail: "a column update with no columns is rejected by Lance; \
                          refusing it here names the cause"
                    .to_string(),
            });
        }
        let mut builder = self
            .artifacts_table()
            .update()
            .only_if(predicate.to_string());
        for (name, expression) in columns {
            builder = builder.column((*name).to_string(), expression.clone());
        }
        let result = builder
            .execute()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("conditional column update on the artifacts table failed: {e}"),
            })?;

        // The P2 version rule, reused verbatim: reported when non-zero,
        // otherwise read, and a read of 0 is an error rather than a value.
        let (version, version_source) = if result.version != 0 {
            (result.version, super::publisher::VersionSource::Reported)
        } else {
            let read =
                self.artifacts_table()
                    .version()
                    .await
                    .map_err(|e| ConstellationError::Io {
                        detail: format!(
                            "cannot read the artifacts version after a column update: {e}"
                        ),
                    })?;
            if read == 0 {
                return Err(ConstellationError::Io {
                    detail: "the artifacts table reported version 0 after a column update"
                        .to_string(),
                });
            }
            (read, super::publisher::VersionSource::ReadFromTable)
        };

        Ok(super::publisher::UpdateOutcome {
            version,
            version_source,
            rows_updated: result.rows_updated,
            predicate: predicate.to_string(),
        })
    }

    /// Rewind the artifacts writer to the published baseline.
    pub async fn restore_artifacts_to(
        &self,
        published_version: u64,
    ) -> Result<u64, ConstellationError> {
        restore_to_published(self.artifacts_table(), "artifacts", published_version).await
    }

    /// Rewind the relations writer to the published baseline.
    pub async fn restore_relations_to(
        &self,
        published_version: u64,
    ) -> Result<u64, ConstellationError> {
        restore_to_published(self.relations_table(), "relations", published_version).await
    }
}

/// Open a connection on the wrapped `file-object-store` route.
///
/// Not a plain local root: the plain scheme bypasses the wrapper, and a
/// `Connection` on one route with tables on another is two storage paths to one
/// directory — the split brain ADR-016 exists to prevent.
pub async fn open_wrapped_connection(
    index_root: &std::path::Path,
) -> Result<lancedb::Connection, ConstellationError> {
    let uri = storage::database_uri(index_root);
    storage::assert_not_wrapped_route(&uri)?;
    lancedb::connect(&uri)
        .execute()
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot open {}: {e}", uri),
        })
}

/// Create both active tables through a wrapped writer.
///
/// **This is where `DurabilityTrackingStore` is consumed.** `write_store_wrapper`
/// is the pinned injection point (`lancedb-0.27.2/src/table.rs:1341-1358`
/// patches `ReadParams` from it, and `create`/`create_empty` pass it straight to
/// `ObjectStoreParams.object_store_wrapper`). Both tables receive the *same*
/// wrapper, which shares one [`super::durability::StageTracker`] — two trackers
/// would each see half the closure and `finish_writes` would report the other
/// half as untouched.
///
/// `params` and `namespace_client` are bare `None` because `lancedb` does not
/// re-export `WriteParams` or `LanceNamespace` (`lib.rs:266-268` re-exports only
/// `Session` and `ObjectStoreRegistry`), so naming them would force a direct
/// `lance` dependency for no benefit.
pub async fn create_active_tables(
    index_root: &std::path::Path,
    marker: &super::manifest::ProtocolMarker,
    wrapper: Arc<DurabilityTrackingStore>,
) -> Result<IndexTables, ConstellationError> {
    marker.validate()?;

    let connection = open_wrapped_connection(index_root).await?;
    let database = Arc::clone(connection.database());
    // The injection point is typed `Option<Arc<dyn WrappingObjectStore>>`.
    // Shadowing the parameter is where the unsizing coercion happens:
    // `Arc::clone` is `impl<T: ?Sized> Clone for Arc<T>` and does not unsize
    // on the way out.
    let wrapper: Arc<dyn lance_io::object_store::WrappingObjectStore> = wrapper;
    let mut created: Vec<lancedb::Table> = Vec::with_capacity(2);
    // `ACTIVE_TABLES` is `[(&str, &str)]` — `(role, table_name)` — so the role
    // is the tuple's first element, not the tuple itself.
    for (role, _) in storage::ACTIVE_TABLES {
        let uri = storage::table_uri_for_role(marker, role, index_root)?;
        storage::assert_not_wrapped_route(&uri)?;

        // Matched on the typed Lance error, not on its message. A string
        // match would break the first time Lance reworded it, and would
        // silently turn the restart transition into a failure — which would
        // make every restart of a bootstrapped workspace look broken.
        let native = match lancedb::table::NativeTable::create_empty(
            &uri,
            role,
            Vec::new(),
            schema_for_role(role)?,
            Some(Arc::clone(&wrapper)),
            None,
            None,
            None,
            false,
        )
        .await
        {
            Ok(table) => table,
            Err(lancedb::Error::TableAlreadyExists { .. }) => {
                return Err(ConstellationError::TableAlreadyExists {
                    role: role.to_string(),
                });
            }
            Err(e) => {
                return Err(ConstellationError::Io {
                    detail: format!("cannot create the {role} table at {uri}: {e}"),
                });
            }
        };

        created.push(lancedb::table::Table::new(
            Arc::new(native),
            Arc::clone(&database),
        ));
    }

    let relations = created.remove(1);
    let artifacts = created.remove(0);
    Ok(IndexTables {
        connection,
        artifacts,
        relations,
    })
}

/// Open the **existing** active pair for staging, at their current heads.
///
/// # How this differs from [`OpenedTable`] — they are not interchangeable
///
/// An [`OpenedTable`] is a **reader pinned to an exact version**: it checks
/// out `version`, exposes no write operation, and answers only about that
/// generation. `IndexTables` is a **writer at the current heads**: it checks
/// out nothing, and every operation it exposes mutates whatever the heads are
/// right now.
///
/// That distinction is the whole reason the types differ. A reader must be
/// pinned so it cannot answer a question about data that moved after it was
/// asked. A writer must *not* be pinned, because staging is supposed to write
/// on top of the current heads — pinning a writer would silently discard
/// whatever a concurrent stage did. Swapping one for the other is therefore
/// a bug in both directions, and there is no API here that lets it happen by
/// accident: `IndexTables` has no `checkout`, and `OpenedTable` has no
/// `merge`.
///
/// The same sealed shape as [`create_active_tables`]: private connection,
/// private tables, operations only.
///
/// The trackers are the publisher's. `wrapper` must be
/// [`super::publisher::IndexPublisher::tracking_store`], the same instance
/// whose closure `publish` reads. It is taken as a parameter rather than
/// looked up in a global, because a process-global store would be shared by
/// two publishers on two indexes — reintroducing exactly the two-tracker
/// defect P2e-7 closed, one level up.
///
/// # Errors
///
/// [`ConstellationError::IndexGenerationUnavailable`] when either table
/// cannot be opened. There is no fallback to latest and no silent creation:
/// a missing table on an index that claims to be active is a broken index,
/// not a first run, and bootstrap decides that case.
pub async fn open_active_tables(
    index_root: &std::path::Path,
    marker: &super::manifest::ProtocolMarker,
    wrapper: Arc<DurabilityTrackingStore>,
) -> Result<IndexTables, ConstellationError> {
    // Shadow the parameter with its trait-object form. `Arc::clone` is
    // `impl<T: ?Sized> Clone for Arc<T>` and does **not** unsize on the way
    // out, so coercing through a generic clone is a type error; shadowing the
    // binding is where the coercion actually happens.
    let wrapper: Arc<dyn lance_io::object_store::WrappingObjectStore> = wrapper;
    let mut opened = Vec::with_capacity(2);
    // `ACTIVE_TABLES` is `[(&str, &str)]` — `(role, table_name)` — so the role
    // is the tuple's first element, not the tuple itself.
    for (role, _) in storage::ACTIVE_TABLES {
        let uri = storage::table_uri_for_role(marker, role, index_root)?;
        storage::assert_not_wrapped_route(&uri)?;
        // A writer opens on the same wrapped route as its creator, with the
        // same wrapper the publisher holds, so the closure it writes into is
        // the publisher's.
        opened.push((
            role,
            lancedb::table::NativeTable::open_with_params(
                &uri,
                role,
                Vec::new(),
                Some(Arc::clone(&wrapper)),
                None,
                None,
                None,
                false,
                None,
            )
            .await
            .map_err(|e| ConstellationError::IndexGenerationUnavailable {
                detail: format!("cannot open the {role} table at {uri}: {e}"),
            })?,
        ));
    }

    let connection = open_wrapped_connection(index_root).await?;
    let database = Arc::clone(connection.database());
    let artifacts = lancedb::table::Table::new(Arc::new(opened.remove(0).1), Arc::clone(&database));
    let relations = lancedb::table::Table::new(Arc::new(opened.remove(0).1), database);

    Ok(IndexTables {
        connection,
        artifacts,
        relations,
    })
}

/// An opened table together with the connection that owns it.
///
/// # Why this type exists instead of returning the `Table`
///
/// A `Table` holds an `Arc<dyn Database>` (`table.rs:369-371`), so returning
/// one while dropping the `Connection` it came from leaves the handle pointing
/// at a database nothing holds any more. The mistake is invisible in review —
/// the signature is perfectly ordinary — and it only fails when the handle is
/// used after the connection is gone.
///
/// So this type does not expose a `Table` at all. It forwards the three
/// operations a pinned reader needs — `schema`, `version` and `query_batches`
/// — and there is consequently **no way** to obtain a `Table` that outlives
/// the connection. Returning `(Connection, Table)` would make the bug
/// *possible to write* rather than impossible to write, which is why that
/// shape was not chosen.
pub struct OpenedTable {
    /// **Held, never read** — the same lifetime role as
    /// [`IndexTables::connection`]. A pinned reader's `Table` must not outlive
    /// the connection that owns its database.
    #[allow(dead_code)]
    connection: lancedb::Connection,
    table: lancedb::Table,
}

impl std::fmt::Debug for OpenedTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `Table::schema()` is async in `lancedb 0.27.2`, so it cannot be
        // called from a synchronous `fmt`. The field count is not worth an
        // await in a diagnostic.
        f.debug_struct("OpenedTable").finish_non_exhaustive()
    }
}

impl OpenedTable {
    /// The table's Arrow schema at the pinned version.
    ///
    /// **Async**: `Table::schema()` returns a future in `lancedb 0.27.2`
    /// (`table.rs:556` region), not a value.
    pub async fn schema(&self) -> Result<arrow_schema::SchemaRef, ConstellationError> {
        self.table
            .schema()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("cannot read the pinned reader schema: {e}"),
            })
    }

    /// The version this handle is checked out at.
    pub async fn version(&self) -> Result<u64, ConstellationError> {
        self.table
            .version()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("cannot read the pinned table version: {e}"),
            })
    }

    /// Every row at the pinned version, as Arrow batches.
    ///
    /// The stream is consumed here so no `Table` and no stream escapes the
    /// owner. `ExecutableQuery::execute` returns a
    /// `SendableRecordBatchStream` (`query.rs:634`, implemented for `Query`
    /// at `query.rs:867`), and lancedb's own documented idiom for it is
    /// `stream.try_collect()` (`table.rs:909-910`).
    pub async fn query_batches(&self) -> Result<Vec<arrow_array::RecordBatch>, ConstellationError> {
        use futures::TryStreamExt;
        use lancedb::query::ExecutableQuery;

        let stream = self
            .table
            .query()
            .execute()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("executing the pinned reader query failed: {e}"),
            })?;
        stream
            .try_collect()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("reading the pinned reader stream failed: {e}"),
            })
    }

    /// Bounded scalar page for the per-document index inventory. Never loads
    /// bodies or embeddings into memory just to display index coverage.
    pub async fn artifact_page(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<arrow_array::RecordBatch>, ConstellationError> {
        use futures::TryStreamExt;
        use lancedb::query::{ExecutableQuery, QueryBase, Select};

        let stream = self
            .table
            .query()
            .select(Select::columns(&[
                "canonical_key",
                "id",
                "relative_path",
                "semantic_state",
                "source_digest",
            ]))
            .offset(offset)
            .limit(limit)
            .execute()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("executing pinned inventory query failed: {e}"),
            })?;
        stream
            .try_collect()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("reading pinned inventory query failed: {e}"),
            })
    }

    /// Stream only scalar fields needed to supplement the source-byte index:
    /// a canonical key may be derived from the filename rather than the
    /// markdown body. Returning `None` on overflow makes the caller use the
    /// complete pinned scan instead of silently omitting a matching row.
    pub async fn lexical_metadata_paths(
        &self,
        needle: &str,
        max_paths: usize,
    ) -> Result<Option<Vec<String>>, ConstellationError> {
        use arrow_array::{Array, StringArray};
        use futures::TryStreamExt;
        use lancedb::query::{ExecutableQuery, QueryBase, Select};

        let mut stream = self
            .table
            .query()
            .select(Select::columns(&[
                "canonical_key",
                "title",
                "relative_path",
            ]))
            .execute()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("executing pinned lexical metadata query failed: {e}"),
            })?;
        let mut paths = Vec::new();
        while let Some(batch) = stream
            .try_next()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("reading pinned lexical metadata failed: {e}"),
            })?
        {
            let read = |column: &str, row: usize| {
                batch
                    .column_by_name(column)
                    .and_then(|value| value.as_any().downcast_ref::<StringArray>())
                    .filter(|value| !value.is_null(row))
                    .map(|value| value.value(row))
            };
            for row in 0..batch.num_rows() {
                if (read("canonical_key", row)
                    .is_some_and(|value| value.to_lowercase().contains(needle))
                    || read("title", row)
                        .is_some_and(|value| value.to_lowercase().contains(needle)))
                    && let Some(path) = read("relative_path", row)
                {
                    paths.push(path.to_owned());
                    if paths.len() >= max_paths {
                        return Ok(None);
                    }
                }
            }
        }
        Ok(Some(paths))
    }

    /// Stream the lexical fallback without embedding vectors or accumulating
    /// the entire corpus in memory. Stop when the caller has enough hits.
    pub async fn scan_lexical_batches(
        &self,
        mut consume: impl FnMut(&arrow_array::RecordBatch) -> bool,
    ) -> Result<(), ConstellationError> {
        use futures::TryStreamExt;
        use lancedb::query::{ExecutableQuery, QueryBase, Select};

        let mut stream = self
            .table
            .query()
            .select(Select::columns(&[
                "canonical_key",
                "title",
                "body",
                "id",
                "kind",
                "status",
                "semantic_state",
            ]))
            .execute()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("executing pinned lexical scan failed: {e}"),
            })?;
        while let Some(batch) = stream
            .try_next()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("reading pinned lexical scan failed: {e}"),
            })?
        {
            if !consume(&batch) {
                break;
            }
        }
        Ok(())
    }

    /// Scalar scan restricted by `predicate` — the point-read half of the
    /// pinned surface: fetch exactly the rows a trigram shortlist names,
    /// instead of collecting the whole table.
    ///
    /// The predicate arrives already-escaped (`escape_predicate_literal`);
    /// this method never builds SQL from raw caller strings.
    pub async fn query_batches_where(
        &self,
        predicate: &str,
    ) -> Result<Vec<arrow_array::RecordBatch>, ConstellationError> {
        use futures::TryStreamExt;
        use lancedb::query::{ExecutableQuery, QueryBase};

        let stream = self
            .table
            .query()
            .only_if(predicate)
            .execute()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("executing the filtered pinned query failed: {e}"),
            })?;
        stream
            .try_collect()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("reading the filtered pinned stream failed: {e}"),
            })
    }

    /// Cosine ANN over the `embedding` column at the pinned version, restricted
    /// by `predicate` before ranking (`only_if` is a prefilter).
    ///
    /// The predicate is supplied already-escaped by the caller — this method
    /// never concatenates raw strings into SQL itself. The result keeps
    /// lance's `_distance` column; the caller converts distance to similarity
    /// so the score semantics stay visible at the call site.
    pub async fn nearest_embeddings(
        &self,
        vector: &[f32],
        predicate: &str,
        limit: usize,
    ) -> Result<Vec<arrow_array::RecordBatch>, ConstellationError> {
        use futures::TryStreamExt;
        use lancedb::query::{ExecutableQuery, QueryBase};

        let stream = self
            .table
            .vector_search(vector)
            .map_err(|e| ConstellationError::Io {
                detail: format!("building the pinned vector query failed: {e}"),
            })?
            .distance_type(lancedb::DistanceType::Cosine)
            .only_if(predicate)
            .limit(limit)
            .execute()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("executing the pinned vector query failed: {e}"),
            })?;
        stream
            .try_collect()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("reading the pinned vector stream failed: {e}"),
            })
    }
}

/// Open an existing table at an **exact** version, on the wrapped route.
///
/// Returns an [`OpenedTable`], which owns the connection; see that type for
/// why a bare `Table` is not returned.
///
/// **The reader is deliberately unwrapped.** `write_store_wrapper` is passed
/// as `None`, so this table observes nothing through a
/// [`DurabilityTrackingStore`]. Lance *can* patch a reader's `ReadParams`
/// from that argument (`lancedb-0.27.2/src/table.rs:1341-1358`), and nothing
/// here prevents a future caller from passing a wrapper. It is not passed
/// because:
///
/// * A reader needs no tracker and no write visibility; registering a read as
///   an intent would poison the stage for no reason.
/// * The durability closure is a **writer** concern. A read-back verifies the
///   *result* of that closure, it does not extend it.
/// * Wrapping a reader would put it on the same `ObjectStore` instance the
///   writer uses, so a read could observe the writer's in-flight writes —
///   exactly what a pinned reader must not do.
///
/// The table is still opened on the `file-object-store` scheme, which is the
/// part that matters for correctness: it is the plain `file` fast path, not
/// the wrapping, that would bypass the pinned store.
///
/// `checkout_latest` is never called. A reader that refreshed itself would
/// answer a different question than the one that pinned it.
pub async fn open_table_at_version(
    index_root: &std::path::Path,
    marker: &super::manifest::ProtocolMarker,
    role: &str,
    version: u64,
) -> Result<OpenedTable, ConstellationError> {
    let uri = storage::table_uri_for_role(marker, role, index_root)?;
    storage::assert_not_wrapped_route(&uri)?;
    let connection = open_wrapped_connection(index_root).await?;
    let database = Arc::clone(connection.database());

    let native = lancedb::table::NativeTable::open_with_params(
        &uri,
        role,
        Vec::new(),
        None,
        None,
        None,
        None,
        false,
        None,
    )
    .await
    .map_err(|e| ConstellationError::IndexGenerationUnavailable {
        detail: format!("cannot open the {role} table at {uri}: {e}"),
    })?;

    let table = lancedb::table::Table::new(Arc::new(native), database);
    // The exact version, never `checkout_latest`. A reader that refreshed
    // itself would answer a different question than the one that pinned it.
    table
        .checkout(version)
        .await
        .map_err(|e| ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "the {role} table has no version {version}: {e}. The generation is \
                 unavailable; there is no latest fallback."
            ),
        })?;
    Ok(OpenedTable { connection, table })
}

/// Rewind one writer to the published baseline: `checkout(v)` → `restore()` →
/// `checkout_latest()`.
///
/// Run **only** on divergence, and only for the affected table, before any
/// delta write. `restore` creates a **new physical head** holding the logical
/// published baseline — so the version number afterwards is not the one that
/// was published, and the candidate records the new pair.
///
/// # Errors
///
/// [`ConstellationError::IndexGenerationUnavailable`] when the published
/// version is gone. Restoring to latest instead would silently publish a
/// different generation under the descriptor's name.
///
/// Private: reachable only through [`IndexTables::restore_artifacts_to`] and
/// [`IndexTables::restore_relations_to`], which own the connection. A public
/// entry point here would take a bare `&Table` that nothing here can keep
/// alive.
async fn restore_to_published(
    table: &lancedb::Table,
    role: &str,
    published_version: u64,
) -> Result<u64, ConstellationError> {
    table.checkout(published_version).await.map_err(|e| {
        ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "cannot check out the published {role} version {published_version}: {e}"
            ),
        }
    })?;
    table.restore().await.map_err(|e| ConstellationError::Io {
        detail: format!("restore of the {role} table failed: {e}"),
    })?;
    table
        .checkout_latest()
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("could not return the {role} writer to latest after restore: {e}"),
        })?;
    // The head created by the restore. Distinct from `published_version` by
    // construction, and recorded as itself.
    table.version().await.map_err(|e| ConstellationError::Io {
        detail: format!("cannot read the {role} version after restore: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field_names(schema: &Schema) -> Vec<&str> {
        schema.fields().iter().map(|f| f.name().as_str()).collect()
    }

    #[test]
    fn the_artifacts_schema_carries_every_binding_column() {
        let schema = artifacts_v1_schema();
        let names = field_names(&schema);
        for required in ARTIFACT_COLUMNS {
            assert!(
                names.contains(required),
                "the binding contract requires `{required}`; schema has {names:?}"
            );
        }
    }

    #[test]
    fn the_artifacts_schema_keeps_the_legacy_columns_without_claiming_readiness() {
        let schema = artifacts_v1_schema();
        let names = field_names(&schema);
        assert!(names.contains(&"body_hash"));
        assert!(names.contains(&"embedding"));
        // Readiness is carried by the binding columns, not by those two.
        assert!(names.contains(&"profile_id"));
        assert!(names.contains(&"semantic_state"));
    }

    #[test]
    fn binding_columns_are_present_and_typed() {
        let schema = artifacts_v1_schema();
        let field = |name: &str| {
            schema
                .field_with_name(name)
                .unwrap_or_else(|e| panic!("missing {name}: {e}"))
                .clone()
        };
        assert_eq!(field("source_digest").data_type(), &DataType::Utf8);
        assert_eq!(field("input_digest").data_type(), &DataType::Utf8);
        assert_eq!(field("profile_id").data_type(), &DataType::Utf8);
        assert_eq!(field("incarnation").data_type(), &DataType::UInt64);
        assert_eq!(field("row_revision").data_type(), &DataType::UInt64);
        assert_eq!(field("semantic_state").data_type(), &DataType::Utf8);
        // Identity is not optional: a row without a canonical key is not a row.
        assert!(!field("canonical_key").is_nullable());
        assert!(!field("relative_path").is_nullable());
        // A missing profile is `null`, never an empty string standing in for one.
        assert!(field("profile_id").is_nullable());
        assert!(field("input_digest").is_nullable());
    }

    #[test]
    fn the_vector_column_matches_the_existing_db_dimension() {
        let schema = artifacts_v1_schema();
        let embedding = schema
            .field_with_name("embedding")
            .expect("embedding column")
            .data_type()
            .clone();
        match embedding {
            DataType::FixedSizeList(_, dim) => {
                assert_eq!(
                    dim, EMBEDDING_DIM,
                    "the db layer's dimension must not drift"
                )
            }
            other => panic!("expected a fixed-size list, got {other:?}"),
        }
    }

    #[test]
    fn the_relations_schema_carries_the_exact_outgoing_triple() {
        let schema = relations_v1_schema();
        let names = field_names(&schema);
        for required in RELATION_COLUMNS {
            assert!(
                names.contains(required),
                "the relation contract requires `{required}`; schema has {names:?}"
            );
        }
        let schema = relations_v1_schema();
        // Case-sensitive identity: the target column is plain Utf8 with no
        // normalisation, matching `Table::delete_relation`'s exact filter.
        let target = schema.field_with_name("target").expect("target");
        assert_eq!(target.data_type(), &DataType::Utf8);
        assert!(!target.is_nullable());
    }

    #[test]
    fn schema_for_role_resolves_both_active_tables_and_refuses_anything_else() {
        let artifacts_role = schema_for_role("artifacts").unwrap();
        let artifacts_schema = artifacts_v1_schema();
        assert_eq!(field_names(&artifacts_role), field_names(&artifacts_schema));
        let relations_role = schema_for_role("relations").unwrap();
        let relations_schema = relations_v1_schema();
        assert_eq!(field_names(&relations_role), field_names(&relations_schema));
        let err = schema_for_role("legacy").unwrap_err();
        assert_eq!(
            err.code(),
            super::super::DiagnosticCode::IndexProtocolMismatch
        );
    }

    #[test]
    fn the_two_active_schemas_share_exactly_the_binding_columns() {
        let schema_a = artifacts_v1_schema();
        let artifacts = field_names(&schema_a);
        let schema_r = relations_v1_schema();
        let relations = field_names(&schema_r);
        // Both tables bind their rows to the same source view: canonical_key
        // names the row, source_digest/row_revision/incarnation judge it.
        // Anything *more* shared would smear the two models together.
        let shared: Vec<&&str> = relations.iter().filter(|r| artifacts.contains(r)).collect();
        let mut shared: Vec<&str> = shared.into_iter().copied().collect();
        shared.sort();
        assert_eq!(
            shared,
            vec![
                "canonical_key",
                "incarnation",
                "row_revision",
                "source_digest"
            ],
            "only the binding columns are shared between the two active tables"
        );
    }
}
