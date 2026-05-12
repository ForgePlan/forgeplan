//! Shared validators for artifact identity inputs.
//!
//! Lifted from `forgeplan-cli::commands::new::validate_title` so that every
//! mutation surface (CLI `new` and `update`, MCP `forgeplan_new` and
//! `forgeplan_update`) can route through the SAME defensive check.
//!
//! Pre-Wave 9, only `forgeplan new` invoked `validate_title`. The other three
//! surfaces accepted any byte sequence below a coarse `mcp_max_title_len`
//! byte cap (or no cap at all, в CLI `update`), letting Trojan-Source bidi
//! overrides, ANSI escape sequences, and embedded newlines reach LanceDB
//! and rendered frontmatter. Audit SEC-C1 (CLI `update`) and SEC-C2 (MCP
//! `forgeplan_new` / `forgeplan_update`).
//!
//! ## Threat model
//!
//! Titles flow into:
//! - markdown frontmatter (`title: ...`) — embedded newline corrupts YAML
//! - heading lines (`# PRD-074: <title>`) — control chars hijack terminal
//! - filesystem path (`prds/PRD-074-<slug>.md`) — overlong → ENAMETOOLONG
//! - MCP JSON responses passed to LLM agents — bidi overrides spoof `Next:`
//!   hint command suggestions (Trojan Source — CWE-1007)
//!
//! ## Scope
//!
//! `validate_title` is a hot path — keep это синхронным, no I/O, no
//! allocations beyond the error message. Called BEFORE any DB or
//! filesystem write so invalid input never produces orphan rows.

use anyhow::Result;

/// Maximum allowed title length in characters (Unicode scalars, not bytes).
///
/// Chosen as a safe upper bound for filesystem path limits across platforms
/// (macOS/Linux filenames cap at 255 bytes; we leave headroom for slug
/// prefix/suffix, extension, and multi-byte characters). Mirrors the
/// historical `forgeplan-cli::commands::new::MAX_TITLE_LEN` constant — kept
/// in sync via re-export so the two crates cannot drift apart.
pub const MAX_TITLE_LEN: usize = 128;

/// Validate an artifact title before any DB or filesystem writes.
///
/// Rejects:
/// - Empty / whitespace-only titles
/// - Titles longer than [`MAX_TITLE_LEN`] characters
/// - **Control characters** (CWE-176) — would corrupt rendered headings
///   and MCP responses passed to LLM agents. Includes `\n`, `\r`, `\t`,
///   ANSI escapes (ESC = U+001B), and the full C0/C1 ranges.
/// - **BIDI override codepoints** (CWE-1007 / Trojan Source) —
///   `U+202A..U+202E` and `U+2066..U+2069` can spoof displayed `Next:`
///   commands suggested back to AI agents.
///
/// Called at the very start of mutation handlers so that invalid input
/// never produces orphan DB rows or partially-rendered files. Per
/// cross-phase security audit L3 (CLI new) and Wave 9 SEC-C1+SEC-C2 (CLI
/// update + MCP new/update).
pub fn validate_title(title: &str) -> Result<()> {
    if title.trim().is_empty() {
        anyhow::bail!("Title cannot be empty. Provide a non-empty title.");
    }
    let len = title.chars().count();
    if len > MAX_TITLE_LEN {
        anyhow::bail!(
            "Title too long (got {} chars, max {}). Shorten the title.",
            len,
            MAX_TITLE_LEN
        );
    }
    // Reject control chars and BIDI overrides. Titles are single-line user
    // input; embedded newlines/tabs break frontmatter rendering and CLI
    // output. ESC (U+001B) is treated as a control character by
    // `char::is_control`, so ANSI sequences are covered here too.
    for c in title.chars() {
        if c.is_control() {
            anyhow::bail!(
                "Title contains control character (U+{:04X}). \
                 Use plain printable text only.",
                c as u32
            );
        }
        // BIDI override / isolate codepoints (Trojan Source class).
        if matches!(c as u32, 0x202A..=0x202E | 0x2066..=0x2069) {
            anyhow::bail!(
                "Title contains BIDI override character (U+{:04X}). \
                 These can spoof rendered output — rejected for security.",
                c as u32
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_ascii_title() {
        assert!(validate_title("Auth System Redesign").is_ok());
        assert!(validate_title("PRD-074").is_ok());
        // Mixed punctuation that is NOT control/bidi is allowed.
        assert!(validate_title("Auth: redesign (phase 2) — round 1").is_ok());
    }

    #[test]
    fn accepts_unicode_titles() {
        // Cyrillic / CJK / emoji are not controls, not bidi overrides.
        assert!(validate_title("Реструктуризация авторизации").is_ok());
        assert!(validate_title("認証システムの再設計").is_ok());
        assert!(validate_title("Auth redesign 🔐").is_ok());
    }

    #[test]
    fn rejects_empty_and_whitespace_only() {
        assert!(validate_title("").is_err());
        assert!(validate_title("   ").is_err());
        assert!(validate_title("\t\t").is_err());
    }

    #[test]
    fn rejects_control_chars_newline_tab_esc() {
        // Embedded newline — corrupts frontmatter YAML.
        let err = validate_title("Auth\nredesign").unwrap_err().to_string();
        assert!(err.contains("control character"), "got: {err}");
        // Carriage return — same corruption class.
        assert!(validate_title("Auth\rredesign").is_err());
        // Tab — single-line input only.
        assert!(validate_title("Auth\tredesign").is_err());
        // ESC (U+001B) — ANSI escape sequence prefix; would hijack terminal
        // output on `forgeplan list` / `forgeplan get` rendering.
        let ansi_err = validate_title("Auth\u{001B}[31mredesign")
            .unwrap_err()
            .to_string();
        assert!(
            ansi_err.contains("U+001B"),
            "ANSI ESC must be rejected by control-char check: {ansi_err}"
        );
    }

    #[test]
    fn rejects_bidi_override_u202e() {
        // Right-to-Left Override — classic Trojan Source vector.
        let err = validate_title("redacti\u{202E}fdp.txt")
            .unwrap_err()
            .to_string();
        assert!(err.contains("BIDI override"), "got: {err}");
        assert!(err.contains("U+202E"));
    }

    #[test]
    fn rejects_bidi_isolate_range() {
        // LRI / RLI / FSI / PDI — full isolate range.
        for cp in 0x2066u32..=0x2069 {
            let c = char::from_u32(cp).unwrap();
            let title: String = format!("Auth{c}redesign");
            let err = validate_title(&title).unwrap_err().to_string();
            assert!(
                err.contains("BIDI override"),
                "U+{cp:04X} must be rejected: {err}"
            );
        }
        // LRE / RLE / PDF / LRO / RLO — full override range.
        for cp in 0x202Au32..=0x202E {
            let c = char::from_u32(cp).unwrap();
            let title: String = format!("Auth{c}redesign");
            assert!(
                validate_title(&title).is_err(),
                "U+{cp:04X} (bidi override) must be rejected"
            );
        }
    }

    #[test]
    fn rejects_oversize_title() {
        let too_long: String = "a".repeat(MAX_TITLE_LEN + 1);
        let err = validate_title(&too_long).unwrap_err().to_string();
        assert!(err.contains("too long"), "got: {err}");
        assert!(err.contains(&MAX_TITLE_LEN.to_string()));
    }

    #[test]
    fn accepts_exactly_max_length() {
        let max_title: String = "a".repeat(MAX_TITLE_LEN);
        assert!(validate_title(&max_title).is_ok());
    }
}
