---
assigned_number: 205
author: null
created: 2026-11-16
depth: critical
id: EVID-205
kind: evidence
links:
- target: SPEC-010
  relation: informs
- target: PRD-081
  relation: informs
- target: RFC-016
  relation: informs
- target: RFC-018
  relation: informs
parent_epic: EPIC-010
predicted_number: 205
slug: evid-c23-acceptance-constellation-canonical-reads-and-bounded-contextpack
status: draft
tags: [constellation, contextpack, acceptance, c23, s1]
title: C23 acceptance Constellation canonical reads and bounded ContextPack
updated: 2026-11-16
valid_until: null
---

---
assigned_number: 205
created: 2026-11-16
predicted_number: 205
slug: evid-c23-acceptance-constellation-canonical-reads-and-bounded-contextpack
updated: 2026-11-16
---

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 4
review_verdict: PASS
source_pin: 8928a80aad9fab05d62cc0e049e696578940a31431e1596783df72d19ccb2e00

## Measurement

2026-11-15 .. 2026-11-16, macOS/arm64. Исполнитель: Main (оркестратор), с исполнительскими артефактами Sol; Sol довёл сборку и большую часть проверок, затем упал на ошибке баланса провайдера (deepseek/deepseek-v4-flash 402) после MCP5. Оркестратор прочёл настоящие файлы результатов, восстановил фикстуры побайтово, завершил MCP6 и BGE-паритет, выписал итог.

Candidate **C23** — тот же набор из **425 парных исходных входов** (Rust/Cargo), замороженный манифест `8928a80aad9fab05d62cc0e049e696578940a31431e1596783df72d19ccb2e00`. Точечная правка из четырёх файлов (federation.rs, snapshot.rs, impact.rs, constellation_cross_source_impact.rs) устраняет два фактических дефекта C22: потерю третьего кандидата на широком квалификаторе GET в live и pinned чтениях, и `collapsible_if` под строгим Rust 1.95 Clippy.

## Receipts (ground truth)

| Чек | Итог | Файл |
|---|---|---|
| Astra bounded corrective-source gate | PASS — diff из 4 файлов, источник не рассширен | `.local/contextpack-c7/final-review-inputs/astra-c23-source-result.json` |
| Rust 1.95 rustfmt | PASS — 4 изменённых файла | `build-23/` receipt |
| Production pair (CLI + standalone MCP, semantic-search, -D warnings) | PASS — immutable saved pair | `final-sol-23/production-binding.json` |
| Strict Clippy (lib+bins, features semantic-search) | PASS exit 0, 38.7s | `final-sol-23/02-production-strict-clippy.json` |
| Core integration constellation_cross_source_impact | PASS 8/8 | `final-sol-23/04-Core-eight-cases.json` |
| MCP constellation_read_roles (affected) | PASS 6/6 | `final-sol-23/` (executed by orchestrator on binary `constellation_read_roles-249d50815e2d9599`) |
| Retained external CLI2 runner | PASS 2/2 against current production child | `final-sol-23/07-C20-external-runner-C23-production-child.json` |
| Runtime broad-qualified GET | PASS 68/68 — live и pinned возвращают все 3 кандидата; Shared scope протухает на реальной мутации, переоткрывается с новым третьим, snapshot сохраняет исходного | `final-sol-23/runtime/verdict.json` |
| BGE-M3 parity CLI + full MCP | PASS — 63/63 eligible, 12 canonical keys в одинаковом порядке, скоры в пределах 1e-6, catalog 76, model cache byte-identical, без fetch | `final-sol-23/semantic-current/semantic-current-verdict.json` |
| Website build | PASS exit 0 — 566 pages, 608 HTML | `docs-runtime-c23/build-receipt.json` |

Bundle manifest с хешами: `.local/contextpack-c7/final-acceptance-c23-manifest.json`.

## Fixture provenance for BGE parity

TMP-фикстуры ws-a/ws-b были удалены ОС. Восстановлены побайтово из `Integration/eval/corpus` — все 26 файлов проходят по `source_input_hashes` в `final-sol-19/semantic-composition.json`, несоответствий 0. `.forgeplan/.lock` и constellation owner locks пересозданы нативным CLI; индекс собран `forgeplan embed` тем же кэшированным BGE-M3 (63 строк ws-a, 53 ws-b), chunk-ключи совпадают с F10 baseline.

## Explicit limits

- Это **не** полный реплей 137-test matrix. Повторный прогон осознанно не делался: scoped supersets покрывают все затронутые пути (source freeze + runtime broad GET + qualified impact).
- Windows local runtime — owner-authorized skip, не PASS.
- F10 — одна фиксированная 180-row сверка; F11 и целое S1–S4 program acceptance открыты и не входят в этот verdict.
- Fresh large-corpus write percentiles — только исторические измерения C6 на dev-binary, не на текущем optimized candidate.
- Исполнитель Sol имел provider-balance сбой после Core+Clippy+CLI2+runtime+build artifacts; оркестратор не подменял модель — докончил MCP6 и BGE и подписал этот отчёт. Формальная independence boundary между «кто писал» и «кто проверял» сохранена: источники и oracle не менялись оркестратором.

## Scope binding

SPEC-010 `spec-contextpack-bounded-federated-evidence-kb8bac5ddcb6edda3c6a10447bff91441`, parent PRD-081, epic EPIC-010. Документационный контракт: PRD-089 / RFC-017. Совместимость обычного чтения: RFC-018. Owner-формализм G1/G2/G3: NOTE-059 (release-scoped DEFERRED, не PASS).

## Out of scope

Issue #538: широкая S1–S4 acceptance — остаётся открытой; cleanup тела и промежуточных комментариев только по явной авторизации. Feature commit/PR/release — отдельные авторизованные шаги.
