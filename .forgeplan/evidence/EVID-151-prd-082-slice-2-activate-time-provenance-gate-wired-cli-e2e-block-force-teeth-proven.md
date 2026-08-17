---
depth: standard
id: EVID-151
kind: evidence
last_modified_at: 2026-08-03T11:35:23.273747+00:00
last_modified_by: claude-code/2.1.220
links:
- target: PRD-082
  relation: informs
status: active
title: 'PRD-082 slice 2: activate-time provenance gate wired, CLI E2E block+force teeth-proven'
---

# PRD-082 slice 2 — activate-time provenance gate

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test
base_sha: f74b4cf
result_sha: 2a5608b
changed_paths: crates/forgeplan-core/src/scoring/provenance.rs, crates/forgeplan-core/src/config/types.rs, crates/forgeplan-cli/src/commands/activate.rs, crates/forgeplan-mcp/src/server.rs, crates/forgeplan-cli/tests/cli_provenance_gate_e2e.rs

CL3 — измерено на этой ветке этими же командами.

## Что проверялось

Подключение примитива слайса 1 к `forgeplan_activate` как гейта: EvidencePack с
git-claim'ом, который не держится (empty delta / missing path / incomplete),
блокирует или предупреждает при активации по конфигу
`integrity.evidence_provenance_gate` = block | warn | off.

## Команды и результат

```
cargo fmt -- --check                                -> 0 diff
cargo clippy --workspace --all-targets -D warnings   -> 0
cargo test --workspace -- --test-threads=4           -> 85/85 наборов зелёные
cargo test -p forgeplan-core --lib -- provenance::gate_tests -> 6 passed
cargo test -p forgeplan --test cli_provenance_gate_e2e       -> 2 passed
```

## Ключевые покрытые исходы

| Тест | Что фиксирует |
|---|---|
| `block_mode_refuses_activation_on_an_empty_delta_claim` (CLI E2E) | Под `block` активация EVID с `base_sha == result_sha` **отклоняется** — центральный случай #360, на реальном бинаре |
| `force_overrides_the_provenance_gate` (CLI E2E) | `--force` обходит гейт |
| `evaluate_turns_a_git_error_into_a_warn_never_a_block` | Недостижимый base_sha → **warn**, не block, даже в режиме block (ADR-019: Core не владеет worktree) |
| `off_mode_always_passes_even_on_a_failed_claim` | `off` короткозамыкает до git-вызова |
| `acceptable_verdicts_pass_in_every_mode` | 148 legacy-пакетов (`NotClaimed`) не гейтятся ни в одном режиме |
| `from_config_...defaults_to_warn` | Неизвестное значение → warn, никогда молча off |

**Зубы E2E:** переключение конфига теста на `off` роняет block-ассерт
(активация не отклоняется) — тест ловит именно гейт, не декорация.

## Решения (зафиксированы в PRD-082)

- **Default = warn.** Новый блокирующий гейт не должен ломать существующие
  потоки при первом включении; владелец переводит в `block`, когда доверяет.
- **git-ошибка → всегда warn**, не block. Проблема окружения — не ложный claim.
- **`force` обходит** — тот же эскейп-хэтч, что у методологических гейтов.
- Подключено в **явный** activate-путь (CLI + MCP). Два link-auto-activate
  пути — осознанный follow-up FR (уже́ blast radius для первого приземления).

## Границы / известное ограничение

- MCP-поверхность в режиме `warn` логирует предупреждение **server-side**, не в
  payload ответа; `block` возвращает видимую агенту ошибку. Вывод warn в
  payload — follow-up.
- Гейт не запускает тесты и не владеет деревом (ADR-019). Проверяет
  происхождение claim'а, не качество диффа.

## Related

- PRD-082 — родительский документ
- EVID-150 — слайс 1 (примитив)
- GitHub #360 — источник; ADR-019 — граница


