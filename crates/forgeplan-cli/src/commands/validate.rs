use std::env;

use forgeplan_core::artifact::types::{ArtifactKind, Mode};
use forgeplan_core::db::store::LanceStore;
use forgeplan_core::validation::{self, Severity, ValidationResult};
use forgeplan_core::workspace;

pub async fn run(id: Option<&str>) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let all_records = store.list_records(None).await?;

    if all_records.is_empty() {
        println!("No artifacts found.");
        return Ok(());
    }

    let to_validate: Vec<_> = if let Some(target_id) = id {
        let upper = target_id.to_uppercase();
        all_records
            .into_iter()
            .filter(|r| r.id.to_uppercase() == upper)
            .collect()
    } else {
        all_records
    };

    if to_validate.is_empty() {
        if let Some(target_id) = id {
            anyhow::bail!("Artifact '{}' not found", target_id);
        }
    }

    let mut total_errors = 0;
    let mut total_warnings = 0;
    let mut total_passed = 0;

    for record in &to_validate {
        let fm = record.frontmatter_map();

        let kind = record.kind.parse::<ArtifactKind>().unwrap_or_else(|_| {
            eprintln!(
                "  Warning: unknown artifact kind '{}', applying base rules only",
                record.kind
            );
            ArtifactKind::Note
        });
        let depth = record.depth.parse::<Mode>().unwrap_or(Mode::Standard);

        let result = validation::validate(&record.id, &record.body, &fm, &kind, &depth);
        print_result(&result, &record.title, &depth);

        total_errors += result.error_count();
        total_warnings += result.warning_count();
        if result.passed() {
            total_passed += 1;
        }
    }

    if to_validate.len() > 1 {
        println!();
        println!(
            "Summary: {} artifact(s), {} passed, {} error(s), {} warning(s)",
            to_validate.len(),
            total_passed,
            total_errors,
            total_warnings
        );
    }

    if total_errors > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn print_result(result: &ValidationResult, title: &str, depth: &Mode) {
    println!();
    println!(
        "{} \"{}\" (depth: {:?})",
        result.artifact_id, title, depth
    );
    println!("{}", "─".repeat(50));

    if result.findings.is_empty() {
        println!("  All checks passed!");
    } else {
        for f in &result.findings {
            let icon = match f.severity {
                Severity::Must => "x",
                Severity::Should => "!",
                Severity::Could => "~",
            };
            println!(
                "  {} [{}] {}: {}",
                icon, f.severity, f.rule_id, f.message
            );
        }
    }

    let passed = result.findings.is_empty();
    let status = if passed {
        "PASS"
    } else if result.passed() {
        "PASS (with warnings)"
    } else {
        "FAIL"
    };
    println!();
    println!(
        "  Result: {} -- {} error(s), {} warning(s)",
        status,
        result.error_count(),
        result.warning_count()
    );
}

