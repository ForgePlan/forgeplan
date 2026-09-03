---
depth: tactical
id: PROB-096
kind: problem
links:
- target: PROB-093
  relation: references
status: draft
title: 'forgeplan reason unusable with the default claude-code provider: 120s budget is hardcoded'
---

---
assigned_number: 96
predicted_number: 96
slug: prob-forgeplan-reason-unusable-with-the-default-claude-code-provider-120s
---

## Problem

`forgeplan reason` — шаг ADI, который методология называет **обязательным**
для глубины Deep и Critical — не отрабатывает с провайдером, настроенным в
этом репозитории по умолчанию.

```
$ forgeplan reason PROB-093
  Analyzing PROB-093 with ADI cycle (claude-code/claude-opus-4-8)...
Error: ADI reasoning failed: claude-code provider: `claude --print` timed out after 120s.
```

Бюджет в 120 с **захардкожен и намеренно не настраивается** в релизных
сборках — `llm/mod.rs:289-297`:

```rust
fn config_timeout(&self) -> std::time::Duration {
    #[cfg(test)]
    if let Ok(ms) = std::env::var("FORGEPLAN_CLAUDE_CODE_TIMEOUT_MS") ...
    std::time::Duration::from_secs(120)
}
```

Комментарий рядом объясняет решение: «the 120s production budget is not
configurable, mirroring the binary-resolution `#[cfg(test)]` gate discipline
(no prod behavior driven by env)». Основание разумное — не давать окружению
менять поведение продакшена. Но следствие в том, что для полного ADI-прохода
крупной моделью этого бюджета не хватает, а поднять его нечем.

## Reproducer

В этом репозитории (`.forgeplan/config.yaml`: `provider: claude-code`,
`model: claude-opus-4-8`):

```
$ forgeplan reason PROB-093
Error: ADI reasoning failed: claude-code provider: `claude --print` timed out after 120s.
Error: LLM call failed
```

Воспроизводится стабильно, не разово.

## Why it matters

Методология требует ADI на Deep/Critical и ставит его в полный цикл между
валидацией и кодом. То есть обязательный шаг **невыполним** при штатной
конфигурации, и обойти его можно только пропустив — что ровно и происходит
на практике.

Отдельно: отказ выглядит как отказ (это хорошо, не тихий), но `Fix:`
предлагает проверить `GEMINI_API_KEY` и блок `llm:` — а конфигурация как раз
корректна. Подсказка уводит от настоящей причины, потому что причина —
таймаут, а не конфигурация.

## Не проверено

- **Отрабатывает ли `reason` с HTTP-провайдером** (OpenAI-совместимым) в
  пределах тех же 120 с. У того же клиента такой же бюджет
  (`llm/mod.rs:115`), но модель и латентность другие. Пока не измерено —
  считать, что проблема только у `claude-code`, преждевременно.
- Сколько времени полный ADI-проход занимает **на самом деле** — замера нет,
  есть только факт превышения 120 с.

## Options

**(a) Замерить сначала.** Сколько нужно на реальный ADI-проход у обоих
провайдеров. Без этого числа любой новый таймаут — такая же догадка, как
текущий.

**(b) Развести бюджеты по типу вызова.** Короткие вызовы (route, capture) и
длинный ADI — разные операции, один общий бюджет им обеим не подходит.

**(c) Сделать бюджет настраиваемым через `config.yaml`, а не через env.**
Это сохраняет исходный принцип («no prod behavior driven by env»), потому что
конфиг — часть проекта и лежит в git, а не в чужой оболочке.

(c) выглядит совместимым с обоснованием, которое запретило env. Но начинать
надо с (a).

## Related

- **PRD-020** — LLM-first routing, где живёт провайдерный слой
- **PROB-093**, **PROB-094**, **PROB-095** — найдены в той же ревизии
  поверхностей CLI


