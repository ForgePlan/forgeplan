---
depth: standard
id: RFC-002
kind: rfc
links:
- target: EPIC-002
  relation: refines
- target: PRD-039
  relation: supersedes
status: superseded
title: Graph Intelligence — petgraph traversal, smart search, gap detection, tree UI
---

# RFC-002: Graph Intelligence — petgraph + embeddings + LLM

## Summary

Расширить KnowledgeGraph (petgraph, v0.11) для: ASCII tree view, smart search (graph+semantic), gap detection, и LLM-powered suggestions. Три столпа: Graph (структура), Embeddings (семантика), LLM (reasoning).

## Motivation

petgraph создан в v0.11 но используется только для from_store(). Граф содержит ценную информацию (60+ nodes, 50+ edges, R_eff/F-G-R weights) которая не эксплуатируется.

FPF B.1.1 определяет DependencyGraph как основу reasoning. Мы построили граф, но не используем его для reasoning.

## Proposed Direction

### Phase 1 (v0.12): ASCII Tree + petgraph everywhere

**`forgeplan tree [ID] [--depth N] [--json]`**
- Использует petgraph DFS для traversal
- ASCII render с цветами (console crate)
- --depth для больших проектов
- --json для AI agents

**petgraph replaces manual DB traversal:**
- r_eff_recursive → graph.evidence_for() + graph.neighbors()
- blocked/order → graph topological sort (вместо kahn_sort через DB)
- health → graph.impact_analysis()

### Phase 2 (v0.13): Smart Search + Gap Detection

**`forgeplan search "query" --smart`**
Combines 3 signals:
1. Keyword match (existing grep)
2. Graph neighbors (1-2 hops from keyword matches)
3. Semantic similarity (embeddings cosine > 0.8)

Returns unified results ranked by combined signal.

**`forgeplan gaps`**
Graph analysis:
- Orphan detection (nodes with 0 edges) — exists in health
- Missing pattern: PRD has RFC but no ADR
- Dependency without evidence: A→B but B has R_eff=0
- Unlinked semantic neighbors: embeddings similar but no graph edge

### Phase 3 (v0.14): LLM Suggestions

**`forgeplan suggest [ID]`**
Uses graph context + F-G-R + embeddings to generate:
- What to do next (based on DerivedStatus pipeline)
- What's missing (based on gap detection)
- What's related (based on semantic similarity)
- Priority ranking (based on R_eff weakest link)

### FPF Alignment

| FPF Pattern | How we use it |
|-------------|--------------|
| B.1.1 DependencyGraph | petgraph = our DependencyGraph |
| B.1.3 Γ_epist | Epic R_eff = aggregate of children |
| B.3 Trust Calculus | Node weights = R_eff + F-G-R |
| B.5.1 Lifecycle | DerivedStatus shows pipeline position |
| B.5.2 Abductive Loop | suggest generates hypotheses |
| C.19 Explore-Exploit | gaps shows explored vs unknown |

## Alternatives Considered

1. **ratatui TUI** — deferred to Phase 5/Tauri. ASCII tree sufficient for CLI.
2. **Neo4j/SurrealDB** — overkill for local-first. petgraph sufficient.
3. **Only embeddings** — misses structural relationships.
4. **Only graph** — misses semantic similarity.

## Implementation Phases

- [ ] Phase 1: `forgeplan tree` + petgraph replaces DB traversal (~200 LOC)
- [ ] Phase 2: Smart search + gap detection (~300 LOC)
- [ ] Phase 3: LLM suggest (~200 LOC)

## Dependencies

- petgraph 0.8 (v0.11, ✅ done)
- fastembed / BGE-M3 (v0.10, ✅ done, feature flag)
- LLM provider (v0.4, ✅ done)



