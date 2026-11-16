//! Paired legacy/Constellation contracts for list, stale, progress and calibrate.
//! These tests exercise the real CLI and project the same Markdown into both
//! layouts. Read commands must not recreate retired tables or publish an index.

use std::path::Path;
use std::process::Output;

use assert_cmd::Command;
use chrono::{NaiveDate, Utc};
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
            let record = store.get_record(&guard, id).await.unwrap().unwrap();
            assert_eq!(
                record.body, *content,
                "legacy seed lost metadata for {relative}"
            );
            if let Some(slug) = frontmatter::slug_from_frontmatter(&fm) {
                assert_eq!(
                    store.resolve_id(&guard, slug).await.unwrap().as_deref(),
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

    fn ok(&self, args: &[&str]) -> (Output, Output) {
        let legacy = run_ok(self.legacy.path(), args);
        let migrated = run_ok(self.migrated.path(), args);
        self.assert_read_only();
        (legacy, migrated)
    }

    fn error(&self, args: &[&str]) -> Output {
        let legacy = run(self.legacy.path(), args);
        let migrated = run(self.migrated.path(), args);
        assert!(!legacy.status.success(), "legacy accepted {args:?}");
        assert!(!migrated.status.success(), "migrated accepted {args:?}");
        assert_eq!(legacy.status.code(), migrated.status.code());
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
        json!({"id": "EVID-001", "canonical_key": "EVID-001", "kind": "evidence", "status": "active", "title": "Old run"}),
        json!({"id": "EVID-002", "canonical_key": "EVID-002", "kind": "evidence", "status": "draft", "title": "Expired verification café résumé"}),
        json!({"id": "EVID-003", "canonical_key": "EVID-003", "kind": "evidence", "status": "active", "title": "Future run"}),
        json!({"id": "NOTE-001", "canonical_key": "NOTE-001", "kind": "note", "status": "active", "title": "No checkboxes"}),
        json!({"id": "NOTE-002", "canonical_key": "NOTE-002", "kind": "note", "status": "draft", "title": "Incoming only"}),
        json!({"id": "NOTE-1000", "canonical_key": "NOTE-1000", "kind": "note", "status": "draft", "title": "Invalid expiry"}),
        json!({"id": "PRD-001", "canonical_key": "prd-authentication-rollout", "kind": "prd", "status": "draft", "title": "Authentication rollout"}),
        json!({"id": "PRD-002", "canonical_key": "prd-completed-rollout", "kind": "prd", "status": "active", "title": "Completed rollout"}),
        json!({"id": "RFC-003", "canonical_key": "rfc-dependency-links", "kind": "rfc", "status": "draft", "title": "Dependency links"}),
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
        let (legacy, migrated) = pair.ok(&args);
        let expected: Vec<_> = all
            .iter()
            .filter(|row| ids.contains(&row["id"].as_str().unwrap()))
            .cloned()
            .collect();
        for out in [&legacy, &migrated] {
            assert_eq!(json_output(out), Value::Array(expected.clone()), "{args:?}");
        }
    }
}

#[test]
fn stale_preserves_expiry_order_and_days() {
    let pair = Pair::new(true);
    let (legacy, migrated) = pair.ok(&["stale", "--json"]);
    let today = Utc::now().date_naive();
    for out in [&legacy, &migrated] {
        assert_eq!(
            json_output(out)["stale"],
            json!([
                {"id": "EVID-001", "title": "Old run", "valid_until": "2000-01-01",
                 "days_expired": (today - NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()).num_days()},
                {"id": "EVID-002", "title": "Expired verification café résumé",
                 "valid_until": "2001-02-03T23:45:00Z", "days_expired": 0}
            ])
        );
    }
}

#[test]
fn progress_preserves_counts_percent_scope_and_slug_resolution() {
    let pair = Pair::new(true);
    let partial = json!({"id": "PRD-001", "title": "Authentication rollout", "kind": "prd",
                         "completed": 1, "total": 3, "percent": 33});
    let complete = json!({"id": "PRD-002", "title": "Completed rollout", "kind": "prd",
                          "completed": 2, "total": 2, "percent": 100});
    let (legacy, migrated) = pair.ok(&["progress", "--json"]);
    for out in [&legacy, &migrated] {
        assert_eq!(
            json_output(out)["progress"],
            json!([partial.clone(), complete.clone()])
        );
    }
    for reference in [
        "PRD-001",
        "prd-1",
        "PRD-1",
        "prd-authentication-rollout",
        " PRD-001 ",
    ] {
        let (legacy, migrated) = pair.ok(&["progress", reference, "--json"]);
        for out in [&legacy, &migrated] {
            assert_eq!(json_output(out)["progress"], json!([partial.clone()]));
        }
    }
    let (legacy, migrated) = pair.ok(&["progress", "prd-completed-rollout", "--json"]);
    for out in [&legacy, &migrated] {
        assert_eq!(json_output(out)["progress"], json!([complete.clone()]));
    }
    let (legacy, migrated) = pair.ok(&["progress", "NOTE-001", "--json"]);
    for out in [&legacy, &migrated] {
        assert_eq!(json_output(out)["progress"], json!([]));
    }

    for args in [
        vec!["progress", "PRD-999", "--json"],
        vec!["progress", "prd-missing"],
    ] {
        let out = pair.error(&args);
        assert_eq!(out.status.code(), Some(3), "{args:?}");
    }
}

#[test]
fn calibrate_preserves_content_signals_and_real_outgoing_link_counts() {
    let pair = Pair::new(true);
    let cases: &[(&[&str], &[&str])] = &[
        (
            &["calibrate", "RFC-003"],
            &[
                "RFC-003 \"Dependency links\"",
                "Current:   \"tactical\"",
                "Suggested: Deep ⬆ ESCALATION",
                "Deep → dependency_links (6 links)",
            ],
        ),
        (&["calibrate", "NOTE-002"], &["Suggested: Tactical"]),
        (
            &["calibrate", "PRD-002"],
            &["Deep → security_section (present)"],
        ),
        (
            &["calibrate"],
            &[
                "RFC-003 \"Dependency links\"",
                "1 artifact(s) need depth escalation.",
            ],
        ),
    ];
    for (args, expected) in cases {
        let (legacy, migrated) = pair.ok(args);
        for out in [&legacy, &migrated] {
            let text = String::from_utf8_lossy(&out.stdout);
            for marker in *expected {
                assert!(text.contains(marker), "{args:?}: {text}");
            }
            if args.contains(&"NOTE-002") {
                assert!(
                    !text.contains("dependency_links"),
                    "incoming links are not outgoing links: {text}"
                );
            }
            if args.contains(&"PRD-002") {
                assert!(!text.contains("ESCALATION"));
            }
            if args.len() == 1 {
                assert!(!text.contains("PRD-002 \"Completed rollout\""));
            }
        }
    }
    let out = pair.error(&["calibrate", "RFC-999"]);
    assert_eq!(out.status.code(), Some(3));
}

#[test]
fn empty_workspaces_keep_empty_shapes_and_missing_reference_statuses() {
    let pair = Pair::new(false);
    let (legacy, migrated) = pair.ok(&["list", "--json"]);
    for out in [&legacy, &migrated] {
        assert_eq!(json_output(out), json!([]));
    }
    let (legacy, migrated) = pair.ok(&["stale", "--json"]);
    for out in [&legacy, &migrated] {
        assert_eq!(json_output(out)["stale"], json!([]));
    }
    let (legacy, migrated) = pair.ok(&["progress", "--json"]);
    for out in [&legacy, &migrated] {
        assert_eq!(json_output(out)["progress"], json!([]));
    }
    let missing = pair.error(&["progress", "PRD-999", "--json"]);
    assert_eq!(missing.status.code(), Some(3));
    let missing = pair.error(&["calibrate", "RFC-999"]);
    assert_eq!(missing.status.code(), Some(3));
}

#[test]
fn fresh_unindexed_source_is_visible_without_mutating_migrated_source() {
    let pair = Pair::new(false);
    let (relative, content) = ARTIFACTS[3];
    for root in [pair.legacy.path(), pair.migrated.path()] {
        let path = root.join(".forgeplan").join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
    run_ok(pair.legacy.path(), &["reindex"]);
    seed_legacy_source_bodies(pair.legacy.path(), &[(relative, content)]);
    let (legacy, migrated) = pair.ok(&["list", "--json"]);
    for out in [&legacy, &migrated] {
        assert_eq!(
            json_output(out),
            json!([{"id": "NOTE-001", "canonical_key": "NOTE-001", "kind": "note", "status": "active", "title": "No checkboxes"}])
        );
    }
    let (legacy, migrated) = pair.ok(&["progress", "--json"]);
    for out in [&legacy, &migrated] {
        assert_eq!(json_output(out)["progress"], json!([]));
    }
    let (legacy, migrated) = pair.ok(&["stale", "--json"]);
    for out in [&legacy, &migrated] {
        assert_eq!(json_output(out)["stale"], json!([]));
    }
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
