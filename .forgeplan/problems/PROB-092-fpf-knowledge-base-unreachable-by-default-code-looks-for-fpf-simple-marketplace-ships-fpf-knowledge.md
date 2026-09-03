---
depth: tactical
id: PROB-092
kind: problem
status: deprecated
title: 'FPF knowledge base unreachable by default: code looks for fpf-simple, marketplace ships fpf-knowledge'
---

---
assigned_number: 92
predicted_number: 92
slug: prob-fpf-knowledge-base-unreachable-by-default-code-looks-for-fpf-simple
---

## Problem

`forgeplan fpf ingest` без `--path` не находит базу знаний **ни у кого**, потому что ищет её
по имени скилла, которого не существует:

```rust
// crates/forgeplan-core/src/fpf/knowledge.rs:180-184
pub fn default_fpf_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let path = PathBuf::from(home).join(".claude/skills/fpf-simple/sections");
    if path.exists() { Some(path) } else { None }
}
```

Маркетплейс поставляет скилл как **`fpf-knowledge`**
(`plugins/fpf/skills/fpf-knowledge/sections`, 21 секция, 5.2 MB). Каталога `fpf-simple` нет
нигде — ни в маркетплейсе, ни в `fpf-standalone-repo`.

Второе место с тем же устаревшим именем — текст подсказки:

```rust
// crates/forgeplan-cli/src/commands/fpf.rs:326
"  Source:    not found (set fpf.path in config or install fpf-simple skill)"
```

То есть пользователю советуют поставить скилл, которого не существует. Совет неисполним —
нарушение того же контракта хинтов (PRD-071), что уже чинилось в `embed.rs`.

## Как проявляется

```
$ forgeplan fpf ingest
Error: FPF spec not found. Use --path to specify location

$ forgeplan fpf search "trust calculus" --semantic
  No FPF sections match 'trust calculus'
  Hint: Run `forgeplan fpf ingest` first
```

Хинт отправляет на команду, которая не может отработать. Круг замкнут, и выйти из него можно
только зная про `--path` и зная правильный путь — а его нигде не написано.

## Почему это не заметили

`fpf search` **деградирует корректно**: без проиндексированных секций она сообщает, что
совпадений нет, и предлагает ingest. Ничего не падает, стек не рвётся. Тот же класс тихого
отказа, что и PROB-088: fallback работает, поэтому никто не спрашивает, почему им пользуются.

Обнаружено при верификации поверхностей после замены движка (EVID-162): `fpf search` была
единственной непроверенной, и попытка её проверить упёрлась в это.

## Что это не

**Не следствие RFC-013.** Замена движка эмбеддингов к резолву пути отношения не имеет — тот
же код с тем же именем скилла лежал до неё. Проверено чтением, а не предположением.

## Что нужно

1. **Починить резолвер** — искать `fpf-knowledge`. Лучше не одно имя, а список кандидатов:
   каталог мог переехать снова, а падать на этом второй раз не хочется.
2. **Исправить хинт** в `fpf.rs:326` на существующее имя скилла и на команду установки,
   которая реально ставит.
3. **Проверить `fpf.path` в конфиге** — хинт ссылается на эту настройку; если её нет,
   хинт врёт дважды.
4. **Задокументировать**, что векторный поиск по FPF требует отдельно установленной базы
   знаний — сейчас об этом не сказано ни в README, ни на сайте, ни в cookbook.

## Обходной путь

```bash
forgeplan fpf ingest --path <marketplace>/plugins/fpf/skills/fpf-knowledge/sections
```

Работает, но требует знать и путь, и то, что проблема вообще в пути.

## Related

- EVID-162 — верификация поверхностей, при которой дефект найден
- PRD-071 — контракт хинтов, нарушаемый строкой `fpf.rs:326`
- PROB-088 — тот же класс: корректный fallback, скрывающий неработающую функцию




