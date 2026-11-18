# Integration: RAG Export Format

> Formal specification of the export format produced by C12 for RAG ingestion.

> Proposed external export format, not Constellation's index schema or an implemented core exporter. All counts, hashes, dates, token counts and vectors below are illustrative; no export or ingestion was exercised here. The `text-embedding-3-small`/1536 example is not Forgeplan's BGE-M3 runtime configuration. See [implementation status](../README.md#implementation-status-2026-10-23).

## Directory structure

```
rag-package/
├── manifest.json
├── chunks.jsonl
├── embeddings.jsonl          # optional, generated if --embed
├── cross-references.jsonl
├── index.md                  # human-readable navigation
├── README.md                 # ingestion guide
└── schemas/
    ├── chunk.schema.json
    ├── manifest.schema.json
    └── cross-reference.schema.json
```

## `manifest.json`

```json
{
  "version": "1.0",
  "generated_at": "2026-04-21T12:00:00Z",
  "source_project": "TripSales",
  "source_commit": "abc123def",
  "source_workspace": ".forgeplan",
  "domains": ["orders", "quotes", "trips", "calculator", "auth"],
  "chunk_count": 842,
  "chunk_by_kind": {
    "glossary": 128,
    "use-case": 142,
    "invariant": 87,
    "scenario": 156,
    "domain-model": 12,
    "hypothesis": 143,
    "canonical": 174
  },
  "confidence_distribution": {
    "verified": 287,
    "strong-inferred": 298,
    "inferred": 199,
    "speculation": 58
  },
  "embedding_model": "text-embedding-3-small",
  "embedding_dimensions": 1536
}
```

## `chunks.jsonl` (one JSON per line)

Each line is a chunk. Schema enforced by `schemas/chunk.schema.json`:

```json
{
  "id": "orders.use-case.UC-003.confirm.main-flow",
  "domain": "orders",
  "kind": "use-case",
  "artifact_id": "UC-003",
  "section": "main-flow",
  "title": "Forwarder confirms an order — main flow",
  "text": "An order participant submits orders_Confirm(id). The system loads the order, verifies participation (INV-003), loads the related sales_order, sets status to forwarder_confirmed, initializes cargo, enqueues markings job, updates Elasticsearch, and returns the updated order.",
  "text_token_count": 85,
  "metadata": {
    "bounded_context": "orders",
    "confidence": "inferred",
    "invariants_referenced": ["INV-003", "INV-005", "INV-012"],
    "terms_referenced": ["order", "forwarder_confirmed", "cargo", "markings"],
    "related_use_cases": ["UC-002", "UC-007"],
    "source_commit": "abc123",
    "canonical_doc_path": "canonical/orders/use-cases.md#uc-003",
    "last_updated": "2026-04-21",
    "lifecycle_state": "active"
  }
}
```

### ID format: `{domain}.{kind}.{artifact_id}.{section_slug}`

- Stable across re-generation if artifact body hasn't changed.
- Easy to filter by prefix (`orders.*` → all Orders chunks).
- Joinable with `cross-references.jsonl`.

## `embeddings.jsonl` (optional)

One JSON per line; references chunk by ID:

```text
{ "id": "orders.use-case.UC-003.confirm.main-flow", "vector": [0.012, -0.453, ...] }
```

The vector above is abbreviated pseudo-JSON, not a loadable record. Actual JSONL must contain only numeric values, with exactly `manifest.embedding_dimensions` entries. Proposed generation occurs only when the exporter is implemented and embedding is requested.

## `cross-references.jsonl`

Edge list for graph-RAG:

```jsonl
{"from":"orders.use-case.UC-003.confirm.main-flow","to":"orders.invariant.INV-003.statement","type":"references","weight":1.0}
{"from":"orders.use-case.UC-003.confirm.preconditions","to":"orders.glossary.TERM-012.definition","type":"uses-term","weight":1.0}
{"from":"orders.scenario.SC-042.happy-path","to":"orders.use-case.UC-003.confirm.main-flow","type":"verifies","weight":1.0}
```

Edge types:
- `references` — artifact A references invariant/term/use-case B.
- `uses-term` — specifically for term lookups.
- `verifies` — scenario verifies use-case or invariant.
- `supersedes` — newer version replacing older.
- `contradicts` — explicit contradiction (consumers may want to display warning).
- `based-on` — derived from another artifact.

## `index.md`

Human-readable navigation:

```markdown
# RAG Package Index

## Domains

### orders (287 chunks)
- [UC-001 Create an order](#orders-uc-001)
- [UC-002 Accept a quote](#orders-uc-002)
- [UC-003 Confirm an order](#orders-uc-003)
...

### quotes (156 chunks)
...

## Glossary (global)
- [forwarder](#forwarder)
- [cargo_owner](#cargo_owner)
...

## Invariants (cross-domain)
- [INV-003 — Order visibility limited to participants](#inv-003)
...
```

## `README.md`

````markdown
# {{project}} RAG Package v1.0

Generated: {{timestamp}}
Source commit: {{hash}}
Chunks: {{count}}
Embedding model: {{model or 'not embedded'}}

## Ingestion

### PostgreSQL + pgvector

Example for a disposable database with pgvector installed and permission to enable it. Run through `psql` with `ON_ERROR_STOP=1` from the package directory. The optional embedding section requires a complete `embeddings.jsonl`; omit that section when no vectors were exported. The column dimension must match the manifest. PostgreSQL COPY has no JSON format: stage each valid JSONL object as one CSV field, using control-byte delimiters/quotes that cannot occur literally in valid JSON.

```sql
\set ON_ERROR_STOP on
CREATE EXTENSION IF NOT EXISTS vector;
CREATE TABLE rag_chunks (
  id TEXT PRIMARY KEY,
  domain TEXT,
  kind TEXT,
  artifact_id TEXT,
  section TEXT,
  text TEXT,
  metadata JSONB,
  embedding vector(1536)
);

-- Stage complete JSONL rows; CSV mode preserves JSON backslash escapes.
CREATE TEMP TABLE chunk_rows (value jsonb);
\copy chunk_rows FROM 'chunks.jsonl' WITH (FORMAT csv, DELIMITER E'\x01', QUOTE E'\x02')
INSERT INTO rag_chunks (id, domain, kind, artifact_id, section, text, metadata)
SELECT value->>'id', value->>'domain', value->>'kind', value->>'artifact_id',
       value->>'section', value->>'text', value->'metadata'
FROM chunk_rows;

-- Optional: only when a complete embeddings.jsonl is present.
CREATE TEMP TABLE embedding_rows (value jsonb);
\copy embedding_rows FROM 'embeddings.jsonl' WITH (FORMAT csv, DELIMITER E'\x01', QUOTE E'\x02')
UPDATE rag_chunks AS c
SET embedding = (e.value->'vector')::text::vector(1536)
FROM embedding_rows AS e
WHERE c.id = e.value->>'id';

-- Create index
CREATE INDEX ON rag_chunks USING hnsw (embedding vector_cosine_ops);
```

### LlamaIndex

```python
from llama_index.core import VectorStoreIndex
from llama_index.core.schema import TextNode
import json

# These are already-exported chunks: preserve their IDs instead of re-chunking.
# Configure a supported embedding provider separately; embedding may send text
# to that provider and incur charges.
nodes = []
with open("chunks.jsonl", encoding="utf-8") as f:
    for line in f:
        c = json.loads(line)
        metadata = {**c["metadata"], "domain": c["domain"]}
        nodes.append(TextNode(text=c["text"], id_=c["id"], metadata=metadata))

index = VectorStoreIndex(nodes)
```

### Qdrant / Weaviate / Pinecone

Use the chunk's `metadata` fields for filtering. For graph-RAG, load `cross-references.jsonl` into a separate edge collection.

## Retrieval tips

- **Scope answers**: the export stores `domain` at top level. Filter the SQL `domain` column, or `metadata.domain` after explicitly copying it into a consumer's metadata as in the LlamaIndex example.
- **High-stakes**: confidence labels can help retrieval filtering, but `verified`/`strong-inferred` do not guarantee correctness; inspect supporting evidence.
- **Freshness**: prefer `metadata.last_updated` within last 90 days.
- **Graph expansion**: after initial retrieval, expand via `cross-references.jsonl` to related chunks.

## Freshness policy

Chunks have `metadata.last_updated`. Recommended refresh cycles:
- Daily: inspect `forgeplan_drift` affected-file signals, then determine which source assertions and chunks actually need review; it does not detect semantic chunk staleness itself.
- Weekly: re-export changed chunks only.
- Monthly: full re-embedding.
````

## Versioning

Manifest includes a `version` field. Consumers should validate compatibility before ingesting:

```
v1.0 — initial format
v1.1 — add confidence_per_assertion sub-chunks (planned)
```

Breaking changes bump major version.

## Redundancy control

Before emission, C12 de-duplicates chunks that overlap > 85% (shingle hashing):
- Keep the higher-confidence chunk.
- If equal confidence, keep the longer (more context).
- Always keep at least one chunk per artifact.

## PII scrubbing

Before emission, C12 runs a PII scan:
- Email addresses → `<EMAIL_REDACTED>`.
- Phone numbers → `<PHONE_REDACTED>`.
- Credit card patterns → flag and halt (require manual review).
- API keys (regex for common patterns) → flag and halt.

Hook into the `pii-detector` agent from the marketplace if available.

## Testing

Fixture: tiny workspace → expect:
- `chunks.jsonl` line-count matches `manifest.chunk_count`.
- `cross-references.jsonl` has no orphan edges.
- `index.md` has entries for every domain in manifest.
- Embedding file (if requested) has exactly one line per chunk.

## Version history

- v1.0.0 — initial format spec.
