//! S14 fault-injection (SPEC-008 L09/F11, issue #538): process-crash proofs
//! for the three write/read seams the publication-stage harness
//! (`constellation_fault_process`) does not reach.
//!
//! 1. **Artifact Markdown write ↔ derived index** (`projection/artifact`).
//!    The atomic markdown write is the choke point every projection mutation
//!    goes through. A SIGKILL before the rename must leave the target
//!    byte-for-byte old with the whole new render staged in the temp file; a
//!    SIGKILL after the rename must leave the canonical file fully new and
//!    the derived index still old, with `sync_file_to_store` / maintenance
//!    completing the reconciliation (ADR-003 file-first) on restart.
//! 2. **Scope read** (`constellation`). A pinned generation handle opened
//!    before a writer is SIGKILLed at the `CURRENT` replacement must keep
//!    serving the complete old generation; a reader killed between a complete
//!    read and its return must leave nothing partial and no lock residue that
//!    blocks the next reader.
//! 3. **Embedding chunk build → pending-jobs publish** (`embed`/`facade`). A
//!    SIGKILL after chunk/row admission but before the pending manifest
//!    publish must leave `CURRENT` on the previous complete generation, and a
//!    fresh owner must restore the diverged head and republish.
//!
//! Every test is a real child process stopped at a real completed operation
//! via the fault barrier, SIGKILLed, then a fresh process asserts the
//! observable on-disk invariant: files and generations are fully old or fully
//! new, never torn; reads never observe partial state; recovery completes.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use arrow_array::{Array, LargeStringArray, StringArray, UInt64Array};
use forgeplan_core::artifact::frontmatter::parse_frontmatter;
use forgeplan_core::constellation::ConstellationError;
use forgeplan_core::constellation::capture::CaptureLimits;
use forgeplan_core::constellation::facade::{MaintainCapability, maintain_index};
use forgeplan_core::constellation::lance_tables::open_table_at_version;
use forgeplan_core::constellation::manifest::{CurrentDescriptor, ProtocolMarker};
use forgeplan_core::constellation::storage::{
    PROTOCOL_RELATIVE_PATH, pin_generation_read, read_current, read_manifest,
};
use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::projection::{self, MutationContext};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::{Child, Command};
use tokio::time::timeout;

const BOUND: Duration = Duration::from_secs(90);
static NEXT_BARRIER_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const CHILD_MODE: &str = "FORGEPLAN_S14_CHILD_MODE";
const CHILD_WORKSPACE: &str = "FORGEPLAN_S14_WORKSPACE";

/// Legacy (LanceDB) fixture artifact: file is the source of truth, the row is
/// the derived index (`update_body_with_projection` legacy-sink path).
const LEGACY_OLD_BODY: &str = "## Context\n\noriginal policy body with exact whitespace.\n";
const LEGACY_NEW_BODY: &str = "## Context\n\nreplacement policy body with exact whitespace.\n";

fn digest_hex(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for &byte in digest.iter() {
        encoded.push(char::from(ALPHABET[usize::from(byte >> 4)]));
        encoded.push(char::from(ALPHABET[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn admission_profile(
    captured: &forgeplan_core::constellation::capture::CapturedSourceView,
) -> forgeplan_core::constellation::jobs::EmbeddingProfile {
    forgeplan_core::constellation::jobs::EmbeddingProfile {
        model_repo: "admission-only-test-space".to_owned(),
        weights_revision: "not-a-resolved-model".to_owned(),
        weights_checksums: Vec::new(),
        tokenizer_checksum: captured.layout.tokenizer_checksum.clone(),
        tokenizer_config_checksum: captured.layout.tokenizer_config_checksum.clone(),
        dimensions: forgeplan_core::db::schema::EMBEDDING_DIM as u32,
        engine_version: "queue-proof-no-encode".to_owned(),
        pooling: "CLS".to_owned(),
        normalization: "L2".to_owned(),
        query_prefix: String::new(),
        document_prefix: String::new(),
        body_prefix_chars: captured.body_prefix_chars,
        max_tokens: captured.layout.max_tokens,
        padding: "none".to_owned(),
        truncation: "none".to_owned(),
        section_layout_fingerprint: captured.layout.fingerprint(),
    }
}

fn metadata_cache(root: &Path) -> PathBuf {
    let cache = root.join("metadata-cache");
    let repo = cache.join("models--BAAI--bge-m3");
    let snapshot = repo.join("snapshots/proof-tokenizer");
    std::fs::create_dir_all(&snapshot).expect("metadata snapshot");
    std::fs::create_dir_all(repo.join("refs")).expect("refs");
    std::fs::write(repo.join("refs/main"), "proof-tokenizer").expect("pinned metadata ref");
    let tokenizer = json!({
        "version":"1.0", "truncation":null, "padding":null,
        "added_tokens":[], "normalizer":null,
        "pre_tokenizer":{"type":"Whitespace"}, "post_processor":null,
        "decoder":null,
        "model":{"type":"WordLevel", "vocab":{"[UNK]":0,"old":1,"new":2}, "unk_token":"[UNK]"}
    });
    std::fs::write(snapshot.join("tokenizer.json"), tokenizer.to_string())
        .expect("valid tokenizer");
    std::fs::write(
        snapshot.join("tokenizer_config.json"),
        "{\"model_max_length\":64}",
    )
    .expect("token limit");
    std::fs::write(
        snapshot.join("config.json"),
        "{\"pad_token_id\":0,\"max_position_embeddings\":64}",
    )
    .expect("model metadata");
    cache
}

/// Constellation fixture: a published workspace plus the metadata that makes
/// a real sections commit happen (old ↔ new source swap, two link targets).
fn source(variant: &str) -> String {
    let target = if variant == "old" {
        "NOTE-901"
    } else {
        "NOTE-902"
    };
    format!(
        "---\nid: PRD-901\nkind: prd\nstatus: draft\ntitle: {variant} policy\ndepth: standard\nlinks:\n  - target: {target}\n    relation: based_on\n---\n\n## {variant} heading\n\n{variant} policy body with exact whitespace.  \n尾🙂\n"
    )
}

fn fixture(root: &Path) -> (PathBuf, PathBuf) {
    let workspace = forgeplan_core::workspace::init_workspace(root, "s14-publication-proof")
        .expect("workspace")
        .canonicalize()
        .expect("canonical workspace fixture");
    std::fs::write(workspace.join("prds/PRD-901-policy.md"), source("old")).expect("old canonical");
    for id in ["NOTE-901", "NOTE-902"] {
        std::fs::write(
            workspace.join(format!("notes/{id}-target.md")),
            format!("---\nid: {id}\nkind: note\nstatus: draft\ntitle: {id}\n---\n\nTarget.\n"),
        )
        .expect("target");
    }
    (workspace, metadata_cache(root))
}

/// Legacy fixture: a LanceDB-sink workspace with one seeded note.
async fn legacy_fixture(root: &Path) -> (PathBuf, PathBuf) {
    let workspace = forgeplan_core::workspace::init_workspace(root, "s14-legacy-proof")
        .expect("legacy workspace")
        .canonicalize()
        .expect("canonical legacy workspace");
    let lock = forgeplan_core::workspace::acquire_workspace_lock(&workspace)
        .await
        .expect("fixture lock");
    let store = LanceStore::init_new(&workspace, &lock)
        .await
        .expect("legacy fixture tables");
    let ctx = MutationContext::new(&workspace, &lock, Some(&store));
    projection::create_artifact_with_projection(
        &ctx,
        &NewArtifact {
            canonical_key: "NOTE-007".to_string(),
            id: "NOTE-007".to_string(),
            kind: "note".to_string(),
            title: "Original note".to_string(),
            body: LEGACY_OLD_BODY.to_string(),
            status: "draft".to_string(),
            depth: "tactical".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        },
    )
    .await
    .expect("seed legacy row");
    drop(store);
    drop(lock);
    // `create_artifact_with_projection` publishes a caller-supplied canonical
    // key under `canonical_filename(key)` = `{key}.md` (projection/mod.rs
    // 2340-2341), not under the title-derived `{id}-{slug}.md` name. The
    // title name is only composed when the caller supplies no key.
    let filename = forgeplan_core::artifact::store::canonical_filename("NOTE-007")
        .expect("canonical filename for the seed key");
    let note_path = workspace.join("notes").join(filename);
    assert!(note_path.exists(), "legacy canonical file written");
    (workspace, note_path)
}

fn child_command(workspace: &Path, cache: &Path, mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    command
        .args([
            "--exact",
            "fault_process_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_MODE, mode)
        .env(CHILD_WORKSPACE, workspace)
        .env("FORGEPLAN_MODEL_CACHE", cache)
        .env_remove("FORGEPLAN_WORKSPACE")
        .env_remove("FORGEPLAN_TEST_BARRIER_ENDPOINT")
        .env_remove("FORGEPLAN_TEST_BARRIER_TOKEN")
        .env_remove("FORGEPLAN_TEST_BARRIER_STAGE")
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

async fn child_output(mut command: Command) -> Value {
    let output = timeout(BOUND, command.output())
        .await
        .expect("child deadline")
        .expect("child output");
    assert!(
        output.status.success(),
        "child exit {:?}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 child output");
    let line = stdout
        .lines()
        .find_map(|line| line.split_once("PROOF_RESULT ").map(|(_, value)| value))
        .expect("child result record");
    serde_json::from_str(line).expect("JSON child result")
}

/// Exact rows of one pinned table at the version `CURRENT` names.
async fn rows(
    index_root: &Path,
    marker: &ProtocolMarker,
    role: &str,
    version: u64,
) -> Result<Vec<Value>, ConstellationError> {
    let opened = open_table_at_version(index_root, marker, role, version).await?;
    let columns: &[&str] = match role {
        "artifacts" => &[
            "canonical_key",
            "title",
            "body",
            "source_digest",
            "input_digest",
            "profile_id",
            "incarnation",
            "row_revision",
            "index_epoch",
            "checkout_id",
            "semantic_state",
        ],
        "relations" => &[
            "canonical_key",
            "target",
            "relation",
            "source_digest",
            "incarnation",
            "row_revision",
        ],
        "sections" => &[
            "canonical_key",
            "parent_key",
            "ordinal",
            "title",
            "heading",
            "content",
            "source_digest",
            "input_digest",
            "profile_id",
            "incarnation",
            "row_revision",
            "index_epoch",
            "checkout_id",
            "semantic_state",
        ],
        _ => panic!("unknown test role"),
    };
    assert_eq!(
        opened.version().await?,
        version,
        "reader checked out the exact CURRENT pin"
    );
    let batches = opened.select_columns(columns).await?;
    let mut result = Vec::new();
    for batch in batches {
        for row in 0..batch.num_rows() {
            let mut values = serde_json::Map::new();
            for name in columns {
                let column = batch.column_by_name(name).expect("selected column");
                let value = if column.is_null(row) {
                    Value::Null
                } else if let Some(array) = column.as_any().downcast_ref::<StringArray>() {
                    json!(array.value(row))
                } else if let Some(array) = column.as_any().downcast_ref::<LargeStringArray>() {
                    json!(array.value(row))
                } else if let Some(array) = column.as_any().downcast_ref::<UInt64Array>() {
                    json!(array.value(row))
                } else {
                    panic!(
                        "unexpected pinned column type for {name}: {:?}",
                        column.data_type()
                    )
                };
                values.insert((*name).to_owned(), value);
            }
            result.push(Value::Object(values));
        }
    }
    result.sort_by_key(Value::to_string);
    Ok(result)
}

/// Read the exact three pinned tables named by `descriptor`, without re-reading
/// `CURRENT`. This is the operation a scope handle performs: it must stay
/// bound to the generation it was opened on even after `CURRENT` moves.
async fn pinned_triple_at(
    index_root: &Path,
    descriptor: &CurrentDescriptor,
) -> Result<Value, ConstellationError> {
    let manifest = read_manifest(index_root, descriptor)?;
    manifest.validate_section_membership()?;
    descriptor.effective_artifacts_version(&manifest)?;
    descriptor.effective_relations_version(&manifest)?;
    descriptor.effective_sections_version(&manifest)?;
    let marker: ProtocolMarker = serde_json::from_slice(
        &std::fs::read(index_root.join(PROTOCOL_RELATIVE_PATH)).expect("marker bytes"),
    )
    .expect("protocol marker");
    let versions = serde_json::to_value(manifest.table_versions).expect("versions");
    for role in ["artifacts", "relations", "sections"] {
        assert!(versions[role].as_u64().expect("version") > 0);
    }
    let mut tables = serde_json::Map::new();
    for role in ["artifacts", "relations", "sections"] {
        tables.insert(
            role.to_string(),
            json!(
                rows(
                    index_root,
                    &marker,
                    role,
                    versions[role].as_u64().expect("version")
                )
                .await?
            ),
        );
    }
    Ok(json!({"result":"persisted", "descriptor":descriptor, "manifest":manifest, "tables":tables}))
}

/// Inspect through a freshly opened scope handle on the current generation.
async fn inspect(workspace: &Path) -> Result<Value, ConstellationError> {
    let index = workspace.join("lance");
    let descriptor = read_current(&index)?.expect("baseline CURRENT exists");
    let _pin = pin_generation_read(&index, &descriptor)?;
    pinned_triple_at(&index, &descriptor).await
}

fn assert_coherent(observation: &Value, allowed: &[&str]) {
    if observation["result"] == "unavailable" {
        assert!(
            matches!(
                observation["code"].as_str(),
                Some("index_generation_unavailable" | "index_protocol_mismatch")
            ),
            "not a typed publication failure: {observation}"
        );
        return;
    }
    assert_eq!(observation["result"], "persisted");
    let artifacts = observation["tables"]["artifacts"]
        .as_array()
        .expect("artifact rows");
    assert_eq!(artifacts.len(), 3, "no partial corpus after kill");
    let artifact = artifacts
        .iter()
        .find(|row| row["canonical_key"] == "PRD-901")
        .expect("policy row");
    let variant = allowed
        .iter()
        .find(|variant| artifact["title"] == format!("{variant} policy"))
        .expect("one whole old/new policy");
    let expected_raw = source(variant);
    let expected_body = parse_frontmatter(&expected_raw).expect("fixture body").1;
    assert_eq!(
        artifact["body"].as_str().expect("stored body"),
        expected_body,
        "exact pinned artifact body"
    );
    let expected_digest = digest_hex(expected_raw.as_bytes());
    assert_eq!(artifact["source_digest"], expected_digest);
    let entries = observation["manifest"]["source_entries"]
        .as_array()
        .expect("manifest entries");
    let entry = entries
        .iter()
        .find(|entry| entry["canonical_key"] == "PRD-901")
        .expect("manifest policy");
    for field in ["source_digest", "incarnation", "row_revision"] {
        assert_eq!(entry[field], artifact[field], "manifest/artifact {field}");
    }
    let relations: Vec<_> = observation["tables"]["relations"]
        .as_array()
        .expect("relations")
        .iter()
        .filter(|row| row["canonical_key"] == "PRD-901")
        .collect();
    assert_eq!(relations.len(), 1, "exact outgoing replacement");
    assert_eq!(
        relations[0]["target"],
        if *variant == "old" {
            "NOTE-901"
        } else {
            "NOTE-902"
        }
    );
    assert_eq!(relations[0]["relation"], "based_on");
    for field in ["source_digest", "incarnation", "row_revision"] {
        assert_eq!(
            relations[0][field], artifact[field],
            "relation binding {field}"
        );
    }
    let mut sections: Vec<_> = observation["tables"]["sections"]
        .as_array()
        .expect("sections")
        .iter()
        .filter(|row| row["parent_key"] == "PRD-901")
        .collect();
    assert!(
        !sections.is_empty(),
        "metadata fixture must force a real sections commit"
    );
    sections.sort_by_key(|row| row["ordinal"].as_u64().expect("ordinal"));
    let concatenated: String = sections
        .iter()
        .map(|row| row["content"].as_str().expect("section bytes"))
        .collect();
    assert_eq!(
        concatenated, expected_body,
        "exact full-body sections, never a mixed generation"
    );
}

struct Barrier {
    listener: TcpListener,
    token: String,
    stage: &'static str,
}

impl Barrier {
    async fn new(stage: &'static str) -> Self {
        Self {
            listener: TcpListener::bind("127.0.0.1:0")
                .await
                .expect("loopback listener"),
            token: format!(
                "{}-{}",
                std::process::id(),
                NEXT_BARRIER_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ),
            stage,
        }
    }

    fn configure(&self, command: &mut Command) {
        command
            .env(
                "FORGEPLAN_TEST_BARRIER_ENDPOINT",
                self.listener.local_addr().expect("endpoint").to_string(),
            )
            .env("FORGEPLAN_TEST_BARRIER_TOKEN", &self.token)
            .env("FORGEPLAN_TEST_BARRIER_STAGE", self.stage)
            .env("FORGEPLAN_TEST_BARRIER_ACK_MS", "90000");
    }

    async fn reached(&self, child: &Child) -> TcpStream {
        let (stream, address) = timeout(BOUND, self.listener.accept())
            .await
            .expect("mandatory stage not reached")
            .expect("barrier connection");
        assert!(address.ip().is_loopback());
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        timeout(Duration::from_secs(5), reader.read_line(&mut line))
            .await
            .expect("barrier frame deadline")
            .expect("barrier line");
        let event: Value = serde_json::from_str(&line).expect("barrier JSON");
        assert_eq!(event["token"], self.token);
        assert_eq!(event["stage"], self.stage);
        assert_eq!(event["pid"].as_u64(), child.id().map(u64::from));
        reader.into_inner()
    }
}

async fn publish(workspace: &Path, cache: &Path) {
    let result = child_output(child_command(workspace, cache, "publish")).await;
    assert_eq!(result["result"], "published", "{result}");
}

/// Assert the child was reaped by SIGKILL (signal 9 on Unix).
fn assert_sigkilled(status: &std::process::ExitStatus) {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(status.signal(), Some(9), "expected SIGKILL, got {status}");
    }
    #[cfg(not(unix))]
    {
        panic!("SIGKILL process fault proof requires a Unix runner; not a Windows PASS");
    }
}

#[tokio::test]
async fn fault_process_child() {
    let Ok(mode) = std::env::var(CHILD_MODE) else {
        // Enumerated directly, this helper is a genuine baseline publication/
        // read proof, not an ignored case.
        let temp = tempfile::TempDir::new().expect("fixture root");
        let (workspace, cache) = fixture(temp.path());
        publish(&workspace, &cache).await;
        assert_coherent(
            &child_output(child_command(&workspace, &cache, "inspect")).await,
            &["old"],
        );
        return;
    };
    let workspace = PathBuf::from(std::env::var_os(CHILD_WORKSPACE).expect("workspace env"));
    let result = match mode.as_str() {
        "publish" => {
            let capability = MaintainCapability::for_strict_maintenance(
                workspace.canonicalize().expect("canonical workspace"),
            );
            match maintain_index(
                &capability,
                CaptureLimits::default(),
                Instant::now() + BOUND,
            )
            .await
            {
                Ok(_) => json!({"result":"published"}),
                Err(error) => {
                    json!({"result":"error", "code":error.code().as_str(), "detail":error.to_string()})
                }
            }
        }
        "inspect" => match inspect(&workspace).await {
            Ok(result) => result,
            Err(error) => {
                json!({"result":"unavailable", "code":error.code().as_str(), "detail":error.to_string()})
            }
        },
        // Legacy file-first body update: canonical file first, then the row.
        "legacy-update" => {
            let lock = forgeplan_core::workspace::acquire_workspace_lock(&workspace)
                .await
                .expect("child lock");
            let store = LanceStore::open_for_write(&workspace, &lock)
                .await
                .expect("child store");
            let ctx = MutationContext::new(&workspace, &lock, Some(&store));
            let outcome =
                projection::update_body_with_projection(&ctx, "NOTE-007", LEGACY_NEW_BODY).await;
            drop(store);
            drop(lock);
            match outcome {
                Ok(()) => json!({"result":"updated"}),
                Err(error) => json!({"result":"error", "detail":format!("{error}")}),
            }
        }
        // Constellation cutover file-first body update (no LanceDB sink):
        // the markdown edit is the whole mutation; the index catches up on
        // the next reconcile.
        "no-store-update" => {
            let lock = forgeplan_core::workspace::acquire_workspace_lock(&workspace)
                .await
                .expect("child lock");
            let ctx = MutationContext::new(&workspace, &lock, None);
            let new_body = parse_frontmatter(&source("new")).expect("new body").1;
            let outcome = projection::update_body_with_projection(&ctx, "PRD-901", &new_body).await;
            drop(lock);
            match outcome {
                Ok(()) => json!({"result":"updated"}),
                Err(error) => json!({"result":"error", "detail":format!("{error}")}),
            }
        }
        // Embedding admission: capture, build chunks, admit rows; the
        // barrier fires inside the publish, before the manifest is swapped.
        "admit-chunks" => {
            use forgeplan_core::constellation::capture::{CaptureRequest, capture_source};
            let captured =
                capture_source(&CaptureRequest::new(&workspace), CaptureLimits::default())
                    .await
                    .expect("admission capture");
            let profile = admission_profile(&captured);
            let capability = MaintainCapability::for_strict_maintenance(
                workspace.canonicalize().expect("canonical workspace"),
            );
            let batch = forgeplan_core::constellation::facade::prepare_embedding_batch(
                &capability,
                &profile,
                Instant::now() + BOUND,
            )
            .await
            .expect("durable admission");
            json!({"result":"admitted", "input_count":batch.inputs.len()})
        }
        // Scope handle opened before a writer crash: pin the old generation,
        // signal, wait for release, then read through the SAME descriptor.
        // (The test fn returns (), so every failure is an assert on the spot.)
        "pinned-read" => {
            let index = workspace.join("lance");
            let descriptor = read_current(&index)
                .expect("read CURRENT")
                .expect("published CURRENT");
            let _pin = pin_generation_read(&index, &descriptor).expect("pin scope");
            println!("HANDLE_OPEN");
            let mut line = String::new();
            let read = timeout(
                BOUND,
                BufReader::new(tokio::io::stdin()).read_line(&mut line),
            )
            .await
            .expect("scope release deadline")
            .expect("scope release line");
            assert!(read > 0, "parent closed stdin before releasing the handle");
            assert_eq!(line.trim(), "release");
            let triple = pinned_triple_at(&index, &descriptor)
                .await
                .expect("pinned triple read");
            json!({"result":"pinned", "triple":triple})
        }
        // Reader completes its read, reports it, then holds — the parent kills
        // it here, between the read and the return to the caller.
        "read-then-hold" => {
            let index = workspace.join("lance");
            let descriptor = read_current(&index)
                .expect("read CURRENT")
                .expect("published CURRENT");
            let _pin = pin_generation_read(&index, &descriptor).expect("pin scope");
            let triple = pinned_triple_at(&index, &descriptor)
                .await
                .expect("pinned triple read");
            println!("READ_COMPLETE {triple}");
            let mut line = String::new();
            let read = timeout(
                BOUND,
                BufReader::new(tokio::io::stdin()).read_line(&mut line),
            )
            .await
            .expect("hold deadline")
            .expect("hold line");
            assert!(read > 0, "parent closed stdin while we held");
            json!({"result":"held"})
        }
        _ => panic!("unknown child operation {mode}"),
    };
    println!("PROOF_RESULT {result}");
}

/// Read the current legacy row body (or the file body) for the seed note.
async fn legacy_row_body(workspace: &Path) -> String {
    let permit = forgeplan_core::workspace::lock::acquire_workspace_read_lock(workspace)
        .await
        .expect("legacy read permit");
    let store = LanceStore::open(workspace).await.expect("legacy read");
    let row = store
        .get_record(&permit, "NOTE-007")
        .await
        .expect("row read")
        .expect("row exists");
    drop(store);
    drop(permit);
    row.body
}

// ── 1. Artifact Markdown write ↔ derived index ────────────────────────────

#[tokio::test]
async fn projection_before_rename_kill_leaves_old_target_and_complete_tmp() {
    #[cfg(not(unix))]
    panic!("SIGKILL process fault proof requires a Unix runner; not a Windows PASS");
    let temp = tempfile::TempDir::new().expect("fixture root");
    let (workspace, note_path) = legacy_fixture(temp.path()).await;
    let note_dir = note_path.parent().expect("notes dir").to_path_buf();
    let file_name = note_path
        .file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned();
    let old_bytes = std::fs::read(&note_path).expect("old canonical bytes");
    let (_, old_body) =
        parse_frontmatter(&String::from_utf8(old_bytes.clone()).expect("UTF-8 old file"))
            .expect("old file parses");
    assert_eq!(
        old_body, LEGACY_OLD_BODY,
        "seeded canonical body is the old body"
    );
    assert_eq!(
        legacy_row_body(&workspace).await,
        LEGACY_OLD_BODY,
        "seeded row is old"
    );

    let barrier = Barrier::new("before_projection_rename").await;
    let mut command = child_command(&workspace, temp.path(), "legacy-update");
    barrier.configure(&mut command);
    let mut child = command.spawn().expect("legacy writer child");
    let _blocked = barrier.reached(&child).await;
    let child_pid = child.id().expect("spawned child pid");
    child.start_kill().expect("SIGKILL writer");
    let status = timeout(BOUND, child.wait())
        .await
        .expect("kill deadline")
        .expect("reap writer");
    assert_sigkilled(&status);

    // The target is byte-for-byte the complete old file: the rename never ran.
    assert_eq!(
        std::fs::read(&note_path).expect("target after kill"),
        old_bytes,
        "the interrupted rename must leave the target completely old"
    );
    // The staged temp holds the ENTIRE new render, never a truncated write.
    let tmp_path = note_dir.join(format!(".{file_name}.tmp.{child_pid}"));
    let tmp_bytes = std::fs::read(&tmp_path).expect("staged temp exists after kill");
    let (_, tmp_body) =
        parse_frontmatter(&String::from_utf8(tmp_bytes.clone()).expect("UTF-8 temp"))
            .expect("staged temp is a complete render");
    assert_eq!(
        tmp_body, LEGACY_NEW_BODY,
        "the staged temp carries the complete new body; readers still see the old file"
    );
    // The derived row is untouched: no partial index update.
    assert_eq!(legacy_row_body(&workspace).await, LEGACY_OLD_BODY);

    // Recovery: a fresh process completes the mutation from scratch (fresh
    // temp name), and the stale temp residue never shadows the document.
    let recovered = child_output(child_command(&workspace, temp.path(), "legacy-update")).await;
    assert_eq!(recovered["result"], "updated", "{recovered}");
    let (_, after_body) =
        parse_frontmatter(&std::fs::read_to_string(&note_path).expect("recovered file"))
            .expect("recovered file parses");
    assert_eq!(
        after_body, LEGACY_NEW_BODY,
        "recovery writes the complete new body"
    );
    assert_eq!(
        legacy_row_body(&workspace).await,
        LEGACY_NEW_BODY,
        "recovery syncs the row"
    );
    let resolved =
        forgeplan_core::artifact::store::resolve_artifact_path(&workspace, "NOTE-007").await;
    assert_eq!(
        resolved,
        Some(note_path.clone()),
        "document resolution ignores the stale temp"
    );
    println!(
        "PROOF_OBSERVATION {}",
        json!({"case":"projection_before_rename","tmp_held_complete_new":true,"target_untouched":true,"recovered":true})
    );
}

#[tokio::test]
async fn projection_after_rename_kill_leaves_old_index_and_reconciles() {
    #[cfg(not(unix))]
    panic!("SIGKILL process fault proof requires a Unix runner; not a Windows PASS");
    let temp = tempfile::TempDir::new().expect("fixture root");
    let (workspace, note_path) = legacy_fixture(temp.path()).await;
    assert_eq!(legacy_row_body(&workspace).await, LEGACY_OLD_BODY);

    let barrier = Barrier::new("after_projection_rename").await;
    let mut command = child_command(&workspace, temp.path(), "legacy-update");
    barrier.configure(&mut command);
    let mut child = command.spawn().expect("legacy writer child");
    let _blocked = barrier.reached(&child).await;
    child.start_kill().expect("SIGKILL writer");
    let status = timeout(BOUND, child.wait())
        .await
        .expect("kill deadline")
        .expect("reap writer");
    assert_sigkilled(&status);

    // The canonical file is a complete, parseable NEW render (rename landed).
    let bytes = std::fs::read(&note_path).expect("file after kill");
    let (_, file_body) = parse_frontmatter(&String::from_utf8(bytes.clone()).expect("UTF-8 file"))
        .expect("post-crash file is a complete render");
    assert_eq!(
        file_body, LEGACY_NEW_BODY,
        "the canonical file is fully new"
    );
    // The derived index is still the complete OLD row: this is the ADR-003
    // crash window between "file written" and "index updated".
    assert_eq!(legacy_row_body(&workspace).await, LEGACY_OLD_BODY);

    // Recovery (ADR-003 file-first): sync_file_to_store reconciles the row
    // from the canonical file, which stays byte-identical through the sync.
    let guard = forgeplan_core::workspace::acquire_workspace_lock(&workspace)
        .await
        .expect("reconcile lock");
    let store = LanceStore::open_for_write(&workspace, &guard)
        .await
        .expect("reconcile store");
    let record = store
        .get_record(&guard, "NOTE-007")
        .await
        .expect("row read")
        .expect("row exists");
    assert_eq!(
        record.body, LEGACY_OLD_BODY,
        "index still old before reconcile"
    );
    let changed = projection::sync_file_to_store(&guard, &store, &workspace, &record)
        .await
        .expect("file-first reconcile");
    assert!(changed, "the body drift must be written");
    let after = store
        .get_record(&guard, "NOTE-007")
        .await
        .expect("row after")
        .expect("row exists");
    // `sync_file_to_store` normalises the row body to the canonical shape
    // (the renderer's single trailing newline is not part of the stored body),
    // while the canonical file keeps its exact bytes — asserted below.
    assert_eq!(
        after.body,
        LEGACY_NEW_BODY.trim_end(),
        "reconcile completes the index from the file (canonical body)"
    );
    assert_eq!(
        std::fs::read(&note_path).expect("reconciled file"),
        bytes,
        "reconcile never rewrites the canonical bytes"
    );
    drop(store);
    drop(guard);
    println!(
        "PROOF_OBSERVATION {}",
        json!({"case":"projection_after_rename","file_new":true,"index_old_then_reconciled":true})
    );
}

#[tokio::test]
async fn no_store_body_crash_reconciles_through_maintenance() {
    #[cfg(not(unix))]
    panic!("SIGKILL process fault proof requires a Unix runner; not a Windows PASS");
    let temp = tempfile::TempDir::new().expect("fixture root");
    let (workspace, cache) = fixture(temp.path());
    publish(&workspace, &cache).await;
    let before = child_output(child_command(&workspace, &cache, "inspect")).await;
    assert_coherent(&before, &["old"]);
    let file = workspace.join("prds/PRD-901-policy.md");
    let old_bytes = std::fs::read(&file).expect("old canonical bytes");
    let new_body = parse_frontmatter(&source("new")).expect("new body").1;

    let barrier = Barrier::new("after_projection_rename").await;
    let mut command = child_command(&workspace, &cache, "no-store-update");
    barrier.configure(&mut command);
    let mut child = command.spawn().expect("no-store writer child");
    let _blocked = barrier.reached(&child).await;
    child.start_kill().expect("SIGKILL writer");
    let status = timeout(BOUND, child.wait())
        .await
        .expect("kill deadline")
        .expect("reap writer");
    assert_sigkilled(&status);

    // The canonical file is a complete, parseable render with the new body:
    // the rename landed before the kill. The frontmatter title is untouched —
    // this is a body-only mutation (`update_body_file_first`), so the file
    // legitimately holds the old title with the full new body, both complete.
    let bytes = std::fs::read(&file).expect("file after kill");
    let (file_fm, file_body) =
        parse_frontmatter(&String::from_utf8(bytes.clone()).expect("UTF-8 file"))
            .expect("post-crash file is a complete render");
    assert_eq!(file_body, new_body, "the canonical file body is fully new");
    assert_eq!(
        file_fm.get("title").and_then(|v| v.as_str()),
        Some("old policy"),
        "a body-only mutation never rewrites the title"
    );
    assert_ne!(bytes, old_bytes, "the file really advanced to the new body");
    // The index still serves the complete OLD generation: nothing torn, and
    // the crash happened before any reconcile.
    let observed = child_output(child_command(&workspace, &cache, "inspect")).await;
    assert_coherent(&observed, &["old"]);

    // Recovery: maintenance reconciles the file into a generation that
    // mirrors the post-crash file exactly (ADR-003 file-first) — same body,
    // same preserved title, sections built from the exact file bytes.
    publish(&workspace, &cache).await;
    let recovered = child_output(child_command(&workspace, &cache, "inspect")).await;
    assert_eq!(recovered["result"], "persisted", "{recovered}");
    let row = recovered["tables"]["artifacts"]
        .as_array()
        .expect("artifact rows")
        .iter()
        .find(|row| row["canonical_key"] == "PRD-901")
        .expect("policy row");
    assert_eq!(
        row["body"].as_str().expect("stored body"),
        new_body,
        "recovery writes the file's new body into the index"
    );
    assert_eq!(
        row["title"].as_str().expect("stored title"),
        "old policy",
        "recovery keeps the title the canonical file actually declares"
    );
    let mut sections: Vec<_> = recovered["tables"]["sections"]
        .as_array()
        .expect("sections")
        .iter()
        .filter(|row| row["parent_key"] == "PRD-901")
        .collect();
    sections.sort_by_key(|row| row["ordinal"].as_u64().expect("ordinal"));
    let concatenated: String = sections
        .iter()
        .map(|row| row["content"].as_str().expect("section bytes"))
        .collect();
    assert_eq!(
        concatenated, new_body,
        "recovered sections are the exact full file body"
    );
    let entry = recovered["manifest"]["source_entries"]
        .as_array()
        .expect("manifest entries")
        .iter()
        .find(|entry| entry["canonical_key"] == "PRD-901")
        .expect("manifest policy");
    assert_eq!(
        entry["source_digest"],
        digest_hex(&bytes),
        "the recovered generation is built from the exact post-crash canonical bytes"
    );
    println!(
        "PROOF_OBSERVATION {}",
        json!({"case":"no_store_body_crash","file_new_index_old":true,"reconciled_via_maintenance":true})
    );
}

// ── 2. Scope read across a writer / reader SIGKILL ─────────────────────────

#[tokio::test]
async fn scope_pin_survives_writer_sigkill_and_serves_complete_old_generation() {
    #[cfg(not(unix))]
    panic!("SIGKILL process fault proof requires a Unix runner; not a Windows PASS");
    let temp = tempfile::TempDir::new().expect("fixture root");
    let (workspace, cache) = fixture(temp.path());
    publish(&workspace, &cache).await;
    let before = child_output(child_command(&workspace, &cache, "inspect")).await;
    assert_coherent(&before, &["old"]);
    std::fs::write(workspace.join("prds/PRD-901-policy.md"), source("new")).expect("mutation");

    // Reader opens the scope handle BEFORE the writer moves the index.
    let mut reader_command = child_command(&workspace, &cache, "pinned-read");
    reader_command.stdin(Stdio::piped());
    let mut reader = reader_command.spawn().expect("reader child");
    let mut reader_stdout = BufReader::new(reader.stdout.take().expect("reader stdout"));
    let mut line = String::new();
    loop {
        line.clear();
        let read = timeout(BOUND, reader_stdout.read_line(&mut line))
            .await
            .expect("handle deadline")
            .expect("handle line");
        assert!(read > 0, "reader exited before holding the scope handle");
        if line
            .split_once("HANDLE_OPEN")
            .is_some_and(|(_, rest)| rest.trim().is_empty())
        {
            break;
        }
    }

    // Writer publishes the new generation and is SIGKILLed at the visibility
    // point (CURRENT replaced, durable ACK not yet done).
    let barrier = Barrier::new("after_current_replace").await;
    let mut command = child_command(&workspace, &cache, "publish");
    barrier.configure(&mut command);
    let mut writer = command.spawn().expect("writer child");
    let _blocked = barrier.reached(&writer).await;
    writer.start_kill().expect("SIGKILL writer");
    let status = timeout(BOUND, writer.wait())
        .await
        .expect("kill deadline")
        .expect("reap writer");
    assert_sigkilled(&status);

    // Release the reader: its already-open handle must serve the complete old
    // generation — the immutable data it was opened on — never torn and never
    // secretly advanced to the new pointer.
    reader
        .stdin
        .take()
        .expect("release channel")
        .write_all(b"release\n")
        .await
        .expect("release");
    let mut result_line = String::new();
    loop {
        result_line.clear();
        let read = timeout(BOUND, reader_stdout.read_line(&mut result_line))
            .await
            .expect("reader result deadline")
            .expect("reader result line");
        assert!(
            read > 0,
            "reader exited without reporting its pinned result"
        );
        if result_line.split_once("PROOF_RESULT ").is_some() {
            break;
        }
    }
    let status = timeout(BOUND, reader.wait())
        .await
        .expect("reader bound")
        .expect("reader status");
    assert!(status.success(), "reader exit {:?}\n{result_line}", status);
    let raw = result_line
        .split_once("PROOF_RESULT ")
        .map(|(_, value)| value)
        .expect("reader result record");
    let result: Value = serde_json::from_str(raw).expect("reader JSON");
    assert_eq!(result["result"], "pinned");
    assert_eq!(
        result["triple"]["tables"], before["tables"],
        "the pinned scope served the exact complete old triple across the writer SIGKILL"
    );
    assert_eq!(
        result["triple"]["manifest"]["generation_id"], before["manifest"]["generation_id"],
        "the scope stayed bound to the immutable generation it was opened on"
    );

    // A fresh reader after the crash sees a complete old or new generation,
    // never a torn one.
    let observed = child_output(child_command(&workspace, &cache, "inspect")).await;
    assert_coherent(&observed, &["old", "new"]);
    // A new owner completes publication.
    publish(&workspace, &cache).await;
    let recovered = child_output(child_command(&workspace, &cache, "inspect")).await;
    assert_coherent(&recovered, &["new"]);
    println!(
        "PROOF_OBSERVATION {}",
        json!({"case":"scope_pin_across_writer_sigkill","pinned_old":true,"fresh":observed,"recovered":recovered})
    );
}

#[tokio::test]
async fn reader_killed_between_read_and_return_leaves_no_partial_state() {
    #[cfg(not(unix))]
    panic!("SIGKILL process fault proof requires a Unix runner; not a Windows PASS");
    let temp = tempfile::TempDir::new().expect("fixture root");
    let (workspace, cache) = fixture(temp.path());
    publish(&workspace, &cache).await;
    let before = child_output(child_command(&workspace, &cache, "inspect")).await;
    assert_coherent(&before, &["old"]);

    let mut reader_command = child_command(&workspace, &cache, "read-then-hold");
    reader_command.stdin(Stdio::piped());
    let mut reader = reader_command.spawn().expect("reader child");
    let mut stdout = BufReader::new(reader.stdout.take().expect("reader stdout"));
    // The child's stdout starts with the libtest preamble (`running 1 test`),
    // so scan lines until the READ_COMPLETE record, exactly like the
    // HANDLE_OPEN handshake in the scope test above.
    let mut line = String::new();
    loop {
        line.clear();
        let read = timeout(BOUND, stdout.read_line(&mut line))
            .await
            .expect("read deadline")
            .expect("read line");
        assert!(read > 0, "reader exited before completing its read");
        if line.split_once("READ_COMPLETE ").is_some() {
            break;
        }
    }
    let raw = line
        .split_once("READ_COMPLETE ")
        .map(|(_, value)| value.to_owned())
        .expect("READ_COMPLETE marker");
    // The read the reader performed was complete and identical to a fresh one.
    assert_eq!(
        serde_json::from_str::<Value>(raw.trim()).expect("reader JSON")["tables"],
        before["tables"]
    );

    // SIGKILL the reader between its completed read and the return to caller.
    reader.start_kill().expect("SIGKILL reader");
    let status = timeout(BOUND, reader.wait())
        .await
        .expect("kill bound")
        .expect("reap reader");
    assert_sigkilled(&status);

    // A fresh process observes byte-identical complete data: no partial state
    // survived and no reader lock residue blocks the next open.
    let after = child_output(child_command(&workspace, &cache, "inspect")).await;
    assert_eq!(
        after, before,
        "reader death between read and return leaves no torn or partial state"
    );
    assert_coherent(&after, &["old"]);
    println!(
        "PROOF_OBSERVATION {}",
        json!({"case":"reader_killed_between_read_and_return","identical_after_kill":true})
    );
}

// ── 3. Embedding chunk admission → pending-jobs publish ────────────────────

#[tokio::test]
async fn embed_crash_between_chunk_admission_and_pending_publish_keeps_old_and_completes() {
    #[cfg(not(unix))]
    panic!("SIGKILL process fault proof requires a Unix runner; not a Windows PASS");
    let temp = tempfile::TempDir::new().expect("fixture root");
    let (workspace, cache) = fixture(temp.path());
    publish(&workspace, &cache).await;
    let before = child_output(child_command(&workspace, &cache, "inspect")).await;
    assert_coherent(&before, &["old"]);

    // The child builds chunks, admits rows, and stops inside the pending-jobs
    // publish — before any generation names the admitted work.
    let barrier = Barrier::new("before_pending_publish").await;
    let mut command = child_command(&workspace, &cache, "admit-chunks");
    barrier.configure(&mut command);
    let mut child = command.spawn().expect("admission child");
    let _blocked = barrier.reached(&child).await;
    child.start_kill().expect("SIGKILL admission");
    let status = timeout(BOUND, child.wait())
        .await
        .expect("kill bound")
        .expect("reap admission");
    assert_sigkilled(&status);

    // Readers still see the exact complete old generation: the crashed
    // admission published nothing, and no torn/partial section is current.
    let observed = child_output(child_command(&workspace, &cache, "inspect")).await;
    assert_coherent(&observed, &["old"]);
    assert_eq!(
        observed["tables"], before["tables"],
        "no partial generation, no stray pending rows served as current"
    );

    // Recovery: a fresh owner restores the abandoned head and republishes the
    // full generation — the pending chunk work is rebuilt, not lost or split.
    publish(&workspace, &cache).await;
    let recovered = child_output(child_command(&workspace, &cache, "inspect")).await;
    assert_coherent(&recovered, &["old"]);
    assert_eq!(
        recovered["tables"]["sections"], before["tables"]["sections"],
        "recovery ends on the same exact complete section set"
    );
    println!(
        "PROOF_OBSERVATION {}",
        json!({"case":"embed_chunk_admission_crash","old_kept":true,"recovered_sections":true})
    );
}
