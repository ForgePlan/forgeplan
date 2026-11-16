"""Regression tests for F10 evidence attribution and result accounting."""

import json
import unittest
import tempfile
from pathlib import Path
from datetime import datetime, timezone

import run_eval


class EvidenceAccountingTests(unittest.TestCase):
    def task(self) -> dict:
        return {
            "id": "T", "kind": "search", "must_include": ["mandatory exception"],
            "must_not_include": [], "expected_sources": ["ws-a"],
            "expected_artifacts": ["SPEC-001"], "expect_found": True,
        }

    def test_echoed_request_and_omitted_target_do_not_count_as_evidence(self):
        outcome = run_eval.Outcome("pack")
        outcome.runs.append(run_eval.Run(
            ["forgeplan", "pack"], Path("."), 0,
            json.dumps({"task": "mandatory exception", "omitted": ["SPEC-001"]}), "", 1,
        ))
        outcome.result = "not_found"
        row = run_eval.score(self.task(), outcome, {"ws-a": {"SPEC-001"}})
        self.assertEqual(row["must_include_missing"], ["mandatory exception"])
        self.assertEqual(row["constraint_recall"], 0)
        self.assertTrue(row["incomplete_negative"])

    def test_unattributed_document_is_not_a_clean_boundary_check(self):
        outcome = run_eval.Outcome("pack")
        outcome.hits.append({"key": "SPEC-001", "source_ws": None,
                             "evidence": "mandatory exception"})
        row = run_eval.score(self.task(), outcome, {"ws-a": {"SPEC-001"}})
        self.assertTrue(row["source_boundary_violation"])

    def test_version_requires_both_source_identity_and_pinned_revision(self):
        versions = {"ws-a": {"source_id": "src:a", "pinned_version": "oid:tree:digest"}}
        source = {"path": "/tmp/ws-a/.forgeplan", "source_id": "src:a",
                  "version_key": "oid:tree:digest"}
        self.assertTrue(run_eval.validate_version(source, versions))
        self.assertFalse(run_eval.validate_version({**source, "source_id": "src:b"}, versions))
        self.assertFalse(run_eval.validate_version({**source, "version_key": "new"}, versions))
        self.assertFalse(run_eval.validate_version(source, {}))

    def test_receipt_is_checked_against_emitted_bytes_not_trusted(self):
        outcome = run_eval.Outcome("pack")
        text = json.dumps({
            "units": [], "sources": [], "source_complete": True,
            "traversal_complete": True, "delivery_complete": True,
            "receipt": {"measured_bytes": 1, "byte_cap": 2},
        }) + "\n"
        run = run_eval.Run(["forgeplan", "pack"], Path("."), 0, text, "", 1)
        run_eval.parse_pack(run, outcome, {})
        row = run_eval.score(self.task(), outcome, {"ws-a": {"SPEC-001"}})
        self.assertTrue(row["pack_contract_violation"])
        self.assertEqual(len(row["contract_errors"]), 2)

    def test_body_words_do_not_turn_an_existing_document_into_not_found(self):
        outcome = run_eval.Outcome("local")
        run = run_eval.Run(["forgeplan", "get"], Path("."), 0, json.dumps({
            "id": "SPEC-001", "body": "A missing path must return not found.",
            "title": "Error policy",
        }), "", 1)
        run_eval.parse_get_local(run, outcome, "ws-a", None)
        self.assertEqual(outcome.result, "found")
        self.assertEqual(outcome.hits[0]["key"], "SPEC-001")

    def test_failed_read_is_not_scored_as_a_clean_absence(self):
        outcome = run_eval.Outcome("local")
        run = run_eval.Run(["forgeplan", "get"], Path("."), 1, "",
                           "source_unavailable: cannot read source", 1)
        run_eval.parse_get_local(run, outcome, "ws-a", None)
        self.assertEqual(outcome.result, "error")
        row = run_eval.score(self.task(), outcome, {"ws-a": {"SPEC-001"}})
        self.assertTrue(row["incomplete_negative"])

    def test_a_matching_substring_with_wrong_span_is_not_a_valid_citation(self):
        body = '# Rule\nAlways encrypt café.\nException: archived copies also stay encrypted.\n'
        raw = ('---\nid: SPEC-001\nkind: spec\n---\n\n' + body).encode()
        unit = {
            'excerpt': body,
            'span': {'coordinate_basis': 'artifact_body_utf8_bytes',
                     'byte_start': 0, 'byte_end': len(body.encode())},
        }
        self.assertTrue(run_eval.valid_whole_body_quote(raw, unit))
        self.assertFalse(run_eval.valid_whole_body_quote(raw, {
            **unit, 'excerpt': 'Always encrypt café.',
            'span': {**unit['span'], 'byte_start': 7, 'byte_end': 29},
        }))
        self.assertFalse(run_eval.valid_whole_body_quote(raw, {
            **unit, 'span': {**unit['span'], 'coordinate_basis': 'file_utf8_bytes'},
        }))

    def test_ambiguity_counts_returned_identity_but_not_requested_body_text(self):
        task = {
            **self.task(), "kind": "get", "expected_sources": ["ws-a", "ws-b"],
            "expected_artifacts": ["ADR-001"], "expect_found": "ambiguous",
            "must_include": ["ADR-001", "secret policy"],
        }
        outcome = run_eval.Outcome("federated")
        run = run_eval.Run(["forgeplan", "get", "secret policy"], Path("."), 0,
                           json.dumps({"result": "ambiguous", "id": "ADR-001",
                                       "sources": [{"source_id": "src:a"},
                                                   {"source_id": "src:b"}]}), "", 1)
        versions = {"ws-a": {"source_id": "src:a"}, "ws-b": {"source_id": "src:b"}}
        run_eval.parse_get_fed(run, outcome, versions)
        row = run_eval.score(task, outcome, {"ws-a": {"ADR-001"}, "ws-b": {"ADR-001"}})
        self.assertEqual(row["must_include_found"], ["ADR-001"])
        self.assertEqual(row["must_include_missing"], ["secret policy"])
        self.assertFalse(row["incomplete_negative"])
        self.assertFalse(row["source_boundary_violation"])

    def test_first_useful_time_excludes_responses_after_useful_material(self):
        for response_index, expected_ms in ((0, 10), (1, 30)):
            with self.subTest(response_index=response_index):
                outcome = run_eval.Outcome("pack")
                outcome.runs = [
                    run_eval.Run(["forgeplan"], Path("."), 0, "", "", elapsed)
                    for elapsed in (10, 20, 100)
                ]
                outcome.hits = [{
                    "key": "SPEC-001", "title": "Rule", "source_ws": "ws-a",
                    "evidence": "mandatory exception",
                    "received_in_run": response_index,
                }]
                row = run_eval.score(self.task(), outcome, {"ws-a": {"SPEC-001"}})
                self.assertEqual(row["first_useful_ms"], expected_ms)
                self.assertEqual(row["latency_ms"], 130)

    def test_snapshot_timestamp_remains_json_serializable(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / "snapshot.yaml"
            path.write_text(
                "snapshot_id: snap-fixed\n"
                "taken_at: 2026-11-13T13:56:16Z\n"
                "sources: []\nskipped: []\n", encoding="utf-8",
            )
            lock = run_eval.load_snapshot_lock(path)
        restored = json.loads(json.dumps(lock))
        self.assertEqual(
            datetime.fromisoformat(restored["taken_at"].replace("Z", "+00:00")),
            datetime(2026, 11, 13, 13, 56, 16, tzinfo=timezone.utc),
        )


if __name__ == "__main__":
    unittest.main()
