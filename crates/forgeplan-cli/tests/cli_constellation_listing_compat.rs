//! Paired legacy/Constellation contracts for list, stale, progress and calibrate.
//! These tests exercise the real CLI and project the same Markdown into both
//! layouts. Read commands must not recreate retired tables or publish an index.

use std::path::Path;
use std::process::Output;

use assert_cmd::Command;
use chrono::{Datelike, NaiveDate, Utc};
use forgeplan_core::artifact::frontmatter;
use forgeplan_core::db::store::LanceStore;
use forgeplan_core::projection::{self, MutationContext};
use forgeplan_core::workspace::acquire_workspace_lock;
use serde_json::{Value, json};
use tempfile::TempDir;

const ARTIFACTS: &[(&str, &str)] = &[
    (
        "prds/PRD-001-authentication-rollout.md",
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Authentication rollout\n\
         slug: prd-authentication-rollout\npredicted_number: 1\nassigned_number: null\n\
         depth: tactical\ntags: [team=platform, legacy]\n---\n\n\
         ## Tasks\n\n- [X] Capture the source\n* [ ] Publish the index\n  - [ ] Check the result\n",
    ),
    (
        "prds/PRD-002-completed-rollout.md",
        "---\nid: PRD-002\nkind: prd\nstatus: active\ntitle: Completed rollout\n\
         slug: prd-completed-rollout\npredicted_number: 2\nassigned_number: 2\n\
         depth: deep\ntags: [team=product, legacy]\n---\n\n\
         ## Security\n\n* [x] Review the design\n- [X] Exercise the CLI\n",
    ),
    (
        "rfcs/RFC-003-dependency-links.md",
        "---\nid: RFC-003\nkind: rfc\nstatus: draft\ntitle: Dependency links\n\
         slug: rfc-dependency-links\ndepth: tactical\ntags: [team=platform]\n\
         links:\n  - target: PRD-001\n    relation: based_on\
         \n  - target: PRD-002\n    relation: based_on\
         \n  - target: NOTE-001\n    relation: based_on\
         \n  - target: NOTE-002\n    relation: based_on\
         \n  - target: EVID-001\n    relation: based_on\
         \n  - target: EVID-002\n    relation: based_on\n---\n\n\
         ## Summary\n\nOne small change with six outgoing links.\n",
    ),
    (
        "notes/NOTE-001-no-checkboxes.md",
        "---\nid: NOTE-001\nkind: note\nstatus: active\ntitle: No checkboxes\n\
         depth: tactical\ntags: [team=platform]\n---\n\nPlain text.\n",
    ),
    (
        "notes/NOTE-002-incoming-only.md",
        "---\nid: NOTE-002\nkind: note\nstatus: draft\ntitle: Incoming only\n\
         depth: tactical\n---\n\nNo outgoing links.\n",
    ),
    (
        "notes/NOTE-1000-invalid-expiry.md",
        "---\nid: NOTE-1000\nkind: note\nstatus: draft\ntitle: Invalid expiry\n\
         depth: tactical\nvalid_until: not-a-date\n---\n\nNo expiry can be parsed.\n",
    ),
    (
        "evidence/EVID-001-old-run.md",
        "---\nid: EVID-001\nkind: evidence\nstatus: active\ntitle: Old run\n\
         depth: tactical\nvalid_until: 2000-01-01\n---\n\n\
         verdict: supports\ncongruence_level: 3\nevidence_type: measurement\n",
    ),
    (
        "evidence/EVID-002-expired-verification-caf-r-sum.md",
        "---\nid: EVID-002\nkind: evidence\nstatus: draft\n\
         title: Expired verification café résumé\ndepth: tactical\n\
         valid_until: 2001-02-03T23:45:00Z\n---\n\n\
         verdict: supports\ncongruence_level: 3\nevidence_type: measurement\n",
    ),
    (
        "evidence/EVID-003-future-run.md",
        "---\nid: EVID-003\nkind: evidence\nstatus: active\ntitle: Future run\n\
         depth: tactical\nvalid_until: 2999-01-01\n---\n\n\
         verdict: supports\ncongruence_level: 3\nevidence_type: measurement\n",
    ),
];

fn run(root: &Path, args: &[&str]) -> Output {
    Command::cargo_bin("forgeplan")
        .unwrap()
        .args(args)
        .current_dir(root)
        .env("HOME", root.join(".test-home"))
        .env("XDG_CONFIG_HOME", root.join(".test-home/.config"))
        .env("HF_HUB_OFFLINE", "1")
        .env("NO_COLOR", "1")
        .env("CLICOLOR", "0")
        .env_remove("CLICOLOR_FORCE")
        .output()
        .unwrap()
}

fn run_ok(root: &Path, args: &[&str]) -> Output {
    let out = run(root, args);
    assert!(
        out.status.success(),
        "forgeplan {args:?} failed: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn json_output(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|error| {
        panic!(
            "expected clean JSON stdout ({error}): {}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

// Legacy readers historically accept records whose body includes frontmatter.
// Reindex prepares the ordinary columns and links but currently strips that
// metadata from the body. Seed the full bytes explicitly for this read-contract
// comparison; this is not a test or a repair of production reindex behavior.
fn seed_legacy_source_bodies(root: &Path, sources: &[(&str, &str)]) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let ws = root.join(".forgeplan");
        let guard = acquire_workspace_lock(&ws).await.unwrap();
        let store = LanceStore::open(&ws).await.unwrap();
        for (relative, content) in sources {
            let (fm, _) = frontmatter::parse_frontmatter(content).unwrap();
            let id = fm["id"].as_str().unwrap();
            projection::sync_body_from_file(
                &MutationContext::new(&ws, &guard, Some(&store)),
                id,
                fm["kind"].as_str().unwrap(),
                fm["title"].as_str().unwrap(),
                content,
            )
            .await
            .unwrap_or_else(|error| panic!("full-body seed for {relative} failed: {error}"));
            let record = store.get_record(id).await.unwrap().unwrap();
            assert_eq!(
                record.body, *content,
                "legacy seed lost metadata for {relative}"
            );
            if let Some(slug) = frontmatter::slug_from_frontmatter(&fm) {
                assert_eq!(
                    store.resolve_id(slug).await.unwrap().as_deref(),
                    Some(id),
                    "legacy seed did not preserve slug {slug}"
                );
            }
        }
        // No table handle or workspace guard survives into a CLI child.
        drop(store);
        drop(guard);
    });
}

fn workspace(migrated: bool, seeded: bool) -> TempDir {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join(".test-home/.config")).unwrap();
    run_ok(tmp.path(), &["init", "-y"]);
    if seeded {
        for (relative, content) in ARTIFACTS {
            let path = tmp.path().join(".forgeplan").join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
    }
    if migrated {
        run_ok(tmp.path(), &["index"]);
        assert!(tmp.path().join(".forgeplan/lance/CURRENT.json").is_file());
    } else if seeded {
        let out = run_ok(tmp.path(), &["reindex"]);
        for bytes in [&out.stdout, &out.stderr] {
            assert!(
                !String::from_utf8_lossy(bytes).contains("create failed"),
                "the legacy fixture must project every artifact: {}",
                String::from_utf8_lossy(bytes)
            );
        }
        seed_legacy_source_bodies(tmp.path(), ARTIFACTS);
    }
    tmp
}

fn lance_inventory(root: &Path) -> Vec<String> {
    let mut entries: Vec<_> = std::fs::read_dir(root.join(".forgeplan/lance"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    entries
}

struct Pair {
    legacy: TempDir,
    migrated: TempDir,
    current: Vec<u8>,
    lance: Vec<String>,
    seeded: bool,
}

impl Pair {
    fn new(seeded: bool) -> Self {
        let legacy = workspace(false, seeded);
        let migrated = workspace(true, seeded);
        let current = std::fs::read(migrated.path().join(".forgeplan/lance/CURRENT.json")).unwrap();
        let lance = lance_inventory(migrated.path());
        Self {
            legacy,
            migrated,
            current,
            lance,
            seeded,
        }
    }

    fn ok(&self, args: &[&str]) -> Output {
        let legacy = run_ok(self.legacy.path(), args);
        let migrated = run_ok(self.migrated.path(), args);
        assert_eq!(
            legacy.stdout, migrated.stdout,
            "stdout differs for {args:?}"
        );
        assert_eq!(
            legacy.stderr, migrated.stderr,
            "stderr differs for {args:?}"
        );
        self.assert_read_only();
        migrated
    }

    fn error(&self, args: &[&str]) -> Output {
        let legacy = run(self.legacy.path(), args);
        let migrated = run(self.migrated.path(), args);
        assert!(!legacy.status.success(), "legacy accepted {args:?}");
        assert!(!migrated.status.success(), "migrated accepted {args:?}");
        assert_eq!(legacy.status.code(), migrated.status.code());
        assert_eq!(legacy.stdout, migrated.stdout);
        assert_eq!(legacy.stderr, migrated.stderr);
        self.assert_read_only();
        migrated
    }

    fn assert_read_only(&self) {
        assert_eq!(
            std::fs::read(self.migrated.path().join(".forgeplan/lance/CURRENT.json")).unwrap(),
            self.current,
            "readers must not publish a new generation"
        );
        // init creates evidence.lance before migration; compare against the
        // baseline rather than pretending it was absent in the fixture.
        assert_eq!(lance_inventory(self.migrated.path()), self.lance);
        assert!(
            !self
                .migrated
                .path()
                .join(".forgeplan/lance/artifacts.lance")
                .exists()
        );
        assert!(
            !self
                .migrated
                .path()
                .join(".forgeplan/lance/relations.lance")
                .exists()
        );
        if self.seeded {
            for (relative, content) in ARTIFACTS {
                assert_eq!(
                    std::fs::read_to_string(self.migrated.path().join(".forgeplan").join(relative))
                        .unwrap(),
                    *content,
                    "reader changed {relative}"
                );
            }
        }
    }
}

fn expected_summaries() -> Vec<Value> {
    vec![
        json!({"id": "EVID-001", "kind": "evidence", "status": "active", "title": "Old run"}),
        json!({"id": "EVID-002", "kind": "evidence", "status": "draft", "title": "Expired verification café résumé"}),
        json!({"id": "EVID-003", "kind": "evidence", "status": "active", "title": "Future run"}),
        json!({"id": "NOTE-001", "kind": "note", "status": "active", "title": "No checkboxes"}),
        json!({"id": "NOTE-002", "kind": "note", "status": "draft", "title": "Incoming only"}),
        json!({"id": "NOTE-1000", "kind": "note", "status": "draft", "title": "Invalid expiry"}),
        json!({"id": "PRD-001", "kind": "prd", "status": "draft", "title": "Authentication rollout"}),
        json!({"id": "PRD-002", "kind": "prd", "status": "active", "title": "Completed rollout"}),
        json!({"id": "RFC-003", "kind": "rfc", "status": "draft", "title": "Dependency links"}),
    ]
}

#[test]
fn list_preserves_bare_arrays_fields_and_composed_filter_flags() {
    let pair = Pair::new(true);
    let all = expected_summaries();
    let cases: &[(&[&str], &[&str])] = &[
        (
            &[],
            &[
                "EVID-001",
                "EVID-002",
                "EVID-003",
                "NOTE-001",
                "NOTE-002",
                "NOTE-1000",
                "PRD-001",
                "PRD-002",
                "RFC-003",
            ],
        ),
        (&["--type", "prd"], &["PRD-001", "PRD-002"]),
        (&["-t", "PRD", "-s", "DRAFT"], &["PRD-001"]),
        (
            &["--status", "active"],
            &["EVID-001", "EVID-003", "NOTE-001", "PRD-002"],
        ),
        (
            &["--tag", "team"],
            &["NOTE-001", "PRD-001", "PRD-002", "RFC-003"],
        ),
        (
            &["--tag", "team=platform"],
            &["NOTE-001", "PRD-001", "RFC-003"],
        ),
        (&["--tag", "legacy"], &["PRD-001", "PRD-002"]),
        (
            &[
                "--type",
                "prd",
                "--status",
                "draft",
                "--tag",
                "team=platform",
            ],
            &["PRD-001"],
        ),
        (
            &[
                "--type",
                "prd",
                "--status",
                "active",
                "--tag",
                "team=platform",
            ],
            &[],
        ),
        (&["--tag", "Team"], &[]),
        (&["--type", "unknown"], &[]),
    ];
    for (filters, ids) in cases {
        let mut args = vec!["list", "--json"];
        args.extend_from_slice(filters);
        let out = pair.ok(&args);
        let expected: Vec<_> = all
            .iter()
            .filter(|row| ids.contains(&row["id"].as_str().unwrap()))
            .cloned()
            .collect();
        assert_eq!(json_output(&out), Value::Array(expected), "{args:?}");
        assert!(String::from_utf8_lossy(&out.stderr).starts_with("Next: forgeplan "));
        if ids.is_empty() {
            assert_eq!(out.stdout, b"[]\n");
        }
    }

    let out = pair.ok(&["list", "--json", "--type", "prd", "--status", "draft"]);
    assert_eq!(
        out.stderr,
        b"Next: forgeplan get prd-authentication-rollout\n"
    );
    let out = pair.ok(&["list", "--json", "-t", "prd", "-s", "active"]);
    assert_eq!(out.stderr, b"Next: forgeplan get PRD-002\n");
}

#[test]
fn list_preserves_text_alignment_and_empty_filter_hints() {
    let pair = Pair::new(true);
    let out = pair.ok(&["list"]);
    let text = String::from_utf8(out.stdout).unwrap();
    let expected_header = format!("{:<9}  {:<8}  {:<6}  {}", "ID", "Kind", "Status", "Title");
    assert_eq!(text.lines().next().unwrap(), expected_header);
    for row in expected_summaries() {
        let expected_line = format!(
            "{:<9}  {:<8}  {:<6}  {}",
            row["id"].as_str().unwrap(),
            row["kind"].as_str().unwrap(),
            row["status"].as_str().unwrap(),
            row["title"].as_str().unwrap()
        );
        assert!(
            text.lines().any(|line| line == expected_line),
            "missing {expected_line:?}: {text}"
        );
    }
    assert!(text.contains("9 artifact(s) total"));
    assert!(text.contains("Next: forgeplan get EVID-001"));

    let out = pair.ok(&["list", "--type", "rfc", "--status", "active"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("  No artifacts found.\n"));
    assert!(text.contains("Next: forgeplan new rfc \"<title>\""));
    let out = pair.ok(&["list", "--json", "--type", "rfc", "--status", "active"]);
    assert_eq!(out.stdout, b"[]\n");
    assert_eq!(out.stderr, b"Next: forgeplan new rfc \"<title>\"\n");
}

#[test]
fn stale_preserves_expiry_order_days_and_terminal_output() {
    let pair = Pair::new(true);
    let out = pair.ok(&["stale", "--json"]);
    let today = Utc::now().date_naive();
    assert_eq!(
        json_output(&out),
        json!({
            "stale": [
                {"id": "EVID-001", "title": "Old run", "valid_until": "2000-01-01",
                 "days_expired": (today - NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()).num_days()},
                {"id": "EVID-002", "title": "Expired verification café résumé",
                 "valid_until": "2001-02-03T23:45:00Z", "days_expired": 0}
            ],
            "_next_action": format!(
                "forgeplan renew EVID-001 --reason \"<why still relevant>\" --until {}-12-31",
                today.year() + 1
            )
        })
    );
    // RFC3339 expiries participate in selection; the existing report's
    // date-only day counter deliberately remains unchanged.
    let out = pair.ok(&["stale"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("Found 2 stale artifact(s) with expired valid_until:"));
    assert!(text.contains("Expired verification café..."));
    assert!(text.contains("2001-02-03T23:45:00Z"));
    assert!(
        text.contains("Hint: Use `forgeplan score <ID>` to check R_eff impact of stale evidence.")
    );
    assert!(text.contains("Next: forgeplan renew EVID-001"));
}

#[test]
fn progress_preserves_counts_percent_scope_and_slug_resolution() {
    let pair = Pair::new(true);
    let partial = json!({"id": "PRD-001", "title": "Authentication rollout", "kind": "prd",
                         "completed": 1, "total": 3, "percent": 33});
    let complete = json!({"id": "PRD-002", "title": "Completed rollout", "kind": "prd",
                          "completed": 2, "total": 2, "percent": 100});
    assert_eq!(
        json_output(&pair.ok(&["progress", "--json"])),
        json!({"progress": [partial.clone(), complete.clone()], "_next_action": "forgeplan get PRD-001"})
    );
    for reference in [
        "PRD-001",
        "prd-1",
        "PRD-1",
        "prd-authentication-rollout",
        " PRD-001 ",
    ] {
        assert_eq!(
            json_output(&pair.ok(&["progress", reference, "--json"])),
            json!({"progress": [partial.clone()], "_next_action": "forgeplan get PRD-001"})
        );
    }
    assert_eq!(
        json_output(&pair.ok(&["progress", "prd-completed-rollout", "--json"])),
        json!({"progress": [complete], "_next_action": "forgeplan activate PRD-002"})
    );
    assert_eq!(
        json_output(&pair.ok(&["progress", "NOTE-001", "--json"])),
        json!({"progress": [], "_next_action": "forgeplan get NOTE-001"})
    );

    let out = pair.ok(&["progress"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("Forgeplan Progress\n=================="));
    assert!(text.contains("3/5  (60%)"));
    assert!(text.contains("2 artifact(s) with checkboxes"));
    let display = pair.ok(&["progress", "PRD-001"]);
    let slug = pair.ok(&["progress", "prd-authentication-rollout"]);
    assert_eq!(display.stdout, slug.stdout);
    let text = String::from_utf8(slug.stdout).unwrap();
    assert!(text.contains("1/3  (33%)  WIP"));
    assert!(text.contains("  [x] Capture the source\n"));
    assert!(text.contains("  [ ] Publish the index\n"));
    assert!(text.contains("  [ ] Check the result\n"));
    let out = pair.ok(&["progress", "PRD-002"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("Next: forgeplan activate PRD-002"));
    let out = pair.ok(&["progress", "NOTE-001"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("No checkboxes found in this artifact."));

    for args in [
        vec!["progress", "PRD-999", "--json"],
        vec!["progress", "prd-missing"],
    ] {
        let out = pair.error(&args);
        let target = args[1];
        assert!(String::from_utf8_lossy(&out.stderr).contains(&format!(
            "Artifact '{target}' not found\nFix: forgeplan list"
        )));
    }
}

#[test]
fn calibrate_preserves_content_signals_and_real_outgoing_link_counts() {
    let pair = Pair::new(true);
    let out = pair.ok(&["calibrate", "RFC-003"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("RFC-003 \"Dependency links\""));
    assert!(text.contains("Current:   \"tactical\""));
    assert!(text.contains("Suggested: Deep ⬆ ESCALATION"));
    assert!(text.contains("Deep → dependency_links (6 links)"));
    assert!(text.contains("Next: forgeplan get RFC-003"));
    let lower = pair.ok(&["calibrate", "rfc-003"]);
    assert_eq!(lower.stdout, text.as_bytes());

    let out = pair.ok(&["calibrate", "NOTE-002"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("Suggested: Tactical"));
    assert!(
        !text.contains("dependency_links"),
        "incoming links are not outgoing links: {text}"
    );
    assert!(text.contains("Next: forgeplan score NOTE-002"));

    let out = pair.ok(&["calibrate", "PRD-002"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("Deep → security_section (present)"));
    assert!(!text.contains("ESCALATION"));
    assert!(text.contains("Next: forgeplan score PRD-002"));

    let out = pair.ok(&["calibrate"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("RFC-003 \"Dependency links\""));
    assert!(!text.contains("PRD-002 \"Completed rollout\""));
    assert!(text.contains("1 artifact(s) need depth escalation."));
    let out = pair.error(&["calibrate", "RFC-999"]);
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("Artifact 'RFC-999' not found\nFix: forgeplan list")
    );
}

#[test]
fn empty_workspaces_keep_all_existing_empty_shapes_and_hints() {
    let pair = Pair::new(false);
    let out = pair.ok(&["list", "--json"]);
    assert_eq!(out.stdout, b"[]\n");
    assert_eq!(out.stderr, b"Next: forgeplan new prd \"<title>\"\n");
    let out = pair.ok(&["list"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("  No artifacts found."));
    assert_eq!(
        json_output(&pair.ok(&["stale", "--json"])),
        json!({"stale": [], "_next_action": null})
    );
    let out = pair.ok(&["stale"]);
    assert_eq!(
        out.stdout,
        b"No stale artifacts found. All valid_until dates are current.\nDone.\n"
    );
    for args in [
        vec!["progress", "--json"],
        vec!["progress", "PRD-999", "--json"],
    ] {
        assert_eq!(
            json_output(&pair.ok(&args)),
            json!({"progress": [], "_next_action": "forgeplan new prd \"<title>\""})
        );
    }
    let out = pair.ok(&["progress"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("No artifacts found.\n"));
    assert!(text.contains("Next: forgeplan new prd \"<title>\""));
    for args in [vec!["calibrate"], vec!["calibrate", "RFC-999"]] {
        assert_eq!(pair.ok(&args).stdout, b"No artifacts found.\n");
    }
}

#[test]
fn fresh_unindexed_sources_keep_nonempty_terminal_and_checkbox_free_reports() {
    let pair = Pair::new(false);
    let (relative, content) = ARTIFACTS[3];
    for root in [pair.legacy.path(), pair.migrated.path()] {
        let path = root.join(".forgeplan").join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
    // Only the legacy projection is refreshed. Captured readers must observe
    // the newly added source without mutating the empty published generation.
    run_ok(pair.legacy.path(), &["reindex"]);
    seed_legacy_source_bodies(pair.legacy.path(), &[(relative, content)]);
    let out = pair.ok(&["list", "--json"]);
    assert_eq!(
        json_output(&out),
        json!([{"id": "NOTE-001", "kind": "note", "status": "active", "title": "No checkboxes"}])
    );
    assert_eq!(
        json_output(&pair.ok(&["progress", "--json"])),
        json!({"progress": [], "_next_action": null})
    );
    let out = pair.ok(&["progress"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("No artifacts with checkboxes found.\n"));
    assert!(text.contains("Next: forgeplan list --type prd"));
    // The emitted action must parse on both layouts, not preserve the old
    // nonexistent --kind flag.
    pair.ok(&["list", "--type", "prd"]);
    assert_eq!(
        json_output(&pair.ok(&["stale", "--json"])),
        json!({"stale": [], "_next_action": null})
    );
    let out = pair.ok(&["stale"]);
    assert_eq!(
        out.stdout,
        b"No stale artifacts found. All valid_until dates are current.\nDone.\n"
    );
    let out = pair.ok(&["calibrate"]);
    assert!(
        String::from_utf8_lossy(&out.stdout)
            .contains("All artifacts are at appropriate depth levels.")
    );
    assert_eq!(
        std::fs::read_to_string(pair.migrated.path().join(".forgeplan").join(relative)).unwrap(),
        content
    );
}

#[test]
fn corrupt_captured_generation_fails_instead_of_emitting_empty_success() {
    let migrated = workspace(true, true);
    let current = migrated.path().join(".forgeplan/lance/CURRENT.json");
    std::fs::write(&current, b"{ not json").unwrap();
    let baseline = lance_inventory(migrated.path());
    for args in [
        vec!["list", "--json"],
        vec!["list"],
        vec!["stale", "--json"],
        vec!["stale"],
        vec!["progress", "--json"],
        vec!["progress", "prd-authentication-rollout", "--json"],
        vec!["progress"],
        vec!["calibrate"],
        vec!["calibrate", "RFC-003"],
    ] {
        let out = run(migrated.path(), &args);
        assert!(
            !out.status.success(),
            "accepted corrupt capture for {args:?}"
        );
        assert!(
            out.stdout.is_empty(),
            "failure emitted a report for {args:?}"
        );
        let error = String::from_utf8_lossy(&out.stderr);
        assert!(
            error.contains("index_generation_unavailable"),
            "{args:?}: {error}"
        );
        assert!(error.contains("CURRENT.json"), "{args:?}: {error}");
        assert_eq!(std::fs::read(&current).unwrap(), b"{ not json");
        assert_eq!(lance_inventory(migrated.path()), baseline);
    }
}
