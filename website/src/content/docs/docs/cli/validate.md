---
title: forgeplan validate
description: "Check an artifact’s structure against the MUST/SHOULD rules for its kind and depth. MUST failures return exit 1 in ordinary mode as well as CI mode; SHOULD findings alone are warnings. This is a structural check, not a verification of the decision or implementation."
---

Check an artifact’s structure against the MUST/SHOULD rules for its kind and depth. MUST failures return exit 1 in ordinary mode as well as CI mode; SHOULD findings alone are warnings. This is a structural check, not a verification of the decision or implementation.

## Usage

```text
forgeplan validate [OPTIONS] [ID]
```

## Arguments

```text
  [ID]  Artifact ID (validates all if omitted)
```

## Options

```text
      --json         Output as JSON for machine consumption
      --adversarial  Run adversarial (devil's advocate) review
      --ci           CI mode: exit code 1 if any MUST rules fail
  -h, --help         Print help
  -V, --version      Print version
```

## Selection and CI behavior

Without an ID, ordinary `validate` examines all stored artifacts, while `validate --ci` selects only `active` and `stale`. Other statuses, including drafts, are skipped completely in that no-ID CI path: they are not printed as draft warnings. `validate ID --ci` validates the specified artifact regardless of its status, so a draft’s MUST errors still fail.

## Examples

```bash
forgeplan validate PRD-001
forgeplan validate --ci
forgeplan validate PRD-001 --ci
forgeplan validate ADR-005 --adversarial
# Ordinary JSON output, without the successful-CI trailer:
forgeplan validate PRD-001 --json | jq '.results[].errors'
```

## JSON and exit status

The normal JSON envelope is `results` plus `_next_action`. A result contains `artifact_id`, `kind`, `depth`, `passed`, integer `errors` and `warnings`, and `findings` entries with `rule_id`, `severity`, `message`, and `section`. There is no `must_violations` field. A successful `--json --ci` run appends a `CI PASSED …` text line to stdout, so that combination is not one pure JSON document. An empty workspace prints `No artifacts found.` even with `--json`. Preserve the producer’s exit status when piping to jq; a successful jq process is not evidence that validation passed.

## Adversarial checks and side effects

`--adversarial` adds deterministic body checks from the built-in adversarial module; it does not call an LLM or require a stronger model when no findings are returned. Successful validation of a specific ID can advance the methodology session to coding. It does not activate the artifact. Activation additionally checks length, evidence links, and stub content unless explicitly forced.

<a id="ci-mode---ci"></a>

## CI integration

```yaml
- name: Forgeplan validate gate
  run: |
    forgeplan reindex
    forgeplan validate --ci
```

This excludes draft work by design. Validate an explicit draft ID separately when that is the intended policy. Pair with `health --ci` for its documented debt thresholds, not as a guarantee that every stale or at-risk artifact fails health by default.

## See also

- [`forgeplan review`](/docs/cli/review/) - adds lifecycle checklist on top of validation
- [`forgeplan activate`](/docs/cli/activate/) - the gate `validate` feeds
- [`forgeplan score`](/docs/cli/score/) - quality beyond structural rules
- [Quality Gates](/docs/methodology/evidence/) - MUST/SHOULD/adversarial philosophy
- [CLI overview](/docs/cli/)
