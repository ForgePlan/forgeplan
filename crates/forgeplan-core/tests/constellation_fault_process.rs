//! Process-crash proof, not power-loss or native-library-hang proof.
//! Each publication stage is reached by a real three-table mutation. A fresh
//! child reads CURRENT and exact pinned tables, never an ephemeral fallback.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use arrow_array::{Array, LargeStringArray, StringArray, UInt64Array};
use forgeplan_core::constellation::ConstellationError;
use forgeplan_core::constellation::capture::CaptureLimits;
use forgeplan_core::constellation::facade::{MaintainCapability, maintain_index};
use forgeplan_core::constellation::lance_tables::open_table_at_version;
use forgeplan_core::constellation::manifest::ProtocolMarker;
use forgeplan_core::constellation::storage::{
    PROTOCOL_RELATIVE_PATH, pin_generation_read, read_current, read_manifest,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::{Child, Command};
use tokio::time::timeout;

const BOUND: Duration = Duration::from_secs(90);
static NEXT_BARRIER_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const CHILD_MODE: &str = "FORGEPLAN_PROOF_CHILD_MODE";
const CHILD_WORKSPACE: &str = "FORGEPLAN_PROOF_WORKSPACE";
const STAGES: [&str; 9] = [
    "after_artifacts_commit",
    "after_relations_commit",
    "after_sections_commit",
    "after_storage_sync",
    "before_manifest_sync",
    "after_manifest_sync",
    "before_current_replace",
    "after_current_replace",
    "after_current_ack",
];

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

#[test]
fn sha256_digest_bytes_have_canonical_lowercase_hex() {
    assert_eq!(
        digest_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        digest_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

fn admission_profile(
    captured: &forgeplan_core::constellation::capture::CapturedSourceView,
) -> forgeplan_core::constellation::jobs::EmbeddingProfile {
    // Deliberately an admission-only test space, not resolved model evidence.
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

fn computed(
    batch: &forgeplan_core::constellation::jobs::PreparedEmbeddingBatch,
    axis: usize,
) -> Vec<forgeplan_core::constellation::jobs::ComputedEmbedding> {
    batch
        .inputs
        .iter()
        .map(|input| {
            let mut vector = vec![0.0; forgeplan_core::db::schema::EMBEDDING_DIM as usize];
            vector[axis] = 1.0;
            forgeplan_core::constellation::jobs::ComputedEmbedding {
                job: input.job.clone(),
                vector,
            }
        })
        .collect()
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
    let workspace =
        forgeplan_core::workspace::init_workspace(root, "publication-proof").expect("workspace");
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

fn child_command(workspace: &Path, cache: &Path, mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    command
        .args([
            "--exact",
            "process_child",
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

async fn rows(
    workspace: &Path,
    marker: &ProtocolMarker,
    role: &str,
    version: u64,
) -> Result<Vec<Value>, ConstellationError> {
    let opened = open_table_at_version(&workspace.join("lance"), marker, role, version).await?;
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

async fn inspect(workspace: &Path) -> Result<Value, ConstellationError> {
    let index = workspace.join("lance");
    let descriptor = read_current(&index)?.expect("baseline CURRENT exists");
    let manifest = read_manifest(&index, &descriptor)?;
    manifest.validate_section_membership()?;
    descriptor.effective_artifacts_version(&manifest)?;
    descriptor.effective_relations_version(&manifest)?;
    descriptor.effective_sections_version(&manifest)?;
    let _pin = pin_generation_read(&index, &descriptor)?;
    let marker: ProtocolMarker = serde_json::from_slice(
        &std::fs::read(index.join(PROTOCOL_RELATIVE_PATH)).expect("marker bytes"),
    )
    .expect("protocol marker");
    // Read the exact triple, not effective/latest overlay heads and not canonical
    // records from open_workspace_read (which could legally be ephemeral).
    let versions = serde_json::to_value(manifest.table_versions).expect("versions");
    for role in ["artifacts", "relations", "sections"] {
        assert!(
            versions[role]
                .as_u64()
                .expect("all three versions are mandatory")
                > 0
        );
    }
    let mut tables = serde_json::Map::new();
    for role in ["artifacts", "relations", "sections"] {
        tables.insert(
            role.to_string(),
            json!(
                rows(
                    workspace,
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
    let expected_body = forgeplan_core::artifact::frontmatter::parse_frontmatter(&expected_raw)
        .expect("fixture body")
        .1;
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
    let memberships = observation["manifest"]["section_bindings"]
        .as_array()
        .expect("durable section bindings");
    let stored_sections = observation["tables"]["sections"]
        .as_array()
        .expect("exact pinned section set");
    assert_eq!(
        memberships.len(),
        stored_sections.len(),
        "no hidden stale/extra current section rows"
    );
    for membership in memberships {
        let stored = stored_sections
            .iter()
            .find(|row| row["canonical_key"] == membership["section_key"])
            .expect("every manifest section has exactly one stored row");
        assert_eq!(stored["input_digest"], membership["section_input_digest"]);
        assert_eq!(stored["parent_key"], membership["parent_key"]);
        assert_eq!(stored["ordinal"], membership["ordinal"]);
        let parent = artifacts
            .iter()
            .find(|row| row["canonical_key"] == membership["parent_key"])
            .expect("section parent");
        let parent_body = parent["body"].as_str().expect("parent body");
        let start = membership["byte_start"].as_u64().expect("byte start") as usize;
        let end = membership["byte_end"].as_u64().expect("byte end") as usize;
        assert_eq!(
            stored["content"],
            parent_body
                .get(start..end)
                .expect("valid contiguous UTF-8 byte range")
        );
        assert_eq!(membership["parent_input_digest"], parent["input_digest"]);
        assert_eq!(
            membership["layout_fingerprint"]
                .as_str()
                .expect("layout fingerprint")
                .len(),
            64
        );
    }
    for section in sections {
        for field in [
            "title",
            "source_digest",
            "incarnation",
            "row_revision",
            "index_epoch",
            "checkout_id",
        ] {
            assert_eq!(
                section[field], artifact[field],
                "section parent binding {field}"
            );
        }
        assert!(
            !section["input_digest"]
                .as_str()
                .expect("section input binding")
                .is_empty()
        );
    }
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
        println!("PROOF_STAGE {event}");
        reader.into_inner()
    }

    async fn release(&self, mut stream: TcpStream) {
        stream
            .write_all(
                format!(
                    "{}\n",
                    json!({"token":self.token,"stage":self.stage,"release":true})
                )
                .as_bytes(),
            )
            .await
            .expect("release ACK");
    }
}

async fn publish(workspace: &Path, cache: &Path) {
    let result = child_output(child_command(workspace, cache, "publish")).await;
    assert_eq!(result["result"], "published", "{result}");
}

#[tokio::test]
async fn process_child() {
    let Ok(mode) = std::env::var(CHILD_MODE) else {
        // The normally enumerated helper test is itself a genuine baseline
        // publication/read check, not an ignored or zero-assertion test case.
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
        "admit" => {
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
        "recover" => {
            use forgeplan_core::constellation::capture::{CaptureRequest, capture_source};
            let captured =
                capture_source(&CaptureRequest::new(&workspace), CaptureLimits::default())
                    .await
                    .expect("recovery capture");
            let descriptor = read_current(&workspace.join("lance"))
                .expect("CURRENT")
                .expect("published");
            let manifest = read_manifest(&workspace.join("lance"), &descriptor).expect("manifest");
            let (recovered, gaps) = forgeplan_core::constellation::runtime::recover_pending(
                &manifest,
                &captured,
                captured.body_prefix_chars,
            );
            assert!(
                gaps.is_empty(),
                "live durable work cannot disappear after process death"
            );
            assert!(
                !manifest.pending_jobs.is_empty(),
                "real durable pending required"
            );
            assert!(
                manifest.pending_jobs.len() <= captured.artifacts.len(),
                "durable queue is bounded by parents, not section/event count"
            );
            for key in manifest.pending_jobs.keys() {
                assert!(
                    forgeplan_core::constellation::jobs::section_parent_key(key).is_none(),
                    "pending map is bounded by parent artifacts"
                );
            }
            let mut inputs = Vec::new();
            for input in recovered {
                let digest = digest_hex(input.input_text.as_bytes());
                assert_eq!(
                    digest, input.job.input_digest,
                    "recovery uses exactly the admitted input bytes"
                );
                if let Some(binding) = manifest.section_binding(&input.job.canonical_key) {
                    assert_eq!(binding.section_input_digest, digest);
                }
                inputs.push(json!({"key":input.job.canonical_key,"input_digest":digest}));
            }
            assert!(!inputs.is_empty(), "zero recovered inputs is not recovery");
            json!({"result":"recovered", "pending":manifest.pending_jobs,"inputs":inputs})
        }
        "late-completions" => {
            use forgeplan_core::constellation::capture::{CaptureRequest, capture_source};
            use forgeplan_core::constellation::facade::{
                complete_embedding_batch, prepare_embedding_batch,
            };
            let capability = MaintainCapability::for_strict_maintenance(
                workspace.canonicalize().expect("canonical workspace"),
            );
            let captured =
                capture_source(&CaptureRequest::new(&workspace), CaptureLimits::default())
                    .await
                    .expect("capture");
            let profile = admission_profile(&captured);
            let a = prepare_embedding_batch(&capability, &profile, Instant::now() + BOUND)
                .await
                .expect("admit A");
            assert!(!a.inputs.is_empty());
            let a_results = computed(&a, 0);
            std::fs::write(workspace.join("prds/PRD-901-policy.md"), source("second"))
                .expect("source B");
            maintain_index(
                &capability,
                CaptureLimits::default(),
                Instant::now() + BOUND,
            )
            .await
            .expect("publish B source");
            let b = prepare_embedding_batch(&capability, &profile, Instant::now() + BOUND)
                .await
                .expect("admit B");
            let b_results = computed(&b, 1);
            std::fs::write(workspace.join("prds/PRD-901-policy.md"), source("third"))
                .expect("source C");
            maintain_index(
                &capability,
                CaptureLimits::default(),
                Instant::now() + BOUND,
            )
            .await
            .expect("publish C source");
            let c = prepare_embedding_batch(&capability, &profile, Instant::now() + BOUND)
                .await
                .expect("admit C");
            let c_results = computed(&c, 2);
            let c_outcome =
                complete_embedding_batch(&capability, &c, &c_results, Instant::now() + BOUND)
                    .await
                    .expect("complete C first");
            assert!(
                !c_outcome.published.is_empty(),
                "C really publishes vectors"
            );
            let before_late = inspect(&workspace).await.expect("pinned C");
            assert_coherent(&before_late, &["third"]);
            let a_outcome =
                complete_embedding_batch(&capability, &a, &a_results, Instant::now() + BOUND)
                    .await
                    .expect("classify late A");
            let b_outcome =
                complete_embedding_batch(&capability, &b, &b_results, Instant::now() + BOUND)
                    .await
                    .expect("classify late B");
            assert!(
                a_outcome.published.is_empty() && b_outcome.published.is_empty(),
                "late worker cannot publish"
            );
            assert!(
                !a_outcome.stale.is_empty() && !b_outcome.stale.is_empty(),
                "stale classification is observable"
            );
            let after_late = inspect(&workspace)
                .await
                .expect("pinned after late results");
            assert_eq!(
                before_late["tables"], after_late["tables"],
                "C->A->B must leave exact rows unchanged"
            );
            let old_incarnation = after_late["tables"]["artifacts"]
                .as_array()
                .expect("artifacts")
                .iter()
                .find(|row| row["canonical_key"] == "PRD-901")
                .expect("C row")["incarnation"]
                .as_u64()
                .expect("incarnation");
            std::fs::remove_file(workspace.join("prds/PRD-901-policy.md"))
                .expect("confirmed delete");
            maintain_index(
                &capability,
                CaptureLimits::default(),
                Instant::now() + BOUND,
            )
            .await
            .expect("publish deletion");
            std::fs::write(workspace.join("prds/PRD-901-policy.md"), source("third"))
                .expect("recreate identical bytes");
            maintain_index(
                &capability,
                CaptureLimits::default(),
                Instant::now() + BOUND,
            )
            .await
            .expect("publish new incarnation");
            let refused =
                complete_embedding_batch(&capability, &c, &c_results, Instant::now() + BOUND)
                    .await
                    .expect("classify old-incarnation completion");
            assert!(refused.published.is_empty() && !refused.stale.is_empty());
            let recreated = inspect(&workspace).await.expect("recreated pinned triple");
            let row = recreated["tables"]["artifacts"]
                .as_array()
                .expect("artifacts")
                .iter()
                .find(|row| row["canonical_key"] == "PRD-901")
                .expect("recreated row");
            assert!(row["incarnation"].as_u64().expect("incarnation") > old_incarnation);
            assert_ne!(
                row["semantic_state"], "ready",
                "old C vector must not be rebound to the recreated row"
            );
            json!({"result":"late_results_rejected", "before":before_late,"after":after_late,"recreated":recreated,
                   "boundary":"real publisher/conditional updates using controlled test vectors, not real-model encode"})
        }
        "prefilter" => {
            use forgeplan_core::constellation::capture::{CaptureRequest, capture_source};
            use forgeplan_core::constellation::facade::{
                PinnedFilter, complete_embedding_batch, open_workspace_read,
                prepare_embedding_batch, search_semantic_in_view,
            };
            let capability = MaintainCapability::for_strict_maintenance(
                workspace.canonicalize().expect("canonical workspace"),
            );
            let captured =
                capture_source(&CaptureRequest::new(&workspace), CaptureLimits::default())
                    .await
                    .expect("capture");
            let profile = admission_profile(&captured);
            let batch = prepare_embedding_batch(&capability, &profile, Instant::now() + BOUND)
                .await
                .expect("admit real stored rows");
            let mut vectors = computed(&batch, 0);
            for result in &mut vectors {
                let parent = forgeplan_core::constellation::jobs::section_parent_key(
                    &result.job.canonical_key,
                )
                .unwrap_or(&result.job.canonical_key);
                if parent == "NOTE-902" {
                    result.vector[0] = 0.0;
                    result.vector[1] = 1.0;
                }
            }
            let written =
                complete_embedding_batch(&capability, &batch, &vectors, Instant::now() + BOUND)
                    .await
                    .expect("publish controlled vectors");
            assert!(!written.published.is_empty());
            std::fs::write(workspace.join("prds/PRD-901-policy.md"), source("new"))
                .expect("invalidate best-scoring parent");
            let note = workspace.join("notes/NOTE-901-target.md");
            std::fs::write(
                &note,
                std::fs::read_to_string(&note).expect("note") + "Changed input.\n",
            )
            .expect("invalidate another early row");
            let read = open_workspace_read(&workspace, None, CaptureLimits::default())
                .await
                .expect("fresh ephemeral capture");
            assert!(
                !read.is_persisted(),
                "the query must bind changed source, not trust old generation"
            );
            let mut query = vec![0.0; forgeplan_core::db::schema::EMBEDDING_DIM as usize];
            query[0] = 1.0;
            let outcome = search_semantic_in_view(
                &workspace.join("lance"),
                read.view(),
                &query,
                &profile.profile_id(),
                1,
                &PinnedFilter::default(),
            )
            .await
            .expect("eligible prefilter");
            assert_eq!(outcome.hits.len(), 1);
            let parent = outcome.hits[0]
                .parent_key
                .as_deref()
                .unwrap_or(&outcome.hits[0].canonical_key);
            assert_eq!(
                parent, "NOTE-902",
                "stale top-K candidates must not hide the remaining eligible row"
            );
            assert!(outcome.eligible_rows > 0);
            let last = workspace.join("notes/NOTE-902-target.md");
            std::fs::write(
                &last,
                std::fs::read_to_string(&last).expect("last note") + "Changed input.\n",
            )
            .expect("make every input stale");
            let all_stale = open_workspace_read(&workspace, None, CaptureLimits::default())
                .await
                .expect("fresh complete source capture");
            let empty = search_semantic_in_view(
                &workspace.join("lance"),
                all_stale.view(),
                &query,
                &profile.profile_id(),
                1,
                &PinnedFilter::default(),
            )
            .await
            .expect("honest empty vector channel");
            assert!(empty.hits.is_empty());
            assert_eq!(empty.eligible_rows, 0);
            assert_eq!(
                all_stale.records().len(),
                3,
                "zero eligible vectors is not an empty source corpus"
            );
            json!({"result":"prefiltered", "eligible_hit":parent,"eligible_rows":outcome.eligible_rows,
                   "all_stale_eligible_rows":empty.eligible_rows,"expected_artifacts":all_stale.records().len(),
                   "semantic_complete":false,"boundary":"actual pinned vector search with controlled vectors, not real-model encode"})
        }
        "control-scan" => {
            use forgeplan_core::constellation::runtime::{
                FULL_SCAN_PERIOD_MS, MaintenanceSupervisor,
            };
            let capability = MaintainCapability::for_strict_maintenance(
                workspace.canonicalize().expect("canonical workspace"),
            );
            let supervisor = MaintenanceSupervisor::register(&workspace);
            let initial = supervisor
                .capture_pass(
                    &capability,
                    CaptureLimits::default(),
                    Instant::now() + BOUND,
                )
                .await
                .expect("initial control scan");
            assert!(initial.ran);
            let before = inspect(&workspace).await.expect("baseline pinned state");
            std::fs::write(workspace.join("prds/PRD-901-policy.md"), source("new"))
                .expect("unnotified edit");
            std::fs::write(
                workspace.join("config.yaml"),
                "embedding:\n  chunk_size: 37\n",
            )
            .expect("unnotified config change");
            assert!(
                !supervisor.is_dirty(),
                "no watcher or dirty signal is installed in this child"
            );
            let early = supervisor
                .capture_pass(
                    &capability,
                    CaptureLimits::default(),
                    Instant::now() + BOUND,
                )
                .await
                .expect("not yet due");
            assert!(!early.ran, "not fabricating an event to force a pass");
            tokio::time::sleep(Duration::from_millis(FULL_SCAN_PERIOD_MS + 25)).await;
            assert!(
                supervisor.full_scan_due(),
                "observe the actual timer condition, not a guessed publication stage"
            );
            let late = supervisor
                .capture_pass(
                    &capability,
                    CaptureLimits::default(),
                    Instant::now() + BOUND,
                )
                .await
                .expect("fallback full scan");
            assert!(late.ran);
            let after = inspect(&workspace).await.expect("pinned full-scan result");
            assert_coherent(&after, &["new"]);
            assert_ne!(
                before["manifest"]["config_digest"],
                after["manifest"]["config_digest"]
            );
            supervisor.unregister();
            json!({"result":"control_scan_recovered","before":before,"after":after,
                   "boundary":"all notifications deliberately absent; elapsed control timer after task suspension, not physical OS power/sleep proof"})
        }
        #[cfg(feature = "semantic-search")]
        "native-query" => {
            // This is intentionally stopped at the REAL CallGate entry before
            // any cache/model open. It needs no weights, network or fake model.
            let request = tokio::spawn(async {
                forgeplan_core::constellation::runtime::cached_query_embedding(
                    "bge-m3",
                    None,
                    forgeplan_core::constellation::jobs::DEFAULT_BODY_PREFIX_CHARS,
                    "deterministic helper entry proof",
                )
                .await
            });
            let mut eof = String::new();
            let read = timeout(
                BOUND,
                BufReader::new(tokio::io::stdin()).read_line(&mut eof),
            )
            .await
            .expect("EOF deadline")
            .expect("read EOF");
            assert_eq!(
                read, 0,
                "parent closes stdin only after actual native CallGate barrier"
            );
            request.abort();
            let cancelled = timeout(Duration::from_secs(5), request)
                .await
                .expect("request cancellation bound");
            assert!(
                cancelled.is_err_and(|error| error.is_cancelled()),
                "the dropped request must cancel its await, not claim native returned"
            );
            json!({"result":"native_helper_eof", "boundary":"real worker/CallGate blocked at entry; no ONNX hang or model encode claim"})
        }
        "workspace-lock" => {
            let guard = forgeplan_core::workspace::acquire_workspace_lock(&workspace)
                .await
                .expect("real WorkspaceLock");
            println!("WORKSPACE_LOCK_HELD");
            let mut release = String::new();
            timeout(
                BOUND,
                BufReader::new(tokio::io::stdin()).read_line(&mut release),
            )
            .await
            .expect("workspace release deadline")
            .expect("workspace release");
            assert_eq!(release.trim(), "release");
            drop(guard);
            json!({"result":"workspace_unlocked"})
        }
        _ => panic!("unknown child operation"),
    };
    println!("PROOF_RESULT {result}");
}

#[tokio::test]
async fn sigkill_at_each_publication_stage_reopens_one_pinned_triple() {
    #[cfg(not(unix))]
    panic!("SIGKILL process fault proof requires a Unix runner; not a Windows PASS");
    for stage in STAGES {
        let temp = tempfile::TempDir::new().expect("fixture root");
        let (workspace, cache) = fixture(temp.path());
        publish(&workspace, &cache).await;
        let old = child_output(child_command(&workspace, &cache, "inspect")).await;
        assert_coherent(&old, &["old"]);
        std::fs::write(workspace.join("prds/PRD-901-policy.md"), source("new"))
            .expect("real three-table mutation");
        let barrier = Barrier::new(stage).await;
        let mut command = child_command(&workspace, &cache, "publish");
        barrier.configure(&mut command);
        let mut child = command.spawn().expect("publisher child");
        let _blocked = barrier.reached(&child).await;
        child.start_kill().expect("SIGKILL publisher");
        let status = timeout(BOUND, child.wait())
            .await
            .expect("kill deadline")
            .expect("reap publisher");
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            assert_eq!(status.signal(), Some(9));
        }
        let observed = child_output(child_command(&workspace, &cache, "inspect")).await;
        assert_coherent(&observed, &["old", "new"]);
        if stage == "after_current_ack" {
            assert_eq!(observed["result"], "persisted");
            assert_coherent(&observed, &["new"]);
        }
        // This is a new authorized owner, not repair by the read observer.
        publish(&workspace, &cache).await;
        let recovered = child_output(child_command(&workspace, &cache, "inspect")).await;
        assert_coherent(&recovered, &["new"]);
        println!(
            "PROOF_OBSERVATION {}",
            json!({"stage":stage,"killed_status":status.to_string(),"before":old,"after_kill":observed,"after_recovery":recovered})
        );
    }
}

#[tokio::test]
async fn owner_death_allows_real_takeover_without_double_publisher() {
    let temp = tempfile::TempDir::new().expect("fixture root");
    let (workspace, cache) = fixture(temp.path());
    publish(&workspace, &cache).await;
    std::fs::write(workspace.join("prds/PRD-901-policy.md"), source("new")).expect("mutation");
    let first_barrier = Barrier::new("owner_acquired").await;
    let mut first_command = child_command(&workspace, &cache, "publish");
    first_barrier.configure(&mut first_command);
    let mut first = first_command.spawn().expect("first owner");
    let _held = first_barrier.reached(&first).await;
    let contended = child_output(child_command(&workspace, &cache, "publish")).await;
    assert_eq!(
        contended["code"], "index_busy",
        "second publisher must not steal live owner: {contended}"
    );
    first.start_kill().expect("kill real owner");
    timeout(BOUND, first.wait())
        .await
        .expect("owner death deadline")
        .expect("reap first owner");
    let takeover_barrier = Barrier::new("owner_acquired").await;
    let mut takeover_command = child_command(&workspace, &cache, "publish");
    takeover_barrier.configure(&mut takeover_command);
    let takeover = takeover_command.spawn().expect("takeover child");
    let stream = takeover_barrier.reached(&takeover).await;
    takeover_barrier.release(stream).await;
    let output = timeout(BOUND, takeover.wait_with_output())
        .await
        .expect("takeover completes")
        .expect("takeover output");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let observation = child_output(child_command(&workspace, &cache, "inspect")).await;
    assert_coherent(&observation, &["new"]);
    println!(
        "PROOF_OBSERVATION {}",
        json!({"case":"owner_takeover","contended":contended,"recovered":observation})
    );
}

#[tokio::test]
async fn workspace_lock_holder_can_release_while_publisher_waits() {
    let temp = tempfile::TempDir::new().expect("fixture root");
    let (workspace, cache) = fixture(temp.path());
    publish(&workspace, &cache).await;
    let mut locker_command = child_command(&workspace, &cache, "workspace-lock");
    locker_command.stdin(Stdio::piped());
    let mut locker = locker_command.spawn().expect("workspace lock caller");
    let mut stdout = BufReader::new(locker.stdout.take().expect("locker stdout"));
    let mut line = String::new();
    loop {
        line.clear();
        let read = timeout(BOUND, stdout.read_line(&mut line))
            .await
            .expect("lock handshake deadline")
            .expect("lock handshake");
        assert!(read > 0, "locker exited before actual lock acquisition");
        if line
            .split_once("WORKSPACE_LOCK_HELD")
            .is_some_and(|(_, rest)| rest.trim().is_empty())
        {
            break;
        }
    }
    std::fs::write(workspace.join("prds/PRD-901-policy.md"), source("new"))
        .expect("external editor mutation");
    let barrier = Barrier::new("owner_acquired").await;
    let mut command = child_command(&workspace, &cache, "publish");
    barrier.configure(&mut command);
    let publisher = command.spawn().expect("publisher");
    let entered = barrier.reached(&publisher).await;
    barrier.release(entered).await;
    locker
        .stdin
        .take()
        .expect("release channel")
        .write_all(b"release\n")
        .await
        .expect("caller releases before waiting for maintenance");
    assert!(
        timeout(BOUND, locker.wait())
            .await
            .expect("locker exit bound")
            .expect("locker status")
            .success()
    );
    let output = timeout(BOUND, publisher.wait_with_output())
        .await
        .expect("publisher does not deadlock")
        .expect("publisher output");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_coherent(
        &child_output(child_command(&workspace, &cache, "inspect")).await,
        &["new"],
    );
}

#[tokio::test]
async fn durable_pending_survives_sigkill_and_reconstructs_actual_inputs() {
    let temp = tempfile::TempDir::new().expect("fixture root");
    let (workspace, cache) = fixture(temp.path());
    publish(&workspace, &cache).await;
    let barrier = Barrier::new("after_current_ack").await;
    let mut command = child_command(&workspace, &cache, "admit");
    barrier.configure(&mut command);
    let mut child = command.spawn().expect("admission child");
    let _acknowledged = barrier.reached(&child).await;
    child.start_kill().expect("kill after durable admission");
    let status = timeout(BOUND, child.wait())
        .await
        .expect("death bound")
        .expect("reap child");
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(status.signal(), Some(9));
    }
    let recovered = child_output(child_command(&workspace, &cache, "recover")).await;
    assert_eq!(recovered["result"], "recovered");
    println!(
        "PROOF_OBSERVATION {}",
        json!({"case":"durable_pending_after_process_death","observation":recovered})
    );
}

#[tokio::test]
async fn metadata_absent_present_absent_publishes_exact_section_sets() {
    let temp = tempfile::TempDir::new().expect("fixture root");
    let (workspace, cache) = fixture(temp.path());
    let absent = temp.path().join("absent-cache");
    publish(&workspace, &absent).await;
    let first = child_output(child_command(&workspace, &absent, "inspect")).await;
    assert_eq!(first["tables"]["sections"], json!([]));
    assert_eq!(first["manifest"]["section_bindings"], json!([]));
    publish(&workspace, &cache).await;
    let present = child_output(child_command(&workspace, &cache, "inspect")).await;
    assert_coherent(&present, &["old"]);
    assert_ne!(
        first["manifest"]["config_digest"],
        present["manifest"]["config_digest"]
    );
    publish(&workspace, &absent).await;
    let last = child_output(child_command(&workspace, &absent, "inspect")).await;
    assert_eq!(
        last["tables"]["sections"],
        json!([]),
        "old cache vectors/rows cannot remain in the active empty layout"
    );
    assert_eq!(last["manifest"]["section_bindings"], json!([]));
    assert_ne!(
        present["manifest"]["config_digest"],
        last["manifest"]["config_digest"]
    );
    println!(
        "PROOF_OBSERVATION {}",
        json!({"case":"metadata_absent_present_absent","absent":first,"present":present,"absent_again":last})
    );
}

#[tokio::test]
async fn actual_c_a_b_completions_and_recreate_do_not_publish_old_results() {
    let temp = tempfile::TempDir::new().expect("fixture root");
    let (workspace, cache) = fixture(temp.path());
    publish(&workspace, &cache).await;
    let observed = child_output(child_command(&workspace, &cache, "late-completions")).await;
    assert_eq!(observed["result"], "late_results_rejected");
    println!("PROOF_OBSERVATION {observed}");
}

#[tokio::test]
async fn actual_pinned_vectors_prefilter_stale_rows_before_limit() {
    let temp = tempfile::TempDir::new().expect("fixture root");
    let (workspace, cache) = fixture(temp.path());
    publish(&workspace, &cache).await;
    let observed = child_output(child_command(&workspace, &cache, "prefilter")).await;
    assert_eq!(observed["eligible_hit"], "NOTE-902");
    assert_eq!(observed["all_stale_eligible_rows"], 0);
    println!("PROOF_OBSERVATION {observed}");
}

#[tokio::test]
async fn missed_notifications_and_suspended_control_timer_recover_bytes_and_config() {
    let temp = tempfile::TempDir::new().expect("fixture root");
    let (workspace, cache) = fixture(temp.path());
    publish(&workspace, &cache).await;
    let observed = child_output(child_command(&workspace, &cache, "control-scan")).await;
    assert_eq!(observed["result"], "control_scan_recovered");
    println!("PROOF_OBSERVATION {observed}");
}

#[cfg(feature = "semantic-search")]
#[tokio::test]
async fn native_callgate_entry_helper_exits_on_eof_without_claiming_native_return() {
    let temp = tempfile::TempDir::new().expect("fixture root");
    let (workspace, cache) = fixture(temp.path());
    publish(&workspace, &cache).await;
    let before = child_output(child_command(&workspace, &cache, "inspect")).await;
    let barrier = Barrier::new("native_call_entered").await;
    let mut command = child_command(&workspace, &cache, "native-query");
    command.stdin(Stdio::piped());
    barrier.configure(&mut command);
    let mut child = command.spawn().expect("real native-query child");
    let _blocked = barrier.reached(&child).await;
    let started = Instant::now();
    drop(child.stdin.take().expect("parent-held stdin"));
    let output = timeout(Duration::from_secs(5), child.wait_with_output())
        .await
        .expect("native helper process must exit before the 90-second barrier ACK timeout")
        .expect("native child output");
    assert!(
        output.status.success(),
        "native helper exit {:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 native output");
    let raw = stdout
        .lines()
        .find_map(|line| line.split_once("PROOF_RESULT ").map(|(_, value)| value))
        .expect("actual EOF result");
    let result: Value = serde_json::from_str(raw).expect("EOF result JSON");
    assert_eq!(result["result"], "native_helper_eof");
    let after = child_output(child_command(&workspace, &cache, "inspect")).await;
    assert_eq!(
        before, after,
        "query worker has no publication capability or late result write"
    );
    println!(
        "PROOF_OBSERVATION {}",
        json!({"case":"native_helper_eof","elapsed_ms":started.elapsed().as_millis(),
        "exit":output.status.code(),"result":result,"boundary":"helper stall only; production real encode/shutdown is a separate case"})
    );
}
