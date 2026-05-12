//! PROB-068 regression tests — `forgeplan init --force` data-loss vectors.
//!
//! These tests exercise the three guarantees added by Wave 6B:
//!
//! 1. **Option A**: `init --force` is strictly additive — existing artifact
//!    `.md` bodies are preserved through a refresh.
//! 2. **Option B**: `scan-import` round-trip preserves the `links:` block
//!    and the `author:` field instead of overwriting them with
//!    `scan-import` defaults.
//! 3. **Option C**: `init --force` auto-creates a
//!    `.forgeplan-backup-<timestamp>/` snapshot unless `--no-backup` is
//!    passed. Backups protect against future regressions in the additive
//!    path.

use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Create a minimal but realistic workspace populated with one PRD that
/// carries body content, a non-default `author:`, and a `links:` block.
fn populate_workspace(root: &std::path::Path) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(root)
        .assert()
        .success();

    // Write a PRD directly to disk that exercises every preservation
    // path we care about (body + author + links + custom field).
    let prd_dir = root.join(".forgeplan/prds");
    fs::create_dir_all(&prd_dir).unwrap();
    fs::write(
        prd_dir.join("PRD-099-data-loss-canary.md"),
        "---\n\
         id: PRD-099\n\
         kind: prd\n\
         title: Data Loss Canary\n\
         status: draft\n\
         depth: standard\n\
         author: human-author\n\
         tags:\n  - source=test\n\
         links:\n  - target: PROB-068\n    relation: informs\n  - target: ADR-003\n    relation: refines\n\
         custom_owner: explosivebit\n\
         ---\n\
         # Data Loss Canary\n\
         \n\
         ## Problem\n\
         \n\
         If init --force or scan-import drops this paragraph, PROB-068 has \
         regressed and we need to re-investigate the union-merge path.\n",
    )
    .unwrap();
}

#[test]
fn init_force_preserves_existing_artifact_bodies() {
    let tmp = TempDir::new().unwrap();
    populate_workspace(tmp.path());

    let prd_path = tmp
        .path()
        .join(".forgeplan/prds/PRD-099-data-loss-canary.md");
    let before = fs::read_to_string(&prd_path).unwrap();
    assert!(before.contains("Data Loss Canary"));
    assert!(before.contains("If init --force or scan-import drops"));

    // Run init --force (interactive prompt disabled via -y).
    forgeplan()
        .args(["init", "-y", "--force", "--no-backup"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // The artifact file must still exist with body + frontmatter intact.
    assert!(
        prd_path.exists(),
        "AC-1 regression: PRD-099 file deleted by init --force"
    );
    let after = fs::read_to_string(&prd_path).unwrap();
    assert!(
        after.contains("If init --force or scan-import drops"),
        "AC-1 regression: PRD-099 body wiped — got:\n{after}"
    );
    assert!(
        after.contains("author: human-author"),
        "AC-1 regression: PRD-099 author overwritten — got:\n{after}"
    );
    assert!(
        after.contains("custom_owner: explosivebit"),
        "AC-1 regression: PRD-099 custom frontmatter field dropped"
    );
}

#[test]
fn init_force_auto_backups_existing_artifacts() {
    let tmp = TempDir::new().unwrap();
    populate_workspace(tmp.path());

    forgeplan()
        .args(["init", "-y", "--force"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Expect at least one `.forgeplan-backup-*` directory containing the
    // PRD file we created.
    let mut backup_dir: Option<std::path::PathBuf> = None;
    for entry in fs::read_dir(tmp.path()).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(".forgeplan-backup-") {
            backup_dir = Some(entry.path());
            break;
        }
    }
    let backup_dir = backup_dir.expect(
        "AC-5 regression: --force without --no-backup created no .forgeplan-backup-* directory",
    );
    let backed_up_prd = backup_dir.join("prds/PRD-099-data-loss-canary.md");
    assert!(
        backed_up_prd.exists(),
        "AC-5 regression: PRD-099 missing from backup at {}",
        backed_up_prd.display()
    );
    let body = fs::read_to_string(&backed_up_prd).unwrap();
    assert!(body.contains("If init --force or scan-import drops"));
}

#[test]
fn init_force_no_backup_flag_skips_backup() {
    let tmp = TempDir::new().unwrap();
    populate_workspace(tmp.path());

    forgeplan()
        .args(["init", "-y", "--force", "--no-backup"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let any_backup = fs::read_dir(tmp.path()).unwrap().any(|e| {
        let e = e.unwrap();
        e.file_name()
            .to_string_lossy()
            .starts_with(".forgeplan-backup-")
    });
    assert!(
        !any_backup,
        "AC-5 regression: --no-backup did not suppress auto-backup"
    );
}

#[test]
fn scan_import_preserves_links_section() {
    // Brownfield-style scenario: an external markdown file under `docs/`
    // carries a `links:` block. After scan-import the projection in
    // `.forgeplan/<kind>s/` must still contain the same `target:`/`relation:`
    // entries — not an empty/missing `links:` field.
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let docs = tmp.path().join("docs");
    fs::create_dir_all(&docs).unwrap();
    fs::write(
        docs.join("PRD-150-feature.md"),
        "---\n\
         id: PRD-150\n\
         kind: prd\n\
         title: Feature\n\
         status: active\n\
         links:\n  - target: ADR-007\n    relation: refines\n  - target: EVID-099\n    relation: informs\n\
         ---\n\
         # Feature\n\
         \n\
         Body content that must survive the round-trip.\n",
    )
    .unwrap();

    forgeplan()
        .args(["scan-import"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Locate the projection file.
    let prds_dir = tmp.path().join(".forgeplan/prds");
    let mut projection: Option<std::path::PathBuf> = None;
    for entry in fs::read_dir(&prds_dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("PRD-150-") {
            projection = Some(entry.path());
            break;
        }
    }
    let projection = projection.expect("scan-import did not create a projection for PRD-150");
    let content = fs::read_to_string(&projection).unwrap();
    assert!(
        content.contains("ADR-007") && content.contains("refines"),
        "PROB-068 Option B regression: links block dropped — got:\n{content}"
    );
    assert!(
        content.contains("EVID-099") && content.contains("informs"),
        "PROB-068 Option B regression: second link dropped — got:\n{content}"
    );
    assert!(
        content.contains("Body content that must survive"),
        "PROB-068 body regression on scan-import"
    );
}

#[test]
fn scan_import_preserves_author_field() {
    // Source frontmatter declares `author: human-author`; scan-import must
    // not overwrite it with the default `scan-import` marker.
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let docs = tmp.path().join("docs");
    fs::create_dir_all(&docs).unwrap();
    fs::write(
        docs.join("ADR-099-original-author.md"),
        "---\n\
         id: ADR-099\n\
         kind: adr\n\
         title: Author Preservation\n\
         status: accepted\n\
         author: human-author\n\
         ---\n\
         # Original Author\n\
         \n\
         Body.\n",
    )
    .unwrap();

    forgeplan()
        .args(["scan-import"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let adrs_dir = tmp.path().join(".forgeplan/adrs");
    let mut projection: Option<std::path::PathBuf> = None;
    for entry in fs::read_dir(&adrs_dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("ADR-099-") {
            projection = Some(entry.path());
            break;
        }
    }
    let projection = projection.expect("scan-import did not create a projection for ADR-099");
    let content = fs::read_to_string(&projection).unwrap();
    assert!(
        content.contains("author: human-author"),
        "PROB-068 Option B regression: author overwritten — got:\n{content}"
    );
    assert!(
        !content.contains("author: scan-import"),
        "PROB-068 Option B regression: default scan-import author leaked despite source having one"
    );
}
