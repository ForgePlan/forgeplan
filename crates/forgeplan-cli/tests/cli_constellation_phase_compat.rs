//! Phase commands preserve the advisory YAML store across the index cutover.
//! Fixtures drive the real CLI with a private HOME and an empty model cache;
//! neither provider calls nor embedding inference is part of these scenarios.

#[path = "common/historical_lance.rs"]
mod historical_lance;

use std::path::PathBuf;
use std::process::Output;
use std::sync::{Arc, Barrier};
use std::time::Duration;

use assert_cmd::Command;
use forgeplan_core::db::store::LanceStore;
use serde_json::{Value, json};
use tempfile::TempDir;

const ID: &str = "PRD-001";
const SLUG: &str = "prd-phase-repair";

struct Fixture {
    _tmp: TempDir,
    root: PathBuf,
    home: PathBuf,
    canonical_key: String,
    tables: Vec<String>,
    current: Option<Vec<u8>>,
}

impl Fixture {
    fn new(migrated: bool) -> Self {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("project");
        let home = tmp.path().join("home");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        let mut fixture = Self {
            _tmp: tmp,
            root,
            home,
            canonical_key: String::new(),
            tables: Vec::new(),
            current: None,
        };
        historical_lance::init_legacy_workspace(&fixture.root, "Phase compatibility fixture");
        let prose = "## Summary\n\nTrack advisory phase history.\n";
        let content = format!(
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Phase repair\n\
             slug: prd-phase-repair\nassigned_number: null\ndepth: standard\n---\n\n{prose}"
        );
        std::fs::write(
            fixture.workspace().join("prds/PRD-001-phase-repair.md"),
            &content,
        )
        .unwrap();
        // Opening the verified writer imports the canonical file; do not insert it twice.
        let workspace = fixture.workspace();
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let lock = forgeplan_core::workspace::acquire_workspace_lock(&workspace)
                    .await
                    .unwrap();
                let store = LanceStore::open_for_write(&workspace, &lock).await.unwrap();
                let record = store.get_record(&lock, ID).await.unwrap().unwrap();
                assert_eq!(record.body, prose);
                assert_eq!(record.canonical_key, SLUG);
            });
        // The runtime, store and lock have all dropped before a CLI child opens them.
        let artifact = fixture.json(&["get", ID, "--json"]);
        assert_eq!(artifact["id"], ID);
        assert_eq!(artifact["title"], "Phase repair");
        let canonical_key = artifact["canonical_key"].as_str().unwrap().to_owned();
        assert_eq!(canonical_key, SLUG);
        fixture.canonical_key = canonical_key;
        assert_eq!(
            std::fs::read_to_string(fixture.workspace().join("prds/PRD-001-phase-repair.md"))
                .unwrap(),
            content
        );
        if migrated {
            fixture.ok(&["index"]);
            assert!(fixture.workspace().join("lance/CURRENT.json").is_file());
            assert!(
                !fixture
                    .workspace()
                    .join("lance/artifacts_identity_v5.lance")
                    .exists()
            );
            assert!(
                !fixture
                    .workspace()
                    .join("lance/relations_identity_v5.lance")
                    .exists()
            );
        } else {
            assert!(fixture.workspace().join("lance/IDENTITY_V5.json").is_file());
            assert!(
                fixture
                    .workspace()
                    .join("lance/artifacts_identity_v5.lance")
                    .exists()
            );
            assert!(!fixture.workspace().join("lance/CURRENT.json").exists());
        }
        assert!(!fixture.state_path().exists());
        fixture.tables = fixture.table_names();
        fixture.current = std::fs::read(fixture.workspace().join("lance/CURRENT.json")).ok();
        fixture
    }

    fn workspace(&self) -> PathBuf {
        self.root.join(".forgeplan")
    }

    /// The admitted immutable key, not a mutable selector or display id.
    fn state_path(&self) -> PathBuf {
        self.workspace()
            .join(format!("state/{}.yaml", self.canonical_key))
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::cargo_bin("forgeplan").unwrap();
        command
            .args(args)
            .current_dir(&self.root)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("XDG_DATA_HOME", &self.home)
            .env("FORGEPLAN_MODEL_CACHE", self.home.join("models"))
            .env("HF_HUB_OFFLINE", "1")
            .env("NO_COLOR", "1")
            .env("CLICOLOR", "0")
            .timeout(Duration::from_secs(60));
        command
    }

    fn ok(&self, args: &[&str]) -> String {
        let output = self.command(args).output().unwrap();
        assert!(
            output.status.success(),
            "forgeplan {args:?} failed: {}\n{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout),
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn json(&self, args: &[&str]) -> Value {
        let stdout = self.ok(args);
        serde_json::from_str(&stdout)
            .unwrap_or_else(|error| panic!("forgeplan {args:?}: {error}: {stdout}"))
    }

    fn fails(&self, args: &[&str], code: i32, message: &str) -> Output {
        let output = self.command(args).output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(code),
            "forgeplan {args:?}: {}\n{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout),
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(message), "forgeplan {args:?}: {stderr}");
        assert!(output.stdout.is_empty(), "error emitted a success payload");
        output
    }

    fn table_names(&self) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(self.workspace().join("lance"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn assert_index_unchanged(&self) {
        assert_eq!(self.table_names(), self.tables, "index inventory changed");
        assert_eq!(
            std::fs::read(self.workspace().join("lance/CURRENT.json")).ok(),
            self.current,
            "phase commands must not publish an index generation",
        );
        // The state file is named after the canonical key — never after the
        // display id a second document may share, and never after a form of
        // it.
        assert!(!self.workspace().join(format!("state/{ID}.yaml")).exists());
        assert!(!self.workspace().join("state/prd-1.yaml").exists());
    }
}

fn assert_unknown(payload: &Value, id: &str, key: &str) {
    assert_eq!(payload["artifact_id"], id);
    assert_eq!(payload["canonical_key"], key);
    assert_eq!(payload["current_phase"], "unknown");
    assert_eq!(payload["workflow_type"], "greenfield");
    assert_eq!(payload["history"], json!([]));
}

fn assert_advance(
    payload: &Value,
    canonical_key: &str,
    phase: &str,
    history_entries: usize,
    reason: Option<&str>,
    next: Option<&str>,
) {
    chrono::DateTime::parse_from_rfc3339(payload["advanced_at"].as_str().unwrap()).unwrap();
    assert_eq!(payload["artifact_id"], ID);
    assert_eq!(payload["canonical_key"], canonical_key);
    assert_eq!(payload["current_phase"], phase);
    assert_eq!(payload["workflow_type"], "greenfield");
    assert_eq!(payload["history_entries"], history_entries);
    assert_eq!(payload["reason"], json!(reason));
    assert_eq!(payload["suggested_next"], json!(next));
}

fn transitions(payload: &Value) -> Value {
    Value::Array(
        payload["history"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| {
                chrono::DateTime::parse_from_rfc3339(entry["at"].as_str().unwrap()).unwrap();
                json!([entry["from"], entry["to"], entry["reason"]])
            })
            .collect(),
    )
}

#[test]
fn phase_show_advance_and_history_preserve_id_slug_and_display_resolution() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated);
        for reference in [ID, SLUG, "prd-1", "PRD-1"] {
            assert_unknown(
                &fixture.json(&["phase", reference, "--json"]),
                ID,
                &fixture.canonical_key,
            );
        }
        assert!(!fixture.state_path().exists());

        let first = fixture.json(&["phase-advance", SLUG, "--to", "shape", "--json"]);
        assert_advance(
            &first,
            &fixture.canonical_key,
            "shape",
            1,
            None,
            Some("validate"),
        );
        let initial = fixture.json(&["phase", "prd-1", "--json"]);
        assert_eq!(initial["schema_version"], 1);
        assert_eq!(initial["workflow_type"], "greenfield");
        assert_eq!(initial["advanced_at"], first["advanced_at"]);
        assert_eq!(transitions(&initial), json!([[null, "shape", null]]));

        let code = fixture.json(&[
            "phase-advance",
            SLUG,
            "--to",
            "code",
            "--reason",
            "skip validation",
            "--json",
        ]);
        assert_advance(
            &code,
            &fixture.canonical_key,
            "code",
            2,
            Some("skip validation"),
            Some("test"),
        );
        let saved = std::fs::read(fixture.state_path()).unwrap();
        let repeated = fixture.json(&[
            "phase-advance",
            "prd-1",
            "--to",
            "code",
            "--reason",
            "no-op",
            "--json",
        ]);
        assert_advance(
            &repeated,
            &fixture.canonical_key,
            "code",
            2,
            Some("no-op"),
            Some("test"),
        );
        assert_eq!(repeated["advanced_at"], code["advanced_at"]);
        assert_eq!(std::fs::read(fixture.state_path()).unwrap(), saved);

        let done = fixture.json(&["phase-advance", "PRD-1", "--to", "done", "--json"]);
        assert_advance(&done, &fixture.canonical_key, "done", 3, None, None);

        // Manual corrections remain advisory, including backwards moves.
        let corrected = fixture.json(&[
            "phase-advance",
            "prd-001",
            "--to",
            "validate",
            "--reason",
            "operator correction",
            "--json",
        ]);
        assert_advance(
            &corrected,
            &fixture.canonical_key,
            "validate",
            4,
            Some("operator correction"),
            Some("adi"),
        );
        let history = fixture.json(&["phase", ID, "--json"]);
        assert_eq!(history["current_phase"], "validate");
        assert_eq!(history["artifact_id"], ID);
        assert_eq!(history["advanced_at"], corrected["advanced_at"]);
        assert_eq!(
            transitions(&history),
            json!([
                [null, "shape", null],
                ["shape", "code", "skip validation"],
                ["code", "done", null],
                ["done", "validate", "operator correction"],
            ]),
        );
        assert_eq!(fixture.json(&["phase", SLUG, "--json"]), history);
        let state: Value =
            serde_yaml::from_slice(&std::fs::read(fixture.state_path()).unwrap()).unwrap();
        assert_eq!(state["artifact_id"], ID);
        assert_eq!(state["current_phase"], "validate");
        assert_eq!(state["history"].as_array().unwrap().len(), 4);
        fixture.assert_index_unchanged();
    }
}

#[test]
fn missing_artifacts_are_not_resolved_from_raw_ids_or_sidecars() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated);
        for reference in ["PRD-999", "prd-untracked-phase"] {
            fixture.fails(
                &["phase", reference, "--json"],
                3,
                &format!("Artifact '{reference}' not found"),
            );
            fixture.fails(
                &["phase-advance", reference, "--to", "code", "--json"],
                3,
                &format!("Artifact '{reference}' not found"),
            );
        }
        let path = fixture.workspace().join("state/PRD-999.yaml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let bytes = b"artifact_id: PRD-999\nworkflow_type: greenfield\ncurrent_phase: code\n\
            advanced_at: 2026-10-24T00:00:00Z\nschema_version: 1\nhistory:\n\
            - from: shape\n  to: code\n  at: 2026-10-24T00:00:00Z\n  reason: offline state\n";
        std::fs::write(&path, bytes).unwrap();
        fixture.fails(
            &["phase", "PRD-999", "--json"],
            3,
            "Artifact 'PRD-999' not found",
        );
        fixture.fails(
            &["phase-advance", "PRD-999", "--to", "test", "--json"],
            3,
            "Artifact 'PRD-999' not found",
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(!fixture.state_path().exists());
        fixture.assert_index_unchanged();
    }
}

#[test]
fn invalid_phase_input_preserves_exit_codes_and_the_existing_state() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated);
        fixture.ok(&["phase-advance", ID, "--to", "shape"]);
        let before = std::fs::read(fixture.state_path()).unwrap();
        for target in ["bogus", "unknown"] {
            fixture.fails(
                &["phase-advance", ID, "--to", target, "--json"],
                2,
                "invalid value",
            );
        }
        fixture.fails(&["phase-advance", ID, "--json"], 2, "--to");
        let excessive_reason = "x".repeat(4097);
        fixture.fails(
            &[
                "phase-advance",
                SLUG,
                "--to",
                "code",
                "--reason",
                &excessive_reason,
                "--json",
            ],
            1,
            "reason too long: 4097 bytes (max: 4096)",
        );
        // The wording for a malformed selector belongs to the ONE shared
        // resolver — the CLI does not keep a second, older message.
        fixture.fails(
            &["phase", "PRD/001", "--json"],
            1,
            "not a valid artifact reference",
        );
        fixture.fails(
            &["phase", "../escape", "--json"],
            1,
            "not a valid artifact reference",
        );
        fixture.fails(
            &["phase-advance", "PRD/001", "--to", "code", "--json"],
            1,
            "not a valid artifact reference",
        );
        assert_eq!(std::fs::read(fixture.state_path()).unwrap(), before);
        fixture.assert_index_unchanged();
    }
}

#[test]
fn a_boundary_length_reason_is_echoed_but_history_retains_the_existing_utf8_cap() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated);
        let reason = "é".repeat(2048);
        assert_eq!(reason.len(), 4096);
        let advanced = fixture.json(&[
            "phase-advance",
            SLUG,
            "--to",
            "test",
            "--reason",
            &reason,
            "--json",
        ]);
        assert_advance(
            &advanced,
            &fixture.canonical_key,
            "test",
            2,
            Some(&reason),
            Some("audit"),
        );
        let reread = fixture.json(&["phase", ID, "--json"]);
        assert_eq!(
            transitions(&reread),
            json!([[null, "shape", null], ["shape", "test", "é".repeat(256)]]),
        );
        assert_eq!(reread["history"][1]["reason"].as_str().unwrap().len(), 512);
        fixture.assert_index_unchanged();
    }
}

#[test]
fn corrupt_phase_yaml_refuses_reads_without_discarding_state() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated);
        let state_dir = fixture.workspace().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let corrupt = b"{not valid yaml";
        std::fs::write(fixture.state_path(), corrupt).unwrap();
        fixture.fails(&["phase", SLUG, "--json"], 1, "malformed");
        assert_eq!(std::fs::read(fixture.state_path()).unwrap(), corrupt);
        assert_eq!(
            std::fs::read_dir(&state_dir).unwrap().count(),
            1,
            "a read must not quarantine or replace phase state"
        );
        fixture.assert_index_unchanged();
    }
}

#[test]
fn conflicting_phase_advances_serialize_without_losing_transition_history() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated);
        fixture.ok(&["phase-advance", ID, "--to", "shape"]);
        let requests = [
            ("validate", "one"),
            ("code", "two"),
            ("audit", "three"),
            ("done", "four"),
        ];
        let barrier = Arc::new(Barrier::new(requests.len()));
        let mut counts = std::thread::scope(|scope| {
            let handles: Vec<_> = requests
                .iter()
                .map(|&(target, reason)| {
                    let barrier = Arc::clone(&barrier);
                    let fixture = &fixture;
                    scope.spawn(move || {
                        barrier.wait();
                        let payload = fixture.json(&[
                            "phase-advance",
                            SLUG,
                            "--to",
                            target,
                            "--reason",
                            reason,
                            "--json",
                        ]);
                        assert_eq!(payload["artifact_id"], ID);
                        assert_eq!(payload["current_phase"], target);
                        assert_eq!(payload["reason"], reason);
                        payload["history_entries"].as_u64().unwrap()
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });
        counts.sort_unstable();
        assert_eq!(counts, [2, 3, 4, 5]);
        let payload = fixture.json(&["phase", ID, "--json"]);
        let history = payload["history"].as_array().unwrap();
        assert_eq!(history.len(), 5);
        assert_eq!(history[0]["from"], Value::Null);
        assert_eq!(history[0]["to"], "shape");
        let mut actual: Vec<_> = history[1..]
            .iter()
            .map(|entry| {
                (
                    entry["to"].as_str().unwrap(),
                    entry["reason"].as_str().unwrap(),
                )
            })
            .collect();
        actual.sort_unstable();
        let mut expected = requests.to_vec();
        expected.sort_unstable();
        assert_eq!(actual, expected);
        for pair in history.windows(2) {
            assert_eq!(pair[1]["from"], pair[0]["to"]);
        }
        assert_eq!(payload["current_phase"], history.last().unwrap()["to"]);
        assert_eq!(fixture.json(&["phase", SLUG, "--json"]), payload);
        fixture.assert_index_unchanged();
    }
}

#[cfg(unix)]
#[test]
fn state_and_lock_symlink_conflicts_refuse_advances_without_changing_the_targets() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated);
        fixture.ok(&["phase-advance", ID, "--to", "code"]);
        let original = std::fs::read(fixture.state_path()).unwrap();
        let victim = fixture.root.join("outside-phase.yaml");
        std::fs::write(&victim, &original).unwrap();
        std::fs::remove_file(fixture.state_path()).unwrap();
        std::os::unix::fs::symlink(&victim, fixture.state_path()).unwrap();
        fixture.fails(&["phase", SLUG, "--json"], 1, "symlink");
        fixture.fails(
            &["phase-advance", SLUG, "--to", "test", "--json"],
            1,
            "symlink",
        );
        assert_eq!(std::fs::read(&victim).unwrap(), original);
        assert!(
            std::fs::symlink_metadata(fixture.state_path())
                .unwrap()
                .file_type()
                .is_symlink()
        );

        std::fs::remove_file(fixture.state_path()).unwrap();
        std::fs::write(fixture.state_path(), &original).unwrap();
        let lock = fixture.workspace().join(".lock");
        std::fs::remove_file(&lock).unwrap();
        std::os::unix::fs::symlink(&victim, &lock).unwrap();
        fixture.fails(&["phase", SLUG, "--json"], 1, "lock");
        fixture.fails(
            &["phase-advance", SLUG, "--to", "test", "--json"],
            1,
            "lock",
        );
        assert_eq!(std::fs::read(fixture.state_path()).unwrap(), original);
        assert_eq!(std::fs::read(&victim).unwrap(), original);
        fixture.assert_index_unchanged();
    }
}

#[test]
fn a_broken_capture_is_an_error_instead_of_unknown_or_a_successful_advance() {
    let fixture = Fixture::new(true);
    fixture.ok(&["phase-advance", SLUG, "--to", "code"]);
    let original = std::fs::read(fixture.state_path()).unwrap();
    let current = fixture.workspace().join("lance/CURRENT.json");
    let corrupt = b"{not valid json";
    std::fs::write(&current, corrupt).unwrap();
    for args in [
        vec!["phase", SLUG, "--json"],
        vec!["phase", "PRD-999", "--json"],
        vec!["phase-advance", SLUG, "--to", "test", "--json"],
    ] {
        fixture.fails(&args, 1, "index_generation_unavailable");
        assert_eq!(std::fs::read(fixture.state_path()).unwrap(), original);
        assert_eq!(std::fs::read(&current).unwrap(), corrupt);
        assert_eq!(fixture.table_names(), fixture.tables);
    }
}
