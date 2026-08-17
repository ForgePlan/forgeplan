---
depth: tactical
id: EVID-156
kind: evidence
links:
- target: ADR-020
  relation: informs
- target: PRD-082
  relation: informs
status: active
title: 'v0.34.0 release cut: all RELEASE-PROTOCOL pre-conditions verified by measurement'
---

---
assigned_number: 156
predicted_number: 156
slug: evid-v0-34-0-release-cut-all-release-protocol-pre-conditions-verified-by
---

## Summary

Релиз v0.34.0 срезан по `docs/operations/RELEASE-PROTOCOL.md`. Все pre-conditions проверены измерением, а не самоотчётом: `dev` зелёный (включая security-гейт, который до этого падал 11 дней), артефакты релиза активны с R_eff > 0, smoke-тест проходит, dependabot-триаж заведён, версия поднята во всех четырёх manifest-местах.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: audit

base_sha: 8a334a7
result_sha: 389b6ba
changed_paths: CHANGELOG.md, CLAUDE.md, README.md, README.ru.md, TODO.md, Cargo.toml, Cargo.lock, crates/forgeplan-cli/Cargo.toml, crates/forgeplan-mcp/Cargo.toml

CL3: измерения выполнены на самом релизном срезе тем бинарём, который релизится.

## Pre-conditions (RELEASE-PROTOCOL:39-50)

| Pre-condition | Статус | Как проверено |
|---|---|---|
| `dev` зелёный | ✅ | security workflow на dev: `completed/success` 2026-08-17T10:10 — впервые после серии `failure` с 2026-08-06 |
| Артефакты релиза `active`, R_eff > 0 | ✅ | PRD-082 активирован (R_eff 1.00), EVID-149..153 + PROB-083/084/085 активированы; `score --all` → 82/89 артефактов ≥ 0.5 |
| `cargo test` зелёный | ✅ | последний зелёный CI: 3243 passed / 7 skipped (nextest) + 9 doc-tests |
| `scripts/smoke-test.sh` | ✅ | `=== SMOKE TEST PASSED ===` дважды: до среза и после bump-а версии |
| dependabot-триаж заведён | ✅ | `docs/operations/dependabot-triage-2026-08-17.md`, 42 алерта размечены |

## Security gate (RED LINE #10)

`cargo deny check advisories` → **advisories ok**.

Закрыто в этом окне: RUSTSEC-2026-0204 (`crossbeam-epoch` 0.9.18→0.9.20), GHSA-4w2j-m93h-cj5j HIGH (`quinn-proto` 0.11.14→0.11.16), GHSA-7gcf-g7xr-8hxj (`serde_with` 3.18.0→3.22.0). Все — lockfile-only именные бампы, без blanket `cargo update`.

Отдельная находка окна: **RustSec не зеркалится в Dependabot**, поэтому cargo-deny падал 11 дней при чистом списке алертов. Для Rust-проекта Dependabot — не достаточный security-гейт; зафиксировано в триаж-доке как урок.

## Version bump (RELEASE-PROTOCOL:113-138)

Четыре manifest-места: `Cargo.toml` workspace, `forgeplan-cli` (2 path-ref), `forgeplan-mcp` `[dependencies]` + `[dev-dependencies]` (последнее — то, что чаще всего забывают). Проверка `grep -rn 'version = "0.33.0"' Cargo.toml crates/*/Cargo.toml` → пусто. Lockfile перегенерирован через `cargo check --workspace` (0 ошибок). Бинарь рапортует `forgeplan 0.34.0`.

Намеренно НЕ тронуты: `Formula/forgeplan.rb` (мёртвый in-tree стаб с placeholder-SHA — реальную формулу публикует cargo-dist), `install.sh` (version-agnostic, `releases/latest`), `dist-workspace.toml` (`cargo-dist-version` — версия инструмента, не проекта), `website/package.json` (версия Astro-сайта), исторические упоминания версий в CHANGELOG/архивных доках.

## Docs (RELEASE-PROTOCOL:140-150)

Счётчики в доках оказались устаревшими сильнее, чем на один релиз, и заменены измеренными:

| Показатель | Было | Стало (измерено) |
|---|---|---|
| CLI-команды | 76 (CLAUDE.md), 33 (README.ru) | **81** (`forgeplan --help`) |
| MCP-инструменты | 73, но 37 в README.ru | **73** (`grep -c '#[tool('`) |
| Тесты | 3095+ / 3084 / 728+ | **3243 + 9 doc-tests** (зелёный CI nextest) |
| Артефакты | 341 / 138 (бейдж) | **394** (`find .forgeplan -maxdepth 2 -name '*.md'`) |

`README.ru.md` нёс числа примерно эпохи v0.10. Также в индекс `docs/README(.ru).md` добавлены отсутствовавшие там `RELEASE-PROTOCOL` и конвенция `dependabot-triage-*`.

## Documentation red line

Релиз добавляет user-facing surface, которого не было ни в одном доке (проверено grep-ом по `docs/`, `website/`, `CLAUDE.md`, `templates/`):

- `integrity.evidence_provenance_gate` → конфиг-референс сайта (EN+RU) + CLAUDE.md;
- `base_sha` / `result_sha` / `changed_paths` → секция в EVIDENCE-PROTOCOL (EN+RU) с пятью вердиктами и режимами гейта, шаблон эвиденции (внутри HTML-комментария, чтобы парсер не принял плейсхолдер за реальный claim — ловушка PROB-034) и CLAUDE.md.

## Breaking changes и измеренный эффект

Два breaking-изменения: R_eff считает только текущую эвиденцию (ADR-020) и `update --status superseded|deprecated|active` отвергается.

Dry-run старых-против-новых оценок (требование vNext-аудита M5): `forgeplan score --all` → **82 из 89 артефактов ≥ 0.5**, и ровно **один** (PRD-005) падает до 0.00 — его единственный пак EVID-010 был deprecated ещё в апреле. То есть новая семантика вскрыла реальный долг, а не сломала оценки.

## Известное состояние на момент релиза

- PRD-005 — единственный At-Risk (см. выше), реальный долг, не регресс.
- PROB-085 (EVID-155) — тесты гоняют cwd процесса, локальный флейк параллельного прогона; CI зелёный, продуктовый код не затронут.
- 39 npm-алертов в `website/` — scheduled отдельным PR (blanket `npm update` там ломает сборку, PR #401).
- ADR-019 и EPIC-009 намеренно НЕ активированы: vNext-пакет получил NEEDS_REWORK от собственного 13-агентного аудита (EVID-149), а ADR-019 объявляет `contradicts ADR-009` — это решение человека, а не релизная рутина.




