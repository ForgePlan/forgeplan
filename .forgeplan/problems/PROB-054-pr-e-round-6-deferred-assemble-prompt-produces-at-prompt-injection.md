---
depth: standard
id: PROB-054
kind: problem
last_modified_at: 2026-05-05T20:38:40.556809+00:00
last_modified_by: claude-code/2.1.128
links:
- target: PROB-050
  relation: based_on
status: draft
title: PR-E Round 6 deferred — assemble_prompt produces_at prompt-injection
---

## Signal

PR-E Round 6 adversarial security audit (3 parallel agents, 2026-05-05) flagged
`crates/forgeplan-core/src/playbook/dispatch/claude_print.rs:214-219`
(`assemble_prompt`) as a prompt-injection-via-filesystem surface:

```rust
// In assemble_prompt() the produces_at path is appended to the natural-
// language stdin prompt without character-set validation:
if let Some(path) = &step.produces_at {
    out.push_str("\n\nWrite output to `");
    out.push_str(&path.to_string_lossy());
    out.push_str("` using the Write tool.\n");
}
```

While `add_dir_for_produces_at` rejects `..` and absolute paths *for argv*,
the prompt string itself contains the raw `to_string_lossy()` value.
Backticks inside a workspace-relative path (`reports/`backdoor`.md`)
close the markdown code-fence and inject prompt content the agent treats
as instruction. This is **prompt-injection via filesystem** (not argv
injection — survived the A-15 audit which only checked argv).

Pre-existing surface (predates PR-E refactor — survived because the
audit lens looked at `--add-dir` argv hardening, not the prompt-body
template).

## Constraints

- MUST NOT break legitimate paths with normal characters (slashes,
  underscores, hyphens, dots).
- MUST NOT silently rewrite the path — operator must see the rejection
  reason.
- Allowed character set should be conservative — easier to expand later
  than to retract.

## Optimization Targets (1-3 max)

- **Validation regex**: `produces_at` MUST match
  `^[A-Za-z0-9._/-]+$` before it can be spliced into the prompt body.
- **Error path**: rejection produces `DispatchError::Transport` with
  the offending character highlighted, exactly: `produces_at contains
  unsupported character '<C>' at position <N>: <PATH>`.
- **Symmetry**: same regex applied at `add_dir_for_produces_at` time
  (already rejects `..`/absolute, this adds character-set check).

## Observation Indicators (Anti-Goodhart)

- Existing playbooks with `produces_at: reports/output.md` keep working.
- Playbooks with `produces_at` containing `` ` `` (backtick), `$`,
  `;`, `\n` etc. fail-fast at validation, not at agent runtime.
- +1 unit test per rejected character class.

## Acceptance Criteria

- [ ] `validate_produces_at_chars(p: &Path) -> Result<(), String>`
  helper added in `claude_print.rs`, checks regex
  `^[A-Za-z0-9._/-]+$` (with `to_string_lossy()` semantics for
  non-UTF-8 boundaries).
- [ ] `assemble_prompt` calls the validator BEFORE splicing; failure
  propagates as `DispatchError::Transport`.
- [ ] `add_dir_for_produces_at` calls the same validator (symmetric).
- [ ] +3 unit tests: backtick rejected, dollar-sign rejected, normal
  path accepted.
- [ ] CHANGELOG entry under **Security** section.

## Refs

- PR-E Round 6 audit (2026-05-05): security-expert agent LOW-1
- CHANGELOG.md (v0.29.0): "Deferred to v0.30.0" section
- PROB-050 A-15 (argv-injection guard — orthogonal hardening)

