// Single source of truth for /guides catalog.
// RU standalone HTML lives in /public/guides-raw/<slug>.html; EN in /public/guides-raw/en/<slug>.html.
// Index pages (EN + RU) iterate this list to render cards.
// Detail pages /guides/<slug> + /ru/guides/<slug> inline the raw HTML at build
// time (extractEmbed); the standalone files stay in dist/guides-raw/ as-is, and
// only the inlined copy rewrites lesson-to-lesson hrefs to catalog routes.

export type Category =
  | 'foundation' // basic concepts
  | 'feature'    // PRD-level
  | 'spec'       // spec-level
  | 'tool'       // forgeplan as tool
  | 'agent'      // AI-agent protocols
  | 'lifecycle'  // state machine
  | 'multi'      // multi-agent
  | 'viz';       // interactive 3D/charts

export type Pill = 'theory' | 'interactive' | 'tool';

export interface Guide {
  slug: string;
  order: number;
  category: Category;
  pill: Pill;
  title_en: string;
  title_ru: string;
  desc_en: string;
  desc_ru: string;
  tag_en: string;
  tag_ru: string;
}

export const guides: Guide[] = [
  {
    slug: 'intro',
    order: 0,
    category: 'foundation',
    pill: 'theory',
    title_en: 'What a decision log is and why a team needs one',
    title_ru: 'Что такое журнал решений и зачем он нужен команде',
    tag_en: 'level zero · part 1 · the basics',
    tag_ru: 'нулевой уровень · часть 1 · база',
    desc_en: 'Decision logs as an engineering practice: ADR and the local six-field DDR convention, six use cases across domains, and preserving the reasoning behind decisions in AI-assisted work.',
    desc_ru: 'Журнал решений как инженерная практика: ADR и локальный шаблон DDR из шести полей, шесть сценариев в разных сферах и сохранение аргументации при работе с AI.',
  },
  {
    slug: 'how-to-use',
    order: 1,
    category: 'foundation',
    pill: 'theory',
    title_en: 'How to introduce decision-logging into your team',
    title_ru: 'Как ввести практику журналирования решений в команде',
    tag_en: 'level zero · part 2 · practice',
    tag_ru: 'нулевой уровень · часть 2 · практика',
    desc_en: 'When to write a record and when to skip it. A local depth scale, an illustrative working day, three ways to give an AI agent access to context, five agent–record interactions, and anti-patterns. Access does not guarantee automatic reading.',
    desc_ru: 'Когда писать запись, а когда нет. Локальная шкала глубины, пример рабочего дня, три способа дать AI-агенту доступ к контексту, пять сценариев работы с записями и антипаттерны. Доступ не гарантирует автоматического чтения.',
  },
  {
    slug: 'artifacts-overview',
    order: 2,
    category: 'foundation',
    pill: 'theory',
    title_en: 'How to manage decision artifacts in a team',
    title_ru: 'Как вести артефакты команды',
    tag_en: 'overview · artifacts',
    tag_ru: 'обзор · артефакты',
    desc_en: 'A local writing convention for team artifacts: three identifying properties, one file per responsibility, a working rhythm, three signs of a maintained record, and five anti-patterns.',
    desc_ru: 'Локальные правила ведения артефактов команды: три признака артефакта, один файл на одну ответственность, рабочий ритм, три признака поддерживаемой записи и пять антипаттернов.',
  },
  {
    slug: 'adi-overview',
    order: 3,
    category: 'foundation',
    pill: 'theory',
    title_en: 'ADI as a daily practice: three versions before fixing',
    title_ru: 'ADI как ежедневная практика: трёхверсионный анализ',
    tag_en: 'overview · adi',
    tag_ru: 'обзор · adi',
    desc_en: 'Abduction → Deduction → Induction as a daily reasoning habit. Write three alternative explanations before choosing a fix: a local anti-anchoring heuristic, not a statistical threshold. When to apply it and a worked cycle.',
    desc_ru: 'Абдукция → дедукция → индукция как ежедневная практика рассуждения. Три альтернативных объяснения перед выбором исправления — локальный приём против первой догадки, а не статистический порог. Когда применять и как выглядит один цикл.',
  },
  {
    slug: 'ddr-format',
    order: 4,
    category: 'foundation',
    pill: 'theory',
    title_en: 'What a good DDR looks like: template with full example',
    title_ru: 'Как выглядит хороший DDR — шаблон с примером',
    tag_en: 'overview · ddr',
    tag_ru: 'обзор · ddr',
    desc_en: 'DDR as a local six-field writing convention, not an external standard or a separate artifact kind. ADR comparison, a worked JWT/session/OAuth scenario, and a suggested 200–400-word length rather than a universal limit.',
    desc_ru: 'DDR как локальный шаблон из шести полей, а не внешний стандарт или отдельный тип артефакта. Сравнение с ADR, учебный сценарий JWT/сессии/OAuth и рекомендуемые 200–400 слов, а не универсальный лимит.',
  },
  {
    slug: 'fpf-overview',
    order: 5,
    category: 'foundation',
    pill: 'theory',
    title_en: 'FPF: the consistency map',
    title_ru: 'FPF: карта согласованности',
    tag_en: 'overview · fpf',
    tag_ru: 'обзор · fpf',
    desc_en: 'Seven selected FPF concepts: holons, context, distinctions, evidence ageing, open/closed world, trust, and reasoning. An overview selection, not an exhaustive kernel; local numerical trust rubrics are distinguished from upstream definitions.',
    desc_ru: 'Семь выбранных понятий FPF: холоны, контекст, различения, старение доказательств, открытый/закрытый мир, доверие и рассуждение. Обзорная подборка, не полное ядро; локальные численные шкалы отделены от определений первоисточника.',
  },
  {
    slug: 'trust-calculus-overview',
    order: 6,
    category: 'foundation',
    pill: 'theory',
    title_en: 'Trust Calculus: F/G/R and its connection to DDR',
    title_ru: 'Исчисление доверия: оценка по трём осям',
    tag_en: 'overview · trust calculus',
    tag_ru: 'обзор · исчисление доверия',
    desc_en: 'A local numerical teaching model of F/G/R, weakest-link aggregation, and context penalties, distinct from upstream FPF and implemented scoring. F/G/R appears in section three of the local DDR convention. A bar chart contrasts average and minimum.',
    desc_ru: 'Локальная численная учебная модель F/G/R, слабого звена и штрафов контекста — отдельно от FPF и реализованного расчёта. F/G/R находится в третьей секции локального шаблона DDR. Диаграмма сравнивает среднее и минимум.',
  },
  {
    slug: 'evidence-and-decay',
    order: 7,
    category: 'foundation',
    pill: 'theory',
    title_en: 'Evidence and expiry: why support needs revisiting',
    title_ru: 'Доказательства и срок годности: зачем пересматривать основания',
    tag_en: 'overview · evidence',
    tag_ru: 'обзор · доказательства',
    desc_en: 'Evidence types, local F/G/R assessment, and review triggers. Illustrative gradual reassessment is distinguished from date-based runtime expiry: the ordinary score holds until expiry, then becomes 0.1; renewing a date is not a new measurement.',
    desc_ru: 'Типы доказательств, локальная оценка F/G/R и условия пересмотра. Учебное постепенное переосмысление отделено от расчёта по дате: обычная оценка сохраняется до истечения срока, затем становится 0.1; новая дата не заменяет новый замер.',
  },
  {
    slug: 'decision-cycle',
    order: 8,
    category: 'foundation',
    pill: 'theory',
    title_en: 'How decisions are made and evidence weighted',
    title_ru: 'Цикл одного решения · ADI · DDR · исчисление доверия',
    tag_en: 'detailed · foundation',
    tag_ru: 'детальный · основа',
    desc_en: 'ADI reasoning (Abduction → Deduction → Induction), a local DDR writing convention, and an illustrative trust rubric. Why average and minimum answer different questions; a teaching model is not an upstream formula or calibrated probability.',
    desc_ru: 'Цикл ADI (абдукция → дедукция → индукция), локальный шаблон DDR и учебная шкала доверия. Почему среднее и минимум отвечают на разные вопросы; учебная модель — не формула первоисточника и не калиброванная вероятность.',
  },
  {
    slug: 'trust-calculus',
    order: 9,
    category: 'viz',
    pill: 'interactive',
    title_en: 'Trust Calculus in 3D F/G/R',
    title_ru: 'Исчисление доверия в 3D F/G/R',
    tag_en: 'companion · interactive',
    tag_ru: 'приложение · интерактив',
    desc_en: 'A rotatable 3D teaching scene with seven evidence items in an illustrative /search latency scenario. F+G+R=12 is a sum threshold, not weakest-link aggregation; hover cards and authored hypothesis selection illustrate local rules, not runtime scoring.',
    desc_ru: 'Вращаемая учебная 3D-сцена с семью свидетельствами в примере задержки /search. F+G+R=12 — порог суммы, а не слабого звена; карточки и заданный автором выбор гипотезы показывают локальные правила, а не рабочий алгоритм скоринга.',
  },
  {
    slug: 'bmad-cycle',
    order: 10,
    category: 'feature',
    pill: 'theory',
    title_en: 'BMAD · local PRD template · adversarial review',
    title_ru: 'BMAD · локальный шаблон PRD · критическое ревью',
    tag_en: 'one feature',
    tag_ru: 'одна фича',
    desc_en: 'Upstream BMAD phases: optional Analysis, Planning, Solutioning, Implementation. A local 13-section PRD exercise and three highlighted review sections are distinguished from upstream requirements and the depth-dependent product validator.',
    desc_ru: 'Фазы BMAD: необязательный анализ (Analysis), затем планирование (Planning), проработка решения (Solutioning) и реализация (Implementation). Локальный учебный PRD из 13 секций и три выделенные секции ревью отделены от требований первоисточника и продуктового валидатора, зависящего от глубины.',
  },
  {
    slug: 'spec-cycle',
    order: 11,
    category: 'spec',
    pill: 'theory',
    title_en: 'Spec · Delta · R_eff',
    title_ru: 'Spec · Delta · R_eff',
    tag_en: 'one specification',
    tag_ru: 'одна спецификация',
    desc_en: 'Spec vs PRD: verifiable behavior, not intent. ADDED/MODIFIED/REMOVED express requirement deltas. Weakest-link evidence aggregation, expiry, and illustrative reassessment are separated rather than treated as one universal decay formula.',
    desc_ru: 'Spec против PRD: проверяемое поведение, а не намерение. ADDED/MODIFIED/REMOVED описывают изменения требований. Агрегация по слабому звену, истечение срока и учебный пересмотр оценки разделены, а не выданы за одну универсальную формулу старения.',
  },
  {
    slug: 'lifecycle-cycle',
    order: 12,
    category: 'lifecycle',
    pill: 'theory',
    title_en: 'Lifecycle · from draft to terminal',
    title_ru: 'Жизненный цикл · от draft до конечного состояния',
    tag_en: 'after activate',
    tag_ru: 'что после activate',
    desc_en: 'Lifecycle vocabulary: draft, active, superseded, deprecated, stale. Distinguishes stored status from an expiry warning; renew is explicit, reopen creates a new draft with lineage, and active files are not technically immutable.',
    desc_ru: 'Жизненный цикл: draft, active, superseded, deprecated, stale. Сохранённый статус отделён от предупреждения об истечении срока; renew выполняется явно, reopen создаёт новый черновик с историей происхождения, а active не делает файл технически неизменяемым.',
  },
  {
    slug: 'fpf-cycle',
    order: 13,
    category: 'foundation',
    pill: 'theory',
    title_en: 'FPF · First Principles Framework as the parent rail',
    title_ru: 'FPF · First Principles Framework как родительская рамка',
    tag_en: 'meta framework',
    tag_ru: 'мета-фреймворк',
    desc_en: 'Six selected FPF concepts connect context, trust, reasoning, category distinctions, and composition. Local plugin modes (decompose / evaluate / reason / lookup) and numerical rubrics are not an exhaustive account of upstream FPF or the product CLI.',
    desc_ru: 'Шесть выбранных понятий FPF связывают контекст, доверие, рассуждение, различение категорий и композицию. Режимы локального плагина (decompose / evaluate / reason / lookup) и численные шкалы — не полное описание FPF и не команды продуктового CLI.',
  },
  {
    slug: 'dag-explorer',
    order: 14,
    category: 'viz',
    pill: 'interactive',
    title_en: 'DAG Explorer · artifacts in 3D',
    title_ru: 'DAG Explorer · артефакты в 3D',
    tag_en: 'companion · interactive',
    tag_ru: 'приложение · интерактив',
    desc_en: 'Plotly 3D teaching dataset for a Knowledge Vault scenario: 24 artifacts, 23 logical edges, and 29 rendered segments when all types are visible because six evidence links are drawn twice. Type filters show chains, blind spots, orphans, and branches.',
    desc_ru: 'Учебный набор Plotly 3D для сценария Knowledge Vault: 24 артефакта, 23 логических ребра и 29 отрисованных сегментов при видимости всех типов, поскольку шесть связей evidence рисуются дважды. Фильтры показывают цепочки, слепые зоны, сирот и ветвления.',
  },
  {
    slug: 'depth-calibrator',
    order: 15,
    category: 'tool',
    pill: 'interactive',
    title_en: 'Depth Calibrator · routing in real-time',
    title_ru: 'Калибратор глубины · маршрутизация в реальном времени',
    tag_en: 'companion · interactive',
    tag_ru: 'приложение · интерактив',
    desc_en: 'An interactive teaching heuristic with complexity, blast-radius, and reversibility sliders plus public-API, security, and cross-team switches. Presets change the displayed depth and pipeline; this browser model is not the runtime maximum-signal router.',
    desc_ru: 'Интерактивная учебная эвристика со слайдерами сложности, масштаба влияния и обратимости, а также переключателями public API, безопасности и нескольких команд. Пресеты меняют глубину и цепочку артефактов; модель в браузере — не рабочий роутер по максимальному сигналу.',
  },
  {
    slug: 'agent-protocol-cycle',
    order: 16,
    category: 'agent',
    pill: 'theory',
    title_en: 'Hint Contract · 5 markers',
    title_ru: 'Контракт подсказок · 5 маркеров',
    tag_en: 'agent ↔ tool language',
    tag_ru: 'язык агента ↔ инструмента',
    desc_en: 'A proposed agent–tool hint contract: Next: / Or: / Wait: / Done. / Fix: and five rules for actionable responses. These markers guide next steps; they neither eliminate hallucinations nor establish universal runtime coverage.',
    desc_ru: 'Предлагаемый контракт подсказок между агентом и инструментом: Next: / Or: / Wait: / Done. / Fix: и пять правил выполнимого ответа. Маркеры направляют следующий шаг, но не устраняют галлюцинации и не доказывают полного покрытия всех ответов.',
  },
  {
    slug: 'multi-agent-cycle',
    order: 17,
    category: 'multi',
    pill: 'theory',
    title_en: 'Multi-agent · dispatch · claim · release',
    title_ru: 'Несколько агентов · dispatch · claim · release',
    tag_en: 'parallel work',
    tag_ru: 'параллельная работа',
    desc_en: 'An illustrative 2–5-agent workflow using dispatch, advisory claims with TTL, and release. File-overlap heuristics do not guarantee conflict-free execution. WIP=1 and a separate reviewer are local workflow rules, not enforced claim limits; the core supports up to 64 agents.',
    desc_ru: 'Пример работы 2–5 агентов с dispatch, рекомендательными claim-маркерами с TTL и release. Эвристика пересечения файлов не гарантирует отсутствия конфликтов. WIP=1 и отдельный ревьюер — локальные правила, не ограничения claim; ядро поддерживает до 64 агентов.',
  },
  {
    slug: 'forgeplan-cycle',
    order: 18,
    category: 'tool',
    pill: 'tool',
    title_en: 'Forgeplan as the operational layer',
    title_ru: 'Forgeplan как рабочий слой методики',
    tag_en: 'the tool itself',
    tag_ru: 'сам инструмент',
    desc_en: 'Forgeplan as one CLI/MCP implementation of decision records, evidence, typed links, validation, and lifecycle operations. Product FGR heuristics, recursive R_eff, flat display scores, routing, and activation gates have distinct contracts; captured help is not runtime proof.',
    desc_ru: 'Forgeplan как одна реализация журнала решений, доказательств, типизированных связей, валидации и жизненного цикла через CLI/MCP. Продуктовые эвристики FGR, рекурсивный R_eff, плоские оценки в выдаче, роутинг и условия активации имеют разные контракты; снимок help не доказывает поведение.',
  },
  {
    slug: 'first-artifact',
    order: 19,
    category: 'tool',
    pill: 'interactive',
    title_en: 'First Artifact · interactive walkthrough',
    title_ru: 'Первый артефакт · интерактивный разбор',
    tag_en: 'tutorial',
    tag_ru: 'практический разбор',
    desc_en: 'An authored terminal simulation: init → health → route → new prd → validate → reason (ADI) → build → new evidence → link + score → activate. Clickable progress, a simulated workspace panel, and error examples; displayed results are not a verified CLI transcript.',
    desc_ru: 'Авторская симуляция терминала: init → health → route → new prd → validate → reason (ADI) → build → new evidence → link + score → activate. Кликабельный прогресс, имитация панели проекта и примеры ошибок; показанные результаты — не проверенная запись CLI.',
  },
];

export function getGuide(slug: string): Guide | undefined {
  return guides.find(g => g.slug === slug);
}
