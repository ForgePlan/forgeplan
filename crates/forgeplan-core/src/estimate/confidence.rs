/// Score confidence of an estimate based on artifact completeness.
/// Returns (confidence 0.0-1.0, reasons).
pub fn score_confidence(
    has_fr: bool,
    fr_count: usize,
    has_rfc_phases: bool,
    phase_count: usize,
    has_spec: bool,
    has_evidence: bool,
) -> (f64, Vec<String>) {
    let mut score: f64 = 0.0;
    let mut reasons = Vec::new();

    // FR presence (+30%)
    if has_fr && fr_count > 0 {
        score += 0.30;
        reasons.push(format!("has {} FR items (+30%)", fr_count));
    } else {
        reasons.push("no FR items found".to_string());
    }

    // RFC phases (+25%)
    if has_rfc_phases && phase_count > 0 {
        score += 0.25;
        reasons.push(format!("has {} RFC phase steps (+25%)", phase_count));
    } else {
        reasons.push("no RFC phases".to_string());
    }

    // Spec presence (+15%)
    if has_spec {
        score += 0.15;
        reasons.push("has Spec (+15%)".to_string());
    }

    // Evidence from past estimates (+20%)
    if has_evidence {
        score += 0.20;
        reasons.push("has calibration evidence (+20%)".to_string());
    }

    // Baseline: even without anything, there's 10% from having the artifact at all
    score += 0.10;

    (score.min(1.0), reasons)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_confidence() {
        let (score, reasons) = score_confidence(true, 5, true, 10, true, true);
        assert!((score - 1.0).abs() < 0.01);
        assert_eq!(reasons.len(), 4);
    }

    #[test]
    fn only_fr() {
        let (score, _reasons) = score_confidence(true, 3, false, 0, false, false);
        assert!((score - 0.40).abs() < 0.01); // 0.30 + 0.10 baseline
    }

    #[test]
    fn nothing() {
        let (score, reasons) = score_confidence(false, 0, false, 0, false, false);
        assert!((score - 0.10).abs() < 0.01); // baseline only
        assert!(reasons.iter().any(|r| r.contains("no FR")));
        assert!(reasons.iter().any(|r| r.contains("no RFC")));
    }

    #[test]
    fn fr_plus_rfc() {
        let (score, _) = score_confidence(true, 5, true, 3, false, false);
        assert!((score - 0.65).abs() < 0.01); // 0.30 + 0.25 + 0.10
    }

    #[test]
    fn has_fr_but_zero_count_is_negative() {
        let (score, reasons) = score_confidence(true, 0, false, 0, false, false);
        assert!((score - 0.10).abs() < 0.01); // baseline only, has_fr=true but count=0 → no bonus
        assert!(reasons.iter().any(|r| r.contains("no FR")));
    }

    #[test]
    fn has_rfc_phases_but_zero_count_no_bonus() {
        let (score, reasons) = score_confidence(false, 0, true, 0, false, false);
        assert!((score - 0.10).abs() < 0.01); // baseline only, phase_count=0 → no +25%
        assert!(reasons.iter().any(|r| r.contains("no RFC")));
    }

    #[test]
    fn spec_only() {
        let (score, _) = score_confidence(false, 0, false, 0, true, false);
        assert!((score - 0.25).abs() < 0.01); // 0.15 + 0.10 baseline
    }

    #[test]
    fn evidence_only() {
        let (score, _) = score_confidence(false, 0, false, 0, false, true);
        assert!((score - 0.30).abs() < 0.01); // 0.20 + 0.10 baseline
    }

    #[test]
    fn clamp_at_one() {
        // Even if we somehow exceed 1.0 (e.g., future bonus added), it should clamp
        // Currently full_confidence = exactly 1.0, so this is a guard test
        let (score, _) = score_confidence(true, 10, true, 20, true, true);
        assert!(score <= 1.0);
        assert!(score >= 0.99); // should be exactly 1.0
    }
}
