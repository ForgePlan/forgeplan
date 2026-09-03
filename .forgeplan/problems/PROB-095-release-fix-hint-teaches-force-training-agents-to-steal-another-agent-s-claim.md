---
depth: tactical
id: PROB-095
kind: problem
links:
- target: PROB-094
  relation: references
status: draft
title: release Fix-hint teaches --force, training agents to steal another agent's claim
---

---
assigned_number: 95
predicted_number: 95
slug: prob-release-fix-hint-teaches-force-training-agents-to-steal-another-agent-s
---

## Problem

Два дефекта в координации агентов, второй серьёзнее первого.

**Документированный сценарий не работает.** CLAUDE.md описывает цикл так:

```
forgeplan claim PRD-NNN --agent <subagent-name> --ttl-minutes 60
# … работа …
forgeplan release PRD-NNN
```

Дословное выполнение падает: `release` по умолчанию представляется как
`cli/<version>`, а не как захвативший агент, и отказывает по несовпадению
владельца.

**Подсказка `Fix:` учит отбирать чужой захват.** На этом отказе выдаётся:

```
Error: Claim on PRD-001 held by rust-pro, not by requester
Fix: forgeplan release PRD-001 --force
```

`--force` — это «orchestrator override», снятие блокировки **независимо от
владельца**. По PRD-071 агент обязан выполнять `Fix:` как есть. То есть
контракт подсказок буквально инструктирует агента ломать координацию, о
которую он только что споткнулся.

Правильное действие — назваться собой: `forgeplan release PRD-001 --agent
rust-pro`. Оно работает и в подсказке не упоминается.

## Reproducer

```
$ forgeplan claim PRD-001 --agent rust-pro --ttl-minutes 60
Claimed PRD-001 for rust-pro

$ forgeplan release PRD-001                    # как написано в CLAUDE.md
Error: Claim on PRD-001 held by rust-pro, not by requester
Fix: forgeplan release PRD-001 --force         # ← учит отобрать

$ forgeplan release PRD-001 --agent rust-pro   # что надо было предложить
Released claim on PRD-001
```

## Что при этом работает — важно не потерять

Сама блокировка **исправна**. Проверено: пока `rust-pro` держит PRD-001, ни
`agent-A`, ни `agent-B` захватить его не могут, отказ внятный и с TTL. Это
не «координация сломана» — это «исправный замок с инструкцией его выламывать».

`--force` тоже работает как заявлено и честно помечает результат
«(forced — orchestrator override)». Проблема не в существовании escape hatch,
а в том, что он предлагается как **первый** ответ рядовому агенту.

## Why it matters

Multi-agent — это место, где тихий отказ дороже всего: агенты работают
параллельно и не видят друг друга. Механизм, который защищает их от
затирания чужой работы, сам подсказывает, как его обойти. Достаточно одного
послушного агента, чтобы двое начали писать в один артефакт.

## Options

**(a) Починить подсказку** — при несовпадении владельца предлагать
`release <id> --agent <holder>`, а `--force` оставить как `Or:`, то есть
явную альтернативу, а не основное действие. Минимально и точечно.

**(b) Починить CLAUDE.md** — записать `--agent` в документированный цикл.
Нужно в любом случае, но само по себе не спасает: подсказку читает агент,
уже получивший отказ.

**(c) Наследовать identity** — чтобы `release` по умолчанию брал ту же
identity, что и `claim` в этой сессии. Устраняет класс, но требует понять,
откуда identity берётся, и не сломать оркестратор, который releases от чужого
имени намеренно.

(a) и (b) выглядят обязательными; (c) — предмет отдельного решения.

## Related

- **PRD-057** — multi-agent dispatcher, чей это цикл
- **PRD-071** — контракт подсказок, требующий, чтобы `Fix:` был исполним;
  здесь он исполним и при этом вреден
- **PROB-094** — соседний дефект документации в том же CLAUDE.md


