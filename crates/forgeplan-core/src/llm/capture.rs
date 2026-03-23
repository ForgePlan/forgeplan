use crate::config::LlmConfig;
use crate::llm::LlmClient;

const CAPTURE_SYSTEM_PROMPT: &str = r#"You are Forgeplan, a decision capture assistant.

Given a decision statement from a conversation, create a structured Note or ADR artifact body.

Determine the type based on content:
- If it's a simple decision or observation → Note format
- If it's an architectural/technical decision with alternatives → ADR format

For Notes:
# [Title derived from decision]
## Decision
[The decision statement, clarified]
## Context
[Why this decision was made — infer from the statement]
## Impact
[What this affects]

For ADRs:
# [Title]
## Context
[Background — infer from the decision]
## Decision
[The decision, clearly stated]
## Alternatives Considered
[If mentioned or inferable, list 2-3 alternatives]
## Consequences
[What follows from this decision]

Write in the same language as the input. Be concise."#;

/// Capture a decision from conversation context into an artifact body.
/// Returns (suggested_kind, body) — "note" for simple decisions, "adr" for architectural ones.
pub async fn capture(
    config: &LlmConfig,
    decision: &str,
    context: Option<&str>,
) -> anyhow::Result<(String, String)> {
    let client = LlmClient::new(config.clone());

    let mut prompt = format!("Capture this decision as a structured artifact:\n\n{decision}");
    if let Some(ctx) = context {
        prompt.push_str(&format!("\n\nAdditional context:\n{ctx}"));
    }
    prompt.push_str("\n\nFirst line of your response MUST be either `KIND: note` or `KIND: adr` to indicate the artifact type. Then the body follows.");

    let system = crate::llm::load_prompt("capture", CAPTURE_SYSTEM_PROMPT);
    let response = client.generate(&prompt, Some(&system)).await?;

    // Parse kind from first line
    let (kind, body) = if let Some(rest) = response.strip_prefix("KIND: ") {
        if let Some((kind_line, body)) = rest.split_once('\n') {
            let kind = kind_line.trim().to_lowercase();
            let kind = if kind == "adr" { "adr" } else { "note" };
            (kind.to_string(), body.trim_start().to_string())
        } else {
            ("note".to_string(), response)
        }
    } else {
        // Default to note if LLM didn't follow format
        ("note".to_string(), response)
    };

    Ok((kind, body))
}
