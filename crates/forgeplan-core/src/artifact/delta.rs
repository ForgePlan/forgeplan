//! Delta-spec parser for ADDED/MODIFIED/REMOVED requirement changes.

/// A single requirement in a delta spec.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeltaRequirement {
    pub name: String,
    pub body: String,
}

/// Parsed delta spec — changes between versions.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeltaSpec {
    pub added: Vec<DeltaRequirement>,
    pub modified: Vec<DeltaRequirement>,
    pub removed: Vec<String>, // just names
}

/// Parse a markdown delta spec.
///
/// Expected format:
/// ```markdown
/// ## ADDED Requirements
/// ### Requirement: Two-Factor Auth
/// Description here...
///
/// ## MODIFIED Requirements
/// ### Requirement: Session Timeout
/// Changed from 30 to 60 minutes...
///
/// ## REMOVED Requirements
/// ### Requirement: Legacy Login
/// ```
pub fn parse_delta(markdown: &str) -> DeltaSpec {
    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut removed = Vec::new();

    let mut current_section = ""; // "added", "modified", "removed"
    let mut current_req_name = String::new();
    let mut current_req_body = String::new();

    for line in markdown.lines() {
        let trimmed = line.trim();

        // Detect section headers
        if trimmed.eq_ignore_ascii_case("## added requirements")
            || trimmed.eq_ignore_ascii_case("## added")
        {
            flush_requirement(
                &mut current_req_name,
                &mut current_req_body,
                current_section,
                &mut added,
                &mut modified,
                &mut removed,
            );
            current_section = "added";
            continue;
        }
        if trimmed.eq_ignore_ascii_case("## modified requirements")
            || trimmed.eq_ignore_ascii_case("## modified")
        {
            flush_requirement(
                &mut current_req_name,
                &mut current_req_body,
                current_section,
                &mut added,
                &mut modified,
                &mut removed,
            );
            current_section = "modified";
            continue;
        }
        if trimmed.eq_ignore_ascii_case("## removed requirements")
            || trimmed.eq_ignore_ascii_case("## removed")
        {
            flush_requirement(
                &mut current_req_name,
                &mut current_req_body,
                current_section,
                &mut added,
                &mut modified,
                &mut removed,
            );
            current_section = "removed";
            continue;
        }

        // Detect requirement headers: ### Requirement: Name
        if let Some(rest) = trimmed
            .strip_prefix("### Requirement:")
            .or_else(|| trimmed.strip_prefix("### requirement:"))
        {
            flush_requirement(
                &mut current_req_name,
                &mut current_req_body,
                current_section,
                &mut added,
                &mut modified,
                &mut removed,
            );
            current_req_name = rest.trim().to_string();
            continue;
        }

        // Accumulate body
        if !current_req_name.is_empty() {
            current_req_body.push_str(line);
            current_req_body.push('\n');
        }
    }

    // Flush last requirement
    flush_requirement(
        &mut current_req_name,
        &mut current_req_body,
        current_section,
        &mut added,
        &mut modified,
        &mut removed,
    );

    DeltaSpec {
        added,
        modified,
        removed,
    }
}

fn flush_requirement(
    name: &mut String,
    body: &mut String,
    section: &str,
    added: &mut Vec<DeltaRequirement>,
    modified: &mut Vec<DeltaRequirement>,
    removed: &mut Vec<String>,
) {
    if name.is_empty() {
        return;
    }
    let trimmed_body = body.trim().to_string();
    match section {
        "added" => added.push(DeltaRequirement {
            name: name.clone(),
            body: trimmed_body,
        }),
        "modified" => modified.push(DeltaRequirement {
            name: name.clone(),
            body: trimmed_body,
        }),
        "removed" => removed.push(name.clone()),
        _ => {}
    }
    name.clear();
    body.clear();
}

/// Generate a simple delta summary string.
pub fn delta_summary(delta: &DeltaSpec) -> String {
    let mut parts = Vec::new();
    if !delta.added.is_empty() {
        parts.push(format!("+{} added", delta.added.len()));
    }
    if !delta.modified.is_empty() {
        parts.push(format!("~{} modified", delta.modified.len()));
    }
    if !delta.removed.is_empty() {
        parts.push(format!("-{} removed", delta.removed.len()));
    }
    if parts.is_empty() {
        "No changes".to_string()
    } else {
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_delta() {
        let md = r#"
## ADDED Requirements
### Requirement: Two-Factor Auth
Users must authenticate via TOTP or SMS.

### Requirement: Audit Log
All login attempts are recorded.

## MODIFIED Requirements
### Requirement: Session Timeout
Changed from 30 to 60 minutes.

## REMOVED Requirements
### Requirement: Legacy Login
"#;
        let delta = parse_delta(md);
        assert_eq!(delta.added.len(), 2);
        assert_eq!(delta.added[0].name, "Two-Factor Auth");
        assert!(delta.added[0].body.contains("TOTP"));
        assert_eq!(delta.added[1].name, "Audit Log");
        assert_eq!(delta.modified.len(), 1);
        assert_eq!(delta.modified[0].name, "Session Timeout");
        assert!(delta.modified[0].body.contains("60 minutes"));
        assert_eq!(delta.removed.len(), 1);
        assert_eq!(delta.removed[0], "Legacy Login");
    }

    #[test]
    fn parse_empty_delta() {
        let delta = parse_delta("");
        assert!(delta.added.is_empty());
        assert!(delta.modified.is_empty());
        assert!(delta.removed.is_empty());
    }

    #[test]
    fn parse_added_only() {
        let md = "## Added\n### Requirement: Feature X\nDescription.";
        let delta = parse_delta(md);
        assert_eq!(delta.added.len(), 1);
        assert_eq!(delta.added[0].name, "Feature X");
        assert!(delta.modified.is_empty());
        assert!(delta.removed.is_empty());
    }

    #[test]
    fn delta_summary_all_types() {
        let delta = DeltaSpec {
            added: vec![DeltaRequirement { name: "A".into(), body: "".into() }],
            modified: vec![
                DeltaRequirement { name: "B".into(), body: "".into() },
                DeltaRequirement { name: "C".into(), body: "".into() },
            ],
            removed: vec!["D".into()],
        };
        assert_eq!(delta_summary(&delta), "+1 added, ~2 modified, -1 removed");
    }

    #[test]
    fn delta_summary_empty() {
        let delta = DeltaSpec {
            added: vec![],
            modified: vec![],
            removed: vec![],
        };
        assert_eq!(delta_summary(&delta), "No changes");
    }
}
