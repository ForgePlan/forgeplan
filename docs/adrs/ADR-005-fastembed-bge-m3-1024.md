---
id: ADR-005
title: "fastembed-rs + BGE-M3 (1024-dim) вместо ort + BGE-M3 (384-dim)"
status: Accepted
depth: standard
valid_until: 2027-03-22
problem_ref: ""
created: 2026-03-22
updated: 2026-03-22
---

# ADR-005: fastembed-rs + BGE-M3 (1024-dim) вместо ort + BGE-M3 (384-dim)

## Context

Phase 4 требует semantic search по артефактам. VISION.md указывает `ort (ONNX Runtime)` с `BGE-M3` на 384 dimensions. Исследование показало, что:
1. `fastembed-rs` (от Qdrant) даёт drop-in support для BGE-M3 + BGE-Reranker-v2 без ручного ONNX export
2. BGE-M3 поддерживает 1024-dim (полноразмерные) — лучшее качество retrieval
3. Reranking (BGE-Reranker-v2-M3) доступен из коробки — нет в `ort` без ручной работы

## Decision

**Selected**: `fastembed-rs` с BGE-M3 full-size (1024-dim) + BGE-Reranker-v2-M3.

**Why Selected**: Production-ready crate (Qdrant backed), zero-config tokenization, auto-download моделей с HF Hub, поддержка квантизации, reranker из коробки. 1024-dim даёт лучшее качество семантического поиска чем 384-dim.

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| `ort` + manual ONNX export (384-dim) | Rejected | Нужен Python для ONNX export, нет reranker, больше boilerplate |
| `fastembed-rs` + BGE-M3 (384-dim) | Rejected | Работает, но 1024-dim даёт лучший recall при незначительном увеличении storage |
| `fastembed-rs` + BGE-M3 (1024-dim) | **Chosen** | Best quality, zero-config, reranker included |
| `candle` (HuggingFace Rust ML) | Rejected | Нет built-in BGE-M3, нужен custom code, меньше production evidence |
| `rust-bert` | Rejected | Legacy, нет BGE support |

## Consequences

### Positive
- 100% Rust — нет Python dependency
- Drop-in BGE-M3 + Reranker-v2 через enum variants
- Auto-download с HF Hub, кэширование в `~/.cache/huggingface/`
- Квантизованные варианты для edge deployment (~350MB vs 1.2GB)
- 3-5x быстрее Python (ONNX Runtime backend)
- 1024-dim = лучший recall на MTEB benchmarks

### Negative (trade-offs)
- Binary size +~25 MB (ONNX Runtime libs) — нужен feature flag
- Model size ~600-1200 MB (загружается при первом использовании)
- NFR-002 (< 15MB binary) → semantic-search = optional feature
- EMBEDDING_DIM 384 → 1024 — breaking change для существующих LanceDB таблиц
- LanceDB `FixedSizeList(1024)` — больше storage на артефакт

### Risks
- fastembed-rs API изменится (pre-1.0 crate) → pin version
- Модель удалена с HF Hub → bundle или зеркало
- 1024-dim слишком медленный на слабых машинах → fallback к quantized model

## Invariants

- `fastembed-rs` используется ТОЛЬКО через feature flag `semantic-search`
- Без feature flag — CLI работает без embedding модели (binary < 15MB)
- EMBEDDING_DIM = 1024 (full BGE-M3)
- Модели загружаются при первом `forgeplan search --semantic`, не при `forgeplan init`
- Reranker используется для `forgeplan search` с `--rerank` flag
- Embedding dimension записан как константа `EMBEDDING_DIM` в `db/schema.rs`

## Evidence Requirements

- fastembed-rs компилируется в Forgeplan workspace
- Embedding generation < 500ms для одного артефакта (среднего размера ~500 слов)
- Vector search < 200ms на 1000 артефактах с LanceDB ANN
- Reranking top-10 results < 1s
- Binary size с feature flag < 40MB (без ONNX: < 15MB)

## Valid Until

**Дата**: 2027-03-22

**Refresh Triggers**:
- fastembed-rs прекратит поддержку
- Появится лучшая multilingual embedding модель для Rust
- ONNX Runtime заменится на другой backend (WGPU, Metal)

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| SPEC-001 | Spec | updates (embedding dimension 384 → 1024) |
| RFC-003 | RFC | informs (LanceDB integration) |
| ADR-002 | ADR | extends (LanceDB + vectors) |
| EPIC-001 | Epic | parent |
| VISION.md | Doc | updates (ort → fastembed) |
