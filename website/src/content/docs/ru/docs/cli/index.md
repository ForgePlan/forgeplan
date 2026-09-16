---
title: Справочник CLI
description: "Полный справочник всех 82 команд Forgeplan CLI."
---

Forgeplan поставляется с **82 командами верхнего уровня**, охватывающими полный жизненный цикл Shape→Validate→ADI→Code→Evidence→Activate.

Все команды перечислены ниже, сгруппированные по назначению. Нажмите на любую команду для получения полной информации об использовании, аргументах и примерах.

### Рабочее пространство и настройка

| Команда | Описание |
|---|---|
| [`forgeplan init`](/ru/docs/cli/init/) | Инициализировать новое рабочее пространство .forgeplan/ |
| [`forgeplan setup-skill`](/ru/docs/cli/setup-skill/) | Установить /forge skill для Claude Code |
| [`forgeplan migrate`](/ru/docs/cli/migrate/) | Запустить миграции схемы на существующем рабочем пространстве |
| [`forgeplan import`](/ru/docs/cli/import/) | Импортировать артефакты из JSON файла |
| [`forgeplan export`](/ru/docs/cli/export/) | Экспортировать все артефакты в JSON файл |

### Создание артефактов

| Команда | Описание |
|---|---|
| [`forgeplan new`](/ru/docs/cli/new/) | Создать новый артефакт из шаблона |
| [`forgeplan generate`](/ru/docs/cli/generate/) | Сгенерировать артефакт с помощью AI из описания на естественном языке |
| [`forgeplan capture`](/ru/docs/cli/capture/) | Захватить решение из разговора в артефакт Note или ADR |
| [`forgeplan promote`](/ru/docs/cli/promote/) | Повысить память до полного артефакта (например, forgeplan promote mem-xxx --kind prd) |

### Чтение артефактов

| Команда | Описание |
|---|---|
| [`forgeplan list`](/ru/docs/cli/list/) | Список артефактов |
| [`forgeplan get`](/ru/docs/cli/get/) | Прочитать полный артефакт по ID |
| [`forgeplan tree`](/ru/docs/cli/tree/) | Показать иерархию артефактов как ASCII дерево |
| [`forgeplan search`](/ru/docs/cli/search/) | Поиск артефактов (по умолчанию умный: поиск по ключевым словам + семантический поиск + бустеры) |
| [`forgeplan recall`](/ru/docs/cli/recall/) | Вспомнить воспоминания - поиск, фильтрация, список |
| [`forgeplan log`](/ru/docs/cli/log/) | Показать журнал изменений - аудит мутаций артефактов |
| [`forgeplan journal`](/ru/docs/cli/journal/) | Показать журнал решений - хронологическая временная шкала с оценками R_eff |
| [`forgeplan session`](/ru/docs/cli/session/) | Показать состояние сеанса методологии (текущая фаза, активный артефакт) |
| [`forgeplan progress`](/ru/docs/cli/progress/) | Показать прогресс чекбоксов для артефактов |
| [`forgeplan graph`](/ru/docs/cli/graph/) | Сгенерировать граф зависимостей mermaid связанных артефактов |
| [`forgeplan order`](/ru/docs/cli/order/) | Показать артефакты в топологическом порядке (порядок зависимостей) |

### Редактирование артефактов

| Команда | Описание |
|---|---|
| [`forgeplan update`](/ru/docs/cli/update/) | Обновить метаданные или содержимое артефакта |
| [`forgeplan delete`](/ru/docs/cli/delete/) | Удалить артефакт |
| [`forgeplan tag`](/ru/docs/cli/tag/) | Добавить теги к артефакту |
| [`forgeplan untag`](/ru/docs/cli/untag/) | Удалить теги из артефакта |
| [`forgeplan link`](/ru/docs/cli/link/) | Связать два артефакта типизированным отношением |
| [`forgeplan unlink`](/ru/docs/cli/unlink/) | Удалить отношение между двумя артефактами |

### Качество и валидация

| Команда | Описание |
|---|---|
| [`forgeplan validate`](/ru/docs/cli/validate/) | Валидировать полноту артефакта согласно правилам схемы |
| [`forgeplan score`](/ru/docs/cli/score/) | Вычислить оценку качества R_eff для решений с доказательствами |
| [`forgeplan fgr`](/ru/docs/cli/fgr/) | Показать оценки качества F-G-R (формальность, гранулярность, надёжность) |
| [`forgeplan review`](/ru/docs/cli/review/) | Проверить артефакт - запустить валидацию и показать чеклист жизненного цикла |
| [`forgeplan estimate`](/ru/docs/cli/estimate/) | Оценить усилия для артефакта на основе элементов FR и Phase |
| [`forgeplan calibrate`](/ru/docs/cli/calibrate/) | Предложить уровень глубины (тактическая/стандартная/глубокая/критическая) на основе содержимого артефакта |
| [`forgeplan calibrate-estimate`](/ru/docs/cli/calibrate-estimate/) | Сравнить оценённые и фактические часы - откалибровать точность оценки |
| [`forgeplan decay`](/ru/docs/cli/decay/) | Показать влияние затухания доказательств на оценки R_eff |
| [`forgeplan stale`](/ru/docs/cli/stale/) | Обнаружить просроченные артефакты с истёкшим valid_until |

### Переходы жизненного цикла

| Команда | Описание |
|---|---|
| [`forgeplan activate`](/ru/docs/cli/activate/) | Активировать артефакт (черновик → активный) с гейтом валидации |
| [`forgeplan supersede`](/ru/docs/cli/supersede/) | Заместить артефакт (активный → замещённый) со ссылкой на замену |
| [`forgeplan deprecate`](/ru/docs/cli/deprecate/) | Отменить артефакт (активный/просроченный → устаревший) с причиной |
| [`forgeplan renew`](/ru/docs/cli/renew/) | Продлить просроченный артефакт (просроченный → активный) с расширенной валидностью |
| [`forgeplan reopen`](/ru/docs/cli/reopen/) | Переоткрыть артефакт - создаёт НОВЫЙ черновик, отменяет старый |

### Рассуждение и AI

| Команда | Описание |
|---|---|
| [`forgeplan reason`](/ru/docs/cli/reason/) | Анализировать артефакт с использованием цикла рассуждений FPF ADI (абдукция→дедукция→индукция) |
| [`forgeplan decompose`](/ru/docs/cli/decompose/) | Разложить PRD на задачи RFC с помощью AI |
| [`forgeplan context`](/ru/docs/cli/context/) | Контекст рассуждения в один вызов - артефакт + граф + валидация + оценка |
| [`forgeplan route`](/ru/docs/cli/route/) | Предложить уровень глубины и конвейер артефактов для описания задачи |

### Панели управления и здоровье

| Команда | Описание |
|---|---|
| [`forgeplan health`](/ru/docs/cli/health/) | Показать панель здоровья проекта - пробелы, риски, слепые пятна, следующие действия |
| [`forgeplan status`](/ru/docs/cli/status/) | Показать панель статуса проекта |
| [`forgeplan gaps`](/ru/docs/cli/gaps/) | Показать пробелы соответствия конвейера по глубине |
| [`forgeplan blocked`](/ru/docs/cli/blocked/) | Показать заблокированные артефакты и их зависимости |
| [`forgeplan blindspots`](/ru/docs/cli/blindspots/) | Показать слепые пятна - решения без доказательств, сироты артефактов |
| [`forgeplan drift`](/ru/docs/cli/drift/) | Проверить дрейф решений (затронутые файлы изменились после решения) |
| [`forgeplan coverage`](/ru/docs/cli/coverage/) | Показать покрытие решений на модуль кода |

### Индексирование и синхронизация

| Команда | Описание |
|---|---|
| [`forgeplan scan`](/ru/docs/cli/scan/) | Сканировать кодовую базу на исходные модули |
| [`forgeplan scan-import`](/ru/docs/cli/scan-import/) | Сканировать существующие документы и импортировать как артефакты |
| [`forgeplan reindex`](/ru/docs/cli/reindex/) | Перестроить индекс LanceDB из .md файлов (синхронизация файлы-первыми) |
| [`forgeplan embed`](/ru/docs/cli/embed/) | Сгенерировать эмбеддинги для всех артефактов (семантический поиск) |
| [`forgeplan watch`](/ru/docs/cli/watch/) | Следить за файлами .forgeplan/ и синхронизировать изменения в LanceDB в реальном времени |
| [`forgeplan git-sync`](/ru/docs/cli/git-sync/) | Синхронизировать изменения артефактов из операций git (pull/merge) в LanceDB |

### Память

| Команда | Описание |
|---|---|
| [`forgeplan remember`](/ru/docs/cli/remember/) | Сохранить воспоминание (факт, соглашение, процедура) для последующего вспоминания |
| [`forgeplan discover`](/ru/docs/cli/discover/) | Начать обнаружение brownfield - создаёт сеанс, выводит протокол для агента |

### База знаний FPF

| Команда | Описание |
|---|---|
| [`forgeplan fpf`](/ru/docs/cli/fpf/) | База знаний FPF - панель управления, приём, поиск, разделы |

### MCP сервер

| Команда | Описание |
|---|---|
| [`forgeplan serve`](/ru/docs/cli/serve/) | Запустить MCP сервер (транспорт stdio) для интеграции с AI агентом |

### Прочее

| Команда | Описание |
|---|---|
| [`forgeplan activity`](/ru/docs/cli/activity/) | Запросить журнал активности - запись только для добавления JSONL каждого вызова инструмента MCP в .forgeplan/logs/tools-YYYY-MM-DD.jsonl. Используйте для восстановления того, что делал агент в течение временного окна, атрибуции расходов LLM-токенов или аудита деструктивных операций |
| [`forgeplan activity-stats`](/ru/docs/cli/activity-stats/) | Агрегированная статистика из журнала активности, сгруппированная по имени инструмента: количество, количество ошибок, p50/p95 длительность, общее время. Используйте для атрибуции расходов LLM-токенов и выявления медленных инструментов |
| [`forgeplan restore`](/ru/docs/cli/restore/) | Восстановить мягко удалённый артефакт из самого последнего неиспользованного квитанции в `.forgeplan/trash/`. Работает для delete (пересоздаёт строку + перемещает проекцию обратно), supersede (сбрасывает статус + удаляет ссылку) и deprecate (сбрасывает статус). Отказывает, если артефакт с тем же ID уже существует. TTL по умолчанию: 30 дней от деструктивной операции |
| [`forgeplan undo-last`](/ru/docs/cli/undo-last/) | Отменить самую последнюю деструктивную операцию (delete, supersede или deprecate), прочитав корзину мягкого удаления и применив restore к самой недавно написанной неиспользованной квитанции. Если подходящая квитанция не найдена, возвращает ошибку с рекомендациями; инструмент никогда не угадывает |
| [`forgeplan claim`](/ru/docs/cli/claim/) | Заявить артефакт (PRD-057 координация мультиагента - мягкий сигнал "я работаю над этим") |
| [`forgeplan claims`](/ru/docs/cli/claims/) | Список активных заявок (отсортированы по истечению, ближайшие первыми) |
| [`forgeplan release`](/ru/docs/cli/release/) | Отпустить заявку (PRD-057). Идемпотентно - отсутствующая заявка = успех |
| [`forgeplan release-notes`](/ru/docs/cli/release-notes/) | Сгенерировать примечания к выпуску в стиле Keep-a-Changelog из артефактов, которые изменились между двумя git ссылками. Проходит `git log` по `.forgeplan/{prds,problems,evidence,rfcs,adrs,specs,epics,solutions}/` и категоризирует каждый затронутый артефакт: PRD→Added, PROB→Fixed, EVID-on-security→Security, RFC/ADR→Changed. Гейт качества: только артефакты с `status==active` или `r_eff_score > 0` выводятся (переопределить с `--draft`) |
| [`forgeplan setup`](/ru/docs/cli/setup/) | Одноразовая подготовка на машину: `fpl` алиас + модель эмбеддинга |
| [`forgeplan dispatch`](/ru/docs/cli/dispatch/) | Вычислить параллельно-безопасный план работы для N подагентов (диспетчер PRD-057) |
| [`forgeplan anomalies`](/ru/docs/cli/anomalies/) | Issue #289 - обнаружить аномалии конвейера (застрявшие черновики, сироты ссылок, неправильно типизированные based_on и т.д.). Каждая запись содержит severity + tier + suggested_resolution, чтобы оркестраторы могли отправлять исправления |
| [`forgeplan phase`](/ru/docs/cli/phase/) | Прочитать консультативное состояние фазы для артефакта. Возвращает current_phase, workflow_type, timestamps и полную историю переходов только для добавления из `.forgeplan/state/<id>.yaml`. Если файл состояния не существует (артефакт до PRD-056 или отслеживание фазы было отключено), возвращает `current_phase: unknown` -- никогда ошибка. Отслеживание фазы консультативно и никогда не блокирует другие инструменты |
| [`forgeplan phase-advance`](/ru/docs/cli/phase-advance/) | Вручную продвинуть (или установить) маркер консультативной фазы для артефакта. Добавляет переход в историю. НЕ валидирует упорядочение фаз -- консультативный слой позволяет переходы вне порядка (например, прямое переопределение `done`). Полное применение фазы приземляется в более позднем PRD под EPIC-005. Используйте, когда автоматическое продвижение пропустило переход или при переклассификации состояния рабочего процесса |
| [`forgeplan ci-assign-id`](/ru/docs/cli/ci-assign-id/) | PROB-060 Фаза 0b - атомный CI назначитель `assigned_number`. Проходит `.forgeplan/**/*.md` в `--head`, находит кандидатов с `assigned_number: null`, вычисляет `next = max(assigned_number)+1` на вид из `--base` git ссылки, переписывает frontmatter (без переименования файла - Фаза 2.1). LanceDB-свободно согласно ADR-003. Обёрнуто в производстве `.github/workflows/assign-id.yml` `concurrency: forgeplan-id-assign` |
| [`forgeplan migrate-dry-run`](/ru/docs/cli/migrate-dry-run/) | PROB-060 Фаза 0b - сухой прогон миграции EVID-C. Сканирует все артефакты в `.forgeplan/`, вычисляет slug, который каждый получит согласно правилам SPEC-005, и обнаруживает коллизии на вид перед миграцией Фазы 4. Только для чтения - никогда не мутирует .md файлы |
| [`forgeplan migrate-secrets`](/ru/docs/cli/migrate-secrets/) | PRD-077 FR-023 Часть B - импортировать известные переменные окружения API-ключей (GEMINI_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY) из окружения процесса в `.forgeplan/secrets.env`, чтобы ключи жили локально в проекте вместо того, чтобы быть в rc вашей оболочки. По умолчанию сухой прогон; передайте `--apply` для записи |
| [`forgeplan reconcile-ids`](/ru/docs/cli/reconcile-ids/) | PROB-060 Фаза 2.4 (W2.C) - инструмент ручной очистки для дрейфа идентичности после слияния. Сканирует `.forgeplan/<kind>/*.md`, обнаруживает четыре категории дрейфа (несоответствие имени файла, отсутствующий `predicted_number`, дрейф ссылок в теле, дублирующийся `assigned_number`) и либо сообщает (`--check-only`), либо автоисправляет безопасные категории. LanceDB никогда не трогается; запустите `forgeplan scan-import` после, если индекс нужно перестроить |
| [`forgeplan mcp`](/ru/docs/cli/mcp/) | Помощники интеграции MCP (установить конфиг для клиентов AI агентов) |
| [`forgeplan playbook`](/ru/docs/cli/playbook/) | Среда выполнения playbook (PRD-065 / SPEC-003) - декларативная оркестровка YAML с делегированиями внешним плагинам |
| [`forgeplan ingest`](/ru/docs/cli/ingest/) | Механизм приёма (PRD-066 / SPEC-004) - применить YAML маппинг к выходам плагинов, сгенерировать forge артефакты со ссылками на источник file:line |
| [`forgeplan plugins`](/ru/docs/cli/plugins/) | Обнаружение плагинов + рекомендации (PRD-067) - список установленных плагинов, проверка здоровья, информация о конкретном плагине |