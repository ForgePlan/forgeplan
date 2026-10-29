//! History is auxiliary state on both workspace layouts, not an artifact index.

use std::path::Path;
use std::sync::Arc;

use arrow_array::{RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use forgeplan_core::changelog::ChangeLogEntry;
use forgeplan_core::db::auxiliary::AuxiliaryStore;
use forgeplan_core::db::schema;
use forgeplan_core::db::store::LanceStore;
use forgeplan_core::workspace::lock::acquire_workspace_lock;

fn entry(timestamp: &str, artifact_id: &str, action: &str, source: &str) -> ChangeLogEntry {
    ChangeLogEntry {
        timestamp: timestamp.into(),
        artifact_id: artifact_id.into(),
        action: action.into(),
        field: None,
        old_value: None,
        new_value: None,
        source: source.into(),
        commit_hash: None,
    }
}

fn rows(entries: &[ChangeLogEntry]) -> serde_json::Value {
    serde_json::to_value(entries).unwrap()
}

async fn connect(workspace: &Path) -> lancedb::Connection {
    let lance = workspace.join("lance");
    std::fs::create_dir_all(&lance).unwrap();
    lancedb::connect(lance.to_str().unwrap())
        .execute()
        .await
        .unwrap()
}

async fn table_names(workspace: &Path) -> Vec<String> {
    let db = connect(workspace).await;
    let mut names = db.table_names().execute().await.unwrap();
    names.sort();
    names
}

fn assert_no_other_history_tables(workspace: &Path) {
    for name in ["artifacts", "evidence", "relations", "fpf_spec"] {
        assert!(
            !workspace
                .join("lance")
                .join(format!("{name}.lance"))
                .exists(),
            "history access must not create or open {name}"
        );
    }
}

/// Seed the old on-disk format directly, without using either history writer.
async fn seed_old_rows(workspace: &Path, entries: &[ChangeLogEntry]) {
    let db = connect(workspace).await;
    let batch = RecordBatch::try_new(
        schema::change_log_schema(),
        vec![
            Arc::new(StringArray::from(
                entries
                    .iter()
                    .map(|e| e.timestamp.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                entries
                    .iter()
                    .map(|e| e.artifact_id.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                entries
                    .iter()
                    .map(|e| e.action.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                entries
                    .iter()
                    .map(|e| e.field.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                entries
                    .iter()
                    .map(|e| e.old_value.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                entries
                    .iter()
                    .map(|e| e.new_value.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                entries
                    .iter()
                    .map(|e| e.source.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                entries
                    .iter()
                    .map(|e| e.commit_hash.as_deref())
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .unwrap();
    db.create_table("change_log", vec![batch])
        .execute()
        .await
        .unwrap();
}

#[tokio::test]
async fn absent_history_read_is_empty_and_creates_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join(".forgeplan");
    let history = AuxiliaryStore::open_history_existing(&workspace)
        .await
        .unwrap();
    assert!(
        history
            .get_change_log(None, None, 100)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(!history.has_fpf());
    let error = history
        .log_change(&entry(
            "2026-01-01T00:00:00Z",
            "DELETED-001",
            "delete",
            "cli",
        ))
        .await
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("change_log table not initialized")
    );
    assert!(
        !workspace.exists(),
        "a read must not allocate even the workspace directory"
    );
}

#[tokio::test]
async fn existing_database_without_history_is_not_initialized_by_a_read() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join(".forgeplan");
    let lock = acquire_workspace_lock(&workspace).await.unwrap();
    let fpf = AuxiliaryStore::open_or_create(&workspace).await.unwrap();
    drop(fpf);
    drop(lock);
    let before = table_names(&workspace).await;
    assert_eq!(before, ["fpf_spec"]);

    let history = AuxiliaryStore::open_history_existing(&workspace)
        .await
        .unwrap();
    assert!(
        history
            .get_change_log(Some("missing"), Some("cli"), 1)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(!history.has_fpf(), "a history handle must not open FPF");
    drop(history);
    assert_eq!(table_names(&workspace).await, before);
    assert!(!workspace.join("lance/change_log.lance").exists());
}

#[tokio::test]
async fn history_writer_creates_only_change_log_under_the_callers_lock() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join(".forgeplan");
    let lock = acquire_workspace_lock(&workspace).await.unwrap();
    let history = AuxiliaryStore::open_history_or_create(&workspace)
        .await
        .unwrap();
    let expected = entry("2026-10-24T10:00:00Z", "DELETED-001", "delete", "reindex")
        .with_field("body")
        .with_values(Some(""), None)
        .with_commit("abc1234");
    history.log_change(&expected).await.unwrap();
    assert_eq!(
        rows(&history.get_change_log(None, None, 10).await.unwrap()),
        rows(std::slice::from_ref(&expected))
    );
    assert!(!history.has_fpf());
    drop(history);
    drop(lock);

    assert_eq!(table_names(&workspace).await, ["change_log"]);
    assert_no_other_history_tables(&workspace);
    let reopened = AuxiliaryStore::open_history_existing(&workspace)
        .await
        .unwrap();
    assert_eq!(
        rows(&reopened.get_change_log(None, None, 10).await.unwrap()),
        rows(&[expected])
    );
}

#[tokio::test]
async fn old_eight_column_rows_survive_append_filters_sort_limit_and_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join(".forgeplan");
    let id = "deleted' OR artifact_id != 'none";
    let source = "git'sync";
    let oldest = entry("2020-01-01T00:00:00Z", id, "create", source);
    let middle = entry("2021-01-01T00:00:00Z", id, "update", "cli")
        .with_field("title")
        .with_values(Some("old é title"), None);
    let newest = entry("2023-01-01T00:00:00Z", id, "update", source)
        .with_field("status")
        .with_values(Some(""), Some("active"))
        .with_commit("123abcd");
    seed_old_rows(
        &workspace,
        &[middle.clone(), newest.clone(), oldest.clone()],
    )
    .await;

    let lock = acquire_workspace_lock(&workspace).await.unwrap();
    let history = AuxiliaryStore::open_history_or_create(&workspace)
        .await
        .unwrap();
    let appended = entry("2022-01-01T00:00:00Z", "PRD-OTHER", "delete", source)
        .with_field("relation")
        .with_values(None, Some("PRD-001:informs"))
        .with_commit("");
    history.log_change(&appended).await.unwrap();
    assert_eq!(
        rows(&history.get_change_log(None, None, 2).await.unwrap()),
        rows(&[newest.clone(), appended.clone()]),
        "the limit applies after timestamp-desc sorting, not before it"
    );
    assert_eq!(
        rows(
            &history
                .get_change_log(Some(id), Some(source), 10)
                .await
                .unwrap()
        ),
        rows(&[newest.clone(), oldest.clone()]),
        "both apostrophe-containing filters are literal and AND-combined"
    );
    assert_eq!(
        rows(&history.get_change_log(Some(id), None, 10).await.unwrap()),
        rows(&[newest.clone(), middle.clone(), oldest.clone()])
    );
    assert_eq!(
        rows(
            &history
                .get_change_log(None, Some(source), 10)
                .await
                .unwrap()
        ),
        rows(&[newest.clone(), appended.clone(), oldest.clone()])
    );
    assert!(
        history
            .get_change_log(None, None, 0)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        history
            .get_change_log(Some("' OR '1' = '1"), None, 10)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        history
            .get_change_log(None, Some("' OR '1' = '1"), 10)
            .await
            .unwrap()
            .is_empty()
    );
    drop(history);
    drop(lock);

    let reopened = AuxiliaryStore::open_history_existing(&workspace)
        .await
        .unwrap();
    assert_eq!(
        rows(
            &reopened
                .get_change_log(None, None, usize::MAX)
                .await
                .unwrap()
        ),
        rows(&[newest, appended, middle, oldest])
    );
    assert_eq!(table_names(&workspace).await, ["change_log"]);
    assert_no_other_history_tables(&workspace);
}

#[tokio::test]
async fn legacy_and_auxiliary_handles_share_history_without_duplicate_events() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join(".forgeplan");
    let lock = acquire_workspace_lock(&workspace).await.unwrap();
    let legacy = LanceStore::init(&workspace).await.unwrap();
    let first = entry("2026-01-01T00:00:00Z", "PRD-001", "create", "cli");
    legacy.log_change(&first).await.unwrap();
    let baseline = table_names(&workspace).await;
    let history = AuxiliaryStore::open_history_or_create(&workspace)
        .await
        .unwrap();
    let second = entry("2026-01-02T00:00:00Z", "PRD-001", "update", "git_sync")
        .with_field("body")
        .with_values(None, Some("new hash"))
        .with_commit("abcdef0");
    history.log_change(&second).await.unwrap();
    drop(history);
    drop(legacy);
    drop(lock);

    let legacy = LanceStore::open(&workspace).await.unwrap();
    let history = AuxiliaryStore::open_history_existing(&workspace)
        .await
        .unwrap();
    for (id, source, limit) in [
        (None, None, 10),
        (Some("PRD-001"), None, 1),
        (None, Some("git_sync"), 10),
        (Some("PRD-001"), Some("cli"), 10),
        (Some("DELETED-001"), None, 10),
        (None, None, 0),
    ] {
        assert_eq!(
            rows(&legacy.get_change_log(id, source, limit).await.unwrap()),
            rows(&history.get_change_log(id, source, limit).await.unwrap())
        );
    }
    assert_eq!(
        rows(&history.get_change_log(None, None, 10).await.unwrap()),
        rows(&[second, first])
    );
    assert_eq!(table_names(&workspace).await, baseline);
}

#[tokio::test]
async fn corrupt_existing_history_is_an_error_and_is_never_replaced() {
    for as_directory in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join(".forgeplan");
        std::fs::create_dir_all(workspace.join("lance")).unwrap();
        let dataset = workspace.join("lance/change_log.lance");
        if as_directory {
            std::fs::create_dir(&dataset).unwrap();
            std::fs::write(dataset.join("sentinel"), b"not a Lance dataset").unwrap();
        } else {
            std::fs::write(&dataset, b"not a Lance dataset").unwrap();
        }
        assert!(
            AuxiliaryStore::open_history_existing(&workspace)
                .await
                .is_err()
        );
        let lock = acquire_workspace_lock(&workspace).await.unwrap();
        assert!(
            AuxiliaryStore::open_history_or_create(&workspace)
                .await
                .is_err()
        );
        drop(lock);
        let sentinel = if as_directory {
            dataset.join("sentinel")
        } else {
            dataset
        };
        assert_eq!(std::fs::read(sentinel).unwrap(), b"not a Lance dataset");
        assert_no_other_history_tables(&workspace);
    }
}

#[tokio::test]
async fn malformed_history_schema_is_not_reported_as_empty_or_migrated() {
    for wrong_type in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join(".forgeplan");
        let db = connect(&workspace).await;
        let schema = if wrong_type {
            let fields = schema::change_log_schema()
                .fields()
                .iter()
                .map(|field| {
                    if field.name() == "action" {
                        Field::new("action", DataType::Int32, false)
                    } else {
                        field.as_ref().clone()
                    }
                })
                .collect::<Vec<_>>();
            Arc::new(Schema::new(fields))
        } else {
            Arc::new(Schema::new(vec![Field::new(
                "unrelated",
                DataType::Utf8,
                true,
            )]))
        };
        let table = db
            .create_table("change_log", vec![RecordBatch::new_empty(schema.clone())])
            .execute()
            .await
            .unwrap();
        let read_error = AuxiliaryStore::open_history_existing(&workspace)
            .await
            .err()
            .unwrap();
        assert!(
            read_error
                .to_string()
                .contains("Incompatible change_log schema")
        );
        let lock = acquire_workspace_lock(&workspace).await.unwrap();
        let write_error = AuxiliaryStore::open_history_or_create(&workspace)
            .await
            .err()
            .unwrap();
        assert!(
            write_error
                .to_string()
                .contains("Incompatible change_log schema")
        );
        drop(lock);
        assert_eq!(table.schema().await.unwrap().fields(), schema.fields());
        assert_eq!(table_names(&workspace).await, ["change_log"]);
        assert_no_other_history_tables(&workspace);
        let legacy = LanceStore::init(&workspace).await.unwrap();
        let message = legacy
            .get_change_log(None, None, 10)
            .await
            .unwrap_err()
            .to_string();
        assert!(
            message.contains("Incompatible change_log schema"),
            "{message}"
        );
        assert_eq!(table.schema().await.unwrap().fields(), schema.fields());
        drop(legacy);
    }
}

#[tokio::test]
async fn unreadable_lance_path_is_not_treated_as_absent_history() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("not-a-directory");
    std::fs::write(&workspace, b"parent is a file").unwrap();
    assert!(
        AuxiliaryStore::open_history_existing(&workspace)
            .await
            .is_err()
    );
    assert!(
        AuxiliaryStore::open_history_or_create(&workspace)
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&workspace).unwrap(), b"parent is a file");
}

#[cfg(unix)]
#[tokio::test]
async fn unreadable_dataset_symlink_is_not_absence_or_recreated() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join(".forgeplan");
    std::fs::create_dir_all(workspace.join("lance")).unwrap();
    let dataset = workspace.join("lance/change_log.lance");
    std::os::unix::fs::symlink("change_log.lance", &dataset).unwrap();
    assert!(
        AuxiliaryStore::open_history_existing(&workspace)
            .await
            .is_err()
    );
    let lock = acquire_workspace_lock(&workspace).await.unwrap();
    assert!(
        AuxiliaryStore::open_history_or_create(&workspace)
            .await
            .is_err()
    );
    drop(lock);
    assert_eq!(
        std::fs::read_link(&dataset).unwrap(),
        Path::new("change_log.lance")
    );
    assert_no_other_history_tables(&workspace);
}

#[tokio::test]
async fn history_does_not_open_or_repair_a_corrupt_unrelated_knowledge_base() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join(".forgeplan");
    std::fs::create_dir_all(workspace.join("lance")).unwrap();
    let fpf_path = workspace.join("lance/fpf_spec.lance");
    std::fs::write(&fpf_path, b"broken knowledge base").unwrap();
    let history = AuxiliaryStore::open_history_existing(&workspace)
        .await
        .unwrap();
    assert!(
        history
            .get_change_log(None, None, 10)
            .await
            .unwrap()
            .is_empty()
    );
    drop(history);
    let lock = acquire_workspace_lock(&workspace).await.unwrap();
    let history = AuxiliaryStore::open_history_or_create(&workspace)
        .await
        .unwrap();
    let expected = entry("2026-10-24T00:00:00Z", "PRD-001", "create", "cli");
    history.log_change(&expected).await.unwrap();
    assert_eq!(
        rows(&history.get_change_log(None, None, 10).await.unwrap()),
        rows(&[expected])
    );
    drop(history);
    drop(lock);
    assert_eq!(std::fs::read(fpf_path).unwrap(), b"broken knowledge base");
    for retired in ["artifacts", "evidence", "relations"] {
        assert!(
            !workspace
                .join("lance")
                .join(format!("{retired}.lance"))
                .exists()
        );
    }
}

#[tokio::test]
async fn history_keeps_legacy_lexical_timestamp_order_without_normalizing_offsets() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join(".forgeplan");
    let lexical_first = entry("2026-10-24T13:00:00+03:00", "PRD-001", "update", "cli");
    let utc_later = entry("2026-10-24T11:00:00Z", "PRD-001", "update", "cli");
    seed_old_rows(&workspace, &[utc_later.clone(), lexical_first.clone()]).await;

    let history = AuxiliaryStore::open_history_existing(&workspace)
        .await
        .unwrap();
    assert_eq!(
        rows(&history.get_change_log(None, None, 1).await.unwrap()),
        rows(&[lexical_first])
    );
    assert_eq!(
        rows(&history.get_change_log(None, None, 10).await.unwrap())[1],
        serde_json::to_value(utc_later).unwrap()
    );
}

#[tokio::test]
async fn query_and_append_failures_are_not_reported_as_empty_success() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join(".forgeplan");
    let lock = acquire_workspace_lock(&workspace).await.unwrap();
    let history = AuxiliaryStore::open_history_or_create(&workspace)
        .await
        .unwrap();
    let change = entry("2026-10-24T00:00:00Z", "PRD-001", "create", "cli");
    history.log_change(&change).await.unwrap();

    let dataset = workspace.join("lance/change_log.lance");
    std::fs::remove_dir_all(&dataset).unwrap();
    std::fs::write(&dataset, b"unreadable dataset path").unwrap();
    assert!(history.get_change_log(None, None, 10).await.is_err());
    assert!(history.log_change(&change).await.is_err());
    assert_eq!(std::fs::read(dataset).unwrap(), b"unreadable dataset path");
    assert_no_other_history_tables(&workspace);
    drop(history);
    drop(lock);
}

#[tokio::test]
async fn nullable_required_history_schema_and_malformed_rows_are_errors_for_both_owners() {
    for required in ["timestamp", "artifact_id", "action", "source"] {
        for empty in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let workspace = temp.path().join(".forgeplan");
            let lock = acquire_workspace_lock(&workspace).await.unwrap();
            let db = connect(&workspace).await;
            let fields = schema::change_log_schema()
                .fields()
                .iter()
                .map(|field| {
                    Field::new(
                        field.name().as_str(),
                        field.data_type().clone(),
                        field.is_nullable() || field.name() == required,
                    )
                })
                .collect::<Vec<_>>();
            let columns = fields
                .iter()
                .map(|field| -> arrow_array::ArrayRef {
                    if empty {
                        return arrow_array::new_empty_array(field.data_type());
                    }
                    let value = match field.name().as_str() {
                        "timestamp" => Some("2026-10-24T00:00:00Z"),
                        "artifact_id" => Some("PRD-001"),
                        "action" => Some("update"),
                        "source" => Some("cli"),
                        _ => None,
                    };
                    let malformed = if field.name() == required {
                        None
                    } else {
                        value.or(Some(""))
                    };
                    Arc::new(StringArray::from(vec![value, malformed]))
                })
                .collect();
            let bad_schema = Arc::new(Schema::new(fields));
            let batch = RecordBatch::try_new(bad_schema.clone(), columns).unwrap();
            let table = db
                .create_table("change_log", vec![batch])
                .execute()
                .await
                .unwrap();
            for result in [
                AuxiliaryStore::open_history_existing(&workspace).await,
                AuxiliaryStore::open_history_or_create(&workspace).await,
            ] {
                let message = result
                    .err()
                    .expect("corrupt history must not be absent")
                    .to_string();
                assert!(
                    message.contains("Incompatible change_log schema"),
                    "{message}"
                );
                assert!(message.contains(required), "{message}");
                assert!(message.contains("non-nullable"), "{message}");
            }
            assert_no_other_history_tables(&workspace);
            assert_eq!(table.schema().await.unwrap().fields(), bad_schema.fields());

            // A real legacy owner reaches the same shared schema/query guard.
            // Its explicit initialization is fixture setup, not an auxiliary
            // history side effect.
            let legacy = LanceStore::init(&workspace).await.unwrap();
            for (id, source, limit) in [
                (None, None, 100),
                (Some("PRD-001"), Some("cli"), 1),
                (None, None, 0),
            ] {
                let message = legacy
                    .get_change_log(id, source, limit)
                    .await
                    .unwrap_err()
                    .to_string();
                assert!(
                    message.contains("Incompatible change_log schema"),
                    "{message}"
                );
                assert!(message.contains(required), "{message}");
            }
            assert_eq!(table.schema().await.unwrap().fields(), bad_schema.fields());
            drop(legacy);
            drop(table);
            drop(db);
            drop(lock);
        }
    }
}
