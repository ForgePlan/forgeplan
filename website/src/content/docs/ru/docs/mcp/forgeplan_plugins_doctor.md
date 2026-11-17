---
title: forgeplan_plugins_doctor
description: "Проверка здоровья всего реестра плагинов: разделяет записи на ok / устаревшие / отсутствующие и предоставляет подсказки по установке для отсутствующих. PRD-067 AC-2."
---

Проверка здоровья всего реестра плагинов: разделяет записи на ok / устаревшие / отсутствующие и предоставляет подсказки по установке для отсутствующих. PRD-067 AC-2.

**Категория**: Прочее

Если [`forgeplan_plugins_list`](/ru/docs/mcp/forgeplan_plugins_list/) перечисляет записи, этот инструмент группирует их по совместимости. Каждая запись попадает ровно в одну категорию; каждая запись вне `ok` добавляет строку команды установки из реестра в `install_hints`.

Это классификация реестра и файлов на диске, не работающих агентов или
подтверждённых возможностей хоста. Встроенный каталог — не живой публичный
marketplace. Устанавливайте отсутствующее только для нужного процесса и
проверяйте актуальные публичные инструкции перед использованием подсказки реестра.

## Пример вызова

Аргументы этого инструмента, а не оболочка JSON-RPC. Замените условные ID, идентификаторы сессий и пути значениями из своего рабочего пространства. Соответствие схеме не отменяет проверки рабочего пространства, жизненного цикла и согласия на выполнение.

```json
{}
```

## Что делает

1. Строит расширенный реестр: встроенные плагины по умолчанию плюс курируемые дополнения, которые несёт реестр.
2. Определяет, что фактически установлено на диске, сканируя директории плагинов Claude, директории agent-skill и расположения Cursor.
3. Классифицирует каждую запись реестра:
   - **ok** - установлен, и обнаруженная версия удовлетворяет требованию версии реестра.
   - **outdated** - установлен, но обнаруженная версия не соответствует требованию.
   - **missing** - вообще не обнаружен.
4. Собирает `install_command` каждой записи `outdated` и `missing` в `install_hints`, в порядке реестра. Одна и та же команда может появиться более одного раза, когда несколько записей реестра поставляются в одном пакете - четыре записи агента C4 все указывают на `/plugin install c4-architecture@claude-code-workflows`.
5. Возвращает три категории, список подсказок и три счётчика.

## Когда агент вызывает это

- Начало сессии в незнакомом окружении: один вызов показывает, какие возможности установлены, прежде чем вы планируете их использование.
- После того, как шаг завершился ошибкой "delegate not found" или ошибкой отсутствия навыка - `fallback_hint` плейбука называет плагин, этот инструмент подтверждает, действительно ли он отсутствует.
- Перед рекомендацией плейбука из движка рекомендаций: предварительные требования рекомендованного плейбука - это плагины, а `install_hints` - это то, что пользователь должен выполнить.
- После установки, чтобы подтвердить, что запись переместилась из `missing` в `ok`.

## Входные параметры

_Объявленных входных полей нет. Вызывайте инструмент с пустым объектом `{}`._

_Источник схемы: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Возвращает

Успех — JSON в текстовом содержимом MCP. Входного `workspace` и трассировки нет;
поиск использует пути относительно процесса и `$HOME`. Отсутствующие корни и
нечитаемые манифесты сами по себе не становятся ошибками инструмента.

| Поле | Тип | Описание |
|---|---|---|
| `ok` | `array` | Записи, которые установлены и совместимы по версии. |
| `outdated` | `array` | Записи, которые установлены, но не соответствуют требованию версии. |
| `missing` | `array` | Записи реестра, не обнаруженные на диске. |
| `install_hints` | `array` | Команда установки для каждой записи `outdated` и `missing`, в порядке реестра. Может содержать дубликаты. |
| `ok_count` | `number` | Длина `ok`. |
| `outdated_count` | `number` | Длина `outdated`. |
| `missing_count` | `number` | Длина `missing`. |
| `_next_action` | `string` | При наличии хотя бы одной подсказки: `Run install hint: <первая подсказка>. Then re-check: forgeplan_plugins_doctor`. При отсутствии: `Done.` |

### Структура записей

Три категории содержат разные наборы ключей - `outdated` использует `required` там, где две другие используют `version_req`.

| Категория | Ключи |
|---|---|
| `ok` | `name`, `detected_version`, `version_req`, `path` |
| `outdated` | `name`, `detected_version`, `required`, `path` |
| `missing` | `name`, `version_req`, `description` |

## Примеры

Диагностика свежей машины - одна запись здорова, одиннадцать отсутствуют:

```json
{}
```

Исторический снимок ответа 0.37.0 с `missing`, сокращённым до трёх записей. Подсказки ниже — сохранённый вывод реестра, не актуальное руководство по установке из публичного marketplace:

```json
{
  "_next_action": "Run install hint: `/plugin install c4-architecture@claude-code-workflows`. Then re-check: `forgeplan_plugins_doctor`.",
  "install_hints": [
    "/plugin install c4-architecture@claude-code-workflows",
    "/plugin install c4-architecture@claude-code-workflows",
    "/plugin install autoresearch@claude-code-workflows",
    "/plugin install agents-pro@ForgePlan-marketplace",
    "/plugin install agents-sparc@ForgePlan-marketplace",
    "/plugin install c4-architecture@claude-code-workflows",
    "/plugin install sparc-specification@ForgePlan-marketplace",
    "/plugin install agents-pro@ForgePlan-marketplace",
    "forgeplan skill install brownfield-docs-pack",
    "/plugin install c4-architecture@claude-code-workflows",
    "/plugin install agent-orchestration@claude-code-workflows"
  ],
  "missing": [
    {
      "description": "C4 context-level diagram agent (via c4-architecture)",
      "name": "c4-context",
      "version_req": ">=1.0"
    },
    {
      "description": "Automated research workflows for greenfield kickoff",
      "name": "autoresearch",
      "version_req": ">=1.0"
    },
    {
      "description": "Brownfield Obsidian/markdown vault migration skill pack",
      "name": "brownfield-docs-pack",
      "version_req": ">=0.1"
    }
  ],
  "missing_count": 11,
  "ok": [
    {
      "detected_version": "<detected on disk>",
      "name": "forgeplan",
      "path": "(built-in)",
      "version_req": "*"
    }
  ],
  "ok_count": 1,
  "outdated": [],
  "outdated_count": 0
}
```

Для внешних записей `detected_version` берётся из манифеста. Встроенная запись `forgeplan` использует версию пакета, скомпилированную в бинарник, путь `(built-in)` и требование `*`. В строках doctor отсутствующая версия равна `null`, поскольку обработчик формирует строки явно; в list/info поле `installed.detected_version` при отсутствии опускается.

Та же проверка из терминала, которая отображает категории как однострочную сводку плюс одну строку на проблему:

```text
$ forgeplan plugins doctor
  ✓ 1 ok, 11 missing, 0 outdated

    ✓ forgeplan                        0.37.0 (req *)

    ✗ agents-sparc-specification       SPARC specification agent (via agents-sparc pack)
        install: /plugin install agents-sparc@ForgePlan-marketplace
    ✗ brownfield-docs-pack             Brownfield Obsidian/markdown vault migration skill pack
        install: forgeplan skill install brownfield-docs-pack

Fix: /plugin install agents-sparc@ForgePlan-marketplace
```

## Примечания

- **Оффлайн-сканирование, не фиксированный порядок.** Сети, менеджера пакетов и LLM нет; результат зависит от файлов и порядка обхода неупорядоченного реестра.
- **Непарсируемая версия считается ok.** Совместимость проверяется с требованием версии реестра; когда любая сторона этого сравнения не может быть распознана как semver, запись попадает в `ok`, а не в `outdated`. Запись, чей манифест вообще не публикует версию, считается `outdated`, а не `ok`.
- **Подсказки не дедуплицируются.** Несколько записей реестра могут указывать на один пакет/команду. Уберите повторы перед показом вариантов установки; число подсказок не равно числу разных пакетов.
- **Порядок категорий и подсказок не стабилен.** Реестр итерируется как неупорядоченная карта, поэтому `missing`, `outdated` и `install_hints` возвращаются в разном порядке при последовательных вызовах - и `_next_action` называет ту подсказку, которая случайно оказалась первой. Сортируйте по `name` перед отображением и никогда не рассматривайте `install_hints[0]` как фиксированный ответ.
- **Запись `missing` - это не ошибка.** Реестр - это каталог того, что ForgePlan *может* использовать; большинство рабочих пространств законно имеют установленное подмножество. Рассматривайте отсутствующий плагин как проблему только тогда, когда плейбук или рабочий процесс, который вы намерены запустить, требует его.
- Команды установки несут явный суффикс `@marketplace` там, где это применимо. Суффикс обязателен - без него установка неоднозначна, когда два маркетплейса публикуют плагин с одинаковым именем.

## Связанные команды

- [`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/) - эквивалент CLI с человекочитаемой сводкой.
- [`forgeplan_plugins_list`](/ru/docs/mcp/forgeplan_plugins_list/) - полная инвентаризация, установленные и отсутствующие.
- [`forgeplan_plugins_info`](/ru/docs/mcp/forgeplan_plugins_info/) - детали для одной записи реестра.
- [`forgeplan_playbook_run`](/ru/docs/mcp/forgeplan_playbook_run/) - потребитель плагинов, которые проверяет этот инструмент.

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_plugins_list`](/ru/docs/mcp/forgeplan_plugins_list/) - инвентаризация перед диагностикой
- [`forgeplan_plugins_info`](/ru/docs/mcp/forgeplan_plugins_info/) - одна запись в деталях
- [`forgeplan_playbook_list`](/ru/docs/mcp/forgeplan_playbook_list/) - какие плейбуки предоставляют установленные пакеты
- [`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/) - CLI-близнец
