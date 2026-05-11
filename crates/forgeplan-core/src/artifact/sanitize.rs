//! Shared sanitizer for agent-visible hint strings (PRD-071 / PROB-060).
//!
//! Hint strings — `Next:`/`Or:`/`Wait:` lines emitted by CLI and MCP — must
//! not contain bidi overrides, zero-width characters, control chars, or
//! shell metacharacters that change the meaning of the hint when an LLM
//! reads it back. This module is the single source of truth for that
//! cleanup; previously the logic lived only in `forgeplan-mcp::server` and
//! the CLI hint sites interpolated identity values verbatim (HIGH-3 audit
//! finding, CWE-117 / prompt injection).
//!
//! ## Why in `forgeplan-core`
//!
//! Both `forgeplan-cli` (decompose, reason, future commands) and
//! `forgeplan-mcp` need the same sanitizer. Putting it here lets both
//! crates depend on a single implementation without forgeplan-cli pulling
//! in forgeplan-mcp.
//!
//! ## Defence class
//!
//! Mirrors `AgentIdentity::is_identity_char_forbidden` (R2 audit MED on
//! identity propagation) but for *outgoing* hint text rather than ingoing
//! frontmatter. The two are aligned but not identical: hint sanitizer also
//! strips a small set of punctuation that affects hint syntax / agent
//! parsing (`` ` ``, `{`, `}`, `"`, `'`, `\`).

/// Length cap for sanitized strings — matches the historical
/// `forgeplan-mcp::server::sanitize_for_hint` budget. Truncation happens
/// AFTER filtering so hidden chars cannot consume budget.
const MAX_HINT_LEN: usize = 80;

/// Sanitize a string before interpolating it into an agent-visible hint.
///
/// Strategy: keep only printable ASCII + printable BMP characters. Strip
/// bidi overrides, zero-width characters, BOM, soft-hyphens, variation
/// selectors, format characters (U+2060..U+206F), tag characters
/// (U+E0000..U+E007F), and control chars. Truncate to [`MAX_HINT_LEN`]
/// chars AFTER filtering. Trim whitespace last.
///
/// Removes additional punctuation that affects hint syntax / agent
/// parsing or, more importantly, would survive in a shell context if an
/// agent ever copy-pastes a hint into a terminal:
///
/// * **Original set** (Round 1): `` ` { } " ' \ ``
///   — backtick / brace expansion / quote injection / escape.
/// * **Round 2 Sec FINDING-6 extension**: POSIX shell metacharacters
///   `;` (command separator), `$` (parameter expansion), `|` (pipe),
///   `&` (background / `&&`), `(` `)` (subshell / arithmetic),
///   `<` `>` (redirection), `!` (history expansion in interactive
///   shells), `#` (comment — hides trailing payload), `*` (glob).
///
/// **Note on `!` (Round 3 Code FINDING-6 — kept by design)**: `!`
/// rejected as defense-in-depth against bash history expansion in
/// interactive shells. The threat is low-impact (history expansion
/// only fires в interactive shells; agent-driven hint copy-paste
/// usually happens in a shell already running a script context with
/// `set +H`). Stripping is cheap, however, and the benefit of a
/// uniform reject set across both interactive and non-interactive
/// surfaces outweighs the cosmetic cost. The trade-off is explicit:
/// titles containing `!` lose the character (e.g. `"Auth! redesign"`
/// sanitizes to `"Auth redesign"`). If a future use case demands
/// `!`-preservation, audit the agent-paste path first — relaxing
/// в isolation re-opens the history-expansion hole on classic bash.
///
/// Concrete threat model from the audit: an attacker plants
/// `slug: "; rm -rf $HOME #"` in frontmatter; without this set, the
/// sanitized hint reads `Next: forgeplan get ; rm -rf $HOME #` — copy-
/// paste that into a shell and the rest of the line is a destructive
/// command. After this fix every shell-relevant byte is stripped, so
/// the surviving text is an obviously-broken identifier that can't
/// execute as anything.
///
/// **Idempotence**: applying twice yields the same result as once.
/// **No allocation surprises**: returns a `String`; callers usually want
/// to feed this into `format!()` or hint construction immediately.
pub fn sanitize_for_hint(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| {
            // Reject explicit invisible/dangerous ranges first (cheapest).
            if matches!(
                *c,
                // Zero-width
                '\u{200B}'..='\u{200F}'
                // LRE/RLE/PDF/LRO/RLO (bidi overrides)
                | '\u{202A}'..='\u{202E}'
                // WJ, FUNCTION APPLICATION, INVISIBLE SEPARATOR/TIMES/PLUS
                | '\u{2060}'..='\u{2064}'
                // Reserved
                | '\u{2065}'
                // LRI/RLI/FSI/PDI (bidi isolates)
                | '\u{2066}'..='\u{2069}'
                // Other format chars (interlinear annotations)
                | '\u{2028}'..='\u{202F}'
                // Soft-hyphen, Arabic letter mark, syriac abbreviation mark
                | '\u{00AD}' | '\u{061C}' | '\u{070F}'
                // Mongolian free/vowel separators
                | '\u{180B}'..='\u{180F}'
                // Variation selectors VS1..VS16
                | '\u{FE00}'..='\u{FE0F}'
                // Zero-width no-break space / BOM
                | '\u{FEFF}'
                // Variation selectors supplement VS17..VS256
                | '\u{E0100}'..='\u{E01EF}'
                // Tag characters (invisible annotation)
                | '\u{E0000}'..='\u{E007F}'
            ) {
                return false;
            }
            // Reject controls (incl. \r, \n, \t).
            if c.is_control() {
                return false;
            }
            // Reject punctuation that affects hint syntax / agent parsing
            // OR would behave as a shell metacharacter on copy-paste.
            // [Round 2 Sec FINDING-6] extended set — see fn-level docs.
            // [Round 3 Code FINDING-6] `!` kept as defense-in-depth for
            // bash history expansion (low-impact threat, cheap fix);
            // trade-off documented in the function-level docstring.
            !matches!(
                *c,
                '`' | '{'
                    | '}'
                    | '"'
                    | '\''
                    | '\\'
                    | ';'
                    | '$'
                    | '|'
                    | '&'
                    | '('
                    | ')'
                    | '<'
                    | '>'
                    | '!'
                    | '#'
                    | '*'
            )
        })
        .take(MAX_HINT_LEN)
        .collect();
    cleaned.trim().to_string()
}

/// Sanitize a filesystem path before interpolating it into an agent-visible
/// `Next:` hint.
///
/// Sibling of [`sanitize_for_hint`], but path-aware: legitimate path bytes
/// (`/`, `.`, `-`, `_`) are preserved so the surviving string can still be
/// re-fed to a CLI accepting a path argument. Every other non-alphanumeric
/// byte is stripped — that includes spaces (a space-containing path either
/// needs quoting which the hint protocol does not provide, or is a sign of
/// adversarial input), shell metacharacters, control bytes, ANSI escapes,
/// bidi overrides, zero-width characters, and tag characters.
///
/// Whitelist approach (vs. the denylist used by [`sanitize_for_hint`])
/// because path inputs come from CLI args (`--output <PATH>`) which an
/// attacker controls in full; we are willing to be more aggressive at the
/// cost of mangling exotic paths. The trade-off is conscious — adversarial
/// path like `'/tmp/foo;rm -rf .'` sanitizes to `/tmp/foorm-rf.`, which is
/// (a) clearly not the path the agent meant to operate on, and (b) cannot
/// execute as anything.
///
/// Truncated to [`MAX_HINT_LEN`] after filtering, matching the budget of
/// `sanitize_for_hint`. Paths longer than 80 visible chars are tail-clipped;
/// callers that need the literal full path should print it на the human-
/// facing line and only emit the sanitized form в the `Next:` hint.
///
/// **Threat model**: CWE-78 hint injection — agent reads `Next: forgeplan
/// import <path>` and executes the line verbatim. Without sanitization, a
/// crafted `--output` value embeds shell metacharacters in the hint and
/// extends the executed command into destructive territory (HIGH-2 audit
/// finding, sibling of HIGH-1 closed in `sanitize_for_hint`).
///
/// **Idempotence**: applying twice yields the same result as once.
pub fn sanitize_path_for_hint(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| {
            // Whitelist: ASCII alphanumerics + path-friendly punctuation.
            // Anything else (controls, shell metas, bidi, ANSI, spaces) is
            // dropped — see fn-level docs for the trade-off vs. denylist.
            c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_')
        })
        .take(MAX_HINT_LEN)
        .collect();
    cleaned.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_passes_clean_ascii_unchanged() {
        assert_eq!(sanitize_for_hint("PRD-074"), "PRD-074");
        assert_eq!(sanitize_for_hint("prd-auth-system"), "prd-auth-system");
    }

    #[test]
    fn sanitize_strips_bidi_override() {
        // RLO (Right-to-Left Override) attempts to flip the visual order
        // of subsequent characters — classic prompt-injection vector.
        let input = "prd-\u{202E}drawkcab";
        let out = sanitize_for_hint(input);
        assert!(!out.contains('\u{202E}'));
        assert_eq!(out, "prd-drawkcab");
    }

    #[test]
    fn sanitize_strips_zero_width_chars() {
        let input = "prd-\u{200B}auth\u{200C}\u{FEFF}-system";
        let out = sanitize_for_hint(input);
        assert_eq!(out, "prd-auth-system");
    }

    #[test]
    fn sanitize_strips_control_and_newline() {
        // [Round 2 Sec FINDING-6] `!` is now in the extended reject set
        // (history expansion in interactive shells), so unlike Round 1 the
        // trailing `!` is dropped along with the controls.
        let input = "prd\nauth\rsystem\t!";
        let out = sanitize_for_hint(input);
        assert_eq!(out, "prdauthsystem");
    }

    #[test]
    fn sanitize_strips_dangerous_punctuation() {
        // [Round 2 Sec FINDING-6] `$` is now in the rejected set (parameter
        // expansion), so the surviving text loses it too. The original
        // Round 1 expectation kept `$HOME` literal; post-fix every shell
        // metacharacter is gone and only the alphanumerics survive.
        let input = "rm`-rf'$HOME\"{bad}\\";
        let out = sanitize_for_hint(input);
        assert_eq!(out, "rm-rfHOMEbad");
    }

    /// [Round 2 Sec FINDING-6] Audit's stated threat: a slug-shaped payload
    /// like `"; rm -rf $HOME #"` planted in frontmatter must not survive
    /// sanitization as anything that could execute on copy-paste. Every
    /// shell-relevant byte (`;`, `$`, `#`) plus the quotes must be gone;
    /// only plain alphanumerics, dashes, and spaces remain (and the
    /// trailing trim drops bordering whitespace).
    #[test]
    fn sanitize_neutralises_shell_metachar_payload() {
        let input = "\"; rm -rf $HOME #\"";
        let out = sanitize_for_hint(input);
        // After filter:  ` rm -rf HOME ` then trim → "rm -rf HOME"
        // The leading `"` and `;` are gone, `$` is gone, `#` is gone, and
        // the trailing `"` is gone — what remains can't execute.
        assert!(!out.contains(';'), "; must be stripped: {out:?}");
        assert!(!out.contains('$'), "$ must be stripped: {out:?}");
        assert!(!out.contains('#'), "# must be stripped: {out:?}");
        assert!(!out.contains('"'), "\" must be stripped: {out:?}");
        assert!(!out.contains('\''), "' must be stripped: {out:?}");
        assert_eq!(out, "rm -rf HOME");
    }

    /// [Round 2 Sec FINDING-6] Cover the rest of the extended reject set —
    /// regression guard for the individual metacharacters we added so a
    /// future contributor can't accidentally drop one without a test
    /// failure.
    #[test]
    fn sanitize_strips_extended_shell_metas() {
        // Every extended-reject byte present once + a benign anchor.
        let input = "a;b|c&d(e)f<g>h!i*j";
        let out = sanitize_for_hint(input);
        // Each separator drops, leaving the alphabetic letters concatenated.
        assert_eq!(out, "abcdefghij");
        // Sanity-check: not even one of the rejected chars survives.
        for c in ";|&()<>!*".chars() {
            assert!(!out.contains(c), "byte {c:?} must be stripped: {out:?}");
        }
    }

    #[test]
    fn sanitize_truncates_to_80_chars() {
        let input = "a".repeat(120);
        let out = sanitize_for_hint(&input);
        assert_eq!(out.len(), 80);
        assert!(out.chars().all(|c| c == 'a'));
    }

    #[test]
    fn sanitize_truncates_after_filtering_invisible() {
        // 80 visible chars + 40 zero-width chars: post-filter must
        // produce 80 visible, not 80-of-mixed.
        let mut input = String::new();
        for _ in 0..40 {
            input.push('a');
            input.push('\u{200B}'); // zero-width — should not consume budget
        }
        for _ in 0..50 {
            input.push('b');
        }
        let out = sanitize_for_hint(&input);
        assert_eq!(out.len(), 80);
        // First 40 'a' then 40 'b' (capped at 80).
        assert_eq!(&out[..40], &"a".repeat(40));
        assert_eq!(&out[40..], &"b".repeat(40));
    }

    #[test]
    fn sanitize_idempotent() {
        let evil = "prd-\u{202E}\u{200B}drawkcab\nshell$(rm -rf /)";
        let once = sanitize_for_hint(evil);
        let twice = sanitize_for_hint(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn sanitize_trims_whitespace_after_filter() {
        let input = "   prd-auth-system   ";
        assert_eq!(sanitize_for_hint(input), "prd-auth-system");
    }

    #[test]
    fn sanitize_handles_empty_input() {
        assert_eq!(sanitize_for_hint(""), "");
        // All-invisible input becomes empty.
        assert_eq!(sanitize_for_hint("\u{200B}\u{FEFF}\u{202E}"), "");
    }

    #[test]
    fn sanitize_strips_tag_chars() {
        // Tag characters U+E0000..U+E007F are invisible annotations
        // sometimes used to smuggle hidden instructions to LLMs.
        let input = "prd\u{E0041}\u{E0042}auth";
        let out = sanitize_for_hint(input);
        assert_eq!(out, "prdauth");
    }

    // ---- sanitize_path_for_hint (w4 HIGH-2 / CWE-78 sibling of HIGH-1) ----

    /// Adversarial `--output` payload — every shell metacharacter the hint
    /// renderer must strip before emitting `Next: forgeplan import <path>`.
    /// Mirrors the threat model from the audit brief.
    #[test]
    fn sanitize_path_for_hint_strips_shell_metacharacters() {
        // Plain happy-path: a clean POSIX path round-trips unchanged.
        assert_eq!(
            sanitize_path_for_hint("/tmp/safe-file.json"),
            "/tmp/safe-file.json"
        );

        // Each of these bytes individually MUST be filtered.
        for c in ";`$|&()<>!#*\"'\\{}".chars() {
            let payload = format!("/tmp/foo{c}rm-rf");
            let cleaned = sanitize_path_for_hint(&payload);
            assert!(
                !cleaned.contains(c),
                "byte {c:?} must be stripped: {cleaned:?}"
            );
        }
    }

    /// The whole point of the path-aware variant: `/`, `.`, `-`, `_` and
    /// alphanumerics survive so the sanitized hint is still a usable path
    /// argument (when the input itself was benign).
    #[test]
    fn sanitize_path_for_hint_preserves_path_chars() {
        let safe = "/tmp/foo.bar/baz_qux-v2.json";
        assert_eq!(sanitize_path_for_hint(safe), safe);

        // Relative paths likewise round-trip.
        assert_eq!(
            sanitize_path_for_hint(".forgeplan/export.json"),
            ".forgeplan/export.json"
        );
    }

    /// Concrete audit threat: `'/tmp/foo;rm -rf .'` must NOT survive as a
    /// shell-executable payload. After sanitization, the surviving text is
    /// a broken-looking path that cannot branch into command execution.
    #[test]
    fn sanitize_path_for_hint_neutralises_destructive_payload() {
        let evil = "/tmp/foo;rm -rf .";
        let out = sanitize_path_for_hint(evil);
        // Separator and the space are gone; `rm` letters survive but cannot
        // execute because they are now part of a single contiguous "path"
        // argument to `forgeplan import`.
        assert!(!out.contains(';'), "; survived: {out:?}");
        assert!(!out.contains(' '), "space survived: {out:?}");
        assert_eq!(out, "/tmp/foorm-rf.");
    }

    /// Spaces are dropped (whitelist policy) — paths containing spaces are
    /// either adversarial or need quoting that the hint protocol doesn't
    /// provide. Documented trade-off, pinned by this test.
    #[test]
    fn sanitize_path_for_hint_drops_spaces_and_controls() {
        let input = "/tmp/with space\nand\tcontrol";
        let out = sanitize_path_for_hint(input);
        assert!(!out.contains(' '), "space survived: {out:?}");
        assert!(!out.contains('\n'), "newline survived: {out:?}");
        assert!(!out.contains('\t'), "tab survived: {out:?}");
        assert_eq!(out, "/tmp/withspaceandcontrol");
    }

    #[test]
    fn sanitize_path_for_hint_strips_bidi_and_zero_width() {
        // Same invisible-character coverage as sanitize_for_hint — the
        // whitelist policy implicitly rejects everything outside ASCII
        // alphanumerics + `/`, `.`, `-`, `_`.
        let input = "/tmp/\u{202E}drow/\u{200B}\u{FEFF}file.json";
        let out = sanitize_path_for_hint(input);
        assert!(!out.contains('\u{202E}'));
        assert!(!out.contains('\u{200B}'));
        assert!(!out.contains('\u{FEFF}'));
        assert_eq!(out, "/tmp/drow/file.json");
    }

    #[test]
    fn sanitize_path_for_hint_idempotent() {
        let evil = "/tmp/foo;rm -rf $HOME`whoami`";
        let once = sanitize_path_for_hint(evil);
        let twice = sanitize_path_for_hint(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn sanitize_path_for_hint_truncates_to_80_chars() {
        // 200 alphanumerics — all valid under the whitelist — must clip to 80.
        let input = "a".repeat(200);
        let out = sanitize_path_for_hint(&input);
        assert_eq!(out.len(), 80);
    }

    #[test]
    fn sanitize_path_for_hint_handles_empty_input() {
        assert_eq!(sanitize_path_for_hint(""), "");
        // All-invalid input collapses to empty.
        assert_eq!(sanitize_path_for_hint(";; |&"), "");
    }
}
