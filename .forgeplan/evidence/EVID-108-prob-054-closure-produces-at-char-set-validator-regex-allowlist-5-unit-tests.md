---
depth: standard
id: EVID-108
kind: evidence
links:
- target: PROB-054
  relation: informs
status: active
title: PROB-054 closure produces_at char-set validator regex allowlist 5 unit tests
---

# EVID-108: PROB-054 closure — `produces_at` prompt-injection validator

## Summary

Closes PR-E Round 6 audit LOW-1 — prompt-injection-via-filesystem (CWE-94 / OWASP A03) at `claude_print.rs::assemble_prompt`. Pre-PROB-054 the `Step.produces_at` path was spliced into the natural-language prompt body via `to_string_lossy()` без character validation; backtick (`reports/`backdoor`.md`) closed the markdown code-fence and turned tail of prompt into authoritative agent instructions. Symmetric rejection в `add_dir_for_produces_at` so prompt-body splice and `--add-dir` argv splice fail-fast on the same input.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Evidence

### Implementation

New `pub(super) fn validate_produces_at_chars(path: &Path) -> Result<(), String>`:
- Conservative allowlist regex `^[A-Za-z0-9._/-]+$`
- Allowed: alphanumeric, dot, underscore, forward slash, hyphen
- Reject: backtick, `$`, `;`, `\n`/`\r`, control chars, all other Unicode
- Renders rejection input via `to_string_lossy().escape_debug()` для defense-in-depth log injection (CWE-117 / CWE-150 — same pattern as PROB-053 shell-exec warning)

Wired into:
- `assemble_prompt()`: validates BEFORE splicing into prompt body. If invalid, omits the path entirely (sibling `add_dir_for_produces_at` will fail and abort dispatch).
- `add_dir_for_produces_at()`: validates after `..`-segment + absolute-path checks. Symmetric guard so the argv splice fails on the same input.

### Tests (+5 new unit tests)

```
test playbook::dispatch::claude_print::tests::validate_produces_at_chars_accepts_typical_path ... ok
test playbook::dispatch::claude_print::tests::validate_produces_at_chars_rejects_backtick ... ok
test playbook::dispatch::claude_print::tests::validate_produces_at_chars_rejects_dollar_sign ... ok
test playbook::dispatch::claude_print::tests::validate_produces_at_chars_rejects_semicolon ... ok
test playbook::dispatch::claude_print::tests::validate_produces_at_chars_rejects_newline ... ok
test playbook::dispatch::claude_print::tests::add_dir_for_produces_at_rejects_disallowed_chars ... ok
```

AC mandate was 3 unit tests; sprint shipped 6 (5 char-class tests + 1 symmetric add_dir_for_produces_at test).

### Quality gates

```
cargo fmt --check                                                  clean
cargo clippy --workspace --all-targets --features test-helpers
  -- -D warnings                                                   clean
cargo test --workspace --features test-helpers                     0 failures (38 suites)
```

Lib tests: 1469 → **1475** (+6 PROB-054 tests).

### AC tracking

- AC-1 ✅ `validate_produces_at_chars` helper added in claude_print.rs, regex `^[A-Za-z0-9._/-]+$`
- AC-2 ✅ `assemble_prompt` calls validator BEFORE splicing
- AC-3 ✅ `add_dir_for_produces_at` calls same validator (symmetric)
- AC-4 ✅ +6 unit tests (mandate was 3 — exceeded)
- AC-5 ✅ CHANGELOG entry under Security

## Hindsight

Tiny fix (XS effort), но class-of-problem is high-value: **prompt-injection-via-filesystem** is a different attack surface от argv injection (PROB-050 A-15 closure). Pre-PROB-054 the audit lens looked at argv hardening — это позволило prompt-body splice escape the review window. Lesson:

**When defending a templated string output (prompt body, log line, error message), every interpolated user-controlled value MUST be character-validated independently от the same value's argv/env validation.** The argv guard и the prompt-body guard are SEPARATE concerns even when they validate the same field.

## Related Artifacts

| Artifact | Relation |
|---|---|
| PROB-054 | informs (this evidence demonstrates closure) |
| PROB-050 | informs (A-15 argv-injection guard — orthogonal hardening on same field) |
| EVID-104 | informs (PROB-053 shell-exec escape_debug pattern — adopted here для error message) |
| ADR-011 | informs (claude --print dispatcher design — produces_at semantics defined here) |



