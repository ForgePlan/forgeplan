---
depth: tactical
id: PROB-094
kind: problem
links:
- target: PROB-093
  relation: references
status: deprecated
title: CLAUDE.md documents 'recall --list' — the flag does not exist
---

---
assigned_number: 94
predicted_number: 94
slug: prob-claude-md-documents-recall-list-the-flag-does-not-exist
---

## Problem

Раздел «Cross-session memory» в **скилле `/forge`** описывает память как
key-value хранилище. Она им не является, и две из трёх приведённых команд
просто падают.

Файл — `crates/forgeplan-cli/src/commands/forge-skill.md`, исходник, который
`forgeplan setup-skill` ставит пользователю в `~/.claude/skills/forge/SKILL.md`.
То есть неверная инструкция **отгружается наружу**, а не лежит локально.

| Что написано в скилле | Что происходит |
|---|---|
| `forgeplan remember "key" "value"` | `error: unexpected argument 'value' found` |
| `forgeplan recall "key"` — «fetch previously stored» | Работает, но это **подстрочный поиск**, а не выборка по ключу |
| `forgeplan recall --list` — «show all keys» | `error: unexpected argument '--list' found` |

Реальные формы (проверены):

```
forgeplan remember "<текст>" --category <fact|convention|procedure|insight>
forgeplan recall "<запрос>"     # подстрока по title+body
forgeplan recall                # без запроса — перечисляет всё
```

## Уточнение исходной формулировки

Первая версия этой находки утверждала, что дефект в `CLAUDE.md`. Это неверно:
в `CLAUDE.md` репозитория такой строки нет, она в исходнике скилла. Разница
существенная в обе стороны — уже, потому что не касается файла, читаемого
каждый ход; шире, потому что скилл раздаётся всем, кто выполнил
`setup-skill`.

## Reproducer

```
$ forgeplan remember "arch-choice" "we picked tract"
error: unexpected argument 'we picked tract' found
Usage: forgeplan remember [OPTIONS] [TEXT]

$ forgeplan recall --list
error: unexpected argument '--list' found
```

Найдено стендом `scripts/cli-surface-exercise.sh`.

## Why it matters

Ошибочна не команда, а **модель**. Агент, прочитавший «save key-value pair»,
будет придумывать короткие ключи вроде `arch-choice`. Даже если он угадает
синтаксис, id выводится из текста (`mem-we-picked-tract-over-onnx`), и
короткая метка сделает память ненаходимой подстрочным поиском — то есть
формально сохранённой и практически потерянной.

## Ширина не измерена

Из скилла извлечены все 18 примеров команд; проверены три из раздела памяти,
потому что о них споткнулся стенд. **Остальные пятнадцать не проверялись.**
Отдельная задача — прогнать через стенд каждый пример команды из скилла и из
`CLAUDE.md`. Пока это не сделано, считать этот раздел единственным испорченным
нет оснований.

## Related

- **PROB-093**, **PROB-095**, **PROB-096** — найдены в той же ревизии
- **PRD-071** — контракт вывода для агентов; здесь нарушена его предпосылка:
  документация обещает интерфейс, которого нет




