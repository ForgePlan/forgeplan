/// Integration smoke test for the full estimate pipeline:
/// extractor → scorer → overrides → calculator → domain inference
///
/// Verifies that all components work together correctly end-to-end,
/// including the DRY-refactored overrides and domain modules.

use std::collections::HashMap;
use forgeplan_core::estimate::{calculator, confidence, domain, extractor, overrides, scorer};
use forgeplan_core::estimate::types::*;

/// Realistic PRD body with FR table (like PRD-022).
const PRD_BODY: &str = r#"---
id: PRD-TEST
kind: prd
domain: backend
---

# PRD-TEST: Test Artifact

## Problem
We need a test fixture for estimate pipeline.

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | User can run estimate command | J1 |
| FR-002 | Core | Must | System extracts work items from FR table | J1 |
| FR-003 | UX | Should | User can specify grade via --grade flag | J2 |
"#;

/// RFC body with phases.
const RFC_BODY: &str = r#"---
id: RFC-TEST
kind: rfc
---

# RFC-TEST: Test Architecture

### Phase 1: Core Types
- [ ] **1.1** Create types.rs — Grade, Complexity enums
- [ ] **1.2** Create extractor.rs — parse FR table
- [ ] **1.3** Create scorer.rs — keyword complexity scoring

### Phase 2: Calculator
- [ ] **2.1** Create calculator.rs — multi-grade hours
- [ ] **2.2** Create display.rs — table formatting
"#;

// ── Full pipeline smoke tests ──────────────────────────────

#[test]
fn smoke_prd_estimate_full_pipeline() {
    // Step 1: Extract work items
    let items = extractor::extract_work_items(PRD_BODY);
    assert_eq!(items.len(), 3, "Should extract 3 FRs");
    assert_eq!(items[0].id, "FR-001");
    assert_eq!(items[0].source, ItemSource::Fr);

    // Step 2: Score complexity
    let scored = scorer::score_items(&items);
    assert_eq!(scored.len(), 3);
    for s in &scored {
        assert!(s.complexity.value() >= 1, "Complexity should be at least Trivial");
    }

    // Step 3: Calculate estimates
    let config = EstimateConfig::default();
    let (conf, reasons) = confidence::score_confidence(true, 3, false, 0, false, false);
    let result = calculator::calculate("PRD-TEST", "Test", &scored, &config, conf, reasons, vec![]);

    assert_eq!(result.artifact_id, "PRD-TEST");
    assert_eq!(result.items.len(), 3);
    assert!(result.total_score > 0.0, "Total score should be positive");
    assert!(result.confidence > 0.0, "Confidence should be positive");

    // Verify all grades present in each item
    for item in &result.items {
        assert!(item.hours.contains_key(&Grade::Junior));
        assert!(item.hours.contains_key(&Grade::Senior));
        assert!(item.hours.contains_key(&Grade::Ai));
        // Junior should take more time than Senior
        assert!(item.hours[&Grade::Junior] > item.hours[&Grade::Senior]);
        // AI should take less time than Senior
        assert!(item.hours[&Grade::Ai] < item.hours[&Grade::Senior]);
    }

    // Verify totals
    assert!(result.totals.contains_key(&Grade::Senior));
    let senior_total: f64 = result.items.iter().map(|i| i.hours[&Grade::Senior]).sum();
    assert!((result.totals[&Grade::Senior] - senior_total).abs() < 0.01);
}

#[test]
fn smoke_rfc_estimate_with_phases() {
    let items = extractor::extract_work_items(RFC_BODY);
    assert!(items.len() >= 5, "Should extract at least 5 phase items, got {}", items.len());

    let scored = scorer::score_items(&items);
    let config = EstimateConfig::default();
    let (conf, reasons) = confidence::score_confidence(false, 0, true, 5, false, false);
    let result = calculator::calculate("RFC-TEST", "Test RFC", &scored, &config, conf, reasons, vec![]);

    assert_eq!(result.artifact_id, "RFC-TEST");
    assert!(result.items.len() >= 5);
    assert!(result.total_score > 0.0);
}

// ── Overrides integration ──────────────────────────────────

#[test]
fn smoke_overrides_in_pipeline() {
    let items = extractor::extract_work_items(PRD_BODY);
    let mut scored = scorer::score_items(&items);

    // Get original complexity for FR-001
    let original = scored[0].complexity;

    // Apply override: FR-001 = Epic (13)
    let map = overrides::parse_complexity_overrides("FR-001=13").unwrap();
    overrides::apply_overrides(&mut scored, &map);

    assert_eq!(scored[0].complexity, Complexity::Epic);
    assert_ne!(scored[0].complexity, original, "Override should change complexity");

    // Calculate — FR-001 should now have Epic-level hours
    let config = EstimateConfig::default();
    let result = calculator::calculate("PRD-TEST", "Test", &scored, &config, 0.5, vec![], vec![]);

    let fr001_senior = result.items[0].hours[&Grade::Senior];
    assert_eq!(fr001_senior, 34.0, "Epic complexity = 34h Senior");
}

#[test]
fn smoke_override_does_not_affect_other_items() {
    let items = extractor::extract_work_items(PRD_BODY);
    let mut scored = scorer::score_items(&items);
    let fr002_before = scored[1].complexity;

    let map = overrides::parse_complexity_overrides("FR-001=13").unwrap();
    overrides::apply_overrides(&mut scored, &map);

    assert_eq!(scored[1].complexity, fr002_before, "FR-002 should be unchanged");
}

// ── Domain inference integration ───────────────────────────

#[test]
fn smoke_domain_inference_from_frontmatter() {
    let d = domain::infer_domain("Test PRD", PRD_BODY);
    assert_eq!(d, "backend", "Should read domain: backend from frontmatter");
}

#[test]
fn smoke_domain_inference_from_keywords() {
    let body = "---\n---\nThis uses react jsx component tailwind responsive design";
    let d = domain::infer_domain("UI Component Library", body);
    assert_eq!(d, "frontend");
}

#[test]
fn smoke_domain_with_grade_profile() {
    let d = domain::infer_domain("Test", PRD_BODY);
    let config = EstimateConfig::default();
    let grade = config.resolve_grade(&d);
    // default config has Senior for all domains
    assert_eq!(grade, Grade::Senior);
}

// ── Confidence scoring integration ─────────────────────────

#[test]
fn smoke_confidence_increases_with_spec_and_evidence() {
    let (base_conf, _) = confidence::score_confidence(true, 3, false, 0, false, false);
    let (with_spec, _) = confidence::score_confidence(true, 3, false, 0, true, false);
    let (with_both, _) = confidence::score_confidence(true, 3, false, 0, true, true);

    assert!(with_spec > base_conf, "Linked Spec should boost confidence");
    assert!(with_both > with_spec, "Linked Evidence should boost further");
    assert!(with_both <= 1.0, "Confidence should never exceed 1.0");
}

// ── Empty/edge cases ───────────────────────────────────────

#[test]
fn smoke_empty_body_returns_no_items() {
    let items = extractor::extract_work_items("");
    assert!(items.is_empty());
}

#[test]
fn smoke_body_without_fr_table() {
    let body = "# Just a title\n\nSome text without any FR table or phases.";
    let items = extractor::extract_work_items(body);
    assert!(items.is_empty());
}

#[test]
fn smoke_hints_for_empty_items() {
    let hints = extractor::collect_hints("# Empty PRD\nNo FRs here.", 0, "prd");
    assert!(!hints.is_empty(), "Should suggest adding FR table for PRD");
}

#[test]
fn smoke_calculate_with_zero_items() {
    let config = EstimateConfig::default();
    let result = calculator::calculate("X", "Empty", &[], &config, 0.0, vec![], vec![]);
    assert_eq!(result.items.len(), 0);
    assert_eq!(result.total_score, 0.0);
    assert!(result.totals.values().all(|&v| v == 0.0));
}

// ── Negative tests ─────────────────────────────────────────

#[test]
fn negative_invalid_grade_string() {
    assert!("wizard".parse::<Grade>().is_err());
    assert!("god".parse::<Grade>().is_err());
    assert!("intern".parse::<Grade>().is_err());
    assert!("".parse::<Grade>().is_err());
    assert!("123".parse::<Grade>().is_err());
}

#[test]
fn negative_grade_case_insensitive() {
    // These should all work (case insensitive)
    assert_eq!("JUNIOR".parse::<Grade>().unwrap(), Grade::Junior);
    assert_eq!("Senior".parse::<Grade>().unwrap(), Grade::Senior);
    assert_eq!("AI".parse::<Grade>().unwrap(), Grade::Ai);
}

#[test]
fn negative_invalid_complexity_overrides() {
    assert!(overrides::parse_complexity_overrides("not-valid").is_err());
    assert!(overrides::parse_complexity_overrides("FR-001=abc").is_err());
    assert!(overrides::parse_complexity_overrides("FR-001=0").is_err());
    assert!(overrides::parse_complexity_overrides("FR-001=4").is_err());  // not Fibonacci
    assert!(overrides::parse_complexity_overrides("FR-001=7").is_err());  // not Fibonacci
    assert!(overrides::parse_complexity_overrides("FR-001=100").is_err());
}

#[test]
fn negative_complexity_from_value_rejects_non_fibonacci() {
    for v in [0, 4, 6, 7, 9, 10, 11, 12, 14, 100, u32::MAX] {
        assert!(Complexity::from_value(v).is_none(), "Should reject {}", v);
    }
}
