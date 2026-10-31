use assert_cmd::Command;
use std::path::Path;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn workspace_with_documents(documents: &[(&str, &str, &str, &str)]) -> TempDir {
    let root = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(root.path())
        .assert()
        .success();

    for (filename, id, slug, title) in documents {
        let path = root.path().join(".forgeplan/prds").join(filename);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            path,
            format!(
                "---\nid: {id}\nkind: prd\nstatus: draft\ntitle: {title}\nslug: {slug}\n---\n\n{title} body.\n"
            ),
        )
        .unwrap();
    }
    root
}

fn run(root: &Path, args: &[&str]) -> std::process::Output {
    forgeplan()
        .args(args)
        .current_dir(root)
        .output()
        .expect("forgeplan runs")
}

const SELECTOR_COMMANDS: &[&[&str]] = &[
    &["reason", "PRD-001"],
    &["context", "PRD-001"],
    &["delete", "PRD-001"],
    &["phase", "PRD-001"],
    &["phase-advance", "PRD-001", "--to", "shape"],
    &["supersede", "PRD-001", "PRD-002"],
];

#[test]
fn ambiguous_alias_exits_five_and_names_every_candidate_in_each_command() {
    let root = workspace_with_documents(&[
        ("PRD-001-alpha.md", "PRD-001", "prd-alpha", "Alpha"),
        ("PRD-001-beta.md", "PRD-001", "prd-beta", "Beta"),
    ]);

    for args in SELECTOR_COMMANDS {
        let output = run(root.path(), args);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(5), "{args:?}: {stderr}");
        for expected in ["prd-alpha", "prd-beta", "Alpha", "Beta"] {
            assert!(stderr.contains(expected), "{args:?} omits {expected:?}: {stderr}");
        }
    }
}

#[test]
fn canonical_key_collision_exits_four_and_names_both_claimants_in_each_command() {
    let root = workspace_with_documents(&[
        ("first.md", "PRD-001", "prd-shared", "First claimant"),
        ("second.md", "PRD-002", "prd-shared", "Second claimant"),
    ]);

    for args in SELECTOR_COMMANDS {
        let mut selected = args.to_vec();
        if selected[0] == "supersede" {
            selected[1] = "prd-shared";
        } else {
            selected[1] = "prd-shared";
        }
        let output = run(root.path(), &selected);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(4), "{selected:?}: {stderr}");
        for expected in ["First claimant", "Second claimant"] {
            assert!(stderr.contains(expected), "{selected:?} omits {expected:?}: {stderr}");
        }
    }
}

#[test]
fn absent_selector_exits_three_in_each_command() {
    let root = workspace_with_documents(&[(
        "PRD-001-existing.md",
        "PRD-001",
        "prd-existing",
        "Existing",
    )]);

    for args in SELECTOR_COMMANDS {
        let mut selected = args.to_vec();
        selected[1] = "PRD-404";
        let output = run(root.path(), &selected);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(3), "{selected:?}: {stderr}");
        assert!(stderr.contains("Artifact 'PRD-404' not found"), "{selected:?}: {stderr}");
    }
}
