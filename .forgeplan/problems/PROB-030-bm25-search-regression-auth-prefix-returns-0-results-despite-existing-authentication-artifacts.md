---
depth: tactical
id: PROB-030
kind: problem
status: active
title: BM25 search regression — 'auth' prefix returns 0 results despite existing 'Authentication' artifacts
---

# PROB-030: BM25 prefix search regression

## Signal

On a test workspace with 9 PRDs including 2 titled "Authentication":

```bash
$ forgeplan search "auth"
  No results for "auth"

$ forgeplan search "OAu"
  No results for "OAu"

$ forgeplan search "OAuth2"    # exact token works
Found 2 result(s) for "OAuth2" (smart search):
  0.43  PRD-002 [prd|draft] "Authentication OAuth2 system"
  0.43  PRD-001 [prd|draft] "Authentication system redesign with OAuth2 1"
```

BM25 tokenization splits titles into whole tokens ("authentication",
"oauth2"), so prefix queries like "auth" or "OAu" don't match because
they are not complete tokens.

**This is a REGRESSION** from v0.16.0 and earlier substring-based
search, which matched any substring — users could type "auth" and
find "Authentication". Sprint 13.2 PRD-039 replaced substring with
BM25 ranking for better precision on full-word queries but lost
prefix matching as a side effect.

## Repro

```bash
cd $(mktemp -d)
forgeplan init -y
forgeplan new prd "Authentication OAuth2 system"
forgeplan new prd "Authentication system redesign"
forgeplan search "auth"
# Expected: both PRDs returned (users expect grep-like partial match)
# Actual:   "No results for auth"
```

## Root cause hypothesis

Sprint 13.2 BM25 implementation in `forgeplan-core/src/search/`
tokenizes both the query and the document text into whole words,
then computes term frequency / inverse document frequency. Whole-word
tokens "auth" and "authentication" are different tokens — BM25 sees
no match.

Possible fixes (not chosen yet, for ADI phase):

1. **Fallback substring on 0 BM25 results** — if BM25 returns nothing,
   fall back to old substring matching with a lower score ceiling.
   Preserves BM25 ranking when it finds matches + restores grep
   behavior when users type prefixes.

2. **Prefix/stem tokenizer** — expand query tokens to prefix-matches
   against the index. Adds `auth*` → `authentication|authorized|
   authority` at query time. More invasive.

3. **Hybrid: substring + BM25** — always check both, merge results.
   Most expensive.

4. **Document explicit "exact tokens only" behavior** — not a fix,
   just tell users they need full words. Regression remains.

Option 1 is recommended for scope of hotfix.

## Constraints

- Must preserve BM25 ranking quality when full tokens match (don't
  drop to substring mode unnecessarily)
- Must not slow down searches significantly
- Substring fallback should have clearly lower score so users can
  see "this is a fuzzy match, not exact"
- Backward compat: existing tests passing BM25 full-token queries
  must still pass

## Acceptance Criteria

1. `search "auth"` with existing "Authentication" artifacts returns
   those artifacts (possibly at a lower rank than exact-token matches)
2. `search "authentication"` still returns same results as today
   (full-token BM25 path unchanged)
3. Output includes an indicator when results came from fallback
   path (e.g., "fuzzy match" tag or visibly lower score)
4. New integration test covers prefix queries
5. Existing BM25 tests pass unchanged

## Impact

**HIGH** — user-facing search regression. Users switching from
v0.16 to v0.17 will hit this on their first prefix search and get
zero results. First impression quality matters.

## Blast Radius

- All CLI `forgeplan search` users
- MCP `forgeplan_search` tool consumers (AI agents relying on
  substring)
- Sprint 13.2 PRD-039 FR-001 acceptance implicit assumption

## Reversibility

HIGH — additive fallback, no schema change, feature-flag not
required for patch.

## Related

| Artifact | Relation |
|---|---|
| PRD-039 | informs (Sprint 13.2 BM25 implementation that introduced this) |
| EVID-065 | informs (Sprint 13.2 backfill evidence — did not catch this regression) |
| EPIC-003 | context |
| PROB-031 | sibling (quality audit 2026-04-09 found both) |
| NOTE-048 | sibling (EPIC-003 verification gaps list) |

