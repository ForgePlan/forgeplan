use crate::config::LlmConfig;
use crate::llm::LlmClient;

const ADI_SYSTEM_PROMPT: &str = r#"You are a structured reasoning engine using the FPF ADI cycle (Abduction → Deduction → Induction).

Given an artifact (PRD, RFC, Problem Card, etc.), perform structured analysis:

## Phase 1: Abduction (Generate Hypotheses)
Generate 3+ distinct hypotheses or approaches. For each:
- State the hypothesis clearly
- Identify key assumptions
- Note what evidence would support or refute it

## Phase 2: Deduction (Evaluate Each)
For each hypothesis:
- Apply logical consequences
- Identify risks and failure modes
- Assess feasibility (Low/Medium/High)
- Note missing evidence

## Phase 3: Induction (Synthesize)
- Rank hypotheses by strength of evidence
- Identify the recommended approach
- State confidence level (Low/Medium/High)
- List remaining unknowns and next steps

Format your response as structured Markdown with clear ## headers for each phase.
Write in the same language as the artifact."#;

/// Run ADI reasoning cycle on an artifact.
pub async fn reason(
    config: &LlmConfig,
    artifact_id: &str,
    artifact_title: &str,
    artifact_kind: &str,
    artifact_body: &str,
) -> anyhow::Result<String> {
    let client = LlmClient::new(config.clone());

    let prompt = format!(
        "Analyze this {kind} artifact using the ADI cycle:\n\n\
         **ID**: {id}\n\
         **Title**: {title}\n\n\
         ---\n\n\
         {body}",
        kind = artifact_kind,
        id = artifact_id,
        title = artifact_title,
        body = artifact_body,
    );

    let system = crate::llm::load_prompt("reason", ADI_SYSTEM_PROMPT);
    client.generate(&prompt, Some(&system)).await
}
