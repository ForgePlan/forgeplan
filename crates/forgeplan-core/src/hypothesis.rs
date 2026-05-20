//! Issue #287 Phase D — Hypothesis lifecycle state machine.
//!
//! A `Hypothesis` (kind `hypothesis`, slug prefix `hyp-`, tier `intent`)
//! carries an explicit `hypothesis_state:` frontmatter field that tracks
//! its position on the verification timeline. The state machine is linear
//! forward, with two non-forward sinks:
//!
//! ```text
//!   parked → inferred → strong-inferred → verified  (terminal)
//!     ↑          ↑              ↑              │
//!     │          │              │              │
//!     └──────────┴──────────────┴──────────────┴──→ refuted (terminal)
//!     └──────────┴──────────────┴──────────────┴──→ parked (hold, reversible)
//! ```
//!
//! # Invariants (the test matrix in this module pins each one)
//!
//! 1. **Forward chain is linear** — `parked → inferred → strong-inferred → verified`.
//!    No edges that skip a step (e.g. `parked → verified` is forbidden) and no
//!    reverse-forward edges (e.g. `verified → inferred` is forbidden — once a
//!    hypothesis is verified you don't downgrade it, you create a new HYP).
//! 2. **`any → refuted`** is always allowed (counter-evidence can land at any time).
//! 3. **`any → parked`** is always allowed (hold the work, reversible later via
//!    `parked → inferred`).
//! 4. **Terminal states reject all forward transitions** — `verified` and `refuted`
//!    are absorbing. From `verified` you may only go to `parked` (held for review)
//!    or `refuted` (someone disproved it). From `refuted` you may only go to
//!    `parked` (hold the dead artifact for archival). Neither may go back to
//!    `inferred`/`strong-inferred`/`verified`.
//! 5. **`same → same`** is allowed (no-op). Callers may rely on this for
//!    idempotent re-application; the MCP tool surfaces this as an
//!    "Unchanged"-like result by checking `old == new` before mutating.
//!
//! # Why this lives in core (not MCP)
//!
//! W2 (brownfield coverage) и W4 (graph extension) both need to inspect the
//! current state of a hypothesis without re-implementing the parser, и will
//! eventually color-code graph nodes by state. Putting the enum + transition
//! function + frontmatter reader in `forgeplan-core` lets every surface
//! (CLI, MCP, graph render, coverage report) share one source of truth.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::artifact::frontmatter::Frontmatter;

/// One position on the hypothesis verification timeline.
///
/// Serialised in YAML frontmatter as kebab-case (`strong-inferred`) so the
/// canonical wire shape matches the template literal pattern (the Phase A
/// template carries `hypothesis_state: inferred`). `schemars` is wired up
/// so MCP tool params that accept a state can render the enum into their
/// JSON Schema directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum HypothesisState {
    /// Work is on hold. Either we haven't started gathering evidence (initial
    /// draft) or we deliberately paused after one of the active states because
    /// signal was insufficient. Reversible — `parked → inferred` reopens.
    Parked,
    /// We've articulated the claim and have *some* signal pointing at it, but
    /// no corroborating evidence yet. The default landing state when a fresh
    /// hypothesis is created from the template.
    Inferred,
    /// One or more pieces of independent evidence support the claim, but the
    /// quality bar for `verified` hasn't been met (e.g. only one source, or
    /// indirect signal). Intermediate confidence step.
    StrongInferred,
    /// Terminal: sufficient evidence agrees. Downstream artifacts (UC, INV,
    /// SCEN that ref this HYP) can treat the claim as load-bearing.
    Verified,
    /// Terminal: counter-evidence proved the claim wrong. Downstream artifacts
    /// must be revisited — cascade enumeration lists which ones.
    Refuted,
}

impl HypothesisState {
    /// Canonical kebab-case wire form. Mirrors `serde(rename_all)` so the
    /// CLI / MCP / graph renderer all emit identical strings.
    pub fn as_str(&self) -> &'static str {
        match self {
            HypothesisState::Parked => "parked",
            HypothesisState::Inferred => "inferred",
            HypothesisState::StrongInferred => "strong-inferred",
            HypothesisState::Verified => "verified",
            HypothesisState::Refuted => "refuted",
        }
    }

    /// Case-insensitive parse from a free-form string. Accepts the canonical
    /// kebab-case forms emitted by `as_str()` plus a snake_case alias for
    /// `strong-inferred` (`strong_inferred`) so hand-edited frontmatter that
    /// drifts in casing or punctuation is still readable.
    ///
    /// Returns `None` on unknown / malformed input — callers default to
    /// `Parked` when the field is unparseable (see
    /// [`current_state_from_frontmatter`]).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "parked" => Some(HypothesisState::Parked),
            "inferred" => Some(HypothesisState::Inferred),
            "strong-inferred" | "strong_inferred" | "stronginferred" => {
                Some(HypothesisState::StrongInferred)
            }
            "verified" => Some(HypothesisState::Verified),
            "refuted" => Some(HypothesisState::Refuted),
            _ => None,
        }
    }

    /// Enumerate transitions that are currently allowed FROM this state.
    /// Used by the MCP tool to render a helpful "valid next states" list
    /// when an illegal transition is rejected.
    ///
    /// The list always contains the current state (no-op transition) plus
    /// the targets allowed by [`transition_allowed`].
    pub fn allowed_next(&self) -> Vec<HypothesisState> {
        use HypothesisState::*;
        let candidates = [Parked, Inferred, StrongInferred, Verified, Refuted];
        candidates
            .into_iter()
            .filter(|to| transition_allowed(*self, *to))
            .collect()
    }
}

/// Check whether `from → to` is a legal transition.
///
/// See the module docs for the full state machine. In short:
/// - `same → same` is allowed (no-op).
/// - `any → parked` is allowed (hold, reversible).
/// - `any → refuted` is allowed (counter-evidence).
/// - Forward chain: `parked → inferred → strong-inferred → verified`.
/// - Reverse-forward (e.g. `verified → inferred`) is forbidden.
/// - Skip-forward (e.g. `parked → verified`) is forbidden.
/// - `verified → strong-inferred / inferred` is forbidden (terminal).
/// - `refuted → inferred / strong-inferred / verified` is forbidden (terminal).
pub fn transition_allowed(from: HypothesisState, to: HypothesisState) -> bool {
    use HypothesisState::*;
    // No-op: same → same always allowed (idempotent re-application).
    if from == to {
        return true;
    }
    // Sinks: any → parked and any → refuted are always allowed.
    if matches!(to, Parked | Refuted) {
        return true;
    }
    // Forward chain, exactly one step.
    matches!(
        (from, to),
        (Parked, Inferred) | (Inferred, StrongInferred) | (StrongInferred, Verified)
    )
}

/// Read the current `hypothesis_state:` value from frontmatter. Returns
/// `Parked` (the safe default) when the field is missing, null, of a
/// non-string type, or fails to parse — the function never panics and
/// never returns `None`.
///
/// Why `Parked` is the safe default: it's the only state from which every
/// forward move is allowed. If we defaulted to e.g. `Inferred` and the
/// field was actually `strong-inferred` but corrupted, an agent's promote
/// call could land us in the wrong cell. `Parked` forces the caller to
/// take an explicit forward step, which is what we want when the
/// authoritative value is missing.
pub fn current_state_from_frontmatter(fm: &Frontmatter) -> HypothesisState {
    fm.get("hypothesis_state")
        .and_then(|v| v.as_str())
        .and_then(HypothesisState::parse)
        .unwrap_or(HypothesisState::Parked)
}

/// Mutate the body of a hypothesis artifact to reflect a promotion:
/// 1. Overwrite the `hypothesis_state:` field in the YAML frontmatter.
/// 2. Append a one-line audit entry to the `## Lifecycle State` body
///    section so the markdown stays self-describing (the frontmatter is
///    the canonical store; the body section is the human-readable trail).
///
/// The lifecycle entry shape is:
///
/// ```text
/// - YYYY-MM-DD: <old> → <new> (refs: EVID-001, EVID-002; rationale: <rationale>)
/// ```
///
/// When the body has no `## Lifecycle State` section (legacy artifact
/// created before the Phase A template existed) we append a fresh section
/// rather than fail — recovery beats hard error here.
///
/// Returns the full new body (frontmatter + content) ready to feed into
/// `projection::update_body_with_projection`.
///
/// # Errors
/// Returns an error if the input has no parseable YAML frontmatter — the
/// artifact is corrupted and the caller must surface that rather than
/// silently rendering a new frontmatter block.
pub fn apply_transition_to_body(
    body: &str,
    new_state: HypothesisState,
    today: &str,
    old_state: HypothesisState,
    evidence_refs: &[String],
    rationale: &str,
) -> anyhow::Result<String> {
    use crate::artifact::frontmatter::{parse_frontmatter, render_frontmatter};

    let (mut fm, content) = parse_frontmatter(body)
        .map_err(|e| anyhow::anyhow!("hypothesis body parse failed: {e}"))?;

    // 1. Frontmatter mutation: canonical kebab-case form.
    fm.insert(
        "hypothesis_state".to_string(),
        serde_yaml::Value::String(new_state.as_str().to_string()),
    );

    // Audit-r5 ARCH-C1: derive `status` from `hypothesis_state` so the
    // general lifecycle/scoring/health surfaces see a meaningful state.
    // Previously a Hypothesis carried two independent state machines
    // and `supports_lifecycle("hypothesis") == false` meant
    // `forgeplan_activate` / `forgeplan_review` skipped the kind —
    // leaving artifacts stuck on `status: draft` while their
    // `hypothesis_state` advanced to `verified`. Mapping:
    //   verified         → active   (load-bearing claim)
    //   refuted          → deprecated (counter-evidence terminal)
    //   parked           → draft    (held / not yet load-bearing)
    //   inferred         → draft    (some signal, not yet sufficient)
    //   strong-inferred  → draft    (intermediate, not yet sufficient)
    let derived_status = match new_state {
        HypothesisState::Verified => "active",
        HypothesisState::Refuted => "deprecated",
        HypothesisState::Parked | HypothesisState::Inferred | HypothesisState::StrongInferred => {
            "draft"
        }
    };
    fm.insert(
        "status".to_string(),
        serde_yaml::Value::String(derived_status.to_string()),
    );

    // 2. Body mutation: append lifecycle audit line.
    let evid_part = if evidence_refs.is_empty() {
        "none".to_string()
    } else {
        evidence_refs.join(", ")
    };
    let trimmed_rationale = rationale.trim();
    let rationale_part = if trimmed_rationale.is_empty() {
        "(no rationale provided)".to_string()
    } else {
        trimmed_rationale.to_string()
    };
    let entry = format!(
        "- {today}: {old} → {new} (refs: {evid_part}; rationale: {rationale_part})",
        old = old_state.as_str(),
        new = new_state.as_str(),
    );

    let new_content = append_lifecycle_entry(&content, &entry);

    render_frontmatter(&fm, &new_content)
        .map_err(|e| anyhow::anyhow!("hypothesis body re-render failed: {e}"))
}

/// Append `entry` (a single line) to the `## Lifecycle State` section of
/// `body`. If the section is absent, a fresh section is appended at the
/// end of the body. Idempotency is the caller's responsibility — we
/// always add a line, even if the new state matches the old (same-state
/// transitions are valid no-ops; the audit log records the attempt).
fn append_lifecycle_entry(body: &str, entry: &str) -> String {
    const HEADING: &str = "## Lifecycle State";

    // Find the heading line. We're tolerant of leading whitespace on the
    // heading (`  ## Lifecycle State`) by trimming the candidate line.
    let lines: Vec<&str> = body.lines().collect();
    let heading_idx = lines
        .iter()
        .position(|l| l.trim_start().starts_with(HEADING));

    match heading_idx {
        Some(idx) => {
            // Find the boundary of the section: the next line that begins
            // with `## ` (any other H2) or `# ` (H1), or end-of-body.
            let mut end_idx = lines.len();
            for (i, line) in lines.iter().enumerate().skip(idx + 1) {
                let t = line.trim_start();
                if t.starts_with("## ") || t.starts_with("# ") {
                    end_idx = i;
                    break;
                }
            }
            // Walk back through trailing blanks within the section so we
            // insert the audit line *before* the section's trailing blank
            // padding (keeps the markdown looking clean across repeated
            // promotes).
            let mut insert_at = end_idx;
            while insert_at > idx + 1 && lines[insert_at - 1].trim().is_empty() {
                insert_at -= 1;
            }
            let mut rebuilt: Vec<String> = lines[..insert_at]
                .iter()
                .map(|s| (*s).to_string())
                .collect();
            rebuilt.push(entry.to_string());
            for line in &lines[insert_at..] {
                rebuilt.push((*line).to_string());
            }
            // Preserve the original trailing newline (if any) so successive
            // promotes don't strip it.
            let mut out = rebuilt.join("\n");
            if body.ends_with('\n') && !out.ends_with('\n') {
                out.push('\n');
            }
            out
        }
        None => {
            // No section yet — append one at the end with the entry.
            let mut out = body.trim_end().to_string();
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str(HEADING);
            out.push_str("\n\n");
            out.push_str(entry);
            out.push('\n');
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;

    // ── parse / as_str round-trip ──────────────────────────────────────

    #[test]
    fn parse_accepts_canonical_kebab_case() {
        assert_eq!(
            HypothesisState::parse("parked"),
            Some(HypothesisState::Parked)
        );
        assert_eq!(
            HypothesisState::parse("inferred"),
            Some(HypothesisState::Inferred)
        );
        assert_eq!(
            HypothesisState::parse("strong-inferred"),
            Some(HypothesisState::StrongInferred)
        );
        assert_eq!(
            HypothesisState::parse("verified"),
            Some(HypothesisState::Verified)
        );
        assert_eq!(
            HypothesisState::parse("refuted"),
            Some(HypothesisState::Refuted)
        );
    }

    #[test]
    fn parse_is_case_insensitive_and_trims() {
        assert_eq!(
            HypothesisState::parse("  Verified "),
            Some(HypothesisState::Verified)
        );
        assert_eq!(
            HypothesisState::parse("STRONG-INFERRED"),
            Some(HypothesisState::StrongInferred)
        );
    }

    #[test]
    fn parse_accepts_snake_case_alias_for_strong_inferred() {
        // Hand-edited frontmatter may use snake_case by mistake — accept it.
        assert_eq!(
            HypothesisState::parse("strong_inferred"),
            Some(HypothesisState::StrongInferred)
        );
    }

    #[test]
    fn parse_rejects_unknown_values() {
        assert_eq!(HypothesisState::parse(""), None);
        assert_eq!(HypothesisState::parse("done"), None);
        assert_eq!(HypothesisState::parse("validated"), None);
    }

    #[test]
    fn as_str_round_trip() {
        use HypothesisState::*;
        for s in [Parked, Inferred, StrongInferred, Verified, Refuted] {
            assert_eq!(HypothesisState::parse(s.as_str()), Some(s));
        }
    }

    // ── State machine transition matrix (the heart of W3 contract) ─────
    //
    // The brief calls for 18 cases. We pin each one individually so a
    // future change to `transition_allowed` lights up exactly which
    // cell broke. Grouped by FROM state for readability.

    // FROM Parked (4 starting transitions — to itself + 4 others)
    #[test]
    fn from_parked_to_parked_allowed_noop() {
        assert!(transition_allowed(
            HypothesisState::Parked,
            HypothesisState::Parked
        ));
    }
    #[test]
    fn from_parked_to_inferred_allowed_forward_step() {
        assert!(transition_allowed(
            HypothesisState::Parked,
            HypothesisState::Inferred
        ));
    }
    #[test]
    fn from_parked_to_strong_inferred_rejected_skip_forward() {
        // Skipping `inferred` is forbidden — must accumulate signal in order.
        assert!(!transition_allowed(
            HypothesisState::Parked,
            HypothesisState::StrongInferred
        ));
    }
    #[test]
    fn from_parked_to_verified_rejected_skip_forward() {
        assert!(!transition_allowed(
            HypothesisState::Parked,
            HypothesisState::Verified
        ));
    }
    #[test]
    fn from_parked_to_refuted_allowed_sink() {
        // Even parked hypotheses can be refuted by counter-evidence.
        assert!(transition_allowed(
            HypothesisState::Parked,
            HypothesisState::Refuted
        ));
    }

    // FROM Inferred (5 transitions)
    #[test]
    fn from_inferred_to_parked_allowed_hold() {
        assert!(transition_allowed(
            HypothesisState::Inferred,
            HypothesisState::Parked
        ));
    }
    #[test]
    fn from_inferred_to_inferred_allowed_noop() {
        assert!(transition_allowed(
            HypothesisState::Inferred,
            HypothesisState::Inferred
        ));
    }
    #[test]
    fn from_inferred_to_strong_inferred_allowed_forward_step() {
        assert!(transition_allowed(
            HypothesisState::Inferred,
            HypothesisState::StrongInferred
        ));
    }
    #[test]
    fn from_inferred_to_verified_rejected_skip_forward() {
        // Must pass through strong-inferred first.
        assert!(!transition_allowed(
            HypothesisState::Inferred,
            HypothesisState::Verified
        ));
    }
    #[test]
    fn from_inferred_to_refuted_allowed_sink() {
        assert!(transition_allowed(
            HypothesisState::Inferred,
            HypothesisState::Refuted
        ));
    }

    // FROM StrongInferred (5 transitions)
    #[test]
    fn from_strong_inferred_to_parked_allowed_hold() {
        assert!(transition_allowed(
            HypothesisState::StrongInferred,
            HypothesisState::Parked
        ));
    }
    #[test]
    fn from_strong_inferred_to_inferred_rejected_reverse_forward() {
        // Reverse-forward forbidden — once you've gathered corroborating
        // evidence you don't downgrade, you either go forward (verified) or
        // refute (counter-evidence).
        assert!(!transition_allowed(
            HypothesisState::StrongInferred,
            HypothesisState::Inferred
        ));
    }
    #[test]
    fn from_strong_inferred_to_strong_inferred_allowed_noop() {
        assert!(transition_allowed(
            HypothesisState::StrongInferred,
            HypothesisState::StrongInferred
        ));
    }
    #[test]
    fn from_strong_inferred_to_verified_allowed_forward_step() {
        assert!(transition_allowed(
            HypothesisState::StrongInferred,
            HypothesisState::Verified
        ));
    }
    #[test]
    fn from_strong_inferred_to_refuted_allowed_sink() {
        assert!(transition_allowed(
            HypothesisState::StrongInferred,
            HypothesisState::Refuted
        ));
    }

    // FROM Verified (terminal forward — only parked + refuted allowed out)
    #[test]
    fn from_verified_to_parked_allowed_hold() {
        // Parked sink allowed even from terminal states.
        assert!(transition_allowed(
            HypothesisState::Verified,
            HypothesisState::Parked
        ));
    }
    #[test]
    fn from_verified_to_inferred_rejected_reverse_forward() {
        // Brief explicitly: `verified → inferred` is REJECTED.
        assert!(!transition_allowed(
            HypothesisState::Verified,
            HypothesisState::Inferred
        ));
    }
    #[test]
    fn from_verified_to_strong_inferred_rejected_reverse_forward() {
        assert!(!transition_allowed(
            HypothesisState::Verified,
            HypothesisState::StrongInferred
        ));
    }
    #[test]
    fn from_verified_to_verified_allowed_noop() {
        assert!(transition_allowed(
            HypothesisState::Verified,
            HypothesisState::Verified
        ));
    }
    #[test]
    fn from_verified_to_refuted_allowed_terminal_swap() {
        // Counter-evidence trumps a prior verification (rare but legal).
        assert!(transition_allowed(
            HypothesisState::Verified,
            HypothesisState::Refuted
        ));
    }

    // FROM Refuted (terminal — only parked allowed out)
    #[test]
    fn from_refuted_to_parked_allowed_hold() {
        assert!(transition_allowed(
            HypothesisState::Refuted,
            HypothesisState::Parked
        ));
    }
    #[test]
    fn from_refuted_to_inferred_rejected_terminal() {
        assert!(!transition_allowed(
            HypothesisState::Refuted,
            HypothesisState::Inferred
        ));
    }
    #[test]
    fn from_refuted_to_strong_inferred_rejected_terminal() {
        assert!(!transition_allowed(
            HypothesisState::Refuted,
            HypothesisState::StrongInferred
        ));
    }
    #[test]
    fn from_refuted_to_verified_rejected_terminal() {
        assert!(!transition_allowed(
            HypothesisState::Refuted,
            HypothesisState::Verified
        ));
    }
    #[test]
    fn from_refuted_to_refuted_allowed_noop() {
        assert!(transition_allowed(
            HypothesisState::Refuted,
            HypothesisState::Refuted
        ));
    }

    // ── allowed_next listing ───────────────────────────────────────────

    #[test]
    fn allowed_next_from_parked_lists_parked_inferred_refuted() {
        let next = HypothesisState::Parked.allowed_next();
        assert!(next.contains(&HypothesisState::Parked));
        assert!(next.contains(&HypothesisState::Inferred));
        assert!(next.contains(&HypothesisState::Refuted));
        assert!(!next.contains(&HypothesisState::StrongInferred));
        assert!(!next.contains(&HypothesisState::Verified));
    }

    #[test]
    fn allowed_next_from_verified_drops_forward_states() {
        let next = HypothesisState::Verified.allowed_next();
        assert!(next.contains(&HypothesisState::Verified)); // no-op
        assert!(next.contains(&HypothesisState::Parked));
        assert!(next.contains(&HypothesisState::Refuted));
        assert!(!next.contains(&HypothesisState::Inferred));
        assert!(!next.contains(&HypothesisState::StrongInferred));
    }

    // ── current_state_from_frontmatter ─────────────────────────────────

    #[test]
    fn current_state_default_parked_when_field_missing() {
        let fm = Frontmatter::new();
        assert_eq!(current_state_from_frontmatter(&fm), HypothesisState::Parked);
    }

    #[test]
    fn current_state_default_parked_when_field_null() {
        let mut fm = Frontmatter::new();
        fm.insert("hypothesis_state".to_string(), Value::Null);
        assert_eq!(current_state_from_frontmatter(&fm), HypothesisState::Parked);
    }

    #[test]
    fn current_state_default_parked_when_field_unknown_string() {
        let mut fm = Frontmatter::new();
        fm.insert(
            "hypothesis_state".to_string(),
            Value::String("done".to_string()),
        );
        assert_eq!(current_state_from_frontmatter(&fm), HypothesisState::Parked);
    }

    #[test]
    fn current_state_reads_inferred() {
        let mut fm = Frontmatter::new();
        fm.insert(
            "hypothesis_state".to_string(),
            Value::String("inferred".to_string()),
        );
        assert_eq!(
            current_state_from_frontmatter(&fm),
            HypothesisState::Inferred
        );
    }

    #[test]
    fn current_state_reads_strong_inferred() {
        let mut fm = Frontmatter::new();
        fm.insert(
            "hypothesis_state".to_string(),
            Value::String("strong-inferred".to_string()),
        );
        assert_eq!(
            current_state_from_frontmatter(&fm),
            HypothesisState::StrongInferred
        );
    }

    #[test]
    fn current_state_default_parked_when_field_non_string() {
        let mut fm = Frontmatter::new();
        fm.insert(
            "hypothesis_state".to_string(),
            Value::Number(serde_yaml::Number::from(1)),
        );
        assert_eq!(current_state_from_frontmatter(&fm), HypothesisState::Parked);
    }

    // ── apply_transition_to_body ───────────────────────────────────────

    fn sample_body() -> String {
        "---\n\
id: HYP-001\n\
title: \"Sample\"\n\
kind: hypothesis\n\
tier: intent\n\
hypothesis_state: inferred\n\
---\n\
\n\
# HYP-001: Sample\n\
\n\
## Hypothesis\n\
A claim.\n\
\n\
## Lifecycle State\n\
**Current**: `inferred`\n\
\n\
## Source\n\
Initial inference.\n"
            .to_string()
    }

    #[test]
    fn apply_transition_updates_frontmatter_field() {
        let new_body = apply_transition_to_body(
            &sample_body(),
            HypothesisState::StrongInferred,
            "2026-05-20",
            HypothesisState::Inferred,
            &["EVID-001".to_string()],
            "two independent sources concur",
        )
        .unwrap();
        assert!(
            new_body.contains("hypothesis_state: strong-inferred"),
            "frontmatter must reflect new state: {new_body}"
        );
        assert!(
            !new_body.contains("hypothesis_state: inferred\n"),
            "old state must be replaced (not duplicated): {new_body}"
        );
    }

    #[test]
    fn apply_transition_appends_audit_line_into_lifecycle_section() {
        let new_body = apply_transition_to_body(
            &sample_body(),
            HypothesisState::StrongInferred,
            "2026-05-20",
            HypothesisState::Inferred,
            &["EVID-001".to_string(), "EVID-002".to_string()],
            "two independent sources",
        )
        .unwrap();
        assert!(
            new_body.contains(
                "- 2026-05-20: inferred → strong-inferred (refs: EVID-001, EVID-002; rationale: two independent sources)"
            ),
            "audit line must land in body: {new_body}"
        );
        // The new line lives BETWEEN `## Lifecycle State` and `## Source`.
        let lifecycle_pos = new_body.find("## Lifecycle State").unwrap();
        let audit_pos = new_body.find("- 2026-05-20:").unwrap();
        let source_pos = new_body.find("## Source").unwrap();
        assert!(
            lifecycle_pos < audit_pos && audit_pos < source_pos,
            "audit line must be inside the Lifecycle State section: ls={lifecycle_pos} audit={audit_pos} src={source_pos}"
        );
    }

    #[test]
    fn apply_transition_handles_missing_lifecycle_section() {
        // Legacy artifact without a `## Lifecycle State` section. We must
        // recover by appending a fresh section, not error out.
        let legacy = "---\n\
id: HYP-009\n\
kind: hypothesis\n\
tier: intent\n\
hypothesis_state: inferred\n\
---\n\
\n\
# HYP-009: legacy\n\
\n\
## Hypothesis\n\
something.\n";
        let new_body = apply_transition_to_body(
            legacy,
            HypothesisState::Refuted,
            "2026-05-20",
            HypothesisState::Inferred,
            &[],
            "counter-example landed",
        )
        .unwrap();
        assert!(
            new_body.contains("## Lifecycle State"),
            "missing section must be appended: {new_body}"
        );
        assert!(
            new_body.contains(
                "- 2026-05-20: inferred → refuted (refs: none; rationale: counter-example landed)"
            ),
            "audit line with empty refs renders `none`: {new_body}"
        );
    }

    #[test]
    fn apply_transition_handles_empty_rationale() {
        let new_body = apply_transition_to_body(
            &sample_body(),
            HypothesisState::Parked,
            "2026-05-20",
            HypothesisState::Inferred,
            &[],
            "   ",
        )
        .unwrap();
        assert!(
            new_body.contains("rationale: (no rationale provided)"),
            "empty / whitespace rationale falls back to placeholder: {new_body}"
        );
    }

    #[test]
    fn apply_transition_round_trips_through_parse() {
        use crate::artifact::frontmatter::parse_frontmatter;
        let new_body = apply_transition_to_body(
            &sample_body(),
            HypothesisState::Verified,
            "2026-05-20",
            HypothesisState::StrongInferred,
            &["EVID-007".to_string()],
            "sufficient confirmation",
        )
        .unwrap();
        let (fm, body) = parse_frontmatter(&new_body).unwrap();
        assert_eq!(
            current_state_from_frontmatter(&fm),
            HypothesisState::Verified
        );
        assert!(body.contains("- 2026-05-20: strong-inferred → verified"));
    }

    /// Audit-r4 TEST-G6 closure: full Cartesian product of the 5-state
    /// transition matrix (25 cells) pinned against a reference table.
    /// Without this meta-test, adding a 6th state could pass the existing
    /// per-cell tests while leaving new transitions unverified.
    #[test]
    fn transition_matrix_full_cartesian() {
        use HypothesisState::*;
        let states = [Parked, Inferred, StrongInferred, Verified, Refuted];

        // Reference table: true iff transition_allowed(from, to) MUST be true.
        // Encoded as a 5x5 grid; rows = from, cols = to.
        // Order: Parked, Inferred, StrongInferred, Verified, Refuted.
        // Rules (from module doc):
        //   - same → same always allowed
        //   - any → parked allowed
        //   - any → refuted allowed
        //   - forward chain: Parked → Inferred → StrongInferred → Verified
        //   - everything else forbidden
        let expected: [[bool; 5]; 5] = [
            //  P    I    SI    V    R
            [true, true, false, false, true],  // from Parked
            [true, true, true, false, true],   // from Inferred
            [true, false, true, true, true],   // from StrongInferred
            [true, false, false, true, true],  // from Verified (terminal forward)
            [true, false, false, false, true], // from Refuted (terminal forward)
        ];

        for (i, &from) in states.iter().enumerate() {
            for (j, &to) in states.iter().enumerate() {
                let actual = transition_allowed(from, to);
                assert_eq!(
                    actual, expected[i][j],
                    "transition_allowed({:?}, {:?}) returned {} — reference says {}",
                    from, to, actual, expected[i][j]
                );
            }
        }
    }

    // ─── Audit-r5 ARCH-C1: derived status field ────────────────────────────

    /// Verified hypothesis must surface as `status: active` so the
    /// general scoring/health/lifecycle pipeline sees a load-bearing
    /// artifact. Previously the artifact stayed `status: draft`
    /// forever because `forgeplan_activate` skipped Hypothesis.
    #[test]
    fn apply_transition_derives_status_active_on_verified() {
        use crate::artifact::frontmatter::parse_frontmatter;
        let new_body = apply_transition_to_body(
            &sample_body(),
            HypothesisState::Verified,
            "2026-05-20",
            HypothesisState::StrongInferred,
            &["EVID-007".to_string()],
            "sufficient confirmation",
        )
        .unwrap();
        let (fm, _) = parse_frontmatter(&new_body).unwrap();
        assert_eq!(
            fm.get("status").and_then(|v| v.as_str()),
            Some("active"),
            "Verified hypothesis must derive status=active"
        );
    }

    /// Refuted hypothesis must surface as `status: deprecated` so
    /// downstream consumers don't treat the claim as load-bearing.
    #[test]
    fn apply_transition_derives_status_deprecated_on_refuted() {
        use crate::artifact::frontmatter::parse_frontmatter;
        let new_body = apply_transition_to_body(
            &sample_body(),
            HypothesisState::Refuted,
            "2026-05-20",
            HypothesisState::Inferred,
            &["EVID-counter".to_string()],
            "evidence against",
        )
        .unwrap();
        let (fm, _) = parse_frontmatter(&new_body).unwrap();
        assert_eq!(
            fm.get("status").and_then(|v| v.as_str()),
            Some("deprecated"),
            "Refuted hypothesis must derive status=deprecated"
        );
    }

    /// Intermediate states (parked / inferred / strong-inferred) all
    /// derive `status: draft` so they don't appear as load-bearing
    /// until verification lands.
    #[test]
    fn apply_transition_derives_status_draft_on_intermediate_states() {
        use crate::artifact::frontmatter::parse_frontmatter;
        for state in [
            HypothesisState::Parked,
            HypothesisState::Inferred,
            HypothesisState::StrongInferred,
        ] {
            let new_body = apply_transition_to_body(
                &sample_body(),
                state,
                "2026-05-20",
                HypothesisState::Inferred,
                &[],
                "intermediate move",
            )
            .unwrap();
            let (fm, _) = parse_frontmatter(&new_body).unwrap();
            assert_eq!(
                fm.get("status").and_then(|v| v.as_str()),
                Some("draft"),
                "{state:?} must derive status=draft"
            );
        }
    }
}
