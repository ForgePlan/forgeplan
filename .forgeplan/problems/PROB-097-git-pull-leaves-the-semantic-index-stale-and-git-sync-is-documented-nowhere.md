---
depth: tactical
id: PROB-097
kind: problem
links:
- target: PRD-085
  relation: references
status: deprecated
title: git pull leaves the semantic index stale and git-sync is documented nowhere
---

---
assigned_number: 97
predicted_number: 97
slug: prob-git-pull-leaves-the-semantic-index-stale-and-git-sync-is-documented-nowhere
---

## Problem

`git pull` меняет markdown на диске и **не трогает LanceDB**. До запуска
`forgeplan git-sync` поиск продолжает отвечать по состоянию **до** pull —
включая семантику, где остаётся вектор прежнего текста.

Команда, снимающая это, существует и работает. Её проблема в другом: **она не
упомянута ни в одном месте, куда смотрит человек или агент.**

Проверено grep'ом: `git-sync` отсутствует в `CLAUDE.md`, в `docs/README.md`,
в `docs/operations/*.md` и в отгружаемом скилле `/forge`
(`crates/forgeplan-cli/src/commands/forge-skill.md` — 0 совпадений). Есть
только `--help` и справочная страница сайта, то есть места, куда идут, уже
зная, что искать.

## Reproducer (два клона, реальный git)

Alice создаёт NOTE-001 про подводные лодки, индексирует, пушит. Bob клонирует,
делает `init` + `reindex` + `embed`. Затем Alice переписывает тело на
хлебопечение и добавляет NOTE-003, пушит.

```
bob$ git pull
bob$ forgeplan search "underwater vessel beneath ice" --semantic
  0.81  NOTE-001 [note] "Polar submarine navigation"     ← содержимого УЖЕ НЕТ
```

Bob получает **правдоподобный ответ по тексту, которого в артефакте больше не
существует**, и никакого признака, что что-то не так.

После `git-sync` всё встаёт на место:

```
bob$ forgeplan git-sync
Git sync complete: 2 synced, 0 deleted, 0 errors

bob$ forgeplan search "underwater vessel beneath ice" --semantic
  2 artifact(s) have no embedding and cannot appear in semantic results.
  Run `forgeplan embed` — it now only encodes what changed.

bob$ forgeplan embed
Done: 2 embedded, 1 already current, 0 failed.       ← платит только за изменившееся
```

## Почему это не то же самое, что PROB-093

PROB-093 закрыл путь **CLI**: `forgeplan update` снимает устаревший вектор,
потому что идёт через `store::update_body`. `git-sync` и `reindex` идут через
`projection::sync_body_from_file`, который вызывает тот же `update_body` — то
есть фикс работает и там, **если эти команды запустить**.

Дефект ровно в этом «если». Одиночный пользователь правит через CLI и
защищён. Команда правит через git — и защита не срабатывает, потому что
никто не сказал, что нужен дополнительный шаг.

То есть класс тот же — молчаливый устаревший ответ, — но входит он через
дверь, которую PROB-093 не закрывал.

## Что делает это хуже в командной работе

- **Частота.** Одиночная правка — событие. `git pull` — рутина, десятки раз в день.
- **Ветки.** Переключение веток меняет markdown так же, как pull. Тот же разрыв.
- **Агенты.** Агент планирует по выдаче поиска. Получив содержимое чужой ветки
  или доpull'ное состояние, он не удивится.

## Goals

- После `pull` / `checkout` / `merge` индекс приходит в соответствие с диском
  без того, чтобы пользователь помнил про отдельную команду.
- Если автоматизировать нельзя — расхождение должно быть **видимым**, а не
  молчаливым.

## Non-Goals

- Не класть `.forgeplan/lance/` в git. Это производный индекс (ADR-003), и
  бинарные конфликты в нём хуже, чем пересборка.
- Не считать эмбеддинги автоматически на каждый pull: 2 с на артефакт, и
  качать 2.1 GB модель в CI никто не просил.

## Options (не решено)

**(a) Задокументировать.** Минимум: `git-sync` в CLAUDE.md, в скилле и в
онбординге. Дёшево, но полагается на память — ровно то, что уже не сработало
с `forgeplan embed`.

**(b) Ленивая проверка при чтении.** `search` / `list` / `get` сравнивают
mtime markdown с `updated_at` в индексе и сообщают о расхождении. Не чинит,
но превращает молчание в сообщение. Дёшево и совместимо с (a).

**(c) Git hook.** `post-merge` / `post-checkout`, ставится через
`forgeplan init`. Закрывает по построению, но лезет в чужой `.git/hooks` и
ломается при worktree и при `--no-verify`.

**(d) Автосинк внутри команд.** `search` и подобные сами вызывают git-sync,
если HEAD сдвинулся с прошлого запуска. Прозрачно для пользователя; цена —
неожиданная задержка и запись в БД из read-only команды.

Предварительно (b) выглядит обязательным независимо от остального: сначала
сделать разрыв видимым, потом спорить об автоматизации. Но это гипотеза —
нужен `forgeplan reason`.

## Related

- **PROB-093** — тот же класс на пути CLI; закрыт
- **PROB-098** — соседняя дыра в онбординге, найдена тем же прогоном
- **ADR-003** — markdown источник истины, LanceDB производный






