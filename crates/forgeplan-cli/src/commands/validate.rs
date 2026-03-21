use std::collections::BTreeMap;
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
        // Reconstruct frontmatter map from record fields
        let fm = record_to_frontmatter(record);

        let kind = record.kind.parse::<ArtifactKind>().unwrap_or_else(|_| {
            eprintln!(
                "  Warning: unknown artifact kind '{}', applying base rules only",
                record.kind
            );
            ArtifactKind::Note
        });
        let depth = parse_depth(&record.depth).unwrap_or(Mode::Standard);

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

/// Reconstruct a frontmatter BTreeMap from an ArtifactRecord.
fn record_to_frontmatter(
    record: &forgeplan_core::db::store::ArtifactRecord,
) -> BTreeMap<String, serde_yaml::Value> {
    let mut fm = BTreeMap::new();
    fm.insert(
        "id".to_string(),
        serde_yaml::Value::String(record.id.clone()),
    );
    fm.insert(
        "title".to_string(),
        serde_yaml::Value::String(record.title.clone()),
    );
    fm.insert(
        "kind".to_string(),
        serde_yaml::Value::String(record.kind.clone()),
    );
    fm.insert(
        "status".to_string(),
        serde_yaml::Value::String(record.status.clone()),
    );
    fm.insert(
        "depth".to_string(),
        serde_yaml::Value::String(record.depth.clone()),
    );
    if let Some(ref a) = record.author {
        fm.insert("author".to_string(), serde_yaml::Value::String(a.clone()));
    }
    if let Some(ref pe) = record.parent_epic {
        fm.insert(
            "parent_epic".to_string(),
            serde_yaml::Value::String(pe.clone()),
        );
    }
    if let Some(ref vu) = record.valid_until {
        fm.insert(
            "valid_until".to_string(),
            serde_yaml::Value::String(vu.clone()),
        );
    }
    fm
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

fn parse_depth(s: &str) -> Option<Mode> {
    match s.to_lowercase().as_str() {
        "note" => Some(Mode::Note),
        "tactical" => Some(Mode::Tactical),
        "standard" => Some(Mode::Standard),
        "deep" | "critical" => Some(Mode::Deep),
        _ => None,
    }
}
