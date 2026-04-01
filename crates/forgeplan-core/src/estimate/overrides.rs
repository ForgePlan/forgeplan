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
}
