# research/ — первоисточники deep-research по ForgeFarm

Пять уникальных отчётов, полученных из внешних deep-research сессий
(ChatGPT/Gemini Deep Research). Скопированы из `dev/deeP-researches/`
2026-07-02 с переименованием; **оригиналы не тронуты**.

> **Правило для агентов**: файлы здесь — read-only первоисточники.
> Не редактировать. Выводы и решения живут в `../synthesis/` и
> `../architecture/`; сюда ходить только для сверки с оригиналом.

## Провенанс

| Файл здесь | Оригинал в `dev/deeP-researches/` | Размер | Суть |
|---|---|---|---|
| `R1-architecture-audit.md` | `Deep Research Report - ForgePlan (1).md` | 42 KB | Архитектурный аудит идеи + 6 недодуманных зон (слои истины, машинные статусы, контракт planning↔coding, конкуренция/изоляция, security, память) |
| `R2-production-stack.md` | `Deep Research Report - ForgePlan (2).md` | 49 KB | Production-стек: Temporal + LangGraph, Postgres + pgvector, K8s + ArgoCD + OTel; ForgeFarm = orchestrator/UI/projection plane |
| `R3-rust-first-control-plane.md` | `Deep Research Report - ForgePlan (3).md` | 66 KB | Самый большой отчёт: Rust-first детерминированный control plane + pluggable execution plane (OpenCode, Codex CLI, LangGraph/Deep Agents, Mastra, VoltAgent); L0–L3 как контракт ролей; MVP-команда 6 ролей |
| `R4-forgejo-plansform-integration.md` | `Отчет по глубоким исследованиям.md` | 34 KB | Интеграция ForgePlan + Forgejo + оркестратор («Forge Plansform»): git-native control plane, предостережения по Forgejo API |
| `R5-sdd-scheme-normalization.md` | `mm.md` | 7 KB | Нормализация чужой SDD-схемы (Gitea = task ledger, agent-session.sh + OpenCode, L0/L1/L2) в термины ForgeFarm |

## Известные особенности файлов

- **Дубликат**: `Deep Research Report - ForgePlan.md` (без номера) в исходной
  папке **байт-в-байт идентичен** `(3)` (md5 `0f7f039d…`). Здесь лежит один
  экземпляр — `R3`.
- **Отсутствующие ассеты**: `R5` ссылается на диаграммы
  (`sandbox:/mnt/data/forgefarm-sdd-architecture.{png,svg}`,
  `…-sequence.{png,svg}`) — эти файлы **не были выгружены** из сессии и
  локально отсутствуют. Если найдутся — класть в `research/assets/`.
- **`citeturn…` / `fileciteturn…` маркеры** в тексте — артефакты цитирования
  deep-research движка, не содержание. При чтении игнорировать.
- `R3` упоминает «присланные SDD-слайды» (`fileciteturn0file0`) — сами слайды
  в корпус не входят; их тезисы пересказаны в тексте R3.

## Как эти отчёты соотносятся

Все пять сходятся в главном (см. `../synthesis/01-consensus.md`), но дают
**разные ставки на стек**: R2 — управляемая инфраструктура
(Temporal/LangGraph/K8s), R3 — Rust-first ядро с адаптерами. Разбор
противоречий и рекомендации — `../synthesis/02-open-decisions.md`.
