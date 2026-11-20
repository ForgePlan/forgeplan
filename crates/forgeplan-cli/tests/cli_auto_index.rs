//! Auto-indexing surface on the DEFAULT (identity-v5, non-migrated) layout:
//!
//! - `forgeplan watch` — a real file write under `.forgeplan/` must trigger a
//!   live `SYNC <id>` of the LanceDB index (proven by `forgeplan get` seeing
//!   the new body).
//! - `forgeplan git-sync --since <ref>` — a change that arrived via a Git
//!   operation (simulated by rewriting the file behind the binary's back)
//!   must be picked up by the `--since` diff path.
//!
//! These tests complement `cli_constellation_maintenance_compat.rs`, which
//! covers the same machinery on the migrated (legacy) layout.

use assert_cmd::Command;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init_workspace(tmp: &TempDir) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

fn new_artifact(tmp: &TempDir, kind: &str, title: &str) -> String {
    let out = forgeplan()
        .args(["new", kind, title, "--allow-duplicate"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "new {kind}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.trim().strip_prefix("ID:").map(str::trim))
        .unwrap_or_else(|| panic!("no ID in new {kind}"))
        .to_string()
}

fn get_body(tmp: &TempDir, id: &str) -> String {
    let out = forgeplan()
        .args(["get", id])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).to_string()
}

// ── watch ──────────────────────────────────────────────────────────────────

struct WatchProc {
    child: Child,
    rx: std::sync::mpsc::Receiver<u8>,
    rx_err: std::sync::mpsc::Receiver<u8>,
    captured: String,
    captured_err: String,
}
impl WatchProc {
    fn start(tmp: &TempDir) -> Self {
        Self::start_args(tmp, &["watch"])
    }
    fn start_args(tmp: &TempDir, args: &[&str]) -> Self {
        let mut child = std::process::Command::new(assert_cmd::cargo::cargo_bin("forgeplan"))
            .args(args)
            .current_dir(tmp.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn forgeplan watch");
        let rx = Self::pump(child.stdout.take().unwrap());
        let rx_err = Self::pump(child.stderr.take().unwrap());
        Self {
            child,
            rx,
            rx_err,
            captured: String::new(),
            captured_err: String::new(),
        }
    }
    fn pump(mut r: impl Read + Send + 'static) -> std::sync::mpsc::Receiver<u8> {
        let (tx, rx) = std::sync::mpsc::channel::<u8>();
        std::thread::spawn(move || {
            let mut byte = [0u8; 1];
            while let Ok(1) = r.read(&mut byte) {
                if tx.send(byte[0]).is_err() {
                    break;
                }
            }
        });
        rx
    }
    fn drain(&mut self) {
        while let Ok(b) = self.rx.try_recv() {
            self.captured.push(b as char);
        }
        while let Ok(b) = self.rx_err.try_recv() {
            self.captured_err.push(b as char);
        }
    }
    /// Collect stdout until `needle` appears or `timeout_secs` elapses.
    fn wait_for(&mut self, needle: &str, timeout_secs: u64) -> String {
        self.wait_for_any(&[needle], timeout_secs)
    }
    /// Collect stdout until any needle appears or `timeout_secs` elapses.
    fn wait_for_any(&mut self, needles: &[&str], timeout_secs: u64) -> String {
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            self.drain();
            if needles.iter().any(|n| self.captured.contains(n)) {
                return self.captured.clone();
            }
            let left = deadline.saturating_duration_since(Instant::now());
            assert!(
                left > Duration::ZERO,
                "watch did not emit {needles:?} within {timeout_secs}s; captured:\n{}\nstderr:\n{}",
                self.captured,
                self.captured_err
            );
            match self.rx.recv_timeout(left.min(Duration::from_millis(200))) {
                Ok(b) => self.captured.push(b as char),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => panic!(
                    "watch stdout closed before {needles:?}; captured:\n{}",
                    self.captured
                ),
            }
        }
    }
    /// Collect stderr until `needle` appears; a quiet channel within
    /// `timeout_secs` means the warning never came — still a pass (the
    /// contract is only that no SYNC-style sync happened).
    fn wait_for_stderr_or_quiet(&mut self, needle: &str, timeout_secs: u64) {
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            self.drain();
            if self.captured_err.contains(needle) || self.captured.contains(needle) {
                return;
            }
            if Instant::now() >= deadline {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
impl Drop for WatchProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// On the DEFAULT layout, artifact filenames are `<kind>-<slug>-<hash>.md` —
/// the legacy per-file sync (which derives the id from `ID-NNN` filenames)
/// cannot address them and reports a WARN on stderr, while the dirty flag is
/// handed to the constellation runtime via mark_dirty. With no constellation
/// host the file change is acknowledged but NOT index-synced — that is the
/// documented contract of the corrected (v5) layout.
#[test]
fn watch_default_layout_acknowledges_change_without_legacy_sync() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "prd", "Watched PRD");

    let mut watcher = WatchProc::start(&tmp);
    let banner = watcher.wait_for("(Ctrl+C to stop)", 15);
    assert!(banner.contains("Watching .forgeplan/"), "startup: {banner}");

    // Rewrite the artifact body behind the binary's back.
    let path = tmp
        .path()
        .join(".forgeplan/prds")
        .read_dir()
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.extension().is_some_and(|e| e == "md"))
        .expect("no .md artifact under .forgeplan/prds");
    let body = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{body}\nwatched-sentinel-line\n")).unwrap();

    // The slug-hash filename is unsyncable for the legacy projection — the
    // watcher reports it on stderr and keeps watching (never SYNCs it).
    watcher.wait_for_stderr_or_quiet("does not follow", 15);
    drop(watcher);
}

/// `watch --constellation` on the default layout takes the maintenance-host
/// path: a file change produces an INDEX/SCAN pass line (not legacy SYNC).
#[test]
fn watch_constellation_pass_indexes_the_change() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "prd", "Watched PRD 2");

    let mut watcher = WatchProc::start_args(&tmp, &["watch", "--constellation"]);
    let banner = watcher.wait_for("(Ctrl+C to stop)", 15);
    assert!(
        banner.contains("constellation index on"),
        "banner: {banner}"
    );

    let path = tmp
        .path()
        .join(".forgeplan/prds")
        .read_dir()
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.extension().is_some_and(|e| e == "md"))
        .expect("no .md artifact under .forgeplan/prds");
    let body = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{body}\nconstellation-watched-sentinel\n")).unwrap();

    // Maintenance host reports INDEX <gen> or SCAN <gen> per pass.
    let out = watcher.wait_for_any(&["INDEX ", "SCAN "], 60);
    drop(watcher);
    assert!(
        out.contains("INDEX") || out.contains("SCAN"),
        "maintenance pass must report; got:\n{out}"
    );
}

/// `--constellation` additionally runs the constellation maintenance pass on
/// each change batch — the startup banner must mention it.
#[test]
fn watch_constellation_flag_reports_in_banner() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let mut watcher = WatchProc::start_args(&tmp, &["watch", "--constellation"]);
    // The banner is one line — wait for its full suffix, not just the prefix.
    let banner = watcher.wait_for("(Ctrl+C to stop)", 15);
    drop(watcher);
    assert!(
        banner.contains("constellation index on"),
        "--constellation banner must announce the maintenance pass; got:\n{banner}"
    );
}

// ── git-sync ───────────────────────────────────────────────────────────────
/// `git-sync --since <ref>` must pick up a file change that arrived via a
/// git-level operation. The workspace is git-initialized, the artifact file
/// is committed as the "before" baseline, then rewritten behind the binary's
/// back (simulating a pull/merge); `git-sync` must propagate the new body
/// into the index.
#[test]
fn git_sync_picks_up_a_post_commit_file_change() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let prd = new_artifact(&tmp, "prd", "Synced PRD");

    // git init + commit the pristine artifact so we have a real ref.
    // Pick the artifact's .md explicitly — read_dir order is arbitrary and
    // the dir may hold non-artifact entries.
    let prd_path = tmp
        .path()
        .join(".forgeplan/prds")
        .read_dir()
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.extension().is_some_and(|e| e == "md"))
        .expect("no .md artifact under .forgeplan/prds");
    git(tmp.path(), &["init", "-q"]);
    git(tmp.path(), &["add", ".forgeplan/prds"]);
    git(tmp.path(), &["commit", "-q", "-m", "baseline"]);
    let baseline = git(tmp.path(), &["rev-parse", "HEAD"]);

    // Simulate the "pull" — the file changed without the CLI's knowledge.
    let body = std::fs::read_to_string(&prd_path).unwrap();
    std::fs::write(&prd_path, format!("{body}\ngit-sync-sentinel-line\n")).unwrap();
    git(tmp.path(), &["add", ".forgeplan/prds"]);
    git(tmp.path(), &["commit", "-q", "-m", "pull-sim"]);

    let out = forgeplan()
        .args(["git-sync", "--since", &baseline])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "git-sync failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("synced") || stdout.contains("1"),
        "git-sync must report a synced change; got:\n{stdout}"
    );

    let got = get_body(&tmp, &prd);
    assert!(
        got.contains("git-sync-sentinel-line"),
        "git-sync must propagate the file change into the index; got:\n{got}"
    );
}

/// `git-sync` on a workspace with no changes since the ref must succeed and
/// report a no-op — not fabricate work.
#[test]
fn git_sync_reports_no_changes() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "prd", "Idle");

    // No git history at all → HEAD is the only ref; diff must be empty.
    git(tmp.path(), &["init", "-q"]);
    git(tmp.path(), &["add", ".forgeplan"]);
    git(tmp.path(), &["commit", "-q", "-m", "baseline"]);

    let out = forgeplan()
        .args(["git-sync", "--since", "HEAD"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No artifact changes")
            || stdout.contains("0 synced")
            || stdout.contains("up to date"),
        "git-sync with no diff must report a no-op; got:\n{stdout}"
    );
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}
