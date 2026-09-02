---
depth: tactical
id: EVID-158
kind: evidence
links:
- target: PRD-083
  relation: informs
- target: ADR-022
  relation: informs
status: active
title: 'Five-target dist build with semantic-search: 4 of 5 fail, three distinct causes'
---

---
assigned_number: 158
predicted_number: 158
slug: evid-five-target-dist-build-with-semantic-search-4-of-5-fail-three-distinct
---

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement

## What was measured

Может ли `semantic-search` быть включён в дистрибутивные бинари ForgePlan — проверено
реальной сборкой на всех пяти релизных таргетах, а не рассуждением.

PRD-083 требовал именно измерения (FR-002): предполагать нельзя, потому что цена ошибки —
несостоявшаяся публикация релиза целиком (cargo-dist роняет весь workflow при падении
любого таргета; прецедент v0.32.0, `CHANGELOG.md:102`).

## How

Штатного способа проверить не существовало. `release.yml` триггерится на `pull_request` и на
push тега, но джоб `build-local-artifacts` включается условием (`release.yml:96`):

```
needs.plan.outputs.publishing == 'true' || …ci.github.pr_run_mode == 'upload'
```

По умолчанию на PR выполняется только `dist plan`. То есть изменение релизной сборки
физически не компилируется до пуша версионного тега — до момента настоящего релиза.

Поэтому в конфиг временно добавлены два ключа:

```toml
features = ["semantic-search"]
pr-run-mode = "upload"
```

`pr-run-mode = "upload"` — документированная возможность cargo-dist, которую upstream прямо
рекомендует для кросс-платформенной проверки и прямо помечает как временную. Оба ключа сняты
после получения результата.

Прогон: GitHub Actions run
[33647382377](https://github.com/ForgePlan/forgeplan/actions/runs/33647382377), PR #455,
коммиты `a278a01` + `f165609`, 2026-09-02.

## Result — 1 of 5 passed

| Таргет | Итог | Причина отказа |
|---|---|---|
| `aarch64-apple-darwin` | **PASS** | — |
| `x86_64-apple-darwin` | FAIL | `ort does not provide prebuilt binaries for the target x86_64-apple-darwin with feature set (no features)` |
| `x86_64-unknown-linux-gnu` | FAIL | `rust-lld: undefined symbol: __isoc23_strtoll / __isoc23_strtoull / __isoc23_strtol` |
| `aarch64-unknown-linux-gnu` | FAIL | то же |
| `x86_64-pc-windows-msvc` | FAIL | `LNK1120: 66 unresolved externals` из `libort_sys` — `__imp__dup`, `__imp_strncpy`, `__imp_modf`, `__imp___timezone`, `__imp_fopen_s`, … |

`plan` прошёл; `build-global-artifacts`, `host`, `publish-homebrew-formula`, `announce` —
skipped, поскольку сборка не состоялась.

## Three distinct causes, not one flaky build

Существенно, что отказы **разной природы** — это не один дефект, который чинится одной правкой.

**C1 — prebuilt отсутствует физически.** Для `x86_64-apple-darwin` в `ort 2.0.0-rc.12`
prebuilt ONNX Runtime не публикуется вовсе. Собственное сообщение сборки предлагает два
пути: компилировать ONNX Runtime из исходников и линковать вручную, либо взять другой
backend (`ort-tract`). Ни то, ни другое не является настройкой — это смена архитектуры
зависимости.

**C2 — конфликт версий glibc (оба Linux).** Символы `__isoc23_strtol*` — семейство C23
`strtol`, появившееся в glibc 2.38. Prebuilt собран против нового glibc, а cargo-dist
линкует Linux-таргеты **внутри контейнера со старым glibc намеренно** — ради совместимости
с широким спектром дистрибутивов. То есть требование prebuilt прямо противоречит политике
переносимости, ради которой контейнер и введён. Отключить контейнер = сузить круг
поддерживаемых Linux-систем.

**C3 — несовместимость CRT (Windows).** 66 неразрешённых внешних символов, все — функции
C-runtime (`_dup`, `strncpy`, `modf`, `_timezone`, `fopen_s`, `_dupenv_s`). Prebuilt собран
против иного варианта/версии MSVC CRT, чем использует тулчейн раннера.

## What this establishes

1. **Вариант «просто включить фичу в дистрибутив» закрыт.** Не «рискован» — невозможен: при
   четырёх падающих таргетах из пяти workflow не публикует ничего.
2. **Исходная гипотеза о том, где сломается, была неверной.** Ожидался Windows (по
   прецеденту v0.32.0). Первым упал Linux, а самый жёсткий случай — Intel macOS, где
   prebuilt не существует в принципе. Догадка не заменила бы измерение.
3. **Отказ от фичи в дистрибутиве — не осторожность, а единственный работающий вариант**
   при текущей цепочке `fastembed → ort rc.12 → download-binaries`.
4. **`aarch64-apple-darwin` работает** — что согласуется с локальной сборкой на этой машине
   (67.2 MB против 47.4 MB без фичи, +41.8 %). Это оставляет открытым вариант отдельного
   артефакта под подмножество таргетов, но не решает задачу «работает из коробки везде».

## Limits of this evidence

- Измерена **одна** конфигурация: `ort 2.0.0-rc.12` с профилем `download-binaries`, как его
  подтягивает `fastembed 5.17.3`. Про `ort-load-dynamic`, `alternative-backend`/`ort-tract`
  или сборку ONNX из исходников это не говорит **ничего** — они не проверялись.
- Прогон единичный. Отказы C1–C3 детерминированы по природе (отсутствующий артефакт,
  версия glibc, набор символов CRT), так что повторение ожидаемо, но не доказано.
- Размер бинаря измерен только на Apple Silicon и только локальной сборкой; профили
  `dist` и локальный `release` не сверялись.

## Related

- PROB-088 — дефект, породивший вопрос
- PRD-083 — задача «измерить → решить → задокументировать»; это FR-002/FR-004
- ADR-022 — решение, принятое на основании этого измерения
- PR #455, run 33647382377



