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
    python3 eval/run_acceptance.py --phase all --suite workspace --profile all \\
        --out <new-output-directory> --include-source <new-unstaged-owned-path>
    `phase` is the default suite. `workspace` enumerates metadata library,
    binary and integration targets; test=false is compile-only NOT_APPLICABLE.
    New unstaged inputs outside the built-in writer list/metadata target paths
    must be passed with --include-source before the frozen run.

Exit code is 0 only when every executed group reported a real run and a real
pass. `--dry-run` performs metadata discovery only; it never builds or tests.
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
from typing import Optional

REPO = Path(__file__).resolve().parent.parent

# The CLI writes a checkpoint before risky work and updates it on failures.
_ACTIVE_REPORT_PATH: Path | None = None
_ACTIVE_REPORT: dict | None = None
_COMMAND_LOG_DIR: Optional[Path] = None
_COMMAND_NUMBER = 0
_EXPLICIT_SOURCE_PATHS: set[Path] = set()
_METADATA_SOURCE_PATHS: set[Path] = set()
_OWNED_NEW_PATHS = (
    "eval/constellation_readiness.py",
    "crates/forgeplan-core/tests/constellation_fault_process.rs",
    "crates/forgeplan-core/src/embed/tokenizer.rs",
    "crates/forgeplan-core/src/constellation/fault_barrier.rs",
    "crates/forgeplan-core/tests/constellation_tokenizer.rs",
    "crates/forgeplan-core/tests/constellation_section_windows.rs",
)
_CRATES = ("forgeplan-core", "forgeplan-cli", "forgeplan-mcp")
_SAFE_ENV_KEYS = (
    "RUSTUP_TOOLCHAIN", "CARGO_BUILD_JOBS", "CARGO_INCREMENTAL", "RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS", "CARGO_TARGET_DIR", "PROTOC",
    "DYLD_FALLBACK_LIBRARY_PATH", "FORGEPLAN_MODEL_CACHE",
    "FORGEPLAN_REQUIRE_MODEL_IN_TESTS",
)

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
                "constellation_tokenizer",
                "constellation_section_windows",
                "constellation_fault_process",
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
                "constellation_local_compat",
            ],
        },
        {
            "name": "cli-migrated-write",
            "package": "forgeplan",
            "tests": ["cli_migrated_write", "cli_constellation_local_compat"],
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
STOP_BELOW_FREE_BYTES = REQUIRED_MIN_FREE_BYTES



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

def _source_category(relative: str) -> Optional[str]:
    if relative in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
        return "production"
    if relative.startswith("eval/"):
        return "harness"
    for crate in _CRATES:
        prefix = f"crates/{crate}/"
        if not relative.startswith(prefix):
            continue
        local = relative[len(prefix):]
        if local in ("Cargo.toml", "build.rs") or local.startswith(("src/", "proto/")):
            return "production"
        if local.startswith(("tests/", "benches/", "examples/")):
            return "tests"
    return None


def source_inventory() -> dict[str, list[str]]:
    """Tracked relevant paths plus explicitly admitted unstaged source files."""
    tracked = subprocess.run(
        ["git", "ls-files", "-z", "--", "Cargo.toml", "Cargo.lock",
         "rust-toolchain.toml", "crates/forgeplan-core", "crates/forgeplan-cli",
         "crates/forgeplan-mcp", "eval"],
        cwd=str(REPO), capture_output=True, check=False,
    )
    if tracked.returncode:
        raise Failure("cannot obtain the exact tracked source inventory")
    paths = {os.fsdecode(path) for path in tracked.stdout.split(b"\0") if path}
    explicit = _EXPLICIT_SOURCE_PATHS | _METADATA_SOURCE_PATHS
    explicit |= {REPO / path for path in _OWNED_NEW_PATHS if (REPO / path).is_file()}
    for path in explicit:
        try:
            relative = path.absolute().relative_to(REPO).as_posix()
        except ValueError as exc:
            raise Failure(f"source path is outside the worktree: {path}") from exc
        if ".." in Path(relative).parts:
            raise Failure(f"source path contains parent traversal: {relative}")
        if _source_category(relative) is None:
            raise Failure(f"source path is outside the accepted roots: {relative}")
        paths.add(relative)
    result: dict[str, list[str]] = {"production": [], "tests": [], "harness": []}
    for relative in sorted(paths):
        category = _source_category(relative)
        if category is not None:
            if (REPO / relative).is_symlink():
                raise Failure(f"source inventory cannot follow a symlink: {relative}")
            candidate = REPO / relative
            if candidate.exists() and not candidate.resolve().is_relative_to(REPO.resolve()):
                raise Failure(f"source inventory escapes through a directory symlink: {relative}")
            result[category].append(relative)
    return result


def _digest_files(paths: set[Path], root: Path = REPO) -> str:
    digest = hashlib.sha256(b"forgeplan-source-inventory-v2\0")
    for path in sorted(paths):
        relative = path.relative_to(root).as_posix().encode()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        if not path.is_file():
            digest.update(b"missing\0")
            continue
        digest.update(b"file\0")
        digest.update(path.stat().st_size.to_bytes(8, "big"))
        with path.open("rb") as source:
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(chunk)
    return digest.hexdigest()


def source_fingerprints() -> dict[str, str]:
    inventory = source_inventory()
    result = {
        category: _digest_files({REPO / path for path in paths}, REPO)
        for category, paths in inventory.items()
    }
    result["cargo_lock"] = binary_sha256(str(REPO / "Cargo.lock"))
    result["path_inventory"] = hashlib.sha256(
        json.dumps(inventory, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    return result


def model_cache_inventory(path: Path) -> dict:
    """Hash actual model/tokenizer bytes; timestamps are not model identity."""
    result: dict = {"model": {}, "tokenizer": {}, "state": "missing"}
    if not path.is_dir():
        return result
    result["state"] = "present"
    for directory, dirs, files in os.walk(path, followlinks=False):
        dirs[:] = sorted(name for name in dirs if not (Path(directory) / name).is_symlink())
        for name in sorted(files):
            item = Path(directory) / name
            if not item.is_file():
                continue
            if not item.resolve().is_relative_to(path.resolve()):
                raise Failure(f"model cache file points outside the supplied cache: {item.name}")
            metadata_files = ("tokenizer.json", "tokenizer_config.json", "config.json", "special_tokens_map.json")
            if name not in metadata_files and item.suffix not in (".onnx", ".bin", ".safetensors") and not name.endswith(".onnx_data") and Path(directory).name != "refs":
                continue
            category = "tokenizer" if name in metadata_files else "model"
            result[category][item.relative_to(path).as_posix()] = {
                "sha256": binary_sha256(str(item)), "bytes": item.stat().st_size,
            }
    return result


def model_cache_fingerprint(path: Path) -> str:
    return hashlib.sha256(
        json.dumps(model_cache_inventory(path), sort_keys=True).encode()
    ).hexdigest()


def freeze_source_patch(path: Path) -> dict:
    """Write the actual source delta, including explicitly admitted new files."""
    paths = sorted(relative for group in source_inventory().values() for relative in group)
    tracked = subprocess.run(["git", "ls-files", "-z", "--", *paths],
                             cwd=str(REPO), capture_output=True, check=False)
    if tracked.returncode:
        raise Failure("cannot enumerate tracked patch inputs")
    tracked_paths = {os.fsdecode(item) for item in tracked.stdout.split(b"\0") if item}
    delta = subprocess.run(["git", "diff", "--binary", "HEAD", "--", *paths],
                           cwd=str(REPO), capture_output=True, check=False)
    if delta.returncode:
        raise Failure("cannot capture the tracked source patch")
    chunks = [delta.stdout]
    for relative in paths:
        if relative in tracked_paths:
            continue
        if not (REPO / relative).is_file():
            raise Failure(f"new frozen source file is missing: {relative}")
        added = subprocess.run(["git", "diff", "--binary", "--no-index", "--", os.devnull, relative],
                               cwd=str(REPO), capture_output=True, check=False)
        if added.returncode not in (0, 1):
            raise Failure(f"cannot capture new source patch: {relative}")
        chunks.append(added.stdout)
    path.write_bytes(b"".join(chunks))
    return {"path": str(path), "sha256": binary_sha256(str(path)), "bytes": path.stat().st_size}


def disk_snapshot(path: Path) -> dict:
    proc = subprocess.run(
        ["df", "-h", str(path)], capture_output=True, text=True, check=False,
    )
    return {"command": ["df", "-h", str(path)], "exit": proc.returncode,
            "stdout": proc.stdout, "stderr": proc.stderr, "free_bytes": free_bytes(path)}


def command_logs(cmd: list[str], stdout: str, stderr: str) -> dict:
    global _COMMAND_NUMBER
    if _COMMAND_LOG_DIR is None:
        return {}
    _COMMAND_NUMBER += 1
    result = {}
    for stream, text in (("stdout", stdout), ("stderr", stderr)):
        path = _COMMAND_LOG_DIR / f"{_COMMAND_NUMBER:05d}.{stream}.log"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        result[stream] = {"path": str(path), "sha256": binary_sha256(str(path)),
                          "bytes": path.stat().st_size}
    return result


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
    logs: Optional[dict] = None
    elapsed_s: float = 0.0
    disk_before: Optional[dict] = None
    disk_after: Optional[dict] = None
    disk_stopped: bool = False


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
    disk_stopped = False
    started = time.monotonic()
    disk_before = disk_snapshot(REPO)
    while True:
        remaining = timeout - (time.monotonic() - started)
        try:
            stdout, stderr = proc.communicate(timeout=max(0.01, min(1.0, remaining)))
            break
        except subprocess.TimeoutExpired:
            disk_stopped = free_bytes(REPO) < STOP_BELOW_FREE_BYTES
            timed_out = remaining <= 1.0
            if disk_stopped or timed_out:
                _terminate_process_tree(proc)
                stdout, stderr = proc.communicate()
                stderr += f"\n[executor] stopped: timeout={timed_out}, disk={disk_stopped}"
                break
    logs = command_logs(cmd, stdout, stderr)
    after = source_fingerprints()
    if _COMMAND_LOG_DIR is not None:
        with (_COMMAND_LOG_DIR / "commands.jsonl").open("a", encoding="utf-8") as stream:
            stream.write(json.dumps({
                "command": cmd, "cwd": str(cwd), "exit": proc.returncode,
                "timed_out": timed_out, "disk_stopped": disk_stopped,
                "elapsed_s": time.monotonic() - started, "logs": logs,
                "source_fingerprints_before": before,
                "source_fingerprints_after": after,
                "environment": {key: os.environ[key] for key in _SAFE_ENV_KEYS if key in os.environ},
            }) + "\n")
    if before != after:
        raise Failure(
            f"source/test/harness files changed while command ran: {' '.join(cmd)}"
        )
    return CommandResult(
        returncode=proc.returncode,
        stdout=stdout,
        stderr=stderr,
        source_fingerprints_before=before,
        source_fingerprints_after=after,
        timed_out=timed_out,
        logs=logs,
        elapsed_s=time.monotonic() - started,
        disk_before=disk_before,
        disk_after=disk_snapshot(REPO),
        disk_stopped=disk_stopped,
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
    if proc.returncode != 0 or proc.timed_out or proc.disk_stopped:
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


def workspace_groups(metadata: dict) -> tuple[list[dict], list[dict]]:
    """Enumerate each workspace library, binary and integration target once."""
    members = set(metadata.get("workspace_members", []))
    groups: list[dict] = []
    compile_only: list[dict] = []
    for package in sorted(metadata["packages"], key=lambda item: item["name"]):
        if members and package["id"] not in members:
            continue
        for target in sorted(package["targets"], key=lambda item: (item["kind"], item["name"])):
            kinds = set(target["kind"])
            kind = next((kind for kind in ("test", "lib", "rlib", "dylib", "staticlib", "cdylib", "proc-macro", "bin") if kind in kinds), None)
            if kind is None:
                continue
            key = f"{package['name']}:{kind}:{target['name']}"
            group = {
                "name": key, "package": package["name"],
                "required_features": target.get("required-features", []) or [],
                "expected_targets": [(kind, target["name"])],
                "target_key": key, "source": target["src_path"],
            }
            if kind == "test":
                group["tests"] = [target["name"]]
            elif kind in ("lib", "rlib", "dylib", "staticlib", "cdylib", "proc-macro"):
                group["lib"] = True
            else:
                group["bin_names"] = [target["name"]]
            if target.get("test", True) is False:
                compile_only.append({**group, "status": "NOT_APPLICABLE",
                                     "reason": "Cargo metadata declares test=false; checked, not executed"})
            else:
                groups.append(group)
    if not groups:
        raise Failure("workspace metadata contains no executable test/library/bin inventory")
    keys = [group["target_key"] for group in groups + compile_only]
    if len(keys) != len(set(keys)):
        raise Failure("duplicate target in workspace metadata inventory")
    return groups, compile_only


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
        if group.get("target_key"):
            enabled = set(PROFILE_FEATURES[profile].get(group["package"], []))
            missing = set(group.get("required_features", [])).difference(enabled)
            if not missing:
                runnable.append(group)
            elif profile == "no-semantic" and missing == {"semantic-search"}:
                not_applicable.append(group["target_key"])
            else:
                blocked.append(f"{group['target_key']} (requires {sorted(missing)})")
            continue
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
        **command_evidence(proc, cmd, manifest_dir),
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


def command_evidence(proc: CommandResult, command: list[str], cwd: Path) -> dict:
    return {
        "command": command, "cwd": str(cwd), "exit": proc.returncode,
        "timed_out": proc.timed_out, "disk_stopped": proc.disk_stopped,
        "elapsed_s": proc.elapsed_s, "logs": proc.logs,
        "environment": {key: os.environ[key] for key in _SAFE_ENV_KEYS if key in os.environ},
        "disk_before": proc.disk_before, "disk_after": proc.disk_after,
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
    for name in group.get("bin_names", []):
        argv.extend(["--bin", name])
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
        and bool(set(target.get("kind", [])) & {"test", "lib", "rlib", "dylib", "staticlib", "cdylib", "proc-macro", "bin"})
    )


def compiler_inventory(stdout: str, expected: set[tuple[str, str]]) -> tuple[list[str], set[tuple[str, str]]]:
    executables: set[str] = set()
    observed: set[tuple[str, str]] = set()
    for line in stdout.splitlines():
        if not line.lstrip().startswith("{"):
            continue
        try:
            message = json.loads(line)
        except json.JSONDecodeError as error:
            raise Failure("malformed Cargo compiler JSON") from error
        if not isinstance(message, dict):
            raise Failure("Cargo compiler record must be an object")
        if not is_test_executable_artifact(message):
            continue
        path = message.get("executable")
        if not isinstance(path, str) or not path:
            continue
        executables.add(path)
        target = message["target"]
        observed.update((kind, target["name"]) for kind in target["kind"])
    if not executables:
        raise Failure("no test executable was produced; RUN DID NOT HAPPEN")
    if expected.difference(observed):
        raise Failure(f"compiler omitted required targets {sorted(expected.difference(observed))}")
    return sorted(executables), observed


def required_phase_groups(phase: str) -> list[dict]:
    groups = PHASE_GROUPS.get(phase) or []
    if not groups:
        raise Failure(f"{phase}: no acceptance suites are declared")
    return groups


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
    if proc.returncode != 0 or proc.timed_out or proc.disk_stopped:
        raise Failure(
            f"link failed for {group['name']} (exit {proc.returncode}):\n"
            f"{proc.stderr[-8000:]}\n{proc.stdout[-4000:]}"
        )

    expected = {tuple(target) for target in group.get("expected_targets", [])}
    if not expected:
        expected = {("test", name) for name in group.get("tests", [])}
    executables, observed = compiler_inventory(proc.stdout, expected)
    hashes: dict[str, str | None] = {}
    for path in executables:
        try:
            hashes[path] = binary_sha256(path)
        except OSError as error:
            raise Failure(f"compiler-reported executable is missing/unreadable: {path}") from error
    support_binaries = {}
    for line in proc.stdout.splitlines():
        if not line.lstrip().startswith("{"):
            continue
        message = json.loads(line)
        executable = message.get("executable")
        if (message.get("reason") == "compiler-artifact" and executable
                and "bin" in message.get("target", {}).get("kind", [])
                and not message.get("profile", {}).get("test")):
            support_binaries[executable] = binary_sha256(executable)
    return {
        "command": argv,
        **command_evidence(proc, argv, manifest_dir),
        "observed_targets": sorted(observed),
        "free_before": before,
        "free_after": after,
        "stdout_tail": proc.stdout[-8000:],
        "stderr_tail": proc.stderr[-8000:],
        "timed_out": proc.timed_out,
        "executables": executables,
        "executable_sha256": hashes,
        "support_binary_sha256": support_binaries,
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
    """Record the actual process exit; ignored/zero cases never imply PASS."""
    try:
        binary_before = binary_sha256(executable)
    except OSError as error:
        raise Failure(f"test executable unavailable; RUN DID NOT HAPPEN: {executable}") from error
    before = free_bytes(REPO)
    proc = run([executable, "--test-threads=1", "--nocapture"], REPO, timeout)
    counts = parse_test_counts(proc.stdout)
    binary_after = binary_sha256(executable)
    return {
        **command_evidence(proc, [executable, "--test-threads=1", "--nocapture"], REPO),
        "executable": executable, "counts": counts,
        "ran_nothing": counts["passed"] + counts["failed"] == 0,
        "free_before": before, "free_after": free_bytes(REPO),
        "stdout_tail": proc.stdout[-8000:], "stderr_tail": proc.stderr[-4000:],
        "source_changed": proc.source_fingerprints_before != proc.source_fingerprints_after,
        "binary_sha256_before": binary_before, "binary_sha256_after": binary_after,
        "binary_changed": binary_before != binary_after,
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
    cmd.extend(["--", "--test-threads=1"])
    before = free_bytes(REPO)
    proc = run(cmd, manifest_dir, timeout)
    counts = parse_test_counts(proc.stdout)
    ran_nothing = counts["passed"] == 0 and counts["failed"] == 0
    return {
        "command": cmd,
        **command_evidence(proc, cmd, manifest_dir),
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
    global _COMMAND_LOG_DIR, _COMMAND_NUMBER, _EXPLICIT_SOURCE_PATHS, _METADATA_SOURCE_PATHS
    global _ACTIVE_REPORT, _ACTIVE_REPORT_PATH
    _ACTIVE_REPORT = None
    _ACTIVE_REPORT_PATH = None
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--phase", default="all", choices=["s1", "s2", "s3", "s4", "all"])
    parser.add_argument("--suite", default="phase", choices=["phase", "workspace"])
    parser.add_argument("--include-source", action="append", default=[],
                        help="explicit new unstaged relevant source/test/harness path; repeatable")
    parser.add_argument("--patch", type=Path, help="owned/integrated patch to fingerprint")
    parser.add_argument("--profile", default="no-semantic", choices=["no-semantic", "semantic", "all"])
    parser.add_argument("--out", required=True, help="directory for result.json and logs")
    parser.add_argument("--model-cache", default=str(Path.home() / ".forgeplan" / "models"))
    parser.add_argument("--target-dir", default=None)
    parser.add_argument("--timeout", type=int, default=3300, help="per-command wall clock bound")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    _COMMAND_NUMBER = 0
    _COMMAND_LOG_DIR = None if args.dry_run else Path(args.out).expanduser().resolve() / "logs"
    _EXPLICIT_SOURCE_PATHS = {
        (REPO / path).absolute() for path in args.include_source
    }
    _METADATA_SOURCE_PATHS = set()

    out_dir = Path(args.out).expanduser().resolve()
    if not args.dry_run and out_dir.exists() and any(out_dir.iterdir()):
        raise Failure("output directory must be new or empty; preserving previous receipts")
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
    all_groups, compile_only = workspace_groups(metadata) if args.suite == "workspace" else ([], [])
    _METADATA_SOURCE_PATHS = {
        Path(target["src_path"]).absolute()
        for package in metadata["packages"]
        for target in package["targets"]
        if _source_category(os.path.relpath(target["src_path"], REPO)) is not None
    }
    declared = declared_test_targets(metadata)
    target = target_dir(metadata, args.target_dir, create=not args.dry_run)
    fingerprints_start = source_fingerprints()
    model_cache_path = Path(args.model_cache).expanduser()
    model_cache_start = model_cache_inventory(model_cache_path)

    profiles = ["no-semantic", "semantic"] if args.profile == "all" else [args.profile]
    phases = ["s1", "s2", "s3", "s4"] if args.phase == "all" else [args.phase]
    requested_phases = list(phases)
    if args.suite == "workspace":
        phases = ["workspace"]

    report = _ACTIVE_REPORT if _ACTIVE_REPORT is not None else {}
    report.update(
        {
            "schema_version": 1,
            "suite": args.suite,
            "source_inventory": source_inventory(),
            "model_cache_before": model_cache_start,
            "disk_before": disk_snapshot(REPO),
            "patch": ({"path": str(args.patch), "sha256": binary_sha256(str(args.patch)),
                       "bytes": args.patch.stat().st_size} if args.patch else None),
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
                    "model_tokenizer_inventory": model_cache_start,
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
    if not args.dry_run and args.patch is None:
        report["patch"] = freeze_source_patch(out_dir / "source.patch")
    if not args.dry_run:
        save_report(report, _ACTIVE_REPORT_PATH)

    if args.dry_run:
        report["dry_run"] = True
        report["declared_test_targets"] = sorted(declared)
        log(json.dumps(report["declared_test_targets"], indent=2))
        return 0

    if args.suite == "workspace":
        for requested in requested_phases:
            try:
                required = required_phase_groups(requested)
            except Failure as error:
                report["failures"].append(str(error))
                continue
            absent = missing_targets(required, declared)
            if absent:
                report["failures"].append(f"{requested}: declared required targets missing from metadata: {absent}")

    for command in (["git", "rev-parse", "HEAD"], ["rustc", "-Vv"], ["cargo", "-V"], ["cc", "--version"]):
        observed = run(command, REPO, min(args.timeout, 60))
        report.setdefault("toolchain", []).append({
            **command_evidence(observed, command, REPO),
            "stdout": observed.stdout, "stderr": observed.stderr,
        })
        if observed.returncode != 0:
            report["failures"].append(f"toolchain prerequisite failed: {command}")

    out_dir.mkdir(parents=True, exist_ok=True)

    for profile in profiles:
        completed_targets: set[str] = set()
        expected_targets: set[str] = set()
        if profile == "semantic":
            os.environ["FORGEPLAN_REQUIRE_MODEL_IN_TESTS"] = "1"
        else:
            os.environ.pop("FORGEPLAN_REQUIRE_MODEL_IN_TESTS", None)
        for phase in phases:
            try:
                groups = all_groups if phase == "workspace" else required_phase_groups(phase)
            except Failure as error:
                report["failures"].append(f"{phase}/{profile}: {error}")
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
            for group in available_groups:
                expected_targets.add(group.get("target_key", group["name"]))
            report.setdefault("inventory", []).extend(
                {"phase": phase, "profile": profile, "target": group.get("target_key", group["name"]),
                 "status": "NOT_RUN", "selectors": test_selection_argv(group)}
                for group in available_groups
            )
            report["inventory"].extend(
                {"phase": phase, "profile": profile, "target": name,
                 "status": "NOT_APPLICABLE",
                 "reason": "semantic-search intentionally absent in no-semantic profile"}
                for name in not_applicable
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
            if check["exit"] != 0 or check["timed_out"] or check.get("disk_stopped"):
                report["failures"].append(f"{phase}/{profile}: cargo check failed")
                save_report(report, _ACTIVE_REPORT_PATH)
                continue
            if phase == "workspace":
                for item in compile_only:
                    report["inventory"].append({**item, "phase": phase, "profile": profile,
                                                "check_exit": check["exit"]})

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
                    "phase": phase,
                    "build_evidence": build,
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
                    if result["counts"]["failed"]:
                        report["failures"].append(f"{group['name']}: test summary contains failures")
                    if result["counts"]["ignored"]:
                        report["failures"].append(
                            f"{group['name']}: {result['counts']['ignored']} ignored mandatory tests"
                        )
                    if result["timed_out"] or result.get("disk_stopped"):
                        report["failures"].append(f"{group['name']}: execution interrupted")
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
                support_after = {path: binary_sha256(path) for path in build["support_binary_sha256"]}
                group_result["support_binary_sha256_before"] = build["support_binary_sha256"]
                group_result["support_binary_sha256_after"] = support_after
                group_result["support_binary_changed"] = support_after != build["support_binary_sha256"]
                if group_result["support_binary_changed"]:
                    report["failures"].append(f"{group['name']}: supporting CLI/MCP binary changed during execution")
                key = group.get("target_key", group["name"])
                completed_targets.add(key)
                for item in report["inventory"]:
                    if item["phase"] == phase and item["profile"] == profile and item["target"] == key:
                        item["status"] = "FAIL" if group_result["support_binary_changed"] or any(
                            result["exit"] != 0 or result["ran_nothing"]
                            or result["counts"]["failed"] or result["counts"]["ignored"]
                            or result["timed_out"] or result.get("disk_stopped")
                            or result["source_changed"] or result["binary_changed"]
                            for result in group_result["executables"]
                        ) else "PASS"
            # The group was checkpointed before execution so partial results survive.

            log(f"== {phase} / {profile} — doctests")
            doc = doctests(REPO, target, profile, args.timeout)
            if doc["exit"] != 0 or doc["counts"]["failed"] or doc["timed_out"] or doc.get("disk_stopped"):
                report["failures"].append(f"{phase}/{profile}: doctests exited {doc['exit']}")
            if doc["ran_nothing"]:
                report["failures"].append(
                    f"{phase}/{profile}: doctests ran zero cases — RUN DID NOT HAPPEN"
                )
            report["runs"].append({"phase": phase, "profile": profile, "doctests": doc})
            if doc["counts"]["ignored"]:
                report["failures"].append(f"{phase}/{profile}: ignored mandatory doctests")
        missing_inventory = sorted(expected_targets.difference(completed_targets))
        if missing_inventory:
            report["failures"].append(f"{profile}: inventory targets not executed: {missing_inventory}")
            save_report(report, _ACTIVE_REPORT_PATH)

    fingerprints_end = source_fingerprints()
    report["source_fingerprints_after"] = fingerprints_end
    if fingerprints_end != fingerprints_start:
        report["failures"].append(
            "source/test/harness fingerprints changed during the acceptance run"
        )
    model_cache_end = model_cache_inventory(model_cache_path)
    report["model_cache_after"] = model_cache_end
    if model_cache_end != model_cache_start:
        report["failures"].append("model/tokenizer bytes changed during acceptance")
    report["free_bytes_at_end"] = free_bytes(REPO)
    if report["free_bytes_at_end"] < STOP_BELOW_FREE_BYTES:
        report["failures"].append("disk stop threshold reached during acceptance")
    report["disk_after"] = disk_snapshot(REPO)
    report["executed_binary_count"] = sum(
        len(run.get("executables", [])) for run in report["runs"]
    )
    report["counts"] = {
        label: sum(result["counts"][label] for run in report["runs"]
                   for result in run.get("executables", []))
        for label in ("passed", "failed", "ignored")
    }
    if not report["executed_binary_count"] or not report["counts"]["passed"]:
        report["failures"].append("RUN DID NOT HAPPEN: zero binaries or zero passed cases")
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
                _ACTIVE_REPORT["disk_after"] = disk_snapshot(REPO)
                _ACTIVE_REPORT["source_fingerprints_after"] = source_fingerprints()
            except OSError:
                pass
            save_report(_ACTIVE_REPORT, _ACTIVE_REPORT_PATH)
        print(f"PREREQUISITE FAILURE: {failure}", file=sys.stderr)
        sys.exit(2)