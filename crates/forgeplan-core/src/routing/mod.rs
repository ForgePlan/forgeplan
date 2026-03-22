//! Smart Routing v2 — rule-based depth calibration and pipeline suggestion.
//!
//! Replaces LLM-based routing with deterministic rules.
//! LLM is used ONLY for optional --explain enrichment.

pub mod pipeline;
pub mod rules;
pub mod signals;

use crate::artifact::types::{ArtifactKind, Mode};

/// Result of routing a task description through the rule engine.
#[derive(Debug, Clone)]
pub struct RoutingResult {
    /// Computed depth level.
    pub depth: Mode,
    /// Ordered pipeline of artifact types to create.
    pub pipeline: Vec<ArtifactKind>,
    /// Signals that contributed to the depth decision.
    pub triggers: Vec<Signal>,
    /// Confidence score (0.0-1.0). More matching signals = higher confidence.
    pub confidence: f64,
}

/// A signal extracted from input that influences depth.
#[derive(Debug, Clone)]
pub struct Signal {
    /// Signal identifier (e.g., "keyword:security", "complexity:fr_count").
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Minimum depth this signal requires.
    pub minimum_depth: Mode,
    /// Signal weight for confidence calculation.
    pub weight: f64,
}

impl std::fmt::Display for RoutingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "## Depth: {}", depth_display(&self.depth))?;
        writeln!(f)?;

        writeln!(f, "## Pipeline")?;
        if self.pipeline.is_empty() {
            writeln!(f, "None (tactical — just do it)")?;
        } else {
            let names: Vec<&str> = self.pipeline.iter().map(|k| kind_display(k)).collect();
            writeln!(f, "{}", names.join(" → "))?;
        }
        writeln!(f)?;

        writeln!(f, "## Triggers Matched")?;
        if self.triggers.is_empty() {
            writeln!(f, "No escalation triggers — defaults to Tactical")?;
        } else {
            for t in &self.triggers {
                writeln!(
                    f,
                    "- **{}**: {} → {}+",
                    t.id,
                    t.description,
                    depth_display(&t.minimum_depth)
                )?;
            }
        }
        writeln!(f)?;

        writeln!(
            f,
            "## Confidence: {:.0}%",
            self.confidence * 100.0
        )?;

        if !self.pipeline.is_empty() {
            writeln!(f)?;
            writeln!(f, "## Next Step")?;
            let first = kind_display(&self.pipeline[0]);
            writeln!(f, "```")?;
            writeln!(
                f,
                "forgeplan new {} \"<title>\"",
                first.to_lowercase()
            )?;
            writeln!(f, "```")?;
        }

        Ok(())
    }
}

/// Route a task description to depth + pipeline using rule engine.
pub fn route(description: &str) -> RoutingResult {
    let signals = signals::extract(description);
    let depth = rules::compute_depth(&signals);
    let pipeline = pipeline::for_depth(&depth);
    let confidence = rules::compute_confidence(&signals, &depth);

    RoutingResult {
        depth,
        pipeline,
        triggers: signals,
        confidence,
    }
}

/// Route an existing artifact (post-factum calibration).
pub fn calibrate_artifact(body: &str, link_count: usize, has_epic: bool) -> RoutingResult {
    let mut signals = signals::extract(body);
    signals.extend(signals::extract_structural(body, link_count, has_epic));
    let depth = rules::compute_depth(&signals);
    let pipeline = pipeline::for_depth(&depth);
    let confidence = rules::compute_confidence(&signals, &depth);

    RoutingResult {
        depth,
        pipeline,
        triggers: signals,
        confidence,
    }
}

fn depth_display(mode: &Mode) -> &'static str {
    match mode {
        Mode::Note => "Note",
        Mode::Tactical => "Tactical",
        Mode::Standard => "Standard",
        Mode::Deep => "Deep/Critical",
    }
}

fn kind_display(kind: &ArtifactKind) -> &'static str {
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
