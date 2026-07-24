# Идеи из Herdr → ForgeFarm

> Herdr (herdr.dev) — терминальный мультиплексер с agent-awareness: semantic
> state detection (working / blocked-on-approve / done / idle) через
> process-detection + эвристики по output; поддержка 13 CLI-агентов из
> коробки; socket API + CLI, через которые агент сам открывает панели,
> запускает команды, читает чужой output и ждёт чужого done. Single binary,
> без Electron, живёт в SSH, AGPL. Владелец предложил «взять оттуда идеи».
> Этот документ: что берём, что отвергаем, и почему.

## Ключевой контраст (почему Herdr ≠ ForgeFarm, но полезен)

**Herdr решает задачу НАБЛЮДЕНИЯ за агентами, у которых нет контракта:**
состояние выводится эвристиками по output, потому что tmux-панель ничего о
себе не сообщает. **ForgeFarm решает задачу УПРАВЛЕНИЯ агентами по
контракту:** состояние приходит типизированными RunEvents через
ExecutorDriver (`status_changed`, `heartbeat`, `gate_request`…), истина живёт
в projection DB, координация — через leases/DAG/gates. Поэтому Herdr-идеи
берутся на слое **UX и механики**, и отвергаются на слое **истины и
координации**.

## Что берём (5 идей)

### H-1. Эвристическая детекция состояния как fallback-канал ExecutorDriver

Herdr доказывает на 13 агентах: состояние CLI-агента (working / blocked /
done / idle) **надёжно выводится из PTY-output и process-состояния** даже без
контракта. Для ForgeFarm это второй источник состояния в ExecutorDriver:

- **контрактный канал** (primary): headless JSON-события (CC `stream-json`,
  Codex exec, OpenCode server) → типизированные RunEvents;
- **эвристический канал** (fallback): PTY-output классификация — для
  executors без чистого headless-режима, для интерактивных сессий и как
  **cross-check** контрактного канала (агент, чей контрактный статус
  `working`, но PTY молчит 10 минут — вход в reconcile как drift).

Это снимает жёсткую зависимость «поддерживаем только executors с идеальным
event-потоком» и даёт детектор зависших ранов, не зависящий от честности
самого агента.

### H-2. `blocked-on-approve` — главный сигнал оператора (подтверждение HAQ)

Killer-инсайт Herdr совпадает с нашим дизайном: главная боль оператора 3–5
параллельных агентов — «кто залип в ожидании yes». У ForgeFarm это уже
первоклассно (`awaiting_human` статус + Human Attention Queue) — Herdr
подтверждает приоритет и добавляет UX-требование: **этот сигнал должен быть
виден одним взглядом и доступен удалённо (SSH/телефон)**, а не закопан в
web-дашборде. Следствие → H-3.

### H-3. `ff top` — терминальный cockpit ДО web-UI

Herdr показывает: для solo-оператора терминальный state-bar закрывает 80%
потребности наблюдения без Electron/браузера. Мастер-синтез уже говорил «HAQ
пока просто таблица+CLI» (Phase 3) — усиливаем это до первоклассного
терминального вида:

- **`ff top`** — live-view поверх projection DB: раны × статусы × tier ×
  lease TTL × cost, строка HAQ сверху; работает по SSH;
- web Board + Run Inspector остаются Phase 4 — терминальный вид не
  заменяется, а остаётся быстрым операторским входом навсегда.

### H-4. Дешёвые wait/status/read примитивы для агентов — но через control plane

Socket API Herdr (`herdr wait agent-status 1-2 --status done`,
`herdr pane read 1-2`) — правильная **эргономика**: агенту нужны копеечные
однострочные примитивы координации. Но у Herdr агенты координируются
**peer-to-peer через сырой output** — мимо какой-либо семантики. Берём
эргономику, меняем субстрат:

```
ff run status <run-id> --json          # состояние рана из projection DB
ff wait run <run-id> --until done|awaiting_verifier --timeout 30m
ff runs list --task <id> | --mine      # чьи раны где
ff run events <run-id> --tail          # типизированные RunEvents (не сырой PTY)
ff attention list                      # HAQ из терминала
```

Ожидание чужого результата = `blockedBy` в DAG + `ff wait` как дешёвый
опрос/подписка. Правило: **агент никогда не читает сырой output другого
агента** («transcripts never cross stages» из map-pack дисциплины) — только
типизированные события и артефакты. Это же защищает от prompt-injection
через чужой вывод.

### H-5. Persistent sessions: раны переживают disconnect оператора

Herdr-свойство «сессии переживают disconnect» — требование к Runtime
Supervisor'у ForgeFarm: агентские процессы живут под супервизором
(detached PTY / process group), а не под терминалом оператора. SSH-обрыв
оператора не убивает ни один ран; `ff top` с любой машины показывает живую
картину. (Уже следует из «ForgeFarm = супервизор процессов», B6 — Herdr
делает требование явным.)

## Что НЕ берём

1. **Peer-to-peer координация через чтение чужого output** — анти-паттерн
   для ForgeFarm: мимо leases, мимо gates, мимо audit; плюс канал
   prompt-injection. Координация — только через control plane.
2. **Эвристики как ЕДИНСТВЕННЫЙ источник истины** — у Herdr нет выбора, у
   нас есть: контрактный канал primary, эвристика — fallback/cross-check.
3. **Терминал как runtime оркестрации** («агенты сами открывают панели и
   запускают команды») — у нас runtime = worktree+executor под супервизором;
   панель — это view, а не место исполнения.
4. **Herdr как компонент ядра** — AGPL (для коммерческого продукта нужна
   лицензия), эвристики дублируют наш контрактный слой, состояние у него
   локальное (не переживает reconcile против DB).

## Build-vs-buy: Herdr в prior-art список

Herdr встаёт **третьим** в prior-art extraction (рядом с gastown и
swarm-forge, см. `03-wsfold-bridge.md` §5), с фокусом:

| Что извлечь | Зачем |
|---|---|
| эвристики state-detection по output (какие сигналы у каких агентов) | fallback-канал H-1 |
| набор поддерживаемых агентов (13) и как детектится каждый | приоритизация ExecutorDriver-адаптеров |
| дизайн wait/read/status CLI-примитивов | эргономика `ff` CLI (H-4) |
| state-bar UX (что оператору нужно видеть одним взглядом) | `ff top` (H-3) |

При этом Herdr **можно использовать как есть** уже сейчас, до ForgeFarm:
операторское наблюдение за ручными мульти-агентными сессиями (Phase 0–2
bootstrap-режим) — он не конфликтует с будущим control plane, потому что
живёт на другом слое (терминальный view). Если приживётся — в Phase 3
воркеры ForgeFarm могут запускаться в herdr-панелях для visibility, пока
`ff top` не готов.

## Дельта к плану

| Куда | Что |
|---|---|
| `rfc-runtime-and-lease-model` (Phase 0) | ExecutorDriver: двухканальное состояние (контрактный + эвристический fallback/cross-check) |
| Phase 3 | `ff top` + `ff wait/status/events/attention` CLI-примитивы (вместо «просто таблица») |
| prior-art EVID (Phase 0) | + Herdr (третьим к gastown/swarm-forge) |
| Runtime Supervisor | detached-сессии: раны переживают disconnect оператора |
