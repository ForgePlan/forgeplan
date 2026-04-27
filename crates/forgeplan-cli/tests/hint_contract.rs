//! Integration test enforcing the PRD-071 hint contract across CLI commands.
//!
//! For every covered subcommand, this test runs the command in an isolated
//! temp workspace and asserts:
//!
//! 1. **Text mode**: stdout (or stderr on errors) carries one of the contract
//!    markers — `Next:`, `Or:`, `Wait:`, `Done.`, or `Fix:` — and contains no
//!    forbidden placeholders (e.g. `<id>`, `EVID-XXX`).
//! 2. **JSON mode** (where `--json` is supported): output parses and has a
//!    top-level `_next_action` field (string or null per the contract).
//!
//! The test acts as a CI guardrail: regressions to the agent protocol surface
//! immediately rather than after agents start hallucinating commands.
//!
//! Reference: `docs/methodology/agent-protocol.md`, PRD-071, Cycle 3 Z.

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Set up an isolated workspace with one PRD-001 so commands have data to
/// operate on. Mirrors `scripts/audit-hints.sh`.
fn setup_workspace() -> TempDir {
    let dir = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(dir.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Hint Contract Subject"])
        .current_dir(dir.path())
        .assert()
        .success();
    dir
}

/// Returns true if any line starts with a contract marker:
/// `Next:`, `Or:`, `Wait:`, `Done.`, or `Fix:`.
fn has_contract_marker(output: &str) -> bool {
    output.lines().any(|l| {
        let trimmed = l.trim_start();
        trimmed.starts_with("Next:")
            || trimmed.starts_with("Or:")
            || trimmed.starts_with("Wait:")
            || trimmed == "Done."
            || trimmed.starts_with("Fix:")
    })
}

/// Counts `Next:` markers — should be exactly 1 (multi-`Next:` violates the
/// "primary action" rule of the contract).
fn count_next_markers(output: &str) -> usize {
    output
        .lines()
        .filter(|l| l.trim_start().starts_with("Next:"))
        .count()
}

/// Returns true if the output contains a placeholder that is **forbidden**
/// in a hint — i.e. a stub the renderer should have filled in.
///
/// Allowed (agent-supplied) placeholders are accepted: `<verification>`,
/// `<title>`, `<parent-id>`, `<reason>`, `<until>`, `EVID-NNN`, `RFC-NNN`,
/// `<query>`. The forbidden list catches the cases where the *target ID*
/// was not substituted (clear contract violation).
fn has_forbidden_placeholder(output: &str) -> bool {
    let forbidden = ["<id>", "<this-id>", "<artifact>", "EVID-XXX", "RFC-XXX"];
    forbidden.iter().any(|p| output.contains(p))
}

/// Extract the marker lines (Next/Or/Wait/Done/Fix) from output. The contract
/// applies to these lines specifically — supporting prose and rationale text
/// is not the hint itself.
fn marker_lines(output: &str) -> Vec<&str> {
    output
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("Next:")
                || t.starts_with("Or:")
                || t.starts_with("Wait:")
                || t == "Done."
                || t.starts_with("Fix:")
        })
        .collect()
}

/// Combined stdout+stderr — many error paths (`activate` rejection, etc.) emit
/// the `Fix:` line on stderr while success paths emit `Next:` on stdout.
fn combined(out: &std::process::Output) -> String {
    let mut s = String::new();
    s.push_str(&String::from_utf8_lossy(&out.stdout));
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    s
}

/// Helper: run `args` in `ws`, return raw output.
fn run(ws: &TempDir, args: &[&str]) -> std::process::Output {
    forgeplan()
        .args(args)
        .current_dir(ws.path())
        .output()
        .expect("forgeplan invocation")
}

/// Assert text-mode contract: marker present, no forbidden placeholder
/// **on the marker line itself**, at most one `Next:`.
///
/// The forbidden-placeholder check is intentionally scoped to the marker
/// lines because hints are the contract surface — surrounding rationale
/// prose is allowed to use stub IDs (e.g. an example command in a tip).
fn assert_text_contract(label: &str, out: &std::process::Output) {
    let body = combined(out);
    assert!(
        has_contract_marker(&body),
        "[{label}] missing contract marker (Next:/Or:/Wait:/Done./Fix:):\n{body}",
    );
    for line in marker_lines(&body) {
        assert!(
            !has_forbidden_placeholder(line),
            "[{label}] marker line contains forbidden placeholder: {line}\n\nfull output:\n{body}",
        );
    }
    let next_count = count_next_markers(&body);
    assert!(
        next_count <= 1,
        "[{label}] expected at most one `Next:` marker (Or: is for fallbacks); got {next_count}:\n{body}",
    );
}

/// Assert JSON-mode contract: stdout parses as JSON object containing a
/// top-level `_next_action` field (string or null).
fn assert_json_contract(label: &str, out: &std::process::Output) {
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let v: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("[{label}] expected valid JSON: {e}\n\nbody:\n{stdout}"));
    let na = v
        .get("_next_action")
        .unwrap_or_else(|| panic!("[{label}] JSON missing top-level `_next_action`:\n{stdout}"));
    assert!(
        na.is_string() || na.is_null(),
        "[{label}] `_next_action` must be string or null, got {na:?}\n\nbody:\n{stdout}",
    );
    if let Some(s) = na.as_str() {
        assert!(
            !has_forbidden_placeholder(s),
            "[{label}] `_next_action` contains forbidden placeholder: {s}",
        );
    }
}

// ----------------------------------------------------------------------------
// Group A: lifecycle / multi-agent (activate, claim, dispatch, blocked)
// ----------------------------------------------------------------------------

#[test]
fn activate_emits_fix_on_blocked_text() {
    let ws = setup_workspace();
    // PRD-001 has no evidence and a stub body — activation must fail with
    // the `Fix:` remediation hint per contract.
    let out = run(&ws, &["activate", "PRD-001"]);
    assert!(!out.status.success(), "activate should fail on stub PRD");
    assert_text_contract("activate", &out);
}

#[test]
fn claim_emits_next_action_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["claim", "PRD-001", "--agent", "agent-A"]);
    assert!(out.status.success());
    assert_text_contract("claim", &out);
}

#[test]
fn claim_emits_next_action_json() {
    let ws = setup_workspace();
    let out = run(&ws, &["claim", "PRD-001", "--agent", "agent-A", "--json"]);
    assert!(out.status.success());
    assert_json_contract("claim --json", &out);
}

#[test]
fn dispatch_emits_next_action_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["dispatch", "--agents", "2"]);
    assert!(out.status.success());
    assert_text_contract("dispatch", &out);
}

#[test]
fn dispatch_emits_next_action_json() {
    let ws = setup_workspace();
    let out = run(&ws, &["dispatch", "--agents", "2", "--json"]);
    assert!(out.status.success());
    assert_json_contract("dispatch --json", &out);
}

#[test]
fn claims_emits_next_action_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["claims"]);
    assert!(out.status.success());
    assert_text_contract("claims", &out);
}

#[test]
fn claims_emits_next_action_json() {
    let ws = setup_workspace();
    let out = run(&ws, &["claims", "--json"]);
    assert!(out.status.success());
    assert_json_contract("claims --json", &out);
}

// PRD-071 Cycle 4 W1 fix: `forgeplan blocked` now emits `Done.` when the
// workspace has zero blocked artifacts (previously violated the contract
// with "No blocked artifacts. ..." prose only). JSON path emits
// `_next_action: null` per contract.
#[test]
fn blocked_emits_contract_marker_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["blocked"]);
    assert!(out.status.success());
    assert_text_contract("blocked", &out);
}

#[test]
fn blocked_emits_next_action_json() {
    let ws = setup_workspace();
    let out = run(&ws, &["blocked", "--json"]);
    assert!(out.status.success());
    assert_json_contract("blocked --json", &out);
}

// ----------------------------------------------------------------------------
// Group B: read / inspection (health, list, get, phase, new, journal)
// ----------------------------------------------------------------------------

#[test]
fn health_emits_next_action_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["health"]);
    assert!(out.status.success());
    assert_text_contract("health", &out);
}

#[test]
fn health_emits_next_action_json() {
    let ws = setup_workspace();
    let out = run(&ws, &["health", "--json"]);
    assert!(out.status.success());
    assert_json_contract("health --json", &out);
}

#[test]
fn list_emits_next_action_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["list"]);
    assert!(out.status.success());
    assert_text_contract("list", &out);
}

// PRD-071 Cycle 5 W5 fix: `list --json` keeps stdout as a bare JSON array
// for bw-compat (`jq '.[]'` consumers must keep working). The `Next:` hint
// is emitted to stderr per the additive marker rule. Contract still
// satisfied — agents reading combined stdout+stderr see the marker, and
// JSON parsers see only the array.
#[test]
fn list_emits_next_action_json() {
    let ws = setup_workspace();
    let out = run(&ws, &["list", "--json"]);
    assert!(out.status.success());

    // stdout MUST remain a bare JSON array (bw-compat for grep/jq scripts).
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let v: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("[list --json] expected valid JSON: {e}\n\nbody:\n{stdout}"));
    assert!(
        v.is_array(),
        "[list --json] stdout must be a bare array (bw-compat), got: {stdout}"
    );

    // The `Next:` hint must reach the agent — combined surface (stderr)
    // carries the contract marker.
    let body = combined(&out);
    assert!(
        has_contract_marker(&body),
        "[list --json] missing Next: marker on stderr:\n{body}",
    );
}

#[test]
fn get_emits_next_action_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["get", "PRD-001"]);
    assert!(out.status.success());
    assert_text_contract("get", &out);
}

#[test]
fn get_emits_next_action_json() {
    let ws = setup_workspace();
    let out = run(&ws, &["get", "PRD-001", "--json"]);
    assert!(out.status.success());
    assert_json_contract("get --json", &out);
}

#[test]
fn phase_emits_next_action_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["phase", "PRD-001"]);
    assert!(out.status.success());
    assert_text_contract("phase", &out);
}

#[test]
fn phase_emits_next_action_json() {
    let ws = setup_workspace();
    let out = run(&ws, &["phase", "PRD-001", "--json"]);
    assert!(out.status.success());
    assert_json_contract("phase --json", &out);
}

#[test]
fn new_emits_next_action_text() {
    let ws = setup_workspace();
    // A second PRD so we can validate the hint without colliding with the
    // setup PRD-001.
    let out = run(&ws, &["new", "prd", "Second Subject"]);
    assert!(out.status.success());
    assert_text_contract("new prd", &out);
}

#[test]
fn journal_emits_next_action_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["journal"]);
    assert!(out.status.success());
    assert_text_contract("journal", &out);
}

// ----------------------------------------------------------------------------
// Group C: methodology (score, review, search, validate, status)
// ----------------------------------------------------------------------------

#[test]
fn score_emits_next_action_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["score", "PRD-001"]);
    assert!(out.status.success());
    assert_text_contract("score", &out);
}

#[test]
fn score_emits_next_action_json() {
    let ws = setup_workspace();
    let out = run(&ws, &["score", "PRD-001", "--json"]);
    assert!(out.status.success());
    assert_json_contract("score --json", &out);
}

#[test]
fn review_emits_next_action_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["review", "PRD-001"]);
    assert!(out.status.success());
    assert_text_contract("review", &out);
}

#[test]
fn search_emits_next_action_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["search", "subject"]);
    assert!(out.status.success());
    assert_text_contract("search", &out);
}

#[test]
fn search_emits_next_action_json() {
    let ws = setup_workspace();
    let out = run(&ws, &["search", "subject", "--json"]);
    assert!(out.status.success());
    assert_json_contract("search --json", &out);
}

#[test]
fn validate_emits_next_action_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["validate", "PRD-001"]);
    assert!(out.status.success());
    assert_text_contract("validate", &out);
}

#[test]
fn validate_emits_next_action_json() {
    let ws = setup_workspace();
    let out = run(&ws, &["validate", "PRD-001", "--json"]);
    assert!(out.status.success());
    assert_json_contract("validate --json", &out);
}

#[test]
fn status_emits_next_action_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["status"]);
    assert!(out.status.success());
    assert_text_contract("status", &out);
}

#[test]
fn order_emits_next_action_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["order"]);
    assert!(out.status.success());
    assert_text_contract("order", &out);
}

#[test]
fn order_emits_next_action_json() {
    let ws = setup_workspace();
    let out = run(&ws, &["order", "--json"]);
    assert!(out.status.success());
    assert_json_contract("order --json", &out);
}

// ----------------------------------------------------------------------------
// Group D: PRD-071 Cycle 4 — newly fixed commands (W1/W2)
// ----------------------------------------------------------------------------
//
// These tests cover commands fixed in Cycle 4 of PRD-071:
//   - `route`               — emits Next: with top-of-pipeline artifact
//   - `update` (error)      — emits Fix: with concrete activate command
//   - `score` (rationale)   — substitutes EVID-NNN, never emits EVID-XXX
//   - `fpf rules`           — substitutes a real artifact ID, no `<id>` stub
//
// Plus contract-only smoke tests for LLM-dependent commands that fall through
// to the no-LLM error branch in test envs (no API key seeded). Success paths
// for those commands require a live LLM and are out of scope here.

// Group D.1: `route`

#[test]
fn route_emits_next_action_text() {
    let ws = setup_workspace();
    // "design new authentication system" should escalate beyond Tactical and
    // produce a non-empty pipeline → Next: line emitted.
    let out = run(&ws, &["route", "design new authentication system"]);
    assert!(out.status.success());
    assert_text_contract("route", &out);
}

// Group D.2: `update` error path

#[test]
fn update_emits_fix_on_invalid_status_change() {
    let ws = setup_workspace();
    // Direct status→active is blocked by lifecycle gates. Per contract the
    // error must include a `Fix:` line with the concrete remediation command
    // (`forgeplan activate <id>`).
    let out = run(&ws, &["update", "PRD-001", "--status", "active"]);
    assert!(
        !out.status.success(),
        "update --status active must fail (lifecycle gate)"
    );
    let body = combined(&out);
    assert!(
        body.contains("Fix:"),
        "update error missing Fix: line:\n{body}"
    );
    assert!(
        body.contains("forgeplan activate PRD-001"),
        "Fix: line missing concrete remediation command:\n{body}"
    );
}

// Group D.3: `score` rationale must not contain the legacy `EVID-XXX`
// placeholder (renderer should substitute `EVID-NNN`, the agent-supplied
// stub form per the contract).

#[test]
fn score_does_not_emit_legacy_evid_xxx_placeholder() {
    let ws = setup_workspace();
    let out = run(&ws, &["score", "PRD-001"]);
    let body = combined(&out);
    assert!(
        !body.contains("EVID-XXX"),
        "score emits legacy `EVID-XXX` placeholder (must be `EVID-NNN`):\n{body}"
    );
}

// Group D.4: `fpf rules`

#[test]
fn fpf_rules_emits_next_action_text() {
    let ws = setup_workspace();
    let out = run(&ws, &["fpf", "rules"]);
    assert!(out.status.success());
    assert_text_contract("fpf rules", &out);
    let body = combined(&out);
    assert!(
        !body.contains("forgeplan fpf check <id>"),
        "fpf rules still emits `<id>` placeholder (must be a real artifact ID):\n{body}"
    );
}

// Group D.5: LLM-dependent commands — no-LLM (Fix:) branch contract.
//
// In test envs no `llm:` block is configured, so `require_llm_config()` bails.
// The contract requires that bail to land with a `Fix:` line so agents can
// recover deterministically. We only verify the contract marker, not the
// specific text — W2 may phrase the remediation differently.

#[test]
fn reason_emits_contract_marker_without_llm() {
    let ws = setup_workspace();
    let out = run(&ws, &["reason", "PRD-001"]);
    assert!(
        !out.status.success(),
        "reason should fail without LLM config in test env"
    );
    assert_text_contract("reason (no LLM)", &out);
}

#[test]
fn decompose_emits_contract_marker_without_llm() {
    let ws = setup_workspace();
    let out = run(&ws, &["decompose", "PRD-001"]);
    assert!(
        !out.status.success(),
        "decompose should fail without LLM config in test env"
    );
    assert_text_contract("decompose (no LLM)", &out);
}

#[test]
fn generate_emits_contract_marker_without_llm() {
    let ws = setup_workspace();
    // `generate` requires a template-key + description; missing args still
    // route through `require_llm_config()` first per current command shape.
    let out = run(&ws, &["generate", "prd", "Test description"]);
    assert!(
        !out.status.success(),
        "generate should fail without LLM config in test env"
    );
    assert_text_contract("generate (no LLM)", &out);
}
