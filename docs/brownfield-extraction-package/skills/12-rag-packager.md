# Skill: rag-packager (C12)

> Packages verified canonical documentation for ingestion into a RAG (Retrieval-Augmented Generation) system. Produces chunks, metadata, and retrieval-friendly structure.

## Why this skill exists

Canonical docs (from C10) are human-readable markdown. RAG systems need:
- Chunks sized for embedding models (typical 512-2048 tokens).
- Stable IDs so re-indexing doesn't churn.
- Metadata for filtering (domain, kind, confidence, freshness).
- Cross-references as explicit fields, not inline text.
- Redundancy control (same fact not in 5 chunks).

Without C12, consumers have to reinvent these conventions, and retrieval quality suffers.

## Input

- `canonical/{domain}/` tree from C10 (validated by C11).
- `glossary`, `invariant`, `use-case`, `scenario`, `domain-model` artifacts with verified confidence.
- Configuration: target chunk size, embedding model hint, retrieval granularity.

## Output

Per workspace:

```
rag-package/
  chunks.jsonl              # one JSON per line, chunked content
  manifest.json             # version, generated_at, counts, source commit
  embeddings.jsonl          # optional: if embedding model specified
  index.md                  # human-readable map of chunks
  cross-references.jsonl    # edge list for graph-RAG
  README.md                 # how to ingest
```

Chunk schema:

```json
{
  "id": "orders.use-case.UC-003.confirm.preconditions",
  "domain": "orders",
  "kind": "use-case",
  "artifact_id": "UC-003",
  "section": "preconditions",
  "text": "An order must exist. The caller must be a participant: either the cargo_owner employee, the forwarder employee, or an operator.",
  "metadata": {
    "confidence": "verified",
    "invariants_referenced": ["INV-003", "INV-007"],
    "terms_referenced": ["order", "participant", "forwarder"],
    "source_commit": "abc123",
    "canonical_doc_path": "canonical/orders/use-cases.md#uc-003",
    "last_updated": "2026-04-21"
  }
}
```

## Modes

### Mode 1: `package`
Produce full RAG package from current workspace.

### Mode 2: `incremental`
Only re-chunk artifacts changed since last package.

### Mode 3: `embed`
Given chunks, produce embeddings (optional — needs embedding model API key).

### Mode 4: `tune-chunk-size`
Analyze which chunk sizes produce best retrieval quality (requires test queries).

## Algorithm

### Package

```
artifacts = forgeplan.query(confidence ≥ inferred, kind in extraction_kinds)
for a in artifacts:
  sections = split_by_headings(a.body)
  for s in sections:
    chunk = {
      id: stable_id(a.id, s.heading),
      text: s.body[:max_chunk_size],
      metadata: derive_metadata(a, s)
    }
    emit(chunk)

build_cross_references()
build_index_md()
build_manifest()
```

### Stable ID

`{domain}.{kind}.{artifact_id}.{section_slug}`

- Stable across re-packaging (so embeddings cache stays valid).
- Domain-prefixed for scoping.
- Kind-aware for filtering.

### Cross-references

For each `[X](../Y.md#anchor)` link in canonical docs, emit:

```json
{ "from": "orders.use-case.UC-003.confirm.flow", "to": "orders.invariant.INV-003", "type": "referenced_in" }
```

Enables graph-RAG strategies.

### Chunk size policy

- **Small** (≤ 512 tokens): DDL fragments, single invariants, scenario steps.
- **Medium** (≤ 1024 tokens): use-case sections, pseudo-code steps.
- **Large** (≤ 2048 tokens): full use-cases, glossary by domain.

Default: medium.

### De-duplication

If two chunks contain overlapping facts (detected by shingle hashing or semantic similarity), keep the one with higher confidence and higher `last_updated`.

## Metric

- `chunks_per_domain`: healthy 50-500 depending on domain size.
- `avg_chunk_token_count`: target 800-1200 tokens.
- `duplicate_rate`: target ≤ 5%.
- `cross_reference_density`: edges per chunk; target 2-5.

## Dependencies

- C10 canonical docs.
- C11 validation (only package validated docs).
- Optional: embedding model (OpenAI `text-embedding-3-small`, Cohere, local BGE).
- `tiktoken` or similar for token counting.

## Integration with autoresearch

`/autoresearch:ship --target rag`:
- Runs this skill.
- Saves to `rag-package/` directory in workspace.

## Downstream consumers

The RAG package is designed for:
- **LlamaIndex** / **LangChain**: ingest `chunks.jsonl`, use `metadata` for filtering.
- **pgvector**: one row per chunk.
- **Qdrant / Weaviate / Pinecone**: metadata filters + hybrid search.
- **Custom retrieval**: use `cross-references.jsonl` for graph-RAG.

## Prompt template

None — this skill is deterministic, not LLM-based (except optional embedding call).

## Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| Chunks too large for embedding model | Token count > limit | Split further on sub-headings |
| Lost cross-references | Graph-RAG retrieval misses related | Validate edge list non-empty per artifact |
| Unstable IDs across re-packaging | Embedding cache invalidated | Use content hash + artifact_id, not position |
| Outdated chunks packaged | `last_updated` old | Honor freshness decay; strip stale chunks |
| PII leak in exported chunks | Grep for PII patterns before shipping | Integrate with pii-detector agent |

## Example README.md output

```markdown
# RAG Package for {project}

Generated: {timestamp}
Source commit: {git_hash}
Domains: {list}
Artifacts: {count_by_kind}
Chunks: {total}

## Ingestion

1. Load `chunks.jsonl` into your vector store.
2. Embed using `text-embedding-3-small` (or provided `embeddings.jsonl`).
3. Index metadata fields for filtering.
4. Use `cross-references.jsonl` for graph-RAG, optional.

## Retrieval tips

- Filter by `domain` to scope answers.
- Filter by `confidence: verified` for high-stakes answers.
- Expand via cross-references for full context.

## Freshness

Chunks carry `last_updated`. Chunks older than 90 days should be re-validated.
```

## Testing

Fixture: small verified workspace → expect chunks with correct IDs + metadata + cross-refs. Deterministic output (same input → same chunks).

## Version history

- v1.0.0 — initial design.
