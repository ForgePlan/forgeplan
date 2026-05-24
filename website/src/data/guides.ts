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
    slug: 'decision-cycle',
    order: 1,
    category: 'foundation',
    pill: 'theory',
    title_en: 'How decisions are made and evidence weighted',
    title_ru: 'Цикл одного решения · ADI · DDR · Trust Calculus',
    tag_en: 'foundation',
    tag_ru: 'базовый цикл',
    desc_en: 'ADI reasoning (Abduction → Deduction → Induction), DDR field on every ADR, and the Trust Calculus framework. Why averaging trust is wrong and the weakest-link principle.',
    desc_ru: 'ADI цикл (Abduction → Deduction → Induction), DDR-поле на каждом ADR и Trust Calculus как способ численно оценивать доверие к источнику. Почему среднее обманывает.',
  },
  {
    slug: 'trust-calculus',
    order: 2,
    category: 'viz',
    pill: 'interactive',
    title_en: 'Trust Calculus in 3D F/G/R',
    title_ru: 'Trust Calculus в 3D F/G/R',
    tag_en: 'companion to 01 · interactive',
    tag_ru: 'приложение к 01 · интерактив',
    desc_en: '3D scene with seven evidence on a /search latency scenario. Plane of threshold F+G+R=12, hover cards, hypothesis selection by WLNK. Rotate the scene with mouse — projector-ready teaching artifact.',
    desc_ru: '3D-сцена с семью evidence на сценарии /search latency. Плоскость порога F+G+R=12, hover-карточки, выбор гипотезы по WLNK. Сцену можно крутить мышью — учебный артефакт для проектора.',
  },
  {
    slug: 'bmad-cycle',
    order: 3,
    category: 'feature',
    pill: 'theory',
    title_en: 'BMAD · 13-step PRD · Adversarial Review',
    title_ru: 'BMAD · 13-step PRD · Adversarial Review',
    tag_en: 'one feature',
    tag_ru: 'одна фича',
    desc_en: 'Four phases of the funnel (Brainstorming → Modeling → Architecting → Delivery), 13 PRD sections of which 3 are critical (Non-Goals, Risks, Open Questions), and adversarial review through fresh eyes as the third quality gate.',
    desc_ru: 'Четыре фазы воронки (Brainstorming → Modeling → Architecting → Delivery), 13 секций PRD из которых 3 критичные (Non-Goals, Risks, Open Questions), и adversarial review чужими глазами как third quality gate.',
  },
  {
    slug: 'spec-cycle',
    order: 4,
    category: 'spec',
    pill: 'theory',
    title_en: 'Spec · Delta · R_eff',
    title_ru: 'Spec · Delta · R_eff',
    tag_en: 'one specification',
    tag_ru: 'одна спецификация',
    desc_en: 'Spec vs PRD: verifiable behavior, not intent. Delta format ADDED/MODIFIED/REMOVED as git diff for requirements. R_eff = min(scores) with Evidence Decay — why average deceives, minimum does not.',
    desc_ru: 'Spec против PRD: проверяемое поведение, а не намерение. Delta-формат ADDED/MODIFIED/REMOVED как git diff для требований. R_eff = min(scores) с Evidence Decay — почему среднее обманывает, а минимум — нет.',
  },
  {
    slug: 'forgeplan-cycle',
    order: 5,
    category: 'tool',
    pill: 'tool',
    title_en: 'Forgeplan as the operational layer',
    title_ru: 'Forgeplan как operational layer',
    tag_en: 'the tool itself',
    tag_ru: 'сам инструмент',
    desc_en: 'POS for methodologies: CLI + MCP + .forgeplan/ absorbs FPF, BMAD, OpenSpec, QuintCode. 10 artifact types (6 actively used) with dependency DAG. Four depth levels and three quality gates in one validate.',
    desc_ru: 'POS для методологий: CLI + MCP + .forgeplan/ забирают на себя FPF, BMAD, OpenSpec, QuintCode. 10 типов артефактов (6 в активном ходу) с DAG зависимостей. Четыре depth-уровня и три quality gates в одном validate.',
  },
  {
    slug: 'dag-explorer',
    order: 6,
    category: 'viz',
    pill: 'interactive',
    title_en: 'DAG Explorer · artifacts in 3D',
    title_ru: 'DAG Explorer · артефакты в 3D',
    tag_en: 'companion to 05 · interactive',
    tag_ru: 'приложение к 05 · интерактив',
    desc_en: 'Plotly 3D scatter with artifact nodes and edge lines. Knowledge Vault MVP as a real example: 22 artifacts, 30 connections, type filters. Shows patterns (healthy chain / blindspot / orphan / branching) and traceability in one picture.',
    desc_ru: 'Plotly 3D scatter с узлами-артефактами и линиями-рёбрами. Knowledge Vault MVP как реальный пример: 22 артефакта, 30 связей, фильтры по типам. Покажет паттерны (здоровая цепочка / blindspot / orphan / разветвление) и трейсируемость одной картинкой.',
  },
  {
    slug: 'depth-calibrator',
    order: 7,
    category: 'tool',
    pill: 'interactive',
    title_en: 'Depth Calibrator · routing in real-time',
    title_ru: 'Depth Calibrator · routing в реальном времени',
    tag_en: 'companion to 05 · interactive',
    tag_ru: 'приложение к 05 · интерактив',
    desc_en: 'Three sliders (complexity / blast radius / reversibility) + three hard gates (public API / security / cross-team) → real-time depth tier + pipeline. Presets for typical scenarios (cosmetic / team feature / public API / auth rewrite). Move sliders to see the marker shift and artifact list change.',
    desc_ru: 'Три слайдера (complexity / blast radius / reversibility) + три хард-гейта (public API / security / cross-team) → реал-тайм depth tier + pipeline. Пресеты для типовых сценариев (cosmetic / team feature / public API / auth rewrite).',
  },
  {
    slug: 'lifecycle-cycle',
    order: 8,
    category: 'lifecycle',
    pill: 'theory',
    title_en: 'Lifecycle · from draft to terminal',
    title_ru: 'Lifecycle · от draft до terminal',
    tag_en: 'after activate',
    tag_ru: 'что после activate',
    desc_en: 'State machine with 5 states and 6 transitions: draft → active → {superseded | deprecated | stale} → renew/reopen. Passport-with-visa analogy. Stale as "forgotten state" requiring explicit decision.',
    desc_ru: 'State machine с 5 состояниями и 6 переходами: draft → active → {superseded | deprecated | stale} → renew/reopen. Аналогия паспорта с визой. Stale как «забытое состояние», требующее явного решения.',
  },
  {
    slug: 'agent-protocol-cycle',
    order: 9,
    category: 'agent',
    pill: 'theory',
    title_en: 'Hint Contract · 5 markers',
    title_ru: 'Hint Contract · 5 маркеров',
    tag_en: 'agent ↔ tool language',
    tag_ru: 'язык агента ↔ инструмента',
    desc_en: 'How the agent stopped hallucinating next steps. Five markers (Next: / Or: / Wait: / Done. / Fix:) + five contract rules (Presence / Actionability / Determinism / Conditionality / Consistency). Reference card.',
    desc_ru: 'Как агент перестал галлюцинировать next steps. Пять маркеров (Next: / Or: / Wait: / Done. / Fix:) + пять правил контракта (Presence / Actionability / Determinism / Conditionality / Consistency). Reference card.',
  },
  {
    slug: 'multi-agent-cycle',
    order: 10,
    category: 'multi',
    pill: 'theory',
    title_en: 'Multi-agent · dispatch · claim · release',
    title_ru: 'Multi-agent · dispatch · claim · release',
    tag_en: 'parallel work',
    tag_ru: 'параллельная работа',
    desc_en: 'Coordinating 2-5 agents in one repo without conflicts. Planner cuts work into conflict-free buckets, worker takes one with TTL, release returns to pool. WIP=1 hard limit. Adversarial review always in a separate agent.',
    desc_ru: 'Координация 2–5 agent\'ов в одном репо без конфликтов. Planner режет работу на конфликт-free buckets, worker берёт под себя с TTL, release возвращает в pool. WIP=1 hard limit.',
  },
  {
    slug: 'fpf-cycle',
    order: 11,
    category: 'foundation',
    pill: 'theory',
    title_en: 'FPF · First Principles Framework as the parent rail',
    title_ru: 'FPF · First Principles Framework как родительская рамка',
    tag_en: 'meta framework',
    tag_ru: 'meta-фреймворк',
    desc_en: 'Six base concepts (Bounded Context, Trust Calculus, F-G-R, ADI Cycle, Category Error, Gamma Algebra) + three usage modes (decompose / evaluate / reason). Shows how Trust Calculus and ADI are not separate ideas but built-in moves inside the same parent framework.',
    desc_ru: 'Шесть базовых понятий (Bounded Context, Trust Calculus, F-G-R, ADI Cycle, Category Error, Gamma Algebra) + три режима использования (decompose / evaluate / reason). Показывает, что Trust Calculus и ADI — не отдельные идеи, а встроенные приёмы внутри одной родительской рамки.',
  },
  {
    slug: 'first-artifact',
    order: 12,
    category: 'tool',
    pill: 'interactive',
    title_en: 'First Artifact · 20-minute hands-on walkthrough',
    title_ru: 'Первый артефакт · 20-минутный hands-on walkthrough',
    tag_en: 'tutorial',
    tag_ru: 'туториал',
    desc_en: 'Step-by-step simulated terminal: init → health → route → new prd → validate → reason (ADI) → build → new evidence → link + score → activate. Progress bar with clickable points, live workspace side-panel, common-error blocks at each step.',
    desc_ru: 'Пошаговый симулированный терминал: init → health → route → new prd → validate → reason (ADI) → build → new evidence → link + score → activate. Прогресс-бар с кликабельными точками, side-panel состояния workspace, типичные ошибки на каждом шаге.',
  },
];

export function getGuide(slug: string): Guide | undefined {
  return guides.find(g => g.slug === slug);
}
