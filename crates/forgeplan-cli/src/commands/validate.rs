use forgeplan_core::artifact::frontmatter;
use forgeplan_core::artifact::store;
use forgeplan_core::artifact::types::{ArtifactKind, Mode};
use forgeplan_core::validation::{self, Severity, ValidationResult};
use forgeplan_core::workspace;

pub async fn run(id: Option<&str>) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let artifacts = store::list_artifacts(&ws).await?;
    if artifacts.is_empty() {
        println!("No artifacts found.");
        return Ok(());
    }

    let to_validate: Vec<_> = if let Some(target_id) = id {
        let upper = target_id.to_uppercase();
        artifacts
            .into_iter()
            .filter(|a| a.id.to_uppercase() == upper)
            .collect()
    } else {
        artifacts
    };

    if to_validate.is_empty() {
        if let Some(target_id) = id {
            anyhow::bail!("Artifact '{}' not found", target_id);
        }
    }

    let mut total_errors = 0;
    let mut total_warnings = 0;
    let mut total_passed = 0;

    for artifact in &to_validate {
        let content = tokio::fs::read_to_string(&artifact.path).await?;
        let (fm, body) = frontmatter::parse_frontmatter(&content)?;

        let kind = parse_kind(&artifact.kind);
        let depth = fm
            .get("depth")
            .and_then(|v| v.as_str())
            .and_then(parse_depth)
            .unwrap_or(Mode::Standard);

        let result = validation::validate(&artifact.id, &body, &fm, &kind, &depth);
        print_result(&result, &artifact.title, &depth);

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
    let status = if passed { "PASS" } else if result.passed() { "PASS (with warnings)" } else { "FAIL" };
    println!();
    println!(
        "  Result: {} -- {} error(s), {} warning(s)",
        status,
        result.error_count(),
        result.warning_count()
    );
}

fn parse_kind(s: &str) -> ArtifactKind {
    match s.to_lowercase().as_str() {
        "prd" => ArtifactKind::Prd,
        "epic" => ArtifactKind::Epic,
        "spec" => ArtifactKind::Spec,
        "rfc" => ArtifactKind::Rfc,
        "adr" => ArtifactKind::Adr,
        "note" => ArtifactKind::Note,
        "problem" | "problemcard" => ArtifactKind::ProblemCard,
        "solution" | "solutionportfolio" => ArtifactKind::SolutionPortfolio,
        "evidence" | "evidencepack" => ArtifactKind::EvidencePack,
        "refresh" | "refreshreport" => ArtifactKind::RefreshReport,
        unknown => {
            eprintln!(
                "  Warning: unknown artifact kind '{}', applying base rules only",
                unknown
            );
            ArtifactKind::Note
        }
    }
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
