#!/usr/bin/env bash
#
# Every `forgeplan …` example in the docs must be runnable as written.
#
# Why this exists
# ---------------
# PROB-094: the `/forge` skill — shipped to every user by `setup-skill` —
# documented `forgeplan recall --list` and `forgeplan remember "key" "value"`.
# Neither exists. A person who hits the refusal reads `--help`; an agent told
# "listing keys is done this way" spends a turn on it, or concludes the feature
# is missing and works without it.
#
# That defect was found by tripping over it, one command at a time. This checks
# all of them.
#
# What it verifies
#   1. the subcommand exists
#   2. every `--flag` in the example exists on that subcommand
#
# What it deliberately does not verify
#   Argument arity and positional shape. `remember "key" "value"` has valid
#   flags and a real subcommand, and is still wrong. Catching that needs
#   execution, which is `cli-surface-exercise.sh`'s job. Two tools, two
#   questions — this one is cheap enough to run on every PR.
#
# Sources scanned: CLAUDE.md and the shipped skill. Add more as they appear.
#
set -uo pipefail

BIN="${FORGEPLAN_BIN:-forgeplan}"
SOURCES=(
  "CLAUDE.md"
  "crates/forgeplan-cli/src/commands/forge-skill.md"
)

while [ $# -gt 0 ]; do
  case "$1" in
    --bin) BIN="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

if ! command -v "$BIN" >/dev/null 2>&1 && [ ! -x "$BIN" ]; then
  echo "forgeplan binary not found: $BIN" >&2
  exit 2
fi

TMP="$(mktemp -d "${TMPDIR:-/tmp}/fpl-docdrift.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

# Valid subcommands, straight from the binary.
"$BIN" --help 2>&1 \
  | awk '/^Commands:/,/^Options:/' \
  | grep -E "^  [a-z]" | awk '{print $1}' | grep -vx help \
  > "$TMP/commands.txt"

# Cache each subcommand's flags on first use.
flags_for() {
  local cmd="$1" f="$TMP/flags.$1"
  [ -f "$f" ] || "$BIN" "$cmd" --help 2>&1 | grep -oE '\-\-[a-z][a-z0-9-]*' | sort -u > "$f"
  cat "$f"
}

problems=0
checked=0

for src in "${SOURCES[@]}"; do
  [ -f "$src" ] || { echo "skip (missing): $src"; continue; }

  # Pull examples from inline code spans and from bare lines in fenced blocks.
  {
    grep -ohE '`forgeplan [a-z][a-z0-9-]*[^`]*`' "$src" 2>/dev/null | sed 's/`//g'
    grep -ohE '^forgeplan [a-z][a-z0-9-]*.*$' "$src" 2>/dev/null
  } \
  | sed 's/[[:space:]]*#.*$//' \
  | sed 's/[[:space:]]*$//' \
  | sort -u > "$TMP/examples.txt"

  while IFS= read -r example; do
    [ -n "$example" ] || continue

    # Prose, not an example: `forgeplan update|new|link|...` names a SET of
    # commands. Treating an alternation as a command name manufactures a
    # finding, and a checker that invents drift gets muted — which costs more
    # than the drift it was built to catch.
    case "$example" in
      *"|"*|*"..."*) continue ;;
    esac

    cmd="$(printf '%s' "$example" | awk '{print $2}')"
    [ -n "$cmd" ] || continue
    checked=$((checked+1))

    if ! grep -qx "$cmd" "$TMP/commands.txt"; then
      printf 'DRIFT  %s\n         subcommand `%s` does not exist\n         in: %s\n' \
        "$example" "$cmd" "$src"
      problems=$((problems+1))
      continue
    fi

    # Flags the example claims, minus anything inside a placeholder.
    for flag in $(printf '%s' "$example" | grep -oE '\-\-[a-z][a-z0-9-]*' | sort -u); do
      if ! flags_for "$cmd" | grep -qx -- "$flag"; then
        printf 'DRIFT  %s\n         `%s` is not a flag of `forgeplan %s`\n         in: %s\n' \
          "$example" "$flag" "$cmd" "$src"
        problems=$((problems+1))
      fi
    done
  done < "$TMP/examples.txt"
done

echo
echo "──────────────────────────────────────────────"
printf 'checked %d documented example(s), %d drift(s)\n' "$checked" "$problems"
echo "──────────────────────────────────────────────"

if [ "$problems" -gt 0 ]; then
  echo
  echo "Docs promise an interface the binary does not have. Fix the docs, or"
  echo "add the flag — but do not leave an agent following an instruction that"
  echo "cannot work."
  exit 1
fi
