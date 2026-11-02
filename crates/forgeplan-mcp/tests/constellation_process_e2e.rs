//! S1 full-MCP process contracts on a genuinely migrated local workspace.
//!
//! Intended path: crates/forgeplan-mcp/tests/constellation_process_e2e.rs.
//! These tests use Cargo's standalone forgeplan-mcp binary and rmcp's actual
//! initialize/call_tool framing. They do not launch the CLI or a read profile.
//! HOME, USERPROFILE and cache overrides are applied to the child command only.
//!
//! The in-process migrated suites cover mutation/lifecycle permutations. This
//! suite covers the missing OS-process boundary: launch defaults, request-level
//! routing, the retained maintenance host, restart, and observable normal exit.
//! A successful get is checked against canonical Markdown bytes; a host pass
//! must additionally move CURRENT to a checksummed manifest of the new bytes.
//!
//! Prerequisites: the existing forgeplan-mcp dev-dependencies (rmcp 1.7 client /
//! transport-child-process, forgeplan-core/test-helpers, tempfile, serde_json).
//! No new dependency, external CLI, cached model, global environment mutation,
//! or test-only production API is required. The binary is mandatory: no PATH
//! fallback or missing-binary skip. These are not NTFS/readonly-mount or real
//! semantic-positive tests, and restart here means startup reconciliation, not
//! a claim that pending inference completed without a model.
//!
//! # One layout on both sides
//!
//! The fixture publishes and re-reads the workspace in this process; the host
//! under observation is a separate process with an isolated, empty model cache.
//! Both must resolve the same layout, or the fixture publishes section rows
//! the child's capture cannot describe and the child's own publication is
//! refused for rows it never wrote. Every test therefore pins this process's
//! layout resolution to the child's cache directory with
//! `forgeplan_core::embed::tokenizer::set_test_layout_cache` — a test-only
//! seam that reads no environment variable and needs no production change.
//!
//! That override is process-global, so all four tests carry
//! `#[serial_test::serial(model_cache_layout)]`. The serial group is not
//! narrowed to the tests that install it: every test here publishes through
//! `maintain_index` and re-reads through `capture_source`, and both resolve a
//! layout, so a swap landing inside another test's capture would change its
//! layout identity mid-capture.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use forgeplan_core::artifact::frontmatter::parse_frontmatter;
use forgeplan_core::constellation::capture::{CaptureLimits, CaptureRequest, capture_source};
use forgeplan_core::constellation::facade::{
    MaintainCapability, maintain_index, workspace_is_migrated,
};
use forgeplan_core::constellation::manifest::CurrentDescriptor;
use forgeplan_core::constellation::storage::{read_current, read_manifest};
use rmcp::model::{CallToolRequestParams, RawContent};
use rmcp::service::{QuitReason, RunningService, RxJsonRpcMessage, TxJsonRpcMessage};
use rmcp::transport::{TokioChildProcess, Transport};
use rmcp::{RoleClient, ServiceExt};
use serde_json::{Value, json};
use tempfile::{NamedTempFile, TempDir};
use tokio::process::Command;
use tokio::sync::oneshot;
use tokio::time::{sleep, timeout};

const ARTIFACT_ID: &str = "PRD-901";
const ARTIFACT_PATH: &str = "prds/PRD-901-process-boundary.md";
const FIXTURE_TIMEOUT: Duration = Duration::from_secs(120);
const RPC_TIMEOUT: Duration = Duration::from_secs(30);
// The host's control scan is 30 s. This allows the scan plus publication I/O,
// rather than asserting a filesystem-specific notify latency.
const PUBLICATION_TIMEOUT: Duration = Duration::from_secs(45);
const CHILD_EXIT_TIMEOUT: Duration = Duration::from_secs(20);
const CLIENT_CLOSE_TIMEOUT: Duration = Duration::from_secs(30);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);

struct MigratedWorkspace {
    root: PathBuf,
    workspace: PathBuf,
    artifact: PathBuf,
}

fn prd(body: &str, outgoing: &str) -> String {
    format!(
        "---\nid: {ARTIFACT_ID}\nkind: prd\nstatus: draft\ntitle: Process boundary\ndepth: standard\n{outgoing}---\n\n{body}"
    )
}

fn assert_no_legacy_tables(workspace: &Path) {
    for name in ["artifacts.lance", "relations.lance", "change_log.lance"] {
        let path = workspace.join("lance").join(name);
        assert!(
            !path.try_exists().expect("inspect a legacy table path"),
            "a migrated process must not recreate the retired table {}",
            path.display()
        );
    }
}

fn current(workspace: &Path) -> CurrentDescriptor {
    read_current(&workspace.join("lance"))
        .expect("read a valid CURRENT descriptor")
        .expect("the migrated fixture has a published generation")
}

async fn migrated_workspace(root: &Path, body: &str) -> MigratedWorkspace {
    std::fs::create_dir_all(root).expect("create project root");
    let root = root.canonicalize().expect("canonical project root");
    let workspace = forgeplan_core::workspace::init_workspace(&root, "s1-process-e2e")
        .expect("initialize canonical workspace directories");
    let artifact = workspace.join(ARTIFACT_PATH);
    std::fs::write(&artifact, prd(body, "")).expect("write canonical fixture artifact");

    // This is the existing migrated fixture recipe. Never initialize or open
    // LanceStore: that would create the very legacy tables under observation.
    let capability = MaintainCapability::for_strict_maintenance(
        workspace
            .canonicalize()
            .expect("canonical workspace directory"),
    );
    timeout(
        FIXTURE_TIMEOUT,
        maintain_index(
            &capability,
            CaptureLimits::default(),
            std::time::Instant::now() + FIXTURE_TIMEOUT,
        ),
    )
    .await
    .expect("fixture bootstrap deadline")
    .expect("publish the actual Constellation index");
    assert!(workspace_is_migrated(&workspace));
    assert_no_legacy_tables(&workspace);
    let descriptor = current(&workspace);
    let manifest =
        read_manifest(&workspace.join("lance"), &descriptor).expect("fixture manifest checksum");
    assert_eq!(manifest.source_entries.len(), 1);
    assert_eq!(manifest.source_entries[0].canonical_key, ARTIFACT_ID);

    MigratedWorkspace {
        root,
        workspace,
        artifact,
    }
}

fn isolated_home(root: &Path) -> PathBuf {
    let home = root.join("child home");
    for relative in ["", "config", "data", "cache", "models"] {
        std::fs::create_dir_all(home.join(relative)).expect("create isolated child home/cache");
    }
    home.canonicalize().expect("canonical isolated child home")
}

/// Pin this process's layout resolution to the cache directory the child
/// process is configured with.
///
/// The fixture publishes and re-reads the workspace *in this process*, while
/// the host under observation is a separate process with its own isolated,
/// empty model cache. Both sides must resolve the same layout. A fixture
/// published under whatever this machine happens to have downloaded carries
/// section rows that the child's capture cannot describe, and the child's own
/// publication is then refused for rows it never wrote.
///
/// The override is process-global, so every test that installs it is
/// serialised. The directory is empty and the child runs offline, so both
/// sides resolve the same "semantic unavailable" layout - and its identity
/// names that directory, so both sides must name the same one.
struct LayoutOverride {
    previous: Option<PathBuf>,
}

impl LayoutOverride {
    fn install(cache_dir: PathBuf) -> Self {
        let previous = forgeplan_core::embed::tokenizer::set_test_layout_cache(Some(cache_dir));
        Self { previous }
    }
}

impl Drop for LayoutOverride {
    fn drop(&mut self) {
        forgeplan_core::embed::tokenizer::set_test_layout_cache(self.previous.take());
    }
}

fn canonical_prose(path: &Path) -> String {
    let raw = std::fs::read_to_string(path).expect("read canonical UTF-8 Markdown");
    parse_frontmatter(&raw)
        .expect("canonical document has valid YAML frontmatter")
        .1
}

fn atomic_save(path: &Path, contents: &str) {
    let mut replacement = NamedTempFile::new_in(path.parent().expect("artifact parent"))
        .expect("create same-directory editor save");
    replacement
        .write_all(contents.as_bytes())
        .expect("write complete replacement bytes");
    replacement.as_file().sync_all().expect("sync editor save");
    replacement
        .persist(path)
        .expect("atomically replace canonical Markdown");
}

/// Keep rmcp's wire transport, but observe a real exit status on close.
///
/// rmcp 1.7 TokioChildProcess::graceful_shutdown can kill after its internal
/// timeout and still return Ok(()). That is cleanup, not proof of orderly host
/// shutdown. into_inner drops the transport's pipes, delivering EOF, while this
/// adapter retains the child and requires a normal successful exit. Emergency
/// killing is bounded and always reported as a failed close, never as success.
struct ExitCheckedTransport {
    inner: Option<TokioChildProcess>,
    exited: Option<oneshot::Sender<Result<ExitStatus, String>>>,
}

impl Transport<RoleClient> for ExitCheckedTransport {
    type Error = io::Error;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleClient>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        self.inner
            .as_mut()
            .expect("transport remains open")
            .send(item)
    }

    fn receive(&mut self) -> impl Future<Output = Option<RxJsonRpcMessage<RoleClient>>> + Send {
        self.inner
            .as_mut()
            .expect("transport remains open")
            .receive()
    }

    fn close(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let transport = self.inner.take();
        let exited = self.exited.take();
        async move {
            let Some(transport) = transport else {
                return Ok(());
            };
            let outcome = match transport.into_inner() {
                Some(mut child) => {
                    let waited = timeout(CHILD_EXIT_TIMEOUT, child.wait()).await;
                    match waited {
                        Ok(Ok(status)) if status.success() => Ok(status),
                        Ok(Ok(status)) => {
                            Err(format!("forgeplan-mcp exited unsuccessfully: {status}"))
                        }
                        waited => {
                            let wait_detail = format!("{waited:?}");
                            let cleanup =
                                timeout(CLEANUP_TIMEOUT, Box::into_pin(child.kill())).await;
                            Err(format!(
                                "forgeplan-mcp did not exit normally after stdin EOF: \
                                 {wait_detail}; emergency kill/reap: {cleanup:?}"
                            ))
                        }
                    }
                }
                None => Err("rmcp lost ownership of the child before shutdown".to_string()),
            };
            let result = match &outcome {
                Ok(_) => Ok(()),
                Err(detail) => Err(io::Error::other(detail.clone())),
            };
            if let Some(exited) = exited {
                // A dropped receiver can only mean the test already failed.
                // The normal close path below requires this report to arrive.
                let _ = exited.send(outcome);
            }
            result
        }
    }
}

struct McpProcess {
    client: RunningService<RoleClient, ()>,
    exited: oneshot::Receiver<Result<ExitStatus, String>>,
    stderr: PathBuf,
}

fn stderr_text(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| format!("cannot read child stderr {}: {error}", path.display()))
}

impl McpProcess {
    async fn start(
        cwd: &Path,
        explicit_workspace: Option<&Path>,
        env_workspace: Option<&Path>,
        home: &Path,
        label: &str,
    ) -> Self {
        let binary = Path::new(env!("CARGO_BIN_EXE_forgeplan-mcp"));
        assert!(
            binary.is_file(),
            "required Cargo-built standalone MCP binary is missing: {}",
            binary.display()
        );
        let stderr = home.join(format!("{label}.stderr.log"));
        let log = std::fs::File::create(&stderr).expect("create child stderr log");
        let mut command = Command::new(binary);
        command
            .current_dir(cwd)
            .kill_on_drop(true)
            .env_remove("FORGEPLAN_WORKSPACE")
            .env_remove("FORGEPLAN_DISABLE_WORKTREE_DETECT")
            .env_remove("FORGEPLAN_EMBEDDING_MODEL")
            .env("HOME", home)
            .env("USERPROFILE", home)
            .env("XDG_CONFIG_HOME", home.join("config"))
            .env("XDG_DATA_HOME", home.join("data"))
            .env("XDG_CACHE_HOME", home.join("cache"))
            .env("FORGEPLAN_MODEL_CACHE", home.join("models"))
            .env("HF_HOME", home.join("cache/huggingface"))
            .env("HF_HUB_OFFLINE", "1")
            .env("TRANSFORMERS_OFFLINE", "1")
            .env("RUST_LOG", "warn");
        if let Some(workspace) = env_workspace {
            command.env("FORGEPLAN_WORKSPACE", workspace);
        }
        if let Some(workspace) = explicit_workspace {
            // Supported by the standalone entrypoint. No --profile flags are
            // passed: this process is the full MCP server, not the CLI.
            command.arg("--workspace").arg(workspace);
        }
        let (transport, _) = TokioChildProcess::builder(command)
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap_or_else(|error| {
                panic!(
                    "spawn {}: {error}; stderr={}",
                    binary.display(),
                    stderr_text(&stderr)
                )
            });
        let (exited_tx, exited) = oneshot::channel();
        let transport = ExitCheckedTransport {
            inner: Some(transport),
            exited: Some(exited_tx),
        };
        let initialized = match timeout(RPC_TIMEOUT, ().serve(transport)).await {
            Ok(initialized) => initialized,
            Err(error) => panic!(
                "initialize deadline: {error}; stderr={}",
                stderr_text(&stderr)
            ),
        };
        let client = initialized.unwrap_or_else(|error| {
            panic!(
                "MCP initialize failed: {error}; stderr={}",
                stderr_text(&stderr)
            )
        });
        Self {
            client,
            exited,
            stderr,
        }
    }

    async fn call(&self, name: &'static str, arguments: Value) -> Value {
        let arguments = arguments
            .as_object()
            .expect("object tool arguments")
            .clone();
        let result = timeout(
            RPC_TIMEOUT,
            self.client
                .peer()
                .call_tool(CallToolRequestParams::new(name).with_arguments(arguments)),
        )
        .await
        .unwrap_or_else(|error| {
            panic!(
                "{name} deadline: {error}; stderr={}",
                stderr_text(&self.stderr)
            )
        })
        .unwrap_or_else(|error| {
            panic!(
                "{name} RPC failed: {error}; stderr={}",
                stderr_text(&self.stderr)
            )
        });
        let texts: Vec<&str> = result
            .content
            .iter()
            .filter_map(|content| match &content.raw {
                RawContent::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect();
        assert_ne!(
            result.is_error,
            Some(true),
            "{name} returned a tool error: {texts:?}; stderr={}",
            stderr_text(&self.stderr)
        );
        assert_eq!(
            texts.len(),
            1,
            "{name} must return one JSON text DTO, got {result:?}; stderr={}",
            stderr_text(&self.stderr)
        );
        let payload: Value = serde_json::from_str(texts[0]).unwrap_or_else(|error| {
            panic!(
                "{name} returned invalid JSON: {error}; text={:?}; stderr={}",
                texts[0],
                stderr_text(&self.stderr)
            )
        });
        assert!(
            payload.is_object(),
            "{name} response must be a JSON object: {payload}"
        );
        payload
    }

    async fn assert_get_matches_file(&self, artifact: &Path, workspace: Option<&Path>) -> Value {
        let mut arguments = json!({ "id": ARTIFACT_ID });
        if let Some(workspace) = workspace {
            arguments["workspace"] = json!(workspace);
        }
        let payload = self.call("forgeplan_get", arguments).await;
        let raw =
            std::fs::read_to_string(artifact).expect("read canonical bytes after the response");
        assert_eq!(payload["id"], ARTIFACT_ID);
        assert_eq!(
            payload["body"].as_str(),
            Some(raw.as_str()),
            "the real MCP get must return the exact canonical document, not an older \
             projection; payload={payload}; stderr={}",
            stderr_text(&self.stderr)
        );
        payload
    }

    async fn close(self) {
        let Self {
            client,
            exited,
            stderr,
        } = self;
        let reason = timeout(CLIENT_CLOSE_TIMEOUT, client.cancel())
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "MCP client close deadline: {error}; stderr={}",
                    stderr_text(&stderr)
                )
            })
            .unwrap_or_else(|error| {
                panic!(
                    "MCP client task failed on close: {error}; stderr={}",
                    stderr_text(&stderr)
                )
            });
        assert!(
            matches!(reason, QuitReason::Cancelled),
            "the live client must close by cancellation, not an earlier transport failure: \
             {reason:?}; stderr={}",
            stderr_text(&stderr)
        );
        let status = timeout(CLEANUP_TIMEOUT, exited)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "no child exit report before deadline: {error}; stderr={}",
                    stderr_text(&stderr)
                )
            })
            .unwrap_or_else(|error| {
                panic!(
                    "child exit report was lost: {error}; stderr={}",
                    stderr_text(&stderr)
                )
            })
            .unwrap_or_else(|error| {
                panic!(
                    "child did not shut down orderly: {error}; stderr={}",
                    stderr_text(&stderr)
                )
            });
        assert_eq!(
            status.code(),
            Some(0),
            "child stderr={}",
            stderr_text(&stderr)
        );
    }
}

#[derive(Debug)]
struct ExpectedPublication {
    source_set_digest: String,
    config_digest: String,
    // Only source identity/digests are compared. A fresh read-only capture
    // deliberately does not invent the publisher's incarnation/job counters.
    sources: Vec<(String, String, String)>,
}

async fn expected_publication(workspace: &Path) -> ExpectedPublication {
    let captured = timeout(
        FIXTURE_TIMEOUT,
        capture_source(&CaptureRequest::new(workspace), CaptureLimits::default()),
    )
    .await
    .expect("read-only expected capture deadline")
    .expect("capture exact canonical bytes without publishing");
    assert!(captured.confirmed && captured.coverage.is_complete());
    let mut sources: Vec<_> = captured
        .entries
        .iter()
        .map(|entry| {
            (
                entry.canonical_key.clone(),
                entry.relative_path.clone(),
                entry.source_digest.clone(),
            )
        })
        .collect();
    sources.sort();
    ExpectedPublication {
        source_set_digest: captured.source_set_digest,
        config_digest: captured.config_digest,
        sources,
    }
}

async fn wait_for_publication(
    process: &McpProcess,
    workspace: &Path,
    expected: &ExpectedPublication,
) -> CurrentDescriptor {
    let index_root = workspace.join("lance");
    let mut last = None;
    let published = timeout(PUBLICATION_TIMEOUT, async {
        loop {
            let descriptor = read_current(&index_root).unwrap_or_else(|error| {
                panic!(
                    "host published an unreadable CURRENT: {error}; stderr={}",
                    stderr_text(&process.stderr)
                )
            });
            last = descriptor.clone();
            if let Some(descriptor) = descriptor
                && descriptor.source_set_digest == expected.source_set_digest
                && descriptor.config_digest == expected.config_digest
            {
                let manifest = read_manifest(&index_root, &descriptor).unwrap_or_else(|error| {
                    panic!(
                        "host publication has no valid checksummed manifest: {error}; stderr={}",
                        stderr_text(&process.stderr)
                    )
                });
                assert_eq!(manifest.generation_id, descriptor.generation_id);
                assert_eq!(manifest.source_set_digest, expected.source_set_digest);
                assert_eq!(manifest.config_digest, expected.config_digest);
                assert!(manifest.coverage.is_complete());
                let mut actual: Vec<_> = manifest
                    .source_entries
                    .into_iter()
                    .map(|entry| {
                        (
                            entry.canonical_key,
                            entry.relative_path,
                            entry.source_digest,
                        )
                    })
                    .collect();
                actual.sort();
                assert_eq!(
                    actual, expected.sources,
                    "manifest must describe the new canonical bytes"
                );
                assert_no_legacy_tables(workspace);
                return descriptor;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await;
    published.unwrap_or_else(|error| {
        panic!(
            "the retained host did not publish the expected canonical source set: \
             {error}; expected={expected:?}; last_CURRENT={last:?}; stderr={}",
            stderr_text(&process.stderr)
        )
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(model_cache_layout)]
async fn migrated_update_get_crosses_real_stdio_and_survives_orderly_process_restart() {
    let tmp = TempDir::new().expect("isolated process fixture");
    let home = isolated_home(tmp.path());
    let _layout = LayoutOverride::install(home.join("models"));
    let fixture = migrated_workspace(&tmp.path().join("project"), "Original process body.\n").await;
    let before = current(&fixture.workspace);
    let process = McpProcess::start(&fixture.root, None, None, &home, "update-first").await;
    process
        .assert_get_matches_file(&fixture.artifact, None)
        .await;
    let replacement =
        "## Summary\n\nS1 STDIO canonical marker: кириллица, \"quotes\", and \\slashes.\n";

    let updated = process
        .call(
            "forgeplan_update",
            json!({
                "id": ARTIFACT_ID,
                "body": replacement,
            }),
        )
        .await;
    assert_eq!(updated["id"], ARTIFACT_ID);
    assert_eq!(updated["status"], "draft");
    assert_eq!(
        updated["index_pending"], true,
        "an unacknowledged runtime handoff is still pending: {updated}"
    );
    assert_eq!(canonical_prose(&fixture.artifact), replacement);
    process
        .assert_get_matches_file(&fixture.artifact, None)
        .await;
    assert_no_legacy_tables(&fixture.workspace);

    // A fresh get alone could use an ephemeral capture while a dropped host
    // never publishes. CURRENT must independently converge to the new bytes.
    let expected = expected_publication(&fixture.workspace).await;
    assert_ne!(before.source_set_digest, expected.source_set_digest);
    let after = wait_for_publication(&process, &fixture.workspace, &expected).await;
    assert_ne!(before.generation_id, after.generation_id);
    process.close().await;
    assert_no_legacy_tables(&fixture.workspace);
    let canonical = std::fs::read(&fixture.artifact).expect("committed canonical bytes");

    let restarted = McpProcess::start(&fixture.root, None, None, &home, "update-restarted").await;
    restarted
        .assert_get_matches_file(&fixture.artifact, None)
        .await;
    wait_for_publication(&restarted, &fixture.workspace, &expected).await;
    restarted.close().await;
    assert_eq!(
        std::fs::read(&fixture.artifact).expect("canonical bytes after restart"),
        canonical
    );
    assert_no_legacy_tables(&fixture.workspace);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(model_cache_layout)]
async fn standalone_workspace_flag_routes_from_other_cwd_and_request_workspace_still_overrides() {
    let tmp = TempDir::new().expect("isolated routing fixture");
    let home = isolated_home(tmp.path());
    let _layout = LayoutOverride::install(home.join("models"));
    let x = migrated_workspace(&tmp.path().join("launch X"), "Only launch workspace X.\n").await;
    let y = migrated_workspace(
        &tmp.path().join("selected Y"),
        "Only selected workspace Y.\n",
    )
    .await;
    let x_before = std::fs::read(&x.artifact).expect("X canonical before startup");
    let x_generation = current(&x.workspace);
    let y_generation = current(&y.workspace);
    let process = McpProcess::start(&x.root, Some(&y.root), None, &home, "explicit-routing").await;

    process.assert_get_matches_file(&y.artifact, None).await;
    let y_body = "## Summary\n\nDefault requests commit to Y, not the launch cwd X.\n";
    let updated_y = process
        .call(
            "forgeplan_update",
            json!({
                "id": ARTIFACT_ID,
                "body": y_body,
            }),
        )
        .await;
    assert_eq!(updated_y["id"], ARTIFACT_ID);
    assert_eq!(canonical_prose(&y.artifact), y_body);
    process.assert_get_matches_file(&y.artifact, None).await;
    let expected_y = expected_publication(&y.workspace).await;
    let y_published = wait_for_publication(&process, &y.workspace, &expected_y).await;
    assert_ne!(y_published.generation_id, y_generation.generation_id);
    assert_eq!(
        std::fs::read(&x.artifact).expect("untouched X canonical"),
        x_before
    );
    assert_eq!(
        current(&x.workspace),
        x_generation,
        "the Y host must not publish into launch workspace X"
    );
    assert_no_legacy_tables(&x.workspace);
    assert_no_legacy_tables(&y.workspace);

    let y_before_override = std::fs::read(&y.artifact).expect("Y before explicit request");
    process
        .assert_get_matches_file(&x.artifact, Some(&x.root))
        .await;
    let x_body = "## Summary\n\nA request-level workspace explicitly selects X.\n";
    let updated_x = process
        .call(
            "forgeplan_update",
            json!({
                "id": ARTIFACT_ID,
                "body": x_body,
                "workspace": x.root,
            }),
        )
        .await;
    assert_eq!(updated_x["id"], ARTIFACT_ID);
    assert_eq!(canonical_prose(&x.artifact), x_body);
    process
        .assert_get_matches_file(&x.artifact, Some(&x.root))
        .await;
    let expected_x = expected_publication(&x.workspace).await;
    let x_published = wait_for_publication(&process, &x.workspace, &expected_x).await;
    assert_ne!(x_published.generation_id, x_generation.generation_id);
    assert_eq!(
        std::fs::read(&y.artifact).expect("Y after explicit request"),
        y_before_override
    );
    process.assert_get_matches_file(&y.artifact, None).await;
    process.close().await;
    assert_no_legacy_tables(&x.workspace);
    assert_no_legacy_tables(&y.workspace);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(model_cache_layout)]
async fn migrated_host_outlives_startup_publishes_editor_saves_and_reconciles_after_restart() {
    let tmp = TempDir::new().expect("isolated retained-host fixture");
    let home = isolated_home(tmp.path());
    let _layout = LayoutOverride::install(home.join("models"));
    let fixture =
        migrated_workspace(&tmp.path().join("project"), "Published before launch.\n").await;
    let original = current(&fixture.workspace);
    let notes = fixture.workspace.join("notes");
    std::fs::remove_file(notes.join(".gitkeep")).expect("remove fixture directory placeholder");
    std::fs::remove_dir(&notes).expect("the notes directory must not exist at server startup");

    // Startup must reconcile edits made while there was no server. Waiting on
    // CURRENT (not get) also establishes that the child host actually started.
    atomic_save(
        &fixture.artifact,
        &prd("Changed before server startup.\n", ""),
    );
    let startup_expected = expected_publication(&fixture.workspace).await;
    let process = McpProcess::start(&fixture.root, None, None, &home, "host-first").await;
    let startup = wait_for_publication(&process, &fixture.workspace, &startup_expected).await;
    assert_ne!(startup.generation_id, original.generation_id);

    // The host must discover an artifact directory created after startup,
    // not only edits to files in its initial watch set.
    let note = notes.join("NOTE-902-lagoon.md");
    std::fs::create_dir(&notes).expect("create a new artifact directory after startup");
    atomic_save(
        &note,
        "---\nid: NOTE-902\nkind: note\nstatus: draft\ntitle: Lagoon process observation\ndepth: tactical\n---\n\n## Observation\n\nlagoonuniquemarker is present only in the new note.\n",
    );
    atomic_save(
        &fixture.artifact,
        &prd(
            "## Summary\n\nEditor save after startup links the newly discovered observation.\n",
            "links:\n  - target: NOTE-902\n    relation: based_on\n",
        ),
    );
    let saved_expected = expected_publication(&fixture.workspace).await;
    assert_eq!(saved_expected.sources.len(), 2);
    let saved = wait_for_publication(&process, &fixture.workspace, &saved_expected).await;
    assert_ne!(saved.generation_id, startup.generation_id);
    let got = process
        .assert_get_matches_file(&fixture.artifact, None)
        .await;
    assert_eq!(
        got["links"]["outbound"],
        json!([
            { "target": "NOTE-902", "relation": "based_on" }
        ])
    );
    let note_got = process
        .call("forgeplan_get", json!({ "id": "NOTE-902" }))
        .await;
    let note_raw = std::fs::read_to_string(&note).expect("new note canonical bytes");
    assert_eq!(note_got["id"], "NOTE-902");
    assert_eq!(note_got["body"].as_str(), Some(note_raw.as_str()));

    // The child has its own empty model cache. A smart read must still return
    // the fresh lexical hit and state explicitly that semantics did not run.
    let searched = process
        .call(
            "forgeplan_search",
            json!({
                "query": "lagoonuniquemarker",
                "mode": "smart",
                "no_expand": true,
                "limit": 10,
            }),
        )
        .await;
    assert_eq!(searched["coverage"]["lexical"], true);
    assert_eq!(searched["coverage"]["semantic"], false);
    let hits = searched["results"].as_array().expect("search result array");
    assert_eq!(
        hits.len(),
        1,
        "only the new observation contains the lexical marker: {searched}"
    );
    assert_eq!(hits[0]["id"], "NOTE-902");
    assert_eq!(searched["total"], 1);
    process.close().await;
    assert_no_legacy_tables(&fixture.workspace);

    // With the OS process gone, a read-only capture can see offline edits but
    // cannot publish them. The NEXT server must perform startup reconciliation.
    let stopped = current(&fixture.workspace);
    atomic_save(
        &fixture.artifact,
        &prd("Saved while the server was stopped.\n", ""),
    );
    let offline_expected = expected_publication(&fixture.workspace).await;
    assert_ne!(
        offline_expected.source_set_digest,
        stopped.source_set_digest
    );
    assert_eq!(
        current(&fixture.workspace),
        stopped,
        "read-only capture must not secretly maintain an index"
    );
    let restarted = McpProcess::start(&fixture.root, None, None, &home, "host-restarted").await;
    let recovered = wait_for_publication(&restarted, &fixture.workspace, &offline_expected).await;
    assert_ne!(recovered.generation_id, stopped.generation_id);
    let got = restarted
        .assert_get_matches_file(&fixture.artifact, None)
        .await;
    assert_eq!(
        canonical_prose(&fixture.artifact),
        "Saved while the server was stopped.\n"
    );
    assert_eq!(
        got["links"]["outbound"],
        json!([]),
        "restart must not resurrect the old edge"
    );
    restarted.close().await;
    assert_no_legacy_tables(&fixture.workspace);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(model_cache_layout)]
async fn environment_workspace_routes_requests_and_maintenance_to_the_same_source() {
    let tmp = TempDir::new().expect("isolated environment-routing fixture");
    let home = isolated_home(tmp.path());
    let _layout = LayoutOverride::install(home.join("models"));
    let x_root = tmp.path().join("launch X");
    std::fs::create_dir_all(&x_root).unwrap();
    let x_root = x_root.canonicalize().unwrap();
    let x_workspace =
        forgeplan_core::workspace::init_workspace(&x_root, "legacy-launch-X").unwrap();
    let lock = forgeplan_core::workspace::acquire_workspace_lock(&x_workspace).await.unwrap();
    drop(
        forgeplan_core::db::store::LanceStore::init_new(&x_workspace, &lock)
            .await
            .unwrap(),
    );
    drop(lock);
    let x = MigratedWorkspace {
        artifact: x_workspace.join(ARTIFACT_PATH),
        root: x_root,
        workspace: x_workspace,
    };
    std::fs::write(&x.artifact, prd("Launch workspace X.\n", "")).unwrap();
    let change_log = x.workspace.join("lance/change_log.lance");
    assert!(change_log.exists());
    std::fs::remove_dir_all(&change_log).unwrap();
    let y = migrated_workspace(
        &tmp.path().join("environment Y"),
        "Environment workspace Y.\n",
    )
    .await;
    let x_bytes = std::fs::read(&x.artifact).expect("X canonical bytes");
    assert!(!workspace_is_migrated(&x.workspace));
    let process =
        McpProcess::start(&x.root, None, Some(&y.root), &home, "environment-routing").await;
    process.assert_get_matches_file(&y.artifact, None).await;

    atomic_save(
        &y.artifact,
        &prd("External save in the environment-selected workspace.\n", ""),
    );
    let expected = expected_publication(&y.workspace).await;
    wait_for_publication(&process, &y.workspace, &expected).await;
    process.assert_get_matches_file(&y.artifact, None).await;
    process.close().await;
    assert_eq!(
        std::fs::read(&x.artifact).expect("X remains untouched"),
        x_bytes
    );
    assert!(
        !workspace_is_migrated(&x.workspace),
        "startup must not migrate launch workspace X"
    );
    assert!(
        !change_log.exists(),
        "startup must not open or migrate the legacy store in X"
    );
    assert!(x.workspace.join("lance/artifacts.lance").exists());
    assert_no_legacy_tables(&y.workspace);
}
