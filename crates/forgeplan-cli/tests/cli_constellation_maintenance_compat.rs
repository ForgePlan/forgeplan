//! Ordinary watch/migrate argv must follow the index owner after cutover.
//! Watchers run only in isolated child processes and are reaped on every exit.

use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use assert_cmd::prelude::*;
use forgeplan_core::constellation::{
    FORMAT_VERSION,
    facade::{self, PinnedArtifact},
    lance_tables,
    manifest::BootstrapState,
    storage::{self, IndexProtocolState},
};
use tempfile::TempDir;

const SOURCE_PATH: &str = "prds/PRD-001-maintenance.md";
const INITIAL_BODY: &str = "maintenance_initial_sentinel";
const WATCH_TIMEOUT: Duration = Duration::from_secs(45);

struct Workspace {
    _temp: TempDir,
    root: PathBuf,
    home: PathBuf,
}

impl Workspace {
    fn legacy() -> Self {
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
        workspace.run_ok(&["init", "-y"]);
        workspace.write_body(INITIAL_BODY);
        let reindex = workspace.run_ok(&["reindex"]);
        assert!(!reindex.contains("create failed"), "{reindex}");
        assert!(workspace.lance().join("artifacts.lance").exists());
        assert!(!workspace.lance().join(storage::CURRENT_FILE_NAME).exists());
        workspace
    }

    fn migrated() -> Self {
        let workspace = Self::legacy();
        workspace.run_ok(&["index"]);
        assert!(matches!(
            storage::read_marker(&workspace.lance()).unwrap(),
            IndexProtocolState::Active(_)
        ));
        assert!(storage::read_current(&workspace.lance()).unwrap().is_some());
        for name in ["artifacts.lance", "relations.lance", "sections.lance"] {
            assert!(
                !workspace.lance().join(name).exists(),
                "{name} was not retired"
            );
        }
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
            .env("FORGEPLAN_MODEL_CACHE", self.home.join("models"))
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

    fn run_ok(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "forgeplan {args:?} failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn write_body(&self, body: &str) {
        let source = format!(
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Maintenance\n\
             slug: prd-maintenance\ndepth: standard\n---\n\n## Summary\n\n{body}\n"
        );
        fs::write(self.workspace().join(SOURCE_PATH), source).unwrap();
    }

    fn table_names(&self) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(self.lance())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".lance"))
            .collect();
        names.sort();
        names
    }

    fn assert_no_retired_tables_added(&self, before: &[String]) {
        // `init` creates evidence.lance; it is auxiliary and can predate the
        // cutover. Compare against that baseline instead of deleting it.
        for name in [
            "artifacts.lance",
            "relations.lance",
            "sections.lance",
            "evidence.lance",
        ] {
            assert!(
                !self.lance().join(name).exists() || before.iter().any(|old| old == name),
                "{name} was resurrected; before: {before:?}, after: {:?}",
                self.table_names()
            );
        }
    }

    async fn pinned(&self) -> (PinnedArtifact, String, u64) {
        facade::get_pinned_artifact(&self.lance(), "PRD-001")
            .await
            .unwrap()
            .expect("the published table must contain PRD-001, not just its Markdown")
    }
}

struct Watcher {
    child: Child,
    stdout: PathBuf,
    stderr: PathBuf,
}

impl Watcher {
    fn start(workspace: &Workspace, explicit_constellation: bool) -> Self {
        let stdout = workspace.root.join("watch.stdout");
        let stderr = workspace.root.join("watch.stderr");
        let args: &[&str] = if explicit_constellation {
            &["watch", "--constellation"]
        } else {
            &["watch"]
        };
        let child = workspace
            .command(args)
            .stdout(fs::File::create(&stdout).unwrap())
            .stderr(fs::File::create(&stderr).unwrap())
            .spawn()
            .unwrap();
        let mut watcher = Self {
            child,
            stdout,
            stderr,
        };
        watcher.wait_for_stdout_after(0, "Watching .forgeplan/ for changes");
        watcher
    }

    fn stdout(&self) -> String {
        fs::read_to_string(&self.stdout).unwrap()
    }

    fn diagnostics(&self) -> String {
        format!(
            "stdout:\n{}\nstderr:\n{}",
            self.stdout(),
            fs::read_to_string(&self.stderr).unwrap()
        )
    }

    fn assert_running(&mut self) {
        assert!(
            self.child.try_wait().unwrap().is_none(),
            "watch exited before the test stopped it: {}",
            self.diagnostics()
        );
    }

    fn wait_for_stdout_after(&mut self, offset: usize, expected: &str) {
        let deadline = Instant::now() + WATCH_TIMEOUT;
        loop {
            self.assert_running();
            if self
                .stdout()
                .get(offset..)
                .is_some_and(|text| text.contains(expected))
            {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "watch did not report {expected:?}: {}",
                self.diagnostics()
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    async fn wait_for_published_body(
        &mut self,
        workspace: &Workspace,
        expected: &str,
        previous_generation: &str,
    ) -> PinnedArtifact {
        let deadline = Instant::now() + WATCH_TIMEOUT;
        loop {
            self.assert_running();
            let (artifact, generation, _) = workspace.pinned().await;
            if artifact
                .body
                .as_deref()
                .is_some_and(|body| body.contains(expected))
                && generation != previous_generation
            {
                return artifact;
            }
            assert!(
                Instant::now() < deadline,
                "the watcher did not publish {expected:?}: {}",
                self.diagnostics()
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        // Kill and reap even when an assertion unwinds. No watcher can escape
        // this test into the developer's project or the next test case.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[tokio::test]
async fn default_watch_keeps_legacy_sync_and_observes_a_file_change() {
    let workspace = Workspace::legacy();
    let mut watcher = Watcher::start(&workspace, false);
    assert!(!watcher.stdout().contains("constellation index on"));
    let offset = watcher.stdout().len();
    workspace.write_body("maintenance_legacy_change_sentinel");
    watcher.wait_for_stdout_after(offset, "SYNC PRD-001");
    drop(watcher);

    // Raw store reads do not lazy-sync Markdown, unlike `forgeplan get`.
    // This checks the child's work rather than making the assertion repair it.
    let store = forgeplan_core::db::store::LanceStore::open(&workspace.workspace())
        .await
        .unwrap();
    let record = store.get_record("PRD-001").await.unwrap().unwrap();
    assert!(record.body.contains("maintenance_legacy_change_sentinel"));
    assert!(!workspace.lance().join(storage::CURRENT_FILE_NAME).exists());
    assert!(
        !workspace
            .lance()
            .join(storage::PROTOCOL_RELATIVE_PATH)
            .exists()
    );
}

#[tokio::test]
async fn default_watch_maintains_migrated_startup_and_file_changes_without_a_flag() {
    let workspace = Workspace::migrated();
    let before_tables = workspace.table_names();
    let (before, before_generation, _) = workspace.pinned().await;
    workspace.write_body("maintenance_startup_change_sentinel");
    assert!(before.body.as_deref().unwrap().contains(INITIAL_BODY));

    let mut watcher = Watcher::start(&workspace, false);
    let startup = watcher
        .wait_for_published_body(
            &workspace,
            "maintenance_startup_change_sentinel",
            &before_generation,
        )
        .await;
    assert_ne!(startup.source_digest, before.source_digest);
    assert!(watcher.stdout().contains("constellation index on"));

    let (_, startup_generation, _) = workspace.pinned().await;
    let offset = watcher.stdout().len();
    workspace.write_body("maintenance_watched_change_sentinel");
    let changed = watcher
        .wait_for_published_body(
            &workspace,
            "maintenance_watched_change_sentinel",
            &startup_generation,
        )
        .await;
    // INDEX is a signalled pass; SCAN would only prove the periodic fallback.
    watcher.wait_for_stdout_after(offset, "INDEX ");
    assert_ne!(changed.source_digest, startup.source_digest);
    workspace.assert_no_retired_tables_added(&before_tables);
    drop(watcher);
}

#[tokio::test]
async fn explicit_constellation_watch_still_migrates_and_observes_changes() {
    let workspace = Workspace::legacy();
    let mut watcher = Watcher::start(&workspace, true);
    assert!(!workspace.lance().join("artifacts.lance").exists());
    assert!(!workspace.lance().join("relations.lance").exists());
    assert!(
        workspace
            .lance()
            .join("artifacts.legacy-backup.lance")
            .exists()
    );
    let before_tables = workspace.table_names();
    let (_, generation, _) = workspace.pinned().await;
    let offset = watcher.stdout().len();

    workspace.write_body("maintenance_explicit_change_sentinel");
    watcher
        .wait_for_published_body(
            &workspace,
            "maintenance_explicit_change_sentinel",
            &generation,
        )
        .await;
    watcher.wait_for_stdout_after(offset, "INDEX ");
    workspace.assert_no_retired_tables_added(&before_tables);
    drop(watcher);
}

#[tokio::test]
async fn migrate_still_runs_real_legacy_schema_migrations_and_keeps_rows() {
    let workspace = Workspace::legacy();
    let change_log = workspace.lance().join("change_log.lance");
    assert!(change_log.exists());
    fs::remove_dir_all(&change_log).unwrap();

    let stdout = workspace.run_ok(&["migrate"]);
    assert!(stdout.contains("Running schema migrations..."));
    assert!(stdout.contains("Migrations complete. Schema up to date."));
    assert!(stdout.contains("forgeplan health"));
    assert!(
        change_log.exists(),
        "legacy migration did not recreate change_log"
    );
    assert!(!workspace.lance().join(storage::CURRENT_FILE_NAME).exists());
    let store = forgeplan_core::db::store::LanceStore::open(&workspace.workspace())
        .await
        .unwrap();
    assert!(
        store
            .get_record("PRD-001")
            .await
            .unwrap()
            .unwrap()
            .body
            .contains(INITIAL_BODY)
    );
    drop(store);
    workspace.run_ok(&["migrate"]);
}

#[tokio::test]
async fn migrate_verifies_the_supported_protocol_and_publishes_real_source_changes() {
    let workspace = Workspace::migrated();
    let before_tables = workspace.table_names();
    let (before, generation, _) = workspace.pinned().await;
    workspace.write_body("maintenance_migrate_change_sentinel");
    assert!(before.body.as_deref().unwrap().contains(INITIAL_BODY));

    let stdout = workspace.run_ok(&["migrate"]);
    assert!(stdout.contains("Migrations complete. Schema up to date."));
    assert!(stdout.contains("forgeplan health"));
    let (after, new_generation, _) = workspace.pinned().await;
    assert!(
        after
            .body
            .as_deref()
            .unwrap()
            .contains("maintenance_migrate_change_sentinel")
    );
    assert_ne!(
        generation, new_generation,
        "migrate was a no-op over stale source"
    );
    assert_ne!(before.source_digest, after.source_digest);

    let index_root = workspace.lance();
    let state = storage::read_marker(&index_root).unwrap();
    let marker = state.marker().unwrap();
    assert_eq!(marker.state, BootstrapState::Active);
    assert_eq!(marker.protocol_version, FORMAT_VERSION);
    let descriptor = storage::read_current(&index_root).unwrap().unwrap();
    assert_eq!(descriptor.format_version, FORMAT_VERSION);
    let manifest = storage::read_manifest(&index_root, &descriptor).unwrap();
    assert_eq!(manifest.schema_version, FORMAT_VERSION);
    manifest.validate_for_publication().unwrap();
    for (role, version) in [
        (
            "artifacts",
            descriptor.effective_artifacts_version(&manifest).unwrap(),
        ),
        ("relations", manifest.table_versions.relations),
        ("sections", manifest.table_versions.sections),
    ] {
        let table = lance_tables::open_table_at_version(&index_root, marker, role, version)
            .await
            .unwrap();
        assert_eq!(
            table.schema().await.unwrap(),
            lance_tables::schema_for_role(role).unwrap()
        );
    }
    workspace.assert_no_retired_tables_added(&before_tables);
    workspace.run_ok(&["migrate"]);
    workspace.assert_no_retired_tables_added(&before_tables);
}

fn assert_protocol_failure(workspace: &Workspace, expected: &str) {
    let before_tables = workspace.table_names();
    let current_path = workspace.lance().join(storage::CURRENT_FILE_NAME);
    let current = fs::read(&current_path).unwrap();
    for args in [
        &["migrate"][..],
        &["watch"][..],
        &["watch", "--constellation"][..],
    ] {
        let output = workspace.run(args);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{args:?}: {stdout}\n{stderr}"
        );
        assert!(stderr.contains(expected), "{args:?}: {stderr}");
        assert!(!stdout.contains("Schema up to date"), "{stdout}");
        assert!(!stdout.contains("Watching .forgeplan/"), "{stdout}");
        assert!(
            !stdout.contains("forgeplan health"),
            "failed migration emitted a success hint: {stdout}"
        );
        assert_eq!(fs::read(&current_path).unwrap(), current);
        workspace.assert_no_retired_tables_added(&before_tables);
    }
}

#[test]
fn corrupt_protocol_marker_is_not_a_healthy_legacy_workspace() {
    let workspace = Workspace::migrated();
    let marker = workspace.lance().join(storage::PROTOCOL_RELATIVE_PATH);
    let broken = b"{ not valid JSON";
    fs::write(&marker, broken).unwrap();
    assert_protocol_failure(&workspace, "index_protocol_mismatch");
    assert_eq!(fs::read(&marker).unwrap(), broken);
}

#[test]
fn unsupported_protocol_versions_are_refused_instead_of_claimed_up_to_date() {
    let workspace = Workspace::migrated();
    let path = workspace.lance().join(storage::PROTOCOL_RELATIVE_PATH);
    let mut marker: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for version in [FORMAT_VERSION - 1, FORMAT_VERSION + 1] {
        marker["protocol_version"] = serde_json::json!(version);
        let bytes = serde_json::to_vec(&marker).unwrap();
        fs::write(&path, &bytes).unwrap();
        assert_protocol_failure(&workspace, "index_protocol_mismatch");
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn corrupt_current_without_a_marker_never_falls_through_to_legacy_migrations() {
    let workspace = Workspace::migrated();
    fs::remove_file(workspace.lance().join(storage::PROTOCOL_RELATIVE_PATH)).unwrap();
    fs::write(
        workspace.lance().join(storage::CURRENT_FILE_NAME),
        b"broken CURRENT",
    )
    .unwrap();
    assert_protocol_failure(&workspace, "index_generation_unavailable");
}

#[test]
fn migrate_reports_incomplete_source_instead_of_a_successful_schema_check() {
    let workspace = Workspace::migrated();
    let before_tables = workspace.table_names();
    let current_path = workspace.lance().join(storage::CURRENT_FILE_NAME);
    let current = fs::read(&current_path).unwrap();
    fs::write(workspace.workspace().join(SOURCE_PATH), b"\xff\xfe").unwrap();

    let output = workspace.run(&["migrate"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stdout}\n{stderr}");
    assert!(
        stderr.contains("source_busy") || stderr.contains("source_unavailable"),
        "{stderr}"
    );
    assert!(!stdout.contains("Schema up to date"), "{stdout}");
    assert!(!stdout.contains("forgeplan health"), "{stdout}");
    assert_eq!(fs::read(&current_path).unwrap(), current);
    workspace.assert_no_retired_tables_added(&before_tables);
}
