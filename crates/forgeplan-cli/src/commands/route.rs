use console::style;
use forgeplan_core::routing;
use forgeplan_core::workspace;

use crate::ui;

pub async fn run(description: &str, explain: bool) -> anyhow::Result<()> {
    // Rule-based routing (instant, offline, no LLM)
    let result = routing::route(description);

    // Styled depth
    println!(
        "## Depth: {}",
        ui::styled_depth(&format!("{}", depth_display(&result.depth)))
    );
    println!();

    // Styled pipeline
    println!("{}", style("## Pipeline").bold());
    if result.pipeline.is_empty() {
        println!(
            "{}",
            style("None (tactical — just do it)").green()
        );
    } else {
        let names: Vec<String> = result
            .pipeline
            .iter()
            .map(|k| style(kind_display(k)).bold().white().to_string())
            .collect();
        println!("{}", names.join(&style(" → ").dim().to_string()));
    }
    println!();

    // Styled triggers
    println!("{}", style("## Triggers Matched").bold());
    if result.triggers.is_empty() {
        println!(
            "{}",
            style("No escalation triggers — defaults to Tactical").dim()
        );
    } else {
        for t in &result.triggers {
            println!(
                "- {}: {} → {}+",
                style(&t.id).yellow(),
                t.description,
                ui::styled_depth(&format!("{}", depth_display(&t.minimum_depth)))
            );
        }
    }
    println!();

    // Styled confidence
    let conf_pct = result.confidence * 100.0;
    let styled_conf = if conf_pct > 70.0 {
        style(format!("{:.0}%", conf_pct)).green()
    } else if conf_pct > 50.0 {
        style(format!("{:.0}%", conf_pct)).yellow()
    } else {
        style(format!("{:.0}%", conf_pct)).red()
    };
    println!("## Confidence: {}", styled_conf);

    // Next step
    if !result.pipeline.is_empty() {
        println!();
        println!("{}", style("## Next Step").bold());
        let first = kind_display(&result.pipeline[0]);
        println!(
            "  {}",
            style(format!("forgeplan new {} \"<title>\"", first.to_lowercase())).cyan()
        );
    }

    // Optional LLM explanation
    if explain {
        let cwd = std::env::current_dir()?;
        let ws = workspace::find_workspace(&cwd);
        if let Some(ws) = ws {
            let config = workspace::load_config(&ws)?;
            if let Some(llm_config) = config.llm {
                let llm_config = llm_config.with_env_overrides();
                println!("\n## AI Explanation\n");
                let explanation =
                    forgeplan_core::llm::route::route(&llm_config, description).await?;
                println!("{explanation}");
            } else {
                println!("\n(--explain requires LLM config in .forgeplan/config.yaml)");
            }
        }
    }

    Ok(())
}

fn depth_display(mode: &forgeplan_core::artifact::types::Mode) -> &'static str {
    use forgeplan_core::artifact::types::Mode;
    match mode {
        Mode::Note => "Note",
        Mode::Tactical => "Tactical",
        Mode::Standard => "Standard",
        Mode::Deep => "Deep/Critical",
    }
}

fn kind_display(kind: &forgeplan_core::artifact::types::ArtifactKind) -> &'static str {
    use forgeplan_core::artifact::types::ArtifactKind;
    match kind {
        ArtifactKind::Epic => "Epic",
        ArtifactKind::Prd => "PRD",
        ArtifactKind::Spec => "Spec",
        ArtifactKind::Rfc => "RFC",
        ArtifactKind::Adr => "ADR",
        ArtifactKind::Note => "Note",
        ArtifactKind::ProblemCard => "Problem",
        ArtifactKind::SolutionPortfolio => "Solution",
        ArtifactKind::EvidencePack => "Evidence",
        ArtifactKind::RefreshReport => "Refresh",
    }
}
