---
depth: standard
id: NOTE-055
kind: note
last_modified_at: 2026-10-16T20:57:11.722339+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: ADR-027
  relation: informs
status: draft
title: ADR-027 bounded Deep engine invocation failure and preserved receipts
---

---
assigned_number: 55
created: 2026-10-16
predicted_number: 55
slug: note-adr-027-bounded-deep-engine-invocation-failure-and-preserved-receipts
updated: 2026-10-16
---

# ADR-027 — неуспешная ограниченная попытка нового Deep ADI

Автор: Eli Rum, https://elirum.me. Profile A creator/operator; назначенный OMP Sol6.1 High. Это фактическая запись вызова, не независимый Evidence, не gate verdict и не engine analysis.

## Authorization and boundary

Исходное разрешение владельца: «Да, оформить и в core»; «Отдельно оформить изменение регламента с независимой проверкой; остальные gates остаются обязательными.» Полное происхождение закреплено в ADR-027. Текущая операция разрешала ровно один новый bounded вызов на ADR-027 и один на RFC-014, только process-local provider/model и временный PATH adapter, без config/auth/source/global изменений и без повторов старого website reasoning. ADR-027 не повторялся после ошибки. Историческое ограничение в ADR/RFC сохраняется.

## Actual receipt

```json
{
  "operation": "single authorized ADR-027 CLI attempt; no retry",
  "workspace": "/Users/explosovebit/Work/ForgePlan",
  "command": [
    "/opt/homebrew/bin/forgeplan",
    "reason",
    "ADR-027",
    "--json",
    "--save"
  ],
  "process_local_provider": "codex-cli",
  "process_local_model": "gpt-6.1-sol",
  "outer_deadline_seconds": 720,
  "started_utc": "2026-10-16T20:53:38Z",
  "finished_utc": "2026-10-16T20:53:39Z",
  "exit_code": 1,
  "examined_body_sha256": "47b8239f90ac5b28eca1d6f7503b1e4fdb3de9b77960b2a11b3d1effdbeec1ab",
  "stdout_bytes": 63,
  "stdout_sha256": "0b51da418e4d30b371496467d221234fc797cbd42a375d7af87b4fa2c7816b08",
  "stderr_bytes": 605,
  "stderr_sha256": "2f705513b563f11bb185132871581cdbc193a0162f00dbad9d17b8191ca15143",
  "forgeplan_version": "0.38.0",
  "forgeplan_binary_sha256": "55c32b6d230b4ebd56a6c4e3f38a6fb20b5c31f8ee91558c0850fc92c3771113",
  "stock_codex_path": "/Users/explosovebit/.local/share/mise/installs/node/22/bin/codex",
  "stock_codex_version": "0.163.0",
  "stock_codex_sha256": "61b0194f3bb6534439c8d26a3ed57d0805f84b884588b761795323eeb92fcf70",
  "adapter_before_sha256": "bbf6ec46ce71401714d63f305b962c4e73db86d71ca5ae55b4a66004670c7dd4",
  "argv_receipt_exists": false,
  "analysis_saved_by_save": false,
  "vendor_stderr": "withheld by ForgePlan; not read or reconstructed",
  "interpretation": "No accepted engine result; model/effort execution not established. Provider release source clears custom env; original adapter required FORGEPLAN_ENGINE_RECEIPT before exec. This supports adapter-preflight failure, not a verified login failure.",
  "source_references": [
    "https://raw.githubusercontent.com/ForgePlan/forgeplan/v0.38.0/crates/forgeplan-core/src/llm/local_cli.rs",
    "https://raw.githubusercontent.com/ForgePlan/forgeplan/v0.38.0/crates/forgeplan-core/src/playbook/dispatch/helpers.rs"
  ]
}
```

Фактическая shell-команда вызова: `/bin/bash /tmp/forgeplan-core-engine.SGWouD/run-reason ADR-027`. Runner передал process-local `PATH=/tmp/forgeplan-core-engine.SGWouD/bin:$PATH`, `FORGEPLAN_LLM_PROVIDER=codex-cli`, `FORGEPLAN_LLM_MODEL=gpt-6.1-sol` и служебную `FORGEPLAN_ENGINE_RECEIPT=/tmp/forgeplan-core-engine.SGWouD/ADR-027`; последний ключ не является ForgePlan setting. stdout/stderr захвачены отдельно; exit измерен у самого CLI, не у pipe.

## Complete stdout

```text
  Analyzing ADR-027 with ADI cycle (codex-cli/gpt-6.1-sol)...

```

## Complete stderr

```text
ForgePlan local CLI provider: reusing a stock CLI login under your vendor subscription. Personal/local development only — not for production, shared, or CI environments. Usage draws from the vendor plan and is subject to its terms; ForgePlan does not extract tokens or spoof client identity.
Error: ADI reasoning failed: codex-cli provider: `codex` exited with status Some(1); vendor stderr was withheld because it may contain prompt or environment data; run `codex login`.
Fix: local provider `codex-cli` is unavailable — run codex login; then retry `forgeplan reason ADR-027`
Error: LLM call failed
```

## Interpretation and next prerequisite

`--save` не дошёл до создания NOTE. Полного ADI output нет, hypotheses count не определён, success/High/model execution не засчитываются. Эта новая NOTE сохраняет факт неуспешной операции, не выдаётся за автоматически сохранённый анализ. Vendor stderr намеренно скрыт продуктом; login hint не доказывает проблему authentication и не даёт разрешения менять login/auth.

После ошибки read-only прочитан исходник установленного релиза v0.38.0. `build_environment`/`build_env_allowlist` вместе с `run_subprocess.env_clear()` удаляют произвольные переменные; Codex extra_env_keys пуст. Исходный adapter требовал `FORGEPLAN_ENGINE_RECEIPT` до записи argv и `exec`. Отсутствие receipt и exit 1 согласуются с этой ранней остановкой. Идентичность исходника релиза с конкретной установленной сборкой отдельно не доказана; заключение основано на версии и наблюдаемом результате, raw vendor diagnostic недоступен.

Для отдельного разрешённого RFC-014 вызова зависимость от custom env устранена только во временном adapter: receipt path задан literal, исходные аргументы и stdin не меняются, добавляется только supported `--config 'model_reasoning_effort="high"'`. Это не повтор ADR-027. Для ADR-027 новый engine-call prerequisite остаётся открытым: нужен отдельно разрешённый будущий вызов через проверяемый process-local путь; текущая авторизация на попытку уже использована. Никакого fallback, нового paid probe, credential inspection или конфигурационной правки не выполнено.

## Gate state

ADR-027/RFC-014 остаются draft/deep. Structural validation обоих дала 0 MUST/0 SHOULD ошибок, но это не independent review. Owner policy, source allowlist 17 файлов, active architecture applicability, safe sequence/red line 7, pre-source reviewer и guardian остаются без авторского PASS. MiniMax source dispatch не разрешён этой записью. EVID-172/173, NOTE-053, PRD-087/SPEC-007 и operative source не менялись на момент сохранения receipt; финальная проверка сохранности записывается в handoff.

## Receipt retention

Run-local файлы сохранены в `/tmp/forgeplan-core-engine.SGWouD/` для следующего reviewer. Их полные stdout/stderr, hashes и status перенесены выше; temporary путь не является единственным durable источником результата. Adapter не установлен глобально и не используется без process-local PATH. Цена вызова/начисление vendor неизвестны; engine success не заявляется.


## Final preservation verification and current pins

Ниже реальные результаты после canonical author updates и links. Это проверка сохранности/происхождения, не независимый verdict. NOTE-055 собственный hash не включён, чтобы не создавать самоссылку; конечный body pin выдаётся в handoff. Исходные 1783 файла включают dirty/untracked baseline и локальный `.forgeplan/config.yaml`; изменились только собственные ADR-027/RFC-014 projections, новые NOTE-054/055 добавлены отдельно. Все 17 будущих source файлов RFC совпали с исходными before hashes. Protected canonical records сравнивались целиком (body/status/depth/links/metadata), raw projections — по точным bytes SHA-256.

```json
{
  "verified_at_utc": "2026-10-16T20:59:43.921175+00:00",
  "baseline_files": 1783,
  "changed_baseline_paths": [
    ".forgeplan/adrs/ADR-027-finite-evidence-backed-independent-review-without-a-defect-quota.md",
    ".forgeplan/rfcs/RFC-014-core-review-policy-bounded-documentation-cutover.md"
  ],
  "source_allowlist_unchanged": [
    {
      "path": "CLAUDE.md",
      "sha256": "38ac9c796127d84278dd00bb1f1d48cb658c5e9fb54a8e79ec1229b25f4549eb"
    },
    {
      "path": "docs/methodology/QUALITY-GATES.md",
      "sha256": "4260eed9183a79d4701292db4678eddc374ec38486502517ec57a7798b1d55e3"
    },
    {
      "path": "docs/methodology/QUALITY-GATES.ru.md",
      "sha256": "a16754d3090ac9d116d98212488d3bdc8f2ea12752778afd8ec241b8890a5bc4"
    },
    {
      "path": "docs/methodology/HOW-TO-USE.md",
      "sha256": "88819a5db9dee46f62006924b098309ccc15a05a9d6a3eeec3a1a6f519a166ea"
    },
    {
      "path": "docs/methodology/HOW-TO-USE.ru.md",
      "sha256": "28d82ed02724e16eecb0abe23011fa15c13f48c781f8191461a464b7127640ce"
    },
    {
      "path": "docs/methodology/GLOSSARY.md",
      "sha256": "651d4ba8ec58c5b4adcf0fcd9fda9410b2480fd05f01f1cdc4ef496f7973a8c0"
    },
    {
      "path": "docs/methodology/GLOSSARY.ru.md",
      "sha256": "13a43610875f79383664a77af499d99a567b3d63cbff8847ab4d703eea7b0f60"
    },
    {
      "path": "docs/methodology/FORGEPLAN-GUIDE.md",
      "sha256": "53f21bc6cda60ee0960de05ef8a247327091d4f9f69399685e2414dd9daa9a3d"
    },
    {
      "path": "docs/methodology/FORGEPLAN-GUIDE.ru.md",
      "sha256": "8325190562e3a209ac0501a12ac3b2972fed152ef1d9c7b35692b3757080dc3a"
    },
    {
      "path": "docs/methodology/METHODOLOGY-COURSE.md",
      "sha256": "ecc9ef9bb357aee75b4e38eeb6d5a0a172851a6c564b4138e2a3d1bff0b5ca39"
    },
    {
      "path": "docs/methodology/METHODOLOGY-COURSE.ru.md",
      "sha256": "bee59df65204ce0a473e33de26fa63405bfd5532c45a1c1857dc282af1e13cd1"
    },
    {
      "path": "docs/methodology/USAGE-BY-ROLE.md",
      "sha256": "6b4608d12d33cd3b81d007c729bac0c170311b09f07b476a3e09a1e7b3f89309"
    },
    {
      "path": "docs/methodology/USAGE-BY-ROLE.ru.md",
      "sha256": "1bd248429d861b7f9d2c6f80a61fd431c9b3d73da995919a80671ea8d79a524f"
    },
    {
      "path": "docs/schemas/PRD-SCHEMA.md",
      "sha256": "a8fc265729dd54aeb02bbfca46f5e0bd0e0c87db19e0685f396fbbe8a85dc7d6"
    },
    {
      "path": "docs/schemas/PRD-SCHEMA.ru.md",
      "sha256": "8912c40b75a96b7076a5c193a208ee88a7815c6e0696bffc5f8af63c87136cf2"
    },
    {
      "path": "templates/prd/_TEMPLATE.md",
      "sha256": "f6a08ace5840bbff35dd9d446292deb9a086ef36eabb75480217e3a9972f40eb"
    },
    {
      "path": "CHANGELOG.md",
      "sha256": "698c09d5f30f306199233c096655d07d2755077ab8dc94fb7a439cf8f7fa519e"
    }
  ],
  "protected_raw_file_hashes_unchanged": {
    ".forgeplan/evidence/EVID-172-prd-087-spec-007-r1-independent-shape-fitness-audit.md": "a0959046856a371d457bc497f6251a827ece9f28b2956663aa6b15ae433c7e7d",
    ".forgeplan/evidence/EVID-173-prd-087-spec-007-note-053-independent-finite-local-shape-re-review.md": "5336eddc3526238893d78ebbaa8ef3392b9f56ac2dcd0634ca5d2de77317885e",
    ".forgeplan/notes/NOTE-053-prd-087-spec-007-bounded-authored-adi-and-factual-shape-handoff-resolution.md": "52788c120dbfdd8fb19c5b51f31005892f8cd53730472da16de56be27014d8ac",
    ".forgeplan/prds/PRD-087-website-onboarding-task-stack-selector-and-revision-bound-public-guide.md": "9f83bd7974b82e07b1b2cb2e90787e1ca742e2afa145c2c8e0f6f52294286e98",
    ".forgeplan/specs/SPEC-007-website-onboarding-bounded-selection-and-public-guidance-behavior.md": "4abb2402c0e24e84383a5dd5cb0b29853502e6d5f5f66d0236c1949405d64833"
  },
  "protected_canonical_records_equal": {
    "EVID-172": true,
    "EVID-173": true,
    "NOTE-053": true,
    "PRD-087": true,
    "SPEC-007": true
  },
  "artifacts": {
    "ADR-027": {
      "status": "draft",
      "depth": "deep",
      "title": "Finite evidence-backed independent review without a defect quota",
      "body_sha256": "e757631639c42e4b44f38a93c9dddb16c260400f465d2b45fb049c3cb5d49da2"
    },
    "RFC-014": {
      "status": "draft",
      "depth": "deep",
      "title": "Core review policy bounded documentation cutover",
      "body_sha256": "6dd0db6465aa30995eaeb18d39318b8bbaee07bbc8c3a6772e18e084b25e7c77"
    },
    "NOTE-054": {
      "status": "draft",
      "depth": "tactical",
      "title": "ADI analysis of RFC-014",
      "body_sha256": "f884501e650d1ad24e848e3cf9d8934d09254de6103fa91345e575815f28f317"
    }
  },
  "structural_validation": "ADR-027 and RFC-014 both 0 errors / 0 warnings; no independent verdict",
  "remaining": "ADR-027 accepted engine result absent; no retries. Independent pre-source review/guardian and lifecycle prerequisites remain. No MiniMax dispatch.",
  "note54_mcp_visibility": "MCP get NOTE-054 returned not found; canonical CLI get and update succeeded. Next reviewer should read actual saved note via owning CLI if MCP still cannot resolve it."
}
```

Оба текущих ADR/RFC повторно structural validated: 0 errors, 0 warnings. CLI дополнительно сообщил, что session routing→coding не разрешён; hints activate/new-prd/score не исполнялись, поскольку находятся вне разрешённой creator роли и не заменяют независимый gate. NOTE-054 была видна и обновлена canonical CLI, но MCP get сразу после --save ответил not found. Эта наблюдённая разница сообщена через report_issue; repair/reindex/перезапуск сервера не выполнялись. Надёжный read path для reviewer — `forgeplan get NOTE-054 --json` из owning workspace.

