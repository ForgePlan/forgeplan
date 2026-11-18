# F10 evaluation — local, federated and ContextPack

The fixed thirty tasks in `corpus/tasks.yaml` compare local CLI reads,
federated reads and ContextPack. The runner measures required-content recall,
reference noise, source attribution, payload bytes, latency p50/p95 and the
first useful source. A runner or a successful build is not a passing evaluation.

## Run

Use Python 3.9+, PyYAML and a built Forgeplan binary. Local semantic comparisons
also require `semantic-search` and a complete existing BGE-M3 cache; downloads
are disabled. `--mode pack` and `--mode federated` use Git objects without
building indexes or loading embeddings.

```bash
python3 -m unittest discover -s eval -p test_run_eval.py -v
python3 eval/run_eval.py --binary /absolute/path/to/forgeplan --out /tmp/f10-results
python3 eval/run_eval.py --mode pack --binary /absolute/path/to/forgeplan --out /tmp/f10-pack
```

`--mode local|federated|pack|all` selects modes. `--repeats` accepts 2–10
passes, default 2. The first pass and repeats use fresh CLI processes; the OS
page cache is not purged. These are not controlled cold-cache measurements.
The task definitions and their original budgets are not rewritten to fit a
particular implementation.

Use `--keep-fixture` to retain the private temporary Git repositories, indexes
and HOME after the run. The metadata records `fixture_root` and whether it was
retained, so a reviewer can inspect the exact snapshot without rebuilding its
embeddings. The default still removes temporary fixtures; no original project
or global registry is reused. Retention does not make a failed run successful.

`first_useful_ms` sums CLI subprocess runtimes through the response containing
the first useful source. It excludes later pages and evaluator overhead; it is
not a streaming first-token measurement. Aggregates also report the number of
tasks for which that timing is available.

## Isolation and version checks

The runner copies only canonical Markdown and configuration into temporary
workspaces. Local mode also initializes their indexes. All modes use a private
HOME/registry and remove inherited `FORGEPLAN_*` bindings; local mode supplies
the existing model cache. Original corpus checkouts and project data are not
modified. A failed index or embedding command stops setup rather than becoming
a warning followed by an acceptance claim.

Federated and pack requests explicitly name one recorded Git snapshot. Setup
compares every local fixture Markdown file against its pinned Git object.
Canonical fixture hashes are checked before and after each task invocation;
local mode reads those same unchanged bytes. Returned federated source ids and
revision keys must match the snapshot lockfile, not merely a generation receipt.
This does not imply an atomic snapshot across independent projects.

## What is measured

Each JSONL row includes raw stdout/stderr, arguments, cwd, exit code and latency
for every CLI invocation. The summary records binary and task SHA-256 hashes,
fixture hashes, the snapshot lockfile and aggregate measurements.

- Only returned evidence counts toward `must_include` and `must_not_include`.
  An echoed question, an omission entry or a missing-target diagnostic is not
  evidence that a requirement was retrieved.
- Search scoring uses actual returned keys/titles and local sections. It does
  not silently fetch full bodies to improve recall. A federated ambiguity
  response counts its returned artifact key as identity evidence, not as body
  content. Full-body context remains a separate cost from a search result.
- Wrong references and wrong-source hits measure relevance noise against the
  frozen task expectations. Declared contextual links can legitimately add
  material beyond that narrow expected set; inspect those rows rather than
  relabeling every extra reference as a boundary violation.
- A source-boundary violation includes an unattributed hit or a key attributed
  to a workspace that does not contain it. Version substitutions are separate.
- ContextPack follows `next_cursor` with unchanged inputs, checks progress and
  repeated unit ids, and measures each actual emitted UTF-8 response against
  `receipt.measured_bytes` and `receipt.byte_cap`. Quotes and source digests
  are checked against the frozen fixture files. Focused Core tests additionally
  verify exact source spans and declared obligation traversal.
- Source, traversal and page completeness remain separate. A typed
  `insufficient_budget` refusal is honest, but is not successful context
  delivery. The original small task budgets may therefore produce explicit
  refusals; they must not be made green by excluding response metadata.

The runner exits nonzero on a detected violation, a budget refusal or missing
required content. Reports retain those categories separately. Passing these
thirty tasks does not establish every adversarial scenario, external adapter
round-trip equivalence (F11), release-binary compatibility or release readiness.

## Corpus

`corpus/ws-a` describes drain-queue reliability; `corpus/ws-b` describes billing.
Only their canonical fixture files are tracked. Runtime stores and Git state are
temporary. `fed_query` and `pack_query` are existing frozen task-specific query
forms for the literal federated search and lexical pack respectively; they do
not represent semantic retrieval support in ContextPack.
