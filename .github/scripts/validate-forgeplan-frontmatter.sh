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

# Slug regex per SPEC-005
SLUG_REGEX="^(prd|rfc|adr|epic|spec|prob|sol|evid|note|ref)-[a-z0-9]+(-[a-z0-9]+)*$"

# Artifact kinds and their directories
ARTIFACT_KINDS=("prds" "rfcs" "adrs" "epics" "specs" "evidence" "problems" "solutions" "refresh" "notes" "memory")

# Helper: extract frontmatter field value from markdown file
# Returns the value or empty string if not found
extract_field() {
    local file="$1"
    local field="$2"

    # Extract YAML frontmatter (between first --- and second ---)
    # and grep for the field
    sed -n '/^---$/,/^---$/p' "$file" | \
        grep "^${field}:" | \
        head -1 | \
        sed "s/^${field}:[[:space:]]*//" | \
        sed 's/^"\(.*\)"$/\1/'
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

    # If not in git yet (new file), it hasn't changed
    if ! git ls-files --error-unmatch "$file" > /dev/null 2>&1; then
        return 1  # false - file is new
    fi

    # CRIT-1 fix: use BASE_REF from CI environment, fail closed if missing
    local base_ref="${BASE_REF:-}"
    if [[ -z "$base_ref" ]]; then
        echo "❌ ERROR: BASE_REF environment variable not set (required for PR validation)"
        exit 1
    fi

    # Get the assigned_number from the base ref version
    local previous
    previous=$(git show "origin/${base_ref}:${file}" 2>/dev/null | \
        sed -n '/^---$/,/^---$/p' | \
        grep "^assigned_number:" | \
        head -1 | \
        sed 's/^assigned_number:[[:space:]]*//' | \
        sed 's/^"\(.*\)"$/\1/' || true)

    # If either is empty, they differ
    if [[ -z "$current" ]] || [[ -z "$previous" ]]; then
        [[ "$current" != "$previous" ]]
        return $?
    fi

    # Compare non-empty values
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

        # CRIT-2 Layer A: Reject pre-set assigned_number on new artifacts
        if [[ -n "$assigned_number" ]]; then
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

    # Get changed files from git diff
    while IFS= read -r file; do
        # Check if file is in a .forgeplan artifact directory
        if [[ "$file" =~ ^\.forgeplan/(prds|rfcs|adrs|epics|specs|evidence|problems|solutions|refresh|notes|memory)/.*\.md$ ]]; then
            artifact_files+=("$file")
        fi
    done < <(git diff --name-only --cached || git diff --name-only HEAD...origin/main 2>/dev/null || true)

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
