---
title: "forgeplan constellation pack"
description: "Stage 6 — build a budgeted ContextPack for a task: lexical ranking, evidence units with roles, omissions and a spend receipt"
---

## Использование

Команда выбирает лексически подходящие артефакты и их явно объявленные
основания, свидетельства, противоречия и замены. Текст каждой выданной
единицы сохраняется целиком вместе с источником, версией, хешем и диапазоном.

| Параметр | По умолчанию | Назначение |
|---|---:|---|
| `--seed <KEY[@SOURCE]>` | нет | Явный артефакт; можно указать несколько раз |
| `--budget <N>` | 8000 | Оценка полного ответа: UTF-8 байты / 4 с округлением вверх |
| `--byte-cap <N>` | 1000000 | Точный предел байтов ответа, включая перевод строки |
| `--max-depth <N>` | 4 | Глубина связей, не более 64 |
| `--max-nodes <N>` | 256 | Число посещённых артефактов, не более 4096 |
| `--max-edges <N>` | 1024 | Число рассмотренных связей, не более 16384 |
| `--max-millis <N>` | 5000 | Время обхода, не более 60000 мс |
| `--require-complete` | false | Отказ при неполных источниках, обходе или выдаче |
| `--cursor <TOKEN>` | нет | Следующая страница с прежними параметрами |
| `--retrieval-mode <MODE>` | lexical | Поддерживается только `lexical` |
| `--mermaid` | false | Навигация по графу страницы |
| `--snapshot <ID>` | нет | Зафиксированные Git-ревизии вместо текущих файлов |
| `--allow-partial` | false | Разрешить отказ обязательного источника |
| `--json` | false | Компактный структурированный JSON |

Полный ответ ограничен меньшим из `--byte-cap` и четырёх `--budget`.
Если целая единица не помещается, увеличьте оба предела; обрезанный текст
не возвращается. `next_cursor` продолжает ту же выдачу, но не возобновляет
обход, остановленный пределом. Поля полноты и графа описаны в
[справке ContextPack](/ru/docs/mcp/constellation_pack/).

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation pack --help`.

```text
Stage 6 — build a budgeted ContextPack for a task: lexical ranking, evidence units with roles, omissions and a spend receipt

Usage: forgeplan constellation pack [OPTIONS] <TASK>

Arguments:
  <TASK>  What the agent must do — the ranking terms come from this text

Options:
      --budget <BUDGET>
          Estimated token allowance for the complete response (UTF-8 bytes / 4) [default: 8000]
      --byte-cap <BYTE_CAP>
          Exact maximum bytes in the complete output [default: 1000000]
      --seed <SEED>
          Explicit artifact selector, optionally qualified as KEY@source
      --max-depth <MAX_DEPTH>
          Maximum declared relation depth [default: 4]
      --max-nodes <MAX_NODES>
          Maximum visited nodes [default: 256]
      --max-edges <MAX_EDGES>
          Maximum traversed edges [default: 1024]
      --max-millis <MAX_MILLIS>
          Maximum traversal time in milliseconds [default: 5000]
      --require-complete
          Refuse incomplete sources, obligations or page delivery
      --cursor <CURSOR>
          Continue a previous response on the same basis and presentation
      --retrieval-mode <RETRIEVAL_MODE>
          Retrieval mode (only lexical is supported) [default: lexical]
      --mermaid
          Include Mermaid graph navigation
      --snapshot <SNAPSHOT>
          Pack at a recorded snapshot instead of the live sources
      --allow-partial
          Tolerate dead sources: even a required source's outage becomes a `failures` entry on a partial answer instead of an error
      --json
          Output as JSON
  -h, --help
          Print help
  -V, --version
          Print version
```
<!-- forgeplan-runtime:end -->

## Примеры

- [Воспроизводимый сценарий Constellation](/ru/docs/guides/constellation/)
- [Быстрый старт с артефактами](/ru/docs/getting-started/quick-start/)

## См. также

- [CLI reference](/ru/docs/cli/)
- [Constellation reference](/ru/docs/reference/constellation/)
