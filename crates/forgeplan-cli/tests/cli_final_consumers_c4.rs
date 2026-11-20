#![cfg(unix)]

use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    process::{Command, Output},
    thread,
    time::{Duration, Instant},
};

use assert_cmd::cargo::CommandCargoExt;
use fs2::FileExt;
use serde_json::Value;
use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    home: PathBuf,
}

impl Fixture {
    fn new(migrated: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        let home = temp.path().join("home");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&home).unwrap();
        let fixture = Self {
            _temp: temp,
            root,
            home,
        };
        fixture.ok(&["init", "-y"]);
        fixture.write_note("NOTE-001", "initial body");
        fixture.ok(&["reindex"]);
        if migrated {
            fixture.ok(&["index"]);
        }
        fixture
    }

    fn workspace(&self) -> PathBuf {
        self.root.join(".forgeplan")
    }

    fn note_path(&self) -> PathBuf {
        self.note_path_for("NOTE-001")
    }

    fn note_path_for(&self, path_id: &str) -> PathBuf {
        self.workspace()
            .join("notes")
            .join(format!("{path_id}-note.md"))
    }

    fn write_note(&self, id: &str, body: &str) {
        self.write_note_at("NOTE-001", id, "draft", body);
    }

    fn write_note_at(&self, path_id: &str, id: &str, status: &str, body: &str) {
        fs::create_dir_all(self.workspace().join("notes")).unwrap();
        fs::write(
            self.note_path_for(path_id),
            format!("---\nid: {id}\nkind: note\nstatus: {status}\ntitle: C4 note\ndepth: tactical\n---\n\n{body}\n"),
        )
        .unwrap();
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
            .env("FORGEPLAN_DISABLE_PLUGIN_DISCOVERY", "1");
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    fn ok(&self, args: &[&str]) -> Output {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "forgeplan {args:?}: stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn history(&self) -> Vec<Value> {
        let output = self.ok(&["log", "--limit", "1000", "--json"]);
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["entries"]
            .as_array()
            .unwrap()
            .clone()
    }

    fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .env("HOME", &self.home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Eli Rum")
            .env("GIT_AUTHOR_EMAIL", "c4@example.invalid")
            .env("GIT_COMMITTER_NAME", "Eli Rum")
            .env("GIT_COMMITTER_EMAIL", "c4@example.invalid")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn commit_baseline(&self) -> String {
        self.git(&["init", "-q"]);
        self.git(&["add", "--", ".forgeplan/notes"]);
        self.git(&["commit", "-q", "-m", "baseline"]);
        self.git(&["rev-parse", "HEAD"])
    }
}

fn hold_index_owner_lock(workspace: &Path) -> File {
    let index_root = workspace.join("lance").canonicalize().unwrap();
    let digest = forgeplan_core::artifact::identity_repair::sha256_hex(
        index_root.as_os_str().as_encoded_bytes(),
    );
    let path = index_root
        .parent()
        .unwrap()
        .join(format!(".constellation-owner-{digest}.lock"));
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)
        .unwrap();
    FileExt::lock_exclusive(&file).unwrap();
    file
}

fn hold_workspace_lock_after_release(workspace: &Path) -> File {
    let path = workspace.join(".lock");
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut observed_busy = false;
    while Instant::now() < deadline {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .unwrap();
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) if observed_busy => return file,
            Ok(()) => {
                FileExt::unlock(&file).unwrap();
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                observed_busy = true;
            }
            Err(error) => panic!("workspace lock probe failed: {error}"),
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("managed command did not release the workspace lock for maintenance");
}

fn real_git_path() -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|directory| directory.join("git"))
        .find(|path| path.is_file())
        .unwrap()
}

fn audit_count(history: &[Value], source: &str) -> usize {
    history
        .iter()
        .filter(|entry| entry["source"] == source)
        .count()
}
#[tokio::test]
async fn git_sync_verifies_metadata_only_inputs_after_releasing_workspace_lock_before_any_audit_append()
 {
    let fixture = Fixture::new(true);
    fixture.write_note_at("NOTE-002", "NOTE-002", "draft", "unchanged metadata body");
    fixture.write_note_at("NOTE-003", "NOTE-003", "draft", "verified deletion");
    fixture.ok(&["reindex"]);
    let baseline = fixture.commit_baseline();
    fixture.write_note("NOTE-001", "body transition");
    fixture.write_note_at("NOTE-002", "NOTE-002", "stale", "unchanged metadata body");
    fs::remove_file(fixture.note_path_for("NOTE-003")).unwrap();
    fixture.git(&["add", "--", ".forgeplan/notes"]);
    fixture.git(&["commit", "-q", "-m", "body metadata and deletion input"]);
    let history_before = fixture.history();
    let index_root = fixture.workspace().join("lance");
    let before_descriptor = forgeplan_core::constellation::storage::read_current(&index_root)
        .unwrap()
        .unwrap();
    let wrapper_dir = fixture._temp.path().join("planning-git-wrapper");
    fs::create_dir_all(&wrapper_dir).unwrap();
    let wrapper = wrapper_dir.join("git");
    let signal = fixture._temp.path().join("planning-started");
    let release = fixture._temp.path().join("planning-continue");
    fs::write(
        &wrapper,
        "#!/bin/sh\n\
         \"$FORGEPLAN_TEST_REAL_GIT\" \"$@\" > \"$FORGEPLAN_TEST_GIT_OUTPUT\"\n\
         status=$?\n\
         if [ \"$1\" = cat-file ] && [ \"$2\" = blob ] && [ \"$3\" = \"$FORGEPLAN_TEST_PAUSE_SPEC\" ]; then\n\
         \tprintf ready > \"$FORGEPLAN_TEST_PLAN_SIGNAL\"\n\
         \tn=0\n\
         \twhile [ ! -f \"$FORGEPLAN_TEST_PLAN_RELEASE\" ] && [ \"$n\" -lt 2000 ]; do\n\
         \t\tsleep 0.01\n\
         \t\tn=$((n + 1))\n\
         \tdone\n\
         \t[ -f \"$FORGEPLAN_TEST_PLAN_RELEASE\" ] || exit 125\n\
         fi\n\
         cat \"$FORGEPLAN_TEST_GIT_OUTPUT\"\n\
         exit \"$status\"\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = std::env::join_paths(
        std::iter::once(wrapper_dir)
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let owner_lock = hold_index_owner_lock(&fixture.workspace());
    let child = fixture
        .command(&["git-sync", "--since", &baseline])
        .env("PATH", path)
        .env("FORGEPLAN_TEST_REAL_GIT", real_git_path())
        .env(
            "FORGEPLAN_TEST_GIT_OUTPUT",
            fixture._temp.path().join("planning-git-output"),
        )
        .env(
            "FORGEPLAN_TEST_PAUSE_SPEC",
            format!("{baseline}:.forgeplan/notes/NOTE-003-note.md"),
        )
        .env("FORGEPLAN_TEST_PLAN_SIGNAL", &signal)
        .env("FORGEPLAN_TEST_PLAN_RELEASE", &release)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while !signal.is_file() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(2));
    }
    assert!(
        signal.is_file(),
        "Git sync did not reach its pinned deletion preimage"
    );
    // The pinned Git blob read is inside the post-audit-lock planning guard.
    // Holding the index owner prevents publication after that guard is released.
    fs::write(&release, b"continue").unwrap();
    let workspace_lock = OpenOptions::new()
        .read(true)
        .write(true)
        .open(fixture.workspace().join(".lock"))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match FileExt::try_lock_exclusive(&workspace_lock) {
            Ok(()) => break,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                thread::sleep(Duration::from_millis(2))
            }
            Err(error) => panic!("planned workspace guard was not released: {error}"),
        }
    }
    fixture.write_note_at(
        "NOTE-002",
        "NOTE-002",
        "deprecated",
        "changed after planning",
    );
    FileExt::unlock(&workspace_lock).unwrap();
    drop(workspace_lock);
    FileExt::unlock(&owner_lock).unwrap();
    drop(owner_lock);
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert_eq!(fixture.history(), history_before);
    let after_descriptor = forgeplan_core::constellation::storage::read_current(&index_root)
        .unwrap()
        .unwrap();
    assert_ne!(
        after_descriptor.generation_id,
        before_descriptor.generation_id
    );
    let rows = forgeplan_core::constellation::facade::read_pinned_artifacts(
        &index_root,
        &after_descriptor,
    )
    .await
    .unwrap();
    let changed = rows
        .iter()
        .find(|row| row.canonical_key == "NOTE-002")
        .unwrap();
    assert_eq!(changed.status.as_deref(), Some("deprecated"));
    assert_eq!(
        changed.body.as_deref().map(str::trim),
        Some("changed after planning")
    );
    assert!(!rows.iter().any(|row| row.canonical_key == "NOTE-003"));
}

#[test]
fn git_sync_refuses_canonical_key_replacement_before_index_or_audit_writes_in_both_layouts() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated);
        let baseline = fixture.commit_baseline();
        let history_before = fixture.history();
        let current_marker = fs::read(fixture.workspace().join("lance/CURRENT.json")).ok();

        fixture.write_note("NOTE-002", "replacement bytes");
        let replacement_bytes = fs::read(fixture.note_path()).unwrap();
        fixture.git(&["add", "--", ".forgeplan/notes"]);
        fixture.git(&["commit", "-q", "-m", "replace canonical key"]);

        let output = fixture.run(&["git-sync", "--since", &baseline]);
        assert!(!output.status.success(), "layout migrated={migrated}");
        assert_eq!(fs::read(fixture.note_path()).unwrap(), replacement_bytes);
        assert_eq!(fixture.history(), history_before);
        if migrated {
            assert_eq!(
                fs::read(fixture.workspace().join("lance/CURRENT.json")).ok(),
                current_marker
            );
        }
    }
}

#[test]
fn concurrent_managed_reindex_replay_serializes_before_baseline_capture() {
    let fixture = Fixture::new(true);
    let before_reindex = audit_count(&fixture.history(), "reindex");
    fixture.write_note("NOTE-001", "changed once");
    let owner_lock = hold_index_owner_lock(&fixture.workspace());

    let first = fixture.command(&["reindex"]).spawn().unwrap();
    let workspace_lock = hold_workspace_lock_after_release(&fixture.workspace());
    let second = fixture.command(&["reindex"]).spawn().unwrap();
    FileExt::unlock(&workspace_lock).unwrap();
    drop(workspace_lock);
    FileExt::unlock(&owner_lock).unwrap();
    drop(owner_lock);

    for child in [first, second] {
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "reindex failed: stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert_eq!(
        audit_count(&fixture.history(), "reindex"),
        before_reindex + 1
    );
}

#[test]
fn git_sync_pins_head_across_diff_and_audit_and_refuses_uncommitted_input_bytes() {
    let fixture = Fixture::new(true);
    let baseline = fixture.commit_baseline();
    fixture.git(&["update-ref", "refs/heads/c4-before", &baseline]);
    fixture.write_note("NOTE-001", "pinned source update");
    fixture.git(&["add", "--", ".forgeplan/notes"]);
    fixture.git(&["commit", "-q", "-m", "pinned source update"]);
    let pinned_head = fixture.git(&["rev-parse", "HEAD"]);
    let pinned_short = fixture.git(&["rev-parse", "--short=7", &pinned_head]);

    fixture.write_note_at("NOTE-002", "NOTE-002", "draft", "only on moved HEAD");
    fixture.git(&["add", "--", ".forgeplan/notes"]);
    fixture.git(&["commit", "-q", "-m", "later source addition"]);
    let next_head = fixture.git(&["rev-parse", "HEAD"]);
    fixture.git(&["checkout", "--detach", &pinned_head]);
    let before = fixture.history();

    fixture.write_note("NOTE-001", "uncommitted input");
    let refused = fixture.run(&["git-sync", "--since", "refs/heads/c4-before"]);
    assert!(!refused.status.success());
    assert_eq!(fixture.history(), before);
    fixture.write_note("NOTE-001", "pinned source update");

    let wrapper = fixture._temp.path().join("git-wrapper");
    fs::create_dir_all(&wrapper).unwrap();
    let output_path = fixture._temp.path().join("git-output");
    let git_wrapper = wrapper.join("git");
    fs::write(
        &git_wrapper,
        "#!/bin/sh\n\
         \"$FORGEPLAN_TEST_REAL_GIT\" \"$@\" > \"$FORGEPLAN_TEST_GIT_OUTPUT\"\n\
         status=$?\n\
         if [ \"$1\" = diff ]; then\n\
         \t\"$FORGEPLAN_TEST_REAL_GIT\" update-ref HEAD \"$FORGEPLAN_TEST_NEXT_HEAD\"\n\
         \t\"$FORGEPLAN_TEST_REAL_GIT\" update-ref refs/heads/c4-before \"$FORGEPLAN_TEST_NEXT_HEAD\"\n\
         fi\n\
         cat \"$FORGEPLAN_TEST_GIT_OUTPUT\"\n\
         exit \"$status\"\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&git_wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let real_git = real_git_path();
    let path = std::env::join_paths(
        std::iter::once(wrapper.clone())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let output = fixture
        .command(&["git-sync", "--since", "refs/heads/c4-before"])
        .env("PATH", path)
        .env("FORGEPLAN_TEST_REAL_GIT", real_git)
        .env("FORGEPLAN_TEST_GIT_OUTPUT", output_path)
        .env("FORGEPLAN_TEST_NEXT_HEAD", &next_head)
        .output()
        .unwrap();
    assert!(output.status.success());

    let git_events = fixture
        .history()
        .into_iter()
        .filter(|entry| entry["source"] == "git_sync")
        .collect::<Vec<_>>();
    assert_eq!(git_events.len(), 1);
    assert_eq!(git_events[0]["artifact_id"], "NOTE-001");
    assert_eq!(git_events[0]["commit_hash"], pinned_short);
    assert_eq!(fixture.git(&["rev-parse", "HEAD"]), next_head);
    assert!(!fixture.note_path_for("NOTE-002").exists());
}
