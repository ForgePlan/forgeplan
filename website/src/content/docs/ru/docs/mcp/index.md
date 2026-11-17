---
title: Инструменты MCP
description: "Справочник по 73 MCP-инструментам Forgeplan 0.38.0 и их фактическим схемам JSON."
---

Forgeplan 0.38.0 предоставляет **73 MCP-инструмента** через Model Context Protocol с транспортом stdio.

Запустите сервер и подключите к нему MCP-клиент:

```bash
forgeplan serve
```

## Источник контракта

Этот справочник основан на ответе **Forgeplan 0.38.0** на `tools/list`, не на сигнатурах обработчиков Rust. Имена JSON-полей, типы и обязательность берутся из `inputSchema`. Например, `workspace` необязателен там, где перечислен, и поддерживается не каждым инструментом. Для инструмента без такого аргумента запускайте сервер в нужном проекте.

Описание, объявленное инструментом, не доказывает выполнение функции. В 0.38.0 оба `forgeplan_interview_packet_*` возвращают `not_implemented` даже после установки marketplace-пакета. `forgeplan_discover_complete` возвращает сводку сессии и предлагает отдельно вызвать health. Подробности ограничений приведены на страницах инструментов.

`capture`, `generate`, `reason` и `decompose` требуют настроенного LLM-провайдера. Перечисление схем не является вызовом этих операций.

Успешные результаты обычно содержат JSON в текстовом содержимом MCP, а не универсальную оболочку `ok: true`. Отказы обработчиков — это результаты ошибок MCP (`isError: true`) с текстом; неверные параметры могут дать ошибку JSON-RPC. Сведения трассировки пространства добавляют только те обработчики, где это реализовано.

`_next_action` и другие подсказки — рекомендательный текст, не автоматические команды и не согласие на деструктивную операцию. В некоторых подсказках 0.38.0 смешаны синтаксис CLI и имена MCP либо упомянуты незарегистрированные инструменты. Используйте зарегистрированное имя с JSON-аргументами из справочника или проверяйте релизный CLI help. Не обходите гейты, не перезаписывайте данные и не удаляйте конфликтующий артефакт автоматически по подсказке.

### Снять tools/list без API-ключей

Для получения схем не нужны LLM-вызовы или учётные данные провайдера. При наличии релизного `forgeplan` в PATH этот пример на стандартной библиотеке Python запускает сервер во временном каталоге, выполняет инициализацию MCP, сохраняет список инструментов и завершает дочерний процесс. Он не вызывает инструменты рабочего пространства или генерации.

```python
import json
import subprocess
import tempfile
from pathlib import Path

output = Path("tools-list.json").resolve()
with tempfile.TemporaryDirectory() as directory:
    server = subprocess.Popen(
        ["forgeplan", "serve"], cwd=directory,
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
    )
    def send(message):
        server.stdin.write(json.dumps(message) + "\n")
        server.stdin.flush()

    def receive(request_id):
        for line in server.stdout:
            message = json.loads(line)
            if message.get("id") == request_id:
                if "error" in message:
                    raise RuntimeError(message["error"])
                return message["result"]
        raise RuntimeError("MCP server closed before replying")

    try:
        send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2024-11-05", "capabilities": {},
            "clientInfo": {"name": "docs-snapshot", "version": "1.0"},
        }})
        initialized = receive(1)
        send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        tools = []
        cursor = None
        request_id = 2
        while True:
            send({"jsonrpc": "2.0", "id": request_id, "method": "tools/list",
                  "params": {} if cursor is None else {"cursor": cursor}})
            listing = receive(request_id)
            request_id += 1
            tools.extend(listing["tools"])
            cursor = listing.get("nextCursor")
            if not cursor:
                break
        capture = {"source": {"version": initialized["serverInfo"]["version"],
                              "initialize": initialized}, "tools": tools}
        output.write_text(json.dumps(capture, indent=2) + "\n", encoding="utf-8")
    finally:
        server.terminate()
        server.wait()
```

Из корня репозитория обновите английские разделы контракта:

```bash
node website/scripts/generate-mcp-docs.mjs --schema-file tools-list.json
```

Форма входа: `{ "source": { "version": "0.38.0" }, "tools": [...] }`. Версия должна быть SemVer, в том числе с необязательными prerelease/build-идентификаторами, например `0.39.0-rc.1`. Некорректные или пустые снимки и неподдерживаемые ключевые слова схемы отклоняются до изменения файлов. Генератор поддерживает подмножество схем из снимка, не произвольный JSON Schema: неизвестные ограничения, включая `minProperties`, `dependentRequired`, `exclusiveMinimum`, вызывают отказ; `format` считается аннотацией, не дополнительным правилом валидации.

По умолчанию снимок, пропускающий существующую страницу инструмента, отклоняется как потенциально неполный. Только при намеренном удалении инструмента используйте `--schema-file tools-list.json --prune`, разрешая удаление пропущенных английских страниц. Пустой снимок запрещён и с `--prune`. Неизменённые страницы не перезаписываются.

Таблицы входных полей и основные примеры вызова создаются заново; frontmatter, пояснения использования и результатов, ссылки сохраняются. Для новой страницы заголовок и описание берутся из метаданных и требуют содержательной проверки. При новом снимке отдельно актуализируйте эти пояснения и русский перевод. Генерация документации не проверяет поведение инструментов.

## Каталог инструментов

### Рабочее пространство и данные

| Инструмент | Описание |
|---|---|
| [`forgeplan_export`](/ru/docs/mcp/forgeplan_export/) | Экспортировать артефакты и связи в JSON. Возвращает данные либо записывает пакет по output; локальная конфигурация не включается. |
| [`forgeplan_import`](/ru/docs/mcp/forgeplan_import/) | Импортировать JSON-экспорт из строки data. force разрешает перезапись существующих ID. |
| [`forgeplan_init`](/ru/docs/mcp/forgeplan_init/) | Инициализирует новое рабочее пространство .forgeplan/. Создает таблицы LanceDB, конфигурацию и подкаталоги артефактов. |

### Создание артефактов

| Инструмент | Описание |
|---|---|
| [`forgeplan_capture`](/ru/docs/mcp/forgeplan_capture/) | Захват решения из разговора в артефакт Note или ADR. Автоматически определяет тип: простые решения становятся Note, архитектурные решения - ADR. |
| [`forgeplan_generate`](/ru/docs/mcp/forgeplan_generate/) | Сгенерировать артефакт по описанию через настроенный LLM: HTTP-провайдер, Claude, Gemini, Ollama или явно выбранный локальный CLI-провайдер с подпиской. |
| [`forgeplan_new`](/ru/docs/mcp/forgeplan_new/) | Создать артефакт из шаблона и вернуть данные идентификации и пути. Тело читается через forgeplan_get. |

### Чтение артефактов

| Инструмент | Описание |
|---|---|
| [`forgeplan_blocked`](/ru/docs/mcp/forgeplan_blocked/) | Показывает заблокированные артефакты и их невыполненные зависимости. Блокируют только черновики артефактов - устаревшие и замещённые считаются разрешёнными. Используются только структурные связи (based_on, refines, supersedes, contradicts). |
| [`forgeplan_get`](/ru/docs/mcp/forgeplan_get/) | Считывает полный артефакт по ID. Возвращает все метаданные и содержимое тела. |
| [`forgeplan_graph`](/ru/docs/mcp/forgeplan_graph/) | Генерирует граф зависимостей Mermaid для всех связанных артефактов. Включает явные связи и рёбра belongs_to из parent_epic. |
| [`forgeplan_journal`](/ru/docs/mcp/forgeplan_journal/) | Показать журнал решений - хронологическую временную шкалу артефактов ADR, Note, Problem, Solution с оценками R_eff и статусом доказательств. |
| [`forgeplan_list`](/ru/docs/mcp/forgeplan_list/) | Перечислить артефакты с необязательными фильтрами kind и status. Возвращает ID, тип, статус и заголовок. |
| [`forgeplan_order`](/ru/docs/mcp/forgeplan_order/) | Показывает артефакты в топологическом порядке (порядке зависимостей). Возвращает упорядоченный список, классификацию на готовые/заблокированные и обнаружение циклов. Использует только структурные связи. |
| [`forgeplan_progress`](/ru/docs/mcp/forgeplan_progress/) | Показывает прогресс выполнения артефактов на основе чекбоксов. Анализирует чекбоксы Markdown (- [ ] / - [x]) и вычисляет проценты завершения. |
| [`forgeplan_search`](/ru/docs/mcp/forgeplan_search/) | Умный поиск по артефактам: поиск по ключевым словам BM25 + опциональный семантический поиск + расширение графа. Поддерживает фильтры по типу/статусу/глубине/доказательствам/дате создания и переключение расширения графа. |
| [`forgeplan_session`](/ru/docs/mcp/forgeplan_session/) | Показать текущее состояние сессии методологии - фазу (idle/routing/shaping/coding/evidence/pr), активный артефакт, глубину, статус принудительного применения. Используйте это, чтобы знать, на каком этапе рабочего процесса вы находитесь. |

### Редактирование артефактов

| Инструмент | Описание |
|---|---|
| [`forgeplan_delete`](/ru/docs/mcp/forgeplan_delete/) | Мягко удалить артефакт, сохранив запись восстановления в корзине. |
| [`forgeplan_link`](/ru/docs/mcp/forgeplan_link/) | Связывает два артефакта типизированным отношением. Допустимые типы: informs, based_on, supersedes, contradicts, refines. |
| [`forgeplan_unlink`](/ru/docs/mcp/forgeplan_unlink/) | Удалить типизированную связь между двумя артефактами. Зеркальная операция к `forgeplan_link` для отмены или исправления неправильно типизированных рёбер. Issue #286. |
| [`forgeplan_update`](/ru/docs/mcp/forgeplan_update/) | Обновляет метаданные артефакта (статус, заголовок) и/или тело. Перерисовывает проекцию markdown после обновления. |

### Качество и валидация

| Инструмент | Описание |
|---|---|
| [`forgeplan_calibrate`](/ru/docs/mcp/forgeplan_calibrate/) | Предлагает уровень глубины (Tactical/Standard/Deep/Critical) для артефактов на основе анализа содержимого. Обнаруживает разделы безопасности, критические изменения, количество ссылок, сложность содержимого. |
| [`forgeplan_coverage`](/ru/docs/mcp/forgeplan_coverage/) | Показать покрытие решений по модулю кода - какие модули имеют архитектурные решения, а какие остаются слепыми пятнами. |
| [`forgeplan_coverage_business`](/ru/docs/mcp/forgeplan_coverage_business/) | Issue #287 Phase C - вычисление extract_score (0.0-1.0) для Domain Model. Читает секцию ## Composition DM, чтобы получить ожидаемые количества glossary / use_cases / invariants / scenarios / hypotheses; подсчитывает фактические значения по рабочему пространству; возвращает взвешенный extract_score. Канонические поля (DDL/SDL/pseudo-code) по умолчанию 0/false до выхода marketplace#79 Phase E. |
| [`forgeplan_decay`](/ru/docs/mcp/forgeplan_decay/) | Показывает влияние устаревания доказательств на оценки R_eff. Перечисляет артефакты, где просроченные доказательства снизили оценки качества, со сравнением текущего и обновлённого R_eff. |
| [`forgeplan_drift`](/ru/docs/mcp/forgeplan_drift/) | Проверка дрейфующих решений - затронутых файлов, которые изменились после создания ADR/RFC. |
| [`forgeplan_estimate`](/ru/docs/mcp/forgeplan_estimate/) | Оценить трудозатраты для артефакта на основе пунктов функциональных требований (FR) и этапов реализации (Phase). Возвращает разбивку по грейдам (Junior/Middle/Senior/Principal/AI) с оценкой достоверности. |
| [`forgeplan_guard`](/ru/docs/mcp/forgeplan_guard/) | Проверить переход фазы сессии методологии, не изменяя её и не записывая фазу артефакта. |
| [`forgeplan_review`](/ru/docs/mcp/forgeplan_review/) | Проверить артефакт - запустить валидацию и показать контрольный список жизненного цикла. Показывает результаты MUST/SHOULD и готовность артефакта к активации. |
| [`forgeplan_score`](/ru/docs/mcp/forgeplan_score/) | Вычисляет показатель качества R_eff для артефакта на основе связанных доказательств. R_eff использует принцип слабого звена: показатель = min(показатели_доказательств). Пакеты с терминальным статусом (superseded/deprecated) остаются в ответе с флагом excluded: true и своей сырой оценкой, но в min не входят (ADR-020). |
| [`forgeplan_stale`](/ru/docs/mcp/forgeplan_stale/) | Обнаруживает просроченные артефакты с истёкшими датами valid_until. Возвращает список просроченных артефактов с указанием количества дней с момента истечения срока. |
| [`forgeplan_validate`](/ru/docs/mcp/forgeplan_validate/) | Валидирует полноту артефакта в соответствии с правилами схемы. Проверяет обязательные разделы для каждого типа артефакта и уровня глубины. Возвращает структурированные результаты с указанием серьёзности (MUST/SHOULD/COULD). |

### Жизненный цикл

| Инструмент | Описание |
|---|---|
| [`forgeplan_activate`](/ru/docs/mcp/forgeplan_activate/) | Активирует артефакт (черновик → активный). Требует прохождения всех правил валидации MUST. |
| [`forgeplan_deprecate`](/ru/docs/mcp/forgeplan_deprecate/) | Перевести артефакт в deprecated с сохранением причины в разделе Deprecation. |
| [`forgeplan_supersede`](/ru/docs/mcp/forgeplan_supersede/) | Заменить артефакт другим через переход superseded со связью и записью восстановления. |

### Рассуждения и ИИ

| Инструмент | Описание |
|---|---|
| [`forgeplan_decompose`](/ru/docs/mcp/forgeplan_decompose/) | Декомпозирует PRD на задачи RFC с помощью ИИ. Анализирует функциональные требования и предлагает 3-7 RFC с заголовками, описаниями, областью действия и зависимостями. Требует поставщика LLM. |
| [`forgeplan_reason`](/ru/docs/mcp/forgeplan_reason/) | Выполнить ADI-анализ артефакта через настроенный LLM и вернуть текст analysis. |
| [`forgeplan_route`](/ru/docs/mcp/forgeplan_route/) | Предлагает уровень глубины (тактическая/стандартная/глубокая/критическая) и конвейер артефактов для описания задачи. Использует классификацию LLM (Уровень 1), если ключ API настроен, в противном случае возвращается к классификации на основе правил по ключевым словам (Уровень 0). |

### Панели состояния

| Инструмент | Описание |
|---|---|
| [`forgeplan_blindspots`](/ru/docs/mcp/forgeplan_blindspots/) | Показать слепые пятна - решения (PRD/RFC/ADR/Epic) без связанных доказательств, а также сироты (артефакты без связей). |
| [`forgeplan_health`](/ru/docs/mcp/forgeplan_health/) | Отчёт о состоянии проекта с вердиктом healthy, needs_attention или unhealthy. LLM не требуется. |
| [`forgeplan_status`](/ru/docs/mcp/forgeplan_status/) | Показать дашборд статуса проекта - общее количество артефактов, количество по типу и статусу. |

### База знаний FPF

| Инструмент | Описание |
|---|---|
| [`forgeplan_fpf_check`](/ru/docs/mcp/forgeplan_fpf_check/) | Оценивает набор правил FPF для конкретного артефакта. Возвращает каждое совпавшее правило, выигрышное правило (первое в порядке приоритета - как и во время выполнения), правила, которые не совпали, и рекомендуемую категорию действий. Используйте это, чтобы ответить на вопрос 'что мне делать дальше с этим артефактом и почему?' |
| [`forgeplan_fpf_list`](/ru/docs/mcp/forgeplan_fpf_list/) | Перечислить разделы FPF, фактически загруженные в базу знаний рабочего пространства. |
| [`forgeplan_fpf_rules`](/ru/docs/mcp/forgeplan_fpf_rules/) | Список активных правил FPF, сгруппированных по категориям действий (EXPLORE / INVESTIGATE / EXPLOIT). Правила кодируют логику принятия решений фреймворка в виде деревьев условий, которые сопоставляются с состоянием артефакта и доказательствами. Используйте это, чтобы понять, что и почему будет делать движок, прежде чем запускать forgeplan_fpf_check для конкретного артефакта. |
| [`forgeplan_fpf_search`](/ru/docs/mcp/forgeplan_fpf_search/) | Поиск по загруженной базе знаний FPF: ключевые слова или BGE-M3 при поддержке сборкой, с предупреждением при возврате к ключевым словам. |
| [`forgeplan_fpf_section`](/ru/docs/mcp/forgeplan_fpf_section/) | Получить полное содержимое конкретного раздела FPF по ID (например, 'B.3', 'C.2.2', 'A.1'). |

### Исследование существующей системы

| Инструмент | Описание |
|---|---|
| [`forgeplan_discover_complete`](/ru/docs/mcp/forgeplan_discover_complete/) | Завершить сессию исследования и вернуть количества и ID созданных черновиков. Health вызывается отдельно. |
| [`forgeplan_discover_finding`](/ru/docs/mcp/forgeplan_discover_finding/) | Записать наблюдение в сессию исследования и создать черновик артефакта с уровнем источника 1–3. |
| [`forgeplan_discover_start`](/ru/docs/mcp/forgeplan_discover_start/) | Начать исследование существующей системы: сессия и семифазный протокол для агента. |

### Прочее

| Инструмент | Описание |
|---|---|
| [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/) | Прочитать журнал вызовов MCP с фильтрами времени, инструмента и статуса. |
| [`forgeplan_activity_stats`](/ru/docs/mcp/forgeplan_activity_stats/) | Агрегировать журнал MCP по инструментам: число вызовов, ошибки, p50/p95 и суммарное время. |
| [`forgeplan_anomalies`](/ru/docs/mcp/forgeplan_anomalies/) | Issue #289 - обнаружение аномалий конвейера (зависшие черновики, сиротские ссылки, опечатки в based_on, отсутствующие обязательные секции, просроченные доказательства, циклы, дубликаты и т.д.). Возвращает структурированный список с severity + tier + suggested_resolution, чтобы оркестраторы (`/forge-cycle`, `/forge-cleanup`) могли распределять задачи по уровням auto/adi/user без повторной классификации. |
| [`forgeplan_claim`](/ru/docs/mcp/forgeplan_claim/) | Захватить артефакт для работы одного агента с ограниченным сроком. Захват рекомендательный и не блокирует другие инструменты. |
| [`forgeplan_claims`](/ru/docs/mcp/forgeplan_claims/) | Перечислить действующие захваты артефактов, отсортированные по сроку истечения. |
| [`forgeplan_contradictions`](/ru/docs/mcp/forgeplan_contradictions/) | Issue #287 Phase C - обнаружение противоречий между артефактами (v1: hypothesis_duplicate через Jaccard ≥ 0.6 по заголовкам). Три других класса (invariant_conflict, glossary_divergence, scenario_vs_invariant) требуют LLM-оценки и отложены - в ответе присутствуют пустые массивы + список ограничений. |
| [`forgeplan_dispatch`](/ru/docs/mcp/forgeplan_dispatch/) | Рассчитать план распределения работы между агентами без изменения рабочего пространства. |
| [`forgeplan_hypothesis_promote`](/ru/docs/mcp/forgeplan_hypothesis_promote/) | Продвигает гипотезу по конечному автомату верификации (parked → inferred → strong-inferred → verified; или любое → refuted; или любое → parked). Изменяет тело артефакта через file-first проекцию: обновляет поле `hypothesis_state:` во frontmatter и добавляет запись аудита в секцию `## Lifecycle State`. Возвращает список артефактов, которые информируют / основаны на гипотезе (каскад) - только для информации, автоматически не обновляются. Отклоняет недопустимые переходы со структурированной ошибкой, перечисляющей разрешённые следующие состояния. |
| [`forgeplan_hypothesis_status`](/ru/docs/mcp/forgeplan_hypothesis_status/) | Issue #287 Phase C - запрос состояния жизненного цикла гипотез. Возвращает распределение состояний по всему рабочему пространству (verified / strong-inferred / inferred / refuted / parked) плюс переходы за последние 30 дней (извлечение аудит-строк из секций `## Lifecycle State`). Опциональный фильтр `id` сужает результат до одного артефакта HYP-. |
| [`forgeplan_ingest`](/ru/docs/mcp/forgeplan_ingest/) | Планирование запуска инжеста (YAML-маппинг × исходный файл). Волна 3: только dry-run - возвращает черновики без записи артефактов. Установите `dry_run: false`, чтобы указать намерение записать, но поверхность MCP всё равно делегирует фактическую запись CLI `forgeplan ingest` (Волна 4 подключит artifact::Store). PRD-066 AC-1. |
| [`forgeplan_interview_packet_draft`](/ru/docs/mcp/forgeplan_interview_packet_draft/) | Экспериментальная заглушка 0.38.0: возвращает not_implemented независимо от установки marketplace-пакета. |
| [`forgeplan_interview_packet_ingest`](/ru/docs/mcp/forgeplan_interview_packet_ingest/) | Экспериментальная заглушка 0.38.0: проверяет путь и возвращает not_implemented, не читая расшифровку. |
| [`forgeplan_orphans`](/ru/docs/mcp/forgeplan_orphans/) | Issue #287 Phase C - обнаружение brownfield-сирот: непокрытые use cases (без связей со сценариями), неверифицированные инварианты (без связей со сценариями), сиротские термины глоссария (не упомянуты ни в одном теле UC/INV), нетриангулированные гипотезы (< 2 ссылок на доказательства). |
| [`forgeplan_phase`](/ru/docs/mcp/forgeplan_phase/) | Read advisory phase state for an artifact - current phase, history, workflow type. |
| [`forgeplan_phase_advance`](/ru/docs/mcp/forgeplan_phase_advance/) | Изменить рекомендательный маркер фазы артефакта и записать переход в историю. |
| [`forgeplan_playbook_list`](/ru/docs/mcp/forgeplan_playbook_list/) | Перечисляет все обнаруживаемые плейбуки (рабочее пространство + пакеты плагинов Claude). Возвращает имя, заголовок, количество шагов и путь к источнику для каждого. PRD-065 AC-1. Сканирование файловой системы только для чтения. |
| [`forgeplan_playbook_run`](/ru/docs/mcp/forgeplan_playbook_run/) | Запускает плейбук от начала до конца. Wave 4 подключает production-диспетчеры plugin/agent/skill/command/forgeplan_core через `RoutingDispatcher`. Отказывает без `yes: true` (гейт безопасности ADR-009). Плейбуки, содержащие шаги `Delegation::Command` (shell-exec), дополнительно требуют `allow_shell: true` (PRD-074 / PROB-053, default-deny). Рабочие пространства могут предварительно одобрить доверенное локальное выполнение shell через `[playbook] allow_shell = true` в `.forgeplan/config.yaml`. Используйте `dry_run: true` для перечисления шагов без вызова диспетчеров. |
| [`forgeplan_playbook_show`](/ru/docs/mcp/forgeplan_playbook_show/) | Показывает полную информацию об одном плейбуке (разрешается по имени или пути). Возвращает распарсенную структуру Playbook + путь к источнику. PRD-065 AC-1. |
| [`forgeplan_playbook_validate`](/ru/docs/mcp/forgeplan_playbook_validate/) | Валидирует YAML-файл плейбука (парсинг + структурные проверки: циклы, неизвестные ссылки на шаги, согласованность mapping/produces_at). Возвращает флаг `passed` + список ошибок. PRD-065 AC-2. |
| [`forgeplan_plugins_doctor`](/ru/docs/mcp/forgeplan_plugins_doctor/) | Проверка здоровья всего реестра плагинов: разделяет записи на ok / устаревшие / отсутствующие и предоставляет подсказки по установке для отсутствующих. PRD-067 AC-2. |
| [`forgeplan_plugins_info`](/ru/docs/mcp/forgeplan_plugins_info/) | Показать детали одного плагина из расширенного реестра. Возвращает статический PluginInfo плюс, если установлен, runtime-запись InstalledPlugin (обнаруженный путь + версия). PRD-067. |
| [`forgeplan_plugins_list`](/ru/docs/mcp/forgeplan_plugins_list/) | Перечисляет установленные плагины, обнаруженные на диске (плагины Claude + agentskills + Cursor) + отсутствующие записи из расширенного реестра. PRD-067 AC-1. Сканирование файловой системы только для чтения. |
| [`forgeplan_release`](/ru/docs/mcp/forgeplan_release/) | Снять рекомендательный захват артефакта; force явно разрешает обход проверки держателя. |
| [`forgeplan_release_notes`](/ru/docs/mcp/forgeplan_release_notes/) | Автоматически генерирует заметки о релизе в формате Keep-a-Changelog из артефактов, изменившихся между двумя git-ссылками. Проходит по `git log` в `.forgeplan/{prds, problems, evidence, rfcs, adrs, specs, epics, solutions}/`, классифицирует каждый затронутый артефакт (PRD→Added, PROB→Fixed, EVID-on-security→Security, RFC/ADR→Changed) и возвращает структурированный результат. Гейт качества (по умолчанию): включаются только артефакты со status==active или r_eff_score > 0. Передайте `draft: true`, чтобы отключить гейт. `since` по умолчанию - последний git-тег, `until` по умолчанию - HEAD. Пример: `{since: "v0.30.0", draft: false}`. |
| [`forgeplan_restore`](/ru/docs/mcp/forgeplan_restore/) | Восстановить артефакт из последней неиспользованной записи в корзине. |
| [`forgeplan_undo_last`](/ru/docs/mcp/forgeplan_undo_last/) | Отменить последнюю деструктивную операцию по доступной записи восстановления. |
