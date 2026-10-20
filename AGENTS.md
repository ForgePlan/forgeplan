# AGENTS.md

Instructions for AI coding agents (Claude Code, Aider, Cursor, Continue, etc.) working in this repository.

This file is the **entry point**. For full details, read the files it points to.

## Start here

1. **`CLAUDE.md`** — complete project instructions: methodology, git workflow, commit conventions, storage model, quality gates, and hard rules. **Read this first.**
2. **`docs/README.md`** — documentation index with cross-references to methodology, operations, schemas.
3. **`forgeplan health`** — run this in the terminal to see current project state (blind spots, orphans, stale artifacts).

## What this project is

**Forgeplan** — Rust-based methodology engine (CLI + MCP server + future Desktop app) for managing engineering artifacts (PRD, RFC, ADR, Epic, Spec, Evidence) with quality scoring, semantic search, and decision tracking.

- **Language:** Rust 1.75+ (crates workspace)
- **Storage:** Markdown files in `.forgeplan/` as source of truth (ADR-003), LanceDB as derived index
- **Distribution:** cargo-dist binaries, brew formula, install script
- **Website:** Astro + Starlight at `website/` (see `website/README.md`)

## Hard rules (non-negotiable)

1. **Follow the Forgeplan methodology itself** when making non-trivial changes:
   - `forgeplan route "task"` → determine depth (tactical / standard / deep / critical)
   - `forgeplan new <kind>` → create artifact for Standard+ depth
   - `forgeplan validate` → must PASS before coding
   - `forgeplan reason` → ADI reasoning (mandatory for Deep+)
   - Code → test each `pub fn` immediately
   - `forgeplan new evidence` + link + score + activate

2. **Never commit to `main` or `dev` directly.** Always feature branch → PR.

3. **Never delete `.forgeplan/` without `forgeplan export` first.**

4. **Never push `--force` to `main`.** The safety hook blocks this.

5. **`cargo fmt` + `cargo check` before every commit.** Git hooks enforce this.

6. **Write tests for every new `pub fn` immediately** — do not move to the next function without a test.

7. **Markdown files in `.forgeplan/` are the source of truth** (per ADR-003). The LanceDB index in `.forgeplan/lance/` is derived — rebuild via `forgeplan scan-import` if needed.

## Parallel agents and the build directory

`target/` in this repository runs 20-45 GB. The disk it lives on is routinely
under 20 GB free. Several agents compiling at once is the single most reliable
way to break a session here, and it has happened four times.

**The failure does not look like a failure.** When the disk fills, cargo dies
with `ld: write() failed, errno=28`, and the harness around it reports
`passed=0 failed=0` at exit 0 — or an empty summary. A check that never ran and
a check that passed are indistinguishable in every summary format we use. On
one occasion this produced 26 phantom clippy warnings that did not exist. Twice
the result was reported as green before anyone noticed.

Rules for anyone dispatching sub-agents in this repo:

- **Never let more than one agent compile at a time.** No parallel `cargo
  build` / `test` / `check` / `clippy`. Feature unification means agents with
  different flags will also thrash each other's artifacts even when the disk
  holds.
- **Prefer read-only review.** Reading source with Read/Grep answers most
  review questions. When execution is genuinely needed, hand agents the
  already-built `./target/debug/forgeplan` and let them work in `/tmp`
  workspaces via `forgeplan init -y`. Say so in the prompt — agents reach for
  `cargo test` by default.
- **Check `df -h .` before any full run**, and again in the same command that
  reports the result. Under ~10 GB free, clean first:
  `cargo clean -p forgeplan -p forgeplan-core -p forgeplan-mcp` recovers
  15-37 GB while keeping compiled dependencies. `find target -maxdepth 2 -type
  d -name incremental -exec rm -r {} +` is the cheap version — note that the
  safety hook blocks `rm -rf` but permits this form.
- **Make the harness say when a run did not happen.** Count the test binaries,
  not just the pass/fail line:

  ```bash
  n=$(grep -c '^test result:' "$log")
  if [ "$n" -eq 0 ]; then echo "RUN DID NOT HAPPEN"; df -h . | tail -1; fi
  ```

  A summary that cannot distinguish "zero failures" from "zero tests" is worse
  than no summary.

Two related traps, same family:

- **Never edit source while a background build runs.** The binary captures the
  file mid-edit, so the tree and the artifact disagree. It presents as a real
  defect visible only in E2E while unit tests pass — suspect the binary before
  the code.
- **Two agents in one worktree is a race.** Use `git worktree add` per agent
  for anything that writes. A concurrent commit from a second session has
  already cost a lost commit-message file and half an hour of misdiagnosis in
  this repo.
- **The exit code you print through a pipe is the wrong one.** `cmd | tail` and
  `echo $?` reports `tail`. `${PIPESTATUS[0]}` is a bashism and expands to the
  empty string in this repo's zsh, so the guard prints `exit=` and reads as
  fine. Redirect to a file and measure without a pipe:
  `cargo clippy ... > /tmp/x.log 2>&1; echo "exit=$?"`. Same family as the
  disk-full trap above — the number is real, it is just measuring something
  else.

## Быстрый цикл Rust: поиск, символы, адресные проверки

Эти правила дополняют, а не заменяют полные проверки и ограничения диска выше.

### Найти код без полной загрузки файлов

1. Сначала ограничить область задачи файлами или модулем. Для известного текста
   использовать Grep; для конструкции Rust — `ast-grep`:

   ```sh
   ast-grep outline crates/forgeplan-core/src/constellation/owner.rs --view signatures
   ast-grep outline crates/forgeplan-core/src/constellation/owner.rs --view expanded
   ast-grep run --lang rust --pattern '$EXPR.unwrap()' crates/forgeplan-core/src/constellation/owner.rs
   ```

   `outline --view signatures` показывает верхний уровень; для методов и членов
   использовать `--view expanded`. Карта экспериментальная и не разрешает типы.
   Совпадение `.unwrap()` само по себе не дефект: различать production, tests
   и инварианты. Не выгружать AST или весь workspace в контекст.
2. Определения, реализации и использования проверять через Serena/rust-analyzer
   или LSP. Перед изменением публичной сигнатуры получить references и мигрировать
   callers. AST-поиск не разрешает типы и не заменяет проверку символов.
   Перед работой в отдельном worktree проверить, что Serena видит файл именно
   этого дерева. `FileNotFound` при существующем файле — не повод искать или
   редактировать одноимённый файл в другом checkout: проверить привязку сервера,
   а до её исправления использовать LSP с абсолютным путём целевого worktree.
3. Прочитать найденные реализации и соседний существующий образец. Отсутствие
   LSP-диагностики не считать успешным `cargo check`: сервер может ещё индексировать
   проект или не запускать Cargo-проверку.

### Serena и Git worktrees

- Запускать MCP из корня нужного checkout с `serena start-mcp-server --context ide
  --project-from-cwd --transport stdio --enable-web-dashboard false`. Не закреплять
  в общей настройке абсолютный `--project` основного checkout.
- Корень Git worktree — отдельный проект Serena: свой процесс/активный root и
  `.serena/cache`. Общий `.git` не означает общий индекс исходников. Не объединять
  кэши ссылкой и не переключать один активный root между параллельными агентами.
- Для первого полного прогрева выполнить `serena project index .` из нужного
  checkout; затем символы запрашивать штатными tools. Кэш проверяется при обращении,
  а не является обещанием постоянно обновляемого полного снимка всех файлов.
  Не запускать полное индексирование после каждой правки, через Git hooks или
  рекурсивно через `activation_command`.
- `.mcp.json` и `.serena/` здесь gitignored: новый worktree не получает их через
  Git автоматически. При подготовке проверить локальную конфигурацию и smoke
  `get_symbols_overview` на файле именно этого дерева; не копировать чужие
  credentials или registry целиком. Для одинакового `project_name` задать уникальное
  имя через `.serena/project.local.yml`; активировать проект по пути, не по имени.
- Навигационный rust-analyzer Serena работает с `checkOnSave: false` и
  `cargo.buildScripts.enable: false`; проверку компилятора выполняет build-owner.
  Это уменьшает фоновую диагностику и может ограничить сведения о generated code
  и proc macros. Отсутствие ошибок Serena не заменяет проверку сборки.
- Изменение MCP-конфига действует при новом подключении. Уже запущенный сервер
  остаётся на прежнем root; переподключить его и повторить smoke, не заявлять
  переключение только по факту изменения JSON.

### Проверять связную правку, а не пересобирать весь workspace после каждой строки

Только назначенный build-owner, после завершения правок и проверки диска:

```sh
# Типы и тестовые targets затронутого пакета, без финальной линковки приложения.
cargo check -p forgeplan-core --all-targets

# Небольшая выборка: обычный cargo test, без накладных расходов nextest.
cargo test -p forgeplan-core --lib -- --test-threads=1
cargo test -p forgeplan-core --features test-helpers --test constellation_embedding_preservation -- --test-threads=1

# Nextest — ради возможностей runner или измеренного выигрыша на выбранном наборе.
cargo nextest run -p forgeplan-core --lib --test-threads=1
```

Для CLI пакет называется `forgeplan`, для MCP — `forgeplan-mcp`.
Использовать требуемые задачей features; группировать проверки с одинаковыми
features последовательно в одном `target`, не переключать их без необходимости.
`--test-threads=1` сохраняет изоляцию нынешних model/environment fixtures;
параллельное исполнение тестов включать лишь для проверенно изолированного набора.
Doctests запускать отдельно через `cargo test --workspace --doc`.
Ни `cargo test --lib`, ни `nextest --lib` не запускают integration targets
или doctests. Выбирать `--test <name>` для соответствующего сценария; запуск
только unit-тестов не закрывает проверку CLI/MCP.

Не обещать ускорение nextest по факту установки: на коротких тестах его
накладные расходы могут быть выше. Изолированный опыт подтверждает рабочий
подход, но не является benchmark ForgePlan и не обосновывает смену runner в CI.

`cargo check` не доказывает codegen, линковку или поведение. После законченного
изменения нужны соответствующие тесты и настоящий CLI/MCP smoke; при изменении
FFI/native dependencies/linker — ранняя сборка. Перед сдачей выполнить всю
обязательную матрицу задачи, включая no-feature/semantic и E2E, без её сокращения.

### Не терять кэш и не дублировать работу

- Не делать `cargo clean` после обычной ошибки. Очистка — отдельное согласованное
  действие при нехватке места; чужие target/model caches не трогать.
- Не менять toolchain, `RUSTFLAGS`, Cargo profiles или `CARGO_TARGET_DIR` ради
  предполагаемого ускорения: это может инвалидировать кэш. `line-tables-only`
  уже настроен. Не включать глобально sccache и не отключать incremental без
  сравнения cold/warm времени; sccache не устраняет финальную линковку.
- IDE/rust-analyzer check-on-save, Bacon и агенты не должны параллельно запускать
  Cargo. Перед передачей build-owner согласовать фоновые проверки; не отключать
  диагностику редактора молча.
- Для подагента задавать ограниченный набор файлов, один контракт и формат
  находки `file:line → условие → последствие`. Независимые срезы читать параллельно;
  повторное ревью направлять на diff исправления и его callers, а не заново на
  весь репозиторий. Неизменённые файлы подтверждать прежним отчётом и fingerprint.
- Новые семантические индексы (CocoIndex/SCIP) и cargo-hakari вводить только после
  замера конкретной задержки; их наличие не является условием обычной работы.

## Working mode: /smith first, then role-matched subagents

Standing owner rule (2026-10): every **non-trivial** task goes through two
steps, in order:

1. **`/smith`** — routing. Reads state (`forgeplan health`, git, memory),
   classifies the context against the routing matrix, and returns a Plan that
   names which specialist subagents to dispatch and in what order. Smith picks
   the route; it never writes code itself.
2. **Role-matched dispatch** — each plan step goes to a subagent of the right
   profile: `subagent_explore` for read-only research, `subagent_general` for
   implementation. File-disjoint steps may run in parallel; **at most one
   agent compiles at a time** (see "Parallel agents and the build directory" —
   the disk rule above applies to every dispatch, no exceptions).

Exempt: single-line tactical fixes, conversational answers, and **stateful
runs** (live processes, DB observation, interactive sessions) — a subagent has
no long-lived process and no session memory, so those stay in the main thread.

## Searching the tree: tgrep, not rg

This repo has a **tgrep** server (`microsoft/tgrep`, trigram index + watcher).
Prefer it over `rg`/`grep` for content search — it answers in milliseconds and
its exit codes/flags follow ripgrep.

```bash
tgrep -- "fn parse_config" "$PWD"          # regex
tgrep -F -- "Vec<Option<T>>" "$PWD"        # literal string — prefer for symbols
tgrep -t rust -l -- "plan_reconcile" "$PWD"  # files only, narrow first
tgrep -g "src/**" -C 2 -- "TODO|FIXME" "$PWD"
tgrep --vimgrep -- "pattern" "$PWD"        # file:line:col, jump-ready
tgrep status "$PWD"                        # server + index health
```

Rules that bite if ignored:

- **`--` before the pattern, absolute path as root.** Bare `index`, `serve`,
  `status`, `search` are parsed as subcommands; everything after `--` is the
  pattern and paths. All flags go before `--`.
- **Pass `$PWD`, not `.`** — this checkout is a *worktree*; `tgrep status .`
  resolves through the gitdir to the main worktree and reports a missing
  index. Absolute path is correct everywhere.
- **Freshness**: the watcher updates the index asynchronously. A search issued
  immediately after an edit can run before the index caught up — use
  `--no-index` when the search must read the exact bytes on disk right now
  (e.g., verifying an edit you just made, or the "another agent touched the
  file" races described above).
- **`.tgrep/` is generated state** — index + `serve.json` (PID/port). It is
  gitignored; never commit it. If the server dies: `tgrep serve .` rebuilds.
- **Flags that force a slow full scan**: `--no-index`, `-u`, `-a`, `--binary`,
  `-L`, naming a single file. Fine on small trees; avoid on the whole repo.
- **Fallback**: if `tgrep` is absent or the server is down, `rg` is still the
  fallback — the server isn't a hard dependency.

## Findings have a home, and it is not the chat

A finding that lives only in a reply is gone at the end of the session. This has
already cost real work here: a measurement across 465 artifacts that cancelled a
planned fix lived nowhere but a chat message until someone thought to file it,
and a handoff document once pointed at a scratch directory that did not survive
the session it described.

**File it the moment you find it, before finishing the thought that produced
it.** Three kinds of finding, three homes:

| Finding | Home | Why there |
|---|---|---|
| A rule that changes how an agent works | **this file** | Read every session. Costs context every session, so it earns its place only by changing behaviour. |
| Work someone should do | **GitHub issue**, open | Assignable, closable, outside the context budget. |
| A measured dead end — checked, decided not to do | **GitHub issue, closed on arrival**, label `measured-not-planned` | Findable by `gh issue list --state closed --label measured-not-planned`. Costs nothing to keep. |

The third row is the one people skip, and it is the one that pays. A measurement
that **cancels** work is worth as much as one that starts it, and it evaporates
faster — nobody files "we checked and it does not matter". Then the next person
smells the same thing and re-derives it. #476 is the worked example: two
plausible defects, both measured, both left alone, numbers on the record.

Rules for filing:

- **Numbers, not adjectives.** "Zero verdicts change across 465 artifacts"
  survives a year. "Seems fine" does not.
- **Every finding carries a trigger to revisit, phrased as a condition.**
  `"later"` is not a trigger. `"if stripping comments would change the verdict
  on one or more artifacts"` is. Same rule ADRs already follow: name what must
  become true for someone to pick this up.
- **Say what was rejected and why.** A finding that records only the conclusion
  invites the next person to relitigate the option you already killed.
- **Do not file the same thing twice.** `gh issue list --search` before
  creating; extend the existing issue if one fits.

Revisit is a step, not a habit: **before opening a `release/v*` PR**, list the
`measured-not-planned` issues and re-check their triggers against the current
corpus. A trigger that has become true reopens the issue; one that has not gets
left alone without discussion.

## GitHub mechanics that silently do nothing

- **`Closes #N` in a PR body does not close anything when merging into `dev`.**
  GitHub honours closing keywords only on merges into the *default* branch, and
  ours is `main`. Feature branches merge into `dev`, so the keyword is inert and
  the issue stays open with no error anywhere. Close the issue by hand at
  dev-merge, naming the merge commit — leaving it open until the release means
  the board says "nobody did this" about finished work.
- **An empty `gh` result is not proof of absence.** `Post
  "https://api.github.com/graphql": EOF` prints nothing to stdout and can exit
  through a pipe as success. Verify a listing you are about to act on by
  querying the object directly, not by trusting that zero rows means zero
  things.

## Repository structure (quick map)

```
ForgePlan/
├── CLAUDE.md, AGENTS.md, README.md
├── crates/                ← Rust workspace (core + cli + mcp)
├── .forgeplan/            ← artifact workspace (markdown tracked, lance/cache/config local)
│   ├── adrs/, rfcs/, prds/, epics/, specs/
│   ├── evidence/, problems/, solutions/, notes/
│   ├── lance/             ← gitignored (derived)
│   └── config.yaml        ← gitignored (local)
├── docs/                  ← production documentation
│   ├── README.md          ← documentation index
│   ├── methodology/       ← how to use Forgeplan
│   ├── operations/        ← agent hooks, enforcement, repo protection
│   └── schemas/           ← formal artifact schemas
├── templates/             ← markdown templates for each artifact kind
├── website/               ← official website (Astro + Starlight)
├── marketplace/           ← plugin marketplace (plugins + skills)
├── scripts/               ← build + release + helper scripts
├── Formula/               ← Homebrew formula
└── .local/                ← gitignored — local notes, research, sessions
```

## Language

- **Documentation & commit bodies:** Russian preferred (matches project conventions)
- **Code identifiers & commit descriptions:** English
- **Communication with the user:** Russian

## How to explain things to the owner

Set by the owner on 2026-09-05, after comparing two answers about the same
decisions: one written in architecture-speak, one in plain language. The
plain one worked; the other one — «хрен поймёшь, о чём речь».

When presenting a decision, a trade-off, or a piece of analysis:

- **Plain, but not banal.** Simple words carrying real content — never
  simple words instead of content. The test: the reader should be able to
  retell the decision to someone else after one read.
- **Name the collision before the answer.** Most decisions exist because two
  things conflict. Show the conflict first («два действующих ADR говорили
  противоположное»), then the resolution. An answer without its tension
  reads as arbitrary.
- **One live example beats three definitions.** «Приходит задача — кто
  решает, что сначала запустится аналитик, потом кодер?» explains an
  orchestration boundary faster than any glossary.
- **Minimal anglicisms.** Code identifiers, artifact kinds, and command
  names stay as-is (`claim`, `EvidencePack`, `based_on`). Everything else
  gets a Russian phrasing: «замок на запись», not «лок»; «происхождение»,
  not «провенанс»; «поверхность команд» needs an explanation the first time
  it appears.
- **Consequences, not just verdicts.** «Deprecated» is a verdict; «хоронить,
  не лечить — всё, что она обещала, уже делают NOTE и Hindsight» is a
  decision someone can agree or argue with.
- **Metaphors must carry weight.** «Нотная тетрадь и приёмная комиссия» is
  good because both halves map to real subsystems. Decoration is worse than
  nothing.

## Authorship (single author)

Forgeplan is a single-author project. When generating ANY author-attributed content, use:

- **Name:** `Eli Rum`
- **URL:** `https://elirum.me`

This applies to:

- Blog post `AuthorBlock` (`website/src/components/blog/AuthorBlock.astro`)
- Blog footer author line (`website/src/components/blog/BlogFooter.astro`)
- JSON-LD `author.name` + `Person.url` (`website/src/components/blog/SeoMeta.astro`)
- Schema.org `Organization.founder` (`website/src/components/SiteJsonLd.astro`)
- OpenGraph `article:author` meta tags
- `package.json` `author` field (if updated)
- Any author byline in `docs/`, `.forgeplan/notes/`, blog `.mdx` frontmatter
- Conventional commit `Co-Authored-By` lines for solo work (do not add — Eli Rum is sole author)

**DO NOT use** placeholder names: `Mike Kubal`, `Forgeplan Author`, `Anonymous`, `Maintainer`, etc.

If a future contributor joins, this section will be updated to reflect multi-author conventions. Until then, treat the single-author invariant as load-bearing for credibility, SEO authority signal, and brand consistency.

## See also

- [`CLAUDE.md`](CLAUDE.md) — full project instructions (primary)
- [`docs/README.md`](docs/README.md) — documentation index
- [`docs/methodology/FORGEPLAN-GUIDE.md`](docs/methodology/FORGEPLAN-GUIDE.md) — full methodology reference
- [`docs/operations/AGENT-ENFORCEMENT.md`](docs/operations/AGENT-ENFORCEMENT.md) — agent rules and guardrails
- [`website/README.md`](website/README.md) — website architecture notes
