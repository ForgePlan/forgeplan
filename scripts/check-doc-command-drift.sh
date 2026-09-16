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
# Sources scanned: the repo-level docs plus the site corpora.
#
# The site corpora were never scanned until now, which is how the lessons
# happily documented commands that do not exist: `CLAUDE.md` is not the only
# place a user copies an example from. Only the ENGLISH tree is scanned —
# `ru/docs` mirrors the same examples and would report every finding twice.
SOURCES=(
  "CLAUDE.md"
  "crates/forgeplan-cli/src/commands/forge-skill.md"
  "website/src/content/docs/docs"
  "website/public/guides-raw"
)

# The changelog is a generated copy of the root `CHANGELOG.md` and an
# append-only release log. It records CLI shapes as they were — including one
# command (`forgeplan score-all`) that the changelog itself calls out as never
# having existed. Demanding that history describe today's binary is not a check,
# it is noise.
EXCLUDE=(
  "website/src/content/docs/docs/changelog.md"
)

# One tutorial's subject IS a feature that does not exist yet: it walks a reader
# through writing a PRD for "add `--dry-run` to `forgeplan new`". Its examples of
# that proposed surface are the lesson, not instructions to run anything — so the
# fictional surface is ignored, in those two files only, and nothing else.
FICTION_FILES=(
  "website/src/content/docs/docs/guides/first-artifact.md"
  "website/public/guides-raw/first-artifact.html"
)
FICTION_CMDS="preview"
FICTION_FLAGS="--dry-run"

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

# A directory entry means "every doc under it". The old `[ -f "$src" ]` guard
# silently skipped a directory, so listing one looked like it worked.
FILES=()
for entry in "${SOURCES[@]}"; do
  if [ -d "$entry" ]; then
    while IFS= read -r f; do
      FILES+=("$f")
    done < <(find "$entry" -type f \( -name '*.md' -o -name '*.mdx' -o -name '*.html' \) | sort)
  else
    FILES+=("$entry")
  fi
done
for exc in "${EXCLUDE[@]}"; do
  keep=()
  for f in "${FILES[@]}"; do
    [ "$f" = "$exc" ] || keep+=("$f")
  done
  FILES=(${keep[@]+"${keep[@]}"})
done

for src in "${FILES[@]}"; do
  [ -f "$src" ] || { echo "skip (missing): $src"; continue; }

  # Pull examples from inline code spans, from bare lines in fenced blocks, and
  # — for the HTML lessons — from the two forms a command actually takes there:
  # a `<code>` element, and the terminal transcript's
  # `<span class="to-prompt">$</span> forgeplan …` line.
  #
  # Both HTML forms are matched as *elements*, not by stripping tags off the
  # whole file: the lessons carry their transcripts as escaped `\n` strings
  # inside one `<script>` blob, and stripping tags across that produced a wall
  # of phantom commands ending in `\n\nProject` or `.',`.
  {
    grep -ohE '`forgeplan [a-z][a-z0-9-]*[^`]*`' "$src" 2>/dev/null | sed 's/`//g'
    grep -ohE '^forgeplan [a-z][a-z0-9-]*.*$' "$src" 2>/dev/null
    if [ "${src##*.}" = "html" ]; then
      # A `<code>` element holding a command. Anchored: `<code>Error: .forgeplan/
      # already exists</code>` mentions the binary without being an example.
      grep -ohE '<code[^>]*>forgeplan[^<]*</code>' "$src" 2>/dev/null \
        | sed -E 's/<[^>]*>//g'
      # A command block: <div class="pc-cmd">forgeplan …</div>
      grep -ohE '<div class="pc-cmd">[^<]*forgeplan[^<]*</div>' "$src" 2>/dev/null \
        | sed -E 's/<[^>]*>//g'
      # Terminal transcripts, both shapes they take here:
      #   <span class="to-prompt">$</span> forgeplan …
      #   <span class="m-next">Next:</span> forgeplan …
      grep -ohE '<span class="to-prompt">\$</span> forgeplan [^\\<]*' "$src" 2>/dev/null \
        | sed -E 's/<[^>]*>//g; s/^\$[[:space:]]*//'
      grep -ohE '<span class="[a-z-]*">(Next|Fix|Or):</span> forgeplan [^\\<]*' "$src" 2>/dev/null \
        | sed -E 's/<[^>]*>//g; s/^(Next|Fix|Or): //'
    fi
  } \
  | sed 's/&lt;/</g; s/&gt;/>/g; s/&quot;/"/g; s/&amp;/\&/g' \
  | sed -E 's/\\n.*$//' \
  | sed 's/[[:space:]]*#.*$//' \
  | sed -E 's/[[:space:]]+$//' \
  | sed -E "s/[[:space:]]*[',.;:)]+\$//" \
  | sed -E 's/[[:space:]]+$//' \
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

    # The proposed-feature tutorial names a surface that does not exist yet —
    # see FICTION_FILES. Ignore exactly that surface, in exactly those files.
    fiction=0
    for ff in "${FICTION_FILES[@]}"; do
      [ "$src" = "$ff" ] && fiction=1
    done
    if [ "$fiction" = 1 ]; then
      case " $FICTION_CMDS " in *" $cmd "*) continue ;; esac
    fi

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
    # A `--flag` inside a quoted argument is data, not a flag: `forgeplan route
    # "add --dry-run to new"` does not make `--dry-run` a flag of `route`. The
    # second form also drops an unbalanced quote a truncated example leaves.
    flags_src="$(printf '%s' "$example" | sed -E 's/"[^"]*"//g; s/".*$//')"
    for flag in $(printf '%s' "$flags_src" | grep -oE '\-\-[a-z][a-z0-9-]*' | sort -u); do
      if [ "$fiction" = 1 ]; then
        case " $FICTION_FLAGS " in *" $flag "*) continue ;; esac
      fi
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
