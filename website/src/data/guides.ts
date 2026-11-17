// Single source of truth for /guides catalog.
// Each lesson is a standalone HTML in /public/guides-raw/<slug>.html.
// Index pages (EN + RU) iterate this list to render cards.
// Detail pages /guides/<slug> + /ru/guides/<slug> iframe the raw HTML.

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
    tag_en: 'decision logs · part 1 · basics',
    tag_ru: 'журнал решений · часть 1 · основы',
    desc_en: 'Decision logging as an engineering practice independent of any tool: ADR and DDR formats, six mandatory record fields, who needs a log, six use cases across domains, and why it matters more when working with AI.',
    desc_ru: 'Журнал решений как инженерная практика, не зависящая от выбора инструмента: форматы ADR и DDR, шесть обязательных полей записи, кому нужен журнал, шесть примеров его применения в разных сферах и почему он важен при работе с AI.',
  },
  {
    slug: 'how-to-use',
    order: 1,
    category: 'foundation',
    pill: 'theory',
    title_en: 'How to introduce decision logging in your team',
    title_ru: 'Как начать вести журнал решений в команде',
    tag_en: 'decision logs · part 2 · practice',
    tag_ru: 'журнал решений · часть 2 · практика',
    desc_en: 'When to write a record and when to skip it (four rules), how to choose task depth, and what a typical day looks like. Three ways an AI agent reads your context, five ways it works with records, and anti-patterns to avoid.',
    desc_ru: 'Когда делать запись, а когда нет, как выбрать глубину проработки задачи и как выглядит типичный день инженера. Три способа дать AI-агенту прочитать контекст, пять сценариев работы агента с записями и антипаттерны.',
  },
  {
    slug: 'artifacts-overview',
    order: 2,
    category: 'foundation',
    pill: 'theory',
    title_en: 'How to manage decision artifacts in a team',
    title_ru: 'Как вести артефакты решений в команде',
    tag_en: 'overview · artifacts',
    tag_ru: 'обзор · артефакты',
    desc_en: 'What makes something an artifact (3 properties), the rule that each file has one responsibility, the rhythm of work, three signs of an artifact in active use, and 5 anti-patterns. Links to the formal decision cycle and artifact lifecycle.',
    desc_ru: 'Три признака артефакта, правило «у каждого файла одна ответственность», ритм работы, три признака артефакта, которым пользуются, и 5 антипаттернов. Ссылки на формальный цикл принятия решений и жизненный цикл артефакта.',
  },
  {
    slug: 'adi-overview',
    order: 3,
    category: 'foundation',
    pill: 'theory',
    title_en: 'ADI in daily work: three possible causes before a fix',
    title_ru: 'ADI в ежедневной работе: три версии причины',
    tag_en: 'overview · ADI',
    tag_ru: 'обзор · ADI',
    desc_en: 'How to make writing down three possible causes a daily habit. The technique comes from 19th-century medical diagnostics. When to use it, a walkthrough of one cycle, why three is the statistical threshold, and links to the formal cycle.',
    desc_ru: 'Как сделать приём «выпиши три версии причины» ежедневной привычкой. Его истоки в медицинской диагностике XIX века. Когда применять, как проходит один цикл, почему именно три (статистический порог). Ссылки на формальный цикл.',
  },
  {
    slug: 'ddr-format',
    order: 4,
    category: 'foundation',
    pill: 'theory',
    title_en: 'What a good DDR looks like: a template with a full example',
    title_ru: 'Как выглядит хороший DDR: шаблон с примером',
    tag_en: 'overview · DDR',
    tag_ru: 'обзор · DDR',
    desc_en: 'DDR and ADR compared, six mandatory fields, and a full Markdown example comparing JWT, sessions, and OAuth. Why a DDR should fit in 200–400 words on one screen. Links to the formal decision cycle and artifact lifecycle.',
    desc_ru: 'Сравнение DDR и ADR, шесть обязательных полей и полный пример на Markdown с выбором между JWT, сессиями и OAuth. Почему DDR стоит уместить в 200–400 слов на одном экране. Ссылки на формальный цикл принятия решений и жизненный цикл артефакта.',
  },
  {
    slug: 'fpf-overview',
    order: 5,
    category: 'foundation',
    pill: 'theory',
    title_en: 'FPF: the consistency map',
    title_ru: 'FPF: карта согласованности',
    tag_en: 'overview · FPF',
    tag_ru: 'обзор · FPF',
    desc_en: 'Seven concepts at the core of FPF: holons, context, distinctions, decay, open/closed world, F-G-R trust, and the reasoning cycle. How they connect and why they need to stay consistent.',
    desc_ru: 'Семь понятий в основе FPF: холоны, контекст, различения, срок годности, открытый/закрытый мир, доверие F-G-R и цикл рассуждения. Как они связаны и зачем поддерживать их согласованность.',
  },
  {
    slug: 'trust-calculus-overview',
    order: 6,
    category: 'foundation',
    pill: 'theory',
    title_en: 'Trust Calculus: F/G/R and its connection to DDR',
    title_ru: 'Trust Calculus: оценка доверия по трём осям',
    tag_en: 'overview · Trust Calculus',
    tag_ru: 'обзор · Trust Calculus',
    desc_en: 'Three axes of Trust Calculus (Formal / Granular / Reliable), the weakest-link rule rather than an average, context-level penalties, the R_eff formula in one line, and where F/G/R belongs in a DDR. A bar chart compares the average with the minimum. Links to the 3D interactive.',
    desc_ru: 'Три оси Trust Calculus (F/G/R), правило слабого звена вместо среднего, штраф за уровень контекста, формула R_eff одной строкой и место F/G/R в шестой секции DDR. Столбчатая диаграмма сравнивает среднее с минимумом. Ссылки на 3D-интерактив.',
  },
  {
    slug: 'evidence-and-decay',
    order: 7,
    category: 'foundation',
    pill: 'theory',
    title_en: 'Evidence and decay: why proofs have a shelf life',
    title_ru: 'Evidence и Decay: почему у доказательств есть срок годности',
    tag_en: 'overview · evidence',
    tag_ru: 'обзор · evidence',
    desc_en: 'Types of evidence: your own measurements, a peer benchmark, vendor docs, and experience from another team. Three dimensions of strength (a brief F/G/R recap), the decay formula that describes how evidence ages, and decay-based revisit triggers. Links to trust-calculus and lifecycle.',
    desc_ru: 'Типы доказательств: свой замер, бенчмарк коллеги, документация поставщика и опыт чужой команды. Три измерения силы (краткий обзор F/G/R), формула decay, которая описывает старение доказательств, и триггеры пересмотра по decay. Ссылки на trust-calculus и lifecycle.',
  },
  {
    slug: 'decision-cycle',
    order: 8,
    category: 'foundation',
    pill: 'theory',
    title_en: 'How decisions are made and evidence is weighed',
    title_ru: 'Цикл одного решения: ADI, DDR и Trust Calculus',
    tag_en: 'in depth · foundations',
    tag_ru: 'подробно · основы',
    desc_en: 'The ADI reasoning cycle (Abduction → Deduction → Induction), the DDR field on every ADR, and Trust Calculus. Why trust scores should follow the weakest-link principle rather than an average.',
    desc_ru: 'Цикл ADI (Abduction → Deduction → Induction), DDR-поле на каждом ADR и Trust Calculus для численной оценки доверия к источнику. Почему средняя оценка доверия обманывает.',
  },
  {
    slug: 'trust-calculus',
    order: 9,
    category: 'viz',
    pill: 'interactive',
    title_en: 'Trust Calculus in 3D: F/G/R',
    title_ru: 'Trust Calculus в 3D: F/G/R',
    tag_en: 'companion · interactive',
    tag_ru: 'приложение · интерактив',
    desc_en: 'A 3D scene with seven pieces of evidence for a /search latency scenario. The F+G+R=12 threshold plane, cards on hover, and hypothesis selection using WLNK. Rotate the scene with the mouse; it is ready for teaching with a projector.',
    desc_ru: '3D-сцена с семью доказательствами для сценария /search latency. Плоскость порога F+G+R=12, карточки при наведении и выбор гипотезы по WLNK. Сцену можно вращать мышью и показывать на проекторе.',
  },
  {
    slug: 'bmad-cycle',
    order: 10,
    category: 'feature',
    pill: 'theory',
    title_en: 'BMAD: 13-step PRD and adversarial review',
    title_ru: 'BMAD: PRD из 13 шагов и adversarial review',
    tag_en: 'one feature',
    tag_ru: 'одна фича',
    desc_en: 'Four funnel phases (Brainstorming → Modeling → Architecting → Delivery), 13 PRD sections with 3 critical ones (Non-Goals, Risks, Open Questions), and adversarial review by a fresh reviewer as the third quality gate.',
    desc_ru: 'Четыре фазы воронки (Brainstorming → Modeling → Architecting → Delivery), 13 секций PRD, из которых 3 критичны (Non-Goals, Risks, Open Questions), и adversarial review другим рецензентом как третья проверка качества.',
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
    desc_en: 'How a Spec describes verifiable behavior while a PRD describes intent. The ADDED/MODIFIED/REMOVED delta format works like git diff for requirements. R_eff = min(scores) with Evidence Decay: why an average misleads and the minimum does not.',
    desc_ru: 'Spec описывает проверяемое поведение, PRD описывает намерение. Delta-формат ADDED/MODIFIED/REMOVED работает как git diff для требований. R_eff = min(scores) с Evidence Decay: почему среднее обманывает, а минимум нет.',
  },
  {
    slug: 'lifecycle-cycle',
    order: 12,
    category: 'lifecycle',
    pill: 'theory',
    title_en: 'Artifact lifecycle: from draft to a terminal state',
    title_ru: 'Жизненный цикл артефакта: от draft до конечного состояния',
    tag_en: 'after activate',
    tag_ru: 'после activate',
    desc_en: 'A state machine with 5 states and 6 transitions: draft → active → {superseded | deprecated | stale} → renew/reopen. Explained through a passport-and-visa analogy. Why stale is a forgotten state that requires an explicit decision.',
    desc_ru: 'Автомат с 5 состояниями и 6 переходами: draft → active → {superseded | deprecated | stale} → renew/reopen. Аналогия с паспортом и визой. Почему stale становится забытым состоянием и требует явного решения.',
  },
  {
    slug: 'fpf-cycle',
    order: 13,
    category: 'foundation',
    pill: 'theory',
    title_en: 'First Principles Framework (FPF): a shared foundation',
    title_ru: 'First Principles Framework (FPF): общая основа',
    tag_en: 'meta-framework',
    tag_ru: 'метафреймворк',
    desc_en: 'Six basic concepts (Bounded Context, Trust Calculus, F-G-R, ADI Cycle, Category Error, Gamma Algebra) and three usage modes (decompose / evaluate / reason). Trust Calculus and ADI are built-in ways of working within the same framework, rather than separate ideas.',
    desc_ru: 'Шесть базовых понятий (Bounded Context, Trust Calculus, F-G-R, ADI Cycle, Category Error, Gamma Algebra) и три режима работы (decompose / evaluate / reason). Trust Calculus и ADI входят в общую методологию как рабочие приёмы, а не существуют отдельно.',
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
    desc_en: 'A Plotly 3D scatter plot with artifact nodes and connecting edges. The real Knowledge Vault MVP example has 22 artifacts, 30 connections, and type filters. See healthy chains, blindspots, orphans, branching, and traceability in one view.',
    desc_ru: 'Трёхмерная диаграмма рассеяния Plotly с узлами-артефактами и рёбрами-связями. Реальный пример Knowledge Vault MVP: 22 артефакта, 30 связей и фильтры по типам. На одной схеме видны здоровые цепочки, слепые зоны, артефакты без связей, разветвления и прослеживаемость.',
  },
  {
    slug: 'depth-calibrator',
    order: 15,
    category: 'tool',
    pill: 'interactive',
    title_en: 'Depth Calibrator: choosing task depth in real time',
    title_ru: 'Depth Calibrator: выбор глубины задачи в реальном времени',
    tag_en: 'companion · interactive',
    tag_ru: 'приложение · интерактив',
    desc_en: 'Three sliders (complexity / blast radius / reversibility) and three hard gates (public API / security / cross-team) determine the depth tier and pipeline in real time. Presets cover cosmetic changes, a team feature, a public API, and an auth rewrite. Move the sliders to see the marker and artifact list change.',
    desc_ru: 'Три ползунка (сложность / масштаб последствий / обратимость) и три обязательных условия (публичный API / безопасность / работа нескольких команд) определяют глубину проработки и последовательность шагов в реальном времени. Готовые настройки для косметических изменений, функции для команды, публичного API и переписывания авторизации.',
  },
  {
    slug: 'agent-protocol-cycle',
    order: 16,
    category: 'agent',
    pill: 'theory',
    title_en: 'Hint Contract: 5 markers',
    title_ru: 'Hint Contract: 5 маркеров',
    tag_en: 'agent ↔ tool language',
    tag_ru: 'язык общения агента и инструмента',
    desc_en: 'How the agent stopped inventing next steps. Five markers (Next: / Or: / Wait: / Done. / Fix:) and five contract rules (Presence / Actionability / Determinism / Conditionality / Consistency), with a reference card.',
    desc_ru: 'Как агент перестал выдумывать следующие шаги. Пять маркеров (Next: / Or: / Wait: / Done. / Fix:) и пять правил контракта (Presence / Actionability / Determinism / Conditionality / Consistency). Карточка-памятка.',
  },
  {
    slug: 'multi-agent-cycle',
    order: 17,
    category: 'multi',
    pill: 'theory',
    title_en: 'Multi-agent work: dispatch, claim, release',
    title_ru: 'Работа нескольких агентов: dispatch, claim, release',
    tag_en: 'parallel work',
    tag_ru: 'параллельная работа',
    desc_en: 'Coordinating 2-5 agents in one repo without conflicts. The planner divides work into conflict-free groups, a worker claims one with a TTL, and release returns it to the pool. A strict WIP=1 limit. Adversarial review always runs in a separate agent.',
    desc_ru: 'Как координировать 2–5 агентов в одном репозитории без конфликтов. Планировщик делит работу на группы без конфликтов, исполнитель берёт одну с TTL, а release возвращает её в общий пул. Строгое ограничение WIP=1.',
  },
  {
    slug: 'forgeplan-cycle',
    order: 18,
    category: 'tool',
    pill: 'tool',
    title_en: 'Forgeplan as the operational layer',
    title_ru: 'Forgeplan для практической работы с методологиями',
    tag_en: 'the tool itself',
    tag_ru: 'сам инструмент',
    desc_en: 'Forgeplan as a POS for methodologies: CLI, MCP, and .forgeplan/ bring together FPF, BMAD, OpenSpec, and QuintCode. 10 artifact types (6 actively used) form a dependency DAG. Four depth levels and three quality gates in one validate.',
    desc_ru: 'Forgeplan как POS для методологий: CLI, MCP и .forgeplan/ объединяют FPF, BMAD, OpenSpec и QuintCode. 10 типов артефактов (6 активно используются) образуют DAG зависимостей. Четыре уровня глубины и три проверки качества в одном validate.',
  },
  {
    slug: 'first-artifact',
    order: 19,
    category: 'tool',
    pill: 'interactive',
    title_en: 'Your first artifact: a 20-minute walkthrough',
    title_ru: 'Первый артефакт: 20 минут практики',
    tag_en: 'tutorial',
    tag_ru: 'практическое руководство',
    desc_en: 'A simulated terminal takes you through init → health → route → new prd → validate → reason (ADI) → build → new evidence → link + score → activate. A progress bar with clickable points, a live workspace side panel, and common errors at each step.',
    desc_ru: 'Учебный терминал с симуляцией команд проводит по шагам: init → health → route → new prd → validate → reason (ADI) → build → new evidence → link + score → activate. Индикатор прогресса с кликабельными точками, боковая панель состояния рабочего пространства и типичные ошибки на каждом шаге.',
  },
];

export function getGuide(slug: string): Guide | undefined {
  return guides.find(g => g.slug === slug);
}
