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

/// One flat section of a body, as a **byte range** of the body it came from.
///
/// A range and not a copy: the whole reason this type exists is that a caller
/// must be able to name body-absolute offsets without asking a `String` where
/// in memory it lives. A copy carries no addressable position — the address of
/// an owned `String` is the address of its heap buffer, not its offset in the
/// source — so any offset derived from one is unrelated to the body it was
/// taken from, however well-formed the subtraction looks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionSpan {
    /// The heading text, e.g. "Problem", "Goals".
    pub heading: String,
    /// Heading level: 1 = `#`, 2 = `##`, 3 = `###`, etc.
    pub level: u8,
    /// From the `#` of this heading to the `#` of the next one, or to
    /// `body.len()` for the last. Never includes the run before the first
    /// heading; that is the preamble, and it belongs to nobody here.
    pub raw_range: std::ops::Range<usize>,
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
/// [`raw_range`](SectionSpan::raw_range) is the raw text from the `#` of this
/// heading up to the `#` of the next heading — or the end of the body for the
/// last one. It is a **range, not a copy**, and it deliberately covers the
/// heading itself: a span that began after the heading would make the heading
/// bytes unreachable to every caller, so no body byte could be indexed.
///
/// The text may still be long, and callers bound it themselves (embedding
/// applies its own token budget per window).
///
/// Uses `into_offset_iter`, so spans are real byte ranges of `body`; headings
/// inside fenced code blocks are ignored exactly as in [`list_sections`].
pub fn list_section_spans(body: &str) -> Vec<SectionSpan> {
    let parser = Parser::new_ext(body, Options::all());

    let mut sections = Vec::new();
    // (heading text, level, byte offset of this heading's `#`).
    let mut open: Option<(String, u8, usize)> = None;
    let mut in_heading = false;
    let mut heading_text = String::new();
    let mut heading_level = 0u8;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                if let Some((heading, level, start)) = open.take() {
                    sections.push(SectionSpan {
                        heading,
                        level,
                        raw_range: start..range.start,
                    });
                }
                in_heading = true;
                heading_text.clear();
                heading_level = level_to_u8(level);
            }
            Event::End(TagEnd::Heading(_)) if in_heading => {
                open = Some((
                    std::mem::take(&mut heading_text),
                    heading_level,
                    range.start,
                ));
                in_heading = false;
            }
            Event::Text(text) | Event::Code(text) if in_heading => {
                heading_text.push_str(&text);
            }
            _ => {}
        }
    }

    if let Some((heading, level, start)) = open {
        sections.push(SectionSpan {
            heading,
            level,
            raw_range: start..body.len(),
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
        // Flat, not nested: the parent's range stops where `###` opens.
        assert_eq!(
            &md[sections[1].raw_range.clone()],
            "## Problem\n\nOverview of the problem.\n\n"
        );
        assert_eq!(sections[2].heading, "Sub-problem");
        assert_eq!(sections[2].level, 3);
        assert_eq!(
            &md[sections[2].raw_range.clone()],
            "### Sub-problem\n\nDetails here.\n\n"
        );
        assert_eq!(sections[3].heading, "Goals");
        assert_eq!(
            &md[sections[3].raw_range.clone()],
            "## Goals\n\nMake it work.\n"
        );
    }

    #[test]
    fn flat_spans_tile_the_body_exactly() {
        // The ranges are the whole point: consecutive, starting at 0, ending
        // at `body.len()`. A span scheme that is off by even one byte makes a
        // body impossible to reproduce, and no per-section content assertion
        // would notice.
        //
        // Every input below opens with its first heading at byte 0, so this
        // covers the spans themselves and nothing more. It says nothing about a
        // body with a preamble: `raw_range` excludes the run before the first
        // heading by design (see [`SectionSpan::raw_range`]), and covering that
        // run is `sections_for`'s job, not this function's — it is what
        // publishes it either as a row of its own or as part of the first
        // section.
        for md in [
            "## A\n\nalpha\n\n## B\n\nalpha\n",
            "## Same\n\nalpha\n\n## Same\n\nalpha\n\n",
            "# Lone title\n",
            "## A\n## B\nx\n",
        ] {
            let sections = list_section_spans(md);
            let mut cursor = 0usize;
            for span in &sections {
                assert_eq!(
                    span.raw_range.start, cursor,
                    "a span of {md:?} does not start where the previous one ended"
                );
                cursor = span.raw_range.end;
            }
            assert_eq!(
                cursor,
                md.len(),
                "the spans of {md:?} end at {cursor} but the body is {} bytes",
                md.len()
            );
        }
    }

    #[test]
    fn flat_spans_are_the_exact_bytes_of_each_section() {
        let cases: [(&str, &[std::ops::Range<usize>]); 4] = [
            ("## A\n\nalpha\n\n## B\n\nalpha\n", &[0..13, 13..25]),
            ("## Same\n\nalpha\n\n## Same\n\nalpha\n\n", &[0..16, 16..32]),
            ("# Lone title\n", &[0..13]),
            ("## A\n## B\nx\n", &[0..5, 5..12]),
        ];
        for (md, expected) in cases {
            let sections = list_section_spans(md);
            let actual: Vec<_> = sections.iter().map(|s| s.raw_range.clone()).collect();
            assert_eq!(actual, expected, "wrong raw ranges for {md:?}");
        }

        // The heading bytes are inside the span, not before it.
        let md = "# Lone title\n";
        let sections = list_section_spans(md);
        assert!(
            !sections[0].raw_range.is_empty(),
            "a heading with no body still owns its own bytes"
        );
        assert_eq!(&md[sections[0].raw_range.clone()], "# Lone title\n");
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
        let text = &md[sections[0].raw_range.clone()];
        assert!(text.contains("## Not a section"));
        assert!(text.contains("After the fence."));
    }

    #[test]
    fn flat_spans_handle_a_heading_with_no_body() {
        let md = "# Lone title\n";
        let sections = list_section_spans(md);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].heading, "Lone title");
        assert_eq!(sections[0].level, 1);
        assert_eq!(sections[0].raw_range, 0..13);
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
