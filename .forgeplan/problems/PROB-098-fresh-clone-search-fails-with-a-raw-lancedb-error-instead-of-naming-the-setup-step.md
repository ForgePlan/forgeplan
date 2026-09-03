---
depth: tactical
id: PROB-098
kind: problem
links:
- target: PRD-085
  relation: references
status: draft
title: 'Fresh clone: search fails with a raw LanceDB error instead of naming the setup step'
---

---
assigned_number: 98
predicted_number: 98
slug: prob-fresh-clone-search-fails-with-a-raw-lancedb-error-instead-of-naming-the
---

## Problem

Первое, что делает новый участник команды — клонирует репозиторий и что-нибудь
спрашивает. Он получает внутреннюю ошибку базы:

```
$ git clone <repo> && cd <repo>
$ forgeplan search "auth" --semantic
Error: Table 'artifacts' was not found

Caused by:
...
```

`.forgeplan/lance/` в gitignore (правильно, ADR-003: индекс производный). Но
пользователю сообщают об отсутствующей таблице LanceDB, а не о том, что нужно
сделать.

Задокументированный путь существует — `git clone → forgeplan init -y →
forgeplan reindex` в CLAUDE.md, — но он не в сообщении, а сообщение приходит
первым.

## Reproducer

Проверено на двух реальных клонах через bare-репозиторий:

```
alice$ forgeplan init -y && forgeplan new note "..." && forgeplan embed
alice$ git add -A && git commit && git push

bob$   git clone <origin> && cd bob
bob$   forgeplan search "underwater vessel" --semantic
Error: Table 'artifacts' was not found
```

Markdown у Bob на месте (2 файла), не хватает только производного индекса.

## Why it matters

Нарушен контракт PRD-071: сообщение об ошибке обязано нести исполнимый
`Fix:`. Здесь нет ни `Fix:`, ни `Next:` — только протечка внутреннего слоя
хранилища.

Отдельно: это **первое впечатление**. Человек может догадаться посмотреть
README. Агент, получив «Table 'artifacts' was not found», с большой
вероятностью сообщит, что forgeplan сломан.

## Goals

- Отсутствие производного индекса опознаётся как таковое и сообщается вместе
  с командой, которая его создаёт.
- Внутренние ошибки LanceDB не доходят до пользователя как есть.

## Non-Goals

- Не создавать индекс автоматически при первом чтении: это скрытая запись из
  read-only команды, и она не решает случай, когда нужен ещё и `embed`.
- Не класть `lance/` в git.

## Options (не решено)

**(a) Перехватывать «table not found» на уровне открытия store** и заменять
на сообщение с `Fix: forgeplan init -y && forgeplan reindex`. Точечно,
но ловит по строке ошибки — хрупко к смене формулировки в LanceDB.

**(b) Проверять наличие `.forgeplan/lance/` до обращения к store.** Явная
проверка вместо разбора чужого текста ошибки. Надёжнее.

**(c) Отдельный `forgeplan doctor`**, который диагностирует состояние
воркспейса. Полезно и шире, но не убирает плохое первое сообщение.

(b) выглядит правильнее (a) — состояние проверяется по факту, а не по строке.

## Related

- **PROB-097** — соседний разрыв: `pull` оставляет индекс устаревшим
- **PRD-071** — контракт подсказок, требующий исполнимый `Fix:`
- **ADR-003** — markdown источник истины, индекс производный



