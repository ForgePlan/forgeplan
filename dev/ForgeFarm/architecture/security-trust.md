# Security и trust boundaries

> Модель доверия ForgeFarm: где проходят границы, какие права у кого, что
> считается привилегированной операцией. Источники: R1/R4 (Forgejo Actions
> security), R3 (runner isolation, branch protection, bot permissions),
> R2 (write-таксономия), консенсус C3.

## 1. Главный вектор: untrusted код × write-возможности

Документация форджей прямо предупреждает: **`pull_request_target` исполняется в контексте базовой ветки с write-capable токеном и доступом к секретам** — взаимодействие с untrusted PR-кодом (форки) в этом контексте — готовый вектор компрометации. Обычный `pull_request` от форка, напротив, идёт без секретов и с урезанным токеном.

Правила:
- untrusted код исполняется ТОЛЬКО в sandbox/worktrees без секретов;
- критичная write-автоматизация (scheduling, queueing, assignment) триггерится только от `issues` events или `workflow_dispatch` (`POST /repos/{owner}/{repo}/actions/workflows/{name}/dispatches`), никогда от непрерывного потока внешних PR;
- merge — только через verified gates оркестратора;
- секреты — только в trusted branches/jobs.

## 2. Раннеры = RCE boundary

Раннер CI — это удалённое исполнение кода by design. Правила (R3):
- никакого host execution mode по умолчанию;
- никаких privileged containers без выделенного security sign-off;
- job network isolation (`network=host` — серьёзный риск хоста/интранета);
- отдельные раннеры/labels/groups для рискованных или чувствительных репозиториев.

## 3. Таксономия записей (R2)

| Класс | Примеры | Требование |
|---|---|---|
| **read** | чтение репо, артефактов, issues | scoped token |
| **safe-write** | комментарий, label-зеркало, draft-артефакт через MCP | audit_event |
| **privileged-write** | merge, branch-protection override, deploy, secret rotation, schema migration, финальный emit map.json, деструктивные правки, mass updates | **human approval ИЛИ deterministic-guardian pass + policy rule**; всегда audit |

Single-writer правило для критичных emitted-артефактов (map.json) **старше любых других write-прав**: даже оркестратору запрещено «чуть подправить».

## 4. Идентичности и токены

- Оркестраторский бот — минимальные права: comment/labels/branch/PR; без admin, пока не доказана необходимость.
- **Агенты никогда не говорят с трекером напрямую** — только через service account control plane.
- Webhook ingest — только с валидацией подписи; scoped tokens.
- Cloud deploy: GitHub Actions OIDC-only с короткоживущими токенами; на Forgejo без OIDC — минимально скоуплённые robot tokens с коротким TTL и ротацией.

## 5. Branch protection (полный набор, R3)

Запрет force-push · обязательный PR · required status checks · conversation resolution · signed commits · dismissal stale reviews · ограниченный список pusher'ов. Поддерживается и GitHub, и Forgejo.

## 6. Trust-границы внутри ForgeFarm

- **Control plane доверен, runtime plane — нет:** любые события от агента (RunEvents) — данные, не команды; переходы состояния валидируются policy engine.
- **Записи агента ограничены scope lease:** попытка записи вне claimed paths (`file_write_attempted` вне scope) — policy violation → fail-loop, не «ну ладно».
- **Артефакты — только через fpl CLI/MCP** (Artifact Gateway): у агентов нет прямого write-доступа к `.forgeplan/` markdown и тем более к LanceDB.
- **Худший сценарий должен быть ограничен worktree:** скомпрометированный/галлюцинирующий агент максимум портит свой worktree и свой брэнч — не main, не артефакты, не чужие worktrees. Это свойство архитектуры (leases + gates + branch protection), а не надежда.

## 7. Чего НЕ строить сейчас

Полный security-hardening (SLSA/Cosign supply chain, multi-tenant auth, OpenFGA, Governance Console) — Phase 5 по flip-сигналам. На MVP достаточно: signed webhooks, scoped bot token, branch protection на main, sandbox worktrees, три fail-closed правила записи (hook + runtime policy + CI) для критичных артефактов.
