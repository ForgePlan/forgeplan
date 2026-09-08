# Dependabot triage — 2026-09-08 (v0.37.0 release window)

Per RED-LINE #10 (CLAUDE.md): each release tags every open Dependabot alert as
**addressed** / **scheduled** / **accepted-with-justification**. Follows the
`docs/operations/RELEASE-PROTOCOL.md` step-4 contract.

## Snapshot at release time

```bash
gh api repos/ForgePlan/forgeplan/dependabot/alerts --paginate \
  -q '[.[] | select(.state=="open")] | length'
```

**33 open: 8 HIGH / 15 MEDIUM / 10 LOW — 1 Rust, 32 npm.**

The split is the same shape as every prior release triage: the one Rust alert sits
in the dependency tree of the shipped `forgeplan` binary; all 32 npm alerts are
confined to `website/` — a static Astro documentation site that ships no server and
is not part of any released artifact.

## Rust — the shipped binary

| Package | Sev | GHSA | Fix in | Verdict |
|---|---|---|---|---|
| `lru` | LOW | GHSA-rhfx-m35p-ff5j | 0.16.3 | **accepted-with-justification** |

### `lru` LOW — accepted-with-justification

`IterMut` violates Stacked Borrows by invalidating an internal pointer. **Cannot be
updated without an upstream change**: the only consumer is `tantivy 0.24.2` (confirmed
again in `Cargo.lock` — `lru 0.12.5`, no direct `forgeplan-core`/`forgeplan-cli`/
`forgeplan-mcp` dependency on the crate), which pins `lru 0.12.x`, while the fix
landed in `0.16.3` — a major bump only `tantivy` can take. Forgeplan never
constructs an `lru` cache itself and never calls `IterMut`; the advisory describes
undefined behaviour observable under Miri, not a reachable exploit in this
dependency path. **Carried forward** — same verdict as v0.33.0, v0.34.0, v0.35.0.
Re-evaluate when `tantivy` bumps its `lru` bound.

## RustSec — not a Dependabot alert, checked separately

GitHub's Dependabot feed does not mirror RustSec, and this gap has bitten this
project twice before this window (v0.34.0: crossbeam-epoch red on `dev` for 11
days; v0.35.0: h2 red for three consecutive merges). Both times the miss was
"Dependabot showed nothing, so nobody looked at `cargo-deny` directly."

Checked directly, not inferred from Dependabot's silence:

```bash
gh run list --branch dev --limit 5 --json name,conclusion,createdAt \
  -q '.[] | select(.name == "security")'
```

`security` (the `cargo-deny` workflow) is **green on `dev`** — last run
2026-09-08T17:18:26Z, immediately after the rust-deps (#473) and github-actions
(#459) Dependabot PRs merged, `success`. Stated explicitly here rather than left
to be assumed from an empty Dependabot list, per the lesson the two prior misses
left behind.

## npm — `website/` only

| Package | Sev | Count |
|---|---|---|
| `browserslist` | HIGH | 2 |
| `astro` | HIGH | 2 |
| `vite` | HIGH | 1 |
| `sharp` | HIGH | 1 |
| `nanoid` | HIGH | 1 |
| `js-yaml` | HIGH | 1 |
| `dompurify` | MEDIUM | 6 |
| `mermaid` | MEDIUM | 4 |
| `astro` | MEDIUM | 3 |
| `vite` | MEDIUM | 1 |
| `@astrojs/rss` | MEDIUM | 1 |
| `dompurify` | LOW | 4 |
| `postcss-selector-parser` | LOW | 1 |
| `mermaid` | LOW | 1 |
| `esbuild` | LOW | 1 |
| `astro` | LOW | 1 |
| `@babel/core` | LOW | 1 |

**Verdict: scheduled** — same reasoning as every prior triage, restated because it
still holds and because one open item makes it concrete this time:

1. **Zero exposure through the released product.** Build-time and render-time
   dependencies of a statically generated documentation site; the `forgeplan`
   binary, MCP server, and marketplace plugins carry none of them.
2. **A blanket update is a known-bad move here** — established by PR #401 (a
   sweeping `npm update` broke the build on a peer-major conflict). This
   release's own scoping work found the concrete case: **#442** (`npm-website`
   Dependabot group, opened 2026-08-17) carries `astro` 6→7 and `@astrojs/mdx`
   5→7 inside it, and predates the `Website build` CI gate — its green
   checkmarks don't include the one check that would actually exercise a
   two-major-version jump. Filed as its own issue (#485) with the concrete
   next step (rebase so the gate runs, verify with a real `npm run build`)
   rather than merged on old green.
3. **Bundling it here would make the release un-reviewable.** v0.37.0 already
   carries a breaking scoring-semantics change (PRD-086) plus a lifecycle
   data-loss fix (#478) plus a CI-coverage fix (PROB-102). A front-end
   dependency sweep on top of that would make bisecting any regression
   materially harder.

**Trigger for the scheduled work:** #485 (already filed) — rebase #442 so the
`Website build` gate actually runs against it, then apply by hand with a
verified `npm run build`, not as a merged auto-group.

## Verification

```bash
cargo deny check advisories                       # → advisories ok
grep -A1 'name = "lru"' Cargo.lock                # → 0.12.5 (tantivy-pinned)
gh api repos/ForgePlan/forgeplan/dependabot/alerts --paginate \
  -q '[.[] | select(.state=="open")] | length'    # → 33
```
