---
depth: standard
id: PRD-023
kind: prd
status: active
title: Distribution — brew, GitHub Actions, install.sh, crates.io
---

---
id: PRD-023
title: "Distribution — brew, GitHub Actions, install.sh, crates.io"
status: Draft
author: gogocat
created: 2026-04-04
updated: 2026-04-04
epic: EPIC-001
priority: P1
depth: deep
domain: general
projectType: cli_tool
stepsCompleted: []
---

# PRD-023: Distribution — brew, GitHub Actions, install.sh, crates.io

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/8  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/8  (  0%)
```

---

## Executive Summary

### Vision

Forgeplan устанавливается одной командой на macOS, Linux и Windows — без ручной сборки из исходников.

### Problem

Сейчас Forgeplan можно установить только собрав из исходников (`cargo build --release`). Это требует установленного Rust toolchain (rustup + cargo), ~2 минут компиляции с 163 зависимостями, и ручной настройки PATH для бинарника. Каждый потенциальный пользователь вынужден проходить через этот барьер, что делает невозможным рекомендацию инструмента коллегам без Rust-опыта. Кроме того, отсутствие автоматизированного release pipeline означает, что каждый релиз требует ~10 ручных шагов: сборка, тестирование, тегирование, загрузка бинарников.

**Impact**: 100% потенциальных пользователей без Rust отсекаются на шаге установки. Нулевой adoption за пределами разработчика. Ручной релиз-процесс замедляет итерации и создаёт риск человеческой ошибки.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Rust Developer | Разработчик с Rust toolchain, знает cargo | Хочет `cargo install forgeplan` без клонирования репо |
| macOS Developer | Разработчик на Mac без Rust | Хочет `brew install forgeplan` как любой CLI tool |
| CI/CD Pipeline | GitHub Actions workflow | Нужен `curl | sh` или предсобранный бинарник для automation |
| Linux User | Разработчик на Linux без Rust | Хочет скачать бинарник или `curl | sh` |

### Differentiators

- Single binary (~41MB) без runtime зависимостей — проще распространять чем Node/Python tools
- Cross-platform: macOS (arm64 + x86_64), Linux (x86_64, musl), Windows (x86_64)
- Автоматический release pipeline — каждый git tag = новая версия на всех каналах

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Install time macOS | Seconds from zero to `forgeplan --version` | ~120s (compile) | < 30s | v0.13.0 | Manual test on clean machine |
| SC-2 | Install channels | Number of working install methods | 1 (cargo build) | 4 (brew, cargo install, curl, GH release) | v0.13.0 | Smoke test each channel |
| SC-3 | Release automation | Manual steps per release | ~10 (build, tag, upload) | 1 (push tag) | v0.13.0 | Count manual steps |
| SC-4 | Platform coverage | Platforms with prebuilt binaries | 1 (macOS arm64) | 5 (macOS arm64/x86, Linux x86/musl, Windows) | v0.13.0 | GH release assets count |

---

## Product Scope

### MVP (In-Scope)

- GitHub Actions release workflow (trigger on tag push, cross-compile, upload assets)
- Homebrew tap formula (`brew tap forgeplan/tap && brew install forgeplan`)
- Shell install script (`curl -fsSL ... | sh` for macOS/Linux)
- `cargo install forgeplan` (publish to crates.io)
- `fpl` alias (symlink created by install script)

### Out of Scope

- Windows MSI/exe installer (prebuilt binary + manual PATH only)
- Docker image
- apt/yum/pacman packages (future)
- Auto-update mechanism
- Code signing / notarization (future, macOS Gatekeeper)
- forgeplan.dev website

### Growth Vision

- macOS notarization for Gatekeeper
- Chocolatey/Scoop for Windows
- Official Docker image for CI pipelines
- apt/yum repos for Linux distros

---

## User Journeys

### Journey 1: macOS Developer — Brew Install

**Goal**: Install forgeplan via Homebrew in < 30 seconds

| Step | User Action | System Response | Notes |
|-----|------------|----------------|-------|
| 1 | `brew tap forgeplan/tap` | Tap added | One-time setup |
| 2 | `brew install forgeplan` | Downloads prebuilt binary, installs to /usr/local/bin | ~10s on fast network |
| 3 | `forgeplan --version` | `forgeplan 0.13.0` | Confirms install |
| 4 | `fpl health` | Project health output | Alias works |

**Result**: Working forgeplan + fpl alias, no Rust required.

### Journey 2: Rust Developer — Cargo Install

**Goal**: Install via crates.io ecosystem

| Step | User Action | System Response | Notes |
|-----|------------|----------------|-------|
| 1 | `cargo install forgeplan` | Compiles and installs binary to ~/.cargo/bin | ~2 min |
| 2 | `forgeplan --version` | Version output | PATH already set by rustup |

**Result**: Latest version from crates.io, standard Rust workflow.

### Journey 3: CI Pipeline — Curl Install

**Goal**: Add forgeplan to CI in one line

| Step | User Action | System Response | Notes |
|-----|------------|----------------|-------|
| 1 | Add `curl -fsSL https://raw.githubusercontent.com/.../install.sh \| sh` to workflow | Downloads correct binary for OS/arch | Auto-detects platform |
| 2 | `forgeplan init -y` | Workspace created | Non-interactive mode |

**Result**: forgeplan available in CI without Rust toolchain.

### Journey 4: Maintainer — Release New Version

**Goal**: Release new version with one command

| Step | User Action | System Response | Notes |
|-----|------------|----------------|-------|
| 1 | `git tag -a v0.13.0 -m "Release"` | Tag created | |
| 2 | `git push origin v0.13.0` | GH Actions triggered | |
| 3 | Wait ~10 min | Binaries built, GH release created, brew formula updated | Fully automated |

**Result**: All distribution channels updated from single tag push.

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | CI/CD | Must | Maintainer can trigger cross-platform build by pushing a version tag | Journey 4 |
| FR-002 | CI/CD | Must | Build pipeline can produce prebuilt binaries for all supported platforms | Journey 4 |
| FR-003 | CI/CD | Must | Build pipeline can publish release with all binaries as downloadable assets | Journey 4 |
| FR-004 | Package | Must | macOS user can install forgeplan via system package manager without Rust toolchain | Journey 1 |
| FR-005 | Package | Must | Rust developer can install forgeplan from the central Rust package registry | Journey 2 |
| FR-006 | Install | Must | User can install forgeplan via shell script that auto-detects OS and architecture | Journey 3 |
| FR-007 | Install | Should | Install method can create `fpl` shorthand alias for `forgeplan` binary | Journey 1, 3 |
| FR-008 | CI/CD | Should | Build pipeline can generate cryptographic checksums for all release artifacts | Journey 3, 4 |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Size | Release binary shall not exceed 50MB | < 50MB | All platforms | `ls -la` on release artifacts |
| NFR-002 | Speed | Brew install shall complete | < 30s | 50 Mbps connection | Manual timing |
| NFR-003 | CI Time | Release pipeline shall complete | < 15 min | All 5 targets | GH Actions run duration |
| NFR-004 | Security | Install script shall verify checksums | SHA256 match | Before executing binary | Script logic review |

---

## Acceptance Criteria

### AC-1: Brew Install on Clean macOS

```gherkin
Given a macOS machine without Rust toolchain
When  user runs `brew tap forgeplan/tap && brew install forgeplan`
Then  `forgeplan --version` outputs the current release version
And   `fpl --version` outputs the same version
```

### AC-2: Tag-Triggered Release

```gherkin
Given code on main branch passes all tests
When  maintainer pushes tag `v0.13.0`
Then  GitHub Actions builds binaries for 5 platforms
And   creates GitHub Release with all binaries and checksums
And   completes within 15 minutes
```

### AC-3: Curl Install on Linux

```gherkin
Given a Linux x86_64 machine without Rust
When  user runs the install script via curl
Then  forgeplan binary is placed in /usr/local/bin (or ~/.local/bin)
And   `forgeplan --version` outputs the correct version
```

### AC-4: Cargo Install from Registry

```gherkin
Given a machine with Rust toolchain installed
When  user runs `cargo install forgeplan`
Then  binary is compiled and placed in ~/.cargo/bin
And   `forgeplan --version` outputs the published version
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| GitHub Actions runners (ubuntu, macos, windows) | Infrastructure | Ready | GitHub |
| crates.io account | External | Ready | gogocat |
| Homebrew tap repo (forgeplan/homebrew-tap) | Infrastructure | To Create | gogocat |
| cross-rs or cargo-zigbuild | Build tool | Evaluate | Sprint |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | crates.io publish is irreversible (can't unpublish) | High | High | Thorough testing before `cargo publish`, use `--dry-run` first | gogocat |
| R-2 | Cross-compilation fails for some targets | Medium | Medium | Use `cross-rs` or `cargo-zigbuild` for reliable cross-compile | Sprint |
| R-3 | macOS Gatekeeper blocks unsigned binary | Medium | High | Document `xattr -d` workaround in install output, plan notarization for v2 | Backlog |
| R-4 | Install script security (`curl | sh` anti-pattern) | Low | Medium | Provide checksums, document manual download alternative | Sprint |

---

## Timeline

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| PRD-023 Approved | 2026-04-04 | Requirements locked |
| RFC Architecture | 2026-04-05 | Build pipeline + install script design |
| ADR: Cross-compile strategy | 2026-04-05 | cross-rs vs zigbuild vs native runners |
| MVP Implementation | 2026-04-06 | GH Actions + brew + install.sh + crates.io |
| Smoke Test All Channels | 2026-04-06 | Verify each install method |

---

## Affected Files

- `.github/workflows/release.yml` (new)
- `install.sh` (new)
- `Cargo.toml` (metadata for crates.io)
- `crates/forgeplan-cli/Cargo.toml` (publish settings)
- `crates/forgeplan-core/Cargo.toml` (publish settings)
- `crates/forgeplan-mcp/Cargo.toml` (publish settings)
- `Formula/forgeplan.rb` (new, in separate tap repo)

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-001 | Parent epic | Active |
| RFC-006 | Architecture proposal | To Create |
| ADR-006 | Cross-compile decision | To Create |

---

