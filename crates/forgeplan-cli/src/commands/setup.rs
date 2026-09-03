//! `forgeplan setup` — one-time per-machine preparation.
//!
//! Two things a fresh install cannot do for itself:
//!
//! 1. **The `fpl` alias.** cargo-dist creates it via `bin-aliases`, so brew and
//!    `install.sh` users get it for free. `cargo install` has no equivalent and
//!    no post-install hook, so anyone who built from source has `forgeplan` but
//!    no `fpl`. We create the symlink ourselves rather than shipping a second
//!    67 MB binary.
//!
//! 2. **The embedding model.** BGE-M3 is a ~2.1 GB first-use download. Doing it
//!    here, deliberately, beats discovering it mid-task when the first semantic
//!    search stalls for several minutes with no explanation.
//!
//! Both steps are idempotent and each can be skipped. Nothing here is required
//! for ForgePlan to work — without the model, search falls back to BM25; without
//! the alias, `forgeplan` still runs.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::ui;
use forgeplan_core::hints::{self, Hint};

/// Name of the short alias users expect from the brew install.
const ALIAS_NAME: &str = "fpl";

/// Outcome of the alias step, so the caller can report accurately rather than
/// claiming success for a no-op.
#[derive(Debug, PartialEq, Eq)]
pub enum AliasOutcome {
    /// Symlink created at the given path.
    Created(PathBuf),
    /// A correct alias was already in place — nothing to do.
    AlreadyCorrect(PathBuf),
    /// Something else occupies the path; we refuse to overwrite it.
    Occupied(PathBuf),
    /// Platform does not get a symlink (Windows); caller should print guidance.
    Unsupported,
}

/// Where the `fpl` alias belongs: next to the running executable.
///
/// Deriving it from `current_exe()` rather than guessing `~/.cargo/bin` means
/// the alias lands beside whichever binary the user actually invoked — cargo
/// install, a manual copy, or a checkout build — instead of pointing at a
/// different installation than the one they are running.
pub fn alias_path_for(exe: &Path) -> PathBuf {
    exe.with_file_name(ALIAS_NAME)
}

/// Create the `fpl` symlink beside the current executable.
///
/// Never overwrites an existing file. If something is already at that path and
/// it is not our symlink, we report `Occupied` and leave it — silently
/// replacing a binary a user put there themselves would be indefensible.
#[cfg(unix)]
pub fn ensure_alias() -> Result<AliasOutcome> {
    let exe = std::env::current_exe().context("cannot determine the running executable's path")?;
    let alias = alias_path_for(&exe);

    if alias.exists() || alias.symlink_metadata().is_ok() {
        // `read_link` succeeds only for symlinks; comparing targets tells us
        // whether this is our own alias from a previous run.
        return match std::fs::read_link(&alias) {
            Ok(target) if target == exe => Ok(AliasOutcome::AlreadyCorrect(alias)),
            _ => Ok(AliasOutcome::Occupied(alias)),
        };
    }

    std::os::unix::fs::symlink(&exe, &alias)
        .with_context(|| format!("failed to create the alias at {}", alias.display()))?;

    Ok(AliasOutcome::Created(alias))
}

/// Windows has no dependable unprivileged symlink, so we do not pretend.
#[cfg(not(unix))]
pub fn ensure_alias() -> Result<AliasOutcome> {
    Ok(AliasOutcome::Unsupported)
}

/// Download the embedding model so the first real search does not stall.
///
/// Constructing the embedder is what triggers the fetch; fastembed prints its
/// own progress bar. Returns `Ok(false)` when the build has no embedding
/// support, so the caller can say so instead of reporting a phantom success.
#[cfg(feature = "semantic-search")]
pub fn warm_model() -> Result<bool> {
    use forgeplan_core::embed::Embedder;

    if let Some(notice) = forgeplan_core::embed::first_run_notice() {
        ui::info(&notice);
    }

    // The handle is dropped immediately — we want the download, not the model.
    let _ = Embedder::new()?;
    Ok(true)
}

#[cfg(not(feature = "semantic-search"))]
pub fn warm_model() -> Result<bool> {
    Ok(false)
}

/// Whether this build can do semantic search at all.
pub const fn has_semantic_search() -> bool {
    cfg!(feature = "semantic-search")
}

/// Report the alias outcome in the user's terms, including what to do about it.
fn report_alias(outcome: &AliasOutcome) {
    match outcome {
        AliasOutcome::Created(path) => {
            ui::success(&format!("Alias created: {} -> forgeplan", path.display()));
        }
        AliasOutcome::AlreadyCorrect(path) => {
            ui::info(&format!("Alias already in place: {}", path.display()));
        }
        AliasOutcome::Occupied(path) => {
            ui::warning(&format!(
                "Not touching {} — something is already there. Remove it first if you want the alias.",
                path.display()
            ));
        }
        AliasOutcome::Unsupported => {
            ui::info(
                "Symlink aliases are not created on this platform. Add a `fpl` \
                 shim to PATH by hand if you want the short form.",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Git hooks — keep the derived index in step with the working tree (PROB-097)
// ---------------------------------------------------------------------------

/// Marker written into every hook we generate, so a later run can tell our
/// file from one the user wrote and refuse to clobber theirs.
const HOOK_MARKER: &str = "# managed by `forgeplan setup --hooks`";

/// Outcome per hook, so the report states what happened instead of implying
/// success for a file we declined to touch.
#[derive(Debug, PartialEq, Eq)]
pub enum HookOutcome {
    Installed(PathBuf),
    AlreadyCurrent(PathBuf),
    /// A hook exists that we did not write. Never overwritten — the user's
    /// automation outranks ours.
    Foreign(PathBuf),
    /// Not a git repository, so there is nowhere to install.
    NotAGitRepo,
}

/// `post-merge` fires after `git pull`. Git sets `ORIG_HEAD` for merges, which
/// is exactly the reference `git-sync` defaults to, so no argument is needed.
///
/// `post-checkout` fires on branch switches and receives the previous HEAD as
/// `$1` — passed through as `--since`, because `ORIG_HEAD` is not set for a
/// checkout and the command would otherwise refuse.
fn hook_body(kind: &str) -> String {
    let since = if kind == "post-checkout" {
        // $3 == 1 means a branch switch; 0 means a file checkout, which does
        // not move HEAD and needs no sync.
        "  [ \"$3\" = \"1\" ] || exit 0\n  [ \"$1\" = \"$2\" ] && exit 0\n  SINCE=\"--since $1\"\n"
    } else {
        "  SINCE=\"\"\n"
    };

    format!(
        "#!/bin/sh\n\
         {HOOK_MARKER}\n\
         #\n\
         # Keeps the LanceDB index in step with the markdown that git just\n\
         # changed. Without it, search answers from the state before the pull —\n\
         # silently, including semantic results computed from text that is no\n\
         # longer in the artifact (PROB-097).\n\
         #\n\
         # Deliberately does NOT run `forgeplan embed`: that needs the 2.1 GB\n\
         # model and seconds per artifact. `search` reports how many artifacts\n\
         # still lack a vector, which is the right place to decide.\n\
         #\n\
         # Remove this file to opt out; `forgeplan setup --hooks` restores it.\n\
         \n\
         # No workspace here, or no forgeplan on PATH: stay out of the way.\n\
         [ -d .forgeplan ] || exit 0\n\
         command -v forgeplan >/dev/null 2>&1 || exit 0\n\
         \n\
         sync_index() {{\n\
         {since}\
         \x20 # Never fail the git operation. A stale index is recoverable with\n\
         \x20 # `forgeplan reindex`; a git command that errors after a\n\
         \x20 # successful merge is confusing and worse.\n\
         \x20 forgeplan git-sync $SINCE >/dev/null 2>&1 || true\n\
         }}\n\
         \n\
         sync_index\n"
    )
}

/// Directory git actually reads hooks from.
///
/// `.git` is a *file* inside a worktree, and `core.hooksPath` may redirect
/// entirely — asking git beats assuming `.git/hooks`, which would install into
/// a path git never reads and report success.
fn hooks_dir(repo: &Path) -> Option<PathBuf> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--git-path", "hooks"])
        .current_dir(repo)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let rel = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if rel.is_empty() {
        return None;
    }
    let p = PathBuf::from(&rel);
    Some(if p.is_absolute() { p } else { repo.join(p) })
}

/// Install (or refresh) one hook. Never overwrites a file we did not write.
pub fn install_hook(repo: &Path, kind: &str) -> Result<HookOutcome> {
    let Some(dir) = hooks_dir(repo) else {
        return Ok(HookOutcome::NotAGitRepo);
    };
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("cannot create hooks directory {}", dir.display()))?;

    let path = dir.join(kind);
    let body = hook_body(kind);

    if path.exists() {
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        if !existing.contains(HOOK_MARKER) {
            return Ok(HookOutcome::Foreign(path));
        }
        if existing == body {
            return Ok(HookOutcome::AlreadyCurrent(path));
        }
    }

    std::fs::write(&path, &body)
        .with_context(|| format!("cannot write hook {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms)?;
    }

    Ok(HookOutcome::Installed(path))
}

/// Install both hooks that move HEAD: `git pull` and branch switches.
pub fn install_git_hooks(repo: &Path) -> Result<Vec<HookOutcome>> {
    ["post-merge", "post-checkout"]
        .iter()
        .map(|k| install_hook(repo, k))
        .collect()
}

fn report_hook(outcome: &HookOutcome) {
    match outcome {
        HookOutcome::Installed(p) => {
            ui::success(&format!("Installed {}", p.display()));
        }
        HookOutcome::AlreadyCurrent(p) => {
            ui::info(&format!("Already current: {}", p.display()));
        }
        HookOutcome::Foreign(p) => {
            ui::warning(&format!(
                "{} already exists and was not written by forgeplan — left alone.",
                p.display()
            ));
            ui::info(
                "Add `forgeplan git-sync >/dev/null 2>&1 || true` to it yourself, \
                 or the index will lag behind git.",
            );
        }
        HookOutcome::NotAGitRepo => {
            ui::warning("Not a git repository — nothing to hook into.");
        }
    }
}

/// `forgeplan setup` entry point.
pub async fn run(skip_model: bool, skip_alias: bool, skip_hooks: bool) -> Result<()> {
    ui::header("forgeplan setup", "one-time per-machine preparation");

    if skip_alias {
        ui::info("Skipping the alias step (--skip-alias).");
    } else {
        report_alias(&ensure_alias()?);
    }

    // PROB-097: `git pull` rewrites the markdown and leaves the derived index
    // describing the state before it — silently, including semantic hits
    // computed from text the artifact no longer contains. `git-sync` fixes
    // that and takes ~1.4s on a 400-artifact workspace, which is cheap enough
    // to spend automatically rather than rely on everyone remembering it.
    if skip_hooks {
        ui::info("Skipping git hooks (--skip-hooks).");
    } else {
        let cwd = std::env::current_dir()?;
        for outcome in install_git_hooks(&cwd)? {
            report_hook(&outcome);
        }
    }

    if skip_model {
        ui::info("Skipping the model download (--skip-model).");
    } else if !has_semantic_search() {
        ui::warning(
            "This build has no semantic-search feature, so there is no model to \
             download. Keyword search works regardless.",
        );
        ui::info(
            "To get vector search: cargo install --git \
             https://github.com/ForgePlan/forgeplan --features semantic-search",
        );
    } else {
        match warm_model() {
            Ok(true) => ui::success("Embedding model ready."),
            // Unreachable while has_semantic_search() gates this branch, but a
            // future refactor could change that; better a plain message than a
            // false claim of success.
            Ok(false) => ui::warning("Embedding support is not compiled into this build."),
            Err(e) => {
                // A failed download is not a failed setup — the alias may well
                // have been created, and everything except semantic search
                // still works. Report and carry on rather than aborting.
                ui::warning(&format!("Could not prepare the embedding model: {e}"));
            }
        }
    }

    let hint_list = vec![Hint::info("Start a workspace").with_action("forgeplan init".to_string())];
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alias_lands_next_to_the_executable_not_in_a_guessed_directory() {
        // Pinning this matters: guessing ~/.cargo/bin would alias a different
        // installation than the one the user is running.
        let exe = Path::new("/opt/somewhere/bin/forgeplan");
        assert_eq!(alias_path_for(exe), PathBuf::from("/opt/somewhere/bin/fpl"),);
    }

    #[test]
    fn alias_path_keeps_the_directory_of_an_unusual_install_location() {
        let exe = Path::new("/Users/someone/.local/bin/forgeplan");
        assert_eq!(
            alias_path_for(exe),
            PathBuf::from("/Users/someone/.local/bin/fpl")
        );
    }

    #[cfg(unix)]
    #[test]
    fn existing_foreign_file_is_reported_occupied_never_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("forgeplan");
        std::fs::write(&exe, b"binary").unwrap();

        let alias = alias_path_for(&exe);
        std::fs::write(&alias, b"someone else's tool").unwrap();

        // Re-implements the decision `ensure_alias` makes, without depending on
        // current_exe() which a test cannot control.
        let outcome = match std::fs::read_link(&alias) {
            Ok(target) if target == exe => AliasOutcome::AlreadyCorrect(alias.clone()),
            _ => AliasOutcome::Occupied(alias.clone()),
        };

        assert_eq!(outcome, AliasOutcome::Occupied(alias.clone()));
        // The point of the test: the foreign file survives untouched.
        assert_eq!(std::fs::read(&alias).unwrap(), b"someone else's tool");
    }

    #[cfg(unix)]
    #[test]
    fn our_own_symlink_is_recognised_as_already_correct() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("forgeplan");
        std::fs::write(&exe, b"binary").unwrap();

        let alias = alias_path_for(&exe);
        std::os::unix::fs::symlink(&exe, &alias).unwrap();

        let outcome = match std::fs::read_link(&alias) {
            Ok(target) if target == exe => AliasOutcome::AlreadyCorrect(alias.clone()),
            _ => AliasOutcome::Occupied(alias.clone()),
        };

        assert_eq!(outcome, AliasOutcome::AlreadyCorrect(alias));
    }

    #[test]
    fn semantic_support_matches_the_compiled_feature_set() {
        assert_eq!(has_semantic_search(), cfg!(feature = "semantic-search"));
    }

    // --- git hooks (PROB-097) ---

    fn git_repo(dir: &Path) {
        for args in [
            vec!["init", "-q"],
            vec!["config", "user.email", "t@example.com"],
            vec!["config", "user.name", "t"],
        ] {
            std::process::Command::new("git")
                .args(&args)
                .current_dir(dir)
                .output()
                .expect("git available in test env");
        }
    }

    #[test]
    fn installs_both_hooks_that_move_head() {
        let tmp = tempfile::tempdir().unwrap();
        git_repo(tmp.path());

        let outcomes = install_git_hooks(tmp.path()).unwrap();
        assert_eq!(outcomes.len(), 2);
        for o in &outcomes {
            assert!(
                matches!(o, HookOutcome::Installed(_)),
                "expected a fresh install, got {o:?}"
            );
        }
        for kind in ["post-merge", "post-checkout"] {
            let p = tmp.path().join(".git/hooks").join(kind);
            assert!(p.exists(), "{kind} not written");
        }
    }

    /// Re-running setup must not churn the file — otherwise every run looks
    /// like a change and the report cannot be trusted.
    #[test]
    fn reinstalling_is_a_no_op() {
        let tmp = tempfile::tempdir().unwrap();
        git_repo(tmp.path());

        install_git_hooks(tmp.path()).unwrap();
        let second = install_git_hooks(tmp.path()).unwrap();
        for o in &second {
            assert!(
                matches!(o, HookOutcome::AlreadyCurrent(_)),
                "second run must be a no-op, got {o:?}"
            );
        }
    }

    /// The user's own automation outranks ours. Silently replacing a hook
    /// someone wrote would be the worst possible way to be helpful.
    #[test]
    fn refuses_to_overwrite_a_hook_we_did_not_write() {
        let tmp = tempfile::tempdir().unwrap();
        git_repo(tmp.path());
        let dir = tmp.path().join(".git/hooks");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("post-merge");
        std::fs::write(&path, "#!/bin/sh\necho mine\n").unwrap();

        let outcome = install_hook(tmp.path(), "post-merge").unwrap();
        assert!(
            matches!(outcome, HookOutcome::Foreign(_)),
            "a foreign hook must be reported, not replaced: {outcome:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "#!/bin/sh\necho mine\n",
            "the user's hook must survive untouched"
        );
    }

    #[test]
    fn outside_a_git_repo_it_reports_rather_than_failing() {
        let tmp = tempfile::tempdir().unwrap();
        let outcome = install_hook(tmp.path(), "post-merge").unwrap();
        assert_eq!(outcome, HookOutcome::NotAGitRepo);
    }

    /// `post-checkout` has no ORIG_HEAD, so it must pass the previous HEAD
    /// explicitly or `git-sync` refuses. `post-merge` must NOT — git sets
    /// ORIG_HEAD there and $1 means something else entirely.
    #[test]
    fn checkout_passes_the_previous_head_and_merge_does_not() {
        let checkout = hook_body("post-checkout");
        assert!(
            checkout.contains("--since $1"),
            "post-checkout must pass the previous HEAD: git-sync has no \
             ORIG_HEAD to fall back on after a branch switch"
        );
        assert!(
            checkout.contains("\"$3\" = \"1\""),
            "a file checkout does not move HEAD and must be skipped"
        );

        let merge = hook_body("post-merge");
        assert!(
            !merge.contains("--since $1"),
            "post-merge gets ORIG_HEAD from git; $1 is the squash flag there"
        );
    }

    /// A hook that can fail the git command is worse than a stale index —
    /// the merge already succeeded, and erroring after it only confuses.
    #[test]
    fn hooks_never_fail_the_git_operation() {
        for kind in ["post-merge", "post-checkout"] {
            let body = hook_body(kind);
            assert!(
                body.contains("|| true"),
                "{kind} must swallow git-sync failures"
            );
            assert!(
                body.contains("[ -d .forgeplan ] || exit 0"),
                "{kind} must stay out of the way where there is no workspace"
            );
            assert!(
                body.contains("command -v forgeplan"),
                "{kind} must not break repos where forgeplan is not installed"
            );
        }
    }

    /// `embed` needs the 2.1 GB model and seconds per artifact. Doing it from
    /// a git hook would make every pull unpredictable.
    #[test]
    fn hooks_do_not_run_embed() {
        for kind in ["post-merge", "post-checkout"] {
            let body = hook_body(kind);
            // Check for an INVOCATION, not a mention: the hook's own comment
            // explains why it does not run embed, and a naive substring match
            // fails on the explanation rather than on the behaviour.
            let invocations: Vec<&str> = body
                .lines()
                .filter(|l| !l.trim_start().starts_with('#'))
                .filter(|l| l.contains("forgeplan embed"))
                .collect();
            assert!(
                invocations.is_empty(),
                "{kind} must not run embed — search reports the gap instead. Found: {invocations:?}"
            );
        }
    }
}
