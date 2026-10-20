#!/usr/bin/env python3
"""Sequential S1 acceptance executor — stdlib only, no imports from run_eval.

This is the *runner*, not a second quality engine. It decides what to build,
what to execute and what to record; every behavioural assertion stays in the
Rust suites it invokes. It exists because `cargo test --workspace` accumulates
every linked test executable before running any of them, which on this machine
does not fit next to a production binary, and because a claim like "52 PASS"
is only evidence if the log says which binaries ran, with what flags, and with
what real exits.

What it does, in order:

1. `cargo metadata --no-deps --format-version 1 --offline --locked` to learn
   the workspace's own target directory and every test target's name,
   `required-features` and path. Nothing here hardcodes a job count, a
   profile or a feature set.
2. `cargo check --workspace --all-targets` with package-qualified profile
   features, so no package receives a feature it does not declare.
3. For each selected group: `cargo test … --no-run --message-format=json`,
   collect **every** compiler artifact, then execute each test executable
   directly with `--test-threads=1` and record its real exit code and counts.
4. Same for doctests, which the executable runner does not cover.

What it deliberately does not do:

* it does not `cargo clean`, and it does not delete anything it did not
  itself link;
* it does not pass or reinterpret a test outcome — `--test-threads=1` and the
  binary's own exit are the result;
* it does not treat "0 tests" as success. A selected group whose binary ran
  zero cases is recorded as `ran_nothing` and, for a required group, fails the
  run. `RUN DID NOT HAPPEN` is a distinct state from `PASS`.

Usage:
    python3 eval/run_acceptance.py --phase s1 --profile no-semantic \\
        --model-cache ~/.forgeplan/models --out .local/…/s1

Exit code is 0 only when every executed group reported a real run and a real
pass. `--dry-run` prints the plan and touches nothing.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import signal
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# The CLI writes a checkpoint before risky work and updates it on failures.
_ACTIVE_REPORT_PATH: Path | None = None
_ACTIVE_REPORT: dict | None = None

# Features are package-scoped. Passing `test-helpers` to every workspace
# package makes Cargo reject the CLI and MCP targets, which do not declare it.
PROFILE_FEATURES = {
    "no-semantic": {
        "forgeplan-core": ["test-helpers"],
        "forgeplan": [],
        "forgeplan-mcp": [],
    },
    "semantic": {
        "forgeplan-core": ["test-helpers", "semantic-search"],
        "forgeplan": ["semantic-search"],
        "forgeplan-mcp": ["semantic-search"],
    },
}

# Which suites each acceptance phase requires. A missing target is a failure,
# not a silently skipped phase — "we could not run the thing" and "the thing
# passed" must never look the same in a report.
PHASE_GROUPS = {
    "s1": [
        {
            "name": "core-constellation",
            "package": "forgeplan-core",
            "tests": [
                "constellation_source_view",
                "constellation_embedding_state",
                "constellation_embedding_preservation",
                "constellation_pinned_search",
                "constellation_runtime_e2e",
            ],
        },
        {
            "name": "mcp-constellation",
            "package": "forgeplan-mcp",
            "tests": [
                "constellation_migrated_read",
                "constellation_migrated_write",
                "constellation_migrated_e2e",
                "constellation_process_e2e",
            ],
        },
        {
            "name": "cli-migrated-write",
            "package": "forgeplan",
            "tests": ["cli_migrated_write"],
        },
        {
            "name": "core-lifecycle-canonical",
            "package": "forgeplan-core",
            "tests": ["lifecycle_canonical"],
        },
        {
            "name": "core-semantic-embedding",
            "package": "forgeplan-core",
            "tests": ["embedding_reference"],
            "required_features": ["semantic-search"],
        },
        {
            "name": "core-lib",
            "package": "forgeplan-core",
            "lib": True,
            "bins": True,
        },
    ],
    # S2 suites are declared against the phase that owns them. Listing them
    # under `s1` made an S1 run fail on suites that do not exist yet — a
    # missing target must stay a failure, but it must be a failure of the
    # phase that actually requires it, not of every earlier phase.
    "s2": [
        {
            "name": "core-constellation-boundary",
            "package": "forgeplan-core",
            "tests": [
                "constellation_source_boundary",
                "constellation_portability",
            ],
        },
        {
            "name": "mcp-constellation-handles",
            "package": "forgeplan-mcp",
            "tests": ["constellation_handles_e2e"],
        },
    ],
    "s3": [],
    "s4": [],
}

REQUIRED_MIN_FREE_BYTES = 10 * 1024**3
STOP_BELOW_FREE_BYTES = 3 * 1024**3



class Failure(Exception):
    """A named prerequisite failure — never an exit 0."""


def log(message: str) -> None:
    print(message, flush=True)


def save_report(report: dict, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2, default=str))


def free_bytes(path: Path) -> int:
    st = os.statvfs(path)
    return st.f_bavail * st.f_frsize

def _digest_files(paths: set[Path], root: Path = REPO) -> str:
    digest = hashlib.sha256()
    for path in sorted(paths):
        if not path.is_file():
            continue
        digest.update(path.relative_to(root).as_posix().encode())
        digest.update(b"\0")
        with path.open("rb") as source:
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(chunk)
    return digest.hexdigest()


def source_fingerprints() -> dict[str, str]:
    manifests = {REPO / "Cargo.toml", REPO / "Cargo.lock"}
    manifests.update(REPO.glob("crates/*/Cargo.toml"))
    production = manifests | set(REPO.glob("crates/**/src/**/*.rs"))
    production |= set(REPO.glob("crates/**/build.rs"))
    production |= set(REPO.glob("crates/**/proto/**/*.proto"))
    tests = set(REPO.glob("crates/**/tests/**/*.rs"))
    harness = set(REPO.glob("eval/**/*.py"))
    return {
        "production": _digest_files(production),
        "tests": _digest_files(tests),
        "harness": _digest_files(harness),
        "cargo_lock": _digest_files({REPO / "Cargo.lock"}),
    }


def model_cache_fingerprint(path: Path) -> str:
    """Fingerprint model-cache file names and metadata without reading weights."""
    digest = hashlib.sha256()
    if not path.is_dir():
        return "missing"
    for item in sorted(path.rglob("*")):
        if item.is_file():
            stat = item.stat()
            digest.update(item.relative_to(path).as_posix().encode())
            digest.update(f"\\0{stat.st_size}\\0{stat.st_mtime_ns}\\n".encode())
    return digest.hexdigest()


def binary_sha256(path: str) -> str:
    digest = hashlib.sha256()
    with Path(path).open("rb") as binary:
        for chunk in iter(lambda: binary.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


@dataclass
class CommandResult:
    returncode: int
    stdout: str
    stderr: str
    source_fingerprints_before: dict[str, str]
    source_fingerprints_after: dict[str, str]
    timed_out: bool = False


def _terminate_process_tree(proc: subprocess.Popen[str]) -> None:
    """Stop the command and descendants after a timeout."""
    if os.name == "nt":
        subprocess.run(
            ["taskkill", "/PID", str(proc.pid), "/T", "/F"],
            capture_output=True,
            text=True,
            check=False,
        )
    else:
        try:
            os.killpg(proc.pid, signal.SIGTERM)
        except ProcessLookupError:
            return
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    if proc.poll() is None:
        proc.kill()


def run(cmd: list[str], cwd: Path, timeout: int) -> CommandResult:
    log(f"  $ {' '.join(cmd)}")
    before = source_fingerprints()
    popen_options: dict = {}
    if os.name == "nt":
        popen_options["creationflags"] = subprocess.CREATE_NEW_PROCESS_GROUP
    else:
        popen_options["start_new_session"] = True
    try:
        proc = subprocess.Popen(
            cmd,
            cwd=str(cwd),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            **popen_options,
        )
    except OSError as exc:
        raise Failure(f"could not start command {' '.join(cmd)}: {exc}") from exc
    timed_out = False
    try:
        stdout, stderr = proc.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
        _terminate_process_tree(proc)
        stdout, stderr = proc.communicate()
        stderr += f"\n[executor] timeout after {timeout}s"
    after = source_fingerprints()
    if before != after:
        raise Failure(
            f"source/test/harness files changed while command ran: {' '.join(cmd)}"
        )
    return CommandResult(
        returncode=124 if timed_out else proc.returncode,
        stdout=stdout,
        stderr=stderr,
        source_fingerprints_before=before,
        source_fingerprints_after=after,
        timed_out=timed_out,
    )


def cargo_metadata(manifest_dir: Path) -> dict:
    cmd = [
        "cargo",
        "metadata",
        "--no-deps",
        "--format-version",
        "1",
        "--offline",
        "--locked",
        "--manifest-path",
        str(REPO / "Cargo.toml"),
    ]
    proc = run(cmd, manifest_dir, timeout=300)
    if proc.returncode != 0:
        raise Failure(f"cargo metadata failed:\n{proc.stderr[-4000:]}")
    return json.loads(proc.stdout)


def target_dir(metadata: dict, explicit: str | None, *, create: bool) -> Path:
    """Resolve a target directory inside this worktree."""
    def resolve(value: str) -> Path:
        requested = Path(value).expanduser()
        if not requested.is_absolute():
            requested = REPO / requested
        return requested.resolve()

    requested = explicit or os.environ.get("CARGO_TARGET_DIR")
    if requested:
        resolved = resolve(requested)
    else:
        resolved = Path(metadata["target_directory"]).resolve()
    try:
        resolved.relative_to(REPO)
    except ValueError as exc:
        raise Failure(
            f"target directory {resolved} is outside this worktree {REPO}; "
            "point --target-dir at its own directory or unset CARGO_TARGET_DIR"
        ) from exc
    if create:
        resolved.mkdir(parents=True, exist_ok=True)
    return resolved


def declared_test_targets(metadata: dict) -> dict[str, dict]:
    """Every Cargo test target, keyed by package and target name."""
    out: dict[str, dict] = {}
    for package in metadata["packages"]:
        for target in package["targets"]:
            if "test" in target["kind"]:
                out[f"{package['name']}:{target['name']}"] = {
                    "package": package["name"],
                    "required_features": target.get("required-features", []) or [],
                }
    return out


def missing_targets(groups: list[dict], declared: dict[str, dict]) -> list[str]:
    return [
        f"{group['package']}:{name}"
        for group in groups
        for name in group.get("tests", [])
        if f"{group['package']}:{name}" not in declared
    ]


def runnable_groups(
    groups: list[dict], declared: dict[str, dict], profile: str
) -> tuple[list[dict], list[str], list[str]]:
    """Return runnable groups, semantic N/A targets, and other feature blockers."""
    runnable = []
    not_applicable = []
    blocked = []
    for group in groups:
        tests = []
        enabled = set(PROFILE_FEATURES[profile].get(group["package"], []))
        for name in group.get("tests", []):
            key = f"{group['package']}:{name}"
            target = declared.get(key)
            if target is None:
                continue
            missing = (
                set(target["required_features"])
                | set(group.get("required_features", []))
            ).difference(enabled)
            if not missing:
                tests.append(name)
            elif profile == "no-semantic" and missing == {"semantic-search"}:
                not_applicable.append(key)
            else:
                blocked.append(f"{key} (requires {sorted(missing)})")
        if tests or group.get("lib") or group.get("bins"):
            runnable.append({**group, "tests": tests})
    return runnable, not_applicable, blocked


def features_for(
    profile: str, target: dict | None, package: str = "forgeplan-core"
) -> list[str]:
    base = PROFILE_FEATURES[profile].get(package, [])
    required = set(target.get("required_features", [])) if target else set()
    missing = required.difference(base)
    if missing:
        raise Failure(
            f"profile {profile} omits {package}-required features: {sorted(missing)}"
        )
    return ["--features", ",".join(base)] if base else []


def workspace_features_for(profile: str) -> list[str]:
    return [
        f"{package}/{feature}"
        for package, features in PROFILE_FEATURES[profile].items()
        for feature in features
    ]


def check_workspace(
    manifest_dir: Path, target_directory: Path, profile: str, timeout: int
) -> dict:
    features = ",".join(workspace_features_for(profile))


    cmd = [
        "cargo",
        "check",
        "--no-default-features",
        "--target-dir",
        str(target_directory),
        "--workspace",
        "--all-targets",
        "--keep-going",
        "--features",
        features,
        "--offline",
        "--locked",
    ]
    before = free_bytes(REPO)
    proc = run(cmd, manifest_dir, timeout)
    after = free_bytes(REPO)
    return {
        "command": cmd,
        "exit": proc.returncode,
        "timed_out": proc.timed_out,
        "stdout_tail": proc.stdout[-8000:],
        "stderr_tail": proc.stderr[-8000:],
        "stdout": proc.stdout,
        "stderr": proc.stderr,
        "free_before": before,
        "free_after": after,
        "source_fingerprints_before": proc.source_fingerprints_before,
        "source_fingerprints_after": proc.source_fingerprints_after,
    }


def test_selection_argv(group: dict) -> list[str]:
    """The `--test` selectors (or `--lib`/`--bins`) for one group."""
    argv: list[str] = []
    for name in group.get("tests", []):
        argv += ["--test", name]
    if group.get("lib"):
        argv.append("--lib")
    if group.get("bins"):
        argv.append("--bins")
    return argv


def build_group_argv(
    target_directory: Path,
    group: dict,
    profile: str,
    declared: dict[str, dict],
) -> list[str]:
    targets = [
        declared[f"{group['package']}:{name}"] for name in group.get("tests", [])
    ]
    wrong_package = [
        target["package"] for target in targets if target["package"] != group["package"]
    ]
    if wrong_package:
        raise Failure(
            f"{group['name']} selects targets from other packages: {sorted(set(wrong_package))}"
        )
    required_features = {
        feature for target in targets for feature in target["required_features"]
    } | set(group.get("required_features", []))
    feature_args = features_for(
        profile,
        {"required_features": sorted(required_features)},
        group["package"],
    )
    return [
        "cargo",
        "test",
        "--no-default-features",
        "--target-dir",
        str(target_directory),
        "-p",
        group["package"],
        *feature_args,
        *test_selection_argv(group),
        "--no-run",
        "--message-format=json",
        "--offline",
        "--locked",
    ]




def is_test_executable_artifact(message: dict) -> bool:
    """Accept integration, library and binary test targets, not ordinary builds."""
    target = message.get("target", {})
    return (
        message.get("reason") == "compiler-artifact"
        and message.get("profile", {}).get("test") is True
        and bool(set(target.get("kind", [])) & {"test", "lib", "bin"})
    )


def build_group(
    manifest_dir: Path,
    target_directory: Path,
    group: dict,
    profile: str,
    declared: dict[str, dict],
    timeout: int,
) -> dict:
    """Link one group and return its command, logs and test executables."""
    argv = build_group_argv(target_directory, group, profile, declared)
    before = free_bytes(REPO)
    proc = run(argv, manifest_dir, timeout)
    after = free_bytes(REPO)
    if proc.returncode != 0:
        raise Failure(
            f"link failed for {group['name']} (exit {proc.returncode}):\n"
            f"{proc.stderr[-8000:]}\n{proc.stdout[-4000:]}"
        )

    executables: list[str] = []
    for line in proc.stdout.splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not is_test_executable_artifact(message):
            continue
        path = message.get("executable")
        if isinstance(path, str) and path:
            executables.append(path)
    if not executables:
        raise Failure(
            f"no test executable was produced for {group['name']}: the suite "
            "is feature-gated off, misdeclared, or silently skipped"
        )
    hashes: dict[str, str | None] = {}
    for path in executables:
        try:
            hashes[path] = binary_sha256(path)
        except OSError:
            hashes[path] = None
    return {
        "command": argv,
        "free_before": before,
        "free_after": after,
        "stdout_tail": proc.stdout[-8000:],
        "stderr_tail": proc.stderr[-8000:],
        "timed_out": proc.timed_out,
        "executables": executables,
        "executable_sha256": hashes,
        "source_fingerprints_before": proc.source_fingerprints_before,
        "source_fingerprints_after": proc.source_fingerprints_after,
    }


TEST_RESULT_RE = re.compile(
    r"\b(?P<passed>\d+) passed;\s*(?P<failed>\d+) failed;\s*"
    r"(?P<ignored>\d+) ignored\b"
)


def parse_test_counts(stdout: str) -> dict[str, int]:
    counts = {"passed": 0, "failed": 0, "ignored": 0}
    for line in stdout.splitlines():
        if not line.strip().startswith("test result:"):
            continue
        match = TEST_RESULT_RE.search(line)
        if match:
            for label in counts:
                counts[label] += int(match.group(label))
    return counts


def text_output(value: str | bytes | None) -> str:
    if isinstance(value, bytes):
        return value.decode(errors="replace")
    return value or ""

def execute(executable: str, timeout: int) -> dict:
    """Run one linked test binary directly and record what it really did."""
    try:
        binary_sha256_before = binary_sha256(executable)
    except OSError:
        binary_sha256_before = None
    before = free_bytes(REPO)
    fingerprints_before = source_fingerprints()
    start = time.monotonic()
    popen_options: dict = {}
    if os.name == "nt":
        popen_options["creationflags"] = subprocess.CREATE_NEW_PROCESS_GROUP
    else:
        popen_options["start_new_session"] = True
    timed_out = False
    try:
        proc = subprocess.Popen(
            [executable, "--test-threads=1"],
            cwd=str(REPO),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            **popen_options,
        )
        try:
            stdout, stderr = proc.communicate(timeout=timeout)
            exit_code = proc.returncode
        except subprocess.TimeoutExpired:
            timed_out = True
            _terminate_process_tree(proc)
            stdout, stderr = proc.communicate()
            stderr += f"\n[executor] timeout after {timeout}s"
            exit_code = 124
    except OSError as exc:
        stdout, stderr = "", f"[executor] could not start {executable}: {exc}"
        exit_code = 127
    try:
        binary_sha256_after = binary_sha256(executable)
    except OSError:
        binary_sha256_after = None
    fingerprints_after = source_fingerprints()
    elapsed = time.monotonic() - start
    counts = parse_test_counts(stdout)
    ran_nothing = counts["passed"] == 0 and counts["failed"] == 0
    return {
        "executable": executable,
        "exit": exit_code,
        "timed_out": timed_out,
        "elapsed_s": round(elapsed, 2),
        "counts": counts,
        "ran_nothing": ran_nothing,
        "free_before": before,
        "free_after": free_bytes(REPO),
        "stdout_tail": stdout[-8000:],
        "stderr_tail": stderr[-4000:],
        "source_fingerprints_before": fingerprints_before,
        "source_fingerprints_after": fingerprints_after,
        "source_changed": fingerprints_before != fingerprints_after,
        "binary_sha256_before": binary_sha256_before,
        "binary_sha256_after": binary_sha256_after,
        "binary_changed": binary_sha256_before != binary_sha256_after,
    }


def doctests(
    manifest_dir: Path, target_directory: Path, profile: str, timeout: int
) -> dict:
    features = ",".join(workspace_features_for(profile))
    cmd = [
        "cargo",
        "test",
        "--no-default-features",
        "--target-dir",
        str(target_directory),
        "--workspace",
        "--features",
        features,
        "--doc",
        "--offline",
        "--locked",
    ]
    before = free_bytes(REPO)
    proc = run(cmd, manifest_dir, timeout)
    counts = parse_test_counts(proc.stdout)
    ran_nothing = counts["passed"] == 0 and counts["failed"] == 0
    return {
        "command": cmd,
        "exit": proc.returncode,
        "counts": counts,
        "ran_nothing": ran_nothing,
        "timed_out": proc.timed_out,
        "stdout_tail": proc.stdout[-8000:],
        "stderr_tail": proc.stderr[-8000:],
        "free_before": before,
        "free_after": free_bytes(REPO),
        "source_fingerprints_before": proc.source_fingerprints_before,
        "source_fingerprints_after": proc.source_fingerprints_after,
    }

def main(argv: list[str] | None = None) -> int:
    global _ACTIVE_REPORT, _ACTIVE_REPORT_PATH
    _ACTIVE_REPORT = None
    _ACTIVE_REPORT_PATH = None
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--phase", default="all", choices=["s1", "s2", "s3", "s4", "all"])
    parser.add_argument("--profile", default="no-semantic", choices=["no-semantic", "semantic", "all"])
    parser.add_argument("--out", required=True, help="directory for result.json and logs")
    parser.add_argument("--model-cache", default=str(Path.home() / ".forgeplan" / "models"))
    parser.add_argument("--target-dir", default=None)
    parser.add_argument("--timeout", type=int, default=3300, help="per-command wall clock bound")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)

    out_dir = Path(args.out).expanduser().resolve()
    started = time.time()
    if not args.dry_run:
        _ACTIVE_REPORT_PATH = out_dir / "result.json"
        _ACTIVE_REPORT = {
            "schema_version": 1,
            "started_at": started,
            "phase": args.phase,
            "profile": args.profile,
            "worktree": str(REPO),
            "runs": [],
            "failures": [],
            "result": "RUNNING",
        }
        save_report(_ACTIVE_REPORT, _ACTIVE_REPORT_PATH)

    if args.model_cache:
        # Explicit, never guessed from a platform path: the model cache is
        # per-machine and a silent miss turns every semantic case into a
        # misleading "unavailable".
        os.environ["FORGEPLAN_MODEL_CACHE"] = str(Path(args.model_cache).expanduser())

    free_start = free_bytes(REPO)
    if not args.dry_run and free_start < REQUIRED_MIN_FREE_BYTES:
        raise Failure(
            f"only {free_start} bytes free under {REPO}; a full batch needs "
            f"{REQUIRED_MIN_FREE_BYTES}. This is a prerequisite, not a reason "
            "to lower the threshold or to delete artifacts that are not ours."
        )

    metadata = cargo_metadata(REPO)
    declared = declared_test_targets(metadata)
    target = target_dir(metadata, args.target_dir, create=not args.dry_run)
    fingerprints_start = source_fingerprints()
    model_cache_path = Path(args.model_cache).expanduser()
    model_cache_start = model_cache_fingerprint(model_cache_path)

    profiles = ["no-semantic", "semantic"] if args.profile == "all" else [args.profile]
    phases = ["s1", "s2", "s3", "s4"] if args.phase == "all" else [args.phase]

    report = _ACTIVE_REPORT if _ACTIVE_REPORT is not None else {}
    report.update(
        {
            "schema_version": 1,
            "started_at": started,
            "phase": args.phase,
            "profile": args.profile,
            "worktree": str(REPO),
            "target_directory": str(target),
            "model_cache": args.model_cache,
            "source_fingerprints_before": fingerprints_start,
            "cargo_lock_sha256": fingerprints_start["cargo_lock"],
            "profile_fingerprints": {
                profile: {
                    "features": workspace_features_for(profile),
                    "semantic_search": profile == "semantic",
                    "model_cache_metadata_sha256": model_cache_start,
                    "require_model_in_tests": profile == "semantic",
                }
                for profile in profiles
            },
            "limits": {
                "timeout_s_per_command": args.timeout,
                "required_min_free_bytes": REQUIRED_MIN_FREE_BYTES,
                "stop_below_free_bytes": STOP_BELOW_FREE_BYTES,
            },
            "free_bytes_at_start": free_start,
            "source_state": "UNCOMMITTED",
            "runs": [],
            "failures": [],
            "result": "RUNNING",
        }
    )
    if not args.dry_run:
        save_report(report, _ACTIVE_REPORT_PATH)

    if args.dry_run:
        report["dry_run"] = True
        report["declared_test_targets"] = sorted(declared)
        log(json.dumps(report["declared_test_targets"], indent=2))
        return 0

    out_dir.mkdir(parents=True, exist_ok=True)

    for profile in profiles:
        if profile == "semantic":
            os.environ["FORGEPLAN_REQUIRE_MODEL_IN_TESTS"] = "1"
        for phase in phases:
            groups = PHASE_GROUPS.get(phase) or []
            if not groups:
                report["failures"].append(
                    f"{phase}/{profile}: no acceptance suites are declared"
                )
                report["runs"].append(
                    {"phase": phase, "profile": profile, "status": "not_implemented"}
                )
                save_report(report, _ACTIVE_REPORT_PATH)
                continue

            absent = missing_targets(groups, declared)
            if absent:
                report["failures"].append(
                    f"{phase}/{profile}: required test targets are missing from Cargo metadata: {absent}"
                )
                report.setdefault("missing_test_targets", []).extend(
                    f"{phase}/{profile}:{name}" for name in absent
                )
                log(f"!! {phase}/{profile}: required targets missing {absent}; running available suites")
            available_groups, not_applicable, blocked = runnable_groups(
                groups, declared, profile
            )
            if not_applicable:
                report.setdefault("not_applicable_test_targets", []).extend(
                    f"{phase}/{profile}:{name}" for name in not_applicable
                )
            if blocked:
                report["failures"].append(
                    f"{phase}/{profile}: targets require unavailable profile features: {blocked}"
                )
            save_report(report, _ACTIVE_REPORT_PATH)

            free = free_bytes(REPO)
            if free < REQUIRED_MIN_FREE_BYTES:
                raise Failure(
                    f"{free} bytes free before {phase}/{profile}; the threshold is "
                    f"{REQUIRED_MIN_FREE_BYTES} and is not lowered here."
                )

            log(f"== {phase} / {profile} — checking")
            check = check_workspace(REPO, target, profile, args.timeout)
            report["runs"].append({"phase": phase, "profile": profile, "check": check})
            save_report(report, _ACTIVE_REPORT_PATH)
            log(f"   check exit={check['exit']}")
            if check["exit"] != 0:
                report["failures"].append(f"{phase}/{profile}: cargo check failed")
                save_report(report, _ACTIVE_REPORT_PATH)
                continue

            for group in available_groups:
                log(f"== {phase} / {profile} — {group['name']}")
                if free_bytes(REPO) < STOP_BELOW_FREE_BYTES:
                    raise Failure(
                        f"{free_bytes(REPO)} bytes free while building {group['name']}; "
                        "stopping rather than filling the disk"
                    )
                try:
                    build = build_group(
                        REPO, target, group, profile, declared, args.timeout
                    )
                except Failure as exc:
                    report["failures"].append(str(exc))
                    log(f"!! {exc}")
                    save_report(report, _ACTIVE_REPORT_PATH)
                    continue

                executables = build["executables"]
                group_result = {
                    "group": group["name"],
                    "profile": profile,
                    "build_command": build["command"],
                    "build_free_before": build["free_before"],
                    "build_free_after": build["free_after"],
                    "build_stdout_tail": build["stdout_tail"],
                    "build_stderr_tail": build["stderr_tail"],
                    "test_binary_count": len(executables),
                    "executables": [],
                    "executable_sha256": build["executable_sha256"],
                    "source_fingerprints_before": build["source_fingerprints_before"],
                    "source_fingerprints_after": build["source_fingerprints_after"],
                }
                report["runs"].append(group_result)
                save_report(report, _ACTIVE_REPORT_PATH)
                for executable in executables:
                    result = execute(executable, args.timeout)
                    group_result["executables"].append(result)
                    log(
                        f"   {Path(executable).name}: exit={result['exit']} "
                        f"passed={result['counts']['passed']} failed={result['counts']['failed']}"
                    )
                    if result["exit"] != 0:
                        report["failures"].append(
                            f"{group['name']}: {Path(executable).name} exited {result['exit']}"
                        )
                    if result["source_changed"]:
                        report["failures"].append(
                            f"{group['name']}: source/test/harness files changed during execution"
                        )
                    if result["binary_changed"]:
                        report["failures"].append(
                            f"{group['name']}: test executable changed during execution"
                        )
                    if result["ran_nothing"]:
                        report["failures"].append(
                            f"{group['name']}: {Path(executable).name} ran zero tests — "
                            "RUN DID NOT HAPPEN, which is not a pass"
                        )
                    save_report(report, _ACTIVE_REPORT_PATH)
            # The group was checkpointed before execution so partial results survive.

            log(f"== {phase} / {profile} — doctests")
            doc = doctests(REPO, target, profile, args.timeout)
            if doc["exit"] != 0:
                report["failures"].append(f"{phase}/{profile}: doctests exited {doc['exit']}")
            if doc["ran_nothing"]:
                report["failures"].append(
                    f"{phase}/{profile}: doctests ran zero cases — RUN DID NOT HAPPEN"
                )
            report["runs"].append({"phase": phase, "profile": profile, "doctests": doc})
            save_report(report, _ACTIVE_REPORT_PATH)

    fingerprints_end = source_fingerprints()
    report["source_fingerprints_after"] = fingerprints_end
    if fingerprints_end != fingerprints_start:
        report["failures"].append(
            "source/test/harness fingerprints changed during the acceptance run"
        )
    model_cache_end = model_cache_fingerprint(model_cache_path)
    report["model_cache_metadata_sha256_after"] = model_cache_end
    if "semantic" in profiles and model_cache_end != model_cache_start:
        report["failures"].append("model cache changed during semantic acceptance")
    report["free_bytes_at_end"] = free_bytes(REPO)
    report["finished_at"] = time.time()
    report["result"] = "FAIL" if report["failures"] else "PASS"

    out_dir.mkdir(parents=True, exist_ok=True)
    save_report(report, _ACTIVE_REPORT_PATH)
    log(f"\nwrote {out_dir / 'result.json'}")
    for failure in report["failures"]:
        log(f"  FAIL: {failure}")
    return 1 if report["failures"] else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Failure as failure:
        if _ACTIVE_REPORT is not None and _ACTIVE_REPORT_PATH is not None:
            _ACTIVE_REPORT["failures"].append(str(failure))
            _ACTIVE_REPORT["result"] = "FAIL"
            _ACTIVE_REPORT["finished_at"] = time.time()
            try:
                _ACTIVE_REPORT["free_bytes_at_failure"] = free_bytes(REPO)
            except OSError:
                pass
            save_report(_ACTIVE_REPORT, _ACTIVE_REPORT_PATH)
        print(f"PREREQUISITE FAILURE: {failure}", file=sys.stderr)
        sys.exit(2)