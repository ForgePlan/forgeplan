//! Capture reference embeddings from the CURRENT engine (fastembed / ONNX
//! Runtime) so a replacement engine can be verified against them.
//!
//! RFC-013 Phase 1. This must run BEFORE the old engine is removed — afterwards
//! there is nothing left to regenerate these values with, and the correctness
//! check for the replacement becomes impossible.
//!
//! Run with:
//!   cargo run -p forgeplan-core --features semantic-search \
//!       --example capture_embedding_reference
//!
//! Writes `crates/forgeplan-core/tests/fixtures/embedding_reference.json`.
//! Regenerating it is a deliberate act: the file is the oracle, and quietly
//! refreshing it would turn a failing correctness test into a passing one
//! without anyone deciding that the new numbers are right.

#[cfg(feature = "semantic-search")]
use std::io::Write;

/// Without the feature there is no engine to capture from. Say so and exit
/// cleanly rather than failing to compile: `cargo clippy --workspace
/// --all-targets` builds examples in BOTH feature configurations, and a CI
/// gate that cannot build the default config is a broken gate.
#[cfg(not(feature = "semantic-search"))]
fn main() {
    eprintln!(
        "This generator needs the semantic-search feature — there is no engine \
         to capture reference vectors from otherwise.\n\
         Run: cargo run -p forgeplan-core --features semantic-search \
         --example capture_embedding_reference"
    );
    std::process::exit(1);
}

#[cfg(feature = "semantic-search")]
fn main() -> anyhow::Result<()> {
    let cases = forgeplan_core::embed::reference_cases();
    let mut embedder = forgeplan_core::embed::Embedder::new()?;

    eprintln!("model: {}", embedder.model_name());
    eprintln!("dim:   {}", embedder.dim());
    eprintln!("cases: {}", cases.len());

    let mut entries = Vec::with_capacity(cases.len());
    for (name, text) in cases {
        let vector = embedder.embed(&text)?;
        eprintln!("  {name:<24} {} chars -> {} dims", text.len(), vector.len());
        entries.push((name, text, vector));
    }

    // Hand-rolled JSON: forgeplan-core has serde, but writing this by hand
    // keeps the fixture format obvious to a reader who has never seen the
    // generator, and the file is read back by a test that parses it the same
    // deliberate way.
    let mut out = String::from("{\n");
    out.push_str(&format!("  \"model\": \"{}\",\n", embedder.model_name()));
    out.push_str(&format!("  \"dim\": {},\n", embedder.dim()));
    out.push_str(
        "  \"generated_by\": \"fastembed/ort — the pre-tract engine (RFC-013 Phase 1)\",\n",
    );
    out.push_str("  \"cases\": [\n");

    for (i, (name, text, vector)) in entries.iter().enumerate() {
        out.push_str("    {\n");
        out.push_str(&format!("      \"name\": \"{name}\",\n"));
        out.push_str(&format!("      \"text\": {},\n", json_string(text)));
        let values: Vec<String> = vector.iter().map(|v| format!("{v:.9}")).collect();
        out.push_str(&format!("      \"vector\": [{}]\n", values.join(", ")));
        out.push_str(if i + 1 == entries.len() {
            "    }\n"
        } else {
            "    },\n"
        });
    }
    out.push_str("  ]\n}\n");

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/embedding_reference.json");
    std::fs::create_dir_all(path.parent().unwrap())?;
    let mut file = std::fs::File::create(&path)?;
    file.write_all(out.as_bytes())?;

    eprintln!("\nwrote {}", path.display());
    Ok(())
}

/// Escape a string for JSON. Only the cases that occur in our fixtures —
/// quotes, backslashes and control characters.
#[cfg(feature = "semantic-search")]
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
