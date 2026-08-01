# Delivery Roadmap

## Phase 0 — Product truth and correctness

- утвердить product boundary;
- синхронизировать site/README/docs terminology;
- закрыть критический CLI/MCP parity и JSON gaps;
- устранить scoring/decay correctness blockers;
- внедрить docs-as-code baseline.

## Phase 1 — Protocol foundation

- Protocol v1 schemas;
- WorkContract;
- ExecutionReceipt;
- EvidenceBundle;
- VerificationVerdict;
- AuthorityPolicy;
- ExternalReference;
- CapabilityManifest.

## Phase 2 — Core implementation

- contract compiler;
- provenance verification;
- claim-centric R_eff;
- policy engine;
- agent API v2;
- conformance harness.

## Phase 3 — Extensions

- Marketplace v2 manifests;
- Cursor adapter;
- Codex adapter;
- OpenCode adapter;
- Claude adapter normalization.

## Phase 4 — Orchestrators

- Kandev;
- Vibe Kanban;
- Conductor;
- Paperclip.

## Phase 5 — Web and product surface

- ForgePlan Web v2;
- website rewrite;
- integration pages;
- compatibility/conformance pages;
- cross-repo docs automation.

## Phase 6 — Optional autonomous infrastructure

- remote MCP/server;
- event ingestion;
- auth and audit;
- OpenTelemetry;
- multi-repo registry;
- replay and idempotency.

## Parallelism

- Site/docs design может идти параллельно Protocol, но не публиковать unshipped features как shipped.
- Web UX может проектироваться параллельно, реализация зависит от Protocol schemas.
- Host adapters начинаются после WorkContract + CapabilityManifest.
- Orchestrator adapters начинаются после ExecutionReceipt + ExternalReference + Verdict.
