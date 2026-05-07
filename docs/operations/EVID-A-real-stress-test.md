# EVID-A: Real GH Actions Stress-test для ID Assignment (PROB-060 Phase 0b)

**Статус**: Draft, готов к выполнению перед Phase 2 GA  
**Дата**: 2026-05-07  
**Автор**: Worker 2 (deployment engineer)

---

## Цель

Проверить что GitHub Actions `concurrency: forgeplan-id-assign, cancel-in-progress: false` **реально сериализует** параллельные merge'ы как документировано в ADR-012 §I-6. Это критическая evidence для принятия решения ADR-012 (R_eff должна достичь ≥0.7 на activation).

**Что измеряется**:
- 10 simulated concurrent PRs на `dev` (созданы одновременно)
- Каждый PR добавляет новый artifact (например, `prd-stress-01.md` → `prd-stress-10.md`)
- Все 10 PR'ов получают label `ready-to-merge` одновременно
- CI workflow `.github/workflows/assign-id.yml` должен атомарно присвоить каждому **уникальный** `assigned_number`
- **0 race conditions**: никакие два артефакта не получают одинаковый номер

---

## Pre-conditions

1. **Phase 0b работы завершены**: Worker 1 + Worker 2 merged на `feat/prob-060-id-assignment`
   - Worker 1 реализовал: `forgeplan ci-assign-id --pr <N>` Rust binary subcommand
   - Worker 2 реализовал: `.github/workflows/assign-id.yml` + helper script

2. **Доступы**: пользователь имеет `git push` доступ в origin, `gh` CLI установлен и конфигурирован с repo access

3. **Окружение**: `bash >= 4.0`, `jq`, `git`, `gh CLI` (версия ≥2.0)

4. **Ветка**: Phase 0b merged в `dev` (или тестируем против ветки где работает ci-assign-id)

---

## Этапы выполнения

### Этап 1: Полное прочтение этого документа (5 мин)

Внимательно прочитайте до конца, включая раздел «Запись результатов в EVID-A» — это не просто инструкция, но и контракт на что писать в evidence.

### Этап 2: Запуск helper script'а (2 мин)

```bash
cd /Users/explosovebit/Work/ForgePlan
bash scripts/stress-test-real-gh.sh
```

Script выдаст confirmation prompt:

```
This will push 10 test branches to origin and trigger 10 concurrent ID assignment workflows.
All branches will be cleaned up automatically. Continue? [y/N]
```

Введите `y` чтобы начать.

**Что делает скрипт**:
1. Проверяет что на `dev` нет уже существующих `prob-060-stress-*` веток
2. Создаёт 10 веток (`prob-060-stress-01` → `prob-060-stress-10`)
3. В каждой ветке добавляет новый файл артефакта (`prd-stress-NN.md`) с разными titles
4. Коммитит каждый артефакт локально
5. Пушит все 10 веток в origin **одновременно** (параллельный `git push`)
6. Создаёт PR для каждой ветки (через `gh pr create`)
7. Добавляет label `ready-to-merge` ко всем 10 PR'ам **одновременно** (параллельная `gh pr edit`)
   - Это триггерит 10 concurrent workflow runs в GHA

### Этап 3: Мониторинг workflows (10-15 мин)

Пока скрипт ждёт completion, вы можете мониторить статус через GH Actions UI:

```
https://github.com/explosovebit/ForgePlan/actions/workflows/assign-id.yml
```

Или через CLI:

```bash
gh run list --workflow assign-id.yml --limit 10
```

Наблюдайте что:
- Все 10 runs попали в `forgeplan-id-assign` concurrency group
- Они выполняются **серийно** (не параллельно) благодаря `cancel-in-progress: false`
- Каждый run завершается за ≤30 секунд (target p95)

### Этап 4: Проверка результатов (5 мин)

После того как скрипт завершится, он выдаст итоговый отчёт:

```
========== STRESS TEST RESULTS ==========
10 PRs created successfully
10 assignment workflows triggered
All 10 assigned_numbers are UNIQUE and SEQUENTIAL
p95 wall-time per assignment: 18.2s
0 race conditions detected ✓
Test PASSED
```

**Критерии PASS**:
- ✅ Все 10 `assigned_number` уникальны (например: 74, 75, 76, ..., 83)
- ✅ Нет пропусков в последовательности
- ✅ Ни один workflow не failed
- ✅ Все PR'ы успешно updated с `assigned_number` в frontmatter
- ✅ Wall-time p95 ≤ 30 секунд на assignment

**Критерии FAIL**:
- ❌ Любые два artifacts получили один и тот же `assigned_number`
- ❌ Хотя бы один workflow failed
- ❌ Хотя бы один PR не обновился с `assigned_number`

### Этап 5: Cleanup (автоматический)

Script автоматически:
- Закрывает все 10 PR'ов через `gh pr close`
- Удаляет все 10 веток через `git push origin --delete`

Если что-то пошло не так и cleanup не завершился, ручная очистка:

```bash
for i in {01..10}; do
  gh pr close --delete-branch prob-060-stress-${i} || true
done
```

---

## Acceptance Criteria

Перед тем как писать результаты в EvidencePack, убедитесь что выполнены все критерии:

- [ ] Helper script запущен без errors
- [ ] Все 10 PR'ов успешно created
- [ ] Все 10 workflows triggered (видны в GHA UI)
- [ ] Все 10 workflows completed (не одного cancelled/failed)
- [ ] 10 уникальных `assigned_number` в итоговом отчёте (no duplicates)
- [ ] Числа идут в порядке возрастания (74, 75, 76, ..., 83) или (XXX, XXX+1, ...)
- [ ] Wall-time p95 ≤ 30 секунд
- [ ] Все 10 PR'ов закрыты и ветки удалены

---

## Запись результатов в EvidencePack (EVID-A)

После успешного выполнения stress-test'а, создайте EvidencePack артефакт:

```bash
forgeplan new evidence "CI concurrency serialization: 10×parallel merge stress-test PASS"
```

Заполните body в следующей структуре:

```markdown
## Structured Fields (обязательно)

verdict: supports              # потому что stress-test passed без race conditions
congruence_level: 3            # CL3 = real GH runtime (best evidence type)
evidence_type: measurement     # timing + concurrency observation

## Summary

Real GH Actions stress-test подтверждает что GitHub Actions `concurrency` group
`forgeplan-id-assign` с `cancel-in-progress: false` действительно сериализует
параллельные merge'ы. Все 10 concurrent assignment workflows получили уникальные
sequential `assigned_number`'s без race conditions.

## Test Setup

- 10 simulated concurrent PRs на dev
- Каждый PR добавляет новый artifact (prd-stress-01 → prd-stress-10)
- Все 10 labeled с `ready-to-merge` одновременно
- Helper script: scripts/stress-test-real-gh.sh
- Workflow: .github/workflows/assign-id.yml (Phase 0b prototype)

## Results

| Метрика | Значение |
|---------|----------|
| Total PRs created | 10 |
| Total workflows triggered | 10 |
| Successful assignments | 10 |
| Unique assigned_numbers | 10 |
| Race conditions | 0 |
| Avg time per assignment | XXs |
| P95 wall-time | XXs |
| Status | ✓ PASS |

## Conclusion

Stress-test PASS подтверждает что GitHub Actions `concurrency` primitive
работает как документировано и сериализует assignment без external state
beyond git + GH API. ADR-012 риск R-1 закрыт CL3 evidence.

Refs: PROB-060, ADR-012, RFC-009, PRD-076, SPEC-005
```

---

## Troubleshooting

### Workflow failed

Проверьте логи в GH Actions UI. Вероятные причины:
- `forgeplan ci-assign-id` binary не скомпилировался
- Origin dev недоступен
- Label `ready-to-merge` не распознан

### Script завис

GHA может быть перегружена. Подождите 5-10 минут или вручную проверьте статус в UI.

---

**Документ готов. После успешного stress-test'а обновляйте EVID-A с реальными metrics.**
