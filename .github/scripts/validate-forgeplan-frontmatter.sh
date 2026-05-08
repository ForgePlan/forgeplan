#!/bin/bash
# validate-forgeplan-frontmatter.sh
#
# Validates Forgeplan artifact frontmatter contract per SPEC-005:
# - New artifacts (no assigned_number) MUST have slug + predicted_number
# - Rejects PRs that mutate existing assigned_number (write-once rule)
# - Rejects pre-set assigned_number on new artifacts (CRIT-2 defense)
#
# Usage: ./validate-forgeplan-frontmatter.sh [--check-only]
# Environment: BASE_REF (required for PR validation)
# Exit 0 if valid, 1 if errors found

set -euo pipefail

CHECK_ONLY="${1:-}"
ERRORS=0
WARNINGS=0

# Slug regex per SPEC-005 (includes mem for memory artifacts)
SLUG_REGEX="^(prd|rfc|adr|epic|spec|prob|sol|evid|note|ref|mem)-[a-z0-9]+(-[a-z0-9]+)*$"

# Helper: extract frontmatter field value from markdown file
# Returns the value or empty string if not found.
#
# CI Round 4 fix: `set -euo pipefail` + grep no-match exits 1 (no field present),
# pipefail propagates to subshell exit 1, which `set -e` then kills script
# в next iteration when caller does `field=$(extract_field ...)`. The legacy
# PROB-060 artifacts (ADR-012, PRD-076, RFC-009, SPEC-005, EVID-114, EVID-115,
# PROB-060, PROB-061) lack `slug:` field в first frontmatter block (двойной
# frontmatter from `forgeplan_new` template + manual edit) — they were created
# pre-Phase-1.5 schema enforcement. Validator silently aborted on first
# missing field.
#
# Fix: append `|| true` so grep no-match doesn't propagate pipefail.
extract_field() {
    local file="$1"
    local field="$2"

    # Extract YAML frontmatter (between first --- and second ---)
    # and grep for the field. `|| true` neutralizes grep no-match.
    { sed -n '/^---$/,/^---$/p' "$file" | \
        grep "^${field}:" | \
        head -1 | \
        sed "s/^${field}:[[:space:]]*//" | \
        sed 's/^"\(.*\)"$/\1/'; } || true
}

# Helper: check if a field exists and has a value
field_exists() {
    local file="$1"
    local field="$2"
    local value

    value=$(extract_field "$file" "$field")
    [[ -n "$value" ]]
}

# Helper: check if assigned_number has changed in a file
assigned_number_changed() {
    local file="$1"

    # Get the assigned_number from the current file
    local current
    current=$(extract_field "$file" "assigned_number")

    # CRIT-1 fix: use BASE_REF from CI environment, fail closed if missing
    local base_ref="${BASE_REF:-}"
    if [[ -z "$base_ref" ]]; then
        echo "❌ ERROR: BASE_REF environment variable not set (required for PR validation)"
        exit 1
    fi

    # Round 2 audit: file existence в base ref — not currently tracked in HEAD
    # (Round 1 used `git ls-files --error-unmatch` which is HEAD-tracking;
    # для write-once rule we need "exists in base ref" not "exists in HEAD").
    if ! git show "origin/${base_ref}:${file}" > /dev/null 2>&1 \
       && ! git show "${base_ref}:${file}" > /dev/null 2>&1; then
        return 1  # file is new in base — write-once rule does not apply
    fi

    # Round 3 audit CRIT-1 fix: bash `|` binds tighter than `||`, so the
    # original form `A || B | sed` parsed as `A || (B | sed)`. When A
    # (`git show "origin/<base>:<file>"`) succeeded — the typical CI case —
    # `previous` captured the entire raw markdown (frontmatter + body) instead
    # of the parsed assigned_number. Smoke test passed by coincidence (tamper
    # is non-equal either way), but legitimate `null → integer` bot stamps
    # were misclassified as write-once violations on every PR.
    #
    # Fix: capture raw output first (with explicit fallback grouping), then
    # pipe through extraction.
    local raw
    raw=$(git show "origin/${base_ref}:${file}" 2>/dev/null \
        || git show "${base_ref}:${file}" 2>/dev/null \
        || true)
    local previous
    previous=$(printf '%s\n' "$raw" \
        | sed -n '/^---$/,/^---$/p' \
        | grep "^assigned_number:" \
        | head -1 \
        | sed 's/^assigned_number:[[:space:]]*//' \
        | sed 's/^"\(.*\)"$/\1/')

    # Round 2 audit FINDING-2: normalize YAML null forms на обоих sides.
    [[ "$current" == "null" || "$current" == "~" ]] && current=""
    [[ "$previous" == "null" || "$previous" == "~" ]] && previous=""

    # Round 2 audit FINDING-4: assign-id workflow self-deadlock fix.
    # CI bot stamps `assigned_number: null → <integer>` and pushes back to PR.
    # Synchronize event re-runs validator. Without this special-case, validator
    # detects null→integer as write-once violation and fails the bot's commit.
    # Treat null→integer as the LEGITIMATE bot stamp (only flag <integer>→<other>
    # or <integer>→null as actual write-once violations).
    if [[ -z "$previous" && -n "$current" && "$current" =~ ^[0-9]+$ ]]; then
        return 1  # false — this is the CI bot's stamp, not a violation
    fi

    # Compare: if either differs (including empty vs non-empty), they've changed
    [[ "$current" != "$previous" ]]
}

# Validate a single artifact file
validate_artifact() {
    local file="$1"
    local basename
    basename=$(basename "$file")

    # Check if it's a new file (not yet committed)
    local is_new=false
    if ! git ls-files --error-unmatch "$file" > /dev/null 2>&1; then
        is_new=true
    fi

    # Extract frontmatter fields
    local slug
    local predicted_number
    local assigned_number

    slug=$(extract_field "$file" "slug")
    predicted_number=$(extract_field "$file" "predicted_number")
    assigned_number=$(extract_field "$file" "assigned_number")

    # Rule 1: New artifacts MUST have slug and predicted_number
    if [[ "$is_new" == "true" ]]; then
        if [[ -z "$slug" ]]; then
            echo "❌ ERROR: New artifact '$basename' missing required field: slug"
            ERRORS=$((ERRORS + 1))
        elif ! [[ "$slug" =~ $SLUG_REGEX ]]; then
            echo "❌ ERROR: New artifact '$basename' has invalid slug format: '$slug'"
            echo "   Regex: $SLUG_REGEX"
            ERRORS=$((ERRORS + 1))
        fi

        if [[ -z "$predicted_number" ]]; then
            echo "❌ ERROR: New artifact '$basename' missing required field: predicted_number"
            ERRORS=$((ERRORS + 1))
        elif ! [[ "$predicted_number" =~ ^[0-9]+$ ]] || [[ "$predicted_number" -lt 1 ]]; then
            echo "❌ ERROR: New artifact '$basename' has invalid predicted_number: '$predicted_number' (must be positive integer)"
            ERRORS=$((ERRORS + 1))
        fi

        # CRIT-2 Layer A: Reject pre-set assigned_number on new artifacts.
        # Round 2 audit FINDING-2: bash extract_field returns literal "null" for
        # YAML scalar null. Treat "null", "~", and empty as YAML-null equivalent
        # (matches Rust serde_yaml semantics).
        if [[ -n "$assigned_number" && "$assigned_number" != "null" && "$assigned_number" != "~" ]]; then
            echo "❌ ERROR: New artifact '$basename' has pre-set assigned_number: '$assigned_number' (invariant I-2 violation)"
            echo "   assigned_number must be null or absent in new artifacts — only CI bot may assign it after merge"
            ERRORS=$((ERRORS + 1))
        fi
    fi

    # Rule 2: Reject PRs that mutate existing assigned_number (write-once rule)
    if assigned_number_changed "$file"; then
        echo "❌ ERROR: Artifact '$basename' attempts to modify assigned_number (write-once rule violation)"
        echo "   assigned_number can only be set by CI bot, not manually"
        ERRORS=$((ERRORS + 1))
    fi

    # Rule 3: If slug exists, validate format
    if [[ -n "$slug" ]]; then
        if ! [[ "$slug" =~ $SLUG_REGEX ]]; then
            echo "⚠️  WARNING: Artifact '$basename' has invalid slug format: '$slug'"
            WARNINGS=$((WARNINGS + 1))
        fi
    fi
}

# Main validation logic
main() {
    echo "🔍 Validating Forgeplan artifact frontmatter..."

    # Find all artifact files that were added or modified in this PR
    local artifact_files=()

    # Round 2 audit FINDING-1: actions/checkout@v4 leaves clean working tree
    # — `git diff --cached` returns empty on every CI run, so the validator
    # discovered ZERO files in production. Use BASE_REF-aware diff instead;
    # fail closed if BASE_REF is missing (script entry guards against this).
    local base_ref="${BASE_REF:-}"
    if [[ -z "$base_ref" ]]; then
        echo "❌ ERROR: BASE_REF environment variable not set (required for file discovery)"
        exit 1
    fi

    # Validate BASE_REF shape (Round 2 FINDING-9 — CWE-78 defense): branch
    # name must be safe для interpolation в `git show "origin/${BASE_REF}:..."`.
    if ! [[ "$base_ref" =~ ^[A-Za-z0-9._/-]+$ ]]; then
        echo "❌ ERROR: BASE_REF '$base_ref' contains unsafe characters"
        exit 1
    fi

    while IFS= read -r file; do
        # Check if file is in a .forgeplan artifact directory
        if [[ "$file" =~ ^\.forgeplan/(prds|rfcs|adrs|epics|specs|evidence|problems|solutions|refresh|notes|memory)/.*\.md$ ]]; then
            artifact_files+=("$file")
        fi
    done < <(git diff --name-only "origin/${base_ref}...HEAD" 2>/dev/null)

    if [[ ${#artifact_files[@]} -eq 0 ]]; then
        echo "ℹ️  No Forgeplan artifacts to validate"
        return 0
    fi

    echo "📋 Found ${#artifact_files[@]} artifact file(s) to validate:"
    printf '   %s\n' "${artifact_files[@]}"
    echo ""

    for file in "${artifact_files[@]}"; do
        if [[ -f "$file" ]]; then
            validate_artifact "$file"
        fi
    done

    echo ""
    echo "📊 Summary:"
    echo "   Errors: $ERRORS"
    echo "   Warnings: $WARNINGS"

    if [[ $ERRORS -gt 0 ]]; then
        echo ""
        echo "❌ Validation FAILED"
        return 1
    else
        echo ""
        echo "✅ Validation PASSED"
        return 0
    fi
}

main "$@"
