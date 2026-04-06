//! FPF Engine configuration — all scoring parameters in one place.
//!
//! Loaded from `config.yaml` under `fpf:` key. Defaults match current hardcoded values
//! for backward compatibility.

use serde::{Deserialize, Serialize};

/// Top-level FPF configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FpfConfig {
    /// Explore-exploit action thresholds.
    pub thresholds: Thresholds,
    /// Reliability component weights for F-G-R.
    pub weights: ReliabilityWeights,
    /// ADI reasoning settings.
    pub adi: AdiConfig,
    /// Congruence Level penalties (CL0..CL3).
    pub cl_penalties: ClPenalties,
    /// Evidence decay settings.
    pub decay: DecayConfig,
}

impl Default for FpfConfig {
    fn default() -> Self {
        Self {
            thresholds: Thresholds::default(),
            weights: ReliabilityWeights::default(),
            adi: AdiConfig::default(),
            cl_penalties: ClPenalties::default(),
            decay: DecayConfig::default(),
        }
    }
}

/// Explore-exploit decision thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Thresholds {
    /// R_eff below this → EXPLORE (draft artifacts). Default: 0.01
    pub explore_reff: f64,
    /// R_eff below this → INVESTIGATE. Default: 0.5
    pub investigate_reff: f64,
    /// R_eff at or above this → EXPLOIT. Default: 0.7
    pub exploit_reff: f64,
    /// F-G-R overall at or above this (combined with exploit_reff) → EXPLOIT. Default: 0.6
    pub exploit_fgr: f64,
    /// F-G-R overall below this (combined with explore_reff) → EXPLORE priority 1. Default: 0.4
    pub explore_fgr: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            explore_reff: 0.01,
            investigate_reff: 0.5,
            exploit_reff: 0.7,
            exploit_fgr: 0.6,
            explore_fgr: 0.4,
        }
    }
}

/// Weights for reliability sub-components in F-G-R.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReliabilityWeights {
    /// Weight for R_eff score. Default: 0.5
    pub reff: f64,
    /// Maximum bonus for link count. Default: 0.3
    pub links: f64,
    /// Bonus for freshness (not stale). Default: 0.2
    pub freshness: f64,
}

impl Default for ReliabilityWeights {
    fn default() -> Self {
        Self {
            reff: 0.5,
            links: 0.3,
            freshness: 0.2,
        }
    }
}

/// ADI reasoning configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AdiConfig {
    /// Max hypotheses to request from LLM. Default: 5
    pub max_hypotheses: u32,
    /// Max FPF KB sections to inject into prompt. Default: 5
    pub kb_sections_limit: usize,
    /// Temperature cap for ADI reasoning (lower = more deterministic). Default: 0.3
    pub temperature_cap: f32,
    /// Auto-save ADI results as AdiRecord. Default: true
    pub auto_save: bool,
}

impl Default for AdiConfig {
    fn default() -> Self {
        Self {
            max_hypotheses: 5,
            kb_sections_limit: 5,
            temperature_cap: 0.3,
            auto_save: true,
        }
    }
}

/// Congruence Level penalties for evidence scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClPenalties {
    /// CL0 (opposed context) penalty. Default: 0.9
    pub cl0: f64,
    /// CL1 (different context) penalty. Default: 0.4
    pub cl1: f64,
    /// CL2 (similar context) penalty. Default: 0.1
    pub cl2: f64,
    /// CL3 (same context) penalty. Default: 0.0
    pub cl3: f64,
}

impl Default for ClPenalties {
    fn default() -> Self {
        Self {
            cl0: 0.9,
            cl1: 0.4,
            cl2: 0.1,
            cl3: 0.0,
        }
    }
}

impl ClPenalties {
    /// Get penalty for a given congruence level (0-3).
    pub fn penalty(&self, cl: u8) -> f64 {
        match cl {
            0 => self.cl0,
            1 => self.cl1,
            2 => self.cl2,
            3 => self.cl3,
            _ => self.cl0, // unknown = worst case
        }
    }
}

/// Evidence decay settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DecayConfig {
    /// Score assigned to expired evidence. Default: 0.1 (stale, not absent)
    pub expired_score: f64,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self { expired_score: 0.1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_matches_hardcoded_values() {
        let cfg = FpfConfig::default();
        assert!((cfg.thresholds.explore_reff - 0.01).abs() < f64::EPSILON);
        assert!((cfg.thresholds.investigate_reff - 0.5).abs() < f64::EPSILON);
        assert!((cfg.thresholds.exploit_reff - 0.7).abs() < f64::EPSILON);
        assert!((cfg.weights.reff - 0.5).abs() < f64::EPSILON);
        assert!((cfg.weights.links - 0.3).abs() < f64::EPSILON);
        assert!((cfg.weights.freshness - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn cl_penalties_lookup() {
        let cl = ClPenalties::default();
        assert!((cl.penalty(0) - 0.9).abs() < f64::EPSILON);
        assert!((cl.penalty(1) - 0.4).abs() < f64::EPSILON);
        assert!((cl.penalty(2) - 0.1).abs() < f64::EPSILON);
        assert!((cl.penalty(3) - 0.0).abs() < f64::EPSILON);
        assert!((cl.penalty(99) - 0.9).abs() < f64::EPSILON); // unknown = worst
    }

    #[test]
    fn config_deserializes_from_yaml() {
        let yaml = r#"
thresholds:
  explore_reff: 0.05
  exploit_reff: 0.8
weights:
  reff: 0.6
  links: 0.2
  freshness: 0.2
adi:
  max_hypotheses: 3
  auto_save: false
"#;
        let cfg: FpfConfig = serde_yaml::from_str(yaml).unwrap();
        assert!((cfg.thresholds.explore_reff - 0.05).abs() < f64::EPSILON);
        assert!((cfg.thresholds.exploit_reff - 0.8).abs() < f64::EPSILON);
        // Unspecified fields keep defaults
        assert!((cfg.thresholds.investigate_reff - 0.5).abs() < f64::EPSILON);
        assert_eq!(cfg.adi.max_hypotheses, 3);
        assert!(!cfg.adi.auto_save);
    }

    #[test]
    fn config_serializes_roundtrip() {
        let cfg = FpfConfig::default();
        let yaml = serde_yaml::to_string(&cfg).unwrap();
        let cfg2: FpfConfig = serde_yaml::from_str(&yaml).unwrap();
        assert!((cfg.thresholds.exploit_reff - cfg2.thresholds.exploit_reff).abs() < f64::EPSILON);
        assert!((cfg.cl_penalties.cl1 - cfg2.cl_penalties.cl1).abs() < f64::EPSILON);
    }
}
