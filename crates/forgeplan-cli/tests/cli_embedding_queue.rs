//! SPEC-013 / RFC-021 Phase 2.3 — the pending-job embedding queue is durable.
//!
//! Admission (`prepare_embedding_batch`) writes `pending_jobs` into the
//! published generation manifest before any vector is computed, so a process
//! exit mid-queue loses nothing: the next drain re-reads the same jobs. A
//! drain either completes the jobs (queue empties) or, when the model cannot
//! be resolved, leaves them pending and says why — never a crash, never a
//! loss.
//!
//! The model is deliberately absent from every fixture here
//! (`FORGEPLAN_MODEL_CACHE` → empty, `HF_HUB_OFFLINE=1`): admission and
//! completion take a plain [`jobs::EmbeddingProfile`] value and never load
//! weights, which is what makes the durability contract exercisable offline.
//! Test-side queue work therefore goes through `forgeplan_core::constellation::
//! facade`'s public admission/completion API; the CLI binary is exercised
//! for the observable model-absent and bootstrap behaviour.

#[path = "common/historical_lance.rs"]
mod historical_lance;

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use assert_cmd::prelude::*;
use forgeplan_core::constellation::facade::{self, MaintainCapability};
use forgeplan_core::constellation::jobs::{
    self, ComputedEmbedding, EmbeddingProfile, PreparedEmbeddingBatch,
};
use forgeplan_core::constellation::lance_tables::EMBEDDING_DIM;
use forgeplan_core::constellation::storage::{self, IndexProtocolState};
use tempfile::TempDir;

const PRD_PATH: &str = "prds/PRD-001-queue.md";
const NOTE_PATH: &str = "notes/NOTE-001-queue.md";
const INITIAL_BODY: &str = "queue_initial_sentinel";
const EDITED_BODY: &str = "queue_edited_sentinel";

/// One shared EMPTY model cache for the whole test binary. The in-process
/// admission path (`facade::prepare_embedding_batch` → section-layout probe)
/// reads the ambient env, while the `forgeplan` subprocess reads the env we
/// pass it — both must see the same empty cache or their captures disagree
/// on `layout_fingerprint`, producing different `config_digest`s and a
/// `ProfileIncompatible` refusal at admission. A single `LazyLock` TempDir
/// means every test sets the same path: no last-set-wins race.
static EMPTY_MODEL_CACHE: LazyLock<TempDir> = LazyLock::new(|| TempDir::new().unwrap());

/// Point this process's ambient env at the shared empty cache. `set_var` is
/// unsafe in Rust 2024; safe here because each call writes the same value and
/// the values never change once set.
fn pin_process_env() {
    let cache = EMPTY_MODEL_CACHE.path().to_path_buf();
    // SAFETY: single-valued env writes; tests run with --test-threads=1 and
    // no thread reads these vars concurrently with the write.
    unsafe {
        std::env::set_var("FORGEPLAN_MODEL_CACHE", &cache);
        std::env::set_var("HF_HUB_OFFLINE", "1");
        std::env::set_var("FORGEPLAN_DISABLE_PLUGIN_DISCOVERY", "1");
    }
}

/// A workspace migrated onto the Constellation protocol: `CURRENT.json`
/// names a published generation and the legacy tables are retired.
/// Mirrors the `cli_lazy_sync.rs` fixture — the queue tests need the same
/// migrated, offline, model-less workspace shape.
struct Workspace {
    _temp: TempDir,
    root: PathBuf,
    home: PathBuf,
}

impl Workspace {
    fn migrated() -> Self {
        pin_process_env();
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("project");
        let home = temp.path().join("home");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&home).unwrap();
        let workspace = Self {
            _temp: temp,
            root,
            home,
        };
        historical_lance::init_legacy_workspace(&workspace.root, "Embedding queue fixture");
        workspace.write_prd(INITIAL_BODY);
        workspace.write_note("note initial sentinel");
        workspace.run_ok(&["index"]);
        // The first `index` may publish a generation whose config_digest
        // the prepare phase still disagrees with (config visible in the
        // capture vs what was committed). A second run settles it.
        workspace.run_ok(&["index"]);
        assert!(
            matches!(
                storage::read_marker(&workspace.lance()).unwrap(),
                IndexProtocolState::Active(_)
            ),
            "fixture did not migrate onto the constellation protocol"
        );
        assert!(storage::read_current(&workspace.lance()).unwrap().is_some());
        workspace
    }

    fn workspace(&self) -> PathBuf {
        self.root.join(".forgeplan")
    }

    fn lance(&self) -> PathBuf {
        self.workspace().join("lance")
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::cargo_bin("forgeplan").unwrap();
        command
            .args(args)
            .current_dir(&self.root)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("XDG_DATA_HOME", &self.home)
            .env("FORGEPLAN_MODEL_CACHE", EMPTY_MODEL_CACHE.path())
            .env("HF_HUB_OFFLINE", "1")
            .env("FORGEPLAN_DISABLE_PLUGIN_DISCOVERY", "1")
            .stdin(Stdio::null());
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        assert_cmd::Command::from_std(self.command(args))
            .timeout(Duration::from_secs(120))
            .output()
            .unwrap()
    }

    fn run_ok(&self, args: &[&str]) -> Output {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "forgeplan {args:?} failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn write_prd(&self, body: &str) {
        let source = format!(
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Queue prd\n\
             slug: prd-queue\ndepth: standard\n---\n\n## Summary\n\n{body}\n"
        );
        let path = self.workspace().join(PRD_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }

    fn write_note(&self, body: &str) {
        let source = format!(
            "---\nid: NOTE-001\nkind: note\nstatus: draft\ntitle: Queue note\n\
             slug: note-queue\ndepth: standard\n---\n\n## Summary\n\n{body}\n"
        );
        let path = self.workspace().join(NOTE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }

    /// Enable semantic embedding in `config.yaml` so post-mutation drain
    /// attempts try — and report — the model path.
    fn enable_embedding_config(&self) {
        let path = self.workspace().join("config.yaml");
        let existing = fs::read_to_string(&path).unwrap_or_default();
        fs::write(
            path,
            format!("{existing}\nembedding:\n  model: bge-m3\n  chunk_size: 2000\n"),
        )
        .unwrap();
        // The manifest pins the config digest of its capture; a config change
        // is a capture-visible change, so reconcile before draining.
        self.run_ok(&["index"]);
    }

    fn capability(&self) -> MaintainCapability {
        MaintainCapability::for_strict_maintenance(self.workspace().canonicalize().unwrap())
    }

    /// The durable pending set in the published manifest.
    fn pending_keys(&self) -> Vec<String> {
        let descriptor = storage::read_current(&self.lance())
            .unwrap()
            .expect("no published generation");
        let manifest = storage::read_manifest(&self.lance(), &descriptor).unwrap();
        manifest
            .pending_jobs
            .values()
            .flat_map(|group| group.inputs.keys().cloned())
            .collect()
    }

    /// Admit the whole eligible corpus as durable pending jobs via the public
    /// admission API — the same call a drain's prepare phase makes, minus the
    /// model worker. Returns the prepared batch for completion/failure calls.
    fn admit_pending(&self) -> PreparedEmbeddingBatch {
        let capability = self.capability();
        runtime().block_on(async {
            facade::prepare_embedding_batch(
                &capability,
                &queue_profile(),
                Instant::now() + Duration::from_secs(60),
            )
            .await
            .expect("admission failed")
        })
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

/// An `EmbeddingProfile` whose `profile_id` is stable and whose dimensions
/// match the embedding column — the same shape `ResolvedProfile` would yield
/// from a cached snapshot, supplied directly because no model is present.
fn queue_profile() -> EmbeddingProfile {
    EmbeddingProfile {
        model_repo: jobs::DEFAULT_MODEL_REPO.to_string(),
        weights_revision: "queue-test-revision".to_string(),
        weights_checksums: vec!["weights-sha".to_string()],
        tokenizer_checksum: "tok-sha".to_string(),
        tokenizer_config_checksum: "tokcfg-sha".to_string(),
        dimensions: EMBEDDING_DIM as u32,
        engine_version: "tract-onnx-0.23".to_string(),
        pooling: "cls".to_string(),
        normalization: "l2".to_string(),
        query_prefix: String::new(),
        document_prefix: String::new(),
        body_prefix_chars: jobs::DEFAULT_BODY_PREFIX_CHARS,
        max_tokens: jobs::DEFAULT_MAX_TOKENS,
        padding: "none".to_string(),
        truncation: "longest_first".to_string(),
        section_layout_fingerprint: "layout-fixture".to_string(),
    }
}

/// Admission stages the queue into the published generation manifest, and a
/// process that exits without draining loses nothing: a fresh read of
/// `CURRENT.json` + the manifest still names every admitted job
/// (SPEC-013 Phase 2.3 "enqueue survives kill").
#[test]
#[serial_test::serial]
fn enqueued_jobs_survive_process_exit() {
    let workspace = Workspace::migrated();
    assert!(
        workspace.pending_keys().is_empty(),
        "fixture started with pending jobs"
    );

    let prepared = workspace.admit_pending();
    assert!(
        !prepared.inputs.is_empty(),
        "admission prepared no inputs — corpus must admit at least one row"
    );

    // The pending set is durable state: the published manifest names the
    // exact jobs admission minted, keyed under their parent artifact.
    let pending = workspace.pending_keys();
    for input in &prepared.inputs {
        assert!(
            pending.contains(&input.job.canonical_key),
            "admitted job {} absent from the published manifest",
            input.job.canonical_key
        );
    }

    // The staging process is gone; a brand-new forgeplan process reading the
    // workspace must still see the same queue — nothing lives in memory.
    workspace.run_ok(&["list"]);
    let pending_after = workspace.pending_keys();
    assert_eq!(
        pending, pending_after,
        "pending jobs did not survive a process boundary"
    );

    // Each surviving job carries its full binding — incarnation, row
    // revision, digests — not just a key, so a later completion can tell a
    // late vector from a current one.
    let descriptor = storage::read_current(&workspace.lance()).unwrap().unwrap();
    let manifest = storage::read_manifest(&workspace.lance(), &descriptor).unwrap();
    for group in manifest.pending_jobs.values() {
        for job in group.inputs.values() {
            assert_eq!(job.attempt, 0, "fresh admission has a spent attempt");
            assert!(
                job.retry_after.is_none(),
                "fresh admission is already backing off"
            );
            assert_eq!(
                job.profile_id,
                prepared.profile.profile_id(),
                "job is bound to a different profile space"
            );
            assert!(
                !job.input_digest.is_empty() && !job.captured_input_ref.is_empty(),
                "job lost its input binding"
            );
        }
    }
}

/// Completing the admitted jobs through the public completion API retires
/// them from the durable queue: the republished manifest's `pending_jobs` is
/// empty (SPEC-013 Phase 2.3 "drain empties queue").
#[test]
#[serial_test::serial]
fn completed_drain_empties_queue() {
    let workspace = Workspace::migrated();
    let prepared = workspace.admit_pending();
    assert!(!prepared.inputs.is_empty(), "nothing was admitted");

    // What a real drain does after the model returns: one `ComputedEmbedding`
    // per prepared input, each carrying the exact job admission issued.
    let results: Vec<ComputedEmbedding> = prepared
        .inputs
        .iter()
        .map(|input| ComputedEmbedding {
            job: input.job.clone(),
            vector: vec![0.25f32; EMBEDDING_DIM as usize],
        })
        .collect();

    let capability = workspace.capability();
    runtime().block_on(async {
        facade::complete_embedding_batch(
            &capability,
            &prepared,
            &results,
            Instant::now() + Duration::from_secs(60),
        )
        .await
        .expect("completion failed")
    });

    assert!(
        workspace.pending_keys().is_empty(),
        "completed jobs still pending in the published manifest"
    );

    // The queue stays empty across a process boundary — a second pass finds
    // no eligible rows to re-admit.
    workspace.run_ok(&["index"]);
    assert!(
        workspace.pending_keys().is_empty(),
        "a fresh maintenance pass re-admitted completed rows"
    );
}

/// With `FORGEPLAN_MODEL_CACHE` empty and `HF_HUB_OFFLINE=1`, a mutation's
/// post-commit drain attempt reports the model's absence on stderr instead
/// of crashing or silently dropping work: the already-pending jobs are still
/// pending in the manifest afterwards (SPEC-013 Phase 2.3 "skipped drain
/// reports model absent").
#[test]
#[serial_test::serial]
fn skipped_drain_reports_model_absent_and_keeps_jobs() {
    let workspace = Workspace::migrated();
    workspace.enable_embedding_config();
    let prepared = workspace.admit_pending();
    let staged = workspace.pending_keys();
    assert_eq!(
        staged.len(),
        prepared.inputs.len(),
        "admitted jobs missing from the durable queue"
    );

    // `update` is the mutation surface: it commits the canonical file, then
    // attempts the semantic drain. With no cached model the drain cannot run;
    // the command reports the model's absence with the canonical commit still
    // standing — the note, not a silent drop and not a crash. `--status active`
    // is lifecycle-gated before the mutation lands, so use `--status draft`
    // (a no-op that still commits the body and reaches the drain).
    workspace.write_prd(EDITED_BODY);
    let output = workspace.run(&["update", "PRD-001", "--status", "draft"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stderr.contains("unavailable")
            || stderr.contains("queue skipped")
            || stderr.contains("semantic backlog remains pending")
            || stderr.contains("model is unavailable"),
        "skipped drain reported nothing about the absent model\n\
         stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let after = workspace.pending_keys();
    for key in &staged {
        assert!(
            after.contains(key),
            "pending job {key} was lost by the skipped drain"
        );
    }
}

/// `index --embed-profile <id>` on an empty model cache must reach the
/// model-obtain path ("Obtaining the embedding model") — the permitted
/// obtainer bootstraps on first use — rather than the
/// `setup --warm-model` refusal that predated lazy bootstrap (SPEC-013
/// "first semantic use bootstraps", RFC-021 Phase 2.2). Offline, the obtain
/// fails; what is asserted is which path failed.
#[test]
#[serial_test::serial]
fn embed_profile_index_bootstraps_not_warm_model_hint() {
    let workspace = Workspace::migrated();
    workspace.enable_embedding_config();

    let output = workspace.run(&["index", "--embed-profile", "any-profile-id"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // With the semantic-search feature the permitted obtainer reaches the
    // model-obtain path; a no-feature build refuses honestly instead — both
    // are bootstrap, neither is the manual warm-model hint.
    let reached_bootstrap = stdout.contains("Obtaining the embedding model")
        || stderr.contains("Obtaining the embedding model")
        || stderr.contains("model bootstrap failed")
        || stderr.contains("no `semantic-search` feature")
        || stderr.contains("rebuild with --features semantic-search")
        || stderr.contains("without the semantic-search feature");
    assert!(
        reached_bootstrap,
        "index --embed-profile did not reach the bootstrap path\n\
         stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        !stdout.contains("setup --warm-model") && !stderr.contains("setup --warm-model"),
        "index --embed-profile pointed at the manual warm-model refusal\n\
         stdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

/// Child half of the SIGKILL durability test. Re-executed by the driver as
/// `current_exe() --exact sigkill_child_admits_then_sleeps` with
/// `QUEUE_TEST_CHILD=1`, it admits the eligible corpus into the durable
/// pending queue and then sleeps — giving the driver a live process to kill
/// after the admission commit has landed. In the parent run (no
/// `QUEUE_TEST_CHILD`) it returns immediately.
#[test]
#[serial_test::serial]
fn sigkill_child_admits_then_sleeps() {
    if std::env::var("QUEUE_TEST_CHILD").ok().as_deref() != Some("1") {
        return;
    }
    // No `pin_process_env()` here: the driver already exported
    // `FORGEPLAN_MODEL_CACHE` pointing at ITS shared `EMPTY_MODEL_CACHE`
    // TempDir. Re-pinning would substitute this process's own fresh TempDir —
    // a different path, hence a different `semantic_unavailable` fingerprint
    // and a `ProfileIncompatible` refusal at admission.
    let workspace_root =
        PathBuf::from(std::env::var("QUEUE_TEST_WS").expect("QUEUE_TEST_WS unset in child mode"));
    let capability = MaintainCapability::for_strict_maintenance(
        workspace_root
            .join(".forgeplan")
            .canonicalize()
            .expect("child workspace .forgeplan missing"),
    );
    let prepared = runtime().block_on(async {
        facade::prepare_embedding_batch(
            &capability,
            &queue_profile(),
            Instant::now() + Duration::from_secs(60),
        )
        .await
        .expect("child admission failed")
    });
    assert!(
        !prepared.inputs.is_empty(),
        "child admitted no inputs — corpus must admit at least one row"
    );
    // Admission is committed; stay alive so the driver can SIGKILL a real
    // process mid-life rather than re-reading files it wrote itself.
    std::thread::sleep(Duration::from_secs(600));
}

/// The existing durability tests re-read the manifest from the same process
/// that wrote it — that proves the file is durable, not that the queue
/// survives a process dying mid-flight. This driver spawns the test binary
/// as a separate process, waits until the child's admission is visible in
/// the published manifest, then SIGKILLs it (unix `child.kill()` — no
/// destructors, no flush) and asserts the pending set is byte-for-byte what
/// was observed before the kill (SPEC-013 Phase 2.3 "enqueue survives
/// kill", the real-kill variant).
#[test]
#[serial_test::serial]
fn sigkill_does_not_lose_pending_jobs() {
    // When this test is itself re-executed as the child binary it must not
    // act as a driver — the child test owns that mode.
    if std::env::var("QUEUE_TEST_CHILD").ok().as_deref() == Some("1") {
        return;
    }
    let workspace = Workspace::migrated();
    assert!(
        workspace.pending_keys().is_empty(),
        "fixture started with pending jobs"
    );

    let exe = std::env::current_exe().expect("no test binary path");
    let mut child = Command::new(exe)
        .args(["--exact", "sigkill_child_admits_then_sleeps", "--nocapture"])
        .current_dir(&workspace.root)
        .env("QUEUE_TEST_CHILD", "1")
        .env("QUEUE_TEST_WS", &workspace.root)
        .env("HOME", &workspace.home)
        .env("USERPROFILE", &workspace.home)
        .env("XDG_DATA_HOME", &workspace.home)
        .env("FORGEPLAN_MODEL_CACHE", EMPTY_MODEL_CACHE.path())
        .env("HF_HUB_OFFLINE", "1")
        .env("FORGEPLAN_DISABLE_PLUGIN_DISCOVERY", "1")
        .stdin(Stdio::null())
        .spawn()
        .expect("failed to spawn child test binary");

    // Poll until the child's admission commit is visible — proves the write
    // reached the durable manifest before we kill, not after.
    let deadline = Instant::now() + Duration::from_secs(60);
    let pending = loop {
        let keys = workspace.pending_keys();
        if !keys.is_empty() {
            break keys;
        }
        assert!(
            Instant::now() < deadline,
            "child never committed a pending job within 60s"
        );
        if let Some(status) = child.try_wait().expect("child try_wait failed") {
            panic!("child exited early with {status} before admitting jobs");
        }
        std::thread::sleep(Duration::from_millis(100));
    };

    child.kill().expect("failed to SIGKILL child");
    child.wait().expect("failed to reap child");

    // A killed process ran no destructors and flushed nothing: whatever the
    // manifest holds now is exactly what the crash left behind.
    let pending_after = workspace.pending_keys();
    assert_eq!(
        pending, pending_after,
        "SIGKILL lost or mutated pending jobs"
    );

    // And the durable state is not just non-empty but uncorrupted: both
    // reads parse cleanly and still describe a coherent queue.
    let descriptor = storage::read_current(&workspace.lance())
        .unwrap()
        .expect("CURRENT.json unreadable after kill");
    let manifest = storage::read_manifest(&workspace.lance(), &descriptor)
        .expect("manifest unreadable after kill");
    let surviving: usize = manifest
        .pending_jobs
        .values()
        .map(|group| group.inputs.len())
        .sum();
    assert_eq!(
        surviving,
        pending.len(),
        "manifest pending count diverged from the pre-kill set"
    );
}
/// A mutation on a workspace with no cached model still succeeds and leaves
/// durable, recoverable work — not a crash and not silent loss. The durable
/// record the mutation itself publishes is the manifest's `source_entries`:
/// the new artifact is marked `unavailable` (there is no model to mark it
/// anything else), which is the committed backlog admission mints
/// `pending_jobs` from. The second half proves exactly that: the same public
/// admission path every drain opens with finds the new key and writes its
/// durable job into the committed manifest (SPEC-013 Phase 2.3 "new work on
/// a model-less workspace is queued durably").
#[test]
fn new_note_records_durable_backlog_that_admission_mints() {
    let workspace = Workspace::migrated();
    workspace.enable_embedding_config();

    let output = workspace.run(&["new", "note", "Queue durability note", "--allow-duplicate"]);
    assert!(
        output.status.success(),
        "new must succeed without a cached model\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let note_key = stdout
        .lines()
        .find_map(|line| line.trim().strip_prefix("Key:").map(str::trim))
        .expect("new printed no canonical Key line")
        .to_string();

    // What the mutation durably committed: the note is a source entry whose
    // semantic work is explicitly not done. The contract is `!= ready` — the
    // manifest must not claim a vector it never computed.
    let descriptor = storage::read_current(&workspace.lance())
        .unwrap()
        .expect("no published generation after new");
    let manifest = storage::read_manifest(&workspace.lance(), &descriptor).unwrap();
    let entry = manifest
        .source_entries
        .iter()
        .find(|entry| entry.canonical_key == note_key)
        .expect("the mutation committed no source entry for the new note");
    assert_ne!(
        entry.semantic_state,
        jobs::SemanticState::Ready,
        "a model-less mutation published the note as semantically ready"
    );

    // The committed backlog is real work, not a dead record: the next
    // admission mints a durable pending job for it under the note's parent
    // key in the published manifest.
    let prepared = workspace.admit_pending();
    let descriptor = storage::read_current(&workspace.lance())
        .unwrap()
        .expect("no published generation after admission");
    let manifest = storage::read_manifest(&workspace.lance(), &descriptor).unwrap();
    let group = manifest.pending_jobs.get(&note_key).unwrap_or_else(|| {
        panic!(
            "admission minted no pending jobs for {note_key} ({} prepared input(s))",
            prepared.inputs.len()
        )
    });
    assert!(
        group
            .inputs
            .values()
            .any(|job| job.canonical_key == note_key
                || job.canonical_key.starts_with(&format!("{note_key}#"))),
        "pending jobs for {note_key} name no input of the new note"
    );
}

/// A job whose drain fails is not dropped: the failure record leaves the
/// same job pending with its attempt spent and a `retry_after` backoff, and
/// a plain `index` maintenance pass afterwards — run with the model still
/// absent — neither needs the model nor loses the queue (SPEC-013 Phase 2.3
/// "a failed drain keeps its jobs").
#[test]
fn failed_drain_keeps_jobs_across_maintenance() {
    let workspace = Workspace::migrated();
    workspace.enable_embedding_config();
    let prepared = workspace.admit_pending();
    assert!(!prepared.inputs.is_empty(), "nothing was admitted");

    // Record what a drained-then-failed batch records: the exact jobs the
    // admission issued, marked failed against the durable attempt counter.
    let failed_jobs: Vec<jobs::EmbeddingJob> = prepared
        .inputs
        .iter()
        .map(|input| input.job.clone())
        .collect();
    let capability = workspace.capability();
    runtime().block_on(async {
        facade::fail_embedding_batch(
            &capability,
            &prepared,
            &failed_jobs,
            "model produced no vector",
            Instant::now() + Duration::from_secs(60),
        )
        .await
        .expect("failure record failed")
    });

    // The jobs were not completed and were not lost: they sit pending with
    // the attempt spent and a backoff recorded, so a later drain can retry
    // them without minting fresh admissions.
    let descriptor = storage::read_current(&workspace.lance())
        .unwrap()
        .expect("no published generation after failure record");
    let manifest = storage::read_manifest(&workspace.lance(), &descriptor).unwrap();
    for job in &failed_jobs {
        let current = manifest
            .pending_jobs
            .values()
            .flat_map(|group| group.inputs.values())
            .find(|pending| pending.canonical_key == job.canonical_key)
            .unwrap_or_else(|| {
                panic!(
                    "failed job {} vanished from the durable queue",
                    job.canonical_key
                )
            });
        assert_eq!(
            current.attempt,
            job.attempt + 1,
            "failed job {} kept its unspent attempt",
            job.canonical_key
        );
        assert!(
            current.retry_after.is_some(),
            "failed job {} has no backoff recorded",
            job.canonical_key
        );
    }

    // A maintenance pass with the model still absent preserves the queue:
    // pending jobs are durable manifest state, not in-memory drain
    // bookkeeping that a reconcile could silently clear.
    workspace.run_ok(&["index"]);
    let after = workspace.pending_keys();
    for job in &failed_jobs {
        assert!(
            after.contains(&job.canonical_key),
            "a maintenance pass dropped the failed job {}",
            job.canonical_key
        );
    }
}
