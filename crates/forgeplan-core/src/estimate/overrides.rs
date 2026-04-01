use std::collections::HashMap;

use anyhow::Result;

use super::types::{Complexity, ScoredItem};

/// Parse "FR-001=5,FR-002=3" into HashMap<String, Complexity>.
pub fn parse_complexity_overrides(input: &str) -> Result<HashMap<String, Complexity>> {
    let mut map = HashMap::new();
    for pair in input.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let parts: Vec<&str> = pair.splitn(2, '=').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid complexity override '{}'. Format: FR-001=5", pair);
        }
        let id = parts[0].trim().to_string();
        let value: u32 = parts[1]
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid number '{}' in complexity override", parts[1].trim()))?;
        let complexity = Complexity::from_value(value).ok_or_else(|| {
            anyhow::anyhow!("Invalid Fibonacci value {}. Valid: 1, 2, 3, 5, 8, 13", value)
        })?;
        map.insert(id, complexity);
    }
    Ok(map)
}

/// Apply manual overrides to scored items — override has highest priority.
pub fn apply_overrides(items: &mut [ScoredItem], overrides: &HashMap<String, Complexity>) {
    for item in items.iter_mut() {
        if let Some(complexity) = overrides.get(&item.id) {
            item.complexity = *complexity;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_overrides() {
        let map = parse_complexity_overrides("FR-001=5,FR-002=3").unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map["FR-001"], Complexity::Complex);  // 5 = Complex
        assert_eq!(map["FR-002"], Complexity::Medium);   // 3 = Medium
    }

    #[test]
    fn test_parse_empty_string() {
        let map = parse_complexity_overrides("").unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn test_parse_trailing_comma() {
        let map = parse_complexity_overrides("FR-001=5,").unwrap();
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_parse_invalid_format() {
        let result = parse_complexity_overrides("FR-001");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_number() {
        let result = parse_complexity_overrides("FR-001=abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_fibonacci() {
        let result = parse_complexity_overrides("FR-001=4");
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_overrides() {
        use super::super::types::{ScoredItem, TaskType};
        let mut items = vec![ScoredItem {
            id: "FR-001".to_string(),
            description: "Test".to_string(),
            task_type: TaskType::PureCoding,
            complexity: Complexity::Trivial,
        }];
        let mut map = HashMap::new();
        map.insert("FR-001".to_string(), Complexity::Complex);
        apply_overrides(&mut items, &map);
        assert_eq!(items[0].complexity, Complexity::Complex);
    }

    // ── Corner cases ──────────────────────────────────────────

    #[test]
    fn test_parse_whitespace_around_values() {
        let map = parse_complexity_overrides("  FR-001 = 5 , FR-002 = 8  ").unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map["FR-001"], Complexity::Complex);
        assert_eq!(map["FR-002"], Complexity::Hard);
    }

    #[test]
    fn test_parse_all_fibonacci_values() {
        for (val, expected) in [(1, Complexity::Trivial), (2, Complexity::Simple),
            (3, Complexity::Medium), (5, Complexity::Complex),
            (8, Complexity::Hard), (13, Complexity::Epic)]
        {
            let input = format!("X={}", val);
            let map = parse_complexity_overrides(&input).unwrap();
            assert_eq!(map["X"], expected, "Failed for value {}", val);
        }
    }

    #[test]
    fn test_parse_duplicate_ids_last_wins() {
        let map = parse_complexity_overrides("FR-001=1,FR-001=13").unwrap();
        assert_eq!(map["FR-001"], Complexity::Epic); // last wins
    }

    #[test]
    fn test_parse_only_commas() {
        let map = parse_complexity_overrides(",,,").unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn test_parse_equals_in_value() {
        // "FR=001=5" → splitn(2, '=') → ["FR", "001=5"] → parse error on "001=5"
        let result = parse_complexity_overrides("FR=001=5");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_zero_value() {
        let result = parse_complexity_overrides("FR-001=0");
        assert!(result.is_err()); // 0 is not a valid Fibonacci
    }

    #[test]
    fn test_parse_negative_value() {
        let result = parse_complexity_overrides("FR-001=-1");
        assert!(result.is_err()); // can't parse as u32
    }

    #[test]
    fn test_parse_very_large_value() {
        let result = parse_complexity_overrides("FR-001=999999");
        assert!(result.is_err()); // not a valid Fibonacci
    }

    // ── Negative: apply_overrides ──────────────────────────────

    #[test]
    fn test_apply_overrides_nonexistent_id() {
        use super::super::types::{ScoredItem, TaskType};
        let mut items = vec![ScoredItem {
            id: "FR-001".to_string(),
            description: "Test".to_string(),
            task_type: TaskType::PureCoding,
            complexity: Complexity::Trivial,
        }];
        let mut map = HashMap::new();
        map.insert("FR-999".to_string(), Complexity::Epic);
        apply_overrides(&mut items, &map);
        assert_eq!(items[0].complexity, Complexity::Trivial); // unchanged
    }

    #[test]
    fn test_apply_overrides_empty_map() {
        use super::super::types::{ScoredItem, TaskType};
        let mut items = vec![ScoredItem {
            id: "FR-001".to_string(),
            description: "Test".to_string(),
            task_type: TaskType::PureCoding,
            complexity: Complexity::Medium,
        }];
        apply_overrides(&mut items, &HashMap::new());
        assert_eq!(items[0].complexity, Complexity::Medium); // unchanged
    }

    #[test]
    fn test_apply_overrides_empty_items() {
        let mut items: Vec<super::super::types::ScoredItem> = vec![];
        let mut map = HashMap::new();
        map.insert("FR-001".to_string(), Complexity::Epic);
        apply_overrides(&mut items, &map); // should not panic
        assert!(items.is_empty());
    }

    // ── Error message quality ──────────────────────────────────

    #[test]
    fn test_error_message_contains_invalid_value() {
        let err = parse_complexity_overrides("FR-001=abc").unwrap_err();
        assert!(err.to_string().contains("abc"), "Error should mention the invalid value");
    }

    #[test]
    fn test_error_message_contains_invalid_fibonacci() {
        let err = parse_complexity_overrides("FR-001=7").unwrap_err();
        assert!(err.to_string().contains("7"), "Error should mention the Fibonacci value");
        assert!(err.to_string().contains("Valid"), "Error should list valid values");
    }

    #[test]
    fn test_error_message_contains_bad_format() {
        let err = parse_complexity_overrides("FR-001").unwrap_err();
        assert!(err.to_string().contains("FR-001"), "Error should mention the bad pair");
        assert!(err.to_string().contains("Format"), "Error should show expected format");
    }
}
