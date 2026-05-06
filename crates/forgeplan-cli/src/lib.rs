//! Library facade exposing select CLI command modules to integration tests.
//!
//! The `forgeplan` crate is primarily a binary (`bin/forgeplan`) — but a few
//! command implementations need to be tested **in-process** (not via
//! `assert_cmd` subprocess) to verify deterministic logic without
//! per-iteration fork/exec overhead.
//!
//! Currently exposed:
//! - `commands::ci_assign_id` — PROB-060 Phase 0b stress test
//!   (`tests/prob_060_stress_test.rs`) calls `run` directly to merge 10 PR
//!   branches under 100 seeded permutations in <30 s.
//!
//! Adding a fresh module here is intentional friction: prefer subprocess
//! integration tests for end-to-end behavior; reach for the in-process
//! library facade only when the test budget or determinism demands it.

pub mod commands;
pub(crate) mod ui;
