//! The three active Lance tables: schemas, wrapped creation, restore, pinned reads.
//!
//! `constellation_artifacts_v1` keeps exactly one row per canonical key,
//! `constellation_relations_v1` one row per outgoing edge, and
//! `constellation_sections_v2` one row per deterministic captured section.
//! The sections table exists so semantic coverage is not capped at the
//! 2000-character body prefix the artifact row embeds — a section's vector
//! is its own binding, and merging its ANN results with the artifact rows'
//! is what lets content past the prefix be found.
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

const AUTO_CLEANUP_INTERVAL_KEY: &str = "lance.auto_cleanup.interval";

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

/// Column names the binding contract requires on the sections table.
///
/// A section row carries its parent's binding columns verbatim — epoch,
/// checkout, incarnation, row revision, source digest — so a stale
/// completion predicates on exactly what the artifact row would, plus the
/// section's own `input_digest` over `title + heading + content prefix`.
///
/// `byte_start`/`byte_end` are here because the window they name is what makes
/// "these rows reproduce the body exactly" checkable against **published**
/// data. A dataset written before the token layout has no such columns, which
/// is why the sections role now resolves to
/// [`TABLE_SECTIONS_V2`](super::manifest::TABLE_SECTIONS_V2): a pre-v2 index
/// is refused by the `FORMAT_VERSION` check rather than reopened under a
/// schema it cannot satisfy.
pub const SECTION_COLUMNS: &[&str] = &[
    // Section identity.
    "canonical_key",
    "parent_key",
    "parent_display_id",
    "ordinal",
    // The byte range of this window inside the parent's body.
    "byte_start",
    "byte_end",
    "heading",
    "level",
    // Rendered attribution and the captured content itself.
    "title",
    "content",
    // The vector and the binding it was published under.
    "embedding",
    "source_digest",
    "input_digest",
    "profile_id",
    "incarnation",
    "row_revision",
    "index_epoch",
    "checkout_id",
    "semantic_state",
    "retry_after",
];

/// Arrow schema for `constellation_sections_v2`.
///
/// One row per captured section — `"{parent}#s{ordinal}"`, never one per
/// artifact. `content` is the exact section bytes (LargeUtf8 like the
/// artifact `body`); `heading`/`level` are `NULL` only on the ordinal-0
/// whole-body fallback and the preamble section, which have no markdown
/// heading. The parent's `title` and `source_digest` are denormalised in so
/// a merged ANN hit renders without a second lookup and a section row can be
/// judged by the same binding fields as its parent.
pub fn sections_v1_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("canonical_key", DataType::Utf8, false),
        Field::new("parent_key", DataType::Utf8, false),
        Field::new("parent_display_id", DataType::Utf8, true),
        Field::new("ordinal", DataType::UInt64, false),
        // The UTF-8 byte range this window covers inside the parent's body.
        // Recorded on the row so "the windows tile the body exactly" is
        // checkable against published data rather than only against the
        // in-memory capture that produced it.
        Field::new("byte_start", DataType::UInt64, false),
        Field::new("byte_end", DataType::UInt64, false),
        Field::new("heading", DataType::Utf8, true),
        Field::new("level", DataType::UInt64, true),
        Field::new("title", DataType::Utf8, false),
        Field::new("content", DataType::LargeUtf8, false),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIM,
            ),
            true,
        ),
        Field::new("source_digest", DataType::Utf8, false),
        Field::new("input_digest", DataType::Utf8, false),
        Field::new("profile_id", DataType::Utf8, true),
        Field::new("incarnation", DataType::UInt64, false),
        Field::new("row_revision", DataType::UInt64, false),
        Field::new("index_epoch", DataType::UInt64, false),
        Field::new("checkout_id", DataType::Utf8, false),
        Field::new("semantic_state", DataType::Utf8, false),
        Field::new("retry_after", DataType::Utf8, true),
    ]))
}

/// The columns a semantic readback projects.
///
/// Everything a preserve decision needs and nothing it does not: the row's
/// name, the identity tuple a stale completion is fenced by, the input the
/// stored vector was computed from, and the semantic columns themselves. The
/// rendered columns — `body`, `title`, `content`, `tags` — are deliberately
/// absent, because a readback that loaded the corpus text would make every
/// reconcile pay for the whole workspace twice.
///
/// Named once, here, so the two call sites (`artifact_semantics`,
/// `section_semantics`) cannot drift and no caller can inject a column name.
const SEMANTIC_READBACK_COLUMNS: &[&str] = &[
    "canonical_key",
    "index_epoch",
    "checkout_id",
    "incarnation",
    "input_digest",
    "profile_id",
    "semantic_state",
    "retry_after",
    "embedding",
];

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
        "sections" => Ok(sections_v1_schema()),
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

/// Active writer tables. Handles remain private to preserve durability tracking.
pub struct IndexTables {
    /// Retained with the writer handles for their database context.
    #[allow(dead_code)]
    connection: lancedb::Connection,
    artifacts: lancedb::Table,
    relations: lancedb::Table,
    sections: lancedb::Table,
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

    fn sections_table(&self) -> &lancedb::Table {
        &self.sections
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

    /// The sections table's physical head version.
    pub async fn sections_version(&self) -> Result<u64, ConstellationError> {
        self.sections
            .version()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("cannot read the sections version: {e}"),
            })
    }

    /// All three head versions, for `compare_heads` and for the manifest pin.
    ///
    /// This build always owns a sections table — created alongside the other
    /// two at bootstrap, or created on first open over a root that predates
    /// it — so all three legs of [`TableVersions`] are always real versions.
    /// A generation with no semantic sections pins the version its empty commit
    /// produced; `0` is never a version and never means "no sections".
    pub async fn head_versions(
        &self,
    ) -> Result<super::manifest::TableVersions, ConstellationError> {
        Ok(super::manifest::TableVersions::new(
            self.artifacts_version().await?,
            self.relations_version().await?,
            self.sections_version().await?,
        ))
    }

    /// Clear an unpublished bootstrap dataset when its complete replacement is
    /// empty. Never use this on a published generation.
    pub(super) async fn clear_empty_bootstrap_dataset(
        &self,
        role: &str,
    ) -> Result<(), ConstellationError> {
        let table = match role {
            "artifacts" => &self.artifacts,
            "relations" => &self.relations,
            "sections" => &self.sections,
            _ => return Err(ConstellationError::Io {
                detail: format!("unknown bootstrap dataset {role}"),
            }),
        };
        table.delete("true").await.map_err(|error| ConstellationError::Io {
            detail: format!("cannot clear unpublished {role} dataset: {error}"),
        })?;
        match role {
            "artifacts" => super::fault_barrier_if_selected("after_artifacts_commit"),
            "relations" => super::fault_barrier_if_selected("after_relations_commit"),
            "sections" => super::fault_barrier_if_selected("after_sections_commit"),
            _ => unreachable!(),
        }
        Ok(())
    }

    /// `merge_insert` against the artifacts table.
    pub async fn merge_into_artifacts(
        &self,
        batch: Arc<arrow_array::RecordBatch>,
        full_source_batch: bool,
    ) -> Result<super::publisher::MergeOutcome, ConstellationError> {
        let outcome = super::publisher::execute_merge(
            self.artifacts_table(),
            "artifacts",
            batch,
            full_source_batch,
            &super::publisher::MERGE_KEY,
        )
        .await?;
        // After the **real** artifacts commit returned. A pass that committed
        // nothing never reaches this line, so no stage is reported for it.
        super::fault_barrier_if_selected("after_artifacts_commit");
        Ok(outcome)
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
        super::publisher::execute_column_update(
            self.artifacts_table(),
            "artifacts",
            keys,
            assignments,
        )
        .await
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
        let outcome = super::publisher::execute_merge(
            self.relations_table(),
            "relations",
            batch,
            full_source_batch,
            &super::publisher::RELATIONS_MERGE_KEY,
        )
        .await?;
        super::fault_barrier_if_selected("after_relations_commit");
        Ok(outcome)
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

    /// `merge_insert` against the sections table.
    ///
    /// Keyed on the section's own `canonical_key` (`"{parent}#s{ordinal}"`).
    /// The batch is the **whole captured section set** — under
    /// `full_source_batch` the source-delete clause removes sections of
    /// deleted parents and sections an edit displaced, atomically inside the
    /// upsert commit.
    pub async fn merge_into_sections(
        &self,
        batch: Arc<arrow_array::RecordBatch>,
        full_source_batch: bool,
    ) -> Result<super::publisher::MergeOutcome, ConstellationError> {
        let outcome = super::publisher::execute_merge(
            self.sections_table(),
            "sections",
            batch,
            full_source_batch,
            &super::publisher::MERGE_KEY,
        )
        .await?;
        // Empty replacements are handled separately by bootstrap; the merge
        // primitive refuses an empty batch.
        super::fault_barrier_if_selected("after_sections_commit");
        Ok(outcome)
    }

    /// A bounded delete against the sections table, by **parent** key.
    ///
    /// The only shape the publish path needs: "every section of these gone
    /// artifacts". Section rows are always replaced by the merge, never
    /// deleted individually — the parent-key predicate is the whole delete
    /// surface.
    pub async fn delete_from_sections(
        &self,
        parent_keys: &[String],
    ) -> Result<super::publisher::DeleteOutcome, ConstellationError> {
        let op = super::publisher::CommitOp::SectionsDelete {
            parent_keys: parent_keys.to_vec(),
        };
        super::publisher::execute_delete(self.sections_table(), &op).await
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
        Self::artifacts_eligibility_for(self.artifacts_table(), captured, profile_id).await
    }

    async fn artifacts_eligibility_for(
        table: &lancedb::Table,
        captured: &super::capture::CapturedSourceView,
        profile_id: &str,
    ) -> Result<super::jobs::EligibilityReport, ConstellationError> {
        use futures::TryStreamExt;
        use lancedb::query::ExecutableQuery;

        let stream = table
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

    /// Read every sections row and decide its eligibility, keyed by section
    /// `canonical_key` (`"{parent}#s{ordinal}"`).
    ///
    /// Same rules as [`Self::artifacts_eligibility`]: a full scalar scan, no
    /// prefilter, and a row the capture does not describe is `CannotJudge`
    /// rather than missing. The captured side is rebuilt per section — the
    /// parent's durable fields (`incarnation`, `row_revision`,
    /// `source_digest`, epoch, checkout) plus the section's own
    /// `input_digest` — so a section whose parent moved is judged stale, not
    /// silently current.
    pub async fn sections_eligibility(
        &self,
        captured: &super::capture::CapturedSourceView,
        profile_id: &str,
    ) -> Result<super::jobs::EligibilityReport, ConstellationError> {
        Self::sections_eligibility_for(self.sections_table(), captured, profile_id).await
    }

    async fn sections_eligibility_for(
        table: &lancedb::Table,
        captured: &super::capture::CapturedSourceView,
        profile_id: &str,
    ) -> Result<super::jobs::EligibilityReport, ConstellationError> {
        use futures::TryStreamExt;
        use lancedb::query::ExecutableQuery;

        let stream =
            table
                .query()
                .execute()
                .await
                .map_err(|e| ConstellationError::Io {
                    detail: format!("executing the sections eligibility query failed: {e}"),
                })?;

        let batches: Vec<arrow_array::RecordBatch> =
            stream
                .try_collect()
                .await
                .map_err(|e| ConstellationError::Io {
                    detail: format!("collecting the sections eligibility scan failed: {e}"),
                })?;

        let mut report = super::jobs::EligibilityReport::default();
        for batch in &batches {
            let row_count = batch.num_rows();
            for row in 0..row_count {
                let Some(key) = batch
                    .column_by_name("canonical_key")
                    .and_then(|c| Self::read_str_from(c, row))
                else {
                    continue;
                };
                let stored = Self::read_binding(batch, row);
                // The captured section names its parent; the parent's
                // `SourceEntry` supplies the durable half of the binding.
                let captured_binding = captured
                    .section(&key)
                    .and_then(|section| {
                        captured
                            .entries
                            .iter()
                            .find(|entry| entry.canonical_key == section.parent_key)
                            .map(|entry| (section, entry))
                    })
                    .and_then(|(section, entry)| {
                        super::publisher::binding_for_section(section, entry, profile_id).ok()
                    });
                match captured_binding {
                    Some(binding) => {
                        report.record(key, super::jobs::eligibility_for(&stored, &binding));
                    }
                    None => {
                        report.record(
                            key,
                            super::jobs::Eligibility::CannotJudge {
                                reason:
                                    "the section row is not present in the capture, or its parent \
                                     is not"
                                        .to_string(),
                            },
                        );
                    }
                }
            }
        }
        Ok(report)
    }

    /// Read the stored semantic columns of every artifacts row, keyed by
    /// `canonical_key`.
    ///
    /// This is the **baseline** half of
    /// [`super::publisher::artifacts_batch`]: a reconcile's merge batch cannot
    /// compute embeddings, so the values a row already carries have to be read
    /// back and spliced in, or an unconditional `when_matched_update_all`
    /// writes the batch's nulls over a live vector.
    ///
    /// A narrow projection, not a full scan: `body`, `title`, `tags` and
    /// every other rendered column are excluded, so the cost is the identity
    /// and vector columns and not the corpus text. The column list is a
    /// module constant, never caller text.
    ///
    /// Reads the **open writer handle**, so it observes exactly the head the
    /// caller's baseline names — after any restore, before the first commit
    /// of this pass. That ordering is the caller's contract, stated here
    /// because a read after a commit would splice the pass's own nulls back.
    ///
    /// # Errors
    ///
    /// [`ConstellationError::Io`] when the query cannot be planned/executed or
    /// the stream cannot be collected. The two are mapped separately, matching
    /// [`Self::artifacts_eligibility`], because they are different failures.
    pub async fn artifact_semantics(
        &self,
    ) -> Result<
        std::collections::BTreeMap<String, super::publisher::SemanticColumns>,
        ConstellationError,
    > {
        let batches = self
            .semantics_batches(self.artifacts_table(), "artifacts")
            .await?;
        Ok(Self::semantic_map(&batches))
    }

    /// Read the stored semantic columns of every sections row, keyed by the
    /// section's own `canonical_key` (`"{parent}#s{ordinal}"`).
    ///
    /// Same projection and same ordering contract as
    /// [`Self::artifact_semantics`]. A section row carries its **parent's**
    /// `incarnation`/epoch/checkout, so the returned tuple is the one
    /// [`super::publisher::sections_batch`] judges the section's own
    /// `input_digest` against.
    pub async fn section_semantics(
        &self,
    ) -> Result<
        std::collections::BTreeMap<String, super::publisher::SemanticColumns>,
        ConstellationError,
    > {
        let batches = self
            .semantics_batches(self.sections_table(), "sections")
            .await?;
        Ok(Self::semantic_map(&batches))
    }

    /// Run the semantic-column projection over one active table.
    ///
    /// Split from [`Self::semantic_map`] so the query side and the decoding
    /// side stay separately readable, and so the two callers differ only in
    /// the table they name.
    async fn semantics_batches(
        &self,
        table: &lancedb::Table,
        role: &'static str,
    ) -> Result<Vec<arrow_array::RecordBatch>, ConstellationError> {
        use futures::TryStreamExt;
        use lancedb::query::{ExecutableQuery, QueryBase, Select};

        let stream = table
            .query()
            .select(Select::columns(SEMANTIC_READBACK_COLUMNS))
            .execute()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("executing the {role} semantic readback failed: {e}"),
            })?;
        stream
            .try_collect()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("reading the {role} semantic readback failed: {e}"),
            })
    }

    /// Decode projected batches into the preserve map.
    ///
    /// A row with no `canonical_key` is skipped — there would be no way to
    /// name it, and a map keyed by something else would be spliced onto the
    /// wrong row. Every other column is read independently, so a row missing
    /// one field simply lacks that field and fails the identity check,
    /// rather than aborting the whole readback.
    fn semantic_map(
        batches: &[arrow_array::RecordBatch],
    ) -> std::collections::BTreeMap<String, super::publisher::SemanticColumns> {
        let mut map = std::collections::BTreeMap::new();
        for batch in batches {
            let row_count = batch.num_rows();
            for row in 0..row_count {
                let Some(key) = batch
                    .column_by_name("canonical_key")
                    .and_then(|column| Self::read_str_from(column, row))
                else {
                    continue;
                };
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
                map.insert(
                    key,
                    super::publisher::SemanticColumns {
                        index_epoch: number("index_epoch"),
                        checkout_id: text("checkout_id"),
                        incarnation: number("incarnation"),
                        input_digest: text("input_digest"),
                        embedding: batch
                            .column_by_name("embedding")
                            .and_then(|column| Self::read_vector_from(column, row)),
                        profile_id: text("profile_id"),
                        semantic_state: text("semantic_state"),
                        retry_after: text("retry_after"),
                    },
                );
            }
        }
        map
    }

    /// Decode one `FixedSizeList<Float32>` cell into an owned vector.
    ///
    /// Same shape as `db::convert::extract_embedding`, re-derived here
    /// because this module must not reach into the legacy store's private
    /// helper. A null cell, a non-list column, or a list whose declared width
    /// is not [`EMBEDDING_DIM`] is `None`: a vector that cannot be spliced
    /// back into a batch is not a vector this protocol can preserve.
    fn read_vector_from(column: &arrow_array::ArrayRef, row: usize) -> Option<Vec<f32>> {
        use arrow_array::Array;
        if row >= column.len() || column.is_null(row) {
            return None;
        }
        let list = column
            .as_any()
            .downcast_ref::<arrow_array::FixedSizeListArray>()?;
        if list.value_length() != EMBEDDING_DIM {
            return None;
        }
        let values = list.value(row);
        let floats = values
            .as_any()
            .downcast_ref::<arrow_array::Float32Array>()?;
        Some((0..floats.len()).map(|i| floats.value(i)).collect())
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
    /// # The profile term is compared exactly, and admission is what makes it work
    ///
    /// The guard includes `profile_id = <binding.profile_id>` with no
    /// `OR … IS NULL`. That is sound because
    /// [`Self::admit_embedding_into_artifacts`] has already assigned the row
    /// this exact profile, under the owner, before the compute began. A late
    /// result from a superseded admission therefore matches nothing instead of
    /// migrating the row backwards — which is the only thing the missing
    /// disjunction used to permit.
    ///
    /// Moving a row into a new space is admission's job, on the publisher's
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
    /// remove. Every public conditional write builds its own guard from a
    /// [`Binding`](super::jobs::Binding) — the publish path with
    /// [`Self::publish_embedding_into_artifacts`]'s exact-profile guard, the
    /// admission path with [`Self::admit_embedding_into_artifacts`]'s
    /// freshness-only one.
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

    /// Assign one row to the target profile and mark it pending — **admission**.
    ///
    /// This is the write that makes the rest of the protocol sound. It runs
    /// under the owner, from the same capture the jobs are minted from, and it
    /// puts the row's `profile_id` on the target **before** any compute exists.
    /// Two things follow, and both are load-bearing:
    ///
    /// * A vector left over from another space stops being claimed. The row
    ///   now names profile B, so a reader asking for profile A cannot be
    ///   served B's bytes.
    /// * [`Binding::predicate`](super::jobs::Binding::predicate) can compare
    ///   `profile_id` exactly. It did not have to before, because nothing had
    ///   ever assigned the profile first; the gap it left is the one a late
    ///   result under the old profile walks through.
    ///
    /// # The guard is the freshness half, deliberately
    ///
    /// [`Binding::freshness_predicate`](super::jobs::Binding::freshness_predicate)
    /// — the seven freshness comparisons with **no** profile term. A row bound
    /// to an older space must be able to re-point, or a profile migration never
    /// converges; the profile term is exactly the comparison that would refuse
    /// it. This is the only caller of that form.
    ///
    /// # Updates only
    ///
    /// `update().only_if(..)` cannot insert. A key with no row matches nothing
    /// and reports `rows_updated: 0`, which is the honest answer: there is no
    /// row to admit, and the next reconcile creates one.
    ///
    /// # Not a vector write
    ///
    /// Nothing here touches `embedding`. The previous vector is left exactly as
    /// it is; `semantic_state = 'pending'` is what stops a reader from serving
    /// it, and a completion overwrites the column in one operation when it
    /// lands.
    ///
    /// # Errors
    ///
    /// [`ConstellationError::Io`] when the update or the version read fails.
    pub async fn admit_embedding_into_artifacts(
        &self,
        binding: &super::jobs::Binding,
    ) -> Result<super::publisher::UpdateOutcome, ConstellationError> {
        self.update_artifacts_columns_where(
            &binding.freshness_predicate(),
            &[
                (
                    "profile_id",
                    super::publisher::escape_predicate_literal(&binding.profile_id),
                ),
                (
                    "semantic_state",
                    super::publisher::escape_predicate_literal(
                        super::jobs::SemanticState::Pending.as_str(),
                    ),
                ),
            ],
        )
        .await
    }

    /// The section-row half of [`Self::admit_embedding_into_artifacts`].
    ///
    /// Identical contract. `canonical_key` is the section's own key and the
    /// freshness fields are the parent's, because a section row borrows its
    /// parent's durable identity.
    pub async fn admit_embedding_into_sections(
        &self,
        binding: &super::jobs::Binding,
    ) -> Result<super::publisher::UpdateOutcome, ConstellationError> {
        self.update_sections_columns_where(
            &binding.freshness_predicate(),
            &[
                (
                    "profile_id",
                    super::publisher::escape_predicate_literal(&binding.profile_id),
                ),
                (
                    "semantic_state",
                    super::publisher::escape_predicate_literal(
                        super::jobs::SemanticState::Pending.as_str(),
                    ),
                ),
            ],
        )
        .await
    }

    /// Record that one attempt at this row's vector failed, and when it may
    /// be attempted again.
    ///
    /// The same guarded contract as
    /// [`Self::publish_embedding_into_artifacts`] — [`Binding::predicate`],
    /// the exact eight comparisons — so a failure recorded for a row that has
    /// since moved writes nothing. It also **never touches `embedding`**: a
    /// failed recompute leaves the previous vector in place, and
    /// `semantic_state = 'failed'` is what stops a reader from being served it.
    /// That is the same rule admission follows.
    ///
    /// `diagnostics` carries the reason, so "this row failed" is answerable
    /// from the index alone rather than from a log nobody kept.
    ///
    /// # Errors
    ///
    /// [`ConstellationError::Io`] on a failed update or version read.
    pub async fn fail_embedding_into_artifacts(
        &self,
        binding: &super::jobs::Binding,
        retry_after: &str,
        diagnostics: &str,
    ) -> Result<super::publisher::UpdateOutcome, ConstellationError> {
        self.update_artifacts_columns_where(
            &binding.predicate(),
            &[
                (
                    "semantic_state",
                    super::publisher::escape_predicate_literal(
                        super::jobs::SemanticState::Failed.as_str(),
                    ),
                ),
                (
                    "retry_after",
                    super::publisher::escape_predicate_literal(retry_after),
                ),
                (
                    "diagnostics",
                    super::publisher::escape_predicate_literal(diagnostics),
                ),
            ],
        )
        .await
    }

    /// The section-row half of [`Self::fail_embedding_into_artifacts`].
    /// Sections store failure state and retry time, but have no diagnostics column.
    pub async fn fail_embedding_into_sections(
        &self,
        binding: &super::jobs::Binding,
        retry_after: &str,
    ) -> Result<super::publisher::UpdateOutcome, ConstellationError> {
        self.update_sections_columns_where(
            &binding.predicate(),
            &[
                (
                    "semantic_state",
                    super::publisher::escape_predicate_literal(
                        super::jobs::SemanticState::Failed.as_str(),
                    ),
                ),
                (
                    "retry_after",
                    super::publisher::escape_predicate_literal(retry_after),
                ),
            ],
        )
        .await
    }

    /// Rewind the artifacts writer to the published baseline.
    pub async fn restore_artifacts_to(
        &self,
        published_version: u64,
        workspace: &std::path::Path,
    ) -> Result<u64, ConstellationError> {
        restore_to_published(
            self.artifacts_table(),
            "artifacts",
            published_version,
            workspace,
        )
        .await
    }

    /// Rewind the relations writer to the published baseline.
    pub async fn restore_relations_to(
        &self,
        published_version: u64,
        workspace: &std::path::Path,
    ) -> Result<u64, ConstellationError> {
        restore_to_published(
            self.relations_table(),
            "relations",
            published_version,
            workspace,
        )
        .await
    }

    /// Rewind the sections writer to the published baseline.
    ///
    /// Called only when the published manifest actually pins a sections
    /// version — a pre-sections generation carries `None` and the head is
    /// free, never rewound against a pin that does not exist.
    pub async fn restore_sections_to(
        &self,
        published_version: u64,
        workspace: &std::path::Path,
    ) -> Result<u64, ConstellationError> {
        restore_to_published(
            self.sections_table(),
            "sections",
            published_version,
            workspace,
        )
        .await
    }

    pub(super) async fn restore_for_repair(
        &self,
        role: &str,
        version: u64,
        workspace: &std::path::Path,
        repair: &super::facade::RepairContext<'_>,
    ) -> Result<u64, ConstellationError> {
        let table = match role {
            "artifacts" => self.artifacts_table(),
            "relations" => self.relations_table(),
            "sections" => self.sections_table(),
            _ => return Err(ConstellationError::Io { detail: format!("unknown table role {role}") }),
        };
        restore_to_published_inner(table, role, version, workspace, Some(repair)).await
    }

    /// Publish a vector onto an **existing** section row, conditionally.
    ///
    /// The same guarded-write contract as
    /// [`Self::publish_embedding_into_artifacts`]: [`Binding::predicate`]
    /// predicates on the row's own stored binding — where `canonical_key` is
    /// the *section* key and `incarnation`/`row_revision`/`source_digest` are
    /// the parent's — so a completion computed against a superseded parent
    /// body or a displaced section matches nothing and updates zero rows.
    /// `stale` is a result, not a failure.
    ///
    /// # Errors
    ///
    /// [`ConstellationError::ModelUnavailable`] on a wrong-dimension or
    /// non-finite vector; [`ConstellationError::Io`] on a failed update.
    pub async fn publish_embedding_into_sections(
        &self,
        binding: &super::jobs::Binding,
        vector: &[f32],
    ) -> Result<super::jobs::EmbeddingPublish, ConstellationError> {
        let literal = embedding_literal(vector, EMBEDDING_DIM as usize)?;
        let guard = binding.predicate();

        let updated = self
            .update_sections_columns_where(
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

        Ok(super::jobs::EmbeddingPublish {
            binding: binding.clone(),
            rows_updated: updated.rows_updated,
            version: updated.version,
            stale: updated.rows_updated == 0,
        })
    }

    /// A bounded column update on the sections table under a
    /// caller-independent guard — private for the same reason as
    /// [`Self::update_artifacts_columns_where`]: an arbitrary `only_if` on a
    /// sealed handle is the unguarded write this layer exists to remove.
    async fn update_sections_columns_where(
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
            .sections_table()
            .update()
            .only_if(predicate.to_string());
        for (name, expression) in columns {
            builder = builder.column((*name).to_string(), expression.clone());
        }
        let result = builder
            .execute()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("conditional column update on the sections table failed: {e}"),
            })?;

        let (version, version_source) = if result.version != 0 {
            (result.version, super::publisher::VersionSource::Reported)
        } else {
            let read =
                self.sections_table()
                    .version()
                    .await
                    .map_err(|e| ConstellationError::Io {
                        detail: format!(
                            "cannot read the sections version after a column update: {e}"
                        ),
                    })?;
            if read == 0 {
                return Err(ConstellationError::Io {
                    detail: "the sections table reported version 0 after a column update"
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
/// Creation uses the default write and namespace parameters. Retention is
/// established explicitly below because our generation manifests pin versions
/// independently of Lance's age-based cleanup policy.
/// The caller holds the owner; each retention commit takes the lock for the
/// explicitly supplied canonical workspace, never an inferred parent path.
pub async fn create_active_tables(
    index_root: &std::path::Path,
    marker: &super::manifest::ProtocolMarker,
    wrapper: Arc<DurabilityTrackingStore>,
    workspace: &std::path::Path,
) -> Result<IndexTables, ConstellationError> {
    create_active_tables_inner(index_root, marker, wrapper, workspace, false)
        .await
        .map(|(tables, _)| tables)
}

/// Resume an unpublished bootstrap, creating or opening each dataset separately.
/// The caller must have established that CURRENT is absent while holding the owner.
pub(super) async fn resume_bootstrap_tables(
    index_root: &std::path::Path,
    marker: &super::manifest::ProtocolMarker,
    wrapper: Arc<DurabilityTrackingStore>,
    workspace: &std::path::Path,
) -> Result<(IndexTables, bool), ConstellationError> {
    create_active_tables_inner(index_root, marker, wrapper, workspace, true).await
}

async fn create_active_tables_inner(
    index_root: &std::path::Path,
    marker: &super::manifest::ProtocolMarker,
    wrapper: Arc<DurabilityTrackingStore>,
    workspace: &std::path::Path,
    resume: bool,
) -> Result<(IndexTables, bool), ConstellationError> {
    marker.validate()?;

    let connection = open_wrapped_connection(index_root).await?;
    let database = Arc::clone(connection.database());
    // The injection point is typed `Option<Arc<dyn WrappingObjectStore>>`.
    // Shadowing the parameter is where the unsizing coercion happens:
    // `Arc::clone` is `impl<T: ?Sized> Clone for Arc<T>` and does not unsize
    // on the way out.
    let wrapper: Arc<dyn lance_io::object_store::WrappingObjectStore> = wrapper;
    let mut created: Vec<lancedb::Table> = Vec::with_capacity(super::storage::ACTIVE_TABLES.len());
    let mut all_created = true;
    // `ACTIVE_TABLES` is `[(&str, &str)]` — `(role, table_name)` — so the role
    // is the tuple's first element, not the tuple itself.
    for (role, _) in storage::ACTIVE_TABLES {
        let uri = storage::table_uri_for_role(marker, role, index_root)?;
        storage::assert_not_wrapped_route(&uri)?;

        // Opening first avoids a failed create operation contaminating the
        // durability tracker when resuming an already-created dataset.
        let existing = if resume {
            match lancedb::table::NativeTable::open_with_params(
                &uri, role, Vec::new(), Some(Arc::clone(&wrapper)),
                None, None, None, false, None,
            ).await {
                Ok(table) => {
                    all_created = false;
                    Some(table)
                }
                Err(lancedb::Error::TableNotFound { .. }) => None,
                Err(error) => return Err(ConstellationError::IndexGenerationUnavailable {
                    detail: format!("cannot open the {role} bootstrap table at {uri}: {error}"),
                }),
            }
        } else {
            None
        };
        let native = if let Some(table) = existing {
            table
        } else {
            match lancedb::table::NativeTable::create_empty(
                &uri, role, Vec::new(), schema_for_role(role)?,
                Some(Arc::clone(&wrapper)), None, None, None, false,
            ).await {
                Ok(table) => table,
                Err(lancedb::Error::TableAlreadyExists { .. }) => {
                    return Err(ConstellationError::TableAlreadyExists { role: role.to_string() });
                }
                Err(error) => return Err(ConstellationError::Io {
                    detail: format!("cannot create the {role} table at {uri}: {error}"),
                }),
            }
        };

        created.push(lancedb::table::Table::new(
            Arc::new(native),
            Arc::clone(&database),
        ));
    }

    // `created` is in `ACTIVE_TABLES` order: artifacts, relations, sections.
    let mut iter = created.into_iter();
    let artifacts = iter.next().ok_or_else(|| ConstellationError::Io {
        detail: "table creation produced no artifacts table".to_string(),
    })?;
    let relations = iter.next().ok_or_else(|| ConstellationError::Io {
        detail: "table creation produced no relations table".to_string(),
    })?;
    let sections = iter.next().ok_or_else(|| ConstellationError::Io {
        detail: "table creation produced no sections table".to_string(),
    })?;
    retain_managed_table_versions(&artifacts, "artifacts", workspace).await?;
    retain_managed_table_versions(&relations, "relations", workspace).await?;
    retain_managed_table_versions(&sections, "sections", workspace).await?;
    Ok((IndexTables {
        connection,
        artifacts,
        relations,
        sections,
    }, all_created))
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
///
/// The caller holds the owner. Retention commits lock the supplied canonical
/// workspace individually; genuinely read-only opens use the separate reader.
pub async fn open_active_tables(
    index_root: &std::path::Path,
    marker: &super::manifest::ProtocolMarker,
    wrapper: Arc<DurabilityTrackingStore>,
    workspace: &std::path::Path,
) -> Result<IndexTables, ConstellationError> {
    open_active_tables_inner(index_root, marker, wrapper, workspace, None).await
}

pub(super) async fn open_active_tables_for_repair(
    index_root: &std::path::Path,
    marker: &super::manifest::ProtocolMarker,
    wrapper: Arc<DurabilityTrackingStore>,
    workspace: &std::path::Path,
    repair: &super::facade::RepairContext<'_>,
) -> Result<IndexTables, ConstellationError> {
    open_active_tables_inner(index_root, marker, wrapper, workspace, Some(repair)).await
}

async fn open_active_tables_inner(
    index_root: &std::path::Path,
    marker: &super::manifest::ProtocolMarker,
    wrapper: Arc<DurabilityTrackingStore>,
    workspace: &std::path::Path,
    repair: Option<&super::facade::RepairContext<'_>>,
) -> Result<IndexTables, ConstellationError> {
    // Shadow the parameter with its trait-object form. `Arc::clone` is
    // `impl<T: ?Sized> Clone for Arc<T>` and does **not** unsize on the way
    // out, so coercing through a generic clone is a type error; shadowing the
    // binding is where the coercion actually happens.
    let wrapper: Arc<dyn lance_io::object_store::WrappingObjectStore> = wrapper;
    let mut opened = Vec::with_capacity(super::storage::ACTIVE_TABLES.len());
    // `ACTIVE_TABLES` is `[(&str, &str)]` — `(role, table_name)` — so the role
    // is the tuple's first element, not the tuple itself.
    for (role, table_name) in storage::ACTIVE_TABLES {
        // `resolve_table_uri`, not `table_uri_for_role`: the sections role must
        // resolve to `TABLE_SECTIONS_V2` even when the marker names something
        // else. Artifacts/relations keep strict lookup.
        //
        // History, stated as history: this tolerance originally existed for a
        // two-name marker written before `constellation_sections_v1` existed at
        // all. That case is gone — such a marker declares `protocol_version` 1
        // and is refused by `ProtocolMarker::validate` with
        // `index_protocol_mismatch` before this loop runs. What survives is the
        // v1→v2 rename: a v2 marker is required to name the v2 sections name,
        // and this resolution is the single place that enforces it.
        let uri = storage::resolve_table_uri(index_root, marker, role, table_name)?;
        storage::assert_not_wrapped_route(&uri)?;
        // A writer opens on the same wrapped route as its creator, with the
        // same wrapper the publisher holds, so the closure it writes into is
        // the publisher's.
        let native = match lancedb::table::NativeTable::open_with_params(
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
        {
            Ok(native) => native,
            // First publication of the v2 sections dataset.
            //
            // This is **not** a v1 upgrade path and must not be read as one. A
            // pre-v2 index is refused before this loop runs, by the
            // `protocol_version` check in `resolve_table_uri` →
            // `ProtocolMarker::validate`: it cannot reach here with a v1
            // manifest. So this arm fires only when a v2 index has been marked
            // active and is opening its sections dataset for the first time —
            // an initial bootstrap, or the first admission after a v2 rebuild.
            //
            // Creating it here — under the owner the caller already holds and
            // through the same tracking wrapper — keeps the write inside the
            // closure barrier that will sync it. It is not a silent repair: a
            // manifest that pins a sections version still fails its pinned
            // open at `open_table_at_version`.
            Err(lancedb::Error::TableNotFound { .. }) if role == "sections" => {
                lancedb::table::NativeTable::create_empty(
                    &uri,
                    role,
                    Vec::new(),
                    sections_v1_schema(),
                    Some(Arc::clone(&wrapper)),
                    None,
                    None,
                    None,
                    false,
                )
                .await
                .map_err(|e| ConstellationError::Io {
                    detail: format!("cannot create the {role} table at {uri}: {e}"),
                })?
            }
            Err(e) => {
                return Err(ConstellationError::IndexGenerationUnavailable {
                    detail: format!("cannot open the {role} table at {uri}: {e}"),
                });
            }
        };
        // No schema check here, deliberately. The sections role resolves to
        // `TABLE_SECTIONS_V2`, so the dataset this build can reach is either
        // absent — created above with the current schema — or one this build
        // wrote. A pre-v2 index is refused earlier, by the `FORMAT_VERSION`
        // check in `ProtocolMarker::validate` and
        // `GenerationManifest::validate_for_publication`, so there is no
        // reachable dataset whose schema could disagree with
        // `sections_v1_schema()`.
        opened.push((role, native));
    }

    let connection = open_wrapped_connection(index_root).await?;
    let database = Arc::clone(connection.database());
    // `opened` is in `ACTIVE_TABLES` order: artifacts, relations, sections.
    let mut iter = opened.into_iter();
    let artifacts = lancedb::table::Table::new(
        Arc::new(
            iter.next()
                .ok_or_else(|| ConstellationError::Io {
                    detail: "no artifacts table was opened".to_string(),
                })?
                .1,
        ),
        Arc::clone(&database),
    );
    let relations = lancedb::table::Table::new(
        Arc::new(
            iter.next()
                .ok_or_else(|| ConstellationError::Io {
                    detail: "no relations table was opened".to_string(),
                })?
                .1,
        ),
        Arc::clone(&database),
    );
    let sections = lancedb::table::Table::new(
        Arc::new(
            iter.next()
                .ok_or_else(|| ConstellationError::Io {
                    detail: "no sections table was opened".to_string(),
                })?
                .1,
        ),
        database,
    );

    retain_managed_table_versions_inner(&artifacts, "artifacts", workspace, repair).await?;
    retain_managed_table_versions_inner(&relations, "relations", workspace, repair).await?;
    retain_managed_table_versions_inner(&sections, "sections", workspace, repair).await?;
    Ok(IndexTables {
        connection,
        artifacts,
        relations,
        sections,
    })
}

/// Read-only active tables, exposing eligibility scans but no mutations.
pub struct ReadonlyIndexTables {
    artifacts: lancedb::Table,
    sections: lancedb::Table,
}

impl ReadonlyIndexTables {
    pub async fn artifacts_eligibility(
        &self,
        captured: &super::capture::CapturedSourceView,
        profile_id: &str,
    ) -> Result<super::jobs::EligibilityReport, ConstellationError> {
        IndexTables::artifacts_eligibility_for(&self.artifacts, captured, profile_id).await
    }

    pub async fn sections_eligibility(
        &self,
        captured: &super::capture::CapturedSourceView,
        profile_id: &str,
    ) -> Result<super::jobs::EligibilityReport, ConstellationError> {
        IndexTables::sections_eligibility_for(&self.sections, captured, profile_id).await
    }
}

/// Open existing active tables without connecting to or creating a database.
pub async fn open_active_tables_readonly(
    index_root: &std::path::Path,
    marker: &super::manifest::ProtocolMarker,
) -> Result<ReadonlyIndexTables, ConstellationError> {
    let mut opened = Vec::with_capacity(storage::ACTIVE_TABLES.len());
    for (role, table_name) in storage::ACTIVE_TABLES {
        let uri = storage::resolve_table_uri(index_root, marker, role, table_name)?;
        storage::assert_not_wrapped_route(&uri)?;
        let path = index_root.join(format!("{table_name}.lance"));
        let table = open_existing_native_table(&path, role).await.map_err(|e| {
            ConstellationError::IndexGenerationUnavailable {
                detail: format!("cannot open the {role} table at {}: {e}", path.display()),
            }
        })?;
        opened.push(table);
    }
    let mut iter = opened.into_iter();
    let artifacts = iter.next().ok_or_else(|| ConstellationError::Io {
        detail: "no artifacts table was opened".to_string(),
    })?;
    let _relations = iter.next().ok_or_else(|| ConstellationError::Io {
        detail: "no relations table was opened".to_string(),
    })?;
    let sections = iter.next().ok_or_else(|| ConstellationError::Io {
        detail: "no sections table was opened".to_string(),
    })?;
    Ok(ReadonlyIndexTables { artifacts, sections })
}

/// Open an existing local dataset without the create-capable database API.
pub(crate) async fn open_existing_native_table(
    dataset_path: &std::path::Path,
    table_name: &str,
) -> anyhow::Result<lancedb::Table> {
    let absolute = if dataset_path.is_absolute() {
        dataset_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(dataset_path)
    };
    let uri = reqwest::Url::from_file_path(&absolute)
        .map_err(|_| anyhow::anyhow!("invalid dataset path: {}", absolute.display()))?;
    let native = lancedb::table::NativeTable::open_with_params(
        uri.as_str(), table_name, Vec::new(), None, None, None, None, false, None,
    ).await?;
    let base: Arc<dyn lancedb::table::BaseTable> = Arc::new(native);
    Ok(lancedb::Table::from(base))
}

/// A table pinned to an exact version, exposing read operations only.
pub struct OpenedTable {
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

    /// All rows of the named scalar columns — the cheap shape a reader
    /// takes when it needs every key/edge but never a body or vector.
    /// Column list is fixed by the caller, never user text.
    pub async fn select_columns(
        &self,
        columns: &[&str],
    ) -> Result<Vec<arrow_array::RecordBatch>, ConstellationError> {
        use futures::TryStreamExt;
        use lancedb::query::{ExecutableQuery, QueryBase, Select};

        let stream = self
            .table
            .query()
            .select(Select::columns(columns))
            .execute()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("executing the pinned column scan failed: {e}"),
            })?;
        stream
            .try_collect()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("reading the pinned column scan failed: {e}"),
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

    /// The `canonical_key` of every row matching `predicate`, at the pinned
    /// version.
    ///
    /// Only the key column is projected, so a filter this wide (every row of
    /// a kind, say) still costs one narrow column scan rather than the corpus
    /// text. It exists for the one filter a row-level column cannot express:
    /// a **section** row carries no `kind`/`status` of its own, so restricting
    /// sections to the documents a filter admits needs their parents' keys.
    ///
    /// The predicate arrives already-escaped, as everywhere else here.
    pub async fn keys_where(
        &self,
        predicate: &str,
    ) -> Result<std::collections::BTreeSet<String>, ConstellationError> {
        use futures::TryStreamExt;
        use lancedb::query::{ExecutableQuery, QueryBase, Select};

        let stream = self
            .table
            .query()
            .select(Select::columns(&["canonical_key"]))
            .only_if(predicate)
            .execute()
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("executing the pinned key scan failed: {e}"),
            })?;
        let batches: Vec<arrow_array::RecordBatch> =
            stream
                .try_collect()
                .await
                .map_err(|e| ConstellationError::Io {
                    detail: format!("reading the pinned key stream failed: {e}"),
                })?;
        let mut keys = std::collections::BTreeSet::new();
        for batch in &batches {
            let Some(column) = batch.column_by_name("canonical_key") else {
                continue;
            };
            for row in 0..batch.num_rows() {
                if let Some(key) = IndexTables::read_str_from(column, row) {
                    keys.insert(key);
                }
            }
        }
        Ok(keys)
    }

    /// How many rows at the pinned version match `predicate` — with **no**
    /// limit.
    ///
    /// `Table::count_rows`
    /// (`lancedb-0.27.2/src/table.rs:565-568`) takes the same SQL filter
    /// `only_if` takes and runs it as a `Dataset::count_rows` scan
    /// (`table.rs:2212-2220`), so this counts at the version this handle
    /// checked out and never at latest.
    ///
    /// This is the count a coverage question needs. Counting the rows an ANN
    /// query *returned* answers a different question — "how many did the limit
    /// admit" — and a caller that reports the second as the first will claim a
    /// corpus is fully embedded while most of it never entered the ranking.
    ///
    /// The predicate arrives already-escaped, exactly as for
    /// [`Self::query_batches_where`].
    pub async fn count_rows_where(&self, predicate: &str) -> Result<usize, ConstellationError> {
        self.table
            .count_rows(Some(predicate.to_string()))
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("counting the pinned rows matching the filter failed: {e}"),
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

/// Open an existing table at an **exact** version, exposing read operations only.
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
/// The reader uses a local file URL and does not use a writer store wrapper.
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
    let table_name = storage::ACTIVE_TABLES.iter()
        .find(|(candidate, _)| *candidate == role)
        .map(|(_, name)| *name)
        .ok_or_else(|| ConstellationError::IndexProtocolMismatch {
            detail: format!("unknown active table role `{role}`"),
        })?;
    let path = index_root.join(format!("{table_name}.lance"));
    let table = open_existing_native_table(&path, role).await.map_err(|e| {
        ConstellationError::IndexGenerationUnavailable {
            detail: format!("cannot open the {role} table at {}: {e}", path.display()),
        }
    })?;
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
    Ok(OpenedTable { table })
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
    workspace: &std::path::Path,
) -> Result<u64, ConstellationError> {
    restore_to_published_inner(table, role, published_version, workspace, None).await
}

async fn restore_to_published_inner(
    table: &lancedb::Table,
    role: &str,
    published_version: u64,
    workspace: &std::path::Path,
    repair: Option<&super::facade::RepairContext<'_>>,
) -> Result<u64, ConstellationError> {
    table.checkout(published_version).await.map_err(|e| {
        ConstellationError::IndexGenerationUnavailable {
            detail: format!(
                "cannot check out the published {role} version {published_version}: {e}"
            ),
        }
    })?;
    let dataset = table
        .dataset()
        .ok_or_else(|| ConstellationError::Io {
            detail: format!("the managed {role} table is not native"),
        })?
        .get()
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot read the published {role} dataset: {e}"),
        })?;
    let latest_version = dataset
        .latest_version_id()
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot read the latest {role} version before restore: {e}"),
        })?;
    let transaction = lance::dataset::transaction::Transaction::new(
        latest_version,
        lance::dataset::transaction::Operation::Restore {
            version: published_version,
        },
        None,
    );
    // Restore also restores the old cleanup configuration. That commit must
    // not collect the original pin before the retention policy is reapplied.
    {
        let _guard = super::facade::acquire_index_workspace_lock(workspace, repair).await?;
        lance::dataset::write::CommitBuilder::new(dataset)
            .with_skip_auto_cleanup(true)
            .execute(transaction)
            .await
            .map_err(|e| ConstellationError::Io {
                detail: format!("restore of the {role} table failed: {e}"),
            })?;
    }
    table
        .checkout_latest()
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("could not return the {role} writer to latest after restore: {e}"),
        })?;
    retain_managed_table_versions_inner(table, role, workspace, repair).await?;
    // The head created by the restore. Distinct from `published_version` by
    // construction, and recorded as itself.
    table.version().await.map_err(|e| ConstellationError::Io {
        detail: format!("cannot read the {role} version after restore: {e}"),
    })
}

/// Only owned writer paths call this; pinned and read-only opens do not.
/// Removing the interval disables automatic cleanup. Zero would instead run
/// cleanup on every commit and could remove versions a generation still pins.
async fn retain_managed_table_versions(
    table: &lancedb::Table,
    role: &str,
    workspace: &std::path::Path,
) -> Result<(), ConstellationError> {
    retain_managed_table_versions_inner(table, role, workspace, None).await
}

async fn retain_managed_table_versions_inner(
    table: &lancedb::Table,
    role: &str,
    workspace: &std::path::Path,
    repair: Option<&super::facade::RepairContext<'_>>,
) -> Result<(), ConstellationError> {
    let dataset = table
        .dataset()
        .ok_or_else(|| ConstellationError::Io {
            detail: format!("the managed {role} table is not native"),
        })?
        .get()
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot read retention policy for {role}: {e}"),
        })?;
    if !dataset
        .manifest()
        .config
        .contains_key(AUTO_CLEANUP_INTERVAL_KEY)
    {
        return Ok(());
    }
    let _guard = super::facade::acquire_index_workspace_lock(workspace, repair).await?;
    table
        .as_native()
        .ok_or_else(|| ConstellationError::Io {
            detail: format!("the managed {role} table is not native"),
        })?
        .delete_config_keys(&[AUTO_CLEANUP_INTERVAL_KEY])
        .await
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot preserve managed versions for {role}: {e}"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_marker() -> super::super::manifest::ProtocolMarker {
        super::super::manifest::ProtocolMarker {
            protocol_version: crate::constellation::FORMAT_VERSION,
            state: super::super::manifest::BootstrapState::Active,
            index_epoch: 1,
            checkout_id: "local-test".to_string(),
            table_names: super::super::manifest::ProtocolMarker::active_table_names(),
            storage_scheme: super::super::manifest::STORAGE_SCHEME_FILE_OBJECT_STORE.to_string(),
            bootstrap_attempt: 1,
        }
    }

    #[tokio::test]
    async fn read_open_never_recreates_deleted_root_and_keeps_exact_pin() {
        use arrow_array::{RecordBatch, StringArray};
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("lance");
        std::fs::create_dir(&root).unwrap();
        let connection = lancedb::connect(root.to_str().unwrap()).execute().await.unwrap();
        let schema = Arc::new(Schema::new(vec![Field::new("value", DataType::Utf8, false)]));
        let batch = RecordBatch::try_new(schema, vec![Arc::new(StringArray::from(vec!["before"]))]).unwrap();
        let table = connection.create_table(super::super::manifest::TABLE_ARTIFACTS_V1, batch)
            .execute().await.unwrap();
        let version = table.version().await.unwrap();
        table.update().column("value", "'after'").execute().await.unwrap();
        let pinned = open_table_at_version(&root, &test_marker(), "artifacts", version).await.unwrap();
        assert_eq!(pinned.version().await.unwrap(), version);
        let batches = pinned.query_batches().await.unwrap();
        let values = batches[0].column(0).as_any().downcast_ref::<StringArray>().unwrap();
        assert_eq!(values.value(0), "before");
        drop(pinned);
        drop(table);
        drop(connection);
        std::fs::remove_dir_all(&root).unwrap();
        assert!(matches!(
            open_table_at_version(&root, &test_marker(), "artifacts", version).await,
            Err(ConstellationError::IndexGenerationUnavailable { .. })
        ));
        assert!(matches!(
            open_active_tables_readonly(&root, &test_marker()).await,
            Err(ConstellationError::IndexGenerationUnavailable { .. })
        ));
        assert!(!root.exists());
    }

    fn field_names(schema: &Schema) -> Vec<&str> {
        schema.fields().iter().map(|f| f.name().as_str()).collect()
    }

    #[tokio::test]
    async fn managed_restore_retains_the_original_pinned_version_and_its_data() {
        use arrow_array::{RecordBatch, StringArray};
        use futures::TryStreamExt;
        use lancedb::query::ExecutableQuery;

        async fn values(table: &lancedb::Table) -> Vec<String> {
            let batches: Vec<RecordBatch> = table
                .query()
                .execute()
                .await
                .unwrap()
                .try_collect()
                .await
                .unwrap();
            batches
                .iter()
                .flat_map(|batch| {
                    batch
                        .column(0)
                        .as_any()
                        .downcast_ref::<StringArray>()
                        .unwrap()
                        .iter()
                        .map(|value| value.unwrap().to_string())
                })
                .collect()
        }

        let tmp = tempfile::TempDir::new().unwrap();
        let connection = lancedb::connect(tmp.path().to_str().unwrap())
            .execute()
            .await
            .unwrap();
        let schema = Arc::new(Schema::new(vec![Field::new(
            "value",
            DataType::Utf8,
            false,
        )]));
        let batch = RecordBatch::try_new(
            Arc::clone(&schema),
            vec![Arc::new(StringArray::from(vec!["before"]))],
        )
        .unwrap();
        let table = connection
            .create_table("retention", batch)
            .execute()
            .await
            .unwrap();
        table
            .as_native()
            .unwrap()
            .update_config([
                ("lance.auto_cleanup.interval".into(), "1".into()),
                ("lance.auto_cleanup.older_than".into(), "0ms".into()),
                ("lance.auto_cleanup.retain_versions".into(), "1".into()),
            ])
            .await
            .unwrap();
        let pinned = table.version().await.unwrap();
        let mut tags = table.tags().await.unwrap();
        // Protect only fixture construction. Production must preserve the
        // untagged pin through recovery and subsequent ordinary writes.
        tags.create("setup-pin", pinned).await.unwrap();
        table
            .update()
            .column("value", "'after'")
            .execute()
            .await
            .unwrap();
        tags.delete("setup-pin").await.unwrap();
        drop(tags);

        restore_to_published(&table, "artifacts", pinned, tmp.path())
            .await
            .unwrap();
        let restored_pin = table.checkout(pinned).await;
        assert!(
            restored_pin.is_ok(),
            "recovery must preserve the original published pin: {restored_pin:?}"
        );
        assert_eq!(values(&table).await, ["before"]);
        table.checkout_latest().await.unwrap();
        table
            .update()
            .column("value", "'after-recovery'")
            .execute()
            .await
            .unwrap();
        assert_eq!(values(&table).await, ["after-recovery"]);
        let retained_pin = table.checkout(pinned).await;
        assert!(
            retained_pin.is_ok(),
            "later writes must not garbage-collect a published pin: {retained_pin:?}"
        );
        assert_eq!(values(&table).await, ["before"]);
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
    fn schema_for_role_resolves_all_active_tables_and_refuses_anything_else() {
        let artifacts_role = schema_for_role("artifacts").unwrap();
        let artifacts_schema = artifacts_v1_schema();
        assert_eq!(field_names(&artifacts_role), field_names(&artifacts_schema));
        let relations_role = schema_for_role("relations").unwrap();
        let relations_schema = relations_v1_schema();
        assert_eq!(field_names(&relations_role), field_names(&relations_schema));
        let sections_role = schema_for_role("sections").unwrap();
        let sections_schema = sections_v1_schema();
        assert_eq!(field_names(&sections_role), field_names(&sections_schema));
        let err = schema_for_role("legacy").unwrap_err();
        assert_eq!(
            err.code(),
            super::super::DiagnosticCode::IndexProtocolMismatch
        );
    }

    #[test]
    fn the_sections_schema_carries_the_section_contract_and_the_binding() {
        let schema = sections_v1_schema();
        let names = field_names(&schema);
        // The section's own identity plus everything a hit needs to name its
        // parent — a section row must be self-describing because a semantic
        // read can return it without touching the artifacts table.
        for required in [
            "canonical_key",
            "parent_key",
            "parent_display_id",
            "ordinal",
            "byte_start",
            "byte_end",
            "heading",
            "level",
            "title",
            "content",
            "embedding",
            "source_digest",
            "input_digest",
            "profile_id",
            "incarnation",
            "row_revision",
            "index_epoch",
            "checkout_id",
            "semantic_state",
            "retry_after",
        ] {
            assert!(
                names.contains(&required),
                "the section contract requires `{required}`; schema has {names:?}"
            );
        }
        let key = schema.field_with_name("canonical_key").unwrap();
        assert_eq!(key.data_type(), &DataType::Utf8);
        assert!(!key.is_nullable(), "the merge key cannot be nullable");
        // Ordinal and level are the positional identity; nullable `level`
        // because the whole-body fallback section carries no heading level.
        assert!(
            !schema.field_with_name("ordinal").unwrap().is_nullable(),
            "ordinal is the positional identity and cannot be null"
        );
        assert!(
            schema.field_with_name("level").unwrap().is_nullable(),
            "the fallback section carries no heading level"
        );
        assert!(
            schema.field_with_name("heading").unwrap().is_nullable(),
            "the fallback and preamble sections carry no heading"
        );
        // The byte range is what makes the window checkable after publication,
        // so it is non-nullable: a section row with no range cannot be placed
        // in its parent's body at all.
        for name in ["byte_start", "byte_end"] {
            let field = schema
                .field_with_name(name)
                .unwrap_or_else(|e| panic!("missing {name}: {e}"));
            assert_eq!(
                field.data_type(),
                &DataType::UInt64,
                "{name} is a byte offset"
            );
            assert!(!field.is_nullable(), "{name} cannot be null");
        }
    }

    /// [`SECTION_COLUMNS`] and [`sections_v1_schema`] are two statements of
    /// the same contract, and the row builder is a third.
    ///
    /// They are written in three places because each is used in a different
    /// place, which is exactly why they can drift: adding a column to the
    /// schema and forgetting the constant leaves a table that is correct and a
    /// description of it that is not. Asserting the schema declares every name
    /// the constant claims makes the drift a test failure rather than a
    /// reviewer noticing.
    #[test]
    fn the_sections_column_constant_matches_the_schema_it_describes() {
        // The schema is bound to a local: `field_names` borrows it, and a
        // borrow of a temporary would end before the names it returned are
        // used.
        let schema = sections_v1_schema();
        let names = field_names(&schema);
        for column in SECTION_COLUMNS {
            assert!(
                names.contains(column),
                "`SECTION_COLUMNS` claims `{column}` but the schema does not declare it; the \
                 constant and the schema are one contract written twice"
            );
        }
        assert!(
            names.contains(&"byte_start") && names.contains(&"byte_end"),
            "the byte range is what makes the windows checkable against published data"
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

    // -------------------------------------------------------------------
    // Semantic readback decoding
    // -------------------------------------------------------------------

    /// A projection batch shaped exactly like the readback's: one row with a
    /// real vector, one row with a null vector.
    ///
    /// Built with `try_new` against an explicit schema rather than
    /// `try_from_iter`, so the column list is checked against the same
    /// nullability the table declares.
    fn semantic_batches() -> Vec<arrow_array::RecordBatch> {
        use arrow_array::builder::{
            FixedSizeListBuilder, Float32Builder, StringBuilder, UInt64Builder,
        };

        let mut keys = StringBuilder::new();
        keys.append_value("PRD-001");
        keys.append_value("PRD-002");

        let mut epochs = UInt64Builder::new();
        epochs.append_value(7);
        epochs.append_value(7);

        let mut checkouts = StringBuilder::new();
        checkouts.append_value("local-x");
        checkouts.append_value("local-x");

        let mut incarnations = UInt64Builder::new();
        incarnations.append_value(1);
        incarnations.append_value(2);

        let mut inputs = StringBuilder::new();
        inputs.append_value("in-1");
        inputs.append_value("in-2");

        let mut profiles = StringBuilder::new();
        profiles.append_value("p1");
        profiles.append_null();

        let mut states = StringBuilder::new();
        states.append_value("ready");
        states.append_value("unavailable");

        let mut retries = StringBuilder::new();
        retries.append_value("2026-10-20T00:00:00Z");
        retries.append_null();

        let dim = EMBEDDING_DIM as usize;
        let mut vectors = FixedSizeListBuilder::new(Float32Builder::new(), EMBEDDING_DIM);
        for value in 0..dim {
            vectors.values().append_value(value as f32);
        }
        vectors.append(true);
        vectors.values().append_nulls(dim);
        vectors.append(false);

        let embedding_type = DataType::FixedSizeList(
            Arc::new(Field::new("item", DataType::Float32, true)),
            EMBEDDING_DIM,
        );
        let schema = Arc::new(Schema::new(vec![
            Field::new("canonical_key", DataType::Utf8, false),
            Field::new("index_epoch", DataType::UInt64, false),
            Field::new("checkout_id", DataType::Utf8, false),
            Field::new("incarnation", DataType::UInt64, false),
            Field::new("input_digest", DataType::Utf8, true),
            Field::new("profile_id", DataType::Utf8, true),
            Field::new("semantic_state", DataType::Utf8, false),
            Field::new("retry_after", DataType::Utf8, true),
            Field::new("embedding", embedding_type, true),
        ]));
        let columns: Vec<arrow_array::ArrayRef> = vec![
            Arc::new(keys.finish()),
            Arc::new(epochs.finish()),
            Arc::new(checkouts.finish()),
            Arc::new(incarnations.finish()),
            Arc::new(inputs.finish()),
            Arc::new(profiles.finish()),
            Arc::new(states.finish()),
            Arc::new(retries.finish()),
            Arc::new(vectors.finish()),
        ];
        vec![
            arrow_array::RecordBatch::try_new(schema, columns)
                .expect("the projection batch is well formed"),
        ]
    }

    #[test]
    fn the_readback_decodes_identity_input_and_the_whole_semantic_tuple() {
        let map = IndexTables::semantic_map(&semantic_batches());
        assert_eq!(
            map.keys().cloned().collect::<Vec<_>>(),
            vec!["PRD-001".to_string(), "PRD-002".to_string()],
            "rows are keyed by canonical_key, in the order the table returned them"
        );

        let ready = &map["PRD-001"];
        assert_eq!(ready.index_epoch, Some(7));
        assert_eq!(ready.checkout_id.as_deref(), Some("local-x"));
        assert_eq!(ready.incarnation, Some(1));
        assert_eq!(ready.input_digest.as_deref(), Some("in-1"));
        assert_eq!(ready.profile_id.as_deref(), Some("p1"));
        assert_eq!(ready.semantic_state.as_deref(), Some("ready"));
        assert_eq!(ready.retry_after.as_deref(), Some("2026-10-20T00:00:00Z"));
        let vector = ready
            .embedding
            .as_ref()
            .expect("a non-null cell decodes to a vector");
        assert_eq!(vector.len(), EMBEDDING_DIM as usize);
        assert_eq!(vector[0], 0.0);
        assert_eq!(vector[1], 1.0);
        assert_eq!(
            vector[EMBEDDING_DIM as usize - 1],
            (EMBEDDING_DIM - 1) as f32
        );
    }

    #[test]
    fn a_null_vector_reads_back_as_absent_rather_than_a_zero_fill() {
        let map = IndexTables::semantic_map(&semantic_batches());
        let bare = &map["PRD-002"];
        assert_eq!(
            bare.embedding, None,
            "a zero vector here would be spliced back as a real one"
        );
        assert_eq!(bare.profile_id, None);
        assert_eq!(bare.retry_after, None);
        assert_eq!(bare.semantic_state.as_deref(), Some("unavailable"));
        // The identity is still read: this row is judged, then cleared — not
        // skipped.
        assert_eq!(bare.incarnation, Some(2));
    }

    #[test]
    fn a_row_with_no_key_is_skipped_rather_than_mapped() {
        // The map's key is the merge key. A row whose key cannot be read has
        // no name to be spliced under, and inventing one would attach a
        // vector to whatever row happened to sort next.
        let batch = arrow_array::RecordBatch::try_new(
            Arc::new(Schema::new(vec![Field::new(
                "canonical_key",
                DataType::Utf8,
                true,
            )])),
            vec![
                Arc::new(arrow_array::builder::StringBuilder::new().finish())
                    as arrow_array::ArrayRef,
            ],
        )
        .expect("a one-column null batch");
        assert!(IndexTables::semantic_map(&[batch]).is_empty());
    }

    #[test]
    fn the_readback_projects_no_rendered_column() {
        // The projection is what keeps a reconcile from loading the corpus
        // text a second time. Asserted against the constant so a future
        // "just add one column" edit fails here rather than in production.
        assert!(!SEMANTIC_READBACK_COLUMNS.contains(&"body"));
        assert!(!SEMANTIC_READBACK_COLUMNS.contains(&"content"));
        assert!(!SEMANTIC_READBACK_COLUMNS.contains(&"title"));
        assert!(!SEMANTIC_READBACK_COLUMNS.contains(&"tags"));

        // Every projected column must exist on **both** tables, or the query
        // fails at run time with a planning error instead of here.
        for name in SEMANTIC_READBACK_COLUMNS {
            for schema in [artifacts_v1_schema(), sections_v1_schema()] {
                assert!(
                    schema.column_with_name(name).is_some(),
                    "`{name}` is projected but `{schema:?}` does not declare it"
                );
            }
        }
    }
}
