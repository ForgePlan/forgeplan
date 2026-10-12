# Dependabot triage — 2026-10-12 (v0.38.0 release window)

Per RED LINE #10 in `CLAUDE.md`, every open alert is classified as
**addressed**, **scheduled**, or **accepted-with-justification**. RustSec is
checked separately because it is not reliably mirrored into Dependabot.

## Snapshot at release preparation

```bash
gh api -X GET \
  'repos/ForgePlan/forgeplan/dependabot/alerts?state=open&per_page=100' \
  --paginate --slurp
```

Snapshot SHA-256:
`62d2c03352dc40746f095a45b23e68b843210795ff0af74d8713173ef9c9d333`.

**56 open: 2 CRITICAL / 19 HIGH / 24 MEDIUM / 11 LOW.** The split is
46 npm alerts and 10 Rust alerts.

| Ecosystem | Critical | High | Medium | Low | Total |
|---|---:|---:|---:|---:|---:|
| npm (`website/`) | 2 | 15 | 20 | 9 | 46 |
| Rust | 0 | 4 | 4 | 2 | 10 |
| **Total** | **2** | **19** | **24** | **11** | **56** |

## Addressed in the release integration chain

| Feed | Package | Severity | Before | Fixed | Verdict |
|---|---|---|---|---|---|
| RustSec | `rustls` | advisory gate failure | 0.23.40 | 0.23.45 | **addressed** |
| Dependabot | `xxhash-rust` | LOW | 0.8.15 | 0.8.16 | **addressed** |

`rustls` is RUSTSEC-2026-0285: TLS 1.3 handshake messages could be accepted at
an invalid encryption level. It was not present in the Dependabot snapshot, but
`cargo deny check advisories` failed on current `dev`. PR #519 updates `rustls`
and its `rustls-webpki` companion, updates `xxhash-rust`, and restores the
release security precondition. The changes are lockfile-only and
semver-compatible.

## Rust alerts retained after the lockfile fixes

| Package | High | Medium | Low | Verdict | Tracker / trigger |
|---|---:|---:|---:|---|---|
| `rmcp` 1.7.0 | 4 | 4 | 0 | **scheduled** | #515; migrate to 2.1+ before enabling OAuth or Streamable HTTP transports, or for v0.39.0, whichever comes first |
| `lru` 0.12.5 | 0 | 0 | 1 | **accepted-with-justification** | Re-evaluate when `tantivy` permits `lru >= 0.16.3` |

### `rmcp` — scheduled

The eight alerts cover OAuth-client and Streamable HTTP server paths. The
released ForgePlan MCP server enables only `server` + `transport-io` and serves
stdio; neither affected transport is enabled or exposed. This makes the
advisories unreachable in v0.38.0, not harmless in the dependency graph.

The first line covering all fixes is `rmcp 2.1+`, which is an API migration,
not a release-window lockfile update. Issue #515 owns that migration and names
the hard trigger: complete it before either affected transport is enabled.

### `lru` — accepted-with-justification

`lru 0.12.5` is transitive through `tantivy 0.24.2`, which pins the 0.12 line;
the fix is 0.16.3. ForgePlan does not construct this cache or call the affected
`IterMut` API. The release cannot update it independently. This carries forward
the v0.37.0 verdict with the same measurable trigger: re-evaluate when
`tantivy` widens or bumps its `lru` dependency.

## npm alerts — scheduled as one verified website migration

All 46 npm alerts are confined to `website/`, an Astro static site. They do not
ship in the ForgePlan CLI, MCP server, Homebrew bottle, or cargo-dist archives.
The site build consumes repository-controlled content; there is no production
Astro server or untrusted image-upload path. The alerts still affect the build
supply chain and are scheduled rather than accepted.

| Package | Count | Severities | Verdict |
|---|---:|---|---|
| `astro` | 10 | CRITICAL, HIGH, MEDIUM, LOW | **scheduled** |
| `dompurify` | 10 | MEDIUM, LOW | **scheduled** |
| `mermaid` | 5 | MEDIUM, LOW | **scheduled** |
| `devalue` | 3 | HIGH, MEDIUM | **scheduled** |
| `sharp` | 3 | HIGH | **scheduled** |
| `browserslist` | 2 | HIGH | **scheduled** |
| `js-yaml` | 2 | HIGH | **scheduled** |
| `svgo` | 2 | HIGH, MEDIUM | **scheduled** |
| `vite` | 2 | HIGH, MEDIUM | **scheduled** |
| `@astrojs/rss` | 1 | MEDIUM | **scheduled** |
| `@babel/core` | 1 | LOW | **scheduled** |
| `baseline-browser-mapping` | 1 | MEDIUM | **scheduled** |
| `esbuild` | 1 | LOW | **scheduled** |
| `nanoid` | 1 | HIGH | **scheduled** |
| `postcss-selector-parser` | 1 | LOW | **scheduled** |
| `smol-toml` | 1 | HIGH | **scheduled** |

PR #442 groups 16 website updates, including Astro 6 → 7 and multiple
integration majors. Its original green checks predate the website build gate;
merging it as-is would treat an untested major migration as a patch. Issue #485
owns the safe path: rebase, resolve the dependency set deliberately, run the
current website CI, and verify the rendered site in a browser.

**Trigger:** complete #485 before deploying server-side Astro, accepting
untrusted build inputs, or cutting v0.39.0 — whichever happens first.

## Verification contract

The release branch must show all of the following before merge:

```bash
cargo deny check advisories
cargo check --workspace
npm run build  # from website/
```

GitHub must also show the `security` workflow green on the release PR. A green
Dependabot count alone is insufficient: RUSTSEC-2026-0285 was absent from the
56-alert Dependabot snapshot and still failed `cargo-deny` on `dev`.
