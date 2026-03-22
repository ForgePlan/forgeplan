//! Rule engine: Signal[] → Depth. Deterministic, no LLM.

use crate::artifact::types::Mode;
use crate::routing::Signal;

/// Compute depth from signals: depth = max(all signal.minimum_depth).
/// Default is Tactical when no signals match.
pub fn compute_depth(signals: &[Signal]) -> Mode {
    signals
        .iter()
        .map(|s| &s.minimum_depth)
        .max_by_key(|m| depth_rank(m))
        .cloned()
        .unwrap_or(Mode::Tactical)
}

/// Confidence = f(signal count, signal weights, agreement).
/// More signals agreeing on the same depth = higher confidence.
pub fn compute_confidence(signals: &[Signal], depth: &Mode) -> f64 {
    if signals.is_empty() {
        // No signals → we're confident it's tactical (default)
        return 0.8;
    }

    // Weight of signals that agree with or exceed the computed depth
    let agreeing_weight: f64 = signals
        .iter()
        .filter(|s| depth_rank(&s.minimum_depth) >= depth_rank(depth))
        .map(|s| s.weight)
        .sum();

    let total_weight: f64 = signals.iter().map(|s| s.weight).sum();

    // Base confidence from agreement ratio
    let agreement = if total_weight > 0.0 {
        agreeing_weight / total_weight
    } else {
        0.5
    };

    // Boost from signal count (more evidence = more confident)
    let count_boost = (signals.len() as f64 * 0.05).min(0.2);

    (0.5 + agreement * 0.3 + count_boost).min(1.0)
}

fn depth_rank(mode: &Mode) -> u8 {
    match mode {
        Mode::Note => 0,
        Mode::Tactical => 1,
        Mode::Standard => 2,
        Mode::Deep => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signal(id: &str, depth: Mode, weight: f64) -> Signal {
        Signal {
            id: id.into(),
            description: String::new(),
            minimum_depth: depth,
            weight,
        }
    }

    #[test]
    fn no_signals_defaults_tactical() {
        assert_eq!(compute_depth(&[]), Mode::Tactical);
    }

    #[test]
    fn single_deep_signal_returns_deep() {
        let signals = vec![signal("sec", Mode::Deep, 0.9)];
        assert_eq!(compute_depth(&signals), Mode::Deep);
    }

    #[test]
    fn max_depth_wins() {
        let signals = vec![
            signal("a", Mode::Standard, 0.5),
            signal("b", Mode::Deep, 0.9),
            signal("c", Mode::Standard, 0.6),
        ];
        assert_eq!(compute_depth(&signals), Mode::Deep);
    }

    #[test]
    fn confidence_no_signals() {
        let c = compute_confidence(&[], &Mode::Tactical);
        assert!(c > 0.7, "No signals should give decent confidence for tactical");
    }

    #[test]
    fn confidence_increases_with_more_signals() {
        let one = vec![signal("a", Mode::Deep, 0.9)];
        let three = vec![
            signal("a", Mode::Deep, 0.9),
            signal("b", Mode::Deep, 0.8),
            signal("c", Mode::Deep, 0.7),
        ];
        let c1 = compute_confidence(&one, &Mode::Deep);
        let c3 = compute_confidence(&three, &Mode::Deep);
        assert!(c3 > c1, "More matching signals = higher confidence");
    }

    #[test]
    fn confidence_capped_at_one() {
        let many: Vec<Signal> = (0..20)
            .map(|i| signal(&format!("s{i}"), Mode::Deep, 0.9))
            .collect();
        let c = compute_confidence(&many, &Mode::Deep);
        assert!(c <= 1.0);
    }
}
