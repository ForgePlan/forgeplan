# Image prompts — Пост 1 «Кладбище решений»

Целевые ассеты для блога: `/blog/decision-graveyard` (EN) + `/ru/blog/decision-graveyard` (RU).
Бренд-обёртка из `ForgePlanMarketing/STYLE-GUIDE.md` §2 и §3.1.

---

## Asset 1 · Hero cover (обложка статьи)

**Назначение**: `<Hero>` блока в верху статьи. Frontmatter: `cover: '/blog-covers/decision-graveyard.webp'`.
**Аспект**: 16:9 (1600×900 для хайреза, 1280×720 для inline).
**Шаблон**: E (атмосферное фото) из STYLE-GUIDE §3.3.
**Сцена**: тёмная переговорка ночью, whiteboard со зачёркнутыми вариантами «JWT? / sessions? / OAuth?», стикеры с вопросами, пустые кофейные кружки. Атмосфера «двухчасовой спор, который никто не помнит».

### EN prompt (для GPT-Image / DALL-E 3 — рекомендую)

```
Photographic editorial cover image. A dimly-lit corporate meeting room at night,
seen from a slightly elevated angle. In the center: a whiteboard, partially
filled with hand-written engineering options — "JWT?", "sessions?", "OAuth?",
"refresh tokens?" — most of them crossed out with thin red marker, only one
("JWT") circled. Several yellow and pink sticky notes on the board with
fragmented words: "Friday demo", "decide later", "ask Petr". On the table
in front of the whiteboard: two closed laptops, three empty coffee mugs,
a half-empty water glass, a single open notebook with a felt-tip pen on it.

Background palette: deep near-black #050505 walls and floor. The only
illumination is the cool monitor glow from a closed-laptop's standby LED
and a single warm desk lamp on the right edge of the frame. Brand
orange #FF5A1F appears ONLY as the one circled "JWT" word on the whiteboard
and the rim of the desk lamp shade.

No people in frame. No faces. No phones. Empty chairs. The whiteboard text
is in clean handwriting — readable, no typos. The mood is "two-hour argument
that no one remembers — yesterday's whiteboard surviving overnight."

Texture: subtle dot grid pattern visible in dark walls (10% opacity white dots,
24px spacing) — barely perceptible engineering aesthetic.

Strict no-nos: no neon glow, no cyberpunk lighting, no RGB keyboards, no
holograms, no robot figures, no AI hands, no glittering text effects, no
illuminated CPUs. Clean photographic late-night corporate mood — like a
Reuters photo, not a stock image.

Style references: late-night-engineer aesthetic, somber but not gloomy,
shot on full-frame mirrorless with 35mm prime lens, ambient ISO 1600,
slight grain, no flash. Color grading: cool blues in shadows, warm tungsten
highlights from the single lamp.

Aspect ratio: 16:9 (1600×900 px). Composition: rule of thirds, whiteboard
occupying right two-thirds, leftmost third dark with hint of doorway.

Output: photorealistic, NOT illustrated, NOT 3D render, NO watermarks.
```

### RU prompt (для Yandex Shedevrum или ручной адаптации)

```
Фотографическая редакторская обложка. Тёмная корпоративная переговорка
ночью, вид со слегка приподнятого угла. В центре: офисная доска
наполовину заполнена от руки инженерными вариантами — "JWT?",
"sessions?", "OAuth?", "refresh tokens?" — большинство зачёркнуто
тонким красным маркером, только один ("JWT") обведён. На доске несколько
жёлтых и розовых стикеров с обрывочными фразами: "пятничное демо",
"решим потом", "спросить Петра". На столе перед доской: два закрытых
ноутбука, три пустые кофейные кружки, наполовину пустой стакан воды,
один открытый блокнот с фетровой ручкой на нём.

Фон: почти чёрные стены и пол #050505. Освещение только от прохладного
индикатора закрытого ноутбука и одной тёплой настольной лампы по правому
краю кадра. Брендовый оранжевый #FF5A1F появляется ТОЛЬКО на обведённом
слове "JWT" и в кайме абажура лампы.

В кадре нет людей. Нет лиц. Нет телефонов. Пустые стулья. Текст на доске
чётким почерком, читаемо, без опечаток. Настроение: "двухчасовой спор,
который никто не помнит — доска вчерашнего обсуждения, оставшаяся на ночь".

Текстура: едва различимая точечная сетка на тёмных стенах (10% непрозрачность
белых точек, шаг 24px) — почти неуловимая инженерная эстетика.

Строгие запреты: никакого неонового свечения, никакой киберпанк-подсветки,
никаких RGB-клавиатур, никаких голограмм, никаких роботов, никаких AI-рук,
никаких блестящих текстовых эффектов, никаких подсвеченных CPU. Чистая
фотографическая ночная корпоративная атмосфера — как кадр Reuters, не сток.

Стилевые референсы: эстетика поздневечернего инженера, мрачное, но не
безнадёжное; снято на полнокадровую беззеркальную камеру с фиксированным
35мм объективом, ISO 1600, лёгкое зерно, без вспышки. Цветовая коррекция:
прохладные синие тона в тенях, тёплые тёплоянтарные блики от одной лампы.

Соотношение сторон: 16:9 (1600×900 px). Композиция: правило третей,
доска занимает правые две трети, левая треть тёмная с намёком на дверной
проём.

Результат: фотореалистичный, не иллюстрация, не 3D-рендер, без водяных
знаков.
```

### Альтернативный prompt (если первый не зайдёт)

Если генератор споткнётся на читаемом тексте на whiteboard (типичная проблема DALL-E с буквами):

```
[все то же самое, но whiteboard содержит только символы и схему, без
конкретных слов]

Modified whiteboard: instead of readable text, show an abstract decision
tree — three branches stemming from a central node, each branch ending
in a different geometric symbol (triangle, square, hexagon). Most branches
crossed out with thin red marker; one — the path leading to the hexagon —
circled in brand orange #FF5A1F. The whiteboard is dense with arrows,
question marks, and small geometric notations — but no actual words.

[остальное без изменений]
```

Это работает потому что генератор не должен «писать текст» — только символы и геометрию.

---

## Asset 2 · In-article comparison (внутри статьи, опционально)

**Назначение**: вставить в секцию «Конкретный пример» или «Что должно быть в формате решения».
**Аспект**: 16:9 (1280×720).
**Шаблон**: D (split-screen) из STYLE-GUIDE §3.3.

```
Editorial split-screen comparison labeled "Было" on the left and "Стало"
on the right (Russian labels — keep them in Cyrillic), separated by a thin
vertical line in the middle.

Left side (Было): three scattered fragments stacked vertically with sloppy
spacing. (1) A truncated Slack thread screenshot — only 3 visible message
bubbles, the rest fades into a "see more" link. (2) A Notion-style page
with the title "Auth decision" in bold and an empty body below — just a
blinking cursor. (3) A Google Doc thumbnail labeled "meeting notes Apr 14"
with five bullets, all unreadable scribbles. All three documents on a
muted gray background, faded edges, slightly tilted, conveying chaos.

Right side (Стало): one clean markdown file titled "DDR-042 · JWT vs sessions"
visible in a code editor frame. Six section headers in JetBrains Mono font:
"Контекст", "Рассмотренные варианты", "Доказательства", "Принятое решение",
"Отвергнутые альтернативы", "Условия пересмотра". Each header has a small
preview of filled content underneath (2-3 lines). The file frame has a
crisp drop-shadow, sits squarely on a dark background, brand orange #FF5A1F
accents on the section markers and active line indicator.

Top of image: thin uppercase mono eyebrow text in orange "ФОРМАТ РЕШЕНИЯ"
with 0.18em letter-spacing.

Background: dark #050505 with the dot-grid pattern (10% opacity white dots,
24px spacing) visible.

Aspect: 16:9. Editorial poster style. No 3D, no shadows on the entire
composition, only on the code editor frame. No people, no hands.
```

---

## Где разместить готовые ассеты

После генерации:

1. Сохранить файлы:
   ```
   website/public/blog-covers/decision-graveyard.webp     # cover
   website/public/blog-covers/decision-graveyard-compare.webp  # in-article
   ```
2. Обновить frontmatter поста:
   ```yaml
   cover: '/blog-covers/decision-graveyard.webp'
   ```
3. Внутри MDX, в нужной секции:
   ```mdx
   <img src="/blog-covers/decision-graveyard-compare.webp"
        alt="Слева три разбросанных документа без структуры; справа один markdown DDR-042 с шестью заполненными секциями"
        width="1280" height="720" />
   ```

## Чек-лист перед публикацией картинки

Из STYLE-GUIDE §5:
- [ ] Соотношение 16:9 (1600×900 или 1280×720)
- [ ] Брендовый `#FF5A1F` использован скупо — только обведённое JWT + кайма лампы
- [ ] Точечная сетка видна на тёмном фоне
- [ ] Нет лиц, нет роботов, нет неона
- [ ] Текст на whiteboard читаемый без опечаток
- [ ] Mood — поздневечерний инженерный, не cyberpunk

---

**Версия документа**: v1, 2026-05-23.
**Источник брифа**: `ForgePlanMarketing/posts/blog-series-decision-cycle.md` §Пост 1.
