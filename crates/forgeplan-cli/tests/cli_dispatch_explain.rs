//! PRD-077 FR-010 — `forgeplan dispatch` must classify *why* each serial-queue
//! entry was deferred. Without this the agent sees only IDs and has to
//! cross-reference `reasoning[]` (which is a free-text audit log) to figure
//! out what to fix.
//!
//! Regression fixture: three artifacts that exercise the three primary
//! deferral causes the dispatcher currently emits:
//!   1. PRD-001 — disjoint `affected_files`, fits a bucket
//!   2. PRD-002 — overlapping `affected_files` with PRD-001 → serial
//!      with reason mentioning "overlap"
//!   3. PRD-003 — no `affected_files` declared at all → serial with reason
//!      mentioning "missing affected_files"

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init(tmp: &TempDir) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

/// Locate the markdown file for a given artifact prefix (`PRD-001` etc.).
fn artifact_path(tmp: &TempDir, kind_dir: &str, prefix: &str) -> std::path::PathBuf {
    let dir = tmp.path().join(format!(".forgeplan/{kind_dir}"));
    let entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .to_uppercase()
                .starts_with(&prefix.to_uppercase())
        })
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "expected exactly one file matching {prefix} in {dir:?}"
    );
    entries[0].path()
}

/// Rewrite an artifact's body so the dispatcher sees exactly the files we
/// specify. We strip any existing `## Affected Files` section the default
/// template injected and replace it with a fresh one. This isolates the
/// test from template churn (the default template lists
/// `crates/forgeplan-core/src/**` etc. — fine for users, noise for
/// FR-010 regression).
fn set_affected_files(tmp: &TempDir, kind_dir: &str, prefix: &str, files: &[&str]) {
    let path = artifact_path(tmp, kind_dir, prefix);
    let body = std::fs::read_to_string(&path).unwrap();
    let mut out = String::new();
    let mut skip = false;
    for line in body.lines() {
        if line.starts_with("## Affected Files") {
            skip = true;
            continue;
        }
        if skip && line.starts_with("## ") {
            skip = false;
        }
        if !skip {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str("\n## Affected Files\n\n");
    for f in files {
        out.push_str(&format!("- `{f}`\n"));
    }
    out.push_str("\n## _End\n");
    std::fs::write(&path, out).unwrap();
}

/// Strip the default `## Affected Files` section entirely so the
/// dispatcher classifies the artifact as "missing affected_files
/// frontmatter" (the canonical reason for "no file list known").
fn strip_affected_files(tmp: &TempDir, kind_dir: &str, prefix: &str) {
    let path = artifact_path(tmp, kind_dir, prefix);
    let body = std::fs::read_to_string(&path).unwrap();
    let mut out = String::new();
    let mut skip = false;
    for line in body.lines() {
        if line.starts_with("## Affected Files") {
            skip = true;
            continue;
        }
        if skip && line.starts_with("## ") {
            skip = false;
        }
        if !skip {
            out.push_str(line);
            out.push('\n');
        }
    }
    std::fs::write(&path, out).unwrap();
}

#[test]
fn dispatch_explain_serial_reasons_overlap_and_missing() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    // PRD-001 and PRD-002 share two files → at the default Jaccard
    // threshold of 0.3 their overlap = 2/3 ≈ 0.667 ≥ 0.3, so PRD-002
    // cannot share a bucket with PRD-001.
    for title in ["Alpha overlapping", "Beta overlapping", "Gamma no files"].iter() {
        forgeplan()
            .args(["new", "prd", title])
            .current_dir(tmp.path())
            .assert()
            .success();
    }
    set_affected_files(&tmp, "prds", "PRD-001", &["src/a.rs", "src/b.rs"]);
    set_affected_files(&tmp, "prds", "PRD-002", &["src/a.rs", "src/b.rs"]);
    // PRD-003 deliberately gets NO `## Affected Files` (template's
    // default is stripped) and no FM — dispatcher must classify it as
    // "missing affected_files frontmatter".
    strip_affected_files(&tmp, "prds", "PRD-003");

    // One agent forces PRD-002 to serial (PRD-001 takes the only bucket).
    let output = forgeplan()
        .args(["dispatch", "--agents", "1", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "dispatch --json failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("expected valid JSON: {e}\n\nbody:\n{stdout}"));

    // FR-010 contract: every serial_queue entry is now an object with
    // `id` + `reason` (was a bare string in v0.31 and earlier).
    let serial = parsed["serial_queue"]
        .as_array()
        .expect("serial_queue must be an array");
    assert!(
        !serial.is_empty(),
        "expected at least PRD-002 + PRD-003 in serial, got: {parsed}"
    );

    // Index by id so we don't depend on ordering.
    let mut by_id: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for item in serial {
        let id = item["id"]
            .as_str()
            .unwrap_or_else(|| panic!("serial entry missing id: {item}"))
            .to_string();
        let reason = item["reason"]
            .as_str()
            .unwrap_or_else(|| panic!("serial entry missing reason: {item}"))
            .to_string();
        by_id.insert(id, reason);
    }

    // PRD-002 — file overlap with bucket containing PRD-001.
    let prd002 = by_id
        .get("PRD-002")
        .unwrap_or_else(|| panic!("PRD-002 must be in serial_queue: {parsed}"));
    assert!(
        prd002.contains("overlap") || prd002.contains("conflicts"),
        "PRD-002 serial reason must mention overlap/conflict (got: {prd002})"
    );

    // PRD-003 — missing affected_files frontmatter.
    let prd003 = by_id
        .get("PRD-003")
        .unwrap_or_else(|| panic!("PRD-003 must be in serial_queue: {parsed}"));
    assert!(
        prd003.contains("missing affected_files") || prd003.contains("affected_files"),
        "PRD-003 serial reason must mention missing affected_files (got: {prd003})"
    );

    // PRD-001 — in some bucket, NOT in serial.
    assert!(
        !by_id.contains_key("PRD-001"),
        "PRD-001 must NOT be in serial (it should hold the bucket): {parsed}"
    );
    let buckets = parsed["buckets"].as_array().expect("buckets must be array");
    let in_bucket = buckets.iter().any(|b| {
        b.as_array()
            .map(|arr| arr.iter().any(|v| v.as_str() == Some("PRD-001")))
            .unwrap_or(false)
    });
    assert!(in_bucket, "PRD-001 must be assigned to a bucket: {parsed}");
}

#[test]
fn dispatch_explain_text_mode_renders_reason_indented() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    // Single artifact with no affected_files → guaranteed serial entry,
    // text mode must surface the reason as a sub-line of the ID. We strip
    // the template's default `## Affected Files` section so the dispatcher
    // truly sees nothing to bucket on.
    forgeplan()
        .args(["new", "prd", "Mystery"])
        .current_dir(tmp.path())
        .assert()
        .success();
    strip_affected_files(&tmp, "prds", "PRD-001");

    let output = forgeplan()
        .args(["dispatch", "--agents", "2"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    // FR-010 CLI rendering: serial entries print as
    //     PRD-XXX
    //       reason: <…>
    assert!(
        stdout.contains("reason:"),
        "text-mode serial must include `reason:` line, got:\n{stdout}"
    );
    assert!(
        stdout.contains("affected_files"),
        "missing-files reason must be surfaced, got:\n{stdout}"
    );
}
