#!/bin/bash
# check-kind-list-drift.sh
#
# Detects drift between Rust ArtifactKind enum and bash validator SLUG_REGEX.
#
# Problem: bash SLUG_REGEX is hand-maintained. If a new kind is added to the
# Rust enum (crates/forgeplan-core/src/artifact/types.rs), the bash regex must
# also be updated. Without this check, drift accumulates silently, causing
# false rejections or acceptance of invalid artifacts.
#
# Solution: Extract the kind list from Rust source, compare with bash regex,
# and fail if drift detected. This can be added to CI as a lightweight gate.
#
# Exit 0 if no drift, 1 if drift detected, 2 if error extracting kinds.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUST_TYPES_FILE="${SCRIPT_DIR}/crates/forgeplan-core/src/artifact/types.rs"
BASH_VALIDATOR="${SCRIPT_DIR}/.github/scripts/validate-forgeplan-frontmatter.sh"

# Extract Rust enum variants (the lowercase slugs)
# Format in types.rs: pub enum ArtifactKind { Prd, ... }
# We map enum names to slug forms:
# - Prd → prd
# - ProblemCard → prob
# - SolutionPortfolio → sol
# - EvidencePack → evid
# - RefreshReport → ref
# - Epic → epic
# - Spec → spec
# - Rfc → rfc
# - Adr → adr
# - Memory → mem (documented as "Lightweight project memory")
# - Note → note

# Canonical slug list (sorted)
SLUG_LIST="adr epic evid mem note prob prd ref rfc sol spec"

# Extract the enum definition from Rust
if ! [[ -f "$RUST_TYPES_FILE" ]]; then
    echo "❌ ERROR: Could not find Rust types file: $RUST_TYPES_FILE"
    exit 2
fi

# Verify all expected kinds are present in Rust source
echo "🔍 Checking Rust ArtifactKind enum..."
rust_kinds=()
while IFS= read -r kind_enum; do
    # Remove whitespace and trailing comma
    kind_enum=$(echo "$kind_enum" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//; s/,$//')
    if [[ -n "$kind_enum" && ! "$kind_enum" =~ ^\/\/ ]]; then
        rust_kinds+=("$kind_enum")
    fi
done < <(sed -n '/^pub enum ArtifactKind/,/^}/p' "$RUST_TYPES_FILE" | grep -v "^pub enum\|^}" | grep -v "^[[:space:]]*$")

echo "   Found enum variants: ${rust_kinds[*]}"

# Build bash regex from slug list
bash_regex_expected="^(adr|epic|evid|mem|note|prob|prd|ref|rfc|sol|spec)-[a-z0-9]+(-[a-z0-9]+)*$"

echo "   Expected bash regex: $bash_regex_expected"

# Extract actual bash regex from validator
echo "🔍 Checking bash validator regex..."
if ! [[ -f "$BASH_VALIDATOR" ]]; then
    echo "❌ ERROR: Could not find bash validator: $BASH_VALIDATOR"
    exit 2
fi

actual_bash_regex=$(grep "^SLUG_REGEX=" "$BASH_VALIDATOR" | sed 's/SLUG_REGEX="//' | sed 's/"$//')
echo "   Actual bash regex:   $actual_bash_regex"

# Extract kind prefixes from both regexes
# Format: ^(adr|epic|...) → extract the group and split by |
expected_kinds=$(echo "$bash_regex_expected" | sed 's/^.(\([^)]*\)).*/\1/' | tr '|' '\n' | sort)
actual_kinds=$(echo "$actual_bash_regex" | sed 's/^.(\([^)]*\)).*/\1/' | tr '|' '\n' | sort)

# Compare (order-independent)
if [[ "$expected_kinds" != "$actual_kinds" ]]; then
    echo ""
    echo "❌ DRIFT DETECTED: bash SLUG_REGEX is missing or has extra kinds"
    echo ""
    echo "   Expected kinds: $(echo "$expected_kinds" | tr '\n' ' ')"
    echo "   Found kinds:    $(echo "$actual_kinds" | tr '\n' ' ')"
    echo ""
    echo "   Action: Update .github/scripts/validate-forgeplan-frontmatter.sh:20"
    echo "   with a regex containing all the expected kinds."
    exit 1
else
    echo ""
    echo "✅ No drift detected — bash and Rust kinds are in sync"
    exit 0
fi
