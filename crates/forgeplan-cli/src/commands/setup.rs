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

/// `forgeplan setup` entry point.
pub async fn run(skip_model: bool, skip_alias: bool) -> Result<()> {
    ui::header("forgeplan setup", "one-time per-machine preparation");

    if skip_alias {
        ui::info("Skipping the alias step (--skip-alias).");
    } else {
        report_alias(&ensure_alias()?);
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
}
