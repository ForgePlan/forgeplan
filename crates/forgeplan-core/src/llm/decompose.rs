use crate::config::LlmConfig;
use crate::llm::LlmClient;

const DECOMPOSE_SYSTEM_PROMPT: &str = r#"You are Forgeplan, a structured project planning assistant.

Given a PRD (Product Requirements Document), decompose it into concrete RFC tasks.

For each RFC task, provide:
1. **Title** — clear, actionable title for the RFC
2. **Description** — 2-3 sentences describing what the RFC should cover
3. **Scope** — which functional requirements (FR) from the PRD this RFC addresses
4. **Dependencies** — which other RFCs should be done first (if any)
5. **Estimated depth** — Tactical / Standard / Deep

Output as a numbered Markdown list. Each item should be a complete RFC proposal.
Aim for 3-7 RFCs that together cover all the PRD's functional requirements.
Write in the same language as the PRD."#;

/// Decompose a PRD into RFC tasks using LLM.
pub async fn decompose(
    config: &LlmConfig,
    prd_id: &str,
    prd_title: &str,
    prd_body: &str,
) -> anyhow::Result<String> {
    let client = LlmClient::new(config.clone());

    let prompt = format!(
        "Decompose this PRD into RFC tasks:\n\n\
         **PRD ID**: {id}\n\
         **Title**: {title}\n\n\
         ---\n\n\
         {body}",
        id = prd_id,
        title = prd_title,
        body = prd_body,
    );

    client
        .generate(&prompt, Some(DECOMPOSE_SYSTEM_PROMPT))
        .await
}
