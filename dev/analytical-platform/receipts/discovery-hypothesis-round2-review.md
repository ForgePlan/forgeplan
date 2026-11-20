# Round 2 — независимое закрывающее ревью Discovery

**Content verdict: PASS.** C1–C4 и восемь предметных замечаний первого раунда закрыты в проверенной редакции.

Это PASS качества Discovery-документов и протоколов будущей проверки, **не подтверждение гипотез, runtime-поведения или рыночного спроса**. Каноническая запись Profile B EVID ещё необходима для формального Define handoff.

**Ревьюер:** `DiscoveryHypothesisReview`  
**Фактическая модель:** `openai-codex/gpt-6-astra`  
**Раунд:** второй, после содержательных исправлений первого. Авторский ADI не засчитывался как независимое ревью.

## 1. Проверенная версия

Проверены SHA-256:

- Round 1 report: `27265a266bbd684e90ff2b06694470d1242ce2e732dd6dc9e1eaff75a3921823`
- `/Users/explosovebit/Work/ForgePlan-Analytical-Planning/discovery-final-handoff.json`:  
  `15baa32f3fbaa59cf35874249c494fd716a508fed432bc82ea71af5ad30ea137`
- Selective export: `70412a92801dffe6c1fca3f34a55f20a5663e542ec0decb5a3ab079154cb88fe`
- Full canonical export: `8d1f966cfaca597b2ea9c6013f515d77a5d15cf5d01ee5c47ddbb8212e7f1b6c`

Все десять абсолютных путей и canonical keys соответствуют handoff. Ниже `ID:строка` обозначает **точный файл из этого проверенного manifest**; пути не изменились относительно карты файлов Round 1.

| Артефакт | Проверенный file SHA-256 | Проверенный body SHA-256 |
|---|---|---|
| NOTE-067 | `1054167047e579b795bb9dea251d95b6c30a573bfe4df991977273876aac6eab` | `dd08bc71759fcd792cb92acf32675a7f034c64c0cbb21185f649b687a4bb8e61` |
| PROB-107 | `294a846f8752167ea85214e5ea156dcb927dbeeaa322b880c6b40700d4b2d5d6` | `a98f728de0de557ae1a2e189344bbcca821cf5e613c1ab221decfccacf414865` |
| HYP-001 | `b2941aa78f0e7a177ece3f886d28860af550a5dc58443a02a050f1165583a3bb` | `4c14c0b560292341742ef40e4edc12429a938875879f37a236e391ced70490a7` |
| HYP-002 | `32e8e8bf7c3ab8c8b745621cbfcbd5b0fedfd88dff40f2a4d3c539c03bfd85b5` | `ad4d567ee35c8df11756dad7f0c13f418291515fb26dab491f7c557afc157caf` |
| HYP-003 | `6365eb15fca5b90347fb666758f50633d23532ae03f30aad1d5ed67a117178ee` | `ac5ba0b98c8dfa726032804b85d698bb2dd26de47d848153b276c9afae276358` |
| HYP-004 | `837ca738720d6a7dca052c806eaa91b305ae5996dc28f6c56d88b1fbd7019b6e` | `7546da8b5236ebc1c54327601cb5f84267946b3da81bfcaf8030b1dcba7b6608` |
| HYP-005 | `50423914396ec0818a2f79e1d4fec1dc2a18ca4276a3cae19ff53b39f5dbb9b2` | `c7d5ae2c730f9ac9be8a9d35ed480a5dd4d274eb0c161c35e215dca6354985ec` |
| HYP-006 | `f9c1325540c8cc7ab41705740c2917bea8b5d01175c541adbb9d31e3a5849251` | `a4c6060de445dbb2d3092d9acc22af9e6daa9204df6fb017f65ae56896071300` |
| HYP-007 | `936ef92abab97e75a707ac6376eb7785e8c582815b8eea374cc38ed061eb8a7d` | `a650486df849d4c6a6efaa046310c6e7f5b6f3ecc4c9e0f83d9489cb050ad461` |
| HYP-008 | `4069e9b5ba6575920effa675e7a7216461f506d7cea95ab55ca8a82c3af2df7b` | `4fb7fbcb1755c4d65cb19f98b36541415c8b4e38f2cd8faa7a27c5586f5575ec` |

Дополнительно установлено:

- Все десять `export_body_sha256` совпадают с manifest; экспортированные тела совпадают с телами проекций после удаления краевых пробелов.
- Все десять записей selective export совпадают с соответствующими записями полного экспорта.
- Все 13 отношений присутствуют в полном экспорте. Четыре внешние цели NOTE разрешаются по точным canonical keys.
- Все десять validation logs совпадают по хешам и содержат `passed: true`, ноль findings/errors/warnings внутри результатов.
- Валидатор выводит **Deep**, хотя raw metadata и экспорт содержат `critical`. Это **не автоматический Critical PASS**.
- Хеши stdout/stderr финального ADI совпадают с manifest. Его текст прямо ограничивает анализ dossier/context, без независимого чтения финальных HYP.

Неизменённые D/R/A целиком повторно не читались; использован предметный baseline первого раунда.

## 2. C1–C4: закрытие

| Замечание | Вердикт | Проверенное исправление |
|---|---|---|
| **C1 — обоснование и заморозка порогов** | **Закрыто** | `NOTE-067:167–173` задаёт единицу эксперимента, знаменатели, конфигурации, некомпенсируемые safety ceilings, baseline-derived полезный эффект и полную стоимость, основания для sample/horizon, разделение calibration/confirmation и версионируемую preregistration. Изменение правил после просмотра результатов не превращается в успех. `PROB-107:87` делает протокол нормативным. Все восемь HYP ссылаются на него в `:51`. |
| **C2 — reading order и документация** | **Закрыто** | `NOTE-067:175` назначает HYP-001 владельцем task-level последствий, HYP-002 — фактов и полноты. Включены tight-budget потеря обязательного ADR, циклы/направление обхода, missing oracle-positive facts, stale docs и сохранность ручного/нормативного содержания. Эти случаи внесены непосредственно в `HYP-001:49` и `HYP-002:49`. |
| **C3 — согласованность принятия между слоями** | **Закрыто** | `NOTE-067:177` связывает identities, revisions/dirty digests, configuration, analyzer/producer, protected policy и evidence provenance по цепочке storage→federation→query→report/UI. Несовпадение должно оставаться rejected/incomplete сквозным образом. Каждый HYP наследует проверку через `:51`; один общий прогон не становится несколькими независимыми доказательствами. |
| **C4 — запрет переноса вывода между гипотезами** | **Закрыто** | `NOTE-067:179` разделяет task benefit, interface benefit и adoption; раскрывает корреляцию участников/задач; не приравнивает аутентичное выполнение к parity; не делает выбор SQLite обязательным условием аналитической системы. Правила отражены в предметных `HYP:51` и `PROB-107:87`. |

Нормативная цепочка согласована: все HYP `based_on` PROB-107; PROB-107 `based_on` NOTE-067; прямые текстовые ссылки HYP ведут к реально существующему разделу §6 общего протокола. Обратный индекс `NOTE-067:181–189` содержит точные восемь keys и корректные предметные обязанности.

## 3. Восемь предметных случаев

| HYP | Content verdict | Закрытие риска первого раунда |
|---|---|---|
| **001 — полезность интеграции** | **PASS** | `:49` явно вводит три самостоятельные ветви, включая **без ForgePlan**, равный доступ/обучение и отдельные результаты сценариев. Критический провал одного сценария нельзя скрыть средним выигрышем. Закрыт риск победы лишь над неудобной интеграцией вместо обычной практики. |
| **002 — семантика и incremental** | **PASS** | `:49` требует независимые положительные/отрицательные ожидаемые факты и полные знаменатели. Молчаливое отсутствие ожидаемого факта теперь является completeness failure, даже если full и incremental одинаково ошибаются. Уточнены materiality, точный Rust/config envelope и смысл method/trait ambiguity. `:71` сохраняет shared-defect counterexample. |
| **003 — федеративная совместимость** | **PASS** | `:49` добавляет unchanged-schema behavioral break, безвредное структурное изменение, независимо ожидаемый bounded consumer inventory, матрицу версий, rollout order и backward/forward expectations. Закрыт риск пропустить смену единиц/семантики при неизменных типах. |
| **004 — доверие к evidence** | **PASS** | `:49` проверяет trusted signing/upload wrapper с attacker-authored JUnit без исполнения, подмену отчёта и противоречие exit/count/report. Принятие связано с фактическим разрешённым producer и protected policy. Отсутствующие required checks не проходят из-за сокращённого impact-плана. Ограниченность защиты от compromised executor названа прямо. |
| **005 — legacy parity и recovery** | **PASS** | `:49` требует state/effect ledger, новые записи, in-flight/retry, point of no return и разрешённую компенсацию. Recovery проверяется **после** новых записей/внешних эффектов, включая old-reader compatibility и duplicate-effect prevention. Нормализация недетерминизма и approved divergences фиксируются до просмотра результатов замены. |
| **006 — storage candidate** | **PASS** | `:49` перечисляет классы authority и требует identity/content/history/exception-expiry retention для каждого durable класса при publish failure, upgrade и recovery. Стоимость включает migration, parallel operation, recovery и maintenance. Восстановление одного поиска больше не удовлетворяет протоколу. |
| **007 — triage и UI** | **PASS** | `:49` включает постоянный поток новых карточек, обоснованный visibility/escalation bound и horizon. Неизвестная зависимость не позволяет считать работу изолированной. Между list/diff/2D/3D/agent закреплены общие версии и контекст. Закрыты конкретные риски starvation и ложной независимости. |
| **008 — adoption** | **PASS** | `:49` задаёт problem-based eligibility, разные user/purchasing/security/deployment роли, meaningful repeat-use cycle, отказы/dropouts/attrition со знаменателями. В полной стоимости остаются помощь, onboarding, privacy/integration и maintenance; ушедшие пользователи не исключаются. Вывод ограничен исследованными сегментами. |

Старые разделы Falsification сохранены, а новые обязательные negative cases добавлены в How To Verify. Они не заменяют исходную проверку и не ослабляют её.

## 4. Статусы, неизвестное и новые findings

**Новых материальных findings в проверенном delta нет.**

Разделы `Evidence For/Against` всех HYP (`:65–71`) описывают основания для исследования, ограничения и ещё не проверенные контрпримеры. Они не утверждают выполненных экспериментов.

Раздел `Lifecycle State` всех HYP (`:73–75`) корректно различает:

- наблюдаемое CLI-generated `inferred`;
- отсутствие выполненного promotion;
- недоступность runtime lifecycle API;
- неизменный статус claims как unvalidated и experiments NOT_RUN.

Это согласовано с `NOTE-067:191,221`. Наличие обязательного заголовка не используется для имитации перехода состояния.

Остаются неизвестными конкретный корпус, конфигурационные профили, участники, численные пороги, sample/horizon, trust identities и результаты экспериментов (`NOTE-067:200–209`; HYP `:51,63`). **Для Discovery это допустимые открытые параметры:** теперь заданы основания выбора, ответственность за согласование и требование заморозки до confirmatory execution. Их отсутствие не требует выдумывать результаты и не блокирует содержательный Define handoff.

## 5. Граница разрешения

- **Содержательная часть этого второго раунда завершена: PASS.**
- После канонической записи независимого Profile B EVID и сверки координатором полного второго раунда возможен Define handoff. Runtime-эксперименты не являются дополнительным условием принятия этих Discovery-документов.
- Автоматическая валидация Deep, авторский ADI и этот отчёт по отдельности не заменяют формальный Critical gate.
- Активация, реализация, подтверждение гипотез и последующие PRD/architecture gates этим отчётом не разрешаются.

Проводилось только чтение документов, сравнение хешей и сохранённых receipts/экспортов. Записей, CLI-мутаций, builds/tests/formatters, cleanup или activation не было.

**Подпись:** `DiscoveryHypothesisReview` · `openai-codex/gpt-6-astra` · **Round 2 content PASS**