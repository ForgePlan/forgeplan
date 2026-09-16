---
title: forgeplan_plugins_doctor
description: "Проверка здоровья всего реестра плагинов: разделяет записи на ok / устаревшие / отсутствующие и предоставляет подсказки по установке для отсутствующих. PRD-067 AC-2."
---

Проверка здоровья всего реестра плагинов: разделяет записи на ok / устаревшие / отсутствующие и предоставляет подсказки по установке для отсутствующих. PRD-067 AC-2.

**Категория**: Прочее

Инструмент диагностики для неисправного окружения. Если [`forgeplan_plugins_list`](/ru/docs/mcp/forgeplan_plugins_list/) отвечает на вопрос "что здесь есть", этот инструмент отвечает на вопрос "что не так и что нужно вставить, чтобы исправить". Каждая запись в расширенном реестре классифицируется ровно в одну из трёх категорий, и каждая запись, не попавшая в `ok`, добавляет исполняемую команду установки в `install_hints`.

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

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| (нет) | - | - | Инструмент не принимает параметров. Он только для чтения, идемпотентный, неразрушающий и работает в закрытом мире. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::forgeplan_plugins_doctor` (сигнатура `Parameters(_p): Parameters<EmptyParams>`)_

## Возвращает

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

Ответ (захвачен в реальном времени, с массивом `missing`, сокращённым до трёх записей):

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

`detected_version` читается из установленного манифеста во время сканирования, поэтому отражает плагин на диске, а не бинарник, обслуживающий запрос - вот почему здесь он показан как заполнитель. Встроенная запись `forgeplan` разрешается в `(built-in)` с требованием `*`.

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

- **Оффлайн и детерминированный.** Обнаружение - это сканирование файловой системы по статическому реестру. Никаких сетевых вызовов, менеджера пакетов или LLM.
- **Непарсируемая версия считается ok.** Совместимость проверяется с требованием версии реестра; когда любая сторона этого сравнения не может быть распознана как semver, запись попадает в `ok`, а не в `outdated`. Запись, чей манифест вообще не публикует версию, считается `outdated`, а не `ok`.
- **`install_hints` - это действенная часть.** Это плоский список, дедуплицированный только случайно из-за реестра - отфильтруйте его сами перед показом пользователю одиннадцати строк, которые на самом деле являются четырьмя различными командами.
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