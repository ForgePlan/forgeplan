//! Source parsers — translate raw plugin output into a structured
//! [`ParsedSource`] consumable by the ingest engine.
//!
//! The parser strategy is selected declaratively by [`Parser`] in the mapping
//! YAML; this module wires each enum variant to a concrete implementation. No
//! parser ever executes user-supplied code — they only walk text.

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;
use thiserror::Error;

use super::types::Parser;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// One heading-delimited section of a parsed source document.
///
/// Line numbers are **1-indexed** and inclusive at both ends to match how
/// editors / SPEC-004 `format: "{path}:{line_start}-{line_end}"` are wired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParsedSection {
    /// `# = 1`, `## = 2`, … `###### = 6`.
    pub heading_level: u8,
    /// Heading text (the `## Foo` becomes `"Foo"`).
    pub heading_text: String,
    /// Line where the heading itself appears (1-indexed).
    pub line_start: usize,
    /// Last line of the section's body (1-indexed, inclusive).
    pub line_end: usize,
    /// Raw body text between this heading and the next one of the same or
    /// higher level. Heading line itself is **not** included.
    pub body: String,
    /// Heading texts of immediate children (one level deeper).
    pub sub_sections: Vec<String>,
}

/// Result of running a [`SourceParser`] on a single input file.
#[derive(Debug, Clone, Serialize)]
pub struct ParsedSource {
    /// Logical path of the source (preserved verbatim from the caller).
    #[serde(skip)]
    pub path: String,
    /// Front-matter as a JSON value (always an object; empty for parsers that
    /// don't extract front-matter).
    pub front_matter: serde_json::Value,
    /// Sections keyed by heading text. When two sections share a heading the
    /// later one wins — callers needing every occurrence should walk
    /// [`Self::full_text`] manually.
    pub sections: HashMap<String, ParsedSection>,
    /// Full original text (without the front-matter delimiter block, if any).
    pub full_text: String,
    /// Number of lines in `full_text`.
    pub line_count: usize,
}

impl ParsedSource {
    /// Convenience: empty parsed source, useful for tests / non-section
    /// parsers (`Json`, `Yaml`).
    pub fn empty(path: &str) -> Self {
        Self {
            path: path.to_owned(),
            front_matter: serde_json::Value::Object(serde_json::Map::new()),
            sections: HashMap::new(),
            full_text: String::new(),
            line_count: 0,
        }
    }
}

/// Errors a parser can raise.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("yaml front-matter parse error in {path}: {source}")]
    FrontMatter {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("json parse error in {path}: {source}")]
    Json {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("yaml document parse error in {path}: {source}")]
    YamlDocument {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },
}

/// Strategy interface implemented by each parser variant.
pub trait SourceParser: Send + Sync {
    fn parse(&self, path: &Path, content: &str) -> Result<ParsedSource, ParseError>;
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Returns a boxed parser matching the declared mapping [`Parser`] kind.
pub fn parser_for(parser_kind: &Parser) -> Box<dyn SourceParser> {
    match parser_kind {
        Parser::FrontMatterPlusSections => Box::new(FrontMatterPlusSections),
        Parser::MarkdownOnly => Box::new(MarkdownOnly),
        Parser::LogWithBlame => Box::new(LogWithBlame),
        Parser::Json => Box::new(JsonParser),
        Parser::Yaml => Box::new(YamlParser),
    }
}

// ---------------------------------------------------------------------------
// FrontMatterPlusSections / MarkdownOnly — share a common section walker.
// ---------------------------------------------------------------------------

/// `front_matter_plus_sections` — strips an optional `---` YAML block from the
/// top of the document and walks `# / ## / ### …` headings.
pub struct FrontMatterPlusSections;

impl SourceParser for FrontMatterPlusSections {
    fn parse(&self, path: &Path, content: &str) -> Result<ParsedSource, ParseError> {
        let path_str = path.display().to_string();
        let (front_matter, body, body_offset) = split_front_matter(content, &path_str)?;
        let sections = walk_sections(body, body_offset);
        Ok(ParsedSource {
            path: path_str,
            front_matter,
            sections,
            full_text: body.to_owned(),
            line_count: body.lines().count(),
        })
    }
}

/// `markdown_only` — same heading walk, but never strips front-matter (the
/// `---` block, if present, is treated as a horizontal rule and stays in
/// `full_text`).
pub struct MarkdownOnly;

impl SourceParser for MarkdownOnly {
    fn parse(&self, path: &Path, content: &str) -> Result<ParsedSource, ParseError> {
        let sections = walk_sections(content, 0);
        Ok(ParsedSource {
            path: path.display().to_string(),
            front_matter: serde_json::Value::Object(serde_json::Map::new()),
            sections,
            full_text: content.to_owned(),
            line_count: content.lines().count(),
        })
    }
}

/// Splits an optional `---\n…\n---\n` YAML front-matter block off the top of
/// `content`. Returns `(front_matter_json, body_slice, body_line_offset)`.
fn split_front_matter<'a>(
    content: &'a str,
    path: &str,
) -> Result<(serde_json::Value, &'a str, usize), ParseError> {
    if !content.starts_with("---") {
        return Ok((empty_object(), content, 0));
    }
    // Skip the first delimiter line.
    let after_first = match content.find('\n') {
        Some(idx) => &content[idx + 1..],
        None => return Ok((empty_object(), content, 0)),
    };
    // Find the closing `---` that sits on its own line.
    let mut search_offset = 0;
    let close_idx = loop {
        match after_first[search_offset..].find("\n---") {
            Some(rel) => {
                let absolute = search_offset + rel;
                // Must be either end-of-string or followed by newline / CR.
                let after = absolute + 4; // "\n---".len()
                let next = after_first.as_bytes().get(after);
                match next {
                    None | Some(b'\n') | Some(b'\r') => break absolute,
                    _ => {
                        search_offset = absolute + 4;
                        continue;
                    }
                }
            }
            None => return Ok((empty_object(), content, 0)),
        }
    };
    let yaml_text = &after_first[..close_idx];
    let body_start = match after_first[close_idx + 1..].find('\n') {
        Some(rel) => close_idx + 1 + rel + 1,
        None => after_first.len(),
    };
    let body = &after_first[body_start..];

    // Parse YAML → serde_json::Value via serde_yaml.
    let fm: serde_json::Value = if yaml_text.trim().is_empty() {
        empty_object()
    } else {
        let yaml: serde_yaml::Value =
            serde_yaml::from_str(yaml_text).map_err(|e| ParseError::FrontMatter {
                path: path.to_owned(),
                source: e,
            })?;
        yaml_to_json(yaml)
    };

    // Body offset = lines consumed by `---` block + opening line.
    let consumed = &content[..content.len() - body.len()];
    let body_offset = consumed.lines().count();
    Ok((fm, body, body_offset))
}

fn empty_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

/// Convert a `serde_yaml::Value` to a `serde_json::Value` lossily (numbers
/// become JSON numbers; YAML-only constructs like binary tags become strings).
fn yaml_to_json(v: serde_yaml::Value) -> serde_json::Value {
    match v {
        serde_yaml::Value::Null => serde_json::Value::Null,
        serde_yaml::Value::Bool(b) => serde_json::Value::Bool(b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::Number(i.into())
            } else if let Some(u) = n.as_u64() {
                serde_json::Value::Number(u.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        serde_yaml::Value::String(s) => serde_json::Value::String(s),
        serde_yaml::Value::Sequence(seq) => {
            serde_json::Value::Array(seq.into_iter().map(yaml_to_json).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                let key = match k {
                    serde_yaml::Value::String(s) => s,
                    other => match serde_yaml::to_string(&other) {
                        Ok(s) => s.trim().to_owned(),
                        Err(_) => continue,
                    },
                };
                out.insert(key, yaml_to_json(v));
            }
            serde_json::Value::Object(out)
        }
        serde_yaml::Value::Tagged(t) => yaml_to_json(t.value),
    }
}

/// Walk `body` and emit one [`ParsedSection`] per ATX heading.
///
/// Headings are detected by the simple rule "line whose first non-whitespace
/// chars are a run of `#` followed by a space or end-of-line". Heading lines
/// inside fenced code blocks (` ``` `) are correctly ignored.
fn walk_sections(body: &str, line_offset: usize) -> HashMap<String, ParsedSection> {
    #[derive(Debug)]
    struct Pending {
        heading_level: u8,
        heading_text: String,
        line_start: usize,
        body_start_line: usize,
    }

    let lines: Vec<&str> = body.lines().collect();
    let mut sections: Vec<ParsedSection> = Vec::new();
    let mut stack: Vec<Pending> = Vec::new();
    let mut in_code_fence = false;

    let close_section = |out: &mut Vec<ParsedSection>,
                         pending: Pending,
                         body_lines: &[&str],
                         end_exclusive_idx: usize| {
        // body lines are zero-indexed inside `body_lines` slice, but
        // `body_start_line` and `line_end` are 1-indexed (with line_offset).
        let body_start_idx = pending.body_start_line.saturating_sub(line_offset + 1);
        let body_end_idx = end_exclusive_idx;
        let body_text = if body_start_idx < body_end_idx {
            body_lines[body_start_idx..body_end_idx].join("\n")
        } else {
            String::new()
        };
        // line_end = last body line, or heading line if body empty.
        let line_end = if body_end_idx > body_start_idx {
            line_offset + body_end_idx
        } else {
            pending.line_start
        };
        out.push(ParsedSection {
            heading_level: pending.heading_level,
            heading_text: pending.heading_text,
            line_start: pending.line_start,
            line_end,
            body: body_text.trim_end_matches('\n').to_owned(),
            sub_sections: Vec::new(),
        });
    };

    for (idx, raw_line) in lines.iter().enumerate() {
        let line_no_1based = line_offset + idx + 1;
        let trimmed = raw_line.trim_start();

        // Track fenced code blocks so heading-like lines inside code don't
        // create spurious sections.
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }
        if in_code_fence {
            continue;
        }

        if let Some((level, text)) = parse_atx_heading(trimmed) {
            // Pop any sections at this level or deeper — they end here.
            while let Some(top) = stack.last() {
                if top.heading_level >= level {
                    let pending = stack.pop().expect("stack non-empty by while cond");
                    close_section(&mut sections, pending, &lines, idx);
                } else {
                    break;
                }
            }
            stack.push(Pending {
                heading_level: level,
                heading_text: text,
                line_start: line_no_1based,
                body_start_line: line_no_1based + 1,
            });
        }
    }
    // Flush remaining sections — they end at EOF.
    let last_idx = lines.len();
    while let Some(pending) = stack.pop() {
        close_section(&mut sections, pending, &lines, last_idx);
    }

    // Compute sub_sections: for each section, find headings strictly nested
    // within its line range that are exactly one level deeper.
    let snapshot: Vec<ParsedSection> = sections.clone();
    let mut by_heading: HashMap<String, ParsedSection> = HashMap::new();
    for mut sec in sections {
        let parent_level = sec.heading_level;
        let parent_start = sec.line_start;
        let parent_end = sec.line_end;
        let mut subs: Vec<String> = Vec::new();
        for child in &snapshot {
            if child.heading_level == parent_level + 1
                && child.line_start > parent_start
                && child.line_start <= parent_end
            {
                subs.push(child.heading_text.clone());
            }
        }
        sec.sub_sections = subs;
        by_heading.insert(sec.heading_text.clone(), sec);
    }
    by_heading
}

/// Parse `## Heading` → `(2, "Heading")`. Returns `None` if `line` is not an
/// ATX heading (must have a space after the `#` run, or be exactly `#`*N).
fn parse_atx_heading(line: &str) -> Option<(u8, String)> {
    let mut count = 0usize;
    for c in line.chars() {
        if c == '#' {
            count += 1;
            if count > 6 {
                return None;
            }
        } else {
            break;
        }
    }
    if count == 0 {
        return None;
    }
    let rest = &line[count..];
    // Heading must be followed by space / tab / EOL.
    if !(rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t')) {
        return None;
    }
    // Trailing `#` runs ("## Heading ##") are dropped per CommonMark.
    let text = rest.trim().trim_end_matches('#').trim().to_owned();
    if text.is_empty() && rest.is_empty() {
        // bare "##" with no text — still a heading, empty text.
        return Some((count as u8, String::new()));
    }
    Some((count as u8, text))
}

// ---------------------------------------------------------------------------
// LogWithBlame — minimal git-log parser.
// ---------------------------------------------------------------------------

/// `log_with_blame` — interprets `content` as the output of `git log` and
/// emits one [`ParsedSection`] per commit. Commit boundaries are detected by
/// `^commit <sha>$` lines (the standard `git log` format).
///
/// Wave 2 ships a stub implementation: it captures commits but does not yet
/// run `git blame` (Wave 3 will read author/date metadata into `front_matter`).
pub struct LogWithBlame;

impl SourceParser for LogWithBlame {
    fn parse(&self, path: &Path, content: &str) -> Result<ParsedSource, ParseError> {
        let lines: Vec<&str> = content.lines().collect();
        let mut sections: HashMap<String, ParsedSection> = HashMap::new();
        let mut current: Option<(String, usize, Vec<String>)> = None;
        let mut last_idx = 0usize;

        for (idx, raw) in lines.iter().enumerate() {
            if let Some(sha) = raw.strip_prefix("commit ") {
                let sha = sha.trim().to_owned();
                if let Some((prev_sha, prev_start, prev_body)) = current.take() {
                    let body = prev_body.join("\n");
                    let body_trimmed = body.trim_end_matches('\n').to_owned();
                    sections.insert(
                        prev_sha.clone(),
                        ParsedSection {
                            heading_level: 1,
                            heading_text: prev_sha,
                            line_start: prev_start,
                            line_end: idx, // 1-indexed: previous line
                            body: body_trimmed,
                            sub_sections: Vec::new(),
                        },
                    );
                }
                current = Some((sha, idx + 1, Vec::new()));
            } else if let Some((_, _, ref mut body)) = current {
                body.push((*raw).to_owned());
            }
            last_idx = idx + 1;
        }
        if let Some((sha, start, body)) = current {
            let body_text = body.join("\n").trim_end_matches('\n').to_owned();
            sections.insert(
                sha.clone(),
                ParsedSection {
                    heading_level: 1,
                    heading_text: sha,
                    line_start: start,
                    line_end: last_idx,
                    body: body_text,
                    sub_sections: Vec::new(),
                },
            );
        }

        Ok(ParsedSource {
            path: path.display().to_string(),
            front_matter: empty_object(),
            sections,
            full_text: content.to_owned(),
            line_count: lines.len(),
        })
    }
}

// ---------------------------------------------------------------------------
// JsonParser / YamlParser — wrap whole document as front_matter.
// ---------------------------------------------------------------------------

/// `json` — wraps a JSON document as `front_matter` with no sections.
pub struct JsonParser;

impl SourceParser for JsonParser {
    fn parse(&self, path: &Path, content: &str) -> Result<ParsedSource, ParseError> {
        let path_str = path.display().to_string();
        let value: serde_json::Value =
            serde_json::from_str(content).map_err(|e| ParseError::Json {
                path: path_str.clone(),
                source: e,
            })?;
        Ok(ParsedSource {
            path: path_str,
            front_matter: value,
            sections: HashMap::new(),
            full_text: content.to_owned(),
            line_count: content.lines().count(),
        })
    }
}

/// `yaml` — wraps a YAML document as `front_matter` with no sections.
pub struct YamlParser;

impl SourceParser for YamlParser {
    fn parse(&self, path: &Path, content: &str) -> Result<ParsedSource, ParseError> {
        let path_str = path.display().to_string();
        let yaml: serde_yaml::Value =
            serde_yaml::from_str(content).map_err(|e| ParseError::YamlDocument {
                path: path_str.clone(),
                source: e,
            })?;
        Ok(ParsedSource {
            path: path_str,
            front_matter: yaml_to_json(yaml),
            sections: HashMap::new(),
            full_text: content.to_owned(),
            line_count: content.lines().count(),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p(name: &str) -> PathBuf {
        PathBuf::from(name)
    }

    #[test]
    fn front_matter_plus_sections_basic() {
        let md =
            "---\nname: hello\nkind: component\n---\n\n# Title\n\nintro line\n\n## Sub\n\nbody\n";
        let parsed = FrontMatterPlusSections.parse(&p("a.md"), md).unwrap();
        assert_eq!(
            parsed.front_matter.get("name").and_then(|v| v.as_str()),
            Some("hello")
        );
        assert!(parsed.sections.contains_key("Title"));
        assert!(parsed.sections.contains_key("Sub"));
        let title = &parsed.sections["Title"];
        assert_eq!(title.heading_level, 1);
        assert!(title.body.contains("intro line"));
    }

    #[test]
    fn front_matter_missing_returns_empty_object() {
        let md = "# Title\n\nbody\n";
        let parsed = FrontMatterPlusSections.parse(&p("a.md"), md).unwrap();
        assert!(parsed.front_matter.is_object());
        assert!(parsed.front_matter.as_object().unwrap().is_empty());
        assert!(parsed.sections.contains_key("Title"));
    }

    #[test]
    fn markdown_only_does_not_strip_front_matter() {
        let md = "---\nfoo: bar\n---\n\n# Title\n\nbody\n";
        let parsed = MarkdownOnly.parse(&p("a.md"), md).unwrap();
        // No front-matter extraction: the YAML lives inside full_text.
        assert!(parsed.front_matter.as_object().unwrap().is_empty());
        assert!(parsed.full_text.contains("foo: bar"));
        // Title still parsed as a section.
        assert!(parsed.sections.contains_key("Title"));
    }

    #[test]
    fn nested_headings_recorded_as_sub_sections() {
        let md = "# Top\n\nintro\n\n## A\n\nA body\n\n## B\n\nB body\n";
        let parsed = MarkdownOnly.parse(&p("x.md"), md).unwrap();
        let top = &parsed.sections["Top"];
        assert_eq!(top.heading_level, 1);
        assert!(top.sub_sections.contains(&"A".to_owned()));
        assert!(top.sub_sections.contains(&"B".to_owned()));
        // Each child appears as its own section too.
        assert_eq!(parsed.sections["A"].heading_level, 2);
        assert_eq!(parsed.sections["B"].heading_level, 2);
    }

    #[test]
    fn line_ranges_are_one_indexed_and_inclusive() {
        let md = "# Title\n\nL3\nL4\n\n## Sub\n\nL8\n";
        let parsed = MarkdownOnly.parse(&p("z.md"), md).unwrap();
        let title = &parsed.sections["Title"];
        assert_eq!(title.line_start, 1);
        // Title body extends until just before the next heading, so line_end
        // should reflect that range covering at least line 4.
        assert!(title.line_end >= 4);
        let sub = &parsed.sections["Sub"];
        assert_eq!(sub.line_start, 6);
        assert!(sub.line_end >= 8);
    }

    #[test]
    fn code_fences_block_heading_detection() {
        let md = "# Title\n\n```\n# not-a-heading\n## also-not\n```\n\n## Real\n\nbody\n";
        let parsed = MarkdownOnly.parse(&p("c.md"), md).unwrap();
        assert!(parsed.sections.contains_key("Title"));
        assert!(parsed.sections.contains_key("Real"));
        assert!(!parsed.sections.contains_key("not-a-heading"));
        assert!(!parsed.sections.contains_key("also-not"));
    }

    #[test]
    fn log_with_blame_parses_two_commits() {
        let log = "commit aaa\nAuthor: a\nDate: now\n\n    msg one\n\ncommit bbb\nAuthor: b\nDate: later\n\n    msg two\n";
        let parsed = LogWithBlame.parse(&p(".git/log"), log).unwrap();
        assert_eq!(parsed.sections.len(), 2);
        assert!(parsed.sections.contains_key("aaa"));
        assert!(parsed.sections.contains_key("bbb"));
    }

    #[test]
    fn json_parser_wraps_document_as_front_matter() {
        let parsed = JsonParser
            .parse(&p("d.json"), "{\"a\": 1, \"b\": [2,3]}")
            .unwrap();
        assert_eq!(parsed.front_matter["a"], serde_json::json!(1));
        assert_eq!(parsed.front_matter["b"], serde_json::json!([2, 3]));
        assert!(parsed.sections.is_empty());
    }

    #[test]
    fn yaml_parser_wraps_document_as_front_matter() {
        let parsed = YamlParser
            .parse(&p("d.yaml"), "a: 1\nb:\n  - 2\n  - 3\n")
            .unwrap();
        assert_eq!(parsed.front_matter["a"], serde_json::json!(1));
        assert_eq!(parsed.front_matter["b"], serde_json::json!([2, 3]));
    }

    #[test]
    fn parser_for_dispatches_correct_kind() {
        // Smoke test: `parser_for` returns something for every variant.
        let kinds = [
            Parser::FrontMatterPlusSections,
            Parser::MarkdownOnly,
            Parser::LogWithBlame,
            Parser::Json,
            Parser::Yaml,
        ];
        for k in &kinds {
            // Construction must not panic; running the parser on input
            // appropriate for FrontMatter/Markdown both succeed.
            let _ = parser_for(k);
        }
        // Round-trip: dispatch FrontMatter and verify it works.
        let p_md = parser_for(&Parser::MarkdownOnly);
        let parsed = p_md.parse(&p("foo.md"), "# Hi\n\nx\n").unwrap();
        assert!(parsed.sections.contains_key("Hi"));
    }

    #[test]
    fn empty_input_produces_no_sections() {
        let parsed = MarkdownOnly.parse(&p("e.md"), "").unwrap();
        assert!(parsed.sections.is_empty());
        assert_eq!(parsed.line_count, 0);
    }
}
