//! `forgeplan get` must report the artifact's edges — #447.
//!
//! The defect: `get` fetched both edge sets, collapsed them into a boolean to
//! pick a hint, and dropped them. An artifact with five links and an orphan
//! rendered identically, and the issue reports that costing a wrong conclusion
//! in a live session.
//!
//! These tests run the real binary against a real workspace, because the
//! defect was invisible to unit tests: every layer worked, the output just did
//! not carry the answer.

use assert_cmd::Command;
use tempfile::TempDir;

fn fpl(ws: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("forgeplan").unwrap();
    cmd.current_dir(ws.path());
    cmd
}

fn init(ws: &TempDir) {
    fpl(ws).args(["init", "-y"]).output().expect("init");
}

fn new_artifact(ws: &TempDir, kind: &str, title: &str) -> String {
    let out = fpl(ws).args(["new", kind, title]).output().expect("new");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // The `ID:` line is indented (`  ID:      ADR-001`), so trim before
    // matching. Parsing this line rather than grepping for an id-shaped token
    // matters: the hint lines below it also contain the id.
    stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix("ID:"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| panic!("no `ID:` line in `new {kind}` output:\n{stdout}"))
}

fn get_json(ws: &TempDir, id: &str) -> serde_json::Value {
    let out = fpl(ws).args(["get", id, "--json"]).output().expect("get");
    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("get --json not JSON ({e}):\n{stdout}"))
}

/// The core of #447: an unlinked artifact must say so explicitly. An absent
/// field and an empty one are the same thing to a caller, which is exactly how
/// the orphan/unreported confusion arose.
#[test]
fn an_artifact_without_links_reports_empty_arrays_not_a_missing_field() {
    let ws = TempDir::new().unwrap();
    init(&ws);
    let id = new_artifact(&ws, "note", "Orphan");

    let v = get_json(&ws, &id);
    assert!(
        v.get("links").is_some(),
        "links must always be present, got keys: {:?}",
        v.as_object().map(|o| o.keys().collect::<Vec<_>>())
    );
    assert_eq!(v["links"]["outbound"].as_array().map(Vec::len), Some(0));
    assert_eq!(v["links"]["inbound"].as_array().map(Vec::len), Some(0));
}

/// Both directions are reported, and from the right side. Inbound is the half
/// `forgeplan graph | grep "<id> -->"` cannot answer, and it is the one behind
/// "which evidence supports this".
#[test]
fn get_reports_outbound_and_inbound_edges_separately() {
    let ws = TempDir::new().unwrap();
    init(&ws);
    let prd = new_artifact(&ws, "prd", "Parent");
    let spec = new_artifact(&ws, "spec", "Child");
    let evid = new_artifact(&ws, "evidence", "Measurement");

    fpl(&ws)
        .args(["link", &spec, &prd, "--relation", "refines"])
        .output()
        .expect("link spec->prd");
    fpl(&ws)
        .args(["link", &evid, &spec, "--relation", "informs"])
        .output()
        .expect("link evid->spec");

    let v = get_json(&ws, &spec);
    let outbound = v["links"]["outbound"].as_array().expect("outbound array");
    let inbound = v["links"]["inbound"].as_array().expect("inbound array");

    assert_eq!(outbound.len(), 1, "expected one outbound edge: {v}");
    assert_eq!(outbound[0]["target"], serde_json::json!(prd));
    assert_eq!(outbound[0]["relation"], serde_json::json!("refines"));

    assert_eq!(inbound.len(), 1, "expected one inbound edge: {v}");
    assert_eq!(inbound[0]["source"], serde_json::json!(evid));
    assert_eq!(inbound[0]["relation"], serde_json::json!("informs"));
}

/// The human path must not disagree with the JSON path — the same question
/// answered two ways is how a reader and an agent end up with different
/// pictures of the same artifact.
#[test]
fn human_output_reports_the_same_edges_as_json() {
    let ws = TempDir::new().unwrap();
    init(&ws);
    let prd = new_artifact(&ws, "prd", "Parent");
    let spec = new_artifact(&ws, "spec", "Child");
    fpl(&ws)
        .args(["link", &spec, &prd, "--relation", "refines"])
        .output()
        .expect("link");

    let out = fpl(&ws).args(["get", &spec]).output().expect("get");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Links out") && stdout.contains(&prd),
        "human output must name the edge, got:\n{stdout}"
    );

    let orphan = new_artifact(&ws, "note", "Orphan");
    let out = fpl(&ws).args(["get", &orphan]).output().expect("get");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Links") && stdout.contains("none"),
        "an unlinked artifact must say so rather than omit the row, got:\n{stdout}"
    );
}
