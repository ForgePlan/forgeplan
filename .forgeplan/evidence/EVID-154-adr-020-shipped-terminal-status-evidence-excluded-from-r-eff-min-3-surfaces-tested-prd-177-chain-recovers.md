---
depth: tactical
id: EVID-154
kind: evidence
links:
- target: ADR-020
  relation: informs
- target: PROB-084
  relation: informs
status: active
title: 'ADR-020 shipped: terminal-status evidence excluded from R_eff min — 3 surfaces tested, PRD-177 chain recovers'
---

---
assigned_number: 154
predicted_number: 154
slug: evid-adr-020-shipped-terminal-status-evidence-excluded-from-r-eff-min-3
---

## Summary

ADR-020 (терминальная эвиденция исключается из weakest-link min) реализован и проверен на всех трёх поверхностях: движок, CLI, MCP. Точная цепочка PRD-177 из upstream-репорта #436 воспроизведена и восстанавливается: активный refutes → 0.00, честное вытеснение через `supersede --by` → 1.00, вытесненный пак остаётся видимым с пометкой `excluded from min`, пропуск логируется в factors.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

base_sha: 92154e19
result_sha: 6cfefad
changed_paths: crates/forgeplan-core/src/scoring/reff.rs, crates/forgeplan-core/src/scoring/evidence.rs, crates/forgeplan-cli/src/commands/score.rs, crates/forgeplan-mcp/src/server.rs, crates/forgeplan-mcp/src/types.rs, crates/forgeplan-cli/tests/cli_score_terminal_evidence.rs, crates/forgeplan-mcp/tests/score_terminal_evidence_e2e.rs

CL3: тесты и догфуд гоняют ровно тот shipped-код, который описывает ADR.

## Test results

- **Юнит (reff.rs)**: 5 новых — mixed active+superseded → min по активным (acceptance #436); deprecated исключается; all-terminal → «no active evidence» 0.0 (quint-code edge); активный refutes продолжает обнулять + draft продолжает считаться (защитный инвариант); CI-популяция без терминальных. Весь scoring-модуль: 91 pass.
- **CLI E2E (реальный бинарь)**: `cli_score_terminal_evidence.rs` — цепочка PRD-177 (0.00 → supersede → 1.00, JSON-поля `excluded`/`status`, factors, текстовая пометка) + all-terminal кейс (замена существует, но не слинкована → 0.0, «No active evidence»).
- **MCP E2E (реальный хендлер)**: `score_terminal_evidence_e2e.rs` — восстановленный r_eff + `excluded: true`/`status: superseded` в DTO.
- **Регрессии**: core 2095 pass / 0 fail; CLI+MCP 60 сьютов, 727 pass / 0 fail. `cargo fmt --check` 0, `clippy --workspace --all-targets -D warnings` 0.
- **Догфуд**: живой workspace, полная цепочка руками — вывод дословно совпал с ожиданием ADR.

## Provenance

Branch `fix/reff-exclude-terminal-evidence` (off origin/dev 92154e19). Код: коммит 6cfefad; доки: f0cd5ad; артефакты: 71f01a2. Исследование перед фиксом: 4-пуловый workflow (доки/граф+первоисточники/forgeplan.dev/код) — включение терминальной эвиденции нигде не задокументировано как намеренное; quint-code `decision.go:818` и FPF F.10:6.1 подтверждают семантику исключения.




