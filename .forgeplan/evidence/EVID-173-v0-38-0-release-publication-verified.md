---
depth: tactical
id: EVID-173
kind: evidence
links:
- target: RFC-014
  relation: informs
- target: PRD-087
  relation: informs
- target: EVID-172
  relation: informs
status: active
title: v0.38.0 release publication verified
---

---
assigned_number: 173
created: 2026-10-12
predicted_number: 173
slug: evid-v0-38-0-release-publication-verified
updated: 2026-10-12
---

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Наблюдение

Релиз v0.38.0 опубликован после merge PR #520 и повторного полного CI после авторизованного Devin smoke.

## Проверки

- PR #520 смержен в `main` коммитом `d56466b10ddf788c62a6e8acffd0e0f578d0c1d1`.
- Post-merge main gates: CodeQL и `cargo-deny` — PASS.
- Аннотированный тег `v0.38.0` указывает на merge commit `d56466b10ddf788c62a6e8acffd0e0f578d0c1d1`.
- cargo-dist run `38296327088` — PASS: пять target builds, global artifacts, GitHub Release, Homebrew formula и announce.
- GitHub Release: `https://github.com/ForgePlan/forgeplan/releases/tag/v0.38.0`, не draft и не prerelease.
- Опубликованный `forgeplan-aarch64-apple-darwin.tar.xz`: checksum PASS (`e5af4032f04155f073432aecd7aeb0a483041db0a58a49b9e027f762af259210`).
- Бинарник из опубликованного архива: `forgeplan 0.38.0`; команда `embed` присутствует, значит release собран с semantic-search.
- Локальная Homebrew-установка после обновления: `brew list --versions forgeplan` → `forgeplan 0.38.0`; `/opt/homebrew/bin/forgeplan reason --help` перечисляет `claude-code`, `codex-cli`, `devin-cli`, `gemini-cli`, `cursor-agent`.
- PR #524 синхронизировал `main` обратно в `dev`; merge commit `6d121779c507c67d61131a51e7918c16f92d8879`. `dev` содержит release merge commit и workspace version `0.38.0`.

## Ограничение

Первый запуск опубликованного `embed` начал загрузку и загрузку в память модели (~2.1 GB), но локальный smoke был остановлен таймаутом 600 s. Это не засчитано как успешная embedding operation. Корректность embedding отдельно доказана зелёным CI `Embedding correctness oracle`; наличие feature в опубликованном бинарнике доказано командой `embed --help`.

## Связанные доказательства

- EVID-172 — реальные Codex и Devin ADI, contract tests и финальный release-PR CI.
- RFC-014 — контракт local subscription CLI providers.
- PRD-087 — требования функции.





