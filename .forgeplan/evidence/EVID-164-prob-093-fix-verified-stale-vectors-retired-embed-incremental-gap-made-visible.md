---
depth: tactical
id: EVID-164
kind: evidence
links:
- target: PROB-093
  relation: informs
status: draft
title: 'PROB-093 fix verified: stale vectors retired, embed incremental, gap made visible'
---

---
assigned_number: 164
predicted_number: 164
slug: evid-prob-093-fix-verified-stale-vectors-retired-embed-incremental-gap-made
---

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement

base_sha: 1804f8f6
result_sha: b69af837
changed_paths: crates/forgeplan-core/src/db/store.rs, crates/forgeplan-core/src/db/convert.rs, crates/forgeplan-cli/src/commands/embed.rs, crates/forgeplan-cli/src/commands/search.rs, crates/forgeplan-core/src/projection/mod.rs

## Что проверялось

Три утверждения PROB-093, каждое на реальном бинаре, а не на unit-тестах.

## 1. Врущий вектор снят

Заметка, тело переписано с навигации подводных лодок на хлебопечение.

| Запрос | До фикса | После фикса |
|---|---|---|
| «underwater vessel beneath ice» — содержимого **нет** | 0.80 | **нет результатов** → после `embed` 0.67 |
| «bread dough yeast fermentation» — содержимое **есть** | 0.62 | **0.81** |

До фикса артефакт находился по удалённому тексту **лучше**, чем по актуальному,
и оценка по удалённому не менялась ни на сотую — то есть вектор не
пересчитывался вовсе. После фикса порядок обратный и правильный.

## 2. `embed` инкрементальный

```
1) embed:        Done: 1 embedded, 0 already current, 0 failed.
2) embed again:  Done: 0 embedded, 1 already current, 0 failed.
3) после правки: Done: 1 embedded, 0 already current, 0 failed.
```

Прежде каждый прогон пересчитывал все записи: 13 мин 18 с на 403 артефакта
(EVID-160). Теперь цена появления одного нового артефакта — один вызов
инференса вместо четырёхсот.

## 3. Разрыв стал видимым

Воркспейс из трёх заметок, одна проиндексирована:

```
  0.79  NOTE-001 [note] "Sourdough bread fermentation"
  2 artifact(s) have no embedding and cannot appear in semantic results.
```

JSON несёт `unindexed_artifacts: 2`. После `embed` — предупреждения нет,
поле 0, все три находятся.

## Мутационная проверка

Тест `update_body_retires_the_vector_of_the_previous_text` проверен снятием
строки инвалидации: падает на нужном утверждении. Тест, который только
проходит, ничего не доказывает.

## Тесты

3283 прошли, 0 упали (`--test-threads=1`; параллельный режим даёт известную
гонку `git::tests` из #454 — 51/51 в изоляции). clippy 0 предупреждений на
обеих конфигурациях фич, fmt чисто.

## Чего этот замер НЕ покрывает

- **Путь с фичей не проверяется в CI.** `search --semantic` и `embed` живут
  за `semantic-search`, который CI для тестов не собирает. Всё выше — локальный
  прогон, того же статуса, что и оракул эмбеддингов. Стенд помечает эти
  команды как EXTERNAL, а не как пройденные.
- **Миграция существующих воркспейсов** проверена только рассуждением: у них
  есть векторы и нет хэшей, значит первый `embed` пересчитает всё один раз и
  проставит хэши. На 400+ артефактах это не прогонялось.

## Related

- **PROB-093** — дефект
- **EVID-160** — замер 13м18с, из которого следовала цена полного пересчёта


