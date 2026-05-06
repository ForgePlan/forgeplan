use crate::artifact::frontmatter::Frontmatter;
use regex::Regex;
use std::sync::LazyLock;

/// PROB-059 — extract artifact IDs (`PRD-NNN`, `EVID-NNN`, etc.) mentioned
/// inside a `## Related Artifacts` table в the body. Returns the set of
/// IDs that appear в table rows. Other body mentions (free-text "see also
/// PRD-005") are intentionally NOT collected — only formal table rows count
/// as a "this artifact claims a relation here" signal.
///
/// Strict parser by design: looks for `^##+\s+Related Artifacts$` heading,
/// then collects table rows (`| ID-NNN | ... |`) until next heading. Code
/// blocks и HTML comments are stripped via `strip_non_prose_for_leakage`
/// (same helper PROB-038 introduced) so example tables в template guidance
/// don't false-flag.
pub fn extract_related_artifacts_table_ids(body: &str) -> Vec<String> {
    use std::collections::BTreeSet;
    let stripped = strip_non_prose_for_leakage(body);
    let lines: Vec<&str> = stripped.lines().collect();
    let mut start_idx: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let after = trimmed.trim_start_matches('#').trim();
            let lower = after.to_lowercase();
            if lower == "related artifacts" || lower == "related" {
                start_idx = Some(i + 1);
                break;
            }
        }
    }
    let Some(start) = start_idx else {
        return Vec::new();
    };
    let mut end = lines.len();
    for (i, line) in lines.iter().enumerate().skip(start) {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            end = i;
            break;
        }
    }
    static ID_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\b([A-Z]+-[0-9]+)\b").expect("valid ID regex"));
    let mut found: BTreeSet<String> = BTreeSet::new();
    for line in &lines[start..end] {
        let trimmed = line.trim();
        // Only consider table rows (start с `|` and contain at least 2 pipes)
        if !trimmed.starts_with('|') {
            continue;
        }
        // Skip separator rows (`|---|---|`)
        if trimmed
            .chars()
            .all(|c| c == '|' || c == '-' || c == ' ' || c == ':')
        {
            continue;
        }
        for m in ID_RE.find_iter(line) {
            found.insert(m.as_str().to_string());
        }
    }
    found.into_iter().collect()
}

/// PROB-059 — extract `target` IDs от frontmatter `links:` array.
pub fn extract_frontmatter_link_targets(fm: &Frontmatter) -> Vec<String> {
    let Some(links_val) = fm.get("links") else {
        return Vec::new();
    };
    let Some(seq) = links_val.as_sequence() else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    for entry in seq {
        if let Some(target) = entry.get("target").and_then(|v| v.as_str()) {
            out.push(target.to_string());
        }
    }
    out
}

/// Check that a frontmatter key exists and is non-empty.
pub fn frontmatter_has(fm: &Frontmatter, key: &str) -> bool {
    fm.get(key)
        .map(|v| match v {
            serde_yaml::Value::Null => false,
            serde_yaml::Value::String(s) => !s.trim().is_empty(),
            _ => true,
        })
        .unwrap_or(false)
}

/// Check that a markdown section with given heading text exists (any heading level).
/// Also checks known aliases (e.g. "Problem" matches "Motivation", "Problem Statement").
/// Uses string matching (no regex compilation per call).
pub fn section_exists(body: &str, heading: &str) -> bool {
    let headings_to_check = expand_aliases(heading);
    for h in headings_to_check {
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                // Strip leading '#' and check if remaining text starts with the heading
                let after_hashes = trimmed.trim_start_matches('#').trim_start();
                if after_hashes.eq_ignore_ascii_case(&h)
                    || after_hashes.to_lowercase().starts_with(&h.to_lowercase())
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Expand a heading name to include known aliases.
fn expand_aliases(heading: &str) -> Vec<String> {
    let mut result = vec![heading.to_string()];
    match heading {
        "Problem" => {
            result.push("Motivation".into());
            result.push("Problem Statement".into());
            result.push("Background".into());
        }
        "Goals" => {
            result.push("Success Criteria".into());
            result.push("Objectives".into());
        }
        "Non-Goals" => {
            result.push("Out of Scope".into());
            result.push("Product Scope".into()); // contains "Out of Scope" subsection
        }
        "Target" | "Audience" | "Users" => {
            result.push("Target Users".into());
            result.push("Target Audience".into());
            result.push("Users".into());
            result.push("Audience".into());
            result.push("Target".into());
        }
        "Related" => {
            result.push("Related Artifacts".into());
            result.push("Dependencies".into());
        }
        "Summary" => {
            result.push("Executive Summary".into());
            result.push("Overview".into());
        }
        "Vision" => {
            result.push("Goals".into());
            result.push("Executive Summary".into());
        }
        "Outcomes" => {
            result.push("Success Criteria".into());
            result.push("Goals".into());
        }
        "Children" => {
            result.push("Artifacts".into());
            result.push("PRDs".into());
            result.push("Components".into());
        }
        "Phases" => {
            result.push("Phase".into());
            result.push("Implementation Phases".into());
            result.push("Timeline".into());
        }
        "Progress" => {
            result.push("Status".into());
        }
        "Proposed" | "Direction" | "Architecture" => {
            result.push("Proposed".into());
            result.push("Proposed Direction".into());
            result.push("Architecture".into());
            result.push("Design".into());
        }
        "Motivation" => {
            result.push("Problem".into());
            result.push("Background".into());
            result.push("Why".into());
        }
        "Options" | "Alternatives" => {
            result.push("Options".into());
            result.push("Options Considered".into());
            result.push("Alternatives".into());
        }
        "Implementation" => {
            result.push("Implementation Phases".into());
            result.push("Phases".into());
            result.push("Plan".into());
        }
        _ => {}
    }
    result
}

/// Count words in a section (from heading to next heading of same or higher level).
/// Checks aliases if the primary heading is not found.
pub fn section_word_count(body: &str, heading: &str) -> usize {
    for h in expand_aliases(heading) {
        if let Some(content) = extract_section(body, &h) {
            let count = content.split_whitespace().count();
            if count > 0 {
                return count;
            }
        }
    }
    0
}

/// Count list items (lines starting with - or *) or table rows (lines with |) in a section.
pub fn section_item_count(body: &str, heading: &str) -> usize {
    if let Some(content) = extract_section(body, heading) {
        content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("- ")
                    || trimmed.starts_with("* ")
                    || (trimmed.starts_with("| ")
                        && !trimmed.starts_with("|---")
                        && !trimmed.contains("| --- |")
                        && !trimmed
                            .chars()
                            .skip(1)
                            .all(|c| c == '-' || c == '|' || c == ' '))
            })
            .count()
    } else {
        0
    }
}

/// Check for template placeholders like {{...}} or TODO/FIXME markers.
pub fn find_placeholders(body: &str) -> Vec<(usize, String)> {
    static PLACEHOLDER_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\{\{[^}]+\}\}").unwrap());
    static TODO_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\bTODO\b|\bFIXME\b|\bXXX\b").unwrap());

    let mut results = Vec::new();
    let mut in_code_fence = false;
    let mut in_comment = false;
    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        // Track fenced code blocks
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }
        if in_code_fence {
            continue;
        }
        // Track HTML comments
        if trimmed.starts_with("<!--") {
            in_comment = true;
        }
        if in_comment {
            if trimmed.contains("-->") {
                in_comment = false;
            }
            continue;
        }
        for m in PLACEHOLDER_RE.find_iter(line) {
            results.push((i + 1, m.as_str().to_string()));
        }
        // Flag TODO outside of comments
        if !trimmed.starts_with("//") {
            for m in TODO_RE.find_iter(line) {
                results.push((i + 1, m.as_str().to_string()));
            }
        }
    }
    results
}

/// BMAD Step 3: Filler phrases that reduce information density.
/// Each entry: (pattern_to_find, suggested_replacement)
const FILLER_CONVERSATIONAL: &[(&str, &str)] = &[
    ("the system will allow users to", "users can"),
    ("it is important to note that", ""),
    ("in order to", "to"),
    ("for the purpose of", "for"),
    ("with regard to", "regarding"),
    ("at this point in time", "now"),
    ("it should be noted that", ""),
    ("the system shall allow", "users can"),
    ("the application will provide", "provides"),
    ("users will be able to", "users can"),
    ("the platform should support", "supports"),
];

const FILLER_WORDY: &[(&str, &str)] = &[
    ("due to the fact that", "because"),
    ("in the event of", "if"),
    ("in a manner that", "how"),
    ("on a regular basis", "regularly"),
    ("in close proximity to", "near"),
];

const FILLER_REDUNDANT: &[(&str, &str)] = &[
    ("future plans", "plans"),
    ("past history", "history"),
    ("absolutely essential", "essential"),
    ("completely finish", "finish"),
    ("basic fundamentals", "fundamentals"),
    ("end result", "result"),
    ("final outcome", "outcome"),
];

/// Check for filler phrases in body text. Returns vec of (found_phrase, replacement, line_number).
pub fn check_filler_phrases(body: &str) -> Vec<(String, String, usize)> {
    let mut findings = Vec::new();

    for (line_num, line) in body.lines().enumerate() {
        let line_lower = line.to_lowercase();
        for (phrase, replacement) in FILLER_CONVERSATIONAL
            .iter()
            .chain(FILLER_WORDY.iter())
            .chain(FILLER_REDUNDANT.iter())
        {
            if line_lower.contains(phrase) {
                findings.push((phrase.to_string(), replacement.to_string(), line_num + 1));
            }
        }
    }
    findings
}

/// Compute density score: filler_count / total_words.
pub fn density_score(body: &str) -> f64 {
    let total_words = body.split_whitespace().count();
    if total_words == 0 {
        return 0.0;
    }
    let filler_count = check_filler_phrases(body).len();
    filler_count as f64 / total_words as f64
}

/// BMAD Step 7: Implementation technology keywords by category.
/// These should NOT appear in Functional Requirements.
const TECH_KEYWORDS_FRONTEND: &[&str] = &[
    "react",
    "vue",
    "angular",
    "svelte",
    "next.js",
    "nuxt",
    "gatsby",
    "webpack",
    "vite",
    "tailwind",
    "bootstrap",
    "material-ui",
];

const TECH_KEYWORDS_BACKEND: &[&str] = &[
    "express", "django", "rails", "spring", "laravel", "fastapi", "nestjs", "flask", "gin",
    "actix", "axum", "rocket",
];

const TECH_KEYWORDS_DATABASE: &[&str] = &[
    "postgresql",
    "mysql",
    "mongodb",
    "redis",
    "dynamodb",
    "cassandra",
    "sqlite",
    "elasticsearch",
    "neo4j",
    "cockroachdb",
    "lancedb",
];

const TECH_KEYWORDS_CLOUD: &[&str] = &[
    "aws",
    "gcp",
    "azure",
    "cloudflare",
    "vercel",
    "netlify",
    "heroku",
    "digitalocean",
    "fly.io",
];

const TECH_KEYWORDS_INFRA: &[&str] = &[
    "docker",
    "kubernetes",
    "terraform",
    "ansible",
    "helm",
    "nginx",
    "apache",
    "caddy",
    "traefik",
];

const TECH_KEYWORDS_AUTH: &[&str] = &[
    "jwt",
    "oauth",
    "oauth2",
    "saml",
    "ldap",
    "keycloak",
    "auth0",
    "cognito",
    "firebase auth",
];

const TECH_KEYWORDS_PROTOCOL: &[&str] = &[
    "rest",
    "graphql",
    "grpc",
    "websocket",
    "mqtt",
    "amqp",
    "kafka",
    "rabbitmq",
    "nats",
];

/// Collect all tech keywords into a single list for matching.
pub fn all_tech_keywords() -> Vec<&'static str> {
    let mut all = Vec::new();
    all.extend_from_slice(TECH_KEYWORDS_FRONTEND);
    all.extend_from_slice(TECH_KEYWORDS_BACKEND);
    all.extend_from_slice(TECH_KEYWORDS_DATABASE);
    all.extend_from_slice(TECH_KEYWORDS_CLOUD);
    all.extend_from_slice(TECH_KEYWORDS_INFRA);
    all.extend_from_slice(TECH_KEYWORDS_AUTH);
    all.extend_from_slice(TECH_KEYWORDS_PROTOCOL);
    all
}

/// Build compiled regexes for all tech keywords (lazy, one-time cost).
static TECH_KEYWORD_REGEXES: LazyLock<Vec<(String, Regex)>> = LazyLock::new(|| {
    all_tech_keywords()
        .into_iter()
        .filter_map(|kw| {
            let pattern = format!(r"(?i)\b{}\b", regex::escape(kw));
            Regex::new(&pattern).ok().map(|re| (kw.to_string(), re))
        })
        .collect()
});

/// Check for technology names in text (implementation leakage).
/// Returns list of (line_number, tech_name) using case-insensitive word boundary matching.
///
/// PROB-038 closure — pre-fix this function read raw lines including HTML
/// comments (`<!-- ... -->`) и markdown code fences (\`\`\`). Template
/// guidance comments в new PRDs contain phrases like "DON'T leak React,
/// Django, AWS into FR" which legitimately mention tech names — those
/// were false-flagged. Code fences и quoted strings can also legitimately
/// reference tech names в documentation context.
///
/// Now we strip both before scanning:
/// - `<!-- ... -->` HTML comments (single-line OR multi-line)
/// - \`\`\`...\`\`\` fenced code blocks
/// - inline backtick code (e.g. \`PostgreSQL\`)
///
/// Real leakage в FR/NFR text body still triggers — only template
/// guidance и code/quote contexts are immune.
pub fn find_tech_leakage(text: &str) -> Vec<(usize, String)> {
    let stripped = strip_non_prose_for_leakage(text);
    let mut results = Vec::new();
    for (i, line) in stripped.lines().enumerate() {
        for (keyword, re) in TECH_KEYWORD_REGEXES.iter() {
            if re.is_match(line) {
                results.push((i + 1, keyword.clone()));
            }
        }
    }
    results
}

/// PROB-038 helper — remove HTML comments, fenced code blocks, и inline
/// backtick code от `text` so that downstream checks (tech-name leakage,
/// numeric-target detection, etc.) only scan **prose**.
///
/// Replaces stripped regions с whitespace of equivalent line count so line
/// numbers in the returned diagnostic match the input. Order of operations:
///
/// 1. HTML comments first (multiline `<!-- ... -->`) — they can wrap
///    other markup, including code fences in template files.
/// 2. Fenced code blocks (\`\`\` opening to \`\`\` closing) — wraps inline
///    code samples that might mention tech names в documentation.
/// 3. Inline backtick code — single-line, narrowest scope, applied last.
fn strip_non_prose_for_leakage(text: &str) -> String {
    // Step 1: strip HTML comments. Replace each comment с newlines so line
    // numbers stay aligned (the linter reports "tech in FR section line N",
    // и we don't want N к shift after comment removal).
    let no_html = strip_html_comments(text);
    // Step 2: strip fenced code blocks.
    let no_fenced = strip_fenced_code(&no_html);
    // Step 3: strip inline backtick spans.
    strip_inline_code(&no_fenced)
}

fn strip_html_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 4 <= bytes.len() && &bytes[i..i + 4] == b"<!--" {
            // Find closing `-->`. Preserve newlines в the stripped region
            // so line numbers downstream match the input.
            if let Some(rel_end) = s[i + 4..].find("-->") {
                let comment_end = i + 4 + rel_end + 3;
                let stripped_region = &s[i..comment_end];
                for ch in stripped_region.chars() {
                    if ch == '\n' {
                        out.push('\n');
                    }
                }
                i = comment_end;
                continue;
            } else {
                // Unclosed comment — treat the rest as comment, preserve newlines.
                for ch in s[i..].chars() {
                    if ch == '\n' {
                        out.push('\n');
                    }
                }
                break;
            }
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn strip_fenced_code(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_fence = false;
    for line in s.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            out.push('\n');
            continue;
        }
        if in_fence {
            out.push('\n');
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn strip_inline_code(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_code = false;
    for ch in s.chars() {
        if ch == '`' {
            in_code = !in_code;
            // keep the backtick replaced by space so word-boundary regex
            // doesn't merge adjacent tokens
            out.push(' ');
            continue;
        }
        if in_code {
            // preserve newlines but blank out content
            if ch == '\n' {
                out.push('\n');
                in_code = false; // inline code never crosses lines
            } else {
                out.push(' ');
            }
            continue;
        }
        out.push(ch);
    }
    out
}

/// Check if text contains numeric targets (numbers with units or comparison).
pub fn has_numeric_targets(text: &str) -> bool {
    static NUMERIC_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[<>≤≥=]\s*\d+|(\d+[%ms]|\d+\.\d+)").unwrap());
    NUMERIC_RE.is_match(text)
}

/// Extract section content between a heading and the next heading of same/higher level.
/// Extract section content between a heading and the next heading of same or higher level.
/// Uses string matching (no regex per call). Includes sub-headings in extracted text.
pub fn extract_section(body: &str, heading: &str) -> Option<String> {
    let heading_lower = heading.to_lowercase();
    let mut found_level = 0usize;
    let mut start_line = 0usize;
    let mut found = false;

    let lines: Vec<&str> = body.lines().collect();

    // Find the heading line
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|c| *c == '#').count();
            let text = trimmed.trim_start_matches('#').trim_start();
            if text.to_lowercase().starts_with(&heading_lower) {
                found_level = level;
                start_line = i + 1;
                found = true;
                break;
            }
        }
    }

    if !found {
        return None;
    }

    // Find next heading of SAME or HIGHER level (lower number = higher level)
    let mut end_line = lines.len();
    for (i, line) in lines.iter().enumerate().skip(start_line) {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|c| *c == '#').count();
            if level <= found_level {
                end_line = i;
                break;
            }
            // Sub-headings (level > found_level) are INCLUDED
        }
    }

    let section: String = lines[start_line..end_line].join("\n");
    Some(section)
}

/// Extract the FR/Requirements section text, checking known heading aliases.
pub fn extract_fr_section(body: &str) -> Option<String> {
    for heading in &["Functional Requirements", "Requirements", "FR"] {
        if let Some(content) = extract_section(body, heading) {
            return Some(content);
        }
    }
    // Also try alias expansion for each
    for heading in &["Functional Requirements", "Requirements", "FR"] {
        for alias in expand_aliases(heading) {
            if let Some(content) = extract_section(body, &alias) {
                return Some(content);
            }
        }
    }
    None
}

/// Extract the NFR/Non-Functional Requirements section text.
pub fn extract_nfr_section(body: &str) -> Option<String> {
    for heading in &["Non-Functional Requirements", "NFR", "Quality Attributes"] {
        if let Some(content) = extract_section(body, heading) {
            return Some(content);
        }
    }
    None
}

/// Extract affected_files from body — lines that look like file paths or glob patterns.
pub fn extract_affected_files(body: &str) -> Vec<String> {
    let section = extract_section(body, "Affected Files")
        .or_else(|| extract_section(body, "Affected Scope"))
        .or_else(|| extract_section(body, "affected_files"));

    match section {
        None => vec![],
        Some(text) => text
            .lines()
            .map(|l| {
                l.trim()
                    .trim_start_matches("- ")
                    .trim_start_matches("* ")
                    .trim_start_matches("| ")
                    .trim()
            })
            .filter(|l| {
                !l.is_empty()
                    && (l.contains('/')
                        || l.contains('*')
                        || l.ends_with(".rs")
                        || l.ends_with(".ts")
                        || l.ends_with(".md"))
            })
            .map(|l| l.to_string())
            .collect(),
    }
}

/// BMAD Step 5: Subjective adjectives that need metrics.
const SUBJECTIVE_ADJECTIVES: &[&str] = &[
    "easy",
    "fast",
    "simple",
    "intuitive",
    "user-friendly",
    "responsive",
    "quick",
    "efficient",
    "robust",
    "scalable",
    "seamless",
    "smooth",
];

/// BMAD Step 5: Vague quantifiers without specifics.
const VAGUE_QUANTIFIERS: &[&str] = &[
    "multiple",
    "several",
    "some",
    "many",
    "few",
    "various",
    "number of",
    "a lot",
    "numerous",
];

/// Check for subjective adjectives in FR/requirements sections.
/// Returns vec of (found_word, line_number) — line numbers are relative to body start.
pub fn check_measurability_adjectives(body: &str) -> Vec<(String, usize)> {
    static ADJECTIVE_REGEXES: LazyLock<Vec<(String, Regex)>> = LazyLock::new(|| {
        SUBJECTIVE_ADJECTIVES
            .iter()
            .filter_map(|word| {
                let pattern = format!(r"(?i)\b{}\b", regex::escape(word));
                Regex::new(&pattern).ok().map(|re| (word.to_string(), re))
            })
            .collect()
    });

    let fr_section = match extract_fr_section(body) {
        Some(s) => s,
        None => return Vec::new(),
    };

    // Find the line offset where the FR section starts in the body
    let fr_start_offset = body.find(&fr_section).unwrap_or(0);
    let line_offset = body[..fr_start_offset].lines().count();

    let mut results = Vec::new();
    for (i, line) in fr_section.lines().enumerate() {
        for (word, re) in ADJECTIVE_REGEXES.iter() {
            if re.is_match(line) {
                results.push((word.clone(), line_offset + i + 1));
            }
        }
    }
    results
}

/// Check for vague quantifiers in FR/requirements sections.
/// Returns vec of (found_word, line_number) — line numbers are relative to body start.
pub fn check_vague_quantifiers(body: &str) -> Vec<(String, usize)> {
    static QUANTIFIER_REGEXES: LazyLock<Vec<(String, Regex)>> = LazyLock::new(|| {
        VAGUE_QUANTIFIERS
            .iter()
            .filter_map(|word| {
                let pattern = format!(r"(?i)\b{}\b", regex::escape(word));
                Regex::new(&pattern).ok().map(|re| (word.to_string(), re))
            })
            .collect()
    });

    let fr_section = match extract_fr_section(body) {
        Some(s) => s,
        None => return Vec::new(),
    };

    let fr_start_offset = body.find(&fr_section).unwrap_or(0);
    let line_offset = body[..fr_start_offset].lines().count();

    let mut results = Vec::new();
    for (i, line) in fr_section.lines().enumerate() {
        for (word, re) in QUANTIFIER_REGEXES.iter() {
            if re.is_match(line) {
                results.push((word.clone(), line_offset + i + 1));
            }
        }
    }
    results
}

/// Check that FR items follow "[Actor] can [capability]" format.
/// Looks for lines starting with "- [ ]" or "- [x]" with FR-NNN prefix in FR section.
/// Returns vec of (problematic_line_text, line_number).
pub fn check_fr_format(body: &str) -> Vec<(String, usize)> {
    static FR_LINE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^-\s+\[[ xX]\]\s+FR-\d+:\s*(.*)").unwrap());
    static ACTOR_CAN_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\b\w+\b\s+can\s+").unwrap());

    let fr_section = match extract_fr_section(body) {
        Some(s) => s,
        None => return Vec::new(),
    };

    let fr_start_offset = body.find(&fr_section).unwrap_or(0);
    let line_offset = body[..fr_start_offset].lines().count();

    let mut results = Vec::new();
    for (i, line) in fr_section.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(caps) = FR_LINE_RE.captures(trimmed) {
            let after_colon = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if !ACTOR_CAN_RE.is_match(after_colon) {
                results.push((trimmed.to_string(), line_offset + i + 1));
            }
        }
    }
    results
}

// ─── FR-002: Traceability Validation (BMAD Step 6) ─────────────────────────

/// Extract FR identifiers (e.g., "FR-001", "FR-002") from the FR section.
pub fn extract_fr_ids(body: &str) -> Vec<String> {
    static FR_ID_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"FR-\d+").unwrap());

    let fr_section = match extract_fr_section(body) {
        Some(s) => s,
        None => return Vec::new(),
    };

    let mut ids: Vec<String> = FR_ID_RE
        .find_iter(&fr_section)
        .map(|m| m.as_str().to_string())
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// Find orphan FRs — FR identifiers not mentioned outside the FR section itself.
/// Checks User Journey, User Stories, and general body text.
pub fn find_orphan_frs(body: &str) -> Vec<String> {
    let fr_ids = extract_fr_ids(body);
    if fr_ids.is_empty() {
        return Vec::new();
    }

    // Get body text excluding the FR section for tracing
    let body_without_fr = match extract_fr_section(body) {
        Some(fr_text) => body.replace(&fr_text, ""),
        None => return Vec::new(),
    };

    fr_ids
        .into_iter()
        .filter(|id| !body_without_fr.contains(id.as_str()))
        .collect()
}

/// Find orphan Goals — goals in the Goals section not supported by any FR.
/// Returns goal lines that don't reference or aren't referenced by any FR.
pub fn find_orphan_goals(body: &str) -> Vec<String> {
    let fr_section = match extract_fr_section(body) {
        Some(s) => s,
        None => return Vec::new(),
    };

    let goals_section = match extract_section(body, "Goals") {
        Some(s) => s,
        None => return Vec::new(),
    };

    // Extract goal items (bullet points)
    let goal_lines: Vec<&str> = goals_section
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("- ") || t.starts_with("* ") || t.starts_with("1.")
        })
        .collect();

    if goal_lines.is_empty() {
        return Vec::new();
    }

    // For each goal, check if any significant words appear in FR section
    let fr_lower = fr_section.to_lowercase();
    goal_lines
        .into_iter()
        .filter(|goal| {
            let goal_words: Vec<&str> = goal
                .trim_start_matches(|c: char| {
                    c == '-' || c == '*' || c.is_ascii_digit() || c == '.' || c == ' '
                })
                .split_whitespace()
                .filter(|w| w.len() > 3) // Skip short words
                .collect();
            // Goal is orphan if none of its significant words appear in FR
            !goal_words
                .iter()
                .any(|w| fr_lower.contains(&w.to_lowercase()))
        })
        .map(|l| l.trim().to_string())
        .collect()
}

// ─── FR-004: Domain Classification (BMAD Step 8) ───────────────────────────

/// Required sections by domain.
pub fn domain_required_sections(domain: &str) -> Vec<(&'static str, &'static str)> {
    match domain.to_lowercase().as_str() {
        "healthcare" | "health" => vec![
            ("Compliance", "HIPAA/regulatory compliance section"),
            ("Privacy", "Data privacy and patient data handling"),
            ("Audit", "Audit trail requirements"),
        ],
        "fintech" | "finance" => vec![
            ("Compliance", "Financial regulatory compliance"),
            ("Security", "Security requirements for financial data"),
            ("Audit", "Audit trail requirements"),
        ],
        "govtech" | "government" => vec![
            ("Compliance", "Government regulatory compliance"),
            (
                "Accessibility",
                "Accessibility requirements (WCAG/Section 508)",
            ),
            ("Security", "Security clearance/classification"),
        ],
        "edtech" | "education" => vec![
            ("Accessibility", "Accessibility requirements"),
            ("Privacy", "Student data privacy (FERPA/COPPA)"),
        ],
        "saas" | "b2b-saas" => vec![
            ("Multi-tenancy", "Multi-tenant architecture considerations"),
            ("SLA", "Service level agreements"),
            ("Security", "Security and data isolation"),
        ],
        _ => Vec::new(), // cli, library, etc. — no domain-specific requirements
    }
}

// ─── FR-005: Project-Type Classification (BMAD Step 9) ─────────────────────

/// Recommended sections by project type.
pub fn project_type_recommended_sections(project_type: &str) -> Vec<(&'static str, &'static str)> {
    match project_type.to_lowercase().as_str() {
        "api-backend" | "api" | "backend" => vec![
            ("API", "API contracts/endpoints"),
            ("Authentication", "Authentication/authorization approach"),
            (
                "Performance",
                "Performance requirements (latency, throughput)",
            ),
        ],
        "mobile-app" | "mobile" => vec![
            ("Platforms", "Supported platforms (iOS, Android)"),
            ("Offline", "Offline/connectivity handling"),
            ("Performance", "Performance requirements"),
        ],
        "saas-b2b" | "saas-b2c" | "web-app" => vec![
            ("Authentication", "Authentication/authorization"),
            ("Performance", "Performance requirements"),
            ("Accessibility", "Accessibility requirements"),
        ],
        "cli-tool" | "cli" => vec![
            ("Interface", "CLI interface/commands"),
            ("Error Handling", "Error messages and exit codes"),
        ],
        "library" | "sdk" | "framework" => vec![
            ("API", "Public API surface"),
            ("Compatibility", "Version compatibility/breaking changes"),
        ],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_exists() {
        let body = "## Goals\n\nSome goals here\n\n## Non-Goals\n\nNope";
        assert!(section_exists(body, "Goals"));
        assert!(section_exists(body, "Non-Goals"));
        assert!(!section_exists(body, "Missing"));
    }

    #[test]
    fn test_section_word_count() {
        let body =
            "## Problem\n\nThis is a problem with five words here and more.\n\n## Goals\n\nGoal 1";
        assert!(section_word_count(body, "Problem") >= 5);
    }

    #[test]
    fn test_find_placeholders() {
        let body = "Title: {{project_name}}\nDescription here\nTODO: fill this";
        let results = find_placeholders(body);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_has_numeric_targets() {
        assert!(has_numeric_targets("< 100ms"));
        assert!(has_numeric_targets("> 80%"));
        assert!(has_numeric_targets("achieve 99.9% uptime"));
        assert!(!has_numeric_targets("improve performance"));
    }

    #[test]
    fn test_find_tech_leakage() {
        let text = "User can login via React component";
        let leaks = find_tech_leakage(text);
        assert_eq!(leaks.len(), 1);
        assert_eq!(leaks[0].1, "react");
    }

    // PROB-038 — non-prose context exclusions

    /// PROB-038 — HTML comments containing tech names (template guidance)
    /// MUST NOT trigger leakage warning. Pre-PROB-038 the validator scanned
    /// raw text including `<!-- -->` blocks, false-flagging template hints
    /// like "DON'T leak React/Django/AWS into FR".
    #[test]
    fn find_tech_leakage_skips_html_comments() {
        let text = "<!-- Don't leak React, Django, AWS into FR -->\nUser can login";
        let leaks = find_tech_leakage(text);
        assert!(
            leaks.is_empty(),
            "tech names в HTML comments must not trigger; got: {leaks:?}"
        );
    }

    /// PROB-038 — multi-line HTML comments (typical в template files)
    /// also must be stripped.
    #[test]
    fn find_tech_leakage_skips_multiline_html_comments() {
        let text = "<!--\n  Avoid:\n  - React в FR\n  - PostgreSQL\n  - Redis\n-->\nFR-001: User can login";
        let leaks = find_tech_leakage(text);
        assert!(
            leaks.is_empty(),
            "multi-line HTML comments must not trigger; got: {leaks:?}"
        );
    }

    /// PROB-038 — fenced code blocks (\`\`\`) are documentation contexts;
    /// MUST NOT trigger leakage warning.
    #[test]
    fn find_tech_leakage_skips_fenced_code_blocks() {
        let text = "Description\n\n```\nReact component example\nDjango view\n```\n\nFR-001: User can login";
        let leaks = find_tech_leakage(text);
        assert!(
            leaks.is_empty(),
            "fenced code must not trigger; got: {leaks:?}"
        );
    }

    /// PROB-038 — inline backtick code (e.g. `\`PostgreSQL\``) MUST NOT
    /// trigger. Real prose still does.
    #[test]
    fn find_tech_leakage_skips_inline_backtick_code() {
        let text =
            "FR-001: User can use `Redis` if available — but the actual implementation is OAuth2";
        let leaks = find_tech_leakage(text);
        // Redis в backticks → skipped. OAuth2 в prose → flagged.
        assert!(
            !leaks.iter().any(|(_, name)| name == "redis"),
            "inline-backtick `Redis` must be skipped; got: {leaks:?}"
        );
        assert!(
            leaks.iter().any(|(_, name)| name == "oauth2"),
            "OAuth2 в prose must be flagged; got: {leaks:?}"
        );
    }

    /// PROB-038 — REGRESSION GUARD: real tech leakage в prose still triggers.
    /// Don't make the strip too aggressive.
    #[test]
    fn find_tech_leakage_still_catches_real_prose_leakage() {
        let text = "FR-001: User authenticates via React component with PostgreSQL backend";
        let leaks = find_tech_leakage(text);
        let names: Vec<&str> = leaks.iter().map(|(_, n)| n.as_str()).collect();
        assert!(names.contains(&"react"), "react в prose: {names:?}");
        assert!(
            names.contains(&"postgresql"),
            "postgresql в prose: {names:?}"
        );
    }

    // PROB-059 — Related Artifacts table extraction tests

    /// Happy path — table rows with IDs are extracted.
    #[test]
    fn extract_related_artifacts_table_ids_finds_table_rows() {
        let body = "
# PRD-007: Title

## Related Artifacts

| Artifact | Relation |
|---|---|
| PRD-001 | refines |
| EVID-042 | informs |
| RFC-003 | based_on |
";
        let ids = extract_related_artifacts_table_ids(body);
        assert_eq!(ids, vec!["EVID-042", "PRD-001", "RFC-003"]);
    }

    /// Free-text mention OUTSIDE the Related Artifacts section is NOT
    /// collected — strict parser by design (no false-flag on "see also").
    #[test]
    fn extract_related_artifacts_table_ids_ignores_freetext_mentions() {
        let body = "
# PRD-007

## Problem

This builds on PRD-001 and refines RFC-003.

## Related Artifacts

| Artifact | Relation |
|---|---|
| EVID-042 | informs |
";
        let ids = extract_related_artifacts_table_ids(body);
        assert_eq!(ids, vec!["EVID-042"]);
    }

    /// HTML comments in the Related Artifacts section are stripped first.
    #[test]
    fn extract_related_artifacts_table_ids_skips_html_comments() {
        let body = "
## Related Artifacts

<!-- Example template:
| Artifact | Relation |
| FAKE-999 | bogus |
-->

| Artifact | Relation |
|---|---|
| PRD-001 | refines |
";
        let ids = extract_related_artifacts_table_ids(body);
        assert_eq!(ids, vec!["PRD-001"]);
    }

    /// No section → empty result, no panic.
    #[test]
    fn extract_related_artifacts_table_ids_returns_empty_when_no_section() {
        let body = "# PRD-007: Title\n\n## Problem\n\nText.";
        let ids = extract_related_artifacts_table_ids(body);
        assert!(ids.is_empty());
    }

    /// Frontmatter link target extraction.
    #[test]
    fn extract_frontmatter_link_targets_basic() {
        let yaml = r#"
id: PRD-007
links:
  - target: PRD-001
    relation: refines
  - target: EVID-042
    relation: informs
"#;
        let fm: Frontmatter = serde_yaml::from_str(yaml).unwrap();
        let targets = extract_frontmatter_link_targets(&fm);
        assert_eq!(targets, vec!["PRD-001", "EVID-042"]);
    }

    /// Empty links: → empty result.
    #[test]
    fn extract_frontmatter_link_targets_empty_when_no_links() {
        let yaml = "id: PRD-007\nstatus: draft";
        let fm: Frontmatter = serde_yaml::from_str(yaml).unwrap();
        let targets = extract_frontmatter_link_targets(&fm);
        assert!(targets.is_empty());
    }
}
