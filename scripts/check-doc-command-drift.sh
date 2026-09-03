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

# Cache the flag set for a command path, on first use.
#
# Takes the full path, not just the top-level name: `--yes` belongs to
# `playbook run`, and asking `playbook --help` for it reports a real flag as
# drift. Nested subcommands (playbook/plugins/mcp/discover/fpf) are the norm
# here, so resolving only the first word produces a wall of false positives.
flags_for() {
  local key f
  key="$(printf '%s' "$*" | tr ' ' '_')"
  f="$TMP/flags.$key"
  # shellcheck disable=SC2086
  [ -f "$f" ] || "$BIN" $* --help 2>&1 | grep -oE '\-\-[a-z][a-z0-9-]*' | sort -u > "$f"
  cat "$f"
}

# Deepest command path an example actually names: `forgeplan playbook run X`
# resolves to `playbook run`, `forgeplan score --all` to `score`.
resolve_path() {
  local first="$1" second="$2"
  case "$second" in
    ""|-*|PLACEHOLDER|\<*|\**) printf '%s' "$first"; return ;;
  esac
  if "$BIN" "$first" "$second" --help >/dev/null 2>&1; then
    printf '%s %s' "$first" "$second"
  else
    printf '%s' "$first"
  fi
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

    # Flags the example claims, checked against the deepest command path it
    # names — see resolve_path.
    sub="$(printf '%s' "$example" | awk '{print $3}')"
    path="$(resolve_path "$cmd" "$sub")"
    for flag in $(printf '%s' "$example" | grep -oE '\-\-[a-z][a-z0-9-]*' | sort -u); do
      if ! flags_for $path | grep -qx -- "$flag"; then
        printf 'DRIFT  %s\n         `%s` is not a flag of `forgeplan %s`\n         in: %s\n' \
          "$example" "$flag" "$path" "$src"
        problems=$((problems+1))
      fi
    done
  done < "$TMP/examples.txt"
done

# ---------------------------------------------------------------------------
# Emitted hints — the surface an agent is CONTRACTUALLY OBLIGED to run
# ---------------------------------------------------------------------------
#
# Documentation drift is bad; hint drift is worse. PRD-071 requires `Next:`
# and `Fix:` to be runnable as-is, and an agent follows them verbatim. A hint
# naming a command that does not exist spends the agent's turn on an error.
#
# Issue #348 is exactly this: `forgeplan link` emitted
# `Next: forgeplan score-all` for months. The real command is
# `forgeplan score --all`. Documentation checks never saw it, because the
# string lives in Rust source, not in a doc file.
#
# Placeholders (`{id}`, `<id>`) are expected in these strings and are ignored —
# only the subcommand name and literal flags are checked.

echo
echo "Scanning emitted hints (Next:/Fix:/with_action)…"

# Only the surfaces that ARE the contract: an action attached to a Hint, or a
# string a user sees prefixed with Next:/Fix:/Or:. Scanning every string that
# happens to contain the word "forgeplan" pulls in prose from comments
# ("the forgeplan binary built by cargo test") and reports it as a missing
# command — a checker that cries wolf gets muted, and then catches nothing.
{
  grep -rhoE 'with_action\(\s*(format!\()?"[^"]+"' crates/ --include="*.rs" 2>/dev/null \
    | sed 's/.*"\(.*\)"*/\1/' | tr -d '"'
  grep -rhoE '"(Next|Fix|Or): forgeplan [^"]*"' crates/ --include="*.rs" 2>/dev/null \
    | tr -d '"' | sed 's/^[A-Za-z]*: //'
} \
  | grep -E '^forgeplan ' \
  | sed 's/{[^}]*}/PLACEHOLDER/g' \
  | sed 's/[[:space:]]*$//' \
  | sort -u > "$TMP/hints.txt" || true

while IFS= read -r hint; do
  [ -n "$hint" ] || continue
  case "$hint" in
    *"|"*|*"..."*|*"<"*) continue ;;   # prose or a placeholder-only template
  esac

  cmd="$(printf '%s' "$hint" | awk '{print $2}')"
  [ -n "$cmd" ] || continue
  case "$cmd" in PLACEHOLDER|\**) continue ;; esac
  checked=$((checked+1))

  if ! grep -qx "$cmd" "$TMP/commands.txt"; then
    printf 'DRIFT  %s\n         emitted hint names `%s`, which is not a command\n' \
      "$hint" "$cmd"
    problems=$((problems+1))
    continue
  fi

  hsub="$(printf '%s' "$hint" | awk '{print $3}')"
  hpath="$(resolve_path "$cmd" "$hsub")"
  for flag in $(printf '%s' "$hint" | grep -oE '\-\-[a-z][a-z0-9-]*' | sort -u); do
    if ! flags_for $hpath | grep -qx -- "$flag"; then
      printf 'DRIFT  %s\n         emitted hint uses `%s`, not a flag of `forgeplan %s`\n' \
        "$hint" "$flag" "$hpath"
      problems=$((problems+1))
    fi
  done
done < "$TMP/hints.txt"

echo
echo "──────────────────────────────────────────────"
printf 'checked %d documented example(s) and emitted hint(s), %d drift(s)\n' \
  "$checked" "$problems"
echo "──────────────────────────────────────────────"

if [ "$problems" -gt 0 ]; then
  echo
  echo "Something promises an interface the binary does not have. Fix it, or add"
  echo "the flag — but do not leave an agent following an instruction that cannot"
  echo "work. For an emitted hint this is a PRD-071 violation, not a typo."
  exit 1
fi
