//! E2E for `forgeplan constellation get KEY@qualifier --snapshot <id>`.
//!
//! The contract under test: `parse_qualified_ref` already splits `KEY@q`
//! before the snapshot arm runs — the qualifier must narrow the pinned
//! source set exactly as it does the live one. A qualifier that names no
//! pinned entry errors; it never widens silently back to every source.
//! HOME is pinned so the per-user registry is hermetic, matching
//! `cli_constellation_rebind.rs`.

use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Pin HOME (and the Windows/XDG twins) so `dirs::home_dir()` — and with
/// it the constellation registry — is the test's own directory.
fn home_env<'a>(cmd: &'a mut Command, home: &Path) -> &'a mut Command {
    cmd.env("HOME", home)
        .env("USERPROFILE", home)
        .env("XDG_DATA_HOME", home)
}

fn constellation(home: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut full = vec!["constellation"];
    full.extend_from_slice(args);
    home_env(forgeplan().args(&full), home).assert()
}

/// A git checkout holding a `.forgeplan` with one ADR — the only shape a
/// snapshot can pin. `title` lands in the file bytes, so each source's
/// answer at the same `ADR-001` id is distinguishable by content.
fn snapshot_source(root: &Path, home: &Path, title: &str) {
    std::fs::create_dir_all(root).unwrap();
    home_env(forgeplan().args(["init", "-y"]), home)
        .current_dir(root)
        .assert()
        .success();
    home_env(forgeplan().args(["new", "adr", title]), home)
        .current_dir(root)
        .assert()
        .success();
    // The lance tables are derived state — they must not inflate the
    // pinned tree (or a later `.git` object count).
    std::fs::write(root.join(".gitignore"), ".forgeplan/lance/\n").unwrap();
    let git = |args: &[&str]| {
        home_env(Command::new("git").arg("-C").arg(root).args(args), home)
            .assert()
            .success();
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "t@t"]);
    git(&["config", "user.name", "t"]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "init"]);
}

/// Two sources both claiming `ADR-001` at one snapshot: an unqualified
/// get is `ambiguous`, a path-fragment or `src:` qualifier answers only
/// the named source's pinned bytes, and an unknown qualifier fails.
#[test]
fn get_at_a_snapshot_honours_the_source_qualifier() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_alpha = tmp.path().join("src-alpha");
    let ws_beta = tmp.path().join("src-beta");
    std::fs::create_dir_all(&home).unwrap();
    snapshot_source(&ws_alpha, &home, "Alpha answer");
    snapshot_source(&ws_beta, &home, "Beta answer");

    constellation(&home, &["create", "demo"]).success();
    constellation(
        &home,
        &["add", &ws_alpha.join(".forgeplan").display().to_string()],
    )
    .success();
    constellation(
        &home,
        &["add", &ws_beta.join(".forgeplan").display().to_string()],
    )
    .success();

    let out = constellation(&home, &["snapshot"]).success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let snap_id = stdout
        .split_whitespace()
        .nth(1)
        .expect("the snapshot output names its id")
        .to_string();
    assert!(snap_id.starts_with("snap-"), "{stdout}");

    // Unqualified: both sources claim ADR-001 — ambiguous, and the output
    // lists each claim's pinned `src:` id.
    let out = constellation(&home, &["get", "ADR-001", "--snapshot", &snap_id]).success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("claimed by 2 sources"), "{stdout}");
    let beta_src = stdout
        .lines()
        .find(|l| l.contains("beta-answer"))
        .expect("the beta claim is listed")
        .trim()
        .trim_end_matches(')')
        .rsplit('(')
        .next()
        .unwrap()
        .trim()
        .to_string();
    assert!(beta_src.starts_with("src:"), "{beta_src}");
    let alpha_src = stdout
        .lines()
        .find(|l| l.contains("alpha-answer"))
        .expect("the alpha claim is listed")
        .trim()
        .trim_end_matches(')')
        .rsplit('(')
        .next()
        .unwrap()
        .trim()
        .to_string();

    // The `src:` qualifier is a PREFIX match on the pinned id — the same
    // `starts_with` reading `specs_matching` gives the live arm. The
    // shortest prefix that names beta alone narrows exactly like the full
    // id; the shared `src:` scheme prefix matches every pinned source, so
    // the read stays `ambiguous` rather than silently picking a winner.
    let prefix_len = (1..=beta_src.len())
        .find(|n| !alpha_src.starts_with(&beta_src[..*n]))
        .expect("two distinct src ids diverge");
    constellation(
        &home,
        &[
            "get",
            &format!("ADR-001@{}", &beta_src[..prefix_len]),
            "--snapshot",
            &snap_id,
        ],
    )
    .success()
    .stdout(predicate::str::contains("Beta answer"))
    .stdout(predicate::str::contains("Alpha answer").not());

    constellation(&home, &["get", "ADR-001@src:", "--snapshot", &snap_id])
        .success()
        .stdout(predicate::str::contains("claimed by 2 sources"));

    // A path-fragment qualifier answers only alpha's pinned bytes.
    constellation(&home, &["get", "ADR-001@src-alpha", "--snapshot", &snap_id])
        .success()
        .stdout(predicate::str::contains("Alpha answer"))
        .stdout(predicate::str::contains("Beta answer").not());

    // A pinned `src:` id qualifier answers only beta's pinned bytes.
    constellation(
        &home,
        &[
            "get",
            &format!("ADR-001@{beta_src}"),
            "--snapshot",
            &snap_id,
        ],
    )
    .success()
    .stdout(predicate::str::contains("Beta answer"))
    .stdout(predicate::str::contains("Alpha answer").not());

    // A qualifier naming nothing pinned is an error — the same refusal
    // the live arm gives, never a silent read of the whole set.
    constellation(
        &home,
        &["get", "ADR-001@no-such-source", "--snapshot", &snap_id],
    )
    .failure()
    .stderr(predicate::str::contains("matches no source"));
}

#[test]
fn live_and_snapshot_get_name_all_canonical_claimants_in_json_and_text() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let root = tmp.path().join("source");
    std::fs::create_dir_all(&home).unwrap();
    snapshot_source(&root, &home, "Unrelated decision");
    let fp = root.join(".forgeplan");
    for (file, id, title) in [
        ("ADR-101-first.md", "ADR-101", "First claimant"),
        ("ADR-102-second.md", "ADR-102", "Second claimant"),
    ] {
        std::fs::write(
            fp.join("adrs").join(file),
            format!(
                "---\nid: {id}\nslug: adr-shared\nkind: adr\nstatus: active\ntitle: {title}\n---\n\n{title} body.\n"
            ),
        )
        .unwrap();
    }
    for args in [
        vec!["add", "-A"],
        vec!["commit", "-qm", "collision fixture"],
    ] {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success());
    }
    constellation(&home, &["create", "demo"]).success();
    constellation(&home, &["add", fp.to_str().unwrap()]).success();
    let snapshot = constellation(&home, &["snapshot"]).success();
    let stdout = String::from_utf8_lossy(&snapshot.get_output().stdout);
    let snapshot_id = stdout.split_whitespace().nth(1).unwrap();
    for snapshot in [None, Some(snapshot_id)] {
        let mut args = vec!["get", "adr-shared"];
        if let Some(id) = snapshot {
            args.extend(["--snapshot", id]);
        }
        let human = constellation(&home, &args).code(4);
        let stderr = String::from_utf8_lossy(&human.get_output().stderr);
        for claimant in [
            "First claimant",
            "Second claimant",
            "adrs/ADR-101-first.md",
            "adrs/ADR-102-second.md",
        ] {
            assert!(stderr.contains(claimant), "{stderr}");
        }
        args.push("--json");
        let json = constellation(&home, &args).code(4);
        let response: serde_json::Value =
            serde_json::from_slice(&json.get_output().stdout).unwrap();
        assert_eq!(response["error"], "conflicting_canonical_key");
        assert_eq!(response["input"], "adr-shared");
        let candidates = response["candidates"].as_array().unwrap();
        assert_eq!(candidates.len(), 2);
        for (id, title, path) in [
            ("ADR-101", "First claimant", "adrs/ADR-101-first.md"),
            ("ADR-102", "Second claimant", "adrs/ADR-102-second.md"),
        ] {
            assert!(candidates.iter().any(|candidate| {
                candidate["canonical_key"] == "adr-shared"
                    && candidate["display_id"] == id
                    && candidate["title"] == title
                    && candidate["relative_path"]
                        .as_str()
                        .is_some_and(|value| value != path && value.ends_with(path))
            }));
        }
    }
}
