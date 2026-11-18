//! Regression tests for the "evidence is not a dependency" scoring rule and
//! for breakdown numbers matching the numbers actually used.
//!
//! Background. `r_eff_recursive` harvests linked evidence in BOTH directions
//! for the self-score, and separately walks OUTGOING `informs` / `based_on` /
//! `refines` / `depends_on` edges as dependencies. An edge that points from a
//! decision INTO an evidence pack satisfied both rules at once, so the same
//! pack was credited (as evidence) and recursed into (as a dependency). A pack
//! usually has no evidence of its own — it does only when another pack links
//! into it — so that recursion returned 0.0, and the weakest-link `min` picked
//! the zero: an artifact was driven to R_eff 0.00 *because* it carried
//! evidence.
//!
//! The defect was completely uncovered: no test in the workspace linked a
//! decision outward to an ACTIVE pack. Every existing test left the pack in
//! `draft`, and the dependency loop skips non-active dependencies, which masked
//! it perfectly. That mask is why the trap below is the sharpest test here —
//! the score is fine until you activate the evidence, and then it collapses.
//!
//! Covers three defects of one class ("the number shown is not the number
//! used"): the dependency double-count, the per-item breakdown showing the raw
//! score while the min consumed the type-modified one, and a confidence
//! interval derived from a different formula than the R_eff it bracketed.
//!
//! Two of the tests here are not about the fix at all, and are the ones most
//! worth keeping: `non_evidence_dependency_still_caps_the_score` fails if the
//! dependency walk is deleted outright (every other test in this file survives
//! that), and `confidence_interval_tracks_the_recursive_score_not_the_flat_one`
//! is the only case where the flat and recursive scorers disagree, so it is the
//! only one that can actually see the confidence-interval defect.

use assert_cmd::Command;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init_workspace() -> TempDir {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    tmp
}

/// Path of the single evidence file in a fresh workspace. Globbed rather than
/// hardcoded so a template rename does not silently turn these into no-ops.
fn sole_evidence_path(tmp: &TempDir) -> std::path::PathBuf {
    let dir = tmp.path().join(".forgeplan/evidence");
    let mut found: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("evidence dir {dir:?} unreadable: {e}"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one evidence file, found {found:?}"
    );
    found.pop().unwrap()
}

fn score_json(tmp: &TempDir, id: &str) -> serde_json::Value {
    let output = forgeplan()
        .args(["score", id, "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "score {id} failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("score --json must emit valid JSON")
}

/// Build a workspace holding PRD-001 plus one active evidence pack, wired with
/// the OUTGOING edge (`PRD --relation--> EVID`) that used to zero the score.
fn workspace_with_outgoing_edge(relation: &str) -> TempDir {
    let tmp = init_workspace();
    forgeplan()
        .args(["new", "prd", "Target"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "evidence", "Proof"])
        .current_dir(tmp.path())
        .assert()
        .success();
    // Decision -> evidence. This is the direction under test; the canonical
    // direction (evidence -> decision) was never affected.
    forgeplan()
        .args(["link", "PRD-001", "EVID-001", "--relation", relation])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["activate", "EVID-001"])
        .current_dir(tmp.path())
        .assert()
        .success();
    tmp
}

/// The walk must be a pure function of the graph plus the starting id.
///
/// Neither `get_relations` nor `list_records` issues an ORDER BY, so row order
/// is whatever storage returns — stable in practice, but not contractual. Order
/// is observable in the breakdown, in `factors`, in `weakest_link` tie-breaking,
/// and (because a revisited node returns a neutral 1.0) in the score itself when
/// two paths of different relation types reach the same node.
///
/// The dependency walk is where the sort has teeth: `get_relations` returns
/// edges in link-creation order, which is unrelated to id order. This test
/// creates the links in DESCENDING id order, so it fails if the sort is
/// removed. (The evidence-side sort is defensive rather than load-bearing —
/// `list_records` happens to return creation order, which for sequentially
/// assigned ids is already ascending.)
#[test]
fn dependency_walk_order_is_fixed_regardless_of_link_creation_order() {
    let tmp = init_workspace();
    // Titles must be pairwise dissimilar — `new` refuses a near-duplicate title
    // in a non-interactive shell, and "Dep A" / "Dep B" score 100% similar.
    for title in ["Target", "Storage layer", "Auth flow", "Search index"] {
        forgeplan()
            .args(["new", "prd", title])
            .current_dir(tmp.path())
            .assert()
            .success();
    }
    forgeplan()
        .args(["new", "evidence", "Proof"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["link", "EVID-001", "PRD-001", "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["activate", "EVID-001"])
        .current_dir(tmp.path())
        .assert()
        .success();
    // The dependencies stay `draft` on purpose. A non-active dependency is
    // skipped, but the skip is still recorded in `factors` in walk order, which
    // is exactly the signal under test — and it keeps the fixture free of the
    // activation step, which is not what this test is about.
    //
    // Reverse of id order: 004, 003, 002.
    for id in ["PRD-004", "PRD-003", "PRD-002"] {
        forgeplan()
            .args(["link", "PRD-001", id, "--relation", "based_on"])
            .current_dir(tmp.path())
            .assert()
            .success();
    }

    let first = score_json(&tmp, "PRD-001");
    let factors: Vec<String> = first["factors"]
        .as_array()
        .expect("factors array")
        .iter()
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .collect();
    let mentioned: Vec<String> = factors
        .iter()
        .filter_map(|f| {
            ["PRD-002", "PRD-003", "PRD-004"]
                .iter()
                .find(|id| f.contains(*id))
                .map(|id| id.to_string())
        })
        .collect();
    assert_eq!(
        mentioned,
        vec!["PRD-002", "PRD-003", "PRD-004"],
        "dependencies must be walked in a fixed order, not in the order their \
         links happened to be created; factors were {factors:?}"
    );

    // And the whole payload must be reproducible.
    let second = score_json(&tmp, "PRD-001");
    assert_eq!(
        first, second,
        "scoring the same unchanged artifact twice must return an identical payload"
    );
}

/// `PRD --informs--> EVID` (active): the pack must be scored as evidence, and
/// must not simultaneously be walked as a zero-scoring dependency.
#[test]
fn outgoing_informs_into_active_evidence_does_not_zero_the_score() {
    let tmp = workspace_with_outgoing_edge("informs");
    let json = score_json(&tmp, "PRD-001");
    let r_eff = json["r_eff"].as_f64().expect("r_eff number");
    assert!(
        r_eff > 0.0,
        "an artifact must not be punished for carrying evidence: template pack is \
         supports/CL3/measurement, so R_eff must be 1.00, got {r_eff}"
    );
    assert!(
        (r_eff - 1.0).abs() < f64::EPSILON,
        "supports + CL3 + measurement = 1.00, got {r_eff}"
    );
}

/// Same rule for `based_on`, the other relation the live graph used here.
#[test]
fn outgoing_based_on_into_active_evidence_does_not_zero_the_score() {
    let tmp = workspace_with_outgoing_edge("based_on");
    let json = score_json(&tmp, "PRD-001");
    let r_eff = json["r_eff"].as_f64().expect("r_eff number");
    assert!(
        (r_eff - 1.0).abs() < f64::EPSILON,
        "based_on into evidence must score identically to informs, got {r_eff}"
    );
}

/// The trap that hid this defect for so long: while the pack is `draft` the
/// dependency loop skips it and the score looks healthy. Activating the pack —
/// the exact step the methodology tells you to take — used to destroy the score
/// of the artifact the pack backs. Activation must be monotonic here.
#[test]
fn activating_evidence_must_not_destroy_the_score_it_backs() {
    let tmp = init_workspace();
    forgeplan()
        .args(["new", "prd", "Target"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "evidence", "Proof"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["link", "PRD-001", "EVID-001", "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let before = score_json(&tmp, "PRD-001")["r_eff"]
        .as_f64()
        .expect("r_eff before");

    forgeplan()
        .args(["activate", "EVID-001"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let after = score_json(&tmp, "PRD-001")["r_eff"]
        .as_f64()
        .expect("r_eff after");

    assert!(
        after >= before,
        "activating evidence dropped the backed artifact from {before} to {after} — \
         following the documented workflow must never lower the score"
    );
    assert!(
        after > 0.0,
        "after activation R_eff collapsed to {after}; the pack is active, \
         supports, CL3 and linked"
    );
}

/// The reported confidence interval must bracket the reported R_eff. The band
/// used to be derived from the flat `r_eff()` (no dependency walk, no
/// evidence-type modifier) while `r_eff` came from the recursive engine, so a
/// single payload could carry `r_eff: 0.00` beside `r_eff_ci.point: 1.00`.
#[test]
fn confidence_interval_brackets_the_reported_r_eff() {
    let tmp = workspace_with_outgoing_edge("based_on");
    let json = score_json(&tmp, "PRD-001");
    let r_eff = json["r_eff"].as_f64().expect("r_eff number");
    let point = json["r_eff_ci"]["point"].as_f64().expect("ci point");
    let low = json["r_eff_ci"]["low"].as_f64().expect("ci low");
    let high = json["r_eff_ci"]["high"].as_f64().expect("ci high");

    assert!(
        (point - r_eff).abs() < f64::EPSILON,
        "CI point {point} must be the same quantity as r_eff {r_eff}"
    );
    assert!(
        low <= r_eff && r_eff <= high,
        "CI [{low} — {high}] must contain r_eff {r_eff}"
    );
}

/// Rewrite the evidence pack's verdict in place, then reindex.
fn set_verdict(tmp: &TempDir, evid_file_contains: &str, verdict: &str) {
    let dir = tmp.path().join(".forgeplan/evidence");
    let path = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains(evid_file_contains))
        })
        .unwrap_or_else(|| panic!("no evidence file matching {evid_file_contains:?} in {dir:?}"));
    let body = std::fs::read_to_string(&path).unwrap();
    assert!(
        body.contains("verdict: supports"),
        "template no longer carries `verdict: supports`; this helper would be a no-op"
    );
    std::fs::write(
        &path,
        body.replace("verdict: supports", &format!("verdict: {verdict}")),
    )
    .unwrap();
    forgeplan()
        .args(["reindex"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

/// The dependency walk itself must stay alive. Excluding evidence from it is a
/// narrow carve-out, not permission to stop capping an artifact by the
/// NON-evidence artifacts it rests on — delete the walk entirely and every
/// other test in this file still passes, so this is the one that notices.
#[test]
fn non_evidence_dependency_still_caps_the_score() {
    let tmp = init_workspace();
    for (kind, title) in [("prd", "Target"), ("prd", "Dep")] {
        forgeplan()
            .args(["new", kind, title])
            .current_dir(tmp.path())
            .assert()
            .success();
    }
    // PRD-001 gets a perfect pack of its own: supports / CL3 / measurement.
    forgeplan()
        .args(["new", "evidence", "Strong"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["link", "EVID-001", "PRD-001", "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["activate", "EVID-001"])
        .current_dir(tmp.path())
        .assert()
        .success();
    // PRD-002 gets a weakens pack, so it scores 0.5 and can cap its dependents.
    forgeplan()
        .args(["new", "evidence", "Weak"])
        .current_dir(tmp.path())
        .assert()
        .success();
    set_verdict(&tmp, "weak", "weakens");
    forgeplan()
        .args(["link", "EVID-002", "PRD-002", "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["activate", "EVID-002"])
        .current_dir(tmp.path())
        .assert()
        .success();
    // Template bodies do not clear the methodology gate; --force is fine here
    // because this test is about scoring, not about the activation gate.
    forgeplan()
        .args(["activate", "PRD-002", "--force"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["link", "PRD-001", "PRD-002", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let json = score_json(&tmp, "PRD-001");
    let r_eff = json["r_eff"].as_f64().expect("r_eff number");
    // PRD-001's own evidence is 1.00; PRD-002 scores 0.5 and `based_on` is CL2,
    // so the cap is 0.5 - 0.1 = 0.4. If the dependency walk were removed the
    // answer would be 1.00.
    assert!(
        (r_eff - 0.4).abs() < 1e-9,
        "a non-evidence dependency must still cap the score: expected 0.40, got {r_eff}"
    );
    assert_eq!(
        json["weakest_link"].as_str(),
        Some("PRD-002"),
        "the cap must be attributed to the dependency that imposed it"
    );
}

/// The confidence band must follow the RECURSIVE score, and this is the case
/// that proves it: an `audit` pack scores 1.0 under the flat helper (no type
/// modifier) but 0.8 under the engine. Before the fix the band was derived from
/// the flat helper, so `point` read 1.0 beside an `r_eff` of 0.8. The other CI
/// test cannot see this — its pack is `measurement`, where the two agree.
#[test]
fn confidence_interval_tracks_the_recursive_score_not_the_flat_one() {
    let tmp = init_workspace();
    forgeplan()
        .args(["new", "prd", "Target"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "evidence", "External audit"])
        .current_dir(tmp.path())
        .assert()
        .success();
    let evid_path = sole_evidence_path(&tmp);
    let body = std::fs::read_to_string(&evid_path).unwrap();
    std::fs::write(
        &evid_path,
        body.replace("evidence_type: measurement", "evidence_type: audit"),
    )
    .unwrap();
    forgeplan()
        .args(["reindex"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["link", "EVID-001", "PRD-001", "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["activate", "EVID-001"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let json = score_json(&tmp, "PRD-001");
    let r_eff = json["r_eff"].as_f64().expect("r_eff");
    let point = json["r_eff_ci"]["point"].as_f64().expect("ci point");
    assert!(
        (r_eff - 0.8).abs() < 1e-9,
        "supports/CL3/audit must score 1.0 - 0.2 = 0.80, got {r_eff}"
    );
    assert!(
        (point - 0.8).abs() < 1e-9,
        "the band must be centred on the recursive score 0.80, not on the flat \
         helper's 1.00; got {point}"
    );
}

/// ADR-020's never-silent discipline, inherited: the exclusion must leave a
/// trace an operator can read, not just change a number.
#[test]
fn skipping_evidence_as_a_dependency_is_recorded_in_factors() {
    let tmp = workspace_with_outgoing_edge("based_on");
    let json = score_json(&tmp, "PRD-001");
    let factors: Vec<String> = json["factors"]
        .as_array()
        .expect("factors array")
        .iter()
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .collect();
    assert!(
        factors
            .iter()
            .any(|f| f.contains("EVID-001") && f.contains("not a dependency")),
        "the skip must be auditable; factors were {factors:?}"
    );
}

/// An `audit` pack carries a 0.2 evidence-type deduction. The breakdown used to
/// print the value BEFORE that deduction while R_eff was computed after it, so
/// `= 0.5` appeared above `R_eff: 0.30` with nothing to explain the gap. The
/// payload must now let a caller reconstruct the arithmetic unaided.
#[test]
fn audit_evidence_breakdown_reports_the_score_actually_used() {
    let tmp = init_workspace();
    forgeplan()
        .args(["new", "prd", "Target"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "evidence", "External audit"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let evid_path = sole_evidence_path(&tmp);
    let body = std::fs::read_to_string(&evid_path).unwrap();
    assert!(
        body.contains("evidence_type: measurement"),
        "template no longer carries the expected evidence_type line; \
         this test would silently stop exercising the audit modifier"
    );
    let body = body.replace("evidence_type: measurement", "evidence_type: audit");
    std::fs::write(&evid_path, body).unwrap();
    forgeplan()
        .args(["reindex"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Canonical direction here — this test is about the rendered numbers, not
    // about edge direction.
    forgeplan()
        .args(["link", "EVID-001", "PRD-001", "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["activate", "EVID-001"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let json = score_json(&tmp, "PRD-001");
    let item = &json["evidence"][0];
    let score = item["score"].as_f64().expect("effective score");
    let score_raw = item["score_raw"].as_f64().expect("raw score");
    let type_modifier = item["type_modifier"].as_f64().expect("type modifier");
    let r_eff = json["r_eff"].as_f64().expect("r_eff number");

    assert!(
        (type_modifier - 0.2).abs() < 1e-9,
        "audit evidence must carry a 0.2 deduction, got {type_modifier}"
    );
    assert!(
        (score_raw - 1.0).abs() < 1e-9,
        "supports + CL3 own-merit score must be 1.0, got {score_raw}"
    );
    assert!(
        (score - (score_raw - type_modifier)).abs() < 1e-9,
        "the payload must be self-consistent: score {score} != score_raw \
         {score_raw} - type_modifier {type_modifier}"
    );
    assert!(
        (r_eff - score).abs() < 1e-9,
        "single-pack R_eff {r_eff} must equal the effective score shown for \
         that pack ({score}) — the breakdown is the explanation of the rollup"
    );
}
