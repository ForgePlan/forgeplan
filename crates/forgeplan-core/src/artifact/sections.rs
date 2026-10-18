//! Markdown section parser using pulldown-cmark.
//!
//! Extracts heading-delimited sections from markdown documents,
//! handling nested headings and code blocks correctly.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// A parsed section from a markdown document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// The heading text, e.g. "Problem", "Goals".
    pub heading: String,
    /// Heading level: 1 = `#`, 2 = `##`, 3 = `###`, etc.
    pub level: u8,
    /// Everything between this heading and the next heading of the same or higher level.
    pub content: String,
}

/// Convert a [`HeadingLevel`] to a numeric value.
fn level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Extract all sections from a markdown body.
///
/// Each section spans from a heading to the next heading of the same or higher
/// level (i.e. lower or equal numeric level). Nested headings (e.g. `###` under
/// `##`) are included as part of the parent section's content.
///
/// Headings inside fenced code blocks are correctly ignored by pulldown-cmark
/// and will not produce spurious sections.
pub fn list_sections(body: &str) -> Vec<Section> {
    let parser = Parser::new_ext(body, Options::all());

    let mut sections: Vec<Section> = Vec::new();
    let mut current_heading = String::new();
    let mut current_level: u8 = 0;
    let mut current_content = String::new();
    let mut collecting = false;
    // Whether we are inside a heading tag — distinguishes heading text from body text.
    let mut in_heading = false;
    // Whether the current heading is nested (deeper level) and should be emitted as content.
    let mut nested_heading = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let new_level = level_to_u8(level);

                if collecting && new_level <= current_level {
                    // Same or higher-level heading: finalise the current section.
                    sections.push(Section {
                        heading: std::mem::take(&mut current_heading),
                        level: current_level,
                        content: current_content.trim().to_string(),
                    });
                    current_content.clear();
                }

                if !collecting || new_level <= current_level {
                    // Start a brand-new section.
                    current_heading.clear();
                    current_level = new_level;
                    collecting = true;
                    nested_heading = false;
                } else {
                    // Nested heading (e.g. ### under ##) — render as content.
                    current_content.push_str(&format!("{} ", "#".repeat(new_level as usize)));
                    nested_heading = true;
                }
                in_heading = true;
            }
            Event::End(TagEnd::Heading(_)) if in_heading => {
                current_content.push('\n');
                in_heading = false;
                nested_heading = false;
            }
            Event::Text(text) | Event::Code(text) => {
                if in_heading {
                    if nested_heading {
                        // Text belongs to a nested heading rendered as content.
                        current_content.push_str(&text);
                    } else {
                        // Text belongs to the section's own heading.
                        current_heading.push_str(&text);
                    }
                } else if collecting {
                    current_content.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak if collecting && !in_heading => {
                current_content.push('\n');
            }
            Event::End(TagEnd::Paragraph) if collecting && !in_heading => {
                current_content.push('\n');
            }
            Event::Start(Tag::Item) if collecting && !in_heading => {
                current_content.push_str("- ");
            }
            Event::End(TagEnd::Item) if collecting && !in_heading => {
                current_content.push('\n');
            }
            Event::Start(Tag::CodeBlock(_)) if collecting && !in_heading => {
                current_content.push_str("```\n");
            }
            Event::End(TagEnd::CodeBlock) if collecting && !in_heading => {
                current_content.push_str("```\n");
            }
            _ => {}
        }
    }

    // Flush the last section.
    if collecting {
        sections.push(Section {
            heading: current_heading,
            level: current_level,
            content: current_content.trim().to_string(),
        });
    }

    sections
}

/// Split a body at **every** heading, regardless of level.
///
/// The flat counterpart of [`list_sections`]: a `###` under a `##` starts its
/// own section instead of folding into the parent's content. This is the
/// split semantic indexing needs — a long `#` document whose `##` blocks hold
/// the meaning must yield one unit per block, or the tail of the single nested
/// section would hit the input-length cap and stay invisible to the vector
/// index, recreating the prefix-only gap the table exists to close.
///
/// `content` is the raw text between the heading's end and the next heading's
/// start — it may still be long, and callers bound it themselves (embedding
/// applies its own prefix cap per section).
///
/// Uses `into_offset_iter`, so spans are real byte ranges of `body`; headings
/// inside fenced code blocks are ignored exactly as in [`list_sections`].
pub fn list_section_spans(body: &str) -> Vec<Section> {
    let parser = Parser::new_ext(body, Options::all());

    let mut sections = Vec::new();
    // (heading text, level, byte offset where this section's content starts).
    let mut open: Option<(String, u8, usize)> = None;
    let mut in_heading = false;
    let mut heading_text = String::new();
    let mut heading_level = 0u8;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                if let Some((heading, level, content_start)) = open.take() {
                    sections.push(Section {
                        heading,
                        level,
                        content: body[content_start..range.start].trim().to_string(),
                    });
                }
                in_heading = true;
                heading_text.clear();
                heading_level = level_to_u8(level);
            }
            Event::End(TagEnd::Heading(_)) if in_heading => {
                open = Some((std::mem::take(&mut heading_text), heading_level, range.end));
                in_heading = false;
            }
            Event::Text(text) | Event::Code(text) if in_heading => {
                heading_text.push_str(&text);
            }
            _ => {}
        }
    }

    if let Some((heading, level, content_start)) = open {
        sections.push(Section {
            heading,
            level,
            content: body[content_start..].trim().to_string(),
        });
    }

    sections
}

/// Extract the content of a specific section by heading name (case-insensitive).
///
/// Returns [`None`] if the section does not exist.
pub fn extract_section(body: &str, heading: &str) -> Option<String> {
    list_sections(body)
        .into_iter()
        .find(|s| s.heading.eq_ignore_ascii_case(heading))
        .map(|s| s.content)
}

/// Check whether a section exists and has meaningful content
/// (not just whitespace or common placeholder tokens).
pub fn section_has_content(body: &str, heading: &str) -> bool {
    match extract_section(body, heading) {
        Some(content) => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                return false;
            }
            // Common placeholder patterns that don't count as real content.
            let placeholders = ["tbd", "todo", "n/a", "...", "\u{2014}", "-"];
            !placeholders.contains(&trimmed.to_lowercase().as_str())
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_section_extraction() {
        let md = "\
## Problem

The system is slow.

## Goals

Make it fast.
";
        let sections = list_sections(md);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].heading, "Problem");
        assert_eq!(sections[0].level, 2);
        assert!(sections[0].content.contains("The system is slow."));
        assert_eq!(sections[1].heading, "Goals");
        assert!(sections[1].content.contains("Make it fast."));
    }

    #[test]
    fn flat_spans_split_at_every_level() {
        let md = "\
# Title

## Problem

Overview of the problem.

### Sub-problem

Details here.

## Goals

Make it work.
";
        let sections = list_section_spans(md);
        assert_eq!(sections.len(), 4);
        assert_eq!(sections[0].heading, "Title");
        assert_eq!(sections[0].level, 1);
        assert_eq!(sections[1].heading, "Problem");
        assert_eq!(sections[1].level, 2);
        // Flat, not nested: the parent's content stops where `###` opens.
        assert_eq!(sections[1].content, "Overview of the problem.");
        assert_eq!(sections[2].heading, "Sub-problem");
        assert_eq!(sections[2].level, 3);
        assert_eq!(sections[2].content, "Details here.");
        assert_eq!(sections[3].heading, "Goals");
        assert_eq!(sections[3].content, "Make it work.");
    }

    #[test]
    fn flat_spans_ignore_headings_inside_code_fences() {
        let md = "\
## Real

```markdown
## Not a section
```

After the fence.
";
        let sections = list_section_spans(md);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].heading, "Real");
        assert!(sections[0].content.contains("## Not a section"));
        assert!(sections[0].content.contains("After the fence."));
    }

    #[test]
    fn flat_spans_handle_a_heading_with_no_body() {
        let md = "# Lone title\n";
        let sections = list_section_spans(md);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].heading, "Lone title");
        assert_eq!(sections[0].level, 1);
        assert_eq!(sections[0].content, "");
    }

    #[test]
    fn flat_spans_match_nested_sections_when_nothing_is_nested() {
        let md = "## A\none\n## B\ntwo\n";
        let flat = list_section_spans(md);
        let nested = list_sections(md);
        assert_eq!(flat.len(), nested.len());
        for (f, n) in flat.iter().zip(nested.iter()) {
            assert_eq!(f.heading, n.heading);
            assert_eq!(f.level, n.level);
        }
    }

    #[test]
    fn nested_headings() {
        let md = "\
## Problem

Overview of the problem.

### Sub-problem

Details here.

## Goals

Make it work.
";
        let sections = list_sections(md);
        // ### Sub-problem is nested inside ## Problem, not a separate top-level section.
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].heading, "Problem");
        assert!(sections[0].content.contains("Sub-problem"));
        assert!(sections[0].content.contains("Details here."));
        assert_eq!(sections[1].heading, "Goals");
    }

    #[test]
    fn code_blocks_with_hashes_not_treated_as_sections() {
        let md = "\
## Problem

Here is some code:

```python
## This is a comment, not a heading
x = 42
```

Still part of Problem.

## Goals

Done.
";
        let sections = list_sections(md);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].heading, "Problem");
        assert!(sections[0].content.contains("Still part of Problem."));
        assert_eq!(sections[1].heading, "Goals");
    }

    #[test]
    fn case_insensitive_matching() {
        let md = "\
## problem

Lower case heading.

## GOALS

Upper case heading.
";
        assert!(extract_section(md, "Problem").is_some());
        assert!(extract_section(md, "PROBLEM").is_some());
        assert!(extract_section(md, "goals").is_some());
        assert!(extract_section(md, "Goals").is_some());
    }

    #[test]
    fn empty_body_returns_empty_vec() {
        assert!(list_sections("").is_empty());
    }

    #[test]
    fn section_with_only_whitespace_has_no_content() {
        let md = "\
## Problem



## Goals

Real content here.
";
        assert!(!section_has_content(md, "Problem"));
        assert!(section_has_content(md, "Goals"));
    }

    #[test]
    fn placeholder_content_treated_as_empty() {
        let md = "\
## Problem

TBD

## Goals

TODO
";
        assert!(!section_has_content(md, "Problem"));
        assert!(!section_has_content(md, "Goals"));
    }

    #[test]
    fn nonexistent_section_returns_none() {
        let md = "## Problem\n\nSome text.\n";
        assert!(extract_section(md, "NonExistent").is_none());
        assert!(!section_has_content(md, "NonExistent"));
    }

    #[test]
    fn h1_and_h3_levels() {
        let md = "\
# Title

Intro text.

### Detail

Some detail.

# Another Title

More text.
";
        let sections = list_sections(md);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].heading, "Title");
        assert_eq!(sections[0].level, 1);
        assert!(sections[0].content.contains("Detail"));
        assert_eq!(sections[1].heading, "Another Title");
        assert_eq!(sections[1].level, 1);
    }
}
