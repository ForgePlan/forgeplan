#!/usr/bin/env python3
"""S1 real CLI/stdio/process acceptance (stdlib, Python >=3.9).

Run already-built binaries; this program never builds, downloads, installs or
replaces a binary. The former 25/42/28 probes are historical input trace only.
Later phases fail closed until their separately approved cases are installed.
A helper CallGate stall is not an ONNX hang or hardware power-loss observation.

Use --phase s1 for this admitted stage. --phase all fails closed on the later,
unadmitted stages; PASS is scoped to the selected feature profile, not S1-G.
Mandatory S1 inputs beyond the preserved production --binary/--standalone are
--acceptance-report (same-fingerprint sole-owner Rust receipt), --helper-binary
(a distinct preserved test-helpers CLI), and --reason-config (public llm
provider/model/deadline scalars only). Missing inputs/model files are BLOCKED,
never skipped PASS. The reason process alone inherits stock vendor-login
HOME/CODEX_HOME; temporary workspace/config stay isolated and no auth is copied.
Run a separate --profile no-semantic receipt with preserved no-feature binaries.
The output directory must be new/empty. Source inventory admits tracked relevant
roots plus the known new files and repeated --include-source paths, never a
recursive sweep of another checkout. Read-only FPF KB operations are independent
of CURRENT; missing KB differs from actual auxiliary-open/data-query failures.
"""
from __future__ import annotations

import argparse
import asyncio
from contextlib import asynccontextmanager
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import stat
import tempfile
import time
from typing import Any, AsyncIterator, Awaitable, Callable, Optional
import uuid

import run_acceptance as inventory

PUBLICATION_STAGES = (
    "after_artifacts_commit", "after_relations_commit", "after_sections_commit",
    "after_storage_sync", "before_manifest_sync", "after_manifest_sync",
    "before_current_replace", "after_current_replace", "after_current_ack",
)
BARRIER_STAGES = PUBLICATION_STAGES + ("owner_acquired", "native_call_entered")
ORDINARY = (
    ("health", ["health", "--json"], {}),
    ("anomalies", ["anomalies", "--json"], {}),
    ("journal", ["journal"], {}),
    ("blindspots", ["blindspots"], {}),
    ("decay", ["decay"], {}),
    ("fpf_search", ["fpf", "search", "proofknowledge"], {"query": "proofknowledge"}),
    ("fpf_section", ["fpf", "section", "B.3"], {"id": "B.3"}),
    ("fpf_list", ["fpf", "list"], {}),
    ("fpf_check", ["fpf", "check", "ADR-901", "--json"], {"id": "ADR-901"}),
    ("drift", ["drift"], {}),
    ("coverage", ["coverage", "--json"], {}),
    ("release_notes", ["release-notes", "--since", "HEAD~1", "--until", "HEAD", "--draft", "--output", "json"],
     {"since": "HEAD~1", "until": "HEAD", "draft": True}),
)
# Each row is a required observation, never a count inferred from files/tests.
RUST_OBSERVATIONS = {
    "L09.pinned_generation_faults": ("constellation_fault_process", "sigkill_at_each_publication_stage_reopens_one_pinned_triple"),
    "L10.owner_takeover": ("constellation_fault_process", "owner_death_allows_real_takeover_without_double_publisher"),
    "L10.workspace_lock": ("constellation_fault_process", "workspace_lock_holder_can_release_while_publisher_waits"),
    "L10.durable_pending": ("constellation_fault_process", "durable_pending_survives_sigkill_and_reconstructs_actual_inputs"),
    "L12.metadata_cache_cycle": ("constellation_fault_process", "metadata_absent_present_absent_publishes_exact_section_sets"),
    "L12.tokenizer_metadata": ("constellation_tokenizer", None),
    "L12.exact_multilingual_windows": ("constellation_section_windows", None),
    "S1-R1.phase_and_health_values": ("forgeplan_core-", "captured_phase_report_keeps_mismatches_read_errors_and_stale_drafts"),
    "S1-R1.decay_evidence_direction": ("forgeplan_core-", "captured_decay_counts_a_pack_from_either_direction"),
    "L11.actual_reverse_completion": ("constellation_fault_process", "actual_c_a_b_completions_and_recreate_do_not_publish_old_results"),
    "L15.exact_prefilter_and_all_stale": ("constellation_fault_process", "actual_pinned_vectors_prefilter_stale_rows_before_limit"),
    "L07.control_scan_without_notifications": ("constellation_fault_process", "missed_notifications_and_suspended_control_timer_recover_bytes_and_config"),
    "S1-R3.core_native_helper": ("constellation_fault_process", "native_callgate_entry_helper_exits_on_eof_without_claiming_native_return"),
}


class ProofFailure(Exception):
    """An observed failure, not an acceptable missing/empty result."""


class ProofBlocked(ProofFailure):
    """A named prerequisite is absent; mandatory work is not counted as PASS."""


def prerequisite(condition: bool, detail: str) -> None:
    if not condition:
        raise ProofBlocked(detail)


def require_cached_model(models: dict) -> None:
    """Refuse incomplete BGE-M3 cache candidates before any ingest can fetch."""
    files = {**models["model"], **models["tokenizer"]}
    candidates = sorted({name.rsplit("/", 1)[0] for name in models["tokenizer"]
                         if name.startswith("models--BAAI--bge-m3/snapshots/") and name.endswith("/tokenizer.json")})
    prerequisite(bool(candidates), "real ONNX model is absent from the explicit cache; no model download or ambient-cache fallback")
    required = ("onnx/model.onnx", "onnx/model.onnx_data", "tokenizer.json",
                "tokenizer_config.json", "config.json", "special_tokens_map.json")
    for snapshot in candidates:
        missing = [name for name in required if files.get(snapshot + "/" + name, {}).get("bytes", 0) <= 0]
        prerequisite(not missing, f"real ONNX cache candidate {snapshot} is incomplete: {missing}; no download attempted")


def require(condition: bool, detail: Any) -> None:
    if not condition:
        raise ProofFailure(str(detail))


def reason_config(path: Path) -> dict[str, Any]:
    """Accept only public provider/model/deadline scalars, never auth material."""
    text = path.read_text(encoding="utf-8")
    if text.lstrip().startswith("{"):
        value = json.loads(text)
        require(set(value) == {"llm"} and isinstance(value["llm"], dict), "reason config must contain only llm")
        fields = value["llm"]
    else:
        fields = {}
        in_llm = False
        for line in text.splitlines():
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            if line == "llm:":
                require(not in_llm, "duplicate llm config")
                in_llm = True
                continue
            require(in_llm and line.startswith("  ") and not line.startswith("    "),
                    "reason config only permits a two-space llm scalar map")
            key, separator, raw = line.strip().partition(":")
            require(bool(separator) and key not in fields, "duplicate or invalid reason field")
            raw = raw.strip()
            if raw.startswith('"'):
                value = json.loads(raw)
            elif raw.startswith("'") and raw.endswith("'"):
                value = raw[1:-1].replace("''", "'")
            elif raw.isdigit():
                value = int(raw)
            else:
                require(bool(re.fullmatch(r"[a-zA-Z0-9_.:/-]*", raw)), "reason scalar requires explicit quoting")
                value = raw
            fields[key] = value
    allowed = {"provider", "model", "timeout_seconds", "reason_timeout_seconds"}
    require(set(fields).issubset(allowed) and "provider" in fields, "non-public reason config fields refused")
    require(fields["provider"] in ("codex-cli", "claude-code", "gemini-cli", "devin-cli", "cursor-agent"),
            "this fixture uses an existing stock CLI login, not copied credentials")
    require(isinstance(fields.get("model", ""), str), "model must be a string")
    for key in ("timeout_seconds", "reason_timeout_seconds"):
        if key in fields:
            require(type(fields[key]) is int and 0 < fields[key] <= 3600, "invalid reason deadline")
    return fields



def active_native_call(text: str) -> Optional[tuple[str, str]]:
    """Find a query entry with no matching return in the observed trace."""
    active: dict[str, str] = {}
    for line in text.splitlines():
        line = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", line)
        match = re.search(r"\bcall_id[=:]\s*(\d+)", line)
        if match is None:
            continue
        call_id = match.group(1)
        if re.search(r'\bevent[=:]\s*"?constellation_native_encode_entered"?', line) and re.search(r'\bop[=:]\s*"?query"?', line):
            active[call_id] = line
        elif re.search(r'\bevent[=:]\s*"?constellation_native_encode_returned"?', line):
            active.pop(call_id, None)
    return next(reversed(active.items()), None) if active else None


async def native_entry(path: Path, process: asyncio.subprocess.Process, seconds: float) -> tuple[str, str]:
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        text = path.read_text(encoding="utf-8")
        entered = active_native_call(text)
        if entered is not None:
            return entered
        require(process.returncode is None, "production process exited before native query entry")
        # Observe an actual trace, never infer the stage from a timed delay.
        await asyncio.sleep(0.005)
    raise ProofFailure("production native query entry not observed before the deadline")

def payload(response: dict) -> Any:
    require("error" not in response, response)
    result = response.get("result")
    require(isinstance(result, dict) and not result.get("isError", False), response)
    if "structuredContent" in result:
        return result["structuredContent"]
    texts = [block["text"] for block in result.get("content", []) if block.get("type") == "text"]
    require(bool(texts), "MCP tool returned no content")
    return json.loads("\n".join(texts))


def explicit_failure(response: dict, codes: tuple[str, ...]) -> None:
    failed = "error" in response or response.get("result", {}).get("isError") is True
    text = json.dumps(response, ensure_ascii=False).lower().replace("_", "")
    require(failed and any(code.lower().replace("_", "") in text for code in codes), response)


def fixture_tree(root: Path, *, canonical_only: bool = False) -> dict[str, str]:
    """Only inspect a harness-created fixture, never the checkout/foreign roots."""
    result: dict[str, str] = {}
    for directory, dirs, files in os.walk(root, followlinks=False):
        if canonical_only:
            dirs[:] = [name for name in dirs if name not in ("lance", ".git", "logs", "journal", "state")]
        for name in sorted(files):
            path = Path(directory) / name
            if path.is_symlink():
                result[path.relative_to(root).as_posix()] = "symlink:" + os.readlink(path)
            elif path.is_file():
                result[path.relative_to(root).as_posix()] = inventory.binary_sha256(str(path))
    return result


class Mcp:
    def __init__(self, proof: Proof, process: asyncio.subprocess.Process, label: str) -> None:
        self.proof, self.process, self.label = proof, process, label
        self.pending: dict[int, asyncio.Future] = {}
        self.serial = 0
        self.reader = asyncio.create_task(self.read())
        self.exited_intentionally = False

    async def send(self, message: dict) -> None:
        self.proof.event("mcp_send", label=self.label, pid=self.process.pid, message=message)
        require(self.process.stdin is not None, "stdin unavailable")
        self.process.stdin.write(json.dumps(message).encode() + b"\n")
        await self.process.stdin.drain()

    async def read(self) -> None:
        error: Exception = ProofFailure("MCP stdout closed before a response")
        try:
            require(self.process.stdout is not None, "stdout unavailable")
            while True:
                line = await self.process.stdout.readline()
                if not line:
                    break
                message = json.loads(line)
                self.proof.event("mcp_receive", label=self.label, pid=self.process.pid, message=message)
                if "id" in message and ("result" in message or "error" in message):
                    future = self.pending.pop(message["id"], None)
                    if future is not None and not future.done():
                        future.set_result(message)
                elif "id" in message:
                    await self.send({"jsonrpc": "2.0", "id": message["id"],
                                     "error": {"code": -32601, "message": "No client callbacks"}})
        except (ValueError, OSError, ProofFailure) as caught:
            error = caught
        finally:
            for future in self.pending.values():
                if not future.done():
                    future.set_exception(error)

    async def rpc(self, method: str, params: dict, timeout: Optional[float] = None) -> dict:
        self.serial += 1
        request_id = self.serial
        future = asyncio.get_running_loop().create_future()
        self.pending[request_id] = future
        try:
            await self.send({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params})
            return await asyncio.wait_for(future, timeout or self.proof.timeout)
        finally:
            self.pending.pop(request_id, None)

    async def tool(self, name: str, arguments: dict) -> Any:
        return payload(await self.rpc("tools/call", {"name": name, "arguments": arguments}))

    async def eof(self) -> dict:
        require(self.process.stdin is not None, "stdin missing")
        started = time.monotonic()
        self.process.stdin.close()
        try:
            code = await asyncio.wait_for(self.process.wait(), self.proof.shutdown_bound)
        except asyncio.TimeoutError as error:
            self.process.kill()
            await self.process.wait()
            self.exited_intentionally = True
            self.proof.event("mcp_exit", label=self.label, pid=self.process.pid,
                             exit=self.process.returncode, emergency_kill=True,
                             elapsed_s=time.monotonic() - started)
            raise ProofFailure("MCP did not exit within the configured EOF bound") from error
        self.exited_intentionally = True
        observed = {"exit": code, "elapsed_s": time.monotonic() - started,
                    "bound_s": self.proof.shutdown_bound, "emergency_kill": False}
        self.proof.event("mcp_exit", label=self.label, pid=self.process.pid, **observed)
        require(code == 0, observed)
        return observed


class Barrier:
    def __init__(self, stage: str) -> None:
        require(stage in BARRIER_STAGES, stage)
        self.stage, self.token = stage, uuid.uuid4().hex
        self.server: Optional[asyncio.AbstractServer] = None
        self.connection: Optional[asyncio.Future] = None

    async def __aenter__(self) -> Barrier:
        self.connection = asyncio.get_running_loop().create_future()

        def accepted(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
            if self.connection is not None and not self.connection.done():
                self.connection.set_result((reader, writer))
            else:
                writer.close()

        self.server = await asyncio.start_server(accepted, "127.0.0.1", 0)
        return self

    def environment(self) -> dict[str, str]:
        require(self.server is not None and bool(self.server.sockets), "barrier not listening")
        port = self.server.sockets[0].getsockname()[1]
        return {"FORGEPLAN_TEST_BARRIER_ENDPOINT": f"127.0.0.1:{port}",
                "FORGEPLAN_TEST_BARRIER_TOKEN": self.token,
                "FORGEPLAN_TEST_BARRIER_STAGE": self.stage,
                "FORGEPLAN_TEST_BARRIER_ACK_MS": "180000"}

    async def reached(self, pid: int, seconds: float) -> tuple[dict, asyncio.StreamWriter]:
        require(self.connection is not None, "barrier not opened")
        reader, writer = await asyncio.wait_for(self.connection, seconds)
        frame = await asyncio.wait_for(reader.readline(), 5)
        message = json.loads(frame)
        require(message == {"token": self.token, "stage": self.stage, "pid": pid}, message)
        return message, writer

    async def __aexit__(self, *exc: Any) -> None:
        if self.connection is not None and self.connection.done() and not self.connection.cancelled():
            _, writer = self.connection.result()
            writer.close()
            await writer.wait_closed()
        if self.server is not None:
            self.server.close()
            await self.server.wait_closed()


class Proof:
    def __init__(self, args: argparse.Namespace, root: Path) -> None:
        self.args, self.root = args, root
        self.out = args.out.resolve()
        self.out.mkdir(parents=True, exist_ok=True)
        self.timeout, self.shutdown_bound = args.timeout, args.shutdown_bound
        self.home = root / "isolated-home"
        self.home.mkdir()
        self.env = dict(os.environ)
        for key in tuple(self.env):
            if key.startswith("FORGEPLAN_") or key in ("HF_HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME"):
                self.env.pop(key)
        self.env.update(HOME=str(self.home), USERPROFILE=str(self.home),
                        XDG_CONFIG_HOME=str(self.home / "config"),
                        XDG_DATA_HOME=str(self.home / "data"), XDG_CACHE_HOME=str(self.home / "cache"),
                        FORGEPLAN_MODEL_CACHE=str(args.model_cache), HF_HUB_OFFLINE="1",
                        TRANSFORMERS_OFFLINE="1", NO_COLOR="1",
                        RUST_LOG="info,forgeplan::constellation=debug")
        self.serial = 0
        self.last_command_elapsed = 0.0
        self.cases: list[dict] = []
        self.events = self.out / "events.jsonl"
        self.before = inventory.source_fingerprints()
        self.models = inventory.model_cache_inventory(args.model_cache)
        self.binaries = {str(path): inventory.binary_sha256(str(path))
                         for path in (args.binary, args.standalone)}

    def event(self, kind: str, **data: Any) -> None:
        with self.events.open("a", encoding="utf-8") as stream:
            stream.write(json.dumps({"kind": kind, "time": time.time(), **data}, ensure_ascii=False) + "\n")

    def stream_log(self, label: str, data: bytes) -> dict:
        self.serial += 1
        path = self.out / f"{self.serial:05d}-{label}.log"
        path.write_bytes(data)
        return {"path": str(path), "sha256": inventory.binary_sha256(str(path)), "bytes": len(data)}

    async def case(self, name: str, action: Callable[[], Awaitable[Any]]) -> None:
        started = time.monotonic()
        row = {"case": name, "status": "NOT_RUN"}
        self.cases.append(row)
        self.event("case_start", **row)
        try:
            row["observation"] = await action()
            row["status"] = "PASS"
        except ProofBlocked as error:
            row.update(status="BLOCKED", error=str(error))
        except Exception as error:
            # Unexpected protocol/data failures are FAIL rows too, never lost
            # receipts or successful empty fallbacks.
            row.update(status="FAIL", error=f"{type(error).__name__}: {error}")
        finally:
            row["elapsed_s"] = time.monotonic() - started
            self.event("case_result", **row)
            (self.out / "cases.json").write_text(json.dumps(self.cases, indent=2, ensure_ascii=False), encoding="utf-8")

    async def command(self, args: list[str], cwd: Path, *, program: Optional[str] = None,
                      success: bool = True, timeout: Optional[float] = None,
                      extra_env: Optional[dict[str, str]] = None) -> tuple[str, str]:
        require(inventory.free_bytes(self.root) >= inventory.STOP_BELOW_FREE_BYTES,
                "disk stop threshold reached; no automatic cleanup")
        if self.args.profile == "semantic" and args[:2] == ["fpf", "ingest"]:
            require_cached_model(self.models)
        argv = [program or str(self.args.binary), *args]
        env = {**self.env, **(extra_env or {})}
        if args and args[0] == "reason":
            # Only this subprocess retains its existing stock-vendor login.
            # No credential is read/copied/logged; cwd/config stay isolated.
            for key in ("HOME", "USERPROFILE", "CODEX_HOME", "CLAUDE_CONFIG_DIR"):
                if key in os.environ:
                    env[key] = os.environ[key]
        disk_before = inventory.disk_snapshot(self.root)
        started = time.monotonic()
        process = await asyncio.create_subprocess_exec(*argv, cwd=str(cwd), env=env,
            stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE, start_new_session=os.name != "nt")
        timed_out = disk_stopped = False
        deadline = started + (timeout or self.timeout)
        pending = asyncio.create_task(process.communicate())
        while not pending.done():
            remaining = deadline - time.monotonic()
            disk_stopped = inventory.free_bytes(self.root) < inventory.STOP_BELOW_FREE_BYTES
            timed_out = remaining <= 0
            if disk_stopped or timed_out:
                if os.name != "nt":
                    os.killpg(process.pid, signal.SIGKILL)
                else:
                    process.kill()
                break
            try:
                await asyncio.wait_for(asyncio.shield(pending), min(1.0, remaining))
            except asyncio.TimeoutError:
                continue
        stdout, stderr = await pending
        self.last_command_elapsed = time.monotonic() - started
        self.event("command", command=argv, cwd=str(cwd), pid=process.pid, exit=process.returncode,
                   timed_out=timed_out, disk_stopped=disk_stopped, elapsed_s=self.last_command_elapsed,
                   disk_before=disk_before, disk_after=inventory.disk_snapshot(self.root),
                   stdout=self.stream_log("stdout", stdout), stderr=self.stream_log("stderr", stderr),
                   source_fingerprints_after=inventory.source_fingerprints())
        require(not timed_out and not disk_stopped and ((process.returncode == 0) == success),
                {"command": argv, "exit": process.returncode, "timed_out": timed_out, "disk_stopped": disk_stopped,
                 "stdout": stdout.decode(errors="replace"), "stderr": stderr.decode(errors="replace")})
        require(self.before == inventory.source_fingerprints(), "source changed during runtime proof")
        return stdout.decode(), stderr.decode()

    async def cli_json(self, args: list[str], cwd: Path) -> Any:
        text, _ = await self.command(args, cwd)
        return json.loads(text)

    @asynccontextmanager
    async def mcp(self, root: Path, label: str, *, cli: bool = False,
                  extra_env: Optional[dict[str, str]] = None) -> AsyncIterator[Mcp]:
        command = ([str(self.args.binary), "serve", "--profile", "full", "--workspace", str(root)]
                   if cli else [str(self.args.standalone), "--workspace", str(root)])
        stderr_path = self.out / f"{label}.stderr.log"
        started = time.monotonic()
        with stderr_path.open("wb") as stderr:
            process = await asyncio.create_subprocess_exec(*command, cwd=str(root),
                env={**self.env, **(extra_env or {})}, stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE, stderr=stderr, limit=8 * 1024 * 1024)
            server = Mcp(self, process, label)
            self.event("mcp_start", command=command, cwd=str(root), label=label, pid=process.pid,
                       binary_sha256=self.binaries[command[0]])
            try:
                initialized = await server.rpc("initialize", {"protocolVersion": "2024-11-05",
                    "capabilities": {}, "clientInfo": {"name": "constellation-readiness", "version": "1"}})
                require("result" in initialized, initialized)
                await server.send({"jsonrpc": "2.0", "method": "notifications/initialized"})
                yield server
            finally:
                try:
                    if not server.exited_intentionally:
                        await server.eof()
                finally:
                    if process.returncode is None:
                        process.kill()
                        await process.wait()
                    server.reader.cancel()
                    await asyncio.gather(server.reader, return_exceptions=True)
                    self.event("mcp_log", label=label, pid=process.pid, exit=process.returncode,
                        elapsed_s=time.monotonic() - started, stderr={"path": str(stderr_path),
                        "sha256": inventory.binary_sha256(str(stderr_path)), "bytes": stderr_path.stat().st_size})

    async def workspace(self, label: str) -> Path:
        root = self.root / label
        root.mkdir()
        await self.command(["init", "-y"], root)
        (root / "src").mkdir()
        (root / "src/policy.rs").write_text("pub fn policy() -> bool { true }\n", encoding="utf-8")
        self.source(root, "ADR-901", "adr", "Policy one", "Initial policy sentinel.\n")
        self.source(root, "NOTE-901", "note", "Independent note", "Separate note.\n")
        (root / ".gitignore").write_text(".forgeplan/lance/\n.forgeplan/logs/\n.forgeplan/journal/\n.forgeplan/.lock\n", encoding="utf-8")
        await self.command(["init", "-q"], root, program="git")
        await self.command(["add", ".gitignore", "src", ".forgeplan/adrs", ".forgeplan/notes"], root, program="git")
        await self.command(["-c", "user.name=Readiness Fixture", "-c", "user.email=fixture@forgeplan.invalid",
                            "commit", "-qm", "Initial isolated fixture"], root, program="git")
        await self.command(["tag", "fixture-base"], root, program="git")
        self.source(root, "PRD-901", "prd", "Module policy", "A scoped requirement.\n")
        await self.command(["reindex"], root)
        await self.command(["add", ".forgeplan/prds"], root, program="git")
        await self.command(["-c", "user.name=Readiness Fixture", "-c", "user.email=fixture@forgeplan.invalid",
                            "commit", "-qm", "Requirement fixture"], root, program="git")
        return root

    def source(self, root: Path, artifact_id: str, kind: str, title: str, body: str,
               extra: str = "") -> Path:
        directory = {"adr": "adrs", "note": "notes", "prd": "prds"}[kind]
        path = root / ".forgeplan" / directory / f"{artifact_id}-fixture.md"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(f"---\nid: {artifact_id}\nkind: {kind}\nstatus: draft\ntitle: {title}\ndepth: tactical\n{extra}---\n\n{body}", encoding="utf-8")
        return path

    def current(self, root: Path) -> tuple[dict, dict]:
        index = root / ".forgeplan/lance"
        descriptor = json.loads((index / "CURRENT.json").read_text(encoding="utf-8"))
        relative = Path(descriptor["manifest_path"])
        require(not relative.is_absolute() and ".." not in relative.parts, descriptor)
        raw = (index / relative).read_bytes()
        require(hashlib.sha256(raw).hexdigest() == descriptor["manifest_sha256"], "manifest checksum")
        manifest = json.loads(raw)
        versions = manifest["table_versions"]
        require(all(isinstance(versions.get(role), int) and versions[role] > 0
                    for role in ("artifacts", "relations", "sections")), versions)
        require(descriptor["generation_id"] == manifest["generation_id"], "CURRENT/manifest generation")
        return descriptor, manifest

    def no_legacy(self, root: Path) -> None:
        for name in ("artifacts", "evidence", "relations"):
            require(not (root / ".forgeplan/lance" / f"{name}.lance").exists(), f"retired table recreated: {name}")


async def ordinary(proof: Proof, server: Mcp, root: Path, label: str) -> dict:
    results = {}
    for name, cli_args, arguments in ORDINARY:
        cli_text, _ = await proof.command(cli_args, root)
        result = await server.tool("forgeplan_" + name, arguments)
        results[name] = result
        require(bool(cli_text.strip()), f"{label}: empty CLI {name}")
        if name == "health":
            cli = json.loads(cli_text)
            require(cli["total"] == result["total"] == 3, {"cli": cli, "mcp": result})
            require(cli["verdict"] == result["verdict"], {"cli": cli, "mcp": result})
        elif name == "journal":
            require(any(row["id"] == "ADR-901" for row in result["entries"]), result)
        elif name == "fpf_section":
            require("proofknowledge" in result["body"], result)
        elif name == "fpf_list":
            require(result["total"] == 1 and result["sections"][0]["section_id"] == "B.3", result)
        elif name == "fpf_search":
            require("B.3" in json.dumps(result), result)
        elif name == "fpf_check":
            require(result.get("artifact_id", result.get("id")) == "ADR-901", result)
        elif name == "release_notes":
            require("PRD-901" in json.dumps(result), result)
        proof.event("ordinary_observation", case=label, tool="forgeplan_" + name,
                    pid=server.process.pid, observed=result)
    return results


async def external_mutation(proof: Proof, server: Mcp, root: Path, marker: str) -> dict:
    body = root / f"{marker}.md"
    body.write_text(f"## Updated constraint\n\n{marker}. Never log raw credentials.\n", encoding="utf-8")
    await proof.command(["update", "ADR-901", "--title", marker, "--body", "@" + str(body)], root)
    await proof.command(["tag", "ADR-901", marker], root)
    await proof.command(["link", "ADR-901", "NOTE-901", "--relation", "based_on", "--replace"], root)
    got = await server.tool("forgeplan_get", {"id": "ADR-901", "workspace": str(root)})
    cli = await proof.cli_json(["get", "ADR-901", "--json"], root)
    require(got["title"] == cli["title"] == marker, {"cli": cli, "mcp": got})
    require(marker in got["body"] and got["body"] == cli["body"], {"cli": cli, "mcp": got})
    require(marker in got["tags"] and got["tags"] == cli["tags"], got)
    outbound = got["links"]["outbound"]
    require(any(link["target"] == "NOTE-901" and link["relation"] == "based_on" for link in outbound), got)
    require(sorted(outbound, key=lambda link: (link["target"], link["relation"])) ==
            sorted(cli["links"], key=lambda link: (link["target"], link["relation"])),
            {"cli": cli, "mcp": got})
    journal = await server.tool("forgeplan_journal", {"workspace": str(root)})
    require(next(row for row in journal["entries"] if row["id"] == "ADR-901")["title"] == marker, journal)
    proof.event("external_cli_freshness", pid=server.process.pid, root=str(root), observed=got, cli=cli)
    return got


async def compatibility(proof: Proof) -> dict:
    root = await proof.workspace("compatibility")
    kb = proof.root / "fpf-input/part-b"
    kb.mkdir(parents=True)
    (kb / "03-b-3---proof.md").write_text("## B.3 — Proof knowledge\n\nproofknowledge: preserve evidence provenance.\n", encoding="utf-8")
    await proof.command(["fpf", "ingest", "--path", str(kb.parent)], root, timeout=600)
    # This pid first reads legacy; all three external CLI boundaries below are
    # observed without restarting the server or substituting same-process writes.
    async with proof.mcp(root, "legacy-through-migration", cli=True) as server:
        pid = server.process.pid
        before = await ordinary(proof, server, root, "legacy")
        await external_mutation(proof, server, root, "legacyexternalfresh")
        await proof.command(["index"], root)
        proof.no_legacy(root)
        migrated = await ordinary(proof, server, root, "same-pid-after-index")
        changed = await external_mutation(proof, server, root, "migratedexternalfresh")
        after = await ordinary(proof, server, root, "post-index-after-external-cli")
        require(server.process.pid == pid, "migration must not restart the full MCP process")
        proof.no_legacy(root)
    return {"pid": pid, "legacy": before, "migrated": migrated, "after": after, "fresh": changed}


async def missing_kb(proof: Proof) -> dict:
    root = await proof.workspace("missing-kb")
    await proof.command(["index"], root)
    before = fixture_tree(root / ".forgeplan/lance")
    async with proof.mcp(root, "missing-kb") as server:
        listed = await server.tool("forgeplan_fpf_list", {})
        require(listed["total"] == 0, listed)
        for name, arguments in (("forgeplan_fpf_search", {"query": "proofknowledge"}),
                                ("forgeplan_fpf_section", {"id": "B.3"})):
            response = await server.rpc("tools/call", {"name": name, "arguments": arguments})
            explicit_failure(response, ("knowledge", "not found", "not loaded"))
    # The server's authorized maintenance may update CURRENT on startup, but
    # an auxiliary read must not create a previously absent fpf_spec table.
    require(not (root / ".forgeplan/lance/fpf_spec.lance").exists(), "FPF read created a table")
    proof.no_legacy(root)
    return {"missing": listed, "before": before, "after": fixture_tree(root / ".forgeplan/lance")}


async def fpf_fixture(proof: Proof, label: str) -> Path:
    root = await proof.workspace("fpf-" + label)
    kb = proof.root / ("fpf-input-" + label) / "part-b"
    kb.mkdir(parents=True)
    (kb / "03-b-3---proof.md").write_text(
        "## B.3 — Proof knowledge\n\nproofknowledge: preserve evidence provenance.\n", encoding="utf-8")
    await proof.command(["fpf", "ingest", "--path", str(kb.parent)], root, timeout=600)
    await proof.command(["index"], root)
    return root


async def fpf_current_independence(proof: Proof) -> dict:
    root = await fpf_fixture(proof, "current-independence")
    pointer = root / ".forgeplan/lance/CURRENT.json"
    saved = pointer.read_bytes()
    result = {}
    # An already-running full server avoids conflating its startup with a KB
    # read. These three reads consult AuxiliaryStore, NOT generation tables.
    async with proof.mcp(root, "fpf-current-independence") as server:
        try:
            pointer.write_bytes(b"{deliberately corrupt CURRENT\n")
            for name, arguments, parameters in ORDINARY:
                if name not in ("fpf_list", "fpf_search", "fpf_section"):
                    continue
                cli, _ = await proof.command(arguments, root)
                observed = await server.tool("forgeplan_" + name, parameters)
                require("B.3" in cli and "B.3" in json.dumps(observed), {"cli": cli, "mcp": observed})
                if name == "fpf_list":
                    require(observed["total"] == 1, observed)
                result[name] = observed
            require(pointer.read_bytes() == b"{deliberately corrupt CURRENT\n",
                    "auxiliary read unexpectedly repaired/wrote the generation pointer")
        finally:
            pointer.write_bytes(saved)
    proof.no_legacy(root)
    return {"observations": result, "boundary": "CURRENT independence is intentional; actual auxiliary failures must still fail"}


async def fpf_auxiliary_failure(proof: Proof, mode: str) -> dict:
    root = await fpf_fixture(proof, "failure-" + mode)
    table = root / ".forgeplan/lance/fpf_spec.lance"
    require(table.is_dir(), "real ingest did not create the auxiliary dataset")
    backup = proof.root / ("damaged-auxiliary-backup-" + mode)
    if mode == "open":
        # Existing named table is damaged, not absent. open_existing must
        # propagate its actual open error rather than map it to detached().
        table.rename(backup)
        table.write_bytes(b"not a Lance dataset\n")
    else:
        # Preserve the actual valid schema/manifests. Missing referenced data
        # fails when the query reads fragments, not at an invented mock seam.
        files = sorted(path for path in (table / "data").rglob("*") if path.is_file())
        require(bool(files), "ingested auxiliary dataset has no data fragments; query-error injection is not established")
        for path in files:
            target = backup / path.relative_to(table)
            target.parent.mkdir(parents=True, exist_ok=True)
            path.rename(target)
    proof.event("required_auxiliary_negative", mode=mode,
                baseline_defect="base 3bf8266 open_existing .ok() discards real table-open errors",
                owner="WholeBody core follow-up with MCPCompat error conversion; no xfail/skip permitted")
    results = {}
    async with proof.mcp(root, "fpf-auxiliary-failure-" + mode) as server:
        for name, arguments, parameters in ORDINARY:
            if name not in ("fpf_list", "fpf_search", "fpf_section"):
                continue
            stdout, stderr = await proof.command(arguments, root, success=False)
            response = await server.rpc("tools/call", {"name": "forgeplan_" + name, "arguments": parameters})
            explicit_failure(response, ("auxiliary", "lance", "query", "read", "storage", "internal", "io"))
            text = json.dumps(response).lower().replace("_", " ")
            require(not any(missing in text for missing in ("not initialized", "not loaded")),
                    "damaged existing KB was mislabeled as an absent knowledge base")
            results[name] = {"cli_stdout": stdout, "cli_stderr": stderr, "mcp": response}
    proof.no_legacy(root)
    return {"mode": mode, "observations": results}


async def fpf_auxiliary_open_failure(proof: Proof) -> dict:
    return await fpf_auxiliary_failure(proof, "open")


async def fpf_auxiliary_query_failure(proof: Proof) -> dict:
    return await fpf_auxiliary_failure(proof, "query")


async def capture_failures(proof: Proof) -> dict:
    root = await proof.workspace("invalid-input")
    await proof.command(["index"], root)
    path = root / ".forgeplan/adrs/ADR-901-fixture.md"
    original = path.read_bytes()
    descriptor = root / ".forgeplan/lance/CURRENT.json"
    cases: dict = {}
    for name, invalid in (("yaml", b"---\nid: [invalid\n---\nbroken\n"),
                          ("links", b"---\nid: ADR-901\nkind: adr\nstatus: draft\ntitle: bad\nlinks: wrong-type\n---\nbroken\n")):
        path.write_bytes(invalid)
        pointer_before = descriptor.read_bytes()
        try:
            text, errors = await proof.command(["health", "--json"], root, success=False)
            require(any(code in (text + errors).lower().replace("_", "") for code in ("sourceunavailable", "invalidlinks", "sourceinvalid")), text + errors)
            async with proof.mcp(root, "invalid-" + name) as server:
                observed = []
                for tool, arguments in (("forgeplan_health", {}), ("forgeplan_coverage", {}),
                                        ("forgeplan_journal", {}), ("forgeplan_fpf_check", {"id": "ADR-901"})):
                    response = await server.rpc("tools/call", {"name": tool, "arguments": arguments})
                    explicit_failure(response, ("source_unavailable", "invalid_links", "source_invalid"))
                    observed.append(response)
            require(descriptor.read_bytes() == pointer_before, "failed scan published a deletion")
            cases[name] = observed
        finally:
            path.write_bytes(original)
    for name in ("current", "manifest", "missing-sections-pin"):
        saved_pointer = descriptor.read_bytes()
        current, manifest = proof.current(root)
        manifest_path = root / ".forgeplan/lance" / current["manifest_path"]
        saved_manifest = manifest_path.read_bytes()
        try:
            if name == "current":
                descriptor.write_bytes(b"{corrupt\n")
            elif name == "manifest":
                manifest_path.write_bytes(b"{corrupt\n")
            else:
                manifest["table_versions"].pop("sections")
                raw = json.dumps(manifest).encode()
                manifest_path.write_bytes(raw)
                current["manifest_sha256"] = hashlib.sha256(raw).hexdigest()
                descriptor.write_text(json.dumps(current), encoding="utf-8")
            stdout, stderr = await proof.command(["health", "--json"], root, success=False)
            require(any(code in (stdout + stderr).lower().replace("_", "") for code in ("indexgenerationunavailable", "indexprotocolmismatch")), stdout + stderr)
            async with proof.mcp(root, "bad-" + name) as server:
                response = await server.rpc("tools/call", {"name": "forgeplan_health", "arguments": {}})
                explicit_failure(response, ("index_generation_unavailable", "index_protocol_mismatch"))
                cases[name] = response
        finally:
            manifest_path.write_bytes(saved_manifest)
            descriptor.write_bytes(saved_pointer)
    if os.name != "nt":
        mode = stat.S_IMODE(path.stat().st_mode)
        path.chmod(0)
        try:
            require(not os.access(path, os.R_OK), "runner can still read chmod(0) fixture; unreadable proof requires an unprivileged runner")
            stdout, stderr = await proof.command(["get", "ADR-901", "--json"], root, success=False)
            require("source" in (stdout + stderr).lower(), stdout + stderr)
            cases["unreadable"] = {"stdout": stdout, "stderr": stderr}
        finally:
            path.chmod(mode)
    else:
        raise ProofFailure("unreadable-file permissions proof not implemented on Windows; not a PASS")
    proof.no_legacy(root)
    return cases


async def scan_import(proof: Proof) -> dict:
    root = await proof.workspace("scan-import")
    await proof.command(["index"], root)
    external = root / "external-docs"
    external.mkdir()
    source = external / "ADR-955-preserved.md"
    raw = ("---\nid: ADR-955\nkind: adr\nstatus: draft\ntitle: Imported provenance\nauthor: Import Fixture\n"
           "custom_provenance:\n  contract: preserve-unknown-value\nlinks:\n  - target: ADR-901\n    relation: based_on\n---\n\n## Imported constraint\n\nExact external body.  \n尾🙂\n").encode()
    source.write_bytes(raw)
    before = fixture_tree(root / ".forgeplan")
    await proof.command(["scan-import", "--path", str(external), "--dry-run"], root)
    require(before == fixture_tree(root / ".forgeplan"), "scan-import dry-run mutated files/tables")
    await proof.command(["scan-import", "--path", str(external)], root)
    imported = await proof.cli_json(["get", "ADR-955", "--json"], root)
    require(imported["author"] == "Import Fixture" and imported["status"] == "draft", imported)
    require("Exact external body.  \n尾🙂" in imported["body"], imported)
    candidates = [path for path in (root / ".forgeplan/adrs").iterdir()
                  if path.is_file() and "preserve-unknown-value" in path.read_text(encoding="utf-8")]
    require(len(candidates) == 1, "canonical import lost unknown frontmatter")
    canonical = candidates[0].read_bytes()
    require(b"relation: based_on" in canonical and b"target: ADR-901" in canonical, canonical.decode())
    await proof.command(["update", "ADR-955", "--title", "Local preserved title"], root)
    changed = fixture_tree(root / ".forgeplan/adrs")
    await proof.command(["scan-import", "--path", str(external)], root)
    require(fixture_tree(root / ".forgeplan/adrs") == changed, "repeat import overwrote existing canonical document")
    require(source.read_bytes() == raw, "import changed or removed external source")
    proof.no_legacy(root)
    return {"imported": imported, "canonical_sha256": hashlib.sha256(canonical).hexdigest(),
            "source_sha256": hashlib.sha256(raw).hexdigest(), "repeat_canonical_files": changed}


async def lifecycle(proof: Proof) -> dict:
    root = await proof.workspace("lifecycle")
    await proof.command(["index"], root)
    score = await proof.cli_json(["score", "ADR-901", "--json"], root)
    require("ADR-901" in json.dumps(score), score)
    await proof.command(["activate", "NOTE-901"], root)
    activated = await proof.cli_json(["get", "NOTE-901", "--json"], root)
    require(activated["status"] == "active", activated)
    prerequisite(proof.args.reason_config is not None,
                 "positive reason compatibility requires explicit non-secret --reason-config")
    fields = reason_config(proof.args.reason_config)
    config_path = root / ".forgeplan/config.yaml"
    original = config_path.read_text(encoding="utf-8")
    require(not re.search(r"^llm:", original, re.MULTILINE), "fixture already has provider config")
    public_yaml = "llm:\n" + "".join(f"  {key}: {json.dumps(value)}\n" for key, value in sorted(fields.items()))
    config_path.write_text(original.rstrip() + "\n" + public_yaml, encoding="utf-8")
    try:
        reason_text, _ = await proof.command(["reason", "ADR-901", "--json"], root, timeout=3600)
    except ProofFailure as error:
        raise ProofBlocked(f"configured reason command failed; cause remains in its real logs, no substitute provider or auth copy: {error}") from error
    reason = json.loads(reason_text)
    proof.event("reason_boundary", provider=fields["provider"], model=fields.get("model", ""),
                config_sha256=hashlib.sha256(public_yaml.encode()).hexdigest(),
                vendor_login="inherited stock login environment; no auth copied")
    require(isinstance(reason, dict) and bool(reason), "real reason produced no analysis")
    proof.no_legacy(root)
    return {"score": score, "activated": activated, "reason": reason}


async def ingest(proof: Proof) -> dict:
    root = await proof.workspace("ingest")
    await proof.command(["index"], root)
    external = root / "input"
    external.mkdir()
    (external / "component.md").write_text("---\nkind: component\nname: Ingest proof\n---\n\n## Purpose\n\nNever drop source provenance.\n", encoding="utf-8")
    mapping = root / "mapping.yaml"
    mapping.write_text('''schema_version: "1.0"
name: readiness-ingest
source_kind: fixture
target_kind: forge
sources:
  - pattern: "*.md"
    type: markdown
    parser: front_matter_plus_sections
rules:
  - id: component-note
    when:
      file_glob: "*.md"
      front_matter:
        kind: component
    target:
      kind: note
    fields:
      title: "{{ front_matter.name | trim }}"
      summary: "{{ section.purpose | trim }}"
    sources_section:
      include: true
      format: "{path}:{line_start}-{line_end}"
      precision: line
      source_hash: true
guards:
  max_artifacts: 2
  require_section: ["Sources"]
  forbid_overwrite_active: true
''', encoding="utf-8")
    before = fixture_tree(root / ".forgeplan", canonical_only=True)
    dry = await proof.cli_json(["ingest", "--mapping", str(mapping), "--source", str(external), "--dry-run", "--json"], root)
    require(before == fixture_tree(root / ".forgeplan", canonical_only=True), "ingest dry-run changed canonical documents")
    applied = await proof.cli_json(["ingest", "--mapping", str(mapping), "--source", str(external), "--json"], root)
    notes = [path.read_text(encoding="utf-8") for path in (root / ".forgeplan/notes").iterdir() if path.suffix == ".md"]
    require(any("Never drop source provenance" in text and "## Sources" in text and "component.md" in text for text in notes), applied)
    proof.no_legacy(root)
    return {"dry_run": dry, "applied": applied}


async def byte_mutations(proof: Proof) -> dict:
    root = await proof.workspace("byte-mutations")
    await proof.command(["index"], root)
    path = root / ".forgeplan/adrs/ADR-901-fixture.md"
    original = path.read_bytes()
    stamp = path.stat()
    changed = original.replace(b"Initial policy sentinel", b"Changed policy sentinel")
    require(len(changed) == len(original), "same-size fixture must actually preserve size")
    path.write_bytes(changed)
    os.utime(path, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
    got = await proof.cli_json(["get", "ADR-901", "--json"], root)
    require("Changed policy sentinel" in got["body"] and got["persisted"] is False, got)
    before_pointer = (root / ".forgeplan/lance/CURRENT.json").read_bytes()
    require(before_pointer, "published baseline")
    await proof.command(["update", "ADR-901", "--body", ""], root)
    cleared = await proof.cli_json(["get", "ADR-901", "--json"], root)
    require("policy sentinel" not in cleared["body"], cleared)
    renamed = path.with_name("ADR-901-renamed.md")
    path.rename(renamed)
    renamed_get = await proof.cli_json(["get", "ADR-901", "--json"], root)
    require(renamed_get["id"] == "ADR-901", renamed_get)
    await proof.command(["delete", "ADR-901", "--yes"], root)
    await proof.command(["get", "ADR-901", "--json"], root, success=False)
    proof.source(root, "ADR-901", "adr", "Recreated", "New incarnation body.\n")
    await proof.command(["index"], root)
    recreated = await proof.cli_json(["get", "ADR-901", "--json"], root)
    require("New incarnation body" in recreated["body"] and "policy sentinel" not in recreated["body"], recreated)
    return {"same_size_mtime": got, "empty": cleared, "renamed": renamed_get, "recreated": recreated}


async def git_and_atomic(proof: Proof) -> dict:
    root = await proof.workspace("git-atomic")
    await proof.command(["index"], root)
    pointer = (root / ".forgeplan/lance/CURRENT.json").read_bytes()
    await proof.command(["checkout", "-qb", "changed"], root, program="git")
    await proof.command(["update", "ADR-901", "--body", "Branch changed bytes."], root)
    await proof.command(["add", ".forgeplan/adrs"], root, program="git")
    await proof.command(["-c", "user.name=Readiness Fixture", "-c", "user.email=fixture@forgeplan.invalid",
                        "commit", "-qm", "Changed branch"], root, program="git")
    await proof.command(["checkout", "-q", "-"], root, program="git")
    restored = await proof.cli_json(["get", "ADR-901", "--json"], root)
    require("Initial policy sentinel" in restored["body"], restored)
    await proof.command(["reset", "--hard", "fixture-base"], root, program="git")
    await proof.command(["get", "PRD-901", "--json"], root, success=False)
    await proof.command(["checkout", "-qb", "rebased-upstream"], root, program="git")
    await proof.command(["update", "NOTE-901", "--body", "Upstream note after real rebase."], root)
    await proof.command(["add", ".forgeplan/notes"], root, program="git")
    await proof.command(["-c", "user.name=Readiness Fixture", "-c", "user.email=fixture@forgeplan.invalid",
                         "commit", "-qm", "Upstream note"], root, program="git")
    await proof.command(["checkout", "-q", "changed"], root, program="git")
    await proof.command(["-c", "user.name=Readiness Fixture", "-c", "user.email=fixture@forgeplan.invalid",
                         "rebase", "rebased-upstream"], root, program="git")
    rebased = await proof.cli_json(["get", "ADR-901", "--json"], root)
    rebased_note = await proof.cli_json(["get", "NOTE-901", "--json"], root)
    require("Branch changed bytes." in rebased["body"] and
            "Upstream note after real rebase." in rebased_note["body"],
            {"rebased": rebased, "note": rebased_note})
    # No watcher is running. Atomic replacement in a newly created nested
    # artifact directory is discovered by the next read-only bytes barrier.
    directory = root / ".forgeplan/notes/nested"
    directory.mkdir()
    path = directory / "NOTE-902-atomic.md"
    temporary = directory / ".editor-save"
    temporary.write_text("---\nid: NOTE-902\nkind: note\nstatus: draft\ntitle: Atomic new directory\n---\n\natomicnewdirectorysentinel\n", encoding="utf-8")
    temporary.replace(path)
    atomic = await proof.cli_json(["get", "NOTE-902", "--json"], root)
    require("atomicnewdirectorysentinel" in atomic["body"] and atomic["persisted"] is False, atomic)
    # reset/checkout are intentional fixture operations, never checkout changes.
    require((root / ".forgeplan/lance/CURRENT.json").read_bytes() == pointer or restored["persisted"] is False,
            "query must not publish; the external CLI update is separately allowed maintenance")
    return {"checkout": restored, "rebase": rebased, "rebased_note": rebased_note, "atomic_without_notify": atomic}


async def worktree_isolation(proof: Proof) -> dict:
    first = await proof.workspace("worktree-origin")
    second = proof.root / "worktree-other"
    await proof.command(["worktree", "add", "--detach", str(second), "HEAD"], first, program="git")
    proof.source(second, "ADR-901", "adr", "Other physical checkout", "otherworktreesentinel\n")
    await proof.command(["index"], first)
    await proof.command(["index"], second)
    first_descriptor, _ = proof.current(first)
    second_descriptor, _ = proof.current(second)
    require(first_descriptor["checkout_id"] != second_descriptor["checkout_id"], "physical worktrees share checkout identity")
    async with proof.mcp(first, "explicit-other-worktree", cli=True) as server:
        origin = await server.tool("forgeplan_get", {"id": "ADR-901", "workspace": str(first)})
        other = await server.tool("forgeplan_get", {"id": "ADR-901", "workspace": str(second)})
        require("Initial policy sentinel" in origin["body"] and "otherworktreesentinel" not in origin["body"], origin)
        require("otherworktreesentinel" in other["body"] and "Initial policy sentinel" not in other["body"], other)
    return {"origin": first_descriptor, "other": second_descriptor, "origin_read": origin, "other_read": other}


async def unavailable_model(proof: Proof) -> dict:
    root = await proof.workspace("unavailable-model")
    empty = proof.root / "no-model-cache"
    env = {"FORGEPLAN_MODEL_CACHE": str(empty)}
    await proof.command(["index"], root, extra_env=env)
    before = (root / ".forgeplan/lance/CURRENT.json").read_bytes()
    stdout, _ = await proof.command(["search", "Initial policy sentinel", "--json"], root, extra_env=env)
    lexical = json.loads(stdout)
    require(lexical["semantic_available"] is False and len(lexical["results"]) > 0, lexical)
    require((root / ".forgeplan/lance/CURRENT.json").read_bytes() == before, "read without a model published an index")
    async with proof.mcp(root, "unavailable-model", cli=True, extra_env=env) as server:
        response = await server.tool("forgeplan_search", {"query": "Initial policy sentinel", "workspace": str(root)})
        require(response["coverage"]["semantic"] is False and response["total"] > 0, response)
        require(not empty.exists(), "ordinary query downloaded/created a model cache")
    if proof.args.profile == "no-semantic":
        text, errors = await proof.command(["search", "policy", "--semantic", "--json"], root,
                                          success=False, extra_env=env)
        require("semantic" in (text + errors).lower(), "feature-off refusal must identify the missing capability")
    return {"lexical": lexical, "mcp": response, "no_download": not empty.exists()}


async def relations_and_scores(proof: Proof) -> dict:
    root = await proof.workspace("relations-and-scores")
    evidence = root / ".forgeplan/evidence/EVID-901-fixture.md"
    evidence.write_text("---\nid: EVID-901\nkind: evidence\nstatus: active\ntitle: Executable fixture evidence\ndepth: tactical\nlinks:\n  - target: ADR-901\n    relation: informs\n---\n\n## Structured Fields\n\nverdict: supports\ncongruence_level: 3\nevidence_type: test\n\n## Results\n\nControlled fixture only.\n", encoding="utf-8")
    await proof.command(["reindex"], root)
    baseline = await proof.cli_json(["score", "ADR-901", "--json"], root)
    require(baseline["r_eff"] == 1.0, baseline)
    await proof.command(["index"], root)
    migrated = await proof.cli_json(["score", "ADR-901", "--json"], root)
    require(migrated["r_eff"] == 1.0 and len(migrated["evidence"]) == 1, migrated)
    await proof.command(["link", "ADR-901", "PRD-901", "--relation", "refines"], root)
    await proof.command(["link", "ADR-901", "NOTE-901", "--relation", "informs"], root)
    async with proof.mcp(root, "relations-and-scores", cli=True) as server:
        await proof.command(["unlink", "ADR-901", "NOTE-901", "--relation", "informs"], root)
        await proof.command(["link", "ADR-901", "PRD-901", "--relation", "contradicts", "--replace"], root)
        got = await server.tool("forgeplan_get", {"id": "ADR-901", "workspace": str(root)})
        require(got["links"]["outbound"] == [{"target": "PRD-901", "relation": "contradicts"}], got)
        await proof.command(["update", "EVID-901", "--body",
                             "## Structured Fields\n\nverdict: refutes\ncongruence_level: 3\nevidence_type: test\n"], root)
        scored = await server.tool("forgeplan_score", {"id": "ADR-901", "workspace": str(root)})
        cli = await proof.cli_json(["score", "ADR-901", "--json"], root)
        require(scored["r_eff"] == cli["r_eff"] == 0.0, {"cli": cli, "mcp": scored})
        health = await server.tool("forgeplan_health", {})
        require(health["total"] == 4 and health["verdict"] != "healthy", health)
    return {"legacy_score": baseline, "post_index_score": migrated, "new_links": got["links"],
            "refuted_score": cli, "health": health}


async def two_serve_takeover(proof: Proof) -> dict:
    prerequisite(proof.args.helper_binary is not None, "two-serve owner barriers require --helper-binary")
    root = await proof.workspace("two-serve")
    await proof.command(["index"], root)
    saved = proof.args.binary
    proof.args.binary = proof.args.helper_binary
    proof.binaries[str(proof.args.helper_binary)] = inventory.binary_sha256(str(proof.args.helper_binary))
    empty = {"FORGEPLAN_MODEL_CACHE": str(proof.root / "owner-no-model")}
    try:
        async with Barrier("owner_acquired") as first_barrier:
            async with proof.mcp(root, "first-owner-serve", cli=True,
                                 extra_env={**empty, **first_barrier.environment()}) as first:
                first_event, _ = await first_barrier.reached(first.process.pid, proof.timeout)
                proof.event("barrier", **first_event)
                async with Barrier("owner_acquired") as takeover_barrier:
                    async with proof.mcp(root, "second-owner-serve", cli=True,
                                         extra_env={**empty, **takeover_barrier.environment()}) as second:
                        # Actual external CLI canonical mutation must complete
                        # while another process holds the real owner OS guard.
                        started = time.monotonic()
                        _, errors = await proof.command(["tag", "ADR-901", "two-owner-external"], root,
                                                       program=str(saved), timeout=8, extra_env=empty)
                        require(time.monotonic() - started < 8 and "index_pending" in errors, errors)
                        for server in (first, second):
                            got = await server.tool("forgeplan_get", {"id": "ADR-901", "workspace": str(root)})
                            require("two-owner-external" in got["tags"], got)
                        first.process.kill()
                        code = await asyncio.wait_for(first.process.wait(), proof.shutdown_bound)
                        first.exited_intentionally = True
                        require(code == -signal.SIGKILL, {"exit": code})
                        proof.event("intentional_sigkill", pid=first.process.pid, exit=code)
                        entered, writer = await takeover_barrier.reached(second.process.pid, proof.timeout)
                        proof.event("barrier", **entered)
                        writer.write(json.dumps({"token": takeover_barrier.token, "stage": "owner_acquired", "release": True}).encode() + b"\n")
                        await writer.drain()
                        deadline = time.monotonic() + proof.timeout
                        wanted = hashlib.sha256((root / ".forgeplan/adrs/ADR-901-fixture.md").read_bytes()).hexdigest()
                        while True:
                            descriptor, manifest = proof.current(root)
                            if any(entry["source_digest"] == wanted for entry in manifest["source_entries"]):
                                break
                            require(time.monotonic() < deadline, "surviving serve failed to publish after actual takeover")
                            await asyncio.sleep(0.01)
                        got = await second.tool("forgeplan_get", {"id": "ADR-901"})
                        require("two-owner-external" in got["tags"], got)
                        return {"first_owner": first_event, "takeover": entered, "killed_exit": code,
                                "descriptor": descriptor, "manifest": manifest,
                                "boundary": "actual two-serve takeover; pinned table coherence is separately observed by the core subprocess suite"}
    finally:
        proof.args.binary = saved


async def real_embed(proof: Proof, root: Path) -> None:
    require_cached_model(proof.models)
    await proof.command(["embed"], root, timeout=1800)


async def semantic(proof: Proof) -> dict:
    root = await proof.workspace("semantic")
    long_body = ("The device inventory lists spare bolts and ordinary brackets.\n" * 180
                 + "\n## Credential lifetime\n\nAuthentication credentials MUST expire after fifteen minutes. Never persist raw passwords.\n")
    proof.source(root, "ADR-901", "adr", "Device inventory " * 180, long_body)
    proof.source(root, "NOTE-901", "note", "Culinary notes", "Bake sourdough loaves at high temperature.\n")
    await proof.command(["index"], root)
    await real_embed(proof, root)
    current, manifest = proof.current(root)
    query = "How long may a sign-in token stay valid?"
    cli = await proof.cli_json(["search", query, "--semantic", "--json", "--limit", "1"], root)
    cold_seconds = proof.last_command_elapsed
    require(cli["semantic_available"] is True and cli["semantic_scope"] == "prefix+sections", cli)
    require(cli["results"] and cli["results"][0]["id"] == "ADR-901", cli)
    tail_key = cli["results"][0]["canonical_key"]
    require(tail_key.startswith("ADR-901#s"), "tail positive must be a section, not the artifact-prefix channel")
    # Require an actual published membership record for that key; a section-
    # shaped result alone does not prove the returned ref covers the tail.
    sections = manifest["section_bindings"]
    require(isinstance(sections, list), "published section membership missing")
    tail = next((section for section in sections if section["section_key"] == tail_key), None)
    require(tail is not None, {"tail_key": tail_key, "section_bindings": sections})
    end = tail.get("byte_end")
    start = tail.get("byte_start")
    require(isinstance(start, int) and isinstance(end, int) and start < end,
            "tail section must expose the published UTF-8 byte range")
    captured_body = long_body.encode()
    require(b"fifteen minutes" in captured_body[start:end], tail)
    negative = await proof.cli_json(["search", "How should sourdough bread be baked?", "--semantic", "--json", "--limit", "1"], root)
    require(negative["semantic_available"] is True and negative["results"], "negative control did not exercise a real semantic result")
    require(all(hit["canonical_key"] != tail_key for hit in negative["results"]), negative)
    async with proof.mcp(root, "warm-model", cli=True) as server:
        series = []
        for index in range(3):
            started = time.monotonic()
            response = await server.tool("forgeplan_search", {"query": query, "mode": "semantic", "workspace": str(root), "no_expand": True})
            require(response["coverage"]["semantic"] is True and any(hit["id"] == "ADR-901" and hit["semantic_score"] > 0 for hit in response["results"]), response)
            series.append({"pid": server.process.pid, "iteration": index, "elapsed_s": time.monotonic() - started, "response": response})
    return {"channel": "prefix+sections", "tail_key": tail_key, "current": current,
            "cold_cli_s": cold_seconds, "cold_cli": cli, "negative": negative,
            "same_process_mcp": series, "not_proved": "no inference about observed ONNX hangs or cross-build speedups"}


async def native_helper(proof: Proof) -> dict:
    prerequisite(proof.args.helper_binary is not None, "--helper-binary is required for the helper-only native CallGate proof")
    root = await proof.workspace("native-helper")
    await proof.command(["index"], root)
    await real_embed(proof, root)
    saved = proof.args.binary
    proof.args.binary = proof.args.helper_binary
    proof.binaries[str(proof.args.helper_binary)] = inventory.binary_sha256(str(proof.args.helper_binary))
    observed = {}
    try:
        for mode in ("response-deadline", "cancel-eof"):
            async with Barrier("native_call_entered") as barrier:
                async with proof.mcp(root, "native-helper-" + mode, cli=True, extra_env=barrier.environment()) as server:
                    started = time.monotonic()
                    request = asyncio.create_task(server.rpc("tools/call", {"name": "forgeplan_search", "arguments": {
                        "query": "policy", "mode": "semantic", "workspace": str(root)}}))
                    try:
                        entered, _ = await barrier.reached(server.process.pid, proof.timeout)
                        proof.event("barrier", **entered)
                        request_id = server.serial
                        ping = await server.rpc("ping", {}, timeout=5)
                        require("result" in ping, ping)
                        response = None
                        if mode == "response-deadline":
                            response = await asyncio.wait_for(request, 125)
                            elapsed = time.monotonic() - started
                            require(elapsed <= 125, {"response_elapsed_s": elapsed, "bound_s": 125})
                            if "error" not in response and not response.get("result", {}).get("isError"):
                                degraded = payload(response)
                                require(degraded["coverage"]["semantic"] is False,
                                        "blocked native entry cannot report semantic-positive success")
                            proof.event("helper_response_deadline", elapsed_s=elapsed,
                                        configured_soft_deadline_ms=120000, response=response)
                        else:
                            await server.send({"jsonrpc": "2.0", "method": "notifications/cancelled",
                                               "params": {"requestId": request_id, "reason": "readiness helper shutdown"}})
                        exited = await server.eof()
                        observed[mode] = {"stage": entered, "ping": ping, "response": response,
                                          "shutdown": exited, "configured_soft_deadline_ms": 120000}
                    finally:
                        request.cancel()
                        await asyncio.gather(request, return_exceptions=True)
        return {"observations": observed, "boundary": "helper-controlled CallGate entry stall, not an observed ONNX hang"}
    finally:
        proof.args.binary = saved


async def production_shutdown(proof: Proof) -> dict:
    root = await proof.workspace("production-shutdown")
    await proof.command(["index"], root)
    await real_embed(proof, root)
    trace = proof.out / "production-inference-shutdown.stderr.log"
    async with proof.mcp(root, "production-inference-shutdown", cli=True) as server:
        request = asyncio.create_task(server.rpc("tools/call", {"name": "forgeplan_search", "arguments": {
            "query": "validity of authentication credentials", "mode": "semantic", "workspace": str(root)}}))
        try:
            call_id, entry_line = await native_entry(trace, server.process, proof.timeout)
            require(not request.done(), "query completed before the native-work shutdown observation")
            proof.event("production_native_entry", pid=server.process.pid, call_id=call_id, trace_line=entry_line)
            exited = await server.eof()
        finally:
            request.cancel()
            await asyncio.gather(request, return_exceptions=True)
    log = trace.read_text(encoding="utf-8")
    returned = any(re.search(r'\bevent[=:]\s*"?constellation_native_encode_returned"?', line) and
                   re.search(rf"\bcall_id[=:]\s*{re.escape(call_id)}\b", line)
                   for line in log.splitlines())
    return {"entry": entry_line, "call_id": call_id, "returned_before_process_exit": returned,
            "shutdown": exited, "boundary": "production encode entry observed before EOF; not an observed ONNX hang"}


async def rust_observations(proof: Proof) -> dict:
    prerequisite(proof.args.acceptance_report is not None,
                 "--acceptance-report from the sole build owner is required for pinned fault/tokenizer observations")
    report = json.loads(proof.args.acceptance_report.read_text(encoding="utf-8"))
    require(report["result"] == "PASS" and report["source_fingerprints_after"] == proof.before,
            "acceptance report is failing or belongs to a different source fingerprint")
    result = {}
    for name, (target, test) in RUST_OBSERVATIONS.items():
        if name == "S1-R3.core_native_helper" and proof.args.profile == "no-semantic":
            result[name] = {"status": "NOT_APPLICABLE", "reason": "helper requires semantic-search; separate semantic run mandatory"}
            continue
        groups = [group for group in report["runs"] if any(target in executable.get("executable", "") for executable in group.get("executables", []))]
        if name == "S1-R3.core_native_helper":
            groups = [group for group in groups if group["profile"] == "semantic"]
        require(groups, f"missing mandatory Rust process observation {name}")
        evidence = []
        for group in groups:
            for executable in group.get("executables", []):
                if target not in executable["executable"]:
                    continue
                require(executable["exit"] == 0 and not executable["ran_nothing"] and executable["counts"]["ignored"] == 0, executable)
                log = executable["logs"]["stdout"]
                path = Path(log["path"])
                require(inventory.binary_sha256(str(path)) == log["sha256"] and path.stat().st_size == log["bytes"], "Rust log hash/size mismatch")
                text = path.read_text(encoding="utf-8")
                if test:
                    require(test in text, f"missing executed test observation for {test}")
                    if target == "constellation_fault_process":
                        require("PROOF_OBSERVATION" in text, f"missing process result record for {test}")
                if name == "L09.pinned_generation_faults":
                    for stage in PUBLICATION_STAGES:
                        require(f'"stage":"{stage}"' in text, f"missing stage {stage}")
                evidence.append({"profile": group["profile"], "executable": executable["executable"], "log": log})
        require(bool(evidence), name)
        result[name] = evidence
    return result


async def main_async(args: argparse.Namespace) -> int:
    if args.helper_binary is not None:
        args.helper_binary = args.helper_binary.resolve()
        require(args.helper_binary not in (args.binary.resolve(), args.standalone.resolve()),
                "helper and production binary must be distinct, separately preserved paths")
    inventory._EXPLICIT_SOURCE_PATHS = {(inventory.REPO / path).absolute() for path in args.include_source}
    if args.acceptance_report is not None:
        frozen = json.loads(args.acceptance_report.read_text(encoding="utf-8"))
        inventory._EXPLICIT_SOURCE_PATHS.update(
            inventory.REPO / path
            for paths in frozen.get("source_inventory", {}).values() for path in paths
        )
    args.binary, args.standalone = args.binary.resolve(), args.standalone.resolve()
    args.model_cache = args.model_cache.expanduser().resolve()
    for path in (args.binary, args.standalone):
        require(path.is_file() and os.access(path, os.X_OK), f"missing executable {path}")
    require(inventory.free_bytes(inventory.REPO) >= inventory.REQUIRED_MIN_FREE_BYTES,
            "less than 10 GiB free; runtime batch not started")
    with tempfile.TemporaryDirectory(prefix="forgeplan-constellation-readiness-") as temporary:
        proof = Proof(args, Path(temporary))
        report = {"phase": args.phase, "boundary": "S1 runtime only; no S2-S4 acceptance",
                  "historical_probe_counts": {"25/42/28": "input trace only, not acceptance"},
                  "source_fingerprints_before": proof.before, "source_inventory": inventory.source_inventory(),
                  "profile": args.profile,
                  "runtime_inputs": {
                      "helper_binary": str(args.helper_binary) if args.helper_binary else None,
                      "reason_config": str(args.reason_config) if args.reason_config else None,
                      "acceptance_report": str(args.acceptance_report) if args.acceptance_report else None,
                      "shutdown_bound_s": args.shutdown_bound,
                  },
                  "binary_sha256_before": proof.binaries, "model_tokenizer_before": proof.models,
                  "disk_before": inventory.disk_snapshot(inventory.REPO), "cases": proof.cases,
                  "environment": {key: os.environ[key] for key in inventory._SAFE_ENV_KEYS if key in os.environ},
                  "result": "RUNNING"}
        selected = ["s1", "s2", "s3", "s4"] if args.phase == "all" else [args.phase]
        if "s1" in selected:
            for name, action in (
                ("S1-R1.live_legacy_migration_external_cli_all_12", compatibility),
                ("S1-R1.missing_fpf_kb", missing_kb),
                ("S1-R1.fpf_current_independence", fpf_current_independence),
                ("S1-R1.fpf_auxiliary_open_error", fpf_auxiliary_open_failure),
                ("S1-R1.fpf_auxiliary_query_error", fpf_auxiliary_query_failure),
                ("L08.corrupt_capture_and_current", capture_failures),
                ("S1-R2.canonical_scan_import", scan_import),
                ("S1-R1.score_activate_reason", lifecycle),
                ("S1-R1.canonical_ingest", ingest),
                ("L02.L04.L05.byte_identity", byte_mutations),
                ("L06.L07.git_atomic_without_watcher", git_and_atomic),
                ("L01.L03.report_values_exact_relation_replacement", relations_and_scores),
                ("L10.two_real_serve_cli_takeover", two_serve_takeover),
                ("L13.physical_worktree_explicit_scope", worktree_isolation),
                ("L14.L16.unavailable_model_fresh_lexical", unavailable_model),
                ("L12.L16.real_tail_vectors_cold_warm", semantic),
                ("S1-R3.native_helper_stall", native_helper),
                ("S1-R3.production_shutdown_while_working", production_shutdown),
                ("S1-R4.pinned_rust_process_and_tokenizer_evidence", rust_observations),
            ):
                if args.profile == "no-semantic" and action in (semantic, native_helper, production_shutdown):
                    proof.cases.append({"case": name, "status": "NOT_APPLICABLE",
                                        "reason": "semantic-search intentionally absent; separate semantic run remains mandatory"})
                    continue
                await proof.case(name, lambda action=action: action(proof))
        for phase in selected:
            if phase != "s1":
                proof.cases.append({"case": phase, "status": "NOT_ADMITTED",
                                    "error": "later-phase scenarios require their approved stage; not an S1 PASS"})
        report.update(source_fingerprints_after=inventory.source_fingerprints(),
                      binary_sha256_after={path: inventory.binary_sha256(path) for path in proof.binaries},
                      model_tokenizer_after=inventory.model_cache_inventory(args.model_cache),
                      disk_after=inventory.disk_snapshot(inventory.REPO), finished_at=time.time())
        if report["disk_after"]["free_bytes"] < inventory.STOP_BELOW_FREE_BYTES:
            proof.cases.append({"case": "disk_stop", "status": "FAIL", "error": "less than 10 GiB free; no cleanup attempted"})
        failures = [case["case"] for case in proof.cases if case["status"] not in ("PASS", "NOT_APPLICABLE")]
        if report["source_fingerprints_after"] != report["source_fingerprints_before"]:
            failures.append("source_changed")
        if report["binary_sha256_after"] != report["binary_sha256_before"]:
            failures.append("binary_changed")
        if report["model_tokenizer_after"] != report["model_tokenizer_before"]:
            failures.append("model_tokenizer_changed")
        if not proof.cases:
            failures.append("RUN DID NOT HAPPEN")
        report["failures"] = failures
        report["result"] = "FAIL" if failures else "PASS"
        report["logs"] = {path.name: {"sha256": inventory.binary_sha256(str(path)), "bytes": path.stat().st_size}
                          for path in proof.out.iterdir() if path.is_file() and path.suffix in (".log", ".jsonl")}
        (proof.out / "result.json").write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
        print(json.dumps({"result": report["result"], "failures": failures, "out": str(proof.out)}))
        return int(bool(failures))


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--phase", choices=("all", "s1", "s2", "s3", "s4"), default="all")
    parser.add_argument("--profile", choices=("semantic", "no-semantic"), default="semantic")
    parser.add_argument("--include-source", action="append", default=[],
                        help="explicit new unstaged relevant path; same list as the frozen acceptance run")
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--standalone", type=Path, required=True)
    parser.add_argument("--model-cache", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--helper-binary", type=Path,
                        help="separately preserved CLI with core/test-helpers and matching feature profile; never the production binary")
    parser.add_argument("--reason-config", type=Path, help="explicit real-provider fixture config (not copied from auth stores)")
    parser.add_argument("--acceptance-report", type=Path, help="current-fingerprint sole-owner Rust result.json")
    parser.add_argument("--timeout", type=float, default=180)
    parser.add_argument("--shutdown-bound", type=float, default=20)
    args = parser.parse_args(argv)
    require(args.timeout > 0 and args.shutdown_bound > 0, "deadlines must be positive")
    if args.out.exists() and any(args.out.iterdir()):
        parser.error("--out must be new or empty; existing evidence is never overwritten")
    args.out.mkdir(parents=True, exist_ok=True)
    try:
        return asyncio.run(main_async(args))
    except (ProofFailure, OSError, ValueError) as error:
        (args.out / "result.json").write_text(json.dumps({"result": "FAIL", "error": str(error)}), encoding="utf-8")
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
