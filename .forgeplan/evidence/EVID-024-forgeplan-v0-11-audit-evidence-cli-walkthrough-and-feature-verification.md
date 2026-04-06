---
depth: standard
id: EVID-024
kind: evidence
links:
- target: PROB-012
  relation: informs
- target: NOTE-012
  relation: informs
status: active
title: Forgeplan v0.11 audit evidence — CLI walkthrough and feature verification
---

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: audit

## Evidence Summary

This evidence records a direct CLI walkthrough of Forgeplan on the ForgePlan workspace performed on 2026-03-25.

## Commands Executed

- `cargo test --workspace --quiet`
- `cargo build --workspace --all-features`
- `cargo run -q -p forgeplan-cli -- status`
- `cargo run -q -p forgeplan-cli -- health --json`
- `cargo run -q -p forgeplan-cli -- list --json`
- `cargo run -q -p forgeplan-cli -- order --json`
- `cargo run -q -p forgeplan-cli -- coverage`
- `cargo run -q -p forgeplan-cli -- validate --json`
- `cargo run -q -p forgeplan-cli -- fgr --json`
- `cargo run -q -p forgeplan-cli -- graph --json`
- `cargo run -q -p forgeplan-cli -- journal --risk`
- `cargo run -q -p forgeplan-cli -- drift --json`
- `cargo run -q -p forgeplan-cli -- fpf status`
- `cargo run -q -p forgeplan-cli -- route "...Linear sync...quality gates..."`
- `cargo run -q -p forgeplan-cli -- score PRD-016 --json`
- `cargo run -q -p forgeplan-cli -- context PRD-016 --json`
- `cargo run -q -p forgeplan-cli -- tree EPIC-001`
- `cargo run -q -p forgeplan-cli --features semantic-search -- search authentication --semantic --json`

## Verified Findings

### Working

- Workspace test suite passes.
- `--all-features` workspace build passes.
- Core artifact management commands work on the dogfood workspace.
- FPF knowledge base is ingested and up to date.
- Graph/order/blocked/drift surfaces produce meaningful output.

### Inconsistent

- `health --json` reports no `at_risk` artifacts.
- `journal --risk` reports 23 artifacts with `NO EVIDENCE`.
- `score/context` and `tree` disagree on `r_eff` display for the same artifacts.

### Broken

- Semantic search path does not compile when enabled from the CLI feature flag.

### Workflow gap

- `coverage` detects modules but remains `0%` useful on ForgePlan because decision artifacts are not sufficiently annotated with `Affected Files`.

## Interpretation

The product is operationally strong in artifact management but still has integrity gaps between claimed features, real behavior, and dogfood usefulness.

This evidence supports:

- PROB-012 Feature integrity gap — dogfood audit of Forgeplan v0.11
- NOTE-012 Forgeplan v0.11 feature audit — product, consistency, and workflow findings

