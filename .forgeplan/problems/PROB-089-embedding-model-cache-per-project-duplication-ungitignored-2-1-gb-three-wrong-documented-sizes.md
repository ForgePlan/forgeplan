---
depth: standard
id: PROB-089
kind: problem
last_modified_at: 2026-09-02T14:47:26.113848+00:00
last_modified_by: claude-code/2.1.220
status: draft
title: 'Embedding model cache: per-project duplication, ungitignored 2.1 GB, three wrong documented sizes'
---

---
assigned_number: 89
predicted_number: 89
slug: prob-embedding-model-cache-per-project-duplication-ungitignored-2-1-gb-three
---

> GitHub: [#453](https://github.com/ForgePlan/forgeplan/issues/453) — найден при реализации
> [#452](https://github.com/ForgePlan/forgeplan/issues/452) / PRD-083, исправлен в той же ветке.

## Problem

Три самостоятельных дефекта вокруг кэша модели эмбеддингов. Все три проявляются только при
собранной фиче `semantic-search` — то есть ровно тогда, когда PROB-088 будет закрыт и
семантика поедет в дистрибутив. Без их устранения «поставил и пользуйся» ломается сразу
после первого успеха.

Это не подмножество PROB-088: тот про **отсутствие** фичи в сборке, этот — про поведение
фичи, когда она есть.

## D1 — кэш модели дублируется на каждый проект

`crates/forgeplan-core/src/embed/mod.rs` вызывал `InitOptions::new(model_enum)` без
`with_cache_dir`. Дефолт fastembed (`common.rs:12`) — `DEFAULT_CACHE_DIR = ".fastembed_cache"`,
**относительный путь от CWD процесса**. Каждый проект, где запускали `forgeplan embed`,
получал собственную полную копию весов.

Измерено на машине репортёра (2026-09-02):

```
2.1G  /Users/explosovebit/Work/ForgePlan/.fastembed_cache/models--BAAI--bge-m3
2.1G  /Users/explosovebit/Work/AeroNuts/.fastembed_cache
```

**4.2 GB на двух проектах.** Линейный рост: десять проектов — 21 GB.

## D2 — 2.1 GB не покрыты gitignore

`git status` в корне ForgePlan показывал `?? .fastembed_cache/`. Корневой `.gitignore`
перечислял `.forgeplan/.fastembed_cache/` (строки 15 и 106) и
`crates/forgeplan-core/.fastembed_cache/` (строка 33), но **не** корневой путь — тот самый,
куда fastembed реально пишет при запуске из корня репозитория.

Один `git add -A` отправил бы 2.1 GB бинарных весов в коммит. Для истории репозитория это
необратимо без rewrite.

Та же дыра тиражировалась пользователям: `GITIGNORE_CANONICAL_BODY`
(`crates/forgeplan-cli/src/commands/init.rs:408`) — шаблон, который `forgeplan init` пишет в
чужие проекты, — содержал ровно тот же неполный список.

## D3 — три места, три разных неверных размера

| Место | Заявлено | Реально |
|---|---|---|
| `README.md:229` | ~150 MB | 2.1 GB |
| `crates/forgeplan-core/src/health/mod.rs:1242` | ~600 MB | 2.1 GB |
| first-run UX | ничего не сообщалось | 2.1 GB |

Расхождение в 14 раз в README. Пользователь, прочитавший «~150 MB», получает загрузку в
четырнадцать раз больше — без предупреждения, потому что уведомления о старте загрузки не
существовало вовсе.

## Fix (реализовано в этой же ветке)

- **D1** → `embed::resolve_cache_dir()`: `FORGEPLAN_MODEL_CACHE` → платформенный
  user-cache (`~/Library/Caches/forgeplan/models`, `~/.cache/forgeplan/models`,
  `%LOCALAPPDATA%\forgeplan\models`) → фоллбэк на прежний путь, если платформенный каталог
  недоступен. Передаётся в `InitOptions::with_cache_dir`.
- **D2** → корневой `.gitignore` получил `.fastembed_cache/` без ведущего слэша (матч на
  любой глубине); тот же паттерн добавлен в `GITIGNORE_CANONICAL_BODY` и в
  `GITIGNORE_DRIFT_PATTERNS` (парная правка, которой требует комментарий в `init.rs:404-407`).
- **D3** → единственный источник `embed::MODEL_DOWNLOAD_SIZE_HINT`; README и
  drift-детектор ссылаются на измеренную цифру.
- **UX** → `embed::first_run_notice()` печатает модель, размер и целевой каталог **до**
  начала загрузки; молчит, когда кэш уже наполнен. Ошибка инициализации оборачивается в
  сообщение про сеть/диск вместо сырой ошибки fastembed.

Проверено: 6 unit-тестов на резолвер и уведомление (`embed::cache_dir_tests`) — зелёные;
`cargo clippy --workspace --all-targets --features semantic-search -- -D warnings` — exit 0;
`cargo fmt --check` — чисто.

## Оговорка, которую нельзя терять

`HF_HOME` в fastembed перебивает переданный `with_cache_dir`
(`fastembed/src/common.rs::pull_from_hf`, задокументировано в `fastembed/src/lib.rs:21`). Если
у пользователя выставлен `HF_HOME`, модель ляжет туда, а наш резолвер окажется советующим,
а не решающим. Это осознанно не переопределяется: общий HuggingFace-кэш — разумная
системная конвенция, и ломать её ради единообразия неправильно. Задокументировано в README
и на сайте.

## Миграция существующих установок

Старые копии **не переносятся автоматически** — перемещать гигабайты без спроса библиотека
не вправе. Вместо этого `first_run_notice()` обнаруживает локальный кэш в CWD и печатает
готовую команду `mv`, чтобы пользователь не качал третью копию. У репортёра на диске сейчас
4.2 GB в старых путях, которые после мержа станут мусором.

## Related

- PROB-088 — отсутствие фичи в дистрибутиве, GitHub #451; этот дефект становится видимым,
  когда тот закрыт
- PRD-083 — задача на решение, покрывает оба, GitHub #452
- `dist-workspace.toml` — включение фичи сделает D1/D2/D3 массовыми

