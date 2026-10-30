//! Phase commands preserve the advisory YAML store across the index cutover.
//! Fixtures drive the real CLI with a private HOME and an empty model cache;
//! neither provider calls nor embedding inference is part of these scenarios.

use std::path::PathBuf;
use std::process::Output;
use std::sync::{Arc, Barrier};
use std::time::Duration;

use assert_cmd::Command;
use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::projection::{self, MutationContext};
use serde_json::{Value, json};
use tempfile::TempDir;

const ID: &str = "PRD-001";
const SLUG: &str = "prd-phase-repair";

struct Fixture {
    _tmp: TempDir,
    root: PathBuf,
    home: PathBuf,
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
            tables: Vec::new(),
            current: None,
        };
        fixture.ok(&["init", "-y"]);
        let content = "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Phase repair\n\
                       slug: prd-phase-repair\nassigned_number: null\ndepth: standard\n---\n\n\
                       ## Summary\n\nTrack advisory phase history.\n";
        std::fs::write(
            fixture.workspace().join("prds/PRD-001-phase-repair.md"),
            content,
        )
        .unwrap();
        // Legacy slug resolution reads frontmatter from the stored body.
        // Seed those bytes directly through the real projection/store seam:
        // legacy reindex strips them, a separate pre-existing behavior.
        let workspace = fixture.workspace();
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let lock = forgeplan_core::workspace::acquire_workspace_lock(&workspace)
                    .await
                    .unwrap();
                let store = LanceStore::open(&workspace).await.unwrap();
                let artifact = NewArtifact {
                    canonical_key: ID.to_string(),
                    id: ID.to_string(),
                    kind: "prd".to_string(),
                    status: "draft".to_string(),
                    title: "Phase repair".to_string(),
                    body: content.to_string(),
                    depth: "standard".to_string(),
                    ..Default::default()
                };
                projection::sync_artifact_from_file(
                    &MutationContext::new(&workspace, &lock, Some(&store)),
                    &artifact,
                )
                .await
                .unwrap();
                assert_eq!(store.get_record(ID).await.unwrap().unwrap().body, content);
            });
        // The runtime, store and lock have all dropped before a CLI child opens them.
        let artifact = fixture.json(&["get", ID, "--json"]);
        assert_eq!(artifact["id"], ID);
        assert_eq!(artifact["title"], "Phase repair");
        assert_eq!(artifact["slug"], SLUG);
        if migrated {
            fixture.ok(&["index"]);
            assert!(fixture.workspace().join("lance/CURRENT.json").is_file());
            assert!(!fixture.workspace().join("lance/artifacts.lance").exists());
            assert!(!fixture.workspace().join("lance/relations.lance").exists());
        } else {
            assert!(fixture.workspace().join("lance/artifacts.lance").exists());
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

    /// The phase state file, named after the document's canonical key.
    ///
    /// The key is the slug the document declares (ADR-030), so the state of
    /// this document lives under its key — not under the display id that a
    /// second document may share.
    fn state_path(&self) -> PathBuf {
        self.workspace().join(format!("state/{SLUG}.yaml"))
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
    assert_eq!(
        payload,
        &json!({
            "artifact_id": id,
            "canonical_key": key,
            "current_phase": "unknown",
            "workflow_type": "greenfield",
            "history": [],
            "message": "No phase state file on disk -- advisory only, never an error",
            "_next_action": format!("forgeplan phase-advance {id} --to shape"),
        }),
    );
}

fn assert_advance(
    payload: &Value,
    phase: &str,
    history_entries: usize,
    reason: Option<&str>,
    next: Option<&str>,
) {
    chrono::DateTime::parse_from_rfc3339(payload["advanced_at"].as_str().unwrap()).unwrap();
    assert_eq!(
        payload,
        &json!({
            "artifact_id": ID,
            "canonical_key": SLUG,
            "current_phase": phase,
            "workflow_type": "greenfield",
            "advanced_at": payload["advanced_at"],
            "history_entries": history_entries,
            "reason": reason,
            "suggested_next": next,
            "_next_action": next.map(|next| format!("forgeplan phase-advance {ID} --to {next}")),
        }),
    );
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
            assert_unknown(&fixture.json(&["phase", reference, "--json"]), ID, SLUG);
        }
        let unknown_text = fixture.ok(&["phase", SLUG]);
        assert!(unknown_text.contains("PRD-001 phase: unknown (no state file)"));
        assert!(unknown_text.contains("Next: forgeplan phase-advance PRD-001 --to shape"));
        assert!(!fixture.state_path().exists());

        let first = fixture.json(&["phase-advance", SLUG, "--to", "shape", "--json"]);
        assert_advance(&first, "shape", 1, None, Some("validate"));
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
        assert_advance(&code, "code", 2, Some("skip validation"), Some("test"));
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
        assert_advance(&repeated, "code", 2, Some("no-op"), Some("test"));
        assert_eq!(repeated["advanced_at"], code["advanced_at"]);
        assert_eq!(std::fs::read(fixture.state_path()).unwrap(), saved);

        let done = fixture.json(&["phase-advance", "PRD-1", "--to", "done", "--json"]);
        assert_advance(&done, "done", 3, None, None);
        let done_text = fixture.ok(&["phase", SLUG]);
        assert!(done_text.contains("terminal phase -- no further advancement recommended"));
        assert!(done_text.ends_with("Done.\n"));
        assert!(!done_text.contains("Next:"));

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
            history["_next_action"],
            "forgeplan phase-advance PRD-001 --to adi"
        );
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
        let text = fixture.ok(&["phase", "prd-1"]);
        assert!(text.contains("history (showing last 3 of 4):"));
        assert!(text.contains("shape -> code -- skip validation"));
        assert!(text.contains("code -> done"));
        assert!(text.contains("done -> validate -- operator correction"));
        assert!(!text.contains("- -> shape"));
        assert!(text.ends_with("Next: forgeplan phase-advance PRD-001 --to adi\n"));
        fixture.assert_index_unchanged();
    }
}

#[test]
fn missing_artifacts_keep_raw_advisory_reads_but_cannot_be_advanced() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated);
        for reference in ["PRD-999", "untracked-phase"] {
            assert_unknown(&fixture.json(&["phase", reference, "--json"]), reference, reference);
            fixture.fails(
                &["phase-advance", reference, "--to", "code", "--json"],
                1,
                &format!("Artifact '{reference}' not found"),
            );
        }
        let path = fixture.workspace().join("state/PRD-999.yaml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let bytes = b"artifact_id: PRD-999\nworkflow_type: greenfield\ncurrent_phase: code\n\
            advanced_at: 2026-10-24T00:00:00Z\nschema_version: 1\nhistory:\n\
            - from: shape\n  to: code\n  at: 2026-10-24T00:00:00Z\n  reason: offline state\n";
        std::fs::write(&path, bytes).unwrap();
        let payload = fixture.json(&["phase", "PRD-999", "--json"]);
        assert_eq!(payload["artifact_id"], "PRD-999");
        assert_eq!(payload["current_phase"], "code");
        assert_eq!(
            payload["_next_action"],
            "forgeplan phase-advance PRD-999 --to test"
        );
        assert_eq!(
            transitions(&payload),
            json!([["shape", "code", "offline state"]])
        );
        fixture.fails(
            &["phase-advance", "PRD-999", "--to", "test", "--json"],
            1,
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
        fixture.fails(
            &["phase", "PRD/001", "--json"],
            1,
            "artifact_id contains invalid character",
        );
        fixture.fails(
            &["phase", "../escape", "--json"],
            1,
            "artifact_id must start with an ASCII letter",
        );
        fixture.fails(
            &["phase-advance", "PRD/001", "--to", "code", "--json"],
            1,
            "Artifact 'PRD/001' not found",
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
        assert_advance(&advanced, "test", 2, Some(&reason), Some("audit"));
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
fn corrupt_phase_yaml_is_quarantined_and_remains_an_advisory_unknown() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated);
        let state_dir = fixture.workspace().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let corrupt = b"{not valid yaml";
        std::fs::write(fixture.state_path(), corrupt).unwrap();
        assert_unknown(&fixture.json(&["phase", SLUG, "--json"]), ID, SLUG);
        assert!(!fixture.state_path().exists());
        let quarantined: Vec<_> = std::fs::read_dir(&state_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(&format!("{SLUG}.yaml.corrupt."))
            })
            .collect();
        assert_eq!(quarantined.len(), 1);
        assert_eq!(std::fs::read(&quarantined[0]).unwrap(), corrupt);
        let recovered = fixture.json(&[
            "phase-advance",
            SLUG,
            "--to",
            "test",
            "--reason",
            "recover advisory state",
            "--json",
        ]);
        assert_advance(
            &recovered,
            "test",
            2,
            Some("recover advisory state"),
            Some("audit"),
        );
        assert_eq!(
            transitions(&fixture.json(&["phase", ID, "--json"])),
            json!([
                [null, "shape", null],
                ["shape", "test", "recover advisory state"]
            ]),
        );
        assert_eq!(std::fs::read(&quarantined[0]).unwrap(), corrupt);
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
        assert_unknown(&fixture.json(&["phase", SLUG, "--json"]), ID, SLUG);
        fixture.fails(
            &["phase-advance", SLUG, "--to", "test", "--json"],
            1,
            "is a symlink — refusing to overwrite",
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
        // A phase read still does not acquire a writer lock.
        assert_eq!(
            fixture.json(&["phase", SLUG, "--json"])["current_phase"],
            "code"
        );
        fixture.fails(
            &["phase-advance", SLUG, "--to", "test", "--json"],
            1,
            "lock file",
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
