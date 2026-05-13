//! PRD-077 FR-007 / CR-C4 — `forgeplan reason --help` must surface the LLM
//! requirement. Without this users see only "Analyze an artifact using FPF
//! ADI reasoning cycle" and don't realise they need an API key in
//! `.forgeplan/secrets.env`. Regression fixture against the discovery gap
//! identified by the 5-agent research panel on 2026-05-13.
//!
//! CR-C4 audit closure (2026-05-14): the file is `secrets.env` (dotenv
//! convention), not `secrets.yaml`. The body is shell `export` syntax,
//! which `.yaml` linters reject; the rename is part of W1.5 F3 fixes.

use assert_cmd::Command;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

#[test]
fn reason_help_mentions_llm_and_env_var_and_secrets_file() {
    let output = forgeplan().args(["reason", "--help"]).output().unwrap();
    assert!(
        output.status.success(),
        "reason --help should exit 0; status={:?}",
        output.status
    );
    let stdout = String::from_utf8(output.stdout).unwrap();

    // The long_about must mention at least two of:
    //   - LLM (the requirement)
    //   - GEMINI_API_KEY or api_key_env (the env var pointer)
    //   - secrets.env (where to put the key — dotenv convention, CR-C4)
    let mentions_llm = stdout.contains("LLM");
    let mentions_env = stdout.contains("GEMINI_API_KEY") || stdout.contains("api_key_env");
    let mentions_secrets = stdout.contains("secrets.env");

    let hits = [mentions_llm, mentions_env, mentions_secrets]
        .iter()
        .filter(|b| **b)
        .count();
    assert!(
        hits >= 2,
        "FR-007 contract: reason --help must mention >=2 of {{LLM, GEMINI_API_KEY/api_key_env, secrets.env}}; \
         got LLM={mentions_llm} env={mentions_env} secrets={mentions_secrets}\n\nfull output:\n{stdout}"
    );

    // Defensive — assert all three really, the bar in the spec is >=2 but
    // we want the canonical experience for agents: clear "LLM required",
    // clear env var name, clear file path. If any future refactor drops
    // one, this catches it.
    assert!(
        mentions_llm && mentions_env && mentions_secrets,
        "FR-007 strong contract: all three markers should be present.\n\
         LLM={mentions_llm} env={mentions_env} secrets={mentions_secrets}\n\
         full output:\n{stdout}"
    );
}

#[test]
fn reason_help_clarifies_it_is_the_only_llm_command() {
    // Discoverability: agents seeing `--help` should learn that *other*
    // forgeplan commands are pure-Rust and do NOT need an LLM, so a
    // missing `GEMINI_API_KEY` is specifically a `reason` problem, not a
    // workspace-wide blocker.
    let output = forgeplan().args(["reason", "--help"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.to_ascii_lowercase().contains("only") && stdout.to_ascii_lowercase().contains("llm"),
        "FR-007: help text should clarify reason is the ONLY LLM-requiring command. \
         full output:\n{stdout}"
    );
}
