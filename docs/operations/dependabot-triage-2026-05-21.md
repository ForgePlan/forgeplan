# Dependabot triage — 2026-05-21 (v0.32.0 release window)

Per RED-LINE #10 (CLAUDE.md): each release tags every open Dependabot alert as **addressed** / **scheduled** / **accepted-with-justification**. Following the `docs/operations/RELEASE-PROTOCOL.ru.md` step-4 contract.

## Snapshot at release time

`gh api repos/ForgePlan/forgeplan/dependabot/alerts --jq '[.[] | select(.state == "open")]'`

| # | Severity | Package | Ecosystem | GHSA | Range |
|---|----------|---------|-----------|------|-------|
| 33 | HIGH | `devalue` | npm | GHSA-77vg-94rm-hx3p | 5.6.3..=5.8.0 → 5.8.1 |
| 3  | LOW  | `lru`     | rust | GHSA-rhfx-m35p-ff5j | < 0.16.3 |

## Per-alert verdict

### #33 — `devalue` (HIGH) — **addressed**

GHSA-77vg-94rm-hx3p — Svelte `devalue` DoS via sparse-array deserialization.

**Action**: bumped `devalue` 5.6.4 → 5.8.1 via lockfile-only update on PR #302 (commit `82d4634`, merged 2026-05-21 04:43 UTC).

**Verification**:
```bash
cd website
npm ls devalue
# → @astrojs/react@5.0.2 → devalue@5.8.1
# → astro@6.1.10 → devalue@5.8.1
npm audit
# → 0 vulnerabilities
```

**Impact assessment** (why not BLOCKER for production binaries):
- `devalue` is a transitive of Astro (`@astrojs/react`, `astro`). Lives only in `website/` — the static documentation portal.
- The Rust CLI/MCP binaries do not include any JavaScript runtime — `devalue` is not in the production binary's dependency tree.
- The exploit surface is limited to: (a) build-time crash on malicious input (we control all build inputs), and (b) client-side crash on payloads we serialize (same — we control them).
- No exposed network endpoint deserializes untrusted user input via `devalue` in our deployment.

**Severity rating HIGH** reflects the upstream advisory; in our deployment context the practical impact is LOW. The bump is shipped regardless because the patched version is freely available and the lockfile bump is risk-free.

**Auto-close timeline**: GitHub Dependabot will auto-close alert #33 once it re-scans `main` (post-PR-#310 merge, post-PR-#311 sync). Typically within 1-4 hours of `main` HEAD movement.

### #3 — `lru` (LOW) — **accepted-with-justification**

GHSA-rhfx-m35p-ff5j — `IterMut` violates Stacked Borrows by invalidating internal pointer. Affects `lru < 0.16.3`.

**Action**: deferred to v0.33+, same posture as documented in v0.28.0 / v0.29.0 / v0.30.0 / v0.31.0 release notes.

**Transitive chain**:
```
forgeplan-core → lancedb 0.27.2 → lance 4.0.0 → tantivy 0.24.2 → lru 0.12.5
```

`cargo update -p lru` is a no-op: tantivy 0.24.2 pins `lru = "^0.12"`. Direct upgrade to 0.16.3 requires bumping tantivy major, which cascades into Lance / LanceDB upgrades — out of scope for a v0.32.x patch release.

**Impact in our context is LOW**:
- Stacked Borrows violations surface as undefined behaviour only under strict aliasing analysis (Miri / asan), not in normal optimized builds.
- The vulnerable code path (`IterMut` on LRU cache) is internal to tantivy's caching layer — never reached by forgeplan user input.
- No remote-trigger surface: forgeplan is local-first CLI/MCP, no untrusted-input network endpoint.

**Tracking**: tied to LanceDB upgrade work for v0.33 (when transitive can be bumped to a tantivy version that pins `lru = "^0.16"`).

## Cross-reference

- CHANGELOG.md v0.32.0 `### Security (RED-LINE #10 compliance)` section
- PR #302 commit `82d4634` — devalue bump
- PR #310 — v0.32.0 release
- `docs/operations/RELEASE-PROTOCOL.ru.md` § Common pitfalls → "Dependabot alerts on release time"
- Prior triages: `dependabot-triage-2026-05-02.md`, `-05-03.md`, `-05-05.md`
