---
depth: standard
id: PROB-088
kind: problem
last_modified_at: 2026-09-02T14:42:16.076193+00:00
last_modified_by: claude-code/2.1.220
status: draft
title: 'Release binaries ship without semantic-search: cargo-dist builds default features only'
---

---
assigned_number: 88
predicted_number: 88
slug: prob-release-binaries-ship-without-semantic-search-cargo-dist-builds-default
---

> GitHub: [#451](https://github.com/ForgePlan/forgeplan/issues/451) (bug) — fix tracked in
> [#452](https://github.com/ForgePlan/forgeplan/issues/452) / PRD-083.

## Problem

Ни один опубликованный бинарь ForgePlan не содержит векторного поиска. Это касается всех
каналов дистрибуции — Homebrew tap, `install.sh`, GitHub Releases, `cargo install forgeplan`
с crates.io — и всех пяти таргетов. Пользователь, поставивший `forgeplan` через brew,
получает `forgeplan embed` с отказом и `fpf search --semantic`, тихо деградирующий в
keyword-поиск.

Это **не** регрессия v0.34.0. Дефект существует с момента введения cargo-dist (Sprint 10,
PR #97) и до сих пор не был замечен, потому что fallback работает корректно и не шумит.

## Root cause

`dist-workspace.toml:5-19` — конфиг cargo-dist. В нём заданы `targets`, `installers`, `tap`,
`bin-aliases`, но **отсутствовали** ключи `features` и `all-features`. Без явного указания
cargo-dist собирает с default-features.

Фича при этом не default:

- `crates/forgeplan-core/Cargo.toml:53-54` — `default = []`, `semantic-search = ["fastembed"]`
- `crates/forgeplan-cli/Cargo.toml:43-44` — `semantic-search = ["forgeplan-core/semantic-search"]`
- `crates/forgeplan-mcp/Cargo.toml:33` — то же самое

Итог: `fastembed` не компилируется → `EmbeddingDriver` отсутствует → `forgeplan embed` падает,
`fpf search --semantic` уходит в fallback.

`install.sh` скачивает те же prebuilt-артефакты cargo-dist, поэтому альтернативного канала
с фичей не существует.

## Reproducer

```bash
brew install ForgePlan/tap/forgeplan     # или install.sh, или скачать релизный бинарь
forgeplan --version                       # forgeplan 0.34.0
forgeplan embed
# Error: Embedding not available. Rebuild with: cargo build --features semantic-search
# Fix: cargo build --features semantic-search
```

Подтверждение на brew-артефакте (measurement, 2026-08-29):

```
$ ls -la /opt/homebrew/Cellar/forgeplan/0.34.0/bin/forgeplan
-r-xr-xr-x  47378368 bytes

$ otool -L /opt/homebrew/Cellar/forgeplan/0.34.0/bin/forgeplan
  /usr/lib/libSystem.B.dylib
  /usr/lib/libobjc.A.dylib
  /System/Library/Frameworks/Foundation.framework/...
  /System/Library/Frameworks/CoreFoundation.framework/...
  /System/Library/Frameworks/CoreServices.framework/...
  /System/Library/Frameworks/Security.framework/...
  /usr/lib/libiconv.2.dylib
```

Ни одной ссылки на ONNX Runtime — фича действительно не вкомпилирована.

## Measurement addendum — локальная сборка с фичей уже существует (2026-08-29)

При проверке выяснилось, что бинарь, реально стоящий в `PATH` на машине репортёра, — **не**
brew-овский:

| Артефакт | Путь | Размер | Дата | semantic-search |
|---|---|---|---|---|
| brew Cellar | `/opt/homebrew/Cellar/forgeplan/0.34.0/bin/forgeplan` | 47 378 368 B | Aug 17 | нет |
| активный в PATH | `~/.local/bin/forgeplan` | 67 187 648 B | Aug 29 16:18 | **да** |
| осиротевший | `/opt/homebrew/bin/forgeplan.new` | 44 979 488 B | Apr 18 | нет |

`brew list --versions forgeplan` показывает установленный 0.34.0, но symlink
`/opt/homebrew/bin/forgeplan` **отсутствует** — остался только `forgeplan.new` от 18 апреля.
Brew-линк снят или сломан, и `which forgeplan` резолвится в локальную сборку.

Локальная сборка отрабатывает `embed` полноценно:

```
  Loading embedding model...
Embedding 396 artifact(s) (title + body, chunk_size=2000)...
```

**Дифференциальный маркер в линковке.** Локальный бинарь линкует две библиотеки, которых нет
у brew-овского:

```
/usr/lib/libc++.1.dylib                                    ← отсутствует в brew-бинаре
/System/Library/Frameworks/SystemConfiguration.framework   ← отсутствует в brew-бинаре
```

`libc++` — C++-рантайм, который тянет статически влинкованный ONNX Runtime. Наличие или
отсутствие `libc++` в выводе `otool -L` — надёжная однокомандная проверка того, несёт ли
конкретный бинарь ForgePlan фичу `semantic-search`. Годится как дешёвый smoke-check
релизного артефакта.

**Значение для PRD-083 FR-004:** дельта размера от фичи на `aarch64-apple-darwin` — примерно
**+19.8 MB (+41.8 %)**, 47.4 MB → 67.2 MB. Это первое приближение, а не контрольный замер:
профили сборки двух бинарей не сверялись. Измерение покрывает **один** таргет из пяти; про
`x86_64-pc-windows-msvc` и `aarch64-unknown-linux-gnu` оно не говорит ничего.

## Три проявления (одна причина, три места правки)

**M1 — релизная сборка без фичи** (корень). `dist-workspace.toml` не передавал `features`.

**M2 — хинт вводит в заблуждение.** `crates/forgeplan-cli/src/commands/embed.rs:83` эмитил
`Fix: cargo build --features semantic-search`. Для человека, поставившего бинарь через brew,
эта команда бесполезна: у него нет чекаута репозитория. Нарушение hint-контракта PRD-071 —
`Fix:` обязан быть исполнимым as-is.

**M3 — документация утверждала обратное.** Изначально это проявление было записано как
«контракт нигде не объявлен». При реализации выяснилось, что дело хуже: документация
**активно дезинформировала**.

`website/src/content/docs/docs/getting-started/configuration.md:141` (и её RU-зеркало)
дословно говорила:

> Requires the `semantic-search` feature flag at build time
> **(included in official release binaries)**.

Это прямое, недвусмысленное утверждение, что фича есть в официальных релизных бинарях.
Пользователь, читавший документацию, имел все основания ждать работающего векторного поиска
и считать его отказ багом. Именно так дефект и был обнаружен.

Там же, в разделе troubleshooting (`configuration.md:507`), совет по диагностике тоже не
работал: «check `forgeplan --version`» — но `--version` печатает только номер версии и о
составе фич не сообщает ничего.

## Почему фича изначально не default — основания есть, ADR нет

Из `Cargo.lock`:

- `fastembed 5.17.3` → `ort 2.0.0-rc.12` → `ort-sys 2.0.0-rc.12`
- зависимости `ort-sys`: `ureq`, `lzma-rust2`, `hmac-sha256` — профиль `download-binaries`,
  то есть ONNX Runtime **скачивается во время сборки**. Релизная сборка начинает требовать
  сети и наличия prebuilt-артефакта под каждый из пяти таргетов, включая
  `x86_64-pc-windows-msvc` и `aarch64-unknown-linux-gnu`.
- `ort` — release candidate в цепочке продакшн-бинаря
- модель BGE-M3 — 2.1 GB при первом запуске (измерено; см. PROB-089 D3)
- бинарь уже 47 MB при `opt-level = "z"`; с фичей — ~67 MB

Причины разумные, но **решение нигде не записано**. Прогреплены `.forgeplan/adrs/`, `docs/`,
`CHANGELOG.md` — ADR на «почему semantic-search не входит в дистрибуцию» не существует.
Поэтому вопрос воспроизводится заново при каждом столкновении с ним.

## Blast radius

- Каждый пользователь бинарной установки — а это основной путь (`README` Install → brew).
- Риск при исправлении реален: cargo-dist роняет **весь** workflow, если валится хоть один
  таргет. v0.32.0 уже падал именно на `x86_64-pc-windows-msvc` (`CHANGELOG.md:102`), и вместе
  с Windows тогда не опубликовались macOS и Linux.

## Related

- PRD-083 — задача на решение, GitHub #452
- PROB-089 — дефекты кэша модели, вскрытые при реализации; становятся массовыми ровно тогда,
  когда этот дефект будет закрыт
- PRD-071 — hint-контракт, который нарушает M2
- `docs/operations/RELEASE-PROTOCOL` — место, где контракт дистрибуции обязан быть зафиксирован


