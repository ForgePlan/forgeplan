//! Does the tract engine compute the same thing as the engine it replaces?
//!
//! RFC-013 Phase 2 decision point. If this fails, the replacement is abandoned
//! and we fall back to loading ONNX Runtime at runtime — having lost only this
//! phase, because nothing has been removed yet.
//!
//! Checked against the frozen oracle in
//! `tests/fixtures/embedding_reference.json`, captured from the old engine in
//! Phase 1. Comparing against the fixture rather than against a live fastembed
//! matters: the fixture is what survives once fastembed is gone, so a green
//! result here means the same test keeps working after Phase 3.
//!
//! Requires the model to be present in the local cache. Skips with a stated
//! reason when it is not — a test that quietly passes because it found nothing
//! to run against is worse than one that says so.

#![cfg(feature = "tract-engine")]

use std::path::PathBuf;

/// Per-component tolerance, same as the oracle's. The spike measured 1.6e-07
/// (EVID-159), so this leaves two orders of magnitude of headroom over
/// observed float32 reordering while staying far below any real divergence.
const TOLERANCE: f32 = 1e-6;

const MODEL_REPO: &str = "BAAI/bge-m3";

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/embedding_reference.json")
}

/// Parse the oracle. Same deliberate shape-specific reader as the Phase 1 test.
fn load_cases() -> Vec<(String, String, Vec<f32>)> {
    let raw = std::fs::read_to_string(fixture_path()).expect("oracle fixture must exist");
    let mut cases = Vec::new();
    let mut rest = raw.as_str();

    while let Some(start) = rest.find("\"name\": \"") {
        rest = &rest[start + 9..];
        let name_end = rest.find('"').unwrap();
        let name = rest[..name_end].to_string();

        let text_start = rest.find("\"text\": \"").unwrap() + 9;
        let text = read_json_string(&rest[text_start..]);

        let vec_start = rest.find("\"vector\": [").unwrap() + 11;
        let vec_end = rest[vec_start..].find(']').unwrap();
        let vector: Vec<f32> = rest[vec_start..vec_start + vec_end]
            .split(',')
            .map(|v| v.trim().parse::<f32>().unwrap())
            .collect();

        cases.push((name, text, vector));
        rest = &rest[vec_start + vec_end..];
    }
    cases
}

fn read_json_string(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => out.push(other),
                None => break,
            },
            c => out.push(c),
        }
    }
    out
}

fn load_engine() -> Option<forgeplan_core::embed::tract_engine::TractEmbedder> {
    let cache = forgeplan_core::embed::resolve_cache_dir();
    let snapshot = forgeplan_core::embed::tract_engine::find_snapshot(&cache, MODEL_REPO)?;
    forgeplan_core::embed::tract_engine::TractEmbedder::from_snapshot(&snapshot, "bge-m3", 1024)
        .ok()
}

/// The decision point: tract must reproduce the old engine's vectors.
#[test]
fn tract_reproduces_the_reference_embeddings() {
    let Some(engine) = load_engine() else {
        eprintln!(
            "SKIPPED: no BGE-M3 snapshot in {}. Fetch it with `forgeplan setup`, \
             then re-run — this test is the Phase 2 decision point and a skip \
             proves nothing.",
            forgeplan_core::embed::resolve_cache_dir().display()
        );
        return;
    };

    let cases = load_cases();
    assert!(!cases.is_empty(), "the oracle has no cases");

    let mut failures = Vec::new();

    for (name, text, expected) in &cases {
        let actual = engine
            .embed(text)
            .unwrap_or_else(|e| panic!("case `{name}` failed to embed on tract: {e}"));

        if actual.len() != expected.len() {
            failures.push(format!(
                "  {name}: dimension {} != expected {}",
                actual.len(),
                expected.len()
            ));
            continue;
        }

        let mut worst = 0.0f32;
        let mut worst_at = 0usize;
        for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
            let d = (a - e).abs();
            if d > worst {
                worst = d;
                worst_at = i;
            }
        }

        if worst > TOLERANCE {
            failures.push(format!(
                "  {name}: max deviation {worst:.3e} at component {worst_at} \
                 (tract {:.9}, reference {:.9})",
                actual[worst_at], expected[worst_at]
            ));
        } else {
            eprintln!("  {name:<20} max deviation {worst:.3e}  OK");
        }
    }

    assert!(
        failures.is_empty(),
        "tract does NOT reproduce the reference embeddings:\n{}\n\n\
         Per RFC-013 this is the decision point — the replacement stops here \
         and the runtime-loaded ONNX fallback is taken instead. Nothing has \
         been removed yet, so the cost is this phase only.",
        failures.join("\n")
    );
}

/// Batch and single-text paths must agree with each other. If they diverge,
/// indexing and querying would embed the same text differently, which makes
/// every similarity score subtly wrong without anything failing.
#[test]
fn batch_matches_single() {
    let Some(engine) = load_engine() else {
        eprintln!("SKIPPED: no model snapshot available");
        return;
    };

    let texts = ["first text for the batch", "второй текст для батча"];
    let batched = engine.embed_batch(&texts).expect("batch must succeed");

    for (i, text) in texts.iter().enumerate() {
        let single = engine.embed(text).expect("single must succeed");
        let worst = batched[i]
            .iter()
            .zip(single.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(
            worst <= TOLERANCE,
            "batch and single disagree on `{text}` by {worst:.3e}"
        );
    }
}

/// Output must be unit length: the index compares by cosine similarity, and an
/// unnormalised vector silently changes every score it participates in.
#[test]
fn output_is_unit_length() {
    let Some(engine) = load_engine() else {
        eprintln!("SKIPPED: no model snapshot available");
        return;
    };

    for (name, text) in forgeplan_core::embed::reference_cases() {
        let v = engine.embed(&text).expect("embed must succeed");
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-5,
            "case `{name}` produced a vector of length {norm}, expected 1.0"
        );
    }
}
