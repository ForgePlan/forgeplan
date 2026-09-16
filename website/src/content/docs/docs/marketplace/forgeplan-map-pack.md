---
title: forgeplan-map-pack
description: "Builds a navigable map of a project - zones, nodes and verified edges - as a `map.json`, through an eight-agent pipeline (scan -> extract -> verify -> emit -> validate) where every stage runs in a fresh isolated context and a deterministic guardian script, not an LLM, is the sole authority on whether the result can be trusted."
---

Builds a navigable map of a project - zones, nodes and verified edges - as a `map.json`, through an eight-agent pipeline (scan -> extract -> verify -> emit -> validate) where every stage runs in a fresh isolated context and a deterministic guardian script, not an LLM, is the sole authority on whether the result can be trusted. Use when you need a structural view of a large or unfamiliar repository that a file tree cannot give, or when a project's forgeplan artifact graph should be shown alongside its code. Not a documentation generator: an edge is either sourced from the artifact graph or proven by a grep, and an unproven one is dropped rather than shipped as noise. Full history in CHANGELOG.md.

**Cost**: medium | **Stability**: beta | **Version**: 0.24.0 | **Category**: workflow

**Targets**: claude-code

**Requires**: CLI: forgeplan >=0.25.0

## Commands

| Command | What it does |
|---|---|
| `/map-build` | Scan the current forgeplan-enabled repo and generate a validated .forgeplan/map/map.json (schema forgeplan.map/v1) for forgeplan-web's composed-map view, by dispatching the forgeplan-map-pack 8-agent pipeline. |
| `/map-build-layer` | Generate ONE zone's drill-down layer (a validated sub-map with its own zones/nodes/edges/flows) for an already-built .forgeplan/map/map.json, on demand and append-only, by dispatching the forgeplan-map-pack pipeline in SCOPED mode. |
| `/map-doctor` | Fast, read-only health check for an already-built .forgeplan/map/map.json + its layers - runs the deterministic guardian on every file, flags stale layers (content changed by git/docs), and finds orphan layers, WITHOUT regenerating anything. |
| `/map-refresh` | Incrementally refresh an already-built .forgeplan/map/map.json - find what changed by git + docs since the map was built and rebuild ONLY the affected layers, leaving fresh layers untouched. |

## Agents

| Agent | What it does |
|---|---|
| `code-scanner` | SCAN-stage EMITTER agent for the forgeplan-map-pack pipeline: scans the target project's source tree, package manifests, and entry points into raw facts (modules, entry points, manifest-declared dependencies) for the downstream `zone-extractor` to bin into zones and mint into content-hash nodes. |
| `docs-scanner` | SCAN-stage EMITTER agent for the forgeplan-map-pack pipeline: scans README/docs prose into raw narration facts for the downstream `zone-extractor`'s `description_ru` attachment step, extracting RU narration from REAL prose ONLY -- never invented; a zone/node with no docs source gets no narration field at all, omitted entirely (SPEC-003 SS D5 language rule, SS E3 dropped-not-faked rule, MASTER-SPEC SS15/SS22). |
| `edge-verifier` | EMITTER-profile agent for the VERIFY stage of forgeplan-map-pack. |
| `forgeplan-scanner` | SCAN-stage EMITTER agent for the forgeplan-map-pack pipeline: scans the `.forgeplan/` artifact graph via READ-ONLY MCP (`forgeplan_graph`, `forgeplan_list`, `forgeplan_get` -- no mutator ever called) into raw artifact/edge facts for the downstream `zone-extractor` to bin and mint. |
| `map-emitter` | EMITTER-profile agent for the EMIT stage of forgeplan-map-pack. |
| `map-guardian` | The structural + advisory-semantic gate for a generated `map.json`. |
| `map-orchestrator` | Master orchestrator for the forgeplan-map-pack `map.json` generation pipeline. |
| `zone-extractor` | THE HEART of the forgeplan-map-pack pipeline (RFC-023's own words). |

## Skills

- `edge-verifier` - Edge trust-classification and grep-verification algorithm for forgeplan-map-pack's VERIFY stage (RFC-023 Proposed Direction SS1 roster row 6; PRD-075 FR-4; SPEC-003 SS D2 the 11 VALID_RELATIONS, SS D3 the typed-link/code-dep namespace default rule).
- `map-emitter` - Document-assembly, pre-write invariant-guard, and atomic-write algorithm for forgeplan-map-pack's EMIT stage (RFC-023 Proposed Direction SS1 roster row 7 + Invariant #1; PRD-075 FR-5; SPEC-003 SS C0 the one-sentence contract, SS C3 gate G4).
- `zone-extractor` - Zone/node/mega-node extraction algorithm for forgeplan-map-pack's EXTRACT stage (RFC-023 Proposed Direction SS1 roster row 5, "THE HEART"; PRD-075 FR-3; SPEC-003 SS C1 INV-2, SS D4).

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install forgeplan-map-pack@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
