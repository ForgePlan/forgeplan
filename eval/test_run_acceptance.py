from __future__ import annotations

import hashlib
import json
import os
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parent))

import run_acceptance as runner


class AcceptanceRunnerTests(unittest.TestCase):
    def test_parses_rust_test_summary_counts(self) -> None:
        output = (
            "test result: ok. 3 passed; 1 failed; 2 ignored; "
            "0 measured; 0 filtered out; finished in 0.01s\n"
        )
        self.assertEqual(
            runner.parse_test_counts(output),
            {"passed": 3, "failed": 1, "ignored": 2},
        )

    def test_build_command_uses_features_and_selected_worktree_target(self) -> None:
        target_dir = runner.REPO / ".local" / "acceptance-target"
        group = {
            "name": "example",
            "package": "forgeplan-core",
            "tests": ["example_suite"],
        }
        declared = {
            "forgeplan-core:example_suite": {
                "package": "forgeplan-core",
                "required_features": ["test-helpers"],
            }
        }

        command = runner.build_group_argv(
            target_dir,
            group,
            "no-semantic",
            declared,
        )

        self.assertEqual(command[command.index("--target-dir") + 1], str(target_dir))
        self.assertEqual(command[command.index("--features") + 1], "test-helpers")
        self.assertEqual(command[command.index("--test") + 1], "example_suite")

    def test_cli_package_does_not_receive_core_only_features(self) -> None:
        command = runner.build_group_argv(
            runner.REPO / ".local" / "acceptance-target",
            {
                "name": "cli",
                "package": "forgeplan",
                "tests": ["cli_migrated_write"],
            },
            "no-semantic",
            {
                "forgeplan:cli_migrated_write": {
                    "package": "forgeplan",
                    "required_features": [],
                }
            },
        )
        self.assertNotIn("--features", command)

    def test_workspace_features_are_package_qualified(self) -> None:
        features = runner.workspace_features_for("no-semantic")
        self.assertEqual(features, ["forgeplan-core/test-helpers"])

    def test_missing_targets_do_not_hide_available_groups(self) -> None:
        groups = [
            {
                "name": "core",
                "package": "forgeplan-core",
                "tests": ["present_suite", "missing_suite"],
            },
            {
                "name": "mcp",
                "package": "forgeplan-mcp",
                "tests": ["missing_group"],
            },
        ]
        declared = {
            "forgeplan-core:present_suite": {
                "package": "forgeplan-core",
                "required_features": [],
            }
        }

        self.assertEqual(
            runner.missing_targets(groups, declared),
            ["forgeplan-core:missing_suite", "forgeplan-mcp:missing_group"],
        )
        available, not_applicable, blocked = runner.runnable_groups(
            groups, declared, "no-semantic"
        )
        self.assertEqual([group["name"] for group in available], ["core"])
        self.assertEqual(available[0]["tests"], ["present_suite"])
        self.assertEqual(not_applicable, [])
        self.assertEqual(blocked, [])

    def test_semantic_only_target_is_n_a_in_no_semantic_profile(self) -> None:
        groups = [
            {
                "name": "semantic",
                "package": "forgeplan-core",
                "tests": ["semantic_suite"],
                "required_features": ["semantic-search"],
            }
        ]
        declared = {
            "forgeplan-core:semantic_suite": {
                "package": "forgeplan-core",
                "required_features": [],
            }
        }
        available, not_applicable, blocked = runner.runnable_groups(
            groups, declared, "no-semantic"
        )
        self.assertEqual(available, [])
        self.assertEqual(not_applicable, ["forgeplan-core:semantic_suite"])
        self.assertEqual(blocked, [])
        command = runner.build_group_argv(
            runner.REPO / ".local" / "acceptance-target",
            groups[0],
            "semantic",
            declared,
        )
        self.assertIn(
            "semantic-search",
            command[command.index("--features") + 1],
        )

    def test_profile_does_not_silently_enable_missing_required_features(self) -> None:
        with self.assertRaisesRegex(runner.Failure, "omits .*required features"):
            runner.features_for(
                "no-semantic",
                {"required_features": ["semantic-search"]},
            )

    def test_command_wrapper_rejects_source_change_during_run(self) -> None:
        before = {"production": "before"}
        after = {"production": "after"}
        process = Mock()
        process.communicate.return_value = ("", "")
        process.returncode = 0
        with (
            patch.object(runner, "source_fingerprints", side_effect=[before, after]),
            patch.object(runner.subprocess, "Popen", return_value=process),
            patch.object(runner, "log"),
        ):
            with self.assertRaisesRegex(runner.Failure, "files changed"):
                runner.run(["cargo", "check"], runner.REPO, timeout=1)

    def test_zero_doctests_are_not_reported_as_a_pass(self) -> None:
        completed = runner.CommandResult(
            returncode=0,
            stdout="test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\\n",
            stderr="",
            source_fingerprints_before={},
            source_fingerprints_after={},
        )
        with (
            patch.object(runner, "run", return_value=completed),
            patch.object(runner, "free_bytes", return_value=100),
        ):
            result = runner.doctests(
                runner.REPO, runner.REPO / "target", "no-semantic", timeout=1
            )
        self.assertTrue(result["ran_nothing"])

    def test_recognizes_unit_test_artifacts_for_lib_and_bin_targets(self) -> None:
        for kind in ("test", "lib", "bin"):
            with self.subTest(kind=kind):
                self.assertTrue(
                    runner.is_test_executable_artifact(
                        {
                            "reason": "compiler-artifact",
                            "profile": {"test": True},
                            "target": {"kind": [kind]},
                        }
                    )
                )
        self.assertFalse(
            runner.is_test_executable_artifact(
                {
                    "reason": "compiler-artifact",
                    "profile": {"test": False},
                    "target": {"kind": ["bin"]},
                }
            )
        )

    def test_cargo_json_retains_complete_test_binary_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            binary = Path(temp_dir) / "example_suite-abc123"
            binary.write_bytes(b"linked test executable")
            messages = [
                {
                    "reason": "compiler-artifact",
                    "profile": {"test": False},
                    "target": {"kind": ["bin"]},
                    "executable": str(Path(temp_dir) / "ordinary-program"),
                },
                {
                    "reason": "compiler-artifact",
                    "profile": {"test": True},
                    "target": {"kind": ["test"]},
                    "executable": str(binary),
                },
                {
                    "reason": "compiler-artifact",
                    "profile": {"test": True},
                    "target": {"kind": ["lib"]},
                    "executable": None,
                },
            ]
            completed = runner.CommandResult(
                returncode=0,
                stdout="\n".join(json.dumps(message) for message in messages),
                stderr="",
                source_fingerprints_before={},
                source_fingerprints_after={},
            )
            with (
                patch.object(runner, "run", return_value=completed),
                patch.object(runner, "free_bytes", return_value=20 * 1024**3),
            ):
                result = runner.build_group(
                    runner.REPO,
                    runner.REPO / "target",
                    {"name": "example", "package": "forgeplan-core", "tests": ["example_suite"]},
                    "no-semantic",
                    {"forgeplan-core:example_suite": {"package": "forgeplan-core", "required_features": []}},
                    timeout=1,
                )
            self.assertEqual(result["executables"], [str(binary)])
            self.assertEqual(
                result["executable_sha256"],
                {str(binary): hashlib.sha256(binary.read_bytes()).hexdigest()},
            )

    def test_target_dir_rejects_neighbor_worktree_and_dry_run_does_not_create(self) -> None:
        with tempfile.TemporaryDirectory(dir=runner.REPO) as temp_dir:
            missing_target = Path(temp_dir) / "not-created"
            resolved = runner.target_dir(
                {"target_directory": str(runner.REPO / "target")},
                str(missing_target),
                create=False,
            )
            self.assertEqual(resolved, missing_target)
            self.assertFalse(missing_target.exists())

        neighbor = runner.REPO.parent / f"{runner.REPO.name}-other/target"
        with self.assertRaisesRegex(runner.Failure, "outside this worktree"):
            runner.target_dir(
                {"target_directory": str(runner.REPO / "target")},
                str(neighbor),
                create=False,
            )

    def test_relative_target_directory_is_based_on_this_worktree(self) -> None:
        resolved = runner.target_dir(
            {"target_directory": str(runner.REPO / "target")},
            ".local/acceptance-target",
            create=False,
        )
        self.assertEqual(resolved, runner.REPO / ".local/acceptance-target")

    def test_empty_phase_is_reported_as_failure(self) -> None:
        with tempfile.TemporaryDirectory(dir=runner.REPO) as temp_dir:
            root = Path(temp_dir)
            out = root / "results"
            metadata = {
                "packages": [],
                "target_directory": str(root / "target"),
            }
            with (
                patch.object(
                    runner,
                    "free_bytes",
                    return_value=runner.REQUIRED_MIN_FREE_BYTES,
                ),
                patch.object(runner, "cargo_metadata", return_value=metadata),
                patch.object(runner, "log"),
                patch.dict(os.environ, {"FORGEPLAN_MODEL_CACHE": "test-cache"}),
            ):
                status = runner.main(
                    [
                        "--phase",
                        "s3",
                        "--profile",
                        "no-semantic",
                        "--out",
                        str(out),
                        "--target-dir",
                        str(root / "target"),
                    ]
                )
            self.assertEqual(status, 1)
            report = json.loads((out / "result.json").read_text())
            self.assertEqual(report["result"], "FAIL")
            self.assertIn(
                "s3/no-semantic: no acceptance suites are declared",
                report["failures"],
            )

    def test_binary_hash_covers_the_file_and_missing_binary_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            binary = Path(temp_dir) / "test-binary"
            binary.write_bytes(b"compiled test payload")
            self.assertEqual(
                runner.binary_sha256(str(binary)),
                hashlib.sha256(b"compiled test payload").hexdigest(),
            )

            missing = Path(temp_dir) / "missing-binary"
            with (
                patch.object(runner, "free_bytes", return_value=100),
                patch.object(runner, "source_fingerprints", return_value={}),
            ):
                result = runner.execute(str(missing), timeout=1)
            self.assertEqual(result["exit"], 127)
            self.assertIsNone(result["binary_sha256_before"])
            self.assertIsNone(result["binary_sha256_after"])
            self.assertIn("could not start", result["stderr_tail"])

    def test_command_timeout_kills_descendants_and_preserves_output(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            marker = Path(temp_dir) / "surviving-child"
            child = (
                "import pathlib,sys,time; time.sleep(0.6); "
                "pathlib.Path(sys.argv[1]).write_text('survived')"
            )
            parent = (
                "import subprocess,sys,time; "
                "subprocess.Popen([sys.executable,'-c',sys.argv[1],sys.argv[2]]); "
                "print('started', flush=True); time.sleep(5)"
            )
            command = [sys.executable, "-c", parent, child, str(marker)]
            with (
                patch.object(runner, "source_fingerprints", return_value={}),
                patch.object(runner, "log"),
            ):
                result = runner.run(command, runner.REPO, timeout=0.2)
            time.sleep(0.7)
            marker_created = marker.exists()

        self.assertTrue(result.timed_out)
        self.assertEqual(result.returncode, 124)
        self.assertIn("started", result.stdout)
        self.assertIn("timeout after", result.stderr)
        self.assertFalse(marker_created, "a child process survived the timed-out run")


if __name__ == "__main__":
    unittest.main()
