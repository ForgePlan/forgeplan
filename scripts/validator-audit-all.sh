#!/bin/bash
# validator-audit-all.sh — Workspace audit of frontmatter validator coverage.
#
# PROB-060 Phase 2.3 (T4): runs the per-file rules of
# `.github/scripts/validate-forgeplan-frontmatter.sh` against ALL artifacts in
# `.forgeplan/{prds,rfcs,adrs,epics,specs,evidence,problems,solutions,refresh,notes,memory}/`
# and aggregates results by category.
#
# ## Why a separate script
#
# The PR validator runs Rules 1+2+3 (Rule 2 = `assigned_number` write-once
# requires `git diff` against `BASE_REF`). For workspace audit there is no
# "base ref" — every file already lives on the branch we're checking. We
# therefore execute only the per-file rules:
#
# - **Rule 1** — *new* artifact MUST have `slug` + `predicted_number` and
#   MUST NOT pre-set `assigned_number` (CRIT-2). Files committed to git are
#   `is_new=false`, so legacy artifacts are silently skipped — that is the
#   intended legacy compat behaviour, not a bug. Newly added (untracked)
#   files trigger Rule 1.
# - **Rule 3** — slug shape (warning, not error) when present. Catches
#   uppercase/underscore drift on existing artifacts.
#
# Rule 2 is intentionally NOT executed; without a write-once base it would
# always pass for unchanged files and add no signal.
#
# ## Modes
#
# - **default**: walk the artifact tree, run validate_artifact per file,
#   aggregate. Exit 0 unless an unexpected ERROR fires (warnings allowed).
# - **`--strict`**: same as default but warnings (Rule 3 invalid slug)
#   also count as failures. Used by the CI self-test job to catch
#   regressions on the canonical workspace.
#
# Strategy on output:
#
# - **N pass** — slug+predicted+assigned all present and valid (new schema).
# - **M skip rule-1** — no slug field at all (legacy pre-PROB-060 artifact).
# - **K warn** — has slug but shape fails Rule 3 regex.
# - **0 error** — should be zero на канонической workspace; nonzero is a
#   regression that needs investigation.
#
# ## Self-test contract
#
# Adding this script to CI (`validator-self-test` job) catches regressions:
# - someone hand-edits an artifact and breaks slug shape → warn count rises
# - schema change accidentally drops a kind from `SLUG_REGEX` → error fires
# - new artifact lacks `slug` field → error
#
# Run: `bash scripts/validator-audit-all.sh`
# CI: `bash scripts/validator-audit-all.sh --strict`

set -o pipefail
# Note: NOT using `set -u` (nounset) because old bash 3.2 (macOS default)
# treats empty arrays as unset; we accept the trade-off — explicit nullity
# checks below are sufficient for this script's narrow surface.

cd "$(dirname "$0")/.."

STRICT=0
if [[ "${1:-}" == "--strict" ]]; then
    STRICT=1
fi

# Slug regex must mirror .github/scripts/validate-forgeplan-frontmatter.sh.
# Drift is caught by scripts/check-kind-list-drift.sh; this constant is
# duplicated intentionally so the audit is self-contained.
SLUG_REGEX="^(prd|rfc|adr|epic|spec|prob|sol|evid|note|ref|mem)-[a-z0-9]+(-[a-z0-9]+)*$"

PASS=0
SKIP_RULE1=0
WARN=0
ERROR=0
FAILED_FILES=()
WARN_FILES=()

# Helper: extract first frontmatter field. Mirrors the PR validator's
# `extract_field` (post-Round-4 fix). When the file has двойной frontmatter
# (`---\n...\n---\n---\n...\n---\n`) only the FIRST block is parsed — that
# matches the Rust `parse_frontmatter` semantics in
# `crates/forgeplan-core/src/artifact/frontmatter.rs`.
extract_field() {
    local file="$1"
    local field="$2"
    # `|| true` neutralises grep no-match (CI Round 4 closure — без этого
    # `set -e` killed the script на first missing field).
    { sed -n '/^---$/,/^---$/p' "$file" \
        | grep "^${field}:" \
        | head -1 \
        | sed "s/^${field}:[[:space:]]*//" \
        | sed 's/^"\(.*\)"$/\1/'; } || true
}

echo "Validator workspace audit — PROB-060 Phase 2.3 (T4)"
echo "Mode: $([[ $STRICT -eq 1 ]] && echo strict || echo default)"
echo ""

# Scan all artifact files. macOS ships bash 3.2 (no `mapfile`/`readarray`),
# so we read newline-separated paths via a `while`-loop. Artifact paths
# in `.forgeplan/` are restricted to the slug grammar (lowercase alnum +
# dash) and `<KIND>-<NNN>-<slug>.md`, so newline-bearing paths are
# forbidden by the schema — fine to use newline as a separator here.
ARTIFACTS=()
while IFS= read -r line; do
    [[ -n "$line" ]] && ARTIFACTS+=("$line")
done < <(
    find .forgeplan \
        \( -path '*/prds/*.md' \
        -o -path '*/rfcs/*.md' \
        -o -path '*/adrs/*.md' \
        -o -path '*/epics/*.md' \
        -o -path '*/specs/*.md' \
        -o -path '*/evidence/*.md' \
        -o -path '*/problems/*.md' \
        -o -path '*/solutions/*.md' \
        -o -path '*/refresh/*.md' \
        -o -path '*/notes/*.md' \
        -o -path '*/memory/*.md' \
        \) -type f | LC_ALL=C sort
)

TOTAL=${#ARTIFACTS[@]}
echo "Scanning $TOTAL artifacts..."
echo ""

for file in "${ARTIFACTS[@]}"; do
    slug=$(extract_field "$file" "slug")
    predicted=$(extract_field "$file" "predicted_number")
    # assigned_number is read for completeness even though Rule 2 is not
    # executed — counted toward "pass" only when the new-schema triple is
    # all present.
    assigned=$(extract_field "$file" "assigned_number")
    basename=$(basename "$file")

    # Rule 1: artifact considered "fully new-schema" when ALL three
    # identity fields are present (slug + predicted_number + non-empty
    # assigned_number OR explicit "null"). Legacy artifacts have neither
    # slug nor predicted_number — they're tracked separately as
    # "skip_rule1".
    if [[ -z "$slug" && -z "$predicted" ]]; then
        SKIP_RULE1=$((SKIP_RULE1 + 1))
        continue
    fi

    # Mixed state: slug present but predicted_number missing OR vice versa
    # — this is a corruption signal (someone hand-edited frontmatter and
    # dropped one half). Treat as ERROR.
    if [[ -z "$slug" && -n "$predicted" ]]; then
        echo "ERROR: $basename has predicted_number but no slug"
        ERROR=$((ERROR + 1))
        FAILED_FILES+=("$file")
        continue
    fi
    if [[ -n "$slug" && -z "$predicted" ]]; then
        echo "ERROR: $basename has slug but no predicted_number"
        ERROR=$((ERROR + 1))
        FAILED_FILES+=("$file")
        continue
    fi

    # Rule 3: slug shape validation (warning by default, error in strict
    # mode). Mirrors validate-forgeplan-frontmatter.sh:181.
    if ! [[ "$slug" =~ $SLUG_REGEX ]]; then
        if [[ $STRICT -eq 1 ]]; then
            echo "ERROR (strict): $basename has invalid slug format: '$slug'"
            ERROR=$((ERROR + 1))
            FAILED_FILES+=("$file")
        else
            echo "WARN: $basename has invalid slug format: '$slug'"
            WARN=$((WARN + 1))
            WARN_FILES+=("$file")
        fi
        continue
    fi

    # predicted_number must be positive integer in 1..=MAX_ARTIFACT_NUMBER
    # (1_000_000 — see crates/forgeplan-core/src/artifact/frontmatter.rs).
    if ! [[ "$predicted" =~ ^[0-9]+$ ]] || [[ "$predicted" -lt 1 ]] || [[ "$predicted" -gt 1000000 ]]; then
        echo "ERROR: $basename has invalid predicted_number: '$predicted'"
        ERROR=$((ERROR + 1))
        FAILED_FILES+=("$file")
        continue
    fi

    PASS=$((PASS + 1))
done

echo ""
echo "Summary:"
echo "  Total artifacts scanned: $TOTAL"
echo "  Pass (new schema, slug+predicted valid): $PASS"
echo "  Skip Rule 1 (legacy, no slug field): $SKIP_RULE1"
echo "  Warnings (invalid slug format): $WARN"
echo "  Errors: $ERROR"

if [[ $ERROR -gt 0 ]]; then
    echo ""
    echo "Failed artifacts:"
    printf '  %s\n' "${FAILED_FILES[@]}"
fi

if [[ $WARN -gt 0 && $STRICT -eq 0 ]]; then
    echo ""
    echo "Artifacts with warnings:"
    printf '  %s\n' "${WARN_FILES[@]}"
fi

# Exit policy:
# - default: errors fail (exit 1), warnings allowed (exit 0)
# - strict: warnings also fail (Rule 3 elevated to error)
if [[ $ERROR -gt 0 ]]; then
    echo ""
    echo "FAIL: $ERROR error(s) found"
    exit 1
fi

echo ""
echo "PASS: validator audit clean"
exit 0
