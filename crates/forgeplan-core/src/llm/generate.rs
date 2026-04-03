use crate::config::LlmConfig;
use crate::llm::LlmClient;

/// System prompt for artifact generation.
fn system_prompt(kind: &str) -> String {
    format!(
        "You are Forgeplan, a structured project planning assistant. \
         Generate a complete {kind} artifact in Markdown format. \
         Do NOT include YAML frontmatter (---) — only the Markdown body. \
         Follow the standard structure for a {kind}:\n\
         {structure}\n\
         Write in the same language as the user's description. \
         Be specific, actionable, and thorough. \
         Use checkboxes (- [ ]) for actionable items.",
        kind = kind,
        structure = structure_hint(kind),
    )
}

fn structure_hint(kind: &str) -> &'static str {
    match kind {
        "prd" => {
            "# Title\n## Summary\n## Motivation\n## Goals\n## Non-Goals\n## Functional Requirements\n## Non-Functional Requirements\n## Success Metrics"
        }
        "rfc" => {
            "# Title\n## Summary\n## Motivation\n## Goals\n## Non-Goals\n## Architecture\n## Implementation Phases\n## Testing\n## References"
        }
        "adr" => {
            "# Title\n## Context\n## Decision\n## Alternatives Considered\n## Consequences\n## References"
        }
        "epic" => {
            "# Title\n## Summary\n## Goals\n## Children (PRDs)\n## Success Criteria\n## Timeline"
        }
        "spec" => "# Title\n## Summary\n## Data Model\n## API Contracts\n## Events\n## Versioning",
        "problem" => "# Title\n## Signal\n## Context\n## Impact\n## Anti-Goodhart Indicators",
        "solution" => {
            "# Title\n## Problem Reference\n## Variants (2-3)\n## Weakest Link Analysis\n## Recommendation"
        }
        "evidence" => {
            "# Title\n## Verdict\n## Methodology\n## Data\n## Congruence Level\n## Valid Until"
        }
        _ => "# Title\n## Summary\n## Details",
    }
}

/// Generate artifact body using LLM.
pub async fn generate_body(
    config: &LlmConfig,
    kind: &str,
    description: &str,
    title: &str,
) -> anyhow::Result<String> {
    let client = LlmClient::new(config.clone());

    let prompt = format!(
        "Create a {kind} titled \"{title}\" based on this description:\n\n{description}",
        kind = kind,
        title = title,
        description = description,
    );

    let system = system_prompt(kind);
    let body = client.generate(&prompt, Some(&system)).await?;

    Ok(body)
}
