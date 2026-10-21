"""Executor/proof boundary tests. These do not certify a Forgeplan binary.

All process tests below execute real children. Metadata/response fixtures are
inputs to pure inventory/protocol validation, not mock production acceptance.
"""
from __future__ import annotations

import asyncio
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import run_acceptance as runner
import constellation_readiness as readiness


def metadata_target(name: str, kind: str, *, test: bool = True,
                    features: tuple[str, ...] = ()) -> dict:
    return {"name": name, "kind": [kind], "test": test, "src_path": f"/fixture/{name}.rs",
            "required-features": list(features)}


def metadata(*targets: dict) -> dict:
    return {"workspace_members": ["core-id"], "packages": [
        {"id": "core-id", "name": "forgeplan-core", "targets": list(targets)},
        {"id": "external-id", "name": "not-a-workspace-member", "targets": [metadata_target("external", "lib")]},
    ]}


class AcceptanceRunnerTests(unittest.TestCase):

    def test_empty_mandatory_phase_refuses_execution(self) -> None:
        with patch.dict(runner.PHASE_GROUPS, {"empty-required-phase": []}):
            with self.assertRaisesRegex(runner.Failure, "no acceptance suites"):
                runner.required_phase_groups("empty-required-phase")

    def test_compiler_inventory_refuses_a_missing_executable_even_if_target_was_named(self) -> None:
        records = [
            {"reason": "compiler-artifact", "profile": {"test": True},
             "target": {"kind": ["test"], "name": "present"}, "executable": "/fixture/present"},
            {"reason": "compiler-artifact", "profile": {"test": True},
             "target": {"kind": ["test"], "name": "missing"}, "executable": None},
        ]
        with self.assertRaisesRegex(runner.Failure, "omitted required"):
            runner.compiler_inventory("\n".join(json.dumps(row) for row in records),
                                      {("test", "present"), ("test", "missing")})
    def test_workspace_inventory_includes_library_binary_integration_and_compile_only(self) -> None:
        groups, compile_only = runner.workspace_groups(metadata(
            metadata_target("forgeplan_core", "lib"), metadata_target("tool", "bin"),
            metadata_target("fault", "test", features=("test-helpers",)),
            metadata_target("compile_only", "bin", test=False),
            metadata_target("build-script-build", "custom-build"),
        ))
        self.assertEqual({group["target_key"] for group in groups}, {
            "forgeplan-core:lib:forgeplan_core", "forgeplan-core:bin:tool", "forgeplan-core:test:fault"})
        self.assertEqual(len(compile_only), 1)
        self.assertEqual(compile_only[0]["status"], "NOT_APPLICABLE")
        self.assertIn("test=false", compile_only[0]["reason"])
        self.assertEqual(runner.test_selection_argv(groups[0]), ["--bin", "tool"])

    def test_empty_inventory_is_a_failure_not_an_empty_success(self) -> None:
        with self.assertRaisesRegex(runner.Failure, "no executable"):
            runner.workspace_groups(metadata(metadata_target("build-script-build", "custom-build")))

    def test_duplicate_metadata_target_fails_completeness(self) -> None:
        target = metadata_target("duplicated", "test")
        with self.assertRaisesRegex(runner.Failure, "duplicate target"):
            runner.workspace_groups(metadata(target, target))

    def test_missing_declared_phase_target_remains_an_error(self) -> None:
        groups = [{"name": "faults", "package": "forgeplan-core", "tests": ["present", "missing"]}]
        declared = {"forgeplan-core:present": {"package": "forgeplan-core", "required_features": []}}
        self.assertEqual(runner.missing_targets(groups, declared), ["forgeplan-core:missing"])
        available, not_applicable, blocked = runner.runnable_groups(groups, declared, "no-semantic")
        self.assertEqual(available[0]["tests"], ["present"])
        self.assertEqual(not_applicable, [])
        self.assertEqual(blocked, [])

    def test_workspace_semantic_gate_is_explicit_and_not_silently_enabled(self) -> None:
        groups, _ = runner.workspace_groups(metadata(metadata_target("native", "test", features=("semantic-search",))))
        available, not_applicable, blocked = runner.runnable_groups(groups, {}, "no-semantic")
        self.assertEqual(available, [])
        self.assertEqual(not_applicable, ["forgeplan-core:test:native"])
        self.assertEqual(blocked, [])
        with self.assertRaises(runner.Failure):
            runner.features_for("no-semantic", {"required_features": ["semantic-search"]})

    def test_target_selectors_preserve_exact_names(self) -> None:
        command = runner.build_group_argv(runner.REPO / "target", {
            "name": "actual-suite", "package": "forgeplan-core", "tests": ["exact_fault_name"]},
            "no-semantic", {"forgeplan-core:exact_fault_name": {
                "package": "forgeplan-core", "required_features": ["test-helpers"]}})
        self.assertEqual(command[command.index("--test") + 1], "exact_fault_name")
        self.assertNotIn("--all-features", command)
        self.assertNotIn("semantic-search", command)

    def test_features_are_package_scoped_and_cli_does_not_inherit_test_helpers(self) -> None:
        command = runner.build_group_argv(runner.REPO / "target", {
            "name": "cli", "package": "forgeplan", "tests": ["compat"]}, "no-semantic",
            {"forgeplan:compat": {"package": "forgeplan", "required_features": []}})
        self.assertNotIn("--features", command)
        self.assertEqual(runner.workspace_features_for("no-semantic"), ["forgeplan-core/test-helpers"])

    def test_counts_require_actual_summary_lines(self) -> None:
        self.assertEqual(runner.parse_test_counts("test result: ok. 3 passed; 1 failed; 2 ignored;\n"),
                         {"passed": 3, "failed": 1, "ignored": 2})
        self.assertEqual(runner.parse_test_counts("0 failed; build completed\n"),
                         {"passed": 0, "failed": 0, "ignored": 0})

    def test_real_child_exit_is_not_overridden_by_success_looking_stdout(self) -> None:
        result = runner.run([sys.executable, "-c",
            "import sys; print('test result: ok. 99 passed; 0 failed; 0 ignored;'); sys.exit(17)"],
            runner.REPO, timeout=10)
        self.assertEqual(result.returncode, 17)
        self.assertFalse(result.timed_out)
        self.assertEqual(runner.parse_test_counts(result.stdout)["passed"], 99)

    def test_real_zero_run_executable_is_not_a_pass(self) -> None:
        if os.name == "nt":
            self.fail("this executable-boundary test requires the Unix runner, not a silent SKIP")
        with tempfile.TemporaryDirectory() as temporary:
            executable = Path(temporary) / "zero-cases"
            executable.write_text(f"#!{sys.executable}\nraise SystemExit(0)\n", encoding="utf-8")
            executable.chmod(0o755)
            result = runner.execute(str(executable), timeout=10)
            self.assertEqual(result["exit"], 0)
            self.assertTrue(result["ran_nothing"])
            self.assertEqual(result["counts"], {"passed": 0, "failed": 0, "ignored": 0})

    def test_timeout_retains_actual_signal_exit_and_partial_logs(self) -> None:
        result = runner.run([sys.executable, "-u", "-c",
                             "import time; print('entered', flush=True); time.sleep(60)"],
                            runner.REPO, timeout=0.3)
        self.assertTrue(result.timed_out)
        self.assertNotEqual(result.returncode, 0)
        if os.name != "nt":
            self.assertIn(result.returncode, (-signal.SIGTERM, -signal.SIGKILL))
        self.assertIn("entered", result.stdout)

    def test_full_stream_logs_have_reproducible_hash_and_size(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            with patch.object(runner, "_COMMAND_LOG_DIR", Path(temporary)):
                result = runner.run([sys.executable, "-c", "import sys; sys.stderr.write('failure detail'); sys.exit(3)"],
                                    runner.REPO, timeout=10)
            log = result.logs["stderr"]
            path = Path(log["path"])
            self.assertEqual(path.read_text(), "failure detail")
            self.assertEqual(log["sha256"], hashlib.sha256(path.read_bytes()).hexdigest())
            self.assertEqual(log["bytes"], len(path.read_bytes()))
            self.assertEqual(result.returncode, 3)

    def test_fingerprint_includes_new_owned_file_and_ignores_foreign_caches(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            tracked = root / "crates/forgeplan-core/src/lib.rs"
            tracked.parent.mkdir(parents=True)
            tracked.write_text("pub fn tracked() {}\n", encoding="utf-8")
            (root / "Cargo.lock").write_text("lock\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(root), "add", "Cargo.lock", "crates/forgeplan-core/src/lib.rs"], check=True)
            new = root / "crates/forgeplan-core/tests/new_owned.rs"
            new.parent.mkdir()
            new.write_text("new behavioral test\n", encoding="utf-8")
            foreign = root / ".local/copied-cache.rs"
            foreign.parent.mkdir()
            foreign.write_text("not source\n", encoding="utf-8")
            with patch.object(runner, "REPO", root), \
                 patch.object(runner, "_EXPLICIT_SOURCE_PATHS", {new}), \
                 patch.object(runner, "_METADATA_SOURCE_PATHS", set()):
                before = runner.source_fingerprints()
                manifest = runner.source_inventory()
                self.assertIn("crates/forgeplan-core/tests/new_owned.rs", manifest["tests"])
                foreign.write_text("changed foreign bytes\n", encoding="utf-8")
                self.assertEqual(before, runner.source_fingerprints())
                new.write_text("changed behavioral test\n", encoding="utf-8")
                changed = runner.source_fingerprints()
                self.assertNotEqual(before["tests"], changed["tests"])
                tracked.unlink()
                self.assertNotEqual(changed["production"], runner.source_fingerprints()["production"])

    def test_model_fingerprint_hashes_bytes_not_same_size_and_mtime(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            cache = Path(temporary)
            file = cache / "tokenizer.json"
            file.write_bytes(b"AAA")
            stamp = file.stat()
            before = runner.model_cache_inventory(cache)
            file.write_bytes(b"BBB")
            os.utime(file, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
            self.assertNotEqual(before, runner.model_cache_inventory(cache))

    def test_target_directory_cannot_cross_to_another_worktree(self) -> None:
        with self.assertRaisesRegex(runner.Failure, "outside this worktree"):
            runner.target_dir({"target_directory": str(runner.REPO / "target")},
                              str(runner.REPO.parent / "other-worktree/target"), create=False)


    def test_frozen_patch_contains_real_unstaged_and_untracked_changes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            (root / "Cargo.lock").write_text("old lock\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(root), "add", "Cargo.lock"], check=True)
            subprocess.run(["git", "-C", str(root), "-c", "user.name=Patch Fixture",
                            "-c", "user.email=fixture@forgeplan.invalid",
                            "-c", "commit.gpgsign=false", "commit", "-qm", "Fixture baseline"], check=True)
            (root / "Cargo.lock").write_text("changed lock\n", encoding="utf-8")
            new = root / "crates/forgeplan-core/tests/new_process.rs"
            new.parent.mkdir(parents=True)
            new.write_text("new behavior bytes\n", encoding="utf-8")
            with patch.object(runner, "REPO", root), \
                 patch.object(runner, "_EXPLICIT_SOURCE_PATHS", {new}), \
                 patch.object(runner, "_METADATA_SOURCE_PATHS", set()):
                receipt = runner.freeze_source_patch(root / "source.patch")
            delta = (root / "source.patch").read_bytes()
            self.assertIn(b"+changed lock", delta)
            self.assertIn(b"b/crates/forgeplan-core/tests/new_process.rs", delta)
            self.assertIn(b"+new behavior bytes", delta)
            self.assertEqual(receipt["sha256"], hashlib.sha256(delta).hexdigest())
            self.assertEqual(receipt["bytes"], len(delta))


class ReadinessBoundaryTests(unittest.TestCase):
    def test_incomplete_capture_cannot_masquerade_as_empty_success(self) -> None:
        healthy_looking = {"result": {"content": [{"type": "text", "text": '{"total":0,"partial":false}'}]}}
        with self.assertRaises(readiness.ProofFailure):
            readiness.explicit_failure(healthy_looking, ("source_unavailable",))
        failure = {"result": {"isError": True, "content": [{"type": "text", "text": "source_unavailable: invalid YAML"}]}}
        readiness.explicit_failure(failure, ("source_unavailable",))
        with self.assertRaises(readiness.ProofFailure):
            readiness.payload(failure)

    def test_native_trace_requires_an_unreturned_query_entry(self) -> None:
        self.assertIsNone(readiness.active_native_call("RPC request pending"))
        entry = 'DEBUG event="constellation_native_encode_entered" call_id=12 op="query"'
        self.assertEqual(readiness.active_native_call(entry), ("12", entry))
        returned = 'DEBUG event="constellation_native_encode_returned" call_id=12 op="query" canonical_key="constellation_native_encode_entered"'
        self.assertIsNone(readiness.active_native_call(entry + "\n" + returned))
        self.assertIsNone(readiness.active_native_call('event="constellation_native_encode_entered" call_id=13 op="inference"'))

    def test_reason_config_allows_public_fields_but_refuses_auth_or_unknown_keys(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "reason.yaml"
            path.write_text("llm:\n  provider: codex-cli\n  model: ''\n", encoding="utf-8")
            self.assertEqual(readiness.reason_config(path), {"provider": "codex-cli", "model": ""})
            path.write_text("llm:\n  provider: codex-cli\n  api_key_env: PRIVATE_KEY\n", encoding="utf-8")
            with self.assertRaises(readiness.ProofFailure):
                readiness.reason_config(path)


class ReadinessPrerequisiteTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_missing_cache_blocks_before_any_model_request(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            cache = root / "empty-cache"
            cache.mkdir()
            proof = SimpleNamespace(models=runner.model_cache_inventory(cache))
            with self.assertRaisesRegex(readiness.ProofBlocked, "real ONNX"):
                await readiness.real_embed(proof, root)

    async def test_mandatory_blocked_case_is_persisted_as_blocked_not_pass(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            proof = readiness.Proof.__new__(readiness.Proof)
            proof.out = Path(temporary)
            proof.events = proof.out / "events.jsonl"
            proof.cases = []

            async def unavailable() -> None:
                readiness.prerequisite(False, "explicit missing input")

            await proof.case("mandatory-negative", unavailable)
            recorded = json.loads((proof.out / "cases.json").read_text(encoding="utf-8"))
            self.assertEqual(recorded[0]["status"], "BLOCKED")
            self.assertEqual(recorded[0]["error"], "explicit missing input")


    async def test_metadata_only_cache_candidate_is_blocked_without_requesting_weights(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            cache = Path(temporary)
            snapshot = cache / "models--BAAI--bge-m3/snapshots/metadata-only"
            snapshot.mkdir(parents=True)
            (snapshot / "tokenizer.json").write_text("{}", encoding="utf-8")
            with self.assertRaisesRegex(readiness.ProofBlocked, "onnx/model.onnx_data"):
                readiness.require_cached_model(runner.model_cache_inventory(cache))


class BarrierTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_socket_rejects_another_child_pid(self) -> None:
        async with readiness.Barrier("owner_acquired") as barrier:
            endpoint = barrier.environment()["FORGEPLAN_TEST_BARRIER_ENDPOINT"]
            host, port = endpoint.split(":")
            _, writer = await asyncio.open_connection(host, int(port))
            writer.write(json.dumps({"token": barrier.token, "stage": barrier.stage, "pid": 42}).encode() + b"\n")
            await writer.drain()
            try:
                with self.assertRaises(readiness.ProofFailure):
                    await barrier.reached(99, 1)
            finally:
                writer.close()
                await writer.wait_closed()

    async def test_real_socket_accepts_only_exact_token_stage_pid(self) -> None:
        async with readiness.Barrier("after_current_ack") as barrier:
            endpoint = barrier.environment()["FORGEPLAN_TEST_BARRIER_ENDPOINT"]
            host, port = endpoint.split(":")
            _, writer = await asyncio.open_connection(host, int(port))
            writer.write(json.dumps({"token": barrier.token, "stage": barrier.stage, "pid": 99}).encode() + b"\n")
            await writer.drain()
            try:
                message, _ = await barrier.reached(99, 1)
                self.assertEqual(message["stage"], "after_current_ack")
            finally:
                writer.close()
                await writer.wait_closed()


if __name__ == "__main__":
    unittest.main()
