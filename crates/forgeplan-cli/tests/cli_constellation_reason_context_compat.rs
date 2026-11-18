//! Exercise the real missing-provider exit, before any ADI provider/model call.

use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

fn forgeplan(root: &Path) -> Command {
    let mut command = Command::cargo_bin("forgeplan").unwrap();
    command
        .current_dir(root)
        .env("HOME", root.join("home"))
        .env("NO_COLOR", "1")
        .env("CLICOLOR_FORCE", "0")
        .env("HF_HUB_OFFLINE", "1")
        .env("FORGEPLAN_MODEL_CACHE", root.join("model-cache"));
    command
}

fn entries(path: &Path) -> Vec<String> {
    let mut entries: Vec<String> = std::fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_str().unwrap().to_string())
        .collect();
    entries.sort();
    entries
}

#[test]
fn reason_missing_llm_uses_one_actual_fix_marker_and_never_saves_or_calls_a_provider() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("home")).unwrap();
    forgeplan(root).args(["init", "-y"]).assert().success();
    let ws = root.join(".forgeplan");
    std::fs::write(
        ws.join("prds/PRD-017-trust-context.md"),
        "---\nid: PRD-017\nslug: prd-trust-context\nkind: prd\ntitle: Trust context\n\
         status: draft\ndepth: deep\n---\n\n## Problem\n\nUse a real source artifact.\n",
    )
    .unwrap();
    forgeplan(root).arg("reindex").assert().success();
    let mut config = forgeplan_core::workspace::load_config(&ws).unwrap();
    config.llm = None;
    std::fs::write(
        ws.join("config.yaml"),
        serde_yaml::to_string(&config).unwrap(),
    )
    .unwrap();

    for migrated in [false, true] {
        if migrated {
            forgeplan(root).arg("index").assert().success();
            assert!(ws.join("lance/CURRENT.json").is_file());
        }
        let before_lance = entries(&ws.join("lance"));
        let before_notes = entries(&ws.join("notes"));
        let before_current = std::fs::read(ws.join("lance/CURRENT.json")).ok();
        for flags in [vec![], vec!["--fpf", "--json", "--save"]] {
            let output = forgeplan(root)
                .args(["reason", "prd-trust-context"])
                .args(flags)
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(1));
            let stderr = String::from_utf8(output.stderr).unwrap();
            let stdout = String::from_utf8(output.stdout).unwrap();
            assert!(stderr.contains("LLM not configured"), "{stderr}");
            assert_eq!(stderr.matches("Fix:").count(), 1, "{stderr}");
            assert!(!stdout.contains("Analyzing"));
            assert!(!stdout.contains("FPF context injected"));
            assert!(!stdout.contains("Saved as"));
            assert_eq!(entries(&ws.join("notes")), before_notes);
            assert_eq!(entries(&ws.join("lance")), before_lance);
            assert_eq!(
                std::fs::read(ws.join("lance/CURRENT.json")).ok(),
                before_current
            );
        }
    }
}
