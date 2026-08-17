# Dependabot triage — 2026-08-17 (v0.34.0 release window)

Per RED-LINE #10 (CLAUDE.md): each release tags every open Dependabot alert as
**addressed** / **scheduled** / **accepted-with-justification**. Follows the
`docs/operations/RELEASE-PROTOCOL.md` step-4 contract.

## Snapshot at release time

```bash
gh api "repos/ForgePlan/forgeplan/dependabot/alerts?state=open&per_page=100" --paginate
```

**42 open: 12 HIGH / 20 MEDIUM / 10 LOW — 3 Rust, 39 npm.**

The split matters more than the count. The 3 Rust alerts sit in the dependency tree of
the shipped `forgeplan` binary. All 39 npm alerts are confined to `website/` (35 in
`website/package-lock.json`, 4 in `website/package.json`) — the Astro documentation
portal, which is a **static site**: it ships no server, is not part of any released
artifact, and no user of the CLI or MCP server executes this code.

## Rust — the shipped binary

| Package | Sev | GHSA | Fix in | Verdict |
|---|---|---|---|---|
| `quinn-proto` | HIGH | GHSA-4w2j-m93h-cj5j | 0.11.15 | **addressed** |
| `serde_with` | MEDIUM | GHSA-7gcf-g7xr-8hxj | 3.21.0 | **addressed** |
| `lru` | LOW | GHSA-rhfx-m35p-ff5j | 0.16.3 | **accepted-with-justification** |

### `quinn-proto` HIGH — addressed

Remote memory exhaustion via unbounded out-of-order stream reassembly. Transitive through
`reqwest → quinn`. Lockfile-only bump **0.11.14 → 0.11.16** (semver-compatible, ≥ the
0.11.15 fix) in this release.

### `serde_with` MEDIUM — addressed

`KeyValueMap` serialization panics on an empty sequence. Transitive through `lancedb`.
Lockfile-only bump **3.18.0 → 3.22.0** (≥ the 3.21.0 fix) in this release.

### `lru` LOW — accepted-with-justification

`IterMut` violates Stacked Borrows by invalidating an internal pointer. **Cannot be
updated without an upstream change**: the only consumer is `tantivy 0.24.2`, which pins
`lru 0.12.x`, while the fix landed in `0.16.3` — a major bump that only `tantivy` can
take. Forgeplan never constructs an `lru` cache itself and never calls `IterMut`; the
advisory describes undefined behaviour observable under Miri, not a reachable exploit in
this dependency path. **Carried forward** (same verdict as v0.33.0). Re-evaluate when
`tantivy` bumps its `lru` bound.

## Not a Dependabot alert, but a release blocker — RUSTSEC-2026-0204

GitHub's Dependabot feed does **not** mirror RustSec, so a Dependabot-only triage would
have missed this one entirely. `cargo-deny` (the `security` workflow) had been failing on
`dev` since 2026-08-06 across five consecutive runs:

```
error[vulnerability]: Invalid pointer dereference in `fmt::Pointer` impl for `Atomic` and
`Shared` when the underlying pointer is invalid
  crossbeam-epoch 0.9.18 — RUSTSEC-2026-0204
  Solution: Upgrade to >=0.9.20
```

Reaches the tree twice transitively (`tera → globwalk → ignore → crossbeam-deque` and
`fastembed → image → exr → rayon-core → crossbeam-deque`). **Addressed**: lockfile-only
bump **0.9.18 → 0.9.20** in this release; `cargo deny check advisories` now reports
`advisories ok`.

**Lesson for future releases:** the Dependabot alert list is not a sufficient security
gate for a Rust project. `cargo-deny` is a separate, non-optional gate, and a red
`security` workflow on `dev` violates the RELEASE-PROTOCOL pre-condition "`dev` is green"
even though nothing in the Dependabot UI shows a problem.

## npm — `website/` only

| Package | Sev | Alerts |
|---|---|---|
| `astro` | HIGH | 9 |
| `js-yaml` | HIGH | 3 |
| `nanoid` | HIGH | 2 |
| `postcss` | HIGH | 2 |
| `sharp` | HIGH | 2 |
| `vite` | HIGH | 2 |
| `svgo` | HIGH | 1 |
| `dompurify` | MEDIUM | 10 |
| `mermaid` | MEDIUM | 5 |
| `@astrojs/rss` | MEDIUM | 1 |
| `@babel/core`, `esbuild` | LOW | 2 |

**Verdict: scheduled** — all of them, to a dedicated `fix/website-npm-security-*` PR
outside the release window.

Justification, and why this is not deferral-by-convenience:

1. **Zero exposure through the released product.** These packages are build-time and
   render-time dependencies of a statically generated documentation site. The `forgeplan`
   binary, the MCP server, and the marketplace plugins do not contain, download or execute
   any of them. The classes involved (ReDoS/quadratic CPU in `js-yaml`, path traversal in
   `postcss` source-map loading, `vite` dev-server `fs.deny` bypass **on Windows**, Astro
   dev/SSR paths) require either a build machine or a running dev server — neither exists
   for a site published as static HTML.
2. **A blanket update is a known-bad move here.** `npm update` in `website/` breaks the
   build on a peer-major conflict (`@tailwindcss/vite` wants vite 8, `astro` wants
   vite 7) — established in PR #401, which is why this repo's convention is *named*
   updates plus `npm run build` before commit. The `astro`/`vite` majors in this list
   need exactly that treatment, with a full site build and a visual check — which is a
   release of its own, not a line item in a scoring-engine release.
3. **Bundling it here would make the release un-reviewable.** This release already carries
   a breaking scoring-semantics change. Mixing in a 39-alert front-end dependency sweep
   would make bisecting any regression materially harder.

**Trigger for the scheduled work:** the next website-touching PR, or sooner if any alert
gains a `sharp`-style native-code RCE (the two `sharp` HIGHs are inherited libvips CVEs —
worth re-checking whether the site's image pipeline even reaches the affected codecs).

## Verification

```bash
cargo deny check advisories        # → advisories ok
grep -A1 'name = "crossbeam-epoch"' Cargo.lock   # → 0.9.20
grep -A1 'name = "quinn-proto"'     Cargo.lock   # → 0.11.16
grep -A1 'name = "serde_with"'      Cargo.lock   # → 3.22.0
```
