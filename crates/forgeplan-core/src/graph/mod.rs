pub mod knowledge;
pub mod topological;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use crate::artifact::frontmatter;
use crate::hypothesis::HypothesisState;
use crate::link;

/// Edge in the dependency graph.
#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub relation: String,
}

/// Brownfield-aware rendering options for [`render_mermaid_with_opts`].
///
/// Issue #287 Phase F (W4) extends graph rendering with three orthogonal
/// concerns:
///
/// 1. **Per-state hypothesis colouring** — each `HYP-NNN` node fills with a
///    palette colour driven by its [`HypothesisState`] (verified=green,
///    inferred=amber, refuted=red, etc.). The state must be pre-loaded by
///    the caller — `render_mermaid_with_opts` is a pure function and does
///    no I/O.
/// 2. **Domain-model composition clusters** — each `DM-NNN` artifact maps to
///    a `subgraph DM-NNN[...]` block enclosing the UC/GLOS/INV/SCEN/HYP ids
///    listed in its `## Composition` section. Parsed by
///    [`composition_for_dm`] from each DM body.
/// 3. **Pipeline-vs-brownfield filter** — when `brownfield_only` is `true`,
///    the renderer drops edges whose endpoints aren't both brownfield kinds
///    (UC/GLOS/INV/SCEN/HYP/DM). Subgraph clusters are preserved.
///
/// Caller responsibility (CLI / MCP handler):
/// - Walk the workspace once, load HYP frontmatter via
///   [`crate::hypothesis::current_state_from_frontmatter`], populate
///   `hypothesis_states`.
/// - Read each DM body and call [`composition_for_dm`] to populate
///   `dm_compositions`.
/// - Set `brownfield_only` from the user-supplied flag.
///
/// The struct is `Default`-able for the legacy zero-arg
/// [`render_mermaid`] caller, which delegates with empty maps and
/// `brownfield_only = false`. Existing pipeline-only behaviour is
/// preserved byte-for-byte in that path.
#[derive(Debug, Default, Clone)]
pub struct BrownfieldRenderOpts {
    /// `HYP-NNN` → current verification state. When an edge touches a
    /// hypothesis whose id is absent from the map, the renderer falls back
    /// to the default amber styling (matches [`HypothesisState::Inferred`]).
    pub hypothesis_states: BTreeMap<String, HypothesisState>,
    /// `DM-NNN` → ordered list of composed artifact ids (UC/GLOS/INV/SCEN/
    /// HYP). Each entry produces a `subgraph DM-NNN[…]` block in the
    /// output. Empty list → cluster is still emitted as a labelled empty
    /// subgraph (signals "we know this DM exists; it has no recorded
    /// composition yet").
    pub dm_compositions: BTreeMap<String, Vec<String>>,
    /// When `true`, drop edges whose endpoints aren't both brownfield
    /// kinds. Subgraph clusters are emitted regardless (they're nodes, not
    /// edges, and the user explicitly asked to see the brownfield slice).
    pub brownfield_only: bool,
}

/// Build all edges by scanning all artifacts in the workspace.
pub async fn build_edges(workspace: &Path) -> anyhow::Result<Vec<Edge>> {
    let mut edges = Vec::new();

    for dir_name in crate::workspace::ARTIFACT_DIRS {
        let dir = workspace.join(dir_name);
        if !dir.exists() {
            continue;
        }
        let mut read_dir = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "md") {
                continue;
            }
            let content = tokio::fs::read_to_string(&path).await?;
            if let Ok((fm, _)) = frontmatter::parse_frontmatter(&content) {
                let id = match fm.get("id").and_then(|v| v.as_str()) {
                    Some(id) => id.to_string(),
                    None => continue,
                };
                let links = link::list_links(&fm);
                for (target, relation) in links {
                    edges.push(Edge {
                        from: id.clone(),
                        to: target,
                        relation,
                    });
                }
                // Also check parent_epic / epic / prd fields
                for field in &["epic", "prd", "parent_epic"] {
                    if let Some(serde_yaml::Value::String(parent)) = fm.get(*field)
                        && !parent.is_empty()
                    {
                        edges.push(Edge {
                            from: id.clone(),
                            to: parent.clone(),
                            relation: "belongs_to".to_string(),
                        });
                    }
                }
            }
        }
    }

    edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));
    Ok(edges)
}

/// Render edges as mermaid graph markup using the default (legacy)
/// rendering profile — pipeline kinds (PRD/RFC/ADR/EPIC/SPEC) only, no
/// hypothesis state colouring, no DM clusters.
///
/// Delegates to [`render_mermaid_with_opts`] with [`BrownfieldRenderOpts::default()`].
/// Existing zero-arg callers compile and behave identically.
pub fn render_mermaid(edges: &[Edge]) -> String {
    render_mermaid_with_opts(edges, &BrownfieldRenderOpts::default())
}

/// Render edges as mermaid graph markup with brownfield-aware extensions.
///
/// See [`BrownfieldRenderOpts`] for the contract on `opts`. This function
/// performs no I/O — all per-artifact metadata must be passed in.
pub fn render_mermaid_with_opts(edges: &[Edge], opts: &BrownfieldRenderOpts) -> String {
    // Step 1: optional pre-filter. Keep only edges where both endpoints
    // resolve to a brownfield kind. We do this BEFORE the empty-edges
    // sentinel check so that a brownfield-only filter that drops every
    // edge still surfaces the canonical "no links" placeholder rather
    // than an empty mermaid skeleton.
    let edges_owned: Vec<Edge>;
    let active_edges: &[Edge] = if opts.brownfield_only {
        edges_owned = edges
            .iter()
            .filter(|e| {
                is_brownfield_kind(kind_from_id(&e.from)) && is_brownfield_kind(kind_from_id(&e.to))
            })
            .cloned()
            .collect();
        &edges_owned
    } else {
        edges
    };

    // Render is governed by edges AND DM clusters: even with no edges, a
    // DM with a non-empty composition still produces visible output.
    if active_edges.is_empty() && opts.dm_compositions.is_empty() {
        return "graph LR\n    %% No links found between artifacts\n".to_string();
    }

    let mut lines = vec!["graph LR".to_string()];

    // Collect all node IDs for styling. We also add HYP ids that appear
    // only inside DM clusters (no incoming edge) so that classDef colour
    // styles fire for them.
    let mut nodes: BTreeMap<String, &str> = BTreeMap::new();
    for edge in active_edges {
        nodes
            .entry(edge.from.clone())
            .or_insert(kind_from_id(&edge.from));
        nodes
            .entry(edge.to.clone())
            .or_insert(kind_from_id(&edge.to));
    }
    // DM cluster contents — surface every composed id so it gets coloured
    // even if no edge touches it.
    for (dm_id, composed) in &opts.dm_compositions {
        nodes.entry(dm_id.clone()).or_insert(kind_from_id(dm_id));
        for member in composed {
            nodes.entry(member.clone()).or_insert(kind_from_id(member));
        }
    }

    // Step 2: emit subgraph clusters for each DM, BEFORE the edge list.
    // Mermaid resolves node ids globally, so declaring them inside a
    // subgraph here pins them visually without preventing later edge
    // references. Sort order is BTreeMap iteration (lexicographic by id)
    // so the output is deterministic.
    for (dm_id, composed) in &opts.dm_compositions {
        let label = render_subgraph_label(dm_id, &nodes);
        lines.push(format!("    subgraph {dm_id}[{label}]"));
        for member in composed {
            lines.push(format!("        {member}"));
        }
        lines.push("    end".to_string());
    }

    // Step 3: render edges. Track per-edge style overrides so we can
    // emit `linkStyle` directives once we know each edge's index.
    let mut link_styles: Vec<(usize, String)> = Vec::new();
    for (edge_idx, edge) in active_edges.iter().enumerate() {
        let edge_line = render_edge_line(edge, &opts.hypothesis_states, edge_idx, &mut link_styles);
        lines.push(edge_line);
    }

    // Step 4: emit linkStyle overrides for edges that need a colour or
    // dash style applied imperatively (mermaid syntax for "make link N
    // look like X").
    if !link_styles.is_empty() {
        lines.push(String::new());
        for (idx, style) in &link_styles {
            lines.push(format!("    linkStyle {idx} {style}"));
        }
    }

    // Step 5: style nodes by kind. The five legacy classes
    // (epic/prd/rfc/adr/spec) keep their colours; the six brownfield
    // classes layer on top. For hypothesis nodes the fill colour is
    // overridden per-id (each verification state gets its own class) so
    // we drop hypothesis ids from the bulk `class hypothesis` line and
    // emit them under a per-state class line further down.
    let mut style_groups: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (id, kind) in &nodes {
        if *kind == "hypothesis" {
            // Hypothesis nodes use per-state classes (rendered separately).
            continue;
        }
        style_groups.entry(kind).or_default().push(id.clone());
    }
    let mut hypothesis_by_state: HashMap<HypothesisState, Vec<String>> = HashMap::new();
    // `nodes` is a BTreeMap so iteration is already lexicographic; pushing
    // in that order yields sorted Vec contents per state without an extra
    // sort pass.
    for (id, kind) in &nodes {
        if *kind != "hypothesis" {
            continue;
        }
        let state = opts
            .hypothesis_states
            .get(id)
            .copied()
            .unwrap_or(HypothesisState::Inferred);
        hypothesis_by_state
            .entry(state)
            .or_default()
            .push(id.clone());
    }

    let has_classes = !style_groups.is_empty() || !hypothesis_by_state.is_empty();
    if has_classes {
        lines.push(String::new());

        // Legacy pipeline palette (unchanged).
        let pipeline_palette: &[(&str, &str, &str)] = &[
            ("epic", "epicStyle", "fill:#e1bee7,stroke:#7b1fa2"),
            ("prd", "prdStyle", "fill:#bbdefb,stroke:#1565c0"),
            ("rfc", "rfcStyle", "fill:#c8e6c9,stroke:#2e7d32"),
            ("adr", "adrStyle", "fill:#fff9c4,stroke:#f9a825"),
            ("spec", "specStyle", "fill:#ffe0b2,stroke:#e65100"),
        ];
        for (kind, class_name, color) in pipeline_palette {
            if let Some(ids) = style_groups.get(kind) {
                lines.push(format!("    classDef {} {}", class_name, color));
                lines.push(format!("    class {} {}", ids.join(","), class_name));
            }
        }

        // Brownfield palette (W4 contract — see module docs).
        let brownfield_palette: &[(&str, &str, &str)] = &[
            ("use_case", "useCaseStyle", "fill:#ffccbc,stroke:#bf360c"),
            ("glossary", "glossaryStyle", "fill:#d7ccc8,stroke:#3e2723"),
            ("invariant", "invariantStyle", "fill:#90caf9,stroke:#0d47a1"),
            ("scenario", "scenarioStyle", "fill:#a5d6a7,stroke:#1b5e20"),
            (
                "domain_model",
                "domainModelStyle",
                "fill:#ce93d8,stroke:#4a148c",
            ),
        ];
        for (kind, class_name, color) in brownfield_palette {
            if let Some(ids) = style_groups.get(kind) {
                lines.push(format!("    classDef {} {}", class_name, color));
                lines.push(format!("    class {} {}", ids.join(","), class_name));
            }
        }

        // Per-state hypothesis classes — five possible classes, each
        // emitted only if at least one HYP node falls into that state.
        // We always emit the classDef so its presence is independent of
        // node-set composition (tests assert the line is renderable
        // by checking the legend; the actual `class` line is gated on
        // presence of nodes).
        let hypothesis_palette: &[(HypothesisState, &str, &str)] = &[
            (
                HypothesisState::Verified,
                "hypothesisVerifiedStyle",
                "fill:#a5d6a7,stroke:#1b5e20",
            ),
            (
                HypothesisState::StrongInferred,
                "hypothesisStrongInferredStyle",
                "fill:#c8e6c9,stroke:#388e3c",
            ),
            (
                HypothesisState::Inferred,
                "hypothesisInferredStyle",
                "fill:#ffe082,stroke:#f57f17",
            ),
            (
                HypothesisState::Refuted,
                "hypothesisRefutedStyle",
                "fill:#ef9a9a,stroke:#b71c1c",
            ),
            (
                HypothesisState::Parked,
                "hypothesisParkedStyle",
                "fill:#cfd8dc,stroke:#37474f",
            ),
        ];
        // Whether any hypothesis node exists in this graph.
        let has_hypothesis_nodes = !hypothesis_by_state.is_empty();
        if has_hypothesis_nodes {
            for (_state, class_name, color) in hypothesis_palette {
                // Always emit classDef when any hypothesis node is present
                // (the legend asserts these lines exist for the brownfield
                // colour palette as a whole). Skip otherwise to keep
                // pipeline-only output unchanged.
                lines.push(format!("    classDef {} {}", class_name, color));
            }
        }
        // Per-state `class` lines — only emit for states that actually
        // have nodes assigned.
        for (state, class_name, _) in hypothesis_palette {
            if let Some(ids) = hypothesis_by_state.get(state) {
                lines.push(format!("    class {} {}", ids.join(","), class_name));
            }
        }
    }

    lines.join("\n") + "\n"
}

/// Single-edge mermaid line. Returns the rendered text and optionally
/// pushes a `linkStyle` override into `link_styles` keyed by `edge_idx`.
fn render_edge_line(
    edge: &Edge,
    hypothesis_states: &BTreeMap<String, HypothesisState>,
    edge_idx: usize,
    link_styles: &mut Vec<(usize, String)>,
) -> String {
    // Step 1: pick arrow + label syntax.
    let line = match edge.relation.as_str() {
        // `references` (glossary → use_case/invariant) is rendered with a
        // dashed arrow to signal a soft conceptual link rather than a
        // structural dependency.
        "references" => format!("    {} -.->|{}| {}", edge.from, edge.relation, edge.to),
        // `belongs_to` keeps the legacy plain-arrow shortcut (no label).
        "" | "belongs_to" => format!("    {} --> {}", edge.from, edge.to),
        // Everything else: solid arrow with label.
        _ => format!("    {} -->|{}| {}", edge.from, edge.relation, edge.to),
    };

    // Step 2: edge style overrides via linkStyle directive.
    match edge.relation.as_str() {
        // `covers` (invariant → scenario) gets a solid blue stroke.
        "covers" => {
            link_styles.push((edge_idx, "stroke:#1565c0,stroke-width:2px".to_string()));
        }
        // `triangulates` (hypothesis → use_case) inherits the hypothesis
        // state colour, so a refuted hypothesis paints its triangulation
        // edge red — the visual signal that the use-case rests on a
        // contradicted claim. When the hypothesis state is unknown we fall
        // back to amber (matches default Inferred).
        "triangulates" => {
            let state = hypothesis_states
                .get(&edge.from)
                .copied()
                .unwrap_or(HypothesisState::Inferred);
            let stroke = hypothesis_state_stroke(state);
            link_styles.push((edge_idx, format!("stroke:{stroke},stroke-width:2px")));
        }
        _ => {}
    }

    line
}

/// Return the stroke colour from the hypothesis-state palette. Mirrors
/// the fill palette but returns only the `stroke:` half (mermaid
/// linkStyle wants `stroke:` not `fill:`).
fn hypothesis_state_stroke(state: HypothesisState) -> &'static str {
    match state {
        HypothesisState::Verified => "#1b5e20",
        HypothesisState::StrongInferred => "#388e3c",
        HypothesisState::Inferred => "#f57f17",
        HypothesisState::Refuted => "#b71c1c",
        HypothesisState::Parked => "#37474f",
    }
}

/// Build the mermaid subgraph label `"DM-001: title"` if we can recover a
/// title from a node entry, otherwise fall back to the raw id. The label
/// is wrapped in double quotes per mermaid syntax for free-form text.
///
/// Why a function: keeps the subgraph block self-contained and ready to
/// accept title-lookup data in a follow-up patch without churning the
/// caller. Right now `nodes` only carries `kind`; titles need a separate
/// data source. We emit the id as-is for V1.
fn render_subgraph_label(dm_id: &str, _nodes: &BTreeMap<String, &str>) -> String {
    // Mermaid requires the bracket body to be quoted when it contains
    // colons / spaces; the simplest form is `"DM-001"`.
    format!("\"{dm_id}\"")
}

/// Parse the `## Composition` section of a domain-model body and return
/// the list of artifact ids it lists, in source order.
///
/// The composition section is documented as:
///
/// ```text
/// ## Composition
/// This model aggregates:
/// - GLOS-XXX — canonical terms
/// - INV-XXX — invariants
/// - UC-XXX — use cases
/// - SCEN-XXX — scenarios
/// - HYP-XXX — hypotheses
/// ```
///
/// Parser contract:
/// - Find the literal `## Composition` heading (case-insensitive,
///   tolerant of leading whitespace).
/// - Consume body lines until the next `## ` heading (or end-of-body).
/// - For each line, extract the FIRST id-like token matching the regex
///   `(UC|GLOS|INV|SCEN|HYP|DM)-\d+`.
/// - Skip lines that contain no such token (intro prose, blanks, fences).
/// - Duplicates are de-duplicated **within** this call (first occurrence
///   wins) so a body that lists `UC-001` twice produces one entry.
///
/// On unparseable input (no `## Composition` heading, malformed content)
/// the function returns an empty vector — never panics. Callers that
/// expect a non-empty list should treat empty as "the DM didn't tell us
/// what it composes".
pub fn composition_for_dm(body: &str) -> Vec<String> {
    composition_from_text(body)
}

fn composition_from_text(body: &str) -> Vec<String> {
    const HEADING: &str = "## composition";

    let lines: Vec<&str> = body.lines().collect();
    let heading_idx = lines
        .iter()
        .position(|l| l.trim_start().to_ascii_lowercase().starts_with(HEADING));
    let Some(idx) = heading_idx else {
        return Vec::new();
    };

    let mut end_idx = lines.len();
    for (i, line) in lines.iter().enumerate().skip(idx + 1) {
        let t = line.trim_start();
        if t.starts_with("## ") || t.starts_with("# ") {
            end_idx = i;
            break;
        }
    }

    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out: Vec<String> = Vec::new();
    for line in &lines[(idx + 1)..end_idx] {
        if let Some(id) = extract_brownfield_id_token(line)
            && seen.insert(id.clone())
        {
            out.push(id);
        }
    }
    out
}

/// Extract the first token matching `^[\s\-*\d]*[A-Z]+-\d+` after
/// trimming non-token leading characters. We accept UC / GLOS / INV /
/// SCEN / HYP / DM prefixes. Returns the uppercased canonical form.
///
/// This is intentionally regex-free — a hand-rolled scanner that is
/// cheap enough at the parse rate we use (DM bodies are small, called
/// once per `forgeplan graph` invocation, never in a hot loop).
fn extract_brownfield_id_token(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Walk character-by-character looking for an uppercase letter run
    // followed by `-` followed by digits.
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Skip until uppercase letter (case-insensitive — we'll uppercase
        // the captured token before returning).
        if !bytes[i].is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        // Scan run of alphabetic chars.
        let prefix_start = i;
        while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
            i += 1;
        }
        let prefix_end = i;
        // Must be followed by `-`.
        if i >= bytes.len() || bytes[i] != b'-' {
            continue;
        }
        let dash_idx = i;
        i += 1;
        // Must be followed by at least one digit.
        let digit_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == digit_start {
            continue;
        }
        let prefix = &trimmed[prefix_start..prefix_end];
        let prefix_upper = prefix.to_ascii_uppercase();
        if matches!(
            prefix_upper.as_str(),
            "UC" | "GLOS" | "INV" | "SCEN" | "HYP" | "DM"
        ) {
            let digits = &trimmed[(dash_idx + 1)..i];
            return Some(format!("{prefix_upper}-{digits}"));
        }
        // Not a brownfield prefix — continue scanning beyond this token.
    }
    None
}

fn kind_from_id(id: &str) -> &'static str {
    let upper = id.to_uppercase();
    if upper.starts_with("EPIC") {
        "epic"
    } else if upper.starts_with("PRD") {
        "prd"
    } else if upper.starts_with("RFC") {
        "rfc"
    } else if upper.starts_with("ADR") {
        "adr"
    } else if upper.starts_with("SPEC") {
        "spec"
    } else if upper.starts_with("UC-") {
        "use_case"
    } else if upper.starts_with("GLOS-") {
        "glossary"
    } else if upper.starts_with("INV-") {
        "invariant"
    } else if upper.starts_with("SCEN-") {
        "scenario"
    } else if upper.starts_with("HYP-") {
        "hypothesis"
    } else if upper.starts_with("DM-") {
        "domain_model"
    } else {
        "other"
    }
}

/// Whether the given `kind_from_id` output corresponds to a brownfield
/// (Issue #287) kind. Used by the `--brownfield-only` filter.
fn is_brownfield_kind(kind: &str) -> bool {
    matches!(
        kind,
        "use_case" | "glossary" | "invariant" | "scenario" | "hypothesis" | "domain_model"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- kind_from_id ---

    #[test]
    fn kind_from_id_all_known_kinds() {
        assert_eq!(kind_from_id("EPIC-001"), "epic");
        assert_eq!(kind_from_id("epic-001"), "epic");
        assert_eq!(kind_from_id("PRD-001"), "prd");
        assert_eq!(kind_from_id("prd-001"), "prd");
        assert_eq!(kind_from_id("RFC-001"), "rfc");
        assert_eq!(kind_from_id("rfc-001"), "rfc");
        assert_eq!(kind_from_id("ADR-001"), "adr");
        assert_eq!(kind_from_id("adr-001"), "adr");
        assert_eq!(kind_from_id("SPEC-001"), "spec");
        assert_eq!(kind_from_id("spec-001"), "spec");
    }

    #[test]
    fn kind_from_id_unknown_returns_other() {
        // Issue #287 Phase F (W4): NOTE / PROB / EVID stay "other"
        // because they're not in the brownfield contract. The six new
        // brownfield prefixes are covered by a dedicated test below.
        assert_eq!(kind_from_id("NOTE-001"), "other");
        assert_eq!(kind_from_id("PROB-001"), "other");
        assert_eq!(kind_from_id("EVID-007"), "other");
        assert_eq!(kind_from_id("SOL-001"), "other");
        assert_eq!(kind_from_id("REF-001"), "other");
        assert_eq!(kind_from_id(""), "other");
    }

    #[test]
    fn kind_from_id_recognises_all_six_brownfield_prefixes() {
        // Issue #287 Phase F (W4) contract: each brownfield kind has a
        // distinct prefix and the `kind_from_id` mapping is the single
        // source of truth feeding both render_mermaid colour styles and
        // the `--brownfield-only` filter predicate.
        assert_eq!(kind_from_id("UC-001"), "use_case");
        assert_eq!(kind_from_id("uc-001"), "use_case");
        assert_eq!(kind_from_id("GLOS-002"), "glossary");
        assert_eq!(kind_from_id("glos-002"), "glossary");
        assert_eq!(kind_from_id("INV-003"), "invariant");
        assert_eq!(kind_from_id("inv-003"), "invariant");
        assert_eq!(kind_from_id("SCEN-004"), "scenario");
        assert_eq!(kind_from_id("scen-004"), "scenario");
        assert_eq!(kind_from_id("HYP-005"), "hypothesis");
        assert_eq!(kind_from_id("hyp-005"), "hypothesis");
        assert_eq!(kind_from_id("DM-006"), "domain_model");
        assert_eq!(kind_from_id("dm-006"), "domain_model");
    }

    #[test]
    fn is_brownfield_kind_only_returns_true_for_six_kinds() {
        for kind in [
            "use_case",
            "glossary",
            "invariant",
            "scenario",
            "hypothesis",
            "domain_model",
        ] {
            assert!(is_brownfield_kind(kind), "expected {kind} to be brownfield");
        }
        for kind in ["prd", "rfc", "adr", "epic", "spec", "other"] {
            assert!(
                !is_brownfield_kind(kind),
                "expected {kind} to NOT be brownfield"
            );
        }
    }

    // --- render_mermaid (legacy zero-arg path) ---

    #[test]
    fn render_mermaid_empty_edges() {
        let output = render_mermaid(&[]);
        assert_eq!(
            output,
            "graph LR\n    %% No links found between artifacts\n"
        );
    }

    #[test]
    fn render_mermaid_single_edge_with_relation() {
        let edges = vec![Edge {
            from: "PRD-001".to_string(),
            to: "RFC-001".to_string(),
            relation: "informs".to_string(),
        }];
        let output = render_mermaid(&edges);
        assert!(output.starts_with("graph LR\n"));
        assert!(output.contains("PRD-001 -->|informs| RFC-001"));
        // Uses classDef syntax, not old `style`
        assert!(output.contains("classDef"));
        assert!(output.contains("class "));
        assert!(!output.contains("\n    style "));
    }

    #[test]
    fn render_mermaid_multiple_edges_same_kind() {
        let edges = vec![
            Edge {
                from: "PRD-001".to_string(),
                to: "RFC-001".to_string(),
                relation: "informs".to_string(),
            },
            Edge {
                from: "PRD-002".to_string(),
                to: "RFC-001".to_string(),
                relation: "informs".to_string(),
            },
        ];
        let output = render_mermaid(&edges);
        // Both PRD nodes should appear in one `class` line for prdStyle
        assert!(output.contains("classDef prdStyle"));
        assert!(output.contains("classDef rfcStyle"));
    }

    #[test]
    fn render_mermaid_empty_relation_uses_plain_arrow() {
        let edges = vec![Edge {
            from: "PRD-001".to_string(),
            to: "EPIC-001".to_string(),
            relation: "belongs_to".to_string(),
        }];
        let output = render_mermaid(&edges);
        // belongs_to relation uses plain --> without label
        assert!(output.contains("PRD-001 --> EPIC-001"));
        assert!(!output.contains("|belongs_to|"));
    }

    #[test]
    fn render_mermaid_named_relation_uses_label_arrow() {
        let edges = vec![Edge {
            from: "RFC-001".to_string(),
            to: "PRD-001".to_string(),
            relation: "based_on".to_string(),
        }];
        let output = render_mermaid(&edges);
        assert!(output.contains("RFC-001 -->|based_on| PRD-001"));
    }

    // --- render_mermaid_with_opts (brownfield path) ---

    #[test]
    fn render_mermaid_emits_brownfield_class_def_lines() {
        // One edge per brownfield kind (chosen to make every kind appear
        // as either edge.from or edge.to). Each classDef line must
        // surface in the output.
        let edges = vec![
            Edge {
                from: "UC-001".to_string(),
                to: "INV-001".to_string(),
                relation: "refines".to_string(),
            },
            Edge {
                from: "GLOS-001".to_string(),
                to: "UC-001".to_string(),
                relation: "references".to_string(),
            },
            Edge {
                from: "INV-001".to_string(),
                to: "SCEN-001".to_string(),
                relation: "covers".to_string(),
            },
            Edge {
                from: "HYP-001".to_string(),
                to: "UC-001".to_string(),
                relation: "triangulates".to_string(),
            },
            Edge {
                from: "DM-001".to_string(),
                to: "UC-001".to_string(),
                relation: "informs".to_string(),
            },
        ];
        let opts = BrownfieldRenderOpts::default();
        let output = render_mermaid_with_opts(&edges, &opts);

        // Each brownfield classDef must be present.
        assert!(
            output.contains("classDef useCaseStyle"),
            "useCaseStyle missing: {output}"
        );
        assert!(
            output.contains("classDef glossaryStyle"),
            "glossaryStyle missing: {output}"
        );
        assert!(
            output.contains("classDef invariantStyle"),
            "invariantStyle missing: {output}"
        );
        assert!(
            output.contains("classDef scenarioStyle"),
            "scenarioStyle missing: {output}"
        );
        assert!(
            output.contains("classDef domainModelStyle"),
            "domainModelStyle missing: {output}"
        );
        // Hypothesis classes are emitted because HYP-001 is in the node set.
        assert!(
            output.contains("classDef hypothesisInferredStyle"),
            "hypothesisInferredStyle missing: {output}"
        );
        assert!(
            output.contains("classDef hypothesisVerifiedStyle"),
            "hypothesisVerifiedStyle missing: {output}"
        );
        assert!(
            output.contains("classDef hypothesisRefutedStyle"),
            "hypothesisRefutedStyle missing: {output}"
        );
    }

    #[test]
    fn render_mermaid_covers_edge_uses_blue_style() {
        let edges = vec![Edge {
            from: "INV-001".to_string(),
            to: "SCEN-001".to_string(),
            relation: "covers".to_string(),
        }];
        let opts = BrownfieldRenderOpts::default();
        let output = render_mermaid_with_opts(&edges, &opts);
        // Plain label arrow with `covers` label PLUS a linkStyle 0 blue.
        assert!(
            output.contains("INV-001 -->|covers| SCEN-001"),
            "edge missing: {output}"
        );
        assert!(
            output.contains("linkStyle 0 stroke:#1565c0"),
            "blue linkStyle for covers missing: {output}"
        );
    }

    #[test]
    fn render_mermaid_references_edge_uses_dashed_arrow() {
        let edges = vec![Edge {
            from: "GLOS-001".to_string(),
            to: "UC-001".to_string(),
            relation: "references".to_string(),
        }];
        let opts = BrownfieldRenderOpts::default();
        let output = render_mermaid_with_opts(&edges, &opts);
        // The dashed mermaid arrow is `-.->`.
        assert!(
            output.contains("GLOS-001 -.->|references| UC-001"),
            "dashed arrow for references missing: {output}"
        );
    }

    #[test]
    fn render_mermaid_brownfield_only_filters_pipeline_edges() {
        let edges = vec![
            Edge {
                from: "PRD-001".to_string(),
                to: "RFC-001".to_string(),
                relation: "informs".to_string(),
            },
            Edge {
                from: "UC-001".to_string(),
                to: "SCEN-001".to_string(),
                relation: "demonstrates".to_string(),
            },
        ];
        let opts = BrownfieldRenderOpts {
            brownfield_only: true,
            ..BrownfieldRenderOpts::default()
        };
        let output = render_mermaid_with_opts(&edges, &opts);
        assert!(
            output.contains("UC-001 -->|demonstrates| SCEN-001"),
            "brownfield edge must survive: {output}"
        );
        assert!(
            !output.contains("PRD-001"),
            "pipeline edge must be filtered: {output}"
        );
        assert!(
            !output.contains("RFC-001"),
            "pipeline endpoint must be filtered: {output}"
        );
    }

    #[test]
    fn render_mermaid_brownfield_only_empty_after_filter_returns_placeholder() {
        // All edges are pipeline → filter drops everything → expect the
        // canonical "no links" placeholder rather than an empty skeleton.
        let edges = vec![Edge {
            from: "PRD-001".to_string(),
            to: "RFC-001".to_string(),
            relation: "informs".to_string(),
        }];
        let opts = BrownfieldRenderOpts {
            brownfield_only: true,
            ..BrownfieldRenderOpts::default()
        };
        let output = render_mermaid_with_opts(&edges, &opts);
        assert_eq!(
            output,
            "graph LR\n    %% No links found between artifacts\n"
        );
    }

    #[test]
    fn render_mermaid_domain_model_subgraph() {
        // DM-001 composes UC-001 + INV-001 + SCEN-001. Composition map
        // is the sole signal; no explicit DM edge in the edge list.
        let edges = vec![Edge {
            from: "UC-001".to_string(),
            to: "INV-001".to_string(),
            relation: "refines".to_string(),
        }];
        let mut dm_compositions = BTreeMap::new();
        dm_compositions.insert(
            "DM-001".to_string(),
            vec![
                "UC-001".to_string(),
                "INV-001".to_string(),
                "SCEN-001".to_string(),
            ],
        );
        let opts = BrownfieldRenderOpts {
            dm_compositions,
            ..BrownfieldRenderOpts::default()
        };
        let output = render_mermaid_with_opts(&edges, &opts);
        assert!(
            output.contains("subgraph DM-001[\"DM-001\"]"),
            "DM subgraph header missing: {output}"
        );
        assert!(
            output.contains("    end"),
            "DM subgraph close missing: {output}"
        );
        // Composed ids must appear inside the subgraph body — we check
        // presence on a line that doesn't also have the `-->` arrow.
        for member in ["UC-001", "INV-001", "SCEN-001"] {
            assert!(
                output.lines().any(|l| l.trim() == member),
                "{member} must appear as a bare node line inside subgraph: {output}"
            );
        }
        // SCEN-001 appeared only in the composition list, no edge — it
        // must still be styled.
        assert!(
            output.contains("scenarioStyle"),
            "composition-only members must still receive their class: {output}"
        );
    }

    #[test]
    fn render_mermaid_hypothesis_colour_by_state() {
        let edges = vec![
            Edge {
                from: "HYP-001".to_string(),
                to: "UC-001".to_string(),
                relation: "triangulates".to_string(),
            },
            Edge {
                from: "HYP-002".to_string(),
                to: "UC-002".to_string(),
                relation: "triangulates".to_string(),
            },
            Edge {
                from: "HYP-003".to_string(),
                to: "UC-003".to_string(),
                relation: "triangulates".to_string(),
            },
        ];
        let mut hypothesis_states = BTreeMap::new();
        hypothesis_states.insert("HYP-001".to_string(), HypothesisState::Verified);
        hypothesis_states.insert("HYP-002".to_string(), HypothesisState::Inferred);
        hypothesis_states.insert("HYP-003".to_string(), HypothesisState::Refuted);
        let opts = BrownfieldRenderOpts {
            hypothesis_states,
            ..BrownfieldRenderOpts::default()
        };
        let output = render_mermaid_with_opts(&edges, &opts);

        // Per-state class lines must each include the right HYP id.
        assert!(
            output.contains("class HYP-001 hypothesisVerifiedStyle"),
            "verified class missing: {output}"
        );
        assert!(
            output.contains("class HYP-002 hypothesisInferredStyle"),
            "inferred class missing: {output}"
        );
        assert!(
            output.contains("class HYP-003 hypothesisRefutedStyle"),
            "refuted class missing: {output}"
        );

        // `triangulates` edges must inherit the per-state stroke colour
        // via a linkStyle directive. We check the three colours appear
        // among the linkStyle lines (order depends on edge index).
        assert!(
            output.contains("stroke:#1b5e20"),
            "verified stroke missing: {output}"
        );
        assert!(
            output.contains("stroke:#f57f17"),
            "inferred stroke missing: {output}"
        );
        assert!(
            output.contains("stroke:#b71c1c"),
            "refuted stroke missing: {output}"
        );
    }

    #[test]
    fn render_mermaid_unknown_hypothesis_state_defaults_to_inferred() {
        // No state recorded for HYP-001 → renderer must fall back to
        // the inferred (amber) class.
        let edges = vec![Edge {
            from: "HYP-001".to_string(),
            to: "UC-001".to_string(),
            relation: "triangulates".to_string(),
        }];
        let opts = BrownfieldRenderOpts::default();
        let output = render_mermaid_with_opts(&edges, &opts);
        assert!(
            output.contains("class HYP-001 hypothesisInferredStyle"),
            "unknown-state HYP must default to inferred class: {output}"
        );
        assert!(
            output.contains("stroke:#f57f17"),
            "default triangulates stroke must be amber: {output}"
        );
    }

    #[test]
    fn render_mermaid_default_opts_matches_legacy_render() {
        // The opts-free `render_mermaid` MUST produce byte-identical
        // output to `render_mermaid_with_opts(edges, &Default::default())`.
        // Pins the legacy contract against accidental drift.
        let edges = vec![
            Edge {
                from: "PRD-001".to_string(),
                to: "RFC-001".to_string(),
                relation: "informs".to_string(),
            },
            Edge {
                from: "RFC-001".to_string(),
                to: "ADR-001".to_string(),
                relation: "informs".to_string(),
            },
        ];
        let legacy = render_mermaid(&edges);
        let via_opts = render_mermaid_with_opts(&edges, &BrownfieldRenderOpts::default());
        assert_eq!(legacy, via_opts);
    }

    // --- composition_for_dm ---

    #[test]
    fn composition_for_dm_extracts_ids_from_section() {
        let body = "---\n\
id: DM-001\n\
title: foo\n\
---\n\
\n\
# DM-001: foo\n\
\n\
## Domain\n\
Some text.\n\
\n\
## Composition\n\
This model aggregates:\n\
- GLOS-001 — terms\n\
- INV-002 — invariants\n\
- UC-003 — use cases\n\
- SCEN-004 — scenarios\n\
- HYP-005 — hypotheses\n\
\n\
## Source\n\
Where it came from.\n";
        let ids = composition_for_dm(body);
        assert_eq!(
            ids,
            vec![
                "GLOS-001".to_string(),
                "INV-002".to_string(),
                "UC-003".to_string(),
                "SCEN-004".to_string(),
                "HYP-005".to_string(),
            ]
        );
    }

    #[test]
    fn composition_for_dm_returns_empty_when_section_absent() {
        let body = "---\n\
id: DM-009\n\
---\n\
\n\
# DM-009\n\
\n\
## Domain\n\
no composition section.\n";
        let ids = composition_for_dm(body);
        assert!(
            ids.is_empty(),
            "missing section must return empty vec: {ids:?}"
        );
    }

    #[test]
    fn composition_for_dm_deduplicates_within_section() {
        let body = "## Composition\n\
- UC-001 — first\n\
- UC-001 — duplicate\n\
- GLOS-002 — single\n\
\n\
## Source\n";
        let ids = composition_for_dm(body);
        assert_eq!(ids, vec!["UC-001".to_string(), "GLOS-002".to_string()]);
    }

    #[test]
    fn composition_for_dm_skips_intro_prose_and_non_id_lines() {
        let body = "## Composition\n\
This is a paragraph with no id tokens at all.\n\
- not an id\n\
- UC-007 — valid pick\n\
\n\
just text\n\
\n\
## Coverage\n\
- UC-999 — outside the section, must be ignored\n";
        let ids = composition_for_dm(body);
        assert_eq!(ids, vec!["UC-007".to_string()]);
    }

    #[test]
    fn composition_for_dm_handles_lowercase_heading() {
        // Hand-edited bodies may write `## composition` — the parser is
        // case-insensitive on the heading to keep recovery cheap.
        let body = "## composition\n- INV-042 — example\n";
        let ids = composition_for_dm(body);
        assert_eq!(ids, vec!["INV-042".to_string()]);
    }

    #[test]
    fn composition_for_dm_ignores_non_brownfield_prefixes() {
        let body = "## Composition\n\
- PRD-001 — not a brownfield kind\n\
- UC-001 — brownfield\n\
- EVID-007 — not in scope\n";
        let ids = composition_for_dm(body);
        assert_eq!(ids, vec!["UC-001".to_string()]);
    }
}
