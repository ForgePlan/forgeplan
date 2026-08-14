//! ADR-020 / #436 regression — a superseded refutes pack must no longer pin
//! R_eff to 0 forever. Drives the REAL binary through the exact PRD-177 chain
//! from the upstream report: active refutes → 0.00 (unchanged guardrail),
//! honest displacement via `supersede --by` → score recovers, the displaced
//! pack stays visible marked "excluded from min".

use assert_cmd::Command;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn run_ok(tmp: &TempDir, args: &[&str]) -> String {
    let out = forgeplan()
        .args(args)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "forgeplan {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn new_artifact(tmp: &TempDir, kind: &str, title: &str) -> String {
    let stdout = run_ok(tmp, &["new", kind, title, "--allow-duplicate"]);
    // Parse the `ID:` line ONLY — a blind EVID-\d+ grep can hit the
    // similarity-guard warning line instead (batch-merge lesson).
    for line in stdout.lines() {
        if let Some(id) = line.trim().strip_prefix("ID:") {
            return id.trim().to_string();
        }
    }
    panic!("no ID parsed from new {kind}: {stdout}");
}

fn set_body(tmp: &TempDir, id: &str, body: &str) {
    let f = tmp.path().join(format!("{id}-body.md"));
    std::fs::write(&f, body).unwrap();
    run_ok(tmp, &["update", id, "--body", &format!("@{}", f.display())]);
}

fn score_json(tmp: &TempDir, id: &str) -> serde_json::Value {
    let stdout = run_ok(tmp, &["score", id, "--json"]);
    serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("bad score JSON: {e}\n{stdout}"))
}

const REFUTES_BODY: &str = "## Summary\n\ntypecheck fails\n\n## Structured Fields\n\nverdict: refutes\ncongruence_level: 3\nevidence_type: test\n";
const SUPPORTS_BODY: &str = "## Summary\n\nfixed, re-verified on a later commit\n\n## Structured Fields\n\nverdict: supports\ncongruence_level: 3\nevidence_type: test\n";

/// The PRD-177 chain: refutes (active) pins to 0; supersede --by the
/// re-verification pack → R_eff recovers; the displaced pack stays listed
/// as excluded.
#[test]
fn superseded_refutes_no_longer_pins_score() {
    let tmp = TempDir::new().unwrap();
    run_ok(&tmp, &["init", "-y"]);

    let prd = new_artifact(&tmp, "prd", "PRD-177 repro");
    let refutes = new_artifact(&tmp, "evidence", "typecheck BLOCKER found");
    set_body(&tmp, &refutes, REFUTES_BODY);
    run_ok(&tmp, &["link", &refutes, &prd, "--relation", "informs"]);
    run_ok(&tmp, &["activate", &refutes, "--force"]);

    // Guardrail unchanged: an ACTIVE refutes pack zeroes the score.
    let before = score_json(&tmp, &prd);
    assert_eq!(
        before["r_eff"].as_f64().unwrap(),
        0.0,
        "active refutes must still zero the score"
    );

    // The laundering shortcut must be CLOSED: a raw status write may not
    // displace the pack (audit BLOCKER — it bypassed transition validation,
    // successor, edge and journal).
    let out = forgeplan()
        .args(["update", &refutes, "--status", "superseded"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "update --status superseded must be rejected"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("supersede"),
        "rejection must redirect to the lifecycle verb: {err}"
    );
    assert_eq!(
        score_json(&tmp, &prd)["r_eff"].as_f64().unwrap(),
        0.0,
        "the blocked write must not have changed the score"
    );

    // Honest displacement: new supports evidence + supersede --by.
    let supports = new_artifact(&tmp, "evidence", "re-verification on later commit passes");
    set_body(&tmp, &supports, SUPPORTS_BODY);
    run_ok(&tmp, &["link", &supports, &prd, "--relation", "informs"]);
    let supersede_out = run_ok(&tmp, &["supersede", &refutes, "--by", &supports]);
    // ADR-020 audit MAJOR: displacement refreshes the informed artifact's
    // CACHED score immediately — no manual `forgeplan score` required.
    assert!(
        supersede_out.contains(&format!("Rescored {prd}")),
        "supersede must rescore the informed artifact: {supersede_out}"
    );

    let after = score_json(&tmp, &prd);
    assert_eq!(
        after["r_eff"].as_f64().unwrap(),
        1.0,
        "R_eff must recover once the refutes pack is displaced (ADR-020); got {after}"
    );

    // The displaced pack stays VISIBLE, flagged excluded with its raw score.
    let ev = after["evidence"].as_array().unwrap();
    let displaced = ev
        .iter()
        .find(|e| e["id"].as_str() == Some(refutes.as_str()))
        .expect("superseded pack must still be listed");
    assert_eq!(displaced["excluded"].as_bool(), Some(true));
    assert_eq!(displaced["status"].as_str(), Some("superseded"));
    let live = ev
        .iter()
        .find(|e| e["id"].as_str() == Some(supports.as_str()))
        .expect("successor pack listed");
    assert_eq!(live["excluded"].as_bool(), Some(false));

    // Factors carry the audit trail of the skip.
    let factors = after["factors"].to_string();
    assert!(
        factors.contains(&format!("Skipped {refutes}")),
        "skip must be logged in factors: {factors}"
    );

    // Text mode marks the exclusion for humans.
    let text = run_ok(&tmp, &["score", &prd]);
    assert!(
        text.contains("excluded from min"),
        "text display must mark the displaced pack: {text}"
    );
}

/// All linked evidence displaced → "no active evidence" (0.0), never the
/// displaced pack's own score. Recovery requires the REPLACEMENT to be
/// linked to the artifact — a bare supersede flag is not enough.
#[test]
fn all_terminal_degrades_to_no_active_evidence() {
    let tmp = TempDir::new().unwrap();
    run_ok(&tmp, &["init", "-y"]);

    let prd = new_artifact(&tmp, "prd", "All terminal case");
    let only = new_artifact(&tmp, "evidence", "sole supports pack");
    set_body(&tmp, &only, SUPPORTS_BODY);
    run_ok(&tmp, &["link", &only, &prd, "--relation", "informs"]);
    run_ok(&tmp, &["activate", &only, "--force"]);

    let before = score_json(&tmp, &prd);
    assert_eq!(before["r_eff"].as_f64().unwrap(), 1.0);

    // Successor exists but is NOT linked to the PRD.
    let successor = new_artifact(&tmp, "evidence", "unlinked successor");
    set_body(&tmp, &successor, SUPPORTS_BODY);
    run_ok(&tmp, &["supersede", &only, "--by", &successor]);

    let after = score_json(&tmp, &prd);
    assert_eq!(
        after["r_eff"].as_f64().unwrap(),
        0.0,
        "all-terminal must degrade to no-active-evidence, got {after}"
    );
    let factors = after["factors"].to_string();
    assert!(
        factors.contains("No active evidence"),
        "factors must explain the degradation: {factors}"
    );
}
