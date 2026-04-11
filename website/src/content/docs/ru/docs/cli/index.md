---
title: Справочник CLI
description: "Полный справочник по всем 61 командам CLI Forgeplan."
---

Forgeplan поставляется с **61 командой верхнего уровня**, охватывающей полный жизненный цикл Shape→Validate→ADI→Code→Evidence→Activate.

Все команды перечислены ниже, сгруппированные по назначению. Нажмите на любую команду для получения полной информации об использовании, аргументах и примерах.

### Рабочее пространство и настройка

| Команда | Описание |
|---|---|
| [`forgeplan init`](/docs/cli/init/) | Инициализирует новое рабочее пространство .forgeplan/ |
| [`forgeplan setup-skill`](/docs/cli/setup-skill/) | Устанавливает навык /forge для Claude Code |
| [`forgeplan migrate`](/docs/cli/migrate/) | Выполняет миграции схемы в существующем рабочем пространстве |
| [`forgeplan import`](/docs/cli/import/) | Импортирует артефакты из JSON-файла |
| [`forgeplan export`](/docs/cli/export/) | Экспортирует все артефакты в JSON-файл |

### Создание артефактов

| Команда | Описание |
|---|---|
| [`forgeplan new`](/docs/cli/new/) | Создаёт новый артефакт из шаблона |
| [`forgeplan generate`](/docs/cli/generate/) | Генерирует артефакт с помощью ИИ на основе описания на естественном языке |
| [`forgeplan capture`](/docs/cli/capture/) | Захватывает решение из разговора в артефакт Note или ADR |
| [`forgeplan promote`](/docs/cli/promote/) | Повышает память до полноценного артефакта (например, forgeplan promote mem-xxx --kind prd) |

### Чтение артефактов

| Команда | Описание |
|---|---|
| [`forgeplan list`](/docs/cli/list/) | Перечисляет артефакты |
| [`forgeplan get`](/docs/cli/get/) | Читает полный артефакт по ID |
| [`forgeplan tree`](/docs/cli/tree/) | Показывает иерархию артефактов в виде ASCII-дерева |
| [`forgeplan search`](/docs/cli/search/) | Ищет артефакты (по умолчанию умный поиск: по ключевым словам + семантический + усилители) |
| [`forgeplan recall`](/docs/cli/recall/) | Вспоминает памяти — поиск, фильтрация, список |
| [`forgeplan log`](/docs/cli/log/) | Показывает журнал изменений — аудиторский след мутаций артефактов |
| [`forgeplan journal`](/docs/cli/journal/) | Показывает журнал решений — хронологическую шкалу с оценками R_eff |
| [`forgeplan session`](/docs/cli/session/) | Показывает состояние сессии методологии (текущая фаза, активный артефакт) |
| [`forgeplan progress`](/docs/cli/progress/) | Показывает прогресс по чекбоксам для артефактов |
| [`forgeplan graph`](/docs/cli/graph/) | Генерирует граф зависимостей связанных артефактов в формате mermaid |
| [`forgeplan order`](/docs/cli/order/) | Показывает артефакты в топологическом порядке (порядок зависимостей) |

### Редактирование артефактов

| Команда | Описание |
|---|---|
| [`forgeplan update`](/docs/cli/update/) | Обновляет метаданные или тело артефакта |
| [`forgeplan delete`](/docs/cli/delete/) | Удаляет артефакт |
| [`forgeplan tag`](/docs/cli/tag/) | Добавляет теги к артефакту |
| [`forgeplan untag`](/docs/cli/untag/) | Удаляет теги из артефакта |
| [`forgeplan link`](/docs/cli/link/) | Связывает два артефакта типизированной связью |
| [`forgeplan unlink`](/docs/cli/unlink/) | Удаляет связь между двумя артефактами |

### Качество и валидация

| Команда | Описание |
|---|---|
| [`forgeplan validate`](/docs/cli/validate/) | Валидирует полноту артефакта по правилам схемы |
| [`forgeplan score`](/docs/cli/score/) | Вычисляет оценку качества R_eff для решений с доказательствами |
| [`forgeplan fgr`](/docs/cli/fgr/) | Показывает оценки качества F-G-R (Formality, Granularity, Reliability) |
| [`forgeplan review`](/docs/cli/review/) | Проверяет артефакт — запускает валидацию и показывает контрольный список жизненного цикла |
| [`forgeplan estimate`](/docs/cli/estimate/) | Оценивает трудозатраты для артефакта на основе элементов FR и Phase |
| [`forgeplan calibrate`](/docs/cli/calibrate/) | Предлагает уровень глубины (Tactical/Standard/Deep/Critical) на основе содержимого артефакта |
| [`forgeplan calibrate-estimate`](/docs/cli/calibrate-estimate/) | Сравнивает оценочные и фактические часы — калибрует точность оценки |
| [`forgeplan decay`](/docs/cli/decay/) | Показывает влияние устаревания доказательств на оценки R_eff |
| [`forgeplan stale`](/docs/cli/stale/) | Обнаруживает просроченные артефакты с истёкшим valid_until |

### Переходы жизненного цикла

| Команда | Описание |
|---|---|
| [`forgeplan activate`](/docs/cli/activate/) | Активирует артефакт (черновик → активный) с гейтом валидации |
| [`forgeplan supersede`](/docs/cli/supersede/) | Замещает артефакт (активный → замещённый) ссылкой на замену |
| [`forgeplan deprecate`](/docs/cli/deprecate/) | Отменяет артефакт (активный/просроченный → отменённый) с указанием причины |
| [`forgeplan renew`](/docs/cli/renew/) | Продлевает просроченный артефакт (просроченный → активный) с продлённым сроком действия |
| [`forgeplan reopen`](/docs/cli/reopen/) | Переоткрывает артефакт — создаёт НОВЫЙ артефакт-черновик, отменяет старый |

### Рассуждения и ИИ

| Команда | Описание |
|---|---|
| [`forgeplan reason`](/docs/cli/reason/) | Анализирует артефакт с использованием цикла рассуждений FPF ADI (Abduction→Deduction→Induction) |
| [`forgeplan decompose`](/docs/cli/decompose/) | Декомпозирует PRD на задачи RFC с использованием ИИ |
| [`forgeplan context`](/docs/cli/context/) | Контекст рассуждений в один вызов — артефакт + граф + валидация + оценка |
| [`forgeplan route`](/docs/cli/route/) | Предлагает уровень глубины и конвейер артефактов для описания задачи |

### Дашборды и состояние

| Команда | Описание |
|---|---|
| [`forgeplan health`](/docs/cli/health/) | Показывает дашборд состояния проекта — пробелы, риски, слепые пятна, следующие действия |
| [`forgeplan status`](/docs/cli/status/) | Показывает дашборд статуса проекта |
| [`forgeplan gaps`](/docs/cli/gaps/) | Показывает пробелы в соответствии конвейера по глубине |
| [`forgeplan blocked`](/docs/cli/blocked/) | Показывает заблокированные артефакты и их зависимости |
| [`forgeplan blindspots`](/docs/cli/blindspots/) | Показывает слепые пятна — решения без доказательств, сироты (артефакты без связей) |
| [`forgeplan drift`](/docs/cli/drift/) | Проверяет на смещённые решения (затронутые файлы изменились после решения) |
| [`forgeplan coverage`](/docs/cli/coverage/) | Показывает покрытие решений по модулям кода |

### Индексация и синхронизация

| Команда | Описание |
|---|---|
| [`forgeplan scan`](/docs/cli/scan/) | Сканирует кодовую базу на предмет исходных модулей |
| [`forgeplan scan-import`](/docs/cli/scan-import/) | Сканирует существующие документы и импортирует их как артефакты |
| [`forgeplan reindex`](/docs/cli/reindex/) | Перестраивает индекс LanceDB из файлов .md (синхронизация по файлам) |
| [`forgeplan embed`](/docs/cli/embed/) | Генерирует эмбеддинги для всех артефактов (семантический поиск) |
| [`forgeplan watch`](/docs/cli/watch/) | Отслеживает файлы .forgeplan/ и синхронизирует изменения с LanceDB в реальном времени |
| [`forgeplan git-sync`](/docs/cli/git-sync/) | Синхронизирует изменения артефактов из операций git (pull/merge) в LanceDB |

### Память

| Команда | Описание |
|---|---|
| [`forgeplan remember`](/docs/cli/remember/) | Сохраняет память (факт, соглашение, процедуру) для последующего вызова |
| [`forgeplan discover`](/docs/cli/discover/) | Начинает brownfield-обнаружение — создаёт сессию, выводит протокол для агента |

### База знаний FPF

| Команда | Описание |
|---|---|
| [`forgeplan fpf`](/docs/cli/fpf/) | База знаний FPF — дашборд, приём данных, поиск, разделы |

### Сервер MCP

| Команда | Описание |
|---|---|
| [`forgeplan serve`](/docs/cli/serve/) | Запускает сервер MCP (транспорт stdio) для интеграции с ИИ-агентами |

## Экосистема и плагины

Помимо встроенного CLI, Forgeplan интегрируется с ИИ-агентами для кодирования через навык `/forge` и связанные плагины маркетплейса:

- [**Forgeplan Workflow**](/docs/marketplace/forgeplan-workflow/) — слеш-команды `/forge`, `/forge-cycle`, `/forge-audit`
- [**Dev Toolkit**](/docs/marketplace/dev-toolkit/) — `/sprint`, `/audit`, `/recall`, `/research`, `/build`
- [**Обзор маркетплейса**](/docs/marketplace/overview/) — полный каталог плагинов

Установите основной навык с помощью `forgeplan setup-skill` или `npx skills add ForgePlan/marketplace --skill forge`. Дополнительные плагины доступны через `npx skills add ForgePlan/marketplace --plugin <name>`.