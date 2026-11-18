---
title: forgeplan scan
description: "Preview source modules, file counts and line counts without persisting a module inventory."
---

`forgeplan scan` walks source directories and prints a module/file/line summary.
It does not save `.forgeplan/modules.json` or a persistent module graph.
`coverage` performs its own scan, while `drift` inspects declared affected files
and Git history; neither requires a prior `scan`.

Directory discovery skips explicit directory names and symbolic links. File and
line counting inside discovered modules can still follow source-file symlinks.
The scanner does not evaluate `.gitignore` as a general ignore policy.

## When to use

- Preview source modules before investigating decision coverage.
- Inspect the current tree after a refactor.
- Compare the printed module summary during a review.
- Help a teammate understand which source directories the scanner recognizes.

## When NOT to use

- Just to read files - use your shell. Scan is workspace-aware, not a generic file lister.
- If coverage/drift already ran and your code hasn't changed - scan is idempotent but wasteful.

## Usage

```text
forgeplan scan [OPTIONS]
```

## Options

```text
      --path <PATH>  Path to project root (default: current dir)
  -h, --help         Print help
  -V, --version      Print version
```

## Examples

### Scan the current project

```bash
forgeplan scan
```

The report prints columns `Module`, `Files`, `Lines`, then aggregate counts.
Counts depend on the scanned tree; no modules file is written.

### Scan a directory inside the current root

```bash
forgeplan scan --path crates
```

The path is canonicalized and must stay inside the current working directory. A sibling path such as `../other-project` is refused.

### Full coverage pipeline

```bash
forgeplan scan && forgeplan coverage && forgeplan drift
```

This is an optional preview followed by coverage and drift reports, not a
persisted-inventory pipeline.

## Output interpretation

| Line          | Meaning                                                    |
|---------------|------------------------------------------------------------|
| module row | source directory with recognized files, plus file and line counts |
| total | modules, files and lines observed by this scan |

Recognized extensions include Rust, JS/TS, Python, Go, Java, Kotlin, Swift and C/C++.
Hidden directories and named trees such as `target`, `node_modules`, `vendor`,
`sources`, `research`, `docs` and `templates` are skipped below the nominated root.
There is no general exclusion of all test-only modules.

## How it fits the workflow

```
scan → coverage (modules ↔ artifacts) → drift (artifacts ↔ git history) → remediate
```

Use `scan` as a preview. Coverage rescans independently; scan output is not a
required stored input for either coverage or drift.

## See also

- [`forgeplan coverage`](/docs/cli/coverage/) - per-module decision coverage
- [`forgeplan drift`](/docs/cli/drift/) - decisions whose code moved away
- [`forgeplan scan-import`](/docs/cli/scan-import/) - import foreign Markdown documents
- [`forgeplan health`](/docs/cli/health/) - separate artifact-health dashboard
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan scan --help`.

```text
Scan codebase for source modules

Usage: forgeplan scan [OPTIONS]

Options:
      --path <PATH>  Path to project root (default: current dir)
  -h, --help         Print help
  -V, --version      Print version
```
<!-- forgeplan-runtime:end -->
