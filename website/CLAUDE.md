# CLAUDE.md — website/

Локальные правила для работы с Astro-сайтом forgeplan.dev. Применяются
дополнительно к корневому CLAUDE.md проекта.

---

## Тон контента (гайды + блог)

Никакой мета-нарратив о устройстве курса в теле контента. Полное
правило с примерами и обоснованием — в hindsight memory
`feedback_no_meta_lesson_framing.md`.

Кратко — не использовать:

- ❌ «Это нулевой урок»
- ❌ «Без отсылок к инструментам, без брендов»
- ❌ «В предыдущем уроке мы разобрали»
- ❌ «В следующих уроках мы»
- ❌ «Сравнение требует контекста, который появится после первых 5–6 уроков»
- ❌ «Если вы это уже знаете — пропускайте»
- ❌ «Двенадцать уроков общим временем чтения два часа»

Лид сразу с содержательного хука (история, цифра, парадокс). Финал —
конкретная навигация (карточки, ссылки) без фразы «дальше будет…».

## Позиционирование Forgeplan в материалах

Серия гайдов — это **обучение методике**, не **продвижение продукта**.

Порядок:

1. Сначала концепты (журнал решений, ADR/DDR форматы, R_eff, evidence decay, lifecycle, FPF)
2. Потом практика (как ввести в команде, как настроить агентов)
3. **Только в самом конце** — Forgeplan как одна из реализаций этих принципов

Forgeplan не упоминается в первых 12 уроках кроме коротких атрибуций.
В первых уроках вместо `forgeplan_get` пишем «открой каталог решений»,
вместо `forgeplan validate` — «проверь по шаблону». Конкретные
CLI/MCP команды — только в финальных двух уроках (forgeplan-cycle,
first-artifact).

То же для блог-постов — методология идёт в основу, Forgeplan
вспоминается только если пост явно про инструмент.

## Типографика для гайдов (public/guides-raw/*.html)

Современные стандарты читальных сайтов (Medium, Substack, NYT) — 18-21px
body. Наш baseline — компромисс:

- `p` — **17px**, line-height **1.65**
- `p.lead` — **19.5px**, line-height **1.6**
- `h1` — `clamp(40px, 5.4vw, 60px)`, font-weight 500
- `h2` — **28px**, margin-top 64px
- `h3` — **18-19px**, font-weight 500
- `code` — **14-14.5px**
- `pre` — **14px**, padding 16px 18px
- `ul/ol` — **17px**, line-height 1.65

Шрифты `Inter` (sans) + `JetBrains Mono` (для кода и mono-уровней) — не
менять без необходимости. Единство по всей серии — важнее.

## Буллеты — обязательно явные с символами и отступом

Дефолтные браузерные `<ul>` с серыми точками — **не использовать**.
Везде применять кастомные с brand-цветом:

```css
ul, ol {
  list-style: none;
  padding-left: 0;
  margin: 16px 0;
  font-size: 17px;
  line-height: 1.65;
}
ul li, ol li {
  margin-bottom: 12px;
  padding-left: 28px;
  position: relative;
}
ul li::before {
  content: "▸";
  position: absolute;
  left: 6px;
  top: 0;
  color: var(--accent); /* #ff5a1f orange */
  font-family: var(--font-mono);
  font-size: 17px;
  line-height: 1.65;
}
ol {
  counter-reset: list-counter;
}
ol li {
  padding-left: 36px;
}
ol li::before {
  content: counter(list-counter, decimal-leading-zero);
  counter-increment: list-counter;
  position: absolute;
  left: 0;
  top: 1px;
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 12px;
  letter-spacing: 0.1em;
  font-weight: 500;
}
```

`ul` → треугольник `▸` brand orange.
`ol` → нумерация в формате `01`, `02`, `03` (mono uppercase brand orange).

## Inline-перечисления — переписывать буллетами

Не писать «У практики есть форматы: ADR — короткая запись; DDR —
расширенная; Journal Entry — простая.» одной строкой через `;`.

Любое перечисление из **3+ пунктов** выносим в `<ul>` с буллетами —
проще читается, видно структуру.

## Square corners — обязательно

Главная страница forgeplan.dev (`pages/index.astro` + React-секции с Tailwind)
использует **square corners везде** — `border-radius: 0` по дефолту, никаких
`rounded-*` утилит в landing-компонентах. Это часть бренд-эстетики
«инженерная точность, без декораций».

В гайдах и блоге **не использовать** `border-radius: 3px / 4px / 6px` —
это «нежно-карточный» стиль, противоречит landing. Используем:

```css
.card, .table, .pre, .aside, .blockquote {
  border-radius: 0;  /* всегда square */
}

/* Единственное исключение — круглые маркеры */
.timeline li::before,
.dot-marker {
  border-radius: 50%;
}
```

Для inline `<code>` пилюль — тоже square (`border-radius: 0`). Для
изображений `<img>` — square (никаких округлений углов).

При создании новых HTML-уроков или компонентов: проверь grep `border-radius`
в файле перед коммитом, всё что не `50%` → `0`.

## Дизайн-система — единые CSS-токены

Все гайды используют один набор `:root` переменных (см. начало любого
файла `public/guides-raw/*.html`):

- `--bg #050505`, dot-grid 24px на body
- `--accent #ff5a1f` (warm orange) — используем скупо, на акцентных элементах
- `--text #f5f5f5`, `--text-2 #a3a3a3` (вторичный), `--text-3 #737373` (мета)
- `--font-sans` Inter, `--font-mono` JetBrains Mono
- Карточки на `--bg-1 #0b0b0b` с border `--line-2`
- Eyebrow всегда mono uppercase orange, letter-spacing 0.18em

Не вводить новые цвета без обсуждения. Если нужен новый акцент —
выбираем из существующих токенов: `--ok #22c55e`, `--err #ef4444`,
`--warn #f59e0b`.

## Cover-обложки для блог-постов

- PNG → WebP конвертация обязательна перед подключением:
  `cwebp -q 82 -m 6 <file>.png -o <file>.webp`
- Типичное сжатие 94-97% (от ~1MB до ~30-60K)
- Schema (см. `src/content.config.ts`) разрешает только локальные пути:
  `/blog-covers/<slug>.{webp|png|jpg|jpeg|svg}`
- Защита от `javascript:`, `data:`, `//evil.tld` — F-3 security guard

EN-посты и RU-посты используют **разные** обложки если на картинке
есть кириллица или латиница. Один файл на оба языка — только когда
визуал language-agnostic (числа, символы, схемы без подписей).

Конвенция именования:
- `<slug>.png` — основная EN-версия
- `<slug>-ru.png` — RU-версия (если отличается)
- `<slug>-2.png` — альтернативный вариант (для сравнения)
- `<slug>-en.png` — EN-версия (если основная сначала была RU)

## Dev server restart обязателен

При изменении `src/data/guides.ts` (orders, new entries, slug changes) —
restart dev сервера **обязателен**. `getStaticPaths` подгружается один
раз на старте, HMR его не перезагружает.

Symptom: новые routes возвращают 404 в браузере, хотя build PASS.

Fix: `kill <PID>` → `npm run dev`.

Изменения в HTML/CSS/MDX content — HMR подхватит автоматически без
restart.

## Build verify после каждого изменения

`npm run build` в `website/` должен закрываться без warning'ов перед
коммитом. Текущий baseline — 398 страниц, ~11-12 секунд.

При проблемах в production но не в dev — проверить:
- Astro.site fallback (set in `astro.config.mjs`)
- Cover paths case-sensitivity (macOS vs Linux)
- WebP браузерная поддержка (Safari < 14 не поддерживает)

## Layout — единая ширина 1280px

Все edge'ы контента (sidebar, main, h1, header) совпадают с
footer-inner: `max-width: 1280px` + `padding: 0 32px`. Если меняешь
ширину в одном месте — проверь все остальные. Пользователь несколько
раз ловил рассогласования.

### Embed-overrides от blog-theme.css — что важно знать автору

При вставке HTML-урока в `/guides/<slug>` через `extractEmbed` его inline
CSS попадает в страницу ПОСЛЕ `blog-theme.css`. Но `blog-theme.css`
имеет несколько `!important` правил, которые перебивают авторский
inline CSS внутри `.guide-embedded`:

- `.guide-embedded main` — принудительно `max-width: 1280px`,
  `padding: 32px 32px 64px`. Авторский `max-width: 1080px` не сработает.
- `.guide-embedded p`, `.guide-embedded p.lead` — `max-width: none`.
  Параграфы растягиваются на всю ширину контейнера (1216px = 1280 − 64).

`ul/ol` НЕ имеют site-wide override. Если автор задал в lesson
`ul, ol { max-width: 760px }` для plain текстовых списков — это
правило применится и к grid-спискам тоже (`.deflist`, `.timeline`,
`.usecase-grid`). Получится: текст широкий (1216), сетки узкие (760
со слипшимися плитками). Пользователь это называет «лютиком».

**Решение для авторов:**

```css
/* Plain inline lists — узкие для читаемости, через direct child of main */
main > ul, main > ol { max-width: 820px; }

/* Каждый grid-список (ol/ul с классом) — явный max-width: none */
.deflist  { display: grid; grid-template-columns: repeat(3, 1fr); max-width: none; }
.timeline { padding-left: 22px; max-width: none; }
.your-grid-list { ...; max-width: none; }
```

Грид-списки из 6 элементов лучше делать в 3 колонки на десктопе
(2 ряда × 3 колонки) — как `.audience-grid` и `.usecase-grid`.
Не в 2 колонки (3 ряда) — текст в плитках сжимается, последняя плитка
может выглядеть одиноко при rounding.

### Verify через Playwright перед коммитом

Сложные layout-изменения проверять через MCP Playwright:

1. `mcp__plugin_playwright_playwright__browser_resize` width=1440 height=900
2. `mcp__plugin_playwright_playwright__browser_navigate` на нужную страницу
3. `mcp__plugin_playwright_playwright__browser_evaluate` — вычислить ширины
   ключевых блоков (`.deflist`, `.audience-grid`, `main`, `p`)
4. Скриншот для визуальной проверки

Это надёжнее чем «build PASS значит ОК» — build не валидирует layout.

## Embed HTML lessons — minimal wrapper

При импорте интерактивных HTML lessons в Astro:

- НЕ использовать iframe (отвергнуто пользователем дважды)
- Извлекать body + style + script, обернуть в `.guide-embedded` класс
- Удалять `:root`, `<html>`, `<body>` теги из CSS
- Префиксовать остальные правила wrapper-классом
- Алиасить токены сайта (`--bg → var(--forge-bg)`)
- Inline-скрипты оборачивать в IIFE
- Wrapper минимальный: Header + breadcrumb + content + prev/next + Footer.
  НЕ дублировать h1/summary над lesson — у lesson своя шапка.
