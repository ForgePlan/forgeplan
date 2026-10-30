//! Public ingest must find an existing marked import behind an unmarked title collision.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use forgeplan_core::ingest::extract_existing_source_hash;
use serde_json::{Value, json};
use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    home: PathBuf,
    cache: PathBuf,
    mapping: PathBuf,
    source: PathBuf,
    migrated: bool,
}

impl Fixture {
    fn new(migrated: bool) -> Self {
        let temp = TempDir::new().expect("sandbox");
        let root = temp.path().join("project");
        let home = temp.path().join("home");
        let cache = temp.path().join("empty-model-cache");
        for path in [&root, &home, &cache] {
            std::fs::create_dir(path).expect("sandbox directory");
        }
        let fixture = Self {
            mapping: root.join("native-mapping.yaml"),
            source: root.join("native-source.md"),
            root,
            home,
            cache,
            migrated,
            _temp: temp,
        };
        fixture.ok(&["init", "-y"]);
        let notes = fixture.root.join(".forgeplan/notes");
        std::fs::write(
            notes.join("NOTE-001-imported-native-note.md"),
            "---\nid: NOTE-001\nkind: note\ntitle: Imported native note\nstatus: draft\ndepth: tactical\n---\n\n\
             ---\nid: NOTE-001\nkind: note\ntitle: Imported native note\nstatus: draft\ncreated: 2026-10-24\n---\n\n\
             ## Note\n\nThis independently authored note must never be overwritten by ingestion.\n",
        )
        .expect("unmarked same-title artifact");
        std::fs::write(
            notes.join("NOTE-002-existing-target.md"),
            "---\nid: NOTE-002\nkind: note\ntitle: Existing target\nstatus: draft\ndepth: tactical\n---\n\n\
             ## Note\n\nThe imported note keeps its based_on edge to this artifact.\n",
        )
        .expect("relation target");
        let reindex = fixture.ok(&["reindex"]);
        assert!(!reindex.contains("create failed"), "{reindex}");
        assert_eq!(
            fixture.json(&["get", "NOTE-001", "--json"])["title"],
            "Imported native note"
        );
        if migrated {
            fixture.ok(&["index"]);
        }
        fixture.assert_owner();
        std::fs::write(
            &fixture.source,
            "# Imported\n\nImportedneedle first revision.\n",
        )
        .expect("source");
        std::fs::write(&fixture.mapping, MAPPING).expect("mapping");
        fixture
    }

    fn command(&self) -> Command {
        let mut command = Command::cargo_bin("forgeplan").expect("built forgeplan binary");
        command
            .current_dir(&self.root)
            .env("HOME", &self.home)
            .env("FORGEPLAN_MODEL_CACHE", &self.cache)
            .env("HF_HUB_OFFLINE", "1");
        command
    }

    fn ok(&self, args: &[&str]) -> String {
        let output = self.command().args(args).output().expect("CLI process");
        assert!(
            output.status.success(),
            "forgeplan {args:?}: {}\n{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout),
        );
        String::from_utf8(output.stdout).expect("UTF-8 stdout")
    }

    fn json(&self, args: &[&str]) -> Value {
        let stdout = self.ok(args);
        serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{args:?}: {error}: {stdout}"))
    }

    fn ingest(&self, update: bool, dry_run: bool) -> Value {
        let mut command = self.command();
        command
            .arg("ingest")
            .arg("--mapping")
            .arg(&self.mapping)
            .arg("--source")
            .arg(&self.source)
            .arg("--json");
        if update {
            command.arg("--update");
        }
        if dry_run {
            command.arg("--dry-run");
        }
        let output = command.output().expect("public ingest");
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout),
        );
        serde_json::from_slice(&output.stdout).expect("ingest operation JSON")
    }

    fn assert_owner(&self) {
        let lance = self.root.join(".forgeplan/lance");
        assert_eq!(lance.join("CURRENT.json").exists(), self.migrated);
        if self.migrated {
            for table in ["artifacts.lance", "relations.lance"] {
                assert!(
                    !lance.join(table).exists(),
                    "retired table recreated: {table}"
                );
            }
        }
    }
}

const MAPPING: &str = r#"schema_version: "1.0"
name: native-import
title: Native import
compat_spec_version: "c4-architecture: ^1.0"
source_kind: c4-documentation
target_kind: forge
sources:
  - pattern: "**/*.md"
    type: markdown
    parser: markdown_only
rules:
  - id: native-note
    when:
      file_glob: "**/*.md"
      heading_path: ["Imported"]
    target: { kind: note }
    fields:
      title: "Imported native note"
      summary: "{{ section.body }}"
    sources_section:
      include: true
      format: "{path}:{line_start}-{line_end}"
      precision: line
      source_hash: true
    links:
      - target_artifact_id: "NOTE-002"
        relation: based_on
        if_exists: skip
"#;

/// The document under `dir` that declares `<id>`.
///
/// The file is named after the document's canonical key (ADR-030), so the
/// display id is read from the frontmatter the document declares.
fn document_declaring(dir: &Path, id: &str) -> PathBuf {
    std::fs::read_dir(dir)
        .expect("directory")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .find(|path| {
            let Ok(text) = std::fs::read_to_string(path) else {
                return false;
            };
            text.lines()
                .take(12)
                .filter_map(|line| line.strip_prefix("id: "))
                .map(|value| value.trim().trim_matches('"').trim_matches('\''))
                .any(|declared| declared == id)
        })
        .unwrap_or_else(|| panic!("no document declaring {id} in {}", dir.display()))
}

fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut result = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).expect("directory") {
            let entry = entry.expect("entry");
            let path = entry.path();
            if entry.file_type().expect("type").is_dir() {
                pending.push(path);
            } else {
                result.insert(
                    path.strip_prefix(root)
                        .expect("relative path")
                        .to_path_buf(),
                    std::fs::read(path).expect("file bytes"),
                );
            }
        }
    }
    result
}

fn assert_no_write(report: &Value, id: &str, reason: &str) {
    assert_eq!(report["written"], json!([]));
    assert_eq!(report["write_errors"], json!([]));
    assert_eq!(report["errors"], json!([]));
    assert_eq!(
        report["skipped_existing"],
        json!([format!("{id} {reason}")])
    );
}

#[test]
fn public_ingest_reuses_the_native_import_and_updates_only_that_record() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated);
        let ws = fixture.root.join(".forgeplan");
        let notes = ws.join("notes");
        let before = files(&ws);
        let dry = fixture.ingest(true, true);
        assert_eq!(dry["drafts"].as_array().expect("planned drafts").len(), 1);
        assert_eq!(
            files(&ws),
            before,
            "dry-run cannot alter any workspace bytes"
        );

        let playbook = fixture.root.join("native-playbook.yaml");
        std::fs::write(
            &playbook,
            serde_yaml::to_string(&json!({
                "schema_version": "1.0",
                "name": "native-ingest-collision",
                "title": "Native ingest collision",
                "steps": [{
                    "id": "import",
                    "delegate_to": {"type": "forgeplan_core", "target": "ingest"},
                    "input": {"mapping_path": fixture.mapping, "source_path": fixture.source, "update": true},
                }],
            }))
            .expect("playbook YAML"),
        )
        .expect("playbook");
        let native = fixture.json(&[
            "playbook",
            "run",
            playbook.to_str().expect("path"),
            "--yes",
            "--json",
        ]);
        assert_eq!(native["report"]["failed"], 0);
        // The imported document is filed under its canonical key (ADR-030).
        let imported = document_declaring(&notes, "NOTE-003");
        let initial_body = std::fs::read_to_string(&imported).expect("native marked import");
        let initial_hash =
            extract_existing_source_hash(&initial_body).expect("persisted source hash");
        assert!(initial_body.contains("Importedneedle first revision."));
        let initial_notes = files(&notes);
        let unmarked = std::fs::read(document_declaring(&notes, "NOTE-001"))
            .expect("unmarked bytes");
        let target =
            std::fs::read(notes.join("NOTE-002-existing-target.md")).expect("target bytes");

        for _ in 0..2 {
            let repeat = fixture.ingest(true, false);
            assert_no_write(&repeat, "NOTE-003", "unchanged (hash match)");
            assert_eq!(repeat["drafts"][0]["source_hash"], initial_hash);
            assert_eq!(
                files(&notes),
                initial_notes,
                "public ingest must not create NOTE-004 or rewrite the collision"
            );
        }

        std::fs::write(
            &fixture.source,
            "# Imported\n\nCollisionupdate second revision.\n",
        )
        .expect("changed source");
        let skipped = fixture.ingest(false, false);
        assert_no_write(&skipped, "NOTE-003", "differs but --update not passed");
        assert_eq!(files(&notes), initial_notes);
        let dry_before = files(&ws);
        fixture.ingest(true, true);
        assert_eq!(files(&ws), dry_before);

        let updated = fixture.ingest(true, false);
        assert_eq!(updated["written"].as_array().expect("writes").len(), 1);
        assert_eq!(updated["written"][0]["id"], "NOTE-003");
        assert_eq!(updated["write_errors"], json!([]));
        assert_eq!(updated["skipped_existing"], json!([]));
        let changed_body = std::fs::read_to_string(&imported).expect("updated import");
        let changed_hash = extract_existing_source_hash(&changed_body).expect("updated hash");
        assert_ne!(changed_hash, initial_hash);
        assert_eq!(updated["drafts"][0]["source_hash"], changed_hash);
        assert!(changed_body.contains("Collisionupdate second revision."));
        assert!(!changed_body.contains("Importedneedle first revision."));
        let mut expected_notes = initial_notes;
        expected_notes.insert(
            imported.file_name().map(PathBuf::from).expect("file name"),
            changed_body.as_bytes().to_vec(),
        );
        assert_eq!(
            files(&notes),
            expected_notes,
            "update must change only the already imported file"
        );
        assert_eq!(
            std::fs::read(document_declaring(&notes, "NOTE-001")).expect("collision"),
            unmarked
        );
        assert_eq!(
            std::fs::read(notes.join("NOTE-002-existing-target.md")).expect("target"),
            target
        );
        let get = fixture.json(&["get", "NOTE-003", "--json"]);
        assert!(
            get["body"]
                .as_str()
                .expect("body")
                .contains("Collisionupdate second revision.")
        );
        // The edge names the document by its display id and carries the
        // canonical key beside it (ADR-030).
        let outbound = get["links"]["outbound"].as_array().expect("outbound");
        assert_eq!(outbound.len(), 1);
        assert_eq!(outbound[0]["target"], "NOTE-002");
        assert_eq!(outbound[0]["relation"], "based_on");
        assert!(
            outbound[0]["target_key"].as_str().is_some_and(|key| !key.is_empty()),
            "the edge must carry the canonical key: {outbound:?}"
        );
        assert_no_write(
            &fixture.ingest(true, false),
            "NOTE-003",
            "unchanged (hash match)",
        );
        assert_eq!(files(&notes), expected_notes);
        fixture.assert_owner();
    }
}

#[test]
fn public_ingest_creates_one_marked_import_then_reuses_it_behind_the_collision() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated);
        let notes = fixture.root.join(".forgeplan/notes");
        let mut expected = files(&notes);
        let first = fixture.ingest(true, false);
        assert_eq!(
            first["written"].as_array().expect("created import").len(),
            1
        );
        assert_eq!(first["written"][0]["id"], "NOTE-003");
        assert_eq!(first["write_errors"], json!([]));
        // The created document is filed under its canonical key (ADR-030),
        // so the file is found by the display id it declares.
        let path = document_declaring(&notes, "NOTE-003");
        let body = std::fs::read_to_string(&path).expect("created import");
        assert_eq!(
            extract_existing_source_hash(&body).as_deref(),
            first["drafts"][0]["source_hash"].as_str(),
        );
        assert!(
            expected
                .insert(
                    path.file_name().map(PathBuf::from).expect("file name"),
                    body.into_bytes()
                )
                .is_none(),
        );
        assert_eq!(files(&notes), expected);
        for update in [false, true] {
            let repeat = fixture.ingest(update, false);
            assert_no_write(&repeat, "NOTE-003", "unchanged (hash match)");
            assert_eq!(files(&notes), expected);
        }
        std::fs::write(
            &fixture.source,
            "# Imported\n\nPubliccreation changed revision.\n",
        )
        .expect("changed source");
        assert_no_write(
            &fixture.ingest(false, false),
            "NOTE-003",
            "differs but --update not passed",
        );
        assert_eq!(files(&notes), expected);
        let updated = fixture.ingest(true, false);
        assert_eq!(
            updated["written"].as_array().expect("updated import").len(),
            1
        );
        assert_eq!(updated["written"][0]["id"], "NOTE-003");
        let updated_body = std::fs::read_to_string(&path).expect("updated body");
        assert!(updated_body.contains("Publiccreation changed revision."));
        expected.insert(
            path.file_name().map(PathBuf::from).expect("file name"),
            updated_body.into_bytes(),
        );
        assert_eq!(files(&notes), expected);
        assert_no_write(
            &fixture.ingest(true, false),
            "NOTE-003",
            "unchanged (hash match)",
        );
        assert_eq!(files(&notes), expected);
        fixture.assert_owner();
    }
}
