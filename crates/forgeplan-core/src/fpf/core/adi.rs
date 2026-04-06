//! ADI Record — trackable reasoning history.
//!
//! Stores hypotheses, deductions, and verdicts as first-class data.
//! Linkable to artifacts for decision traceability.

use serde::{Deserialize, Serialize};

/// A single ADI reasoning session result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdiRecord {
    /// Unique ID (e.g., "ADI-001").
    pub id: String,
    /// Artifact this reasoning was performed on.
    pub artifact_id: String,
    /// When the reasoning was performed.
    pub created_at: String,
    /// LLM provider and model used.
    pub model: String,
    /// Hypotheses generated during abduction phase.
    pub hypotheses: Vec<Hypothesis>,
    /// Deductive consequences evaluated.
    pub deductions: Vec<Deduction>,
    /// Evidence gaps identified.
    pub evidence_needed: Vec<EvidenceGap>,
    /// Final recommendation.
    pub recommendation: String,
    /// Overall confidence level.
    pub confidence: Confidence,
}

/// A hypothesis from the abduction phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub id: String,
    pub description: String,
    pub assumptions: Vec<String>,
    pub confidence: Confidence,
    /// Verdict after induction: supported, weakened, or refuted.
    pub verdict: Option<HypothesisVerdict>,
}

/// Deductive consequence of a hypothesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deduction {
    pub hypothesis_id: String,
    pub consequence: String,
    pub risks: Vec<String>,
    pub feasibility: Confidence,
}

/// An identified evidence gap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceGap {
    pub for_hypothesis: String,
    pub test: String,
    pub effort: String,
}

/// Confidence level.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::High => write!(f, "high"),
            Confidence::Medium => write!(f, "medium"),
            Confidence::Low => write!(f, "low"),
        }
    }
}

impl std::str::FromStr for Confidence {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // LLM often returns "High — justification here", so parse only first word
        let first_word = s.split_whitespace().next().unwrap_or(s);
        let cleaned = first_word.trim_end_matches(|c: char| !c.is_alphabetic());
        match cleaned.to_lowercase().as_str() {
            "high" => Ok(Confidence::High),
            "medium" => Ok(Confidence::Medium),
            "low" => Ok(Confidence::Low),
            _ => Err(format!("unknown confidence: {s}")),
        }
    }
}

/// Verdict on a hypothesis after evidence check.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HypothesisVerdict {
    Supported,
    Weakened,
    Refuted,
}

/// A snapshot of ADI state at a point in time (for tracking evolution).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdiSnapshot {
    pub record_id: String,
    pub timestamp: String,
    /// Confidence at this point.
    pub confidence: Confidence,
    /// Number of hypotheses at this point.
    pub hypothesis_count: usize,
    /// How many supported / weakened / refuted.
    pub supported: usize,
    pub weakened: usize,
    pub refuted: usize,
}

impl AdiRecord {
    /// Convert from LLM AdiOutput to structured AdiRecord.
    pub fn from_adi_output(
        id: String,
        artifact_id: String,
        model: String,
        output: &crate::llm::reason::AdiOutput,
    ) -> Self {
        let hypotheses = output
            .hypotheses
            .iter()
            .map(|h| Hypothesis {
                id: h.id.clone(),
                description: h.description.clone(),
                assumptions: h.assumptions.clone(),
                confidence: h.confidence.parse().unwrap_or(Confidence::Medium),
                verdict: None, // ADI doesn't produce verdicts inline
            })
            .collect();

        let deductions = output
            .deductions
            .iter()
            .map(|d| Deduction {
                hypothesis_id: d.hypothesis_id.clone(),
                consequence: d.consequence.clone(),
                risks: d.risks.clone(),
                feasibility: d.feasibility.parse().unwrap_or(Confidence::Medium),
            })
            .collect();

        let evidence_needed = output
            .evidence_needed
            .iter()
            .map(|e| EvidenceGap {
                for_hypothesis: e.for_hypothesis.clone(),
                test: e.test.clone(),
                effort: e.effort.clone(),
            })
            .collect();

        let confidence = output.confidence.parse().unwrap_or(Confidence::Medium);

        Self {
            id,
            artifact_id,
            created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            model,
            hypotheses,
            deductions,
            evidence_needed,
            recommendation: output.recommendation.clone(),
            confidence,
        }
    }

    /// Serialize to pretty JSON for embedding in note body.
    pub fn to_json_body(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Create a snapshot of current state.
    pub fn snapshot(&self) -> AdiSnapshot {
        let (mut supported, mut weakened, mut refuted) = (0, 0, 0);
        for h in &self.hypotheses {
            match h.verdict {
                Some(HypothesisVerdict::Supported) => supported += 1,
                Some(HypothesisVerdict::Weakened) => weakened += 1,
                Some(HypothesisVerdict::Refuted) => refuted += 1,
                None => {}
            }
        }
        AdiSnapshot {
            record_id: self.id.clone(),
            timestamp: self.created_at.clone(),
            confidence: self.confidence,
            hypothesis_count: self.hypotheses.len(),
            supported,
            weakened,
            refuted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_roundtrip() {
        assert_eq!("high".parse::<Confidence>().unwrap(), Confidence::High);
        assert_eq!("Medium".parse::<Confidence>().unwrap(), Confidence::Medium);
        assert_eq!(Confidence::Low.to_string(), "low");
    }

    #[test]
    fn adi_record_snapshot() {
        let record = AdiRecord {
            id: "ADI-001".into(),
            artifact_id: "RFC-001".into(),
            created_at: "2026-04-06T12:00:00".into(),
            model: "gemini-3-flash".into(),
            hypotheses: vec![
                Hypothesis {
                    id: "H1".into(),
                    description: "Option C".into(),
                    assumptions: vec![],
                    confidence: Confidence::High,
                    verdict: Some(HypothesisVerdict::Supported),
                },
                Hypothesis {
                    id: "H2".into(),
                    description: "Option B".into(),
                    assumptions: vec![],
                    confidence: Confidence::Medium,
                    verdict: Some(HypothesisVerdict::Weakened),
                },
                Hypothesis {
                    id: "H3".into(),
                    description: "Quick win".into(),
                    assumptions: vec![],
                    confidence: Confidence::High,
                    verdict: None,
                },
            ],
            deductions: vec![],
            evidence_needed: vec![],
            recommendation: "Go with C".into(),
            confidence: Confidence::High,
        };
        let snap = record.snapshot();
        assert_eq!(snap.hypothesis_count, 3);
        assert_eq!(snap.supported, 1);
        assert_eq!(snap.weakened, 1);
        assert_eq!(snap.refuted, 0);
    }

    #[test]
    fn adi_record_serializes() {
        let record = AdiRecord {
            id: "ADI-001".into(),
            artifact_id: "RFC-001".into(),
            created_at: "2026-04-06".into(),
            model: "test".into(),
            hypotheses: vec![],
            deductions: vec![],
            evidence_needed: vec![],
            recommendation: "test".into(),
            confidence: Confidence::High,
        };
        let json = serde_json::to_string(&record).unwrap();
        let back: AdiRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "ADI-001");
        assert_eq!(back.confidence, Confidence::High);
    }

    #[test]
    fn from_adi_output_converts_correctly() {
        use crate::llm::reason::{AdiHypothesis, AdiOutput};

        let output = AdiOutput {
            hypotheses: vec![AdiHypothesis {
                id: "H1".into(),
                description: "Use Option C".into(),
                assumptions: vec!["testable".into()],
                confidence: "High — good fit".into(),
            }],
            deductions: vec![],
            evidence_needed: vec![],
            recommendation: "Go with C".into(),
            confidence: "High — validated".into(),
            raw_markdown: None,
        };

        let record = AdiRecord::from_adi_output(
            "NOTE-042".into(),
            "RFC-001".into(),
            "gemini/flash".into(),
            &output,
        );

        assert_eq!(record.id, "NOTE-042");
        assert_eq!(record.artifact_id, "RFC-001");
        assert_eq!(record.model, "gemini/flash");
        assert_eq!(record.hypotheses.len(), 1);
        assert_eq!(record.hypotheses[0].id, "H1");
        assert_eq!(record.hypotheses[0].confidence, Confidence::High);
        assert!(record.hypotheses[0].verdict.is_none());
        assert_eq!(record.confidence, Confidence::High);
        assert_eq!(record.recommendation, "Go with C");
        assert!(!record.created_at.is_empty());
    }

    #[test]
    fn to_json_body_roundtrip() {
        use crate::llm::reason::AdiOutput;

        let output = AdiOutput {
            hypotheses: vec![],
            deductions: vec![],
            evidence_needed: vec![],
            recommendation: "test".into(),
            confidence: "Medium".into(),
            raw_markdown: None,
        };

        let record =
            AdiRecord::from_adi_output("NOTE-001".into(), "PRD-001".into(), "test".into(), &output);
        let json = record.to_json_body();
        let back: AdiRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.recommendation, "test");
    }
}
