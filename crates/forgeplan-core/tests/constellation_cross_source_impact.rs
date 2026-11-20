use std::path::{Path, PathBuf};
use std::process::Command;

use forgeplan_core::constellation::federation::{
    FederatedGetResult, QueryMode, SourceSpec, federated_get,
};
use forgeplan_core::constellation::graph::{GraphBounds, GraphOutcome, graph_live, graph_snapshot};
use forgeplan_core::constellation::impact::{
    ImpactBounds, ImpactReport, ImpactSeverity, impact_live, impact_snapshot,
};
use forgeplan_core::constellation::snapshot::{capture_snapshot, snapshot_get};
use tempfile::TempDir;

fn doc(id: &str, kind: &str, links: &[(&str, &str)]) -> String {
    let mut text = format!("---\nid: {id}\nkind: {kind}\nstatus: accepted\ntitle: {id}\n");
    if !links.is_empty() {
        text.push_str("links:\n");
        for (target, relation) in links {
            text.push_str(&format!("  - target: {target}\n    relation: {relation}\n"));
        }
    }
    text.push_str("---\n\nBody.\n");
    text
}

fn git(repo: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

struct Fixture {
    _temp: TempDir,
    hub: PathBuf,
    specs: Vec<SourceSpec>,
}

impl Fixture {
    fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let hub = temp.path().join("hub");
        let shared = temp.path().join("shared");
        for repo in [&hub, &shared] {
            for dir in ["adrs", "prds", "notes"] {
                std::fs::create_dir_all(repo.join(".forgeplan").join(dir)).unwrap();
            }
        }
        let put = |repo: &Path, file: &str, id: &str, kind: &str, links: &[(&str, &str)]| {
            std::fs::write(repo.join(".forgeplan").join(file), doc(id, kind, links)).unwrap();
        };
        put(&hub, "adrs/ADR-001.md", "ADR-001", "adr", &[]);
        put(
            &shared,
            "adrs/ADR-001.md",
            "ADR-001",
            "adr",
            &[("PRD-004@hub", "based_on")],
        );
        put(
            &hub,
            "prds/PRD-002.md",
            "PRD-002",
            "prd",
            &[("ADR-001@shared", "based_on")],
        );
        put(
            &shared,
            "prds/PRD-003.md",
            "PRD-003",
            "prd",
            &[("PRD-002@hub", "based_on")],
        );
        put(
            &hub,
            "prds/PRD-004.md",
            "PRD-004",
            "prd",
            &[("PRD-003@shared", "based_on")],
        );
        put(
            &hub,
            "notes/NOTE-001.md",
            "NOTE-001",
            "note",
            &[("ADR-001@shared", "references")],
        );
        put(
            &hub,
            "notes/NOTE-002.md",
            "NOTE-002",
            "note",
            &[("NOTE-001", "based_on")],
        );
        for repo in [&hub, &shared] {
            git(repo, &["init", "-q"]);
            git(repo, &["config", "user.email", "test@example.invalid"]);
            git(repo, &["config", "user.name", "Eli Rum"]);
            git(repo, &["add", "-A"]);
            git(repo, &["commit", "-qm", "initial"]);
        }
        let specs = vec![
            SourceSpec::new(hub.join(".forgeplan")),
            SourceSpec::new(shared.join(".forgeplan")),
        ];
        Self {
            _temp: temp,
            hub,
            specs,
        }
    }
}

fn check_graph_and_impact(report: &ImpactReport, graph: &GraphOutcome) {
    assert_eq!(report.sources.len(), 2);
    assert!(report.failures.is_empty());
    let hub = &report
        .sources
        .iter()
        .find(|s| s.path.contains("/hub/"))
        .unwrap()
        .source_id;
    let shared = &report
        .sources
        .iter()
        .find(|s| s.path.contains("/shared/"))
        .unwrap()
        .source_id;
    assert_ne!(hub, shared);
    let find = |id: &str| {
        report
            .impacted
            .iter()
            .find(|node| node.display_id.as_deref() == Some(id))
            .unwrap()
    };
    let direct = find("PRD-002");
    assert_eq!(direct.source.source_id, *hub);
    assert_eq!(direct.severity, ImpactSeverity::Direct);
    assert_eq!(direct.path[0].source_id, *hub);
    assert_eq!(direct.path[0].relation, "based_on");
    let middle = find("PRD-003");
    assert_eq!(middle.source.source_id, *shared);
    assert_eq!(middle.severity, ImpactSeverity::Transitive);
    assert_eq!(middle.depth, 2);
    let last = find("PRD-004");
    assert_eq!(last.source.source_id, *hub);
    assert_eq!(last.severity, ImpactSeverity::Transitive);
    assert_eq!(last.depth, 3);
    assert_eq!(
        last.path
            .iter()
            .map(|p| p.source_id.as_str())
            .collect::<Vec<_>>(),
        vec![hub.as_str(), shared.as_str(), hub.as_str()]
    );
    assert!(last.path.iter().all(|step| step.relation == "based_on"));
    assert_eq!(find("NOTE-001").severity, ImpactSeverity::Noted);
    assert!(
        !report
            .impacted
            .iter()
            .any(|n| n.display_id.as_deref() == Some("NOTE-002"))
    );
    assert!(
        !report
            .impacted
            .iter()
            .any(|n| n.display_id.as_deref() == Some("ADR-001") && n.source.source_id == *hub)
    );
    assert!(report.stops.iter().any(|s| s.reason == "cycle"));
    let edge = graph
        .edges
        .iter()
        .find(|e| e.from == "PRD-002" && e.source_id == *hub)
        .unwrap();
    assert_eq!(edge.resolved.as_deref(), Some("ADR-001"));
    assert_eq!(edge.resolved_source_id.as_deref(), Some(shared.as_str()));
}

#[tokio::test]
async fn qualified_live_and_pinned_git_traverse_across_sources() {
    let fixture = Fixture::new();
    let live = impact_live(
        &fixture.specs,
        "ADR-001@shared",
        ImpactBounds {
            max_depth: 8,
            max_nodes: 256,
        },
        QueryMode::Strict,
    )
    .await
    .unwrap();
    let graph = graph_live(&fixture.specs, GraphBounds::default(), QueryMode::Strict)
        .await
        .unwrap();
    check_graph_and_impact(&live, &graph);

    let snapshot = capture_snapshot(&fixture.specs, None).unwrap();
    std::fs::write(
        fixture.hub.join(".forgeplan/prds/PRD-002.md"),
        doc("PRD-002", "prd", &[]),
    )
    .unwrap();
    let pinned = impact_snapshot(
        &snapshot,
        "ADR-001@shared",
        ImpactBounds {
            max_depth: 8,
            max_nodes: 256,
        },
    )
    .unwrap();
    let pinned_graph = graph_snapshot(&snapshot, GraphBounds::default()).unwrap();
    check_graph_and_impact(&pinned, &pinned_graph);
    let shared_id = &pinned
        .sources
        .iter()
        .find(|s| s.path.contains("/shared/"))
        .unwrap()
        .source_id;
    let by_id = impact_snapshot(
        &snapshot,
        &format!("ADR-001@{shared_id}"),
        ImpactBounds::default(),
    )
    .unwrap();
    assert!(
        by_id
            .impacted
            .iter()
            .any(|n| n.display_id.as_deref() == Some("PRD-004"))
    );
    let changed = impact_live(
        &fixture.specs,
        "ADR-001@shared",
        ImpactBounds::default(),
        QueryMode::Strict,
    )
    .await
    .unwrap();
    assert!(
        !changed
            .impacted
            .iter()
            .any(|n| n.display_id.as_deref() == Some("PRD-002"))
    );
}

#[tokio::test]
async fn bounds_are_global_across_cross_source_walk() {
    let fixture = Fixture::new();
    let shallow = impact_live(
        &fixture.specs,
        "ADR-001@shared",
        ImpactBounds {
            max_depth: 1,
            max_nodes: 100,
        },
        QueryMode::Strict,
    )
    .await
    .unwrap();
    assert!(
        shallow
            .stops
            .iter()
            .any(|stop| stop.reason == "depth_limit")
    );
    assert!(
        !shallow
            .impacted
            .iter()
            .any(|n| n.display_id.as_deref() == Some("PRD-003"))
    );
    let limited = impact_live(
        &fixture.specs,
        "ADR-001@shared",
        ImpactBounds {
            max_depth: 10,
            max_nodes: 1,
        },
        QueryMode::Strict,
    )
    .await
    .unwrap();
    assert_eq!(limited.impacted.len(), 1);
    assert!(limited.stops.iter().any(|stop| stop.reason == "node_limit"));
}

#[tokio::test]
async fn ambiguous_qualified_link_never_chooses_a_display_id_claimant() {
    let fixture = Fixture::new();
    let shared = fixture.specs[1].path.as_path();
    for (file, key) in [
        ("first", "adr-choice-first"),
        ("second", "adr-choice-second"),
    ] {
        std::fs::write(
            shared.join(format!("adrs/ADR-101-{file}.md")),
            format!("---\nid: ADR-101\nslug: {key}\nkind: adr\nstatus: accepted\ntitle: {key}\n---\n\nChoice.\n"),
        ).unwrap();
    }
    std::fs::write(
        fixture.hub.join(".forgeplan/prds/PRD-006.md"),
        doc("PRD-006", "prd", &[("ADR-101@shared", "based_on")]),
    )
    .unwrap();
    let report = impact_live(
        &fixture.specs,
        "adr-choice-first@shared",
        ImpactBounds::default(),
        QueryMode::Strict,
    )
    .await
    .unwrap();
    assert!(
        !report
            .impacted
            .iter()
            .any(|node| node.display_id.as_deref() == Some("PRD-006"))
    );
    assert!(
        report
            .unresolved
            .iter()
            .any(|edge| edge.from == "PRD-006" && edge.target == "ADR-101@shared")
    );
    let graph = graph_live(&fixture.specs, GraphBounds::default(), QueryMode::Strict)
        .await
        .unwrap();
    let edge = graph
        .edges
        .iter()
        .find(|edge| edge.from == "PRD-006")
        .unwrap();
    assert!(edge.resolved.is_none());
    assert!(edge.resolved_source_id.is_none());
}

fn add_alias(root: &Path, file: &str, id: &str, slug: Option<&str>) {
    let slug = slug.map(|s| format!("slug: {s}\n")).unwrap_or_default();
    std::fs::write(
        root.join("adrs").join(file),
        format!("---\nid: {id}\n{slug}kind: adr\nstatus: accepted\ntitle: {file}\n---\n\nBody.\n"),
    )
    .unwrap();
}

fn commit_fixture(fixture: &Fixture) {
    for spec in &fixture.specs {
        let repo = spec.path.parent().unwrap();
        git(repo, &["add", "-A"]);
        git(repo, &["commit", "--allow-empty", "-qm", "identity cases"]);
    }
}

fn check_ambiguity(
    error: forgeplan_core::constellation::ConstellationError,
    keys: &[&str],
    sources: usize,
    failures: usize,
) -> Vec<forgeplan_core::constellation::federation::SourceRef> {
    use forgeplan_core::artifact::canonical_key::ResolveError;
    use forgeplan_core::constellation::ConstellationError;
    match error {
        ConstellationError::ResolveRead {
            error: ResolveError::Ambiguous { candidates, .. },
            sources: basis,
            failures: failed,
        } => {
            assert_eq!(basis.len(), sources);
            assert_eq!(failed.len(), failures);
            assert!(basis.iter().all(|source| !source.version_key.is_empty()));
            assert_eq!(candidates.len(), keys.len());
            for key in keys {
                let candidate = candidates.iter().find(|c| &c.canonical_key == key).unwrap();
                assert!(!candidate.title.is_empty());
                assert!(
                    basis
                        .iter()
                        .any(|source| candidate.relative_path.starts_with(&source.source_id))
                );
                assert!(!candidate.relative_path.is_absolute());
            }
            basis
        }
        other => panic!("expected witnessed ambiguity: {other:?}"),
    }
}

#[tokio::test]
async fn ambiguous_seed_reports_all_candidates_with_live_and_pinned_witnesses() {
    let fixture = Fixture::new();
    let shared = &fixture.specs[1].path;
    add_alias(shared, "choice-a.md", "ADR-101", Some("adr-choice-a"));
    add_alias(shared, "choice-b.md", "ADR-101", Some("adr-choice-b"));
    commit_fixture(&fixture);
    let mut specs = fixture.specs.clone();
    specs.push(SourceSpec::new_optional(fixture.hub.join("missing")));
    let basis = graph_live(&fixture.specs, GraphBounds::default(), QueryMode::Strict)
        .await
        .unwrap()
        .sources;
    let live = impact_live(
        &specs,
        "ADR-101@shared",
        ImpactBounds::default(),
        QueryMode::Strict,
    )
    .await
    .unwrap_err();
    assert_eq!(
        check_ambiguity(live, &["adr-choice-a", "adr-choice-b"], 2, 1),
        basis
    );
    let snapshot = capture_snapshot(&fixture.specs, None).unwrap();
    let basis = graph_snapshot(&snapshot, GraphBounds::default())
        .unwrap()
        .sources;
    let pinned = impact_snapshot(&snapshot, "ADR-101@shared", ImpactBounds::default()).unwrap_err();
    assert_eq!(
        check_ambiguity(pinned, &["adr-choice-a", "adr-choice-b"], 2, 0),
        basis
    );
}

#[tokio::test]
async fn numeric_canonical_does_not_outrank_an_alias_live_or_pinned() {
    let fixture = Fixture::new();
    let shared = &fixture.specs[1].path;
    add_alias(shared, "legacy.md", "ADR-101", None);
    add_alias(shared, "other.md", "ADR-101", Some("adr-other"));
    commit_fixture(&fixture);
    let live = impact_live(
        &fixture.specs,
        "ADR-101@shared",
        ImpactBounds::default(),
        QueryMode::Strict,
    )
    .await
    .unwrap_err();
    check_ambiguity(live, &["ADR-101", "adr-other"], 2, 0);
    let snapshot = capture_snapshot(&fixture.specs, None).unwrap();
    check_ambiguity(
        impact_snapshot(&snapshot, "ADR-101@shared", ImpactBounds::default()).unwrap_err(),
        &["ADR-101", "adr-other"],
        2,
        0,
    );
}

#[tokio::test]
async fn numeric_alias_normalization_resolves_seeds_and_edges_at_both_bases() {
    let fixture = Fixture::new();
    std::fs::write(
        fixture.hub.join(".forgeplan/prds/PRD-009.md"),
        doc("PRD-009", "prd", &[("adr-1@shared", "based_on")]),
    )
    .unwrap();
    commit_fixture(&fixture);
    let report = impact_live(
        &fixture.specs,
        "adr-1@shared",
        ImpactBounds::default(),
        QueryMode::Strict,
    )
    .await
    .unwrap();
    assert!(
        report
            .impacted
            .iter()
            .any(|node| node.display_id.as_deref() == Some("PRD-009"))
    );
    let snapshot = capture_snapshot(&fixture.specs, None).unwrap();
    let report = impact_snapshot(&snapshot, "ADR-1@shared", ImpactBounds::default()).unwrap();
    assert!(
        report
            .impacted
            .iter()
            .any(|node| node.display_id.as_deref() == Some("PRD-009"))
    );
    let graph = graph_snapshot(&snapshot, GraphBounds::default()).unwrap();
    let edge = graph
        .edges
        .iter()
        .find(|edge| edge.from == "PRD-009")
        .unwrap();
    assert_eq!(edge.resolved.as_deref(), Some("ADR-001"));
}

#[tokio::test]
async fn broad_qualifier_never_discards_ambiguous_source_for_unique_source() {
    let fixture = Fixture::new();
    add_alias(&fixture.specs[0].path, "a.md", "ADR-101", Some("adr-a"));
    add_alias(&fixture.specs[0].path, "b.md", "ADR-101", Some("adr-b"));
    add_alias(&fixture.specs[1].path, "c.md", "ADR-101", Some("adr-c"));
    std::fs::write(
        fixture.hub.join(".forgeplan/prds/PRD-009.md"),
        doc("PRD-009", "prd", &[("ADR-101@.forgeplan", "based_on")]),
    )
    .unwrap();
    commit_fixture(&fixture);
    let check_get = |error, sources: &[forgeplan_core::constellation::federation::SourceRef]| {
        use forgeplan_core::artifact::canonical_key::{ArtifactCandidate, ResolveError};

        let expected: Vec<_> = [
            ("adr-a", "a.md", "hub"),
            ("adr-b", "b.md", "hub"),
            ("adr-c", "c.md", "shared"),
        ]
        .into_iter()
        .map(|(key, file, project)| {
            let source = sources
                .iter()
                .find(|source| source.path.ends_with(&format!("/{project}/.forgeplan")))
                .unwrap();
            ArtifactCandidate {
                canonical_key: key.into(),
                display_id: "ADR-101".into(),
                title: file.into(),
                relative_path: PathBuf::from(&source.source_id).join("adrs").join(file),
            }
        })
        .collect();
        match error {
            ResolveError::Ambiguous { input, candidates } => {
                assert_eq!(input, "ADR-101");
                assert_eq!(candidates, expected);
            }
            other => panic!("expected complete get ambiguity: {other:?}"),
        }
    };
    let reversed: Vec<_> = fixture.specs.iter().cloned().rev().collect();
    for specs in [&fixture.specs, &reversed] {
        let live = federated_get(specs, "ADR-101", QueryMode::Strict)
            .await
            .unwrap();
        match live.result {
            FederatedGetResult::ResolveError(error) => check_get(error, &live.read.sources),
            other => panic!("expected ambiguous live get: {other:?}"),
        }
        let pinned = capture_snapshot(specs, None).unwrap();
        let basis = graph_snapshot(&pinned, GraphBounds::default())
            .unwrap()
            .sources;
        match snapshot_get(&pinned, "ADR-101", Some(".forgeplan")).unwrap_err() {
            forgeplan_core::constellation::ConstellationError::Resolve(error) => {
                check_get(error, &basis);
            }
            other => panic!("expected ambiguous pinned get: {other:?}"),
        }
    }
    let error = impact_live(
        &fixture.specs,
        "ADR-101@.forgeplan",
        ImpactBounds::default(),
        QueryMode::Strict,
    )
    .await
    .unwrap_err();
    check_ambiguity(error, &["adr-a", "adr-b", "adr-c"], 2, 0);
    let graph = graph_live(&fixture.specs, GraphBounds::default(), QueryMode::Strict)
        .await
        .unwrap();
    let edge = graph
        .edges
        .iter()
        .find(|edge| edge.from == "PRD-009")
        .unwrap();
    assert!(edge.resolved.is_none());
    let snapshot = capture_snapshot(&fixture.specs, None).unwrap();
    let error =
        impact_snapshot(&snapshot, "ADR-101@.forgeplan", ImpactBounds::default()).unwrap_err();
    check_ambiguity(error, &["adr-a", "adr-b", "adr-c"], 2, 0);
    let graph = graph_snapshot(&snapshot, GraphBounds::default()).unwrap();
    assert!(
        graph
            .edges
            .iter()
            .find(|edge| edge.from == "PRD-009")
            .unwrap()
            .resolved
            .is_none()
    );
}

#[tokio::test]
async fn literal_aliases_are_exact_and_canonical_keys_take_priority() {
    let fixture = Fixture::new();
    let shared = &fixture.specs[1].path;
    add_alias(shared, "literal.md", "ADR-ABC", Some("adr-literal"));
    add_alias(
        shared,
        "competitor.md",
        "adr-literal",
        Some("adr-competitor"),
    );
    std::fs::write(
        fixture.hub.join(".forgeplan/prds/PRD-010.md"),
        doc("PRD-010", "prd", &[("ADR-ABC@shared", "based_on")]),
    )
    .unwrap();
    commit_fixture(&fixture);
    let report = impact_live(
        &fixture.specs,
        "adr-literal@shared",
        ImpactBounds::default(),
        QueryMode::Strict,
    )
    .await
    .unwrap();
    assert!(
        report
            .impacted
            .iter()
            .any(|node| node.display_id.as_deref() == Some("PRD-010"))
    );
    let different_case = impact_live(
        &fixture.specs,
        "adr-abc@shared",
        ImpactBounds::default(),
        QueryMode::Strict,
    )
    .await
    .unwrap();
    assert!(
        !different_case
            .impacted
            .iter()
            .any(|node| node.display_id.as_deref() == Some("PRD-010"))
    );
    let snapshot = capture_snapshot(&fixture.specs, None).unwrap();
    let report = impact_snapshot(&snapshot, "adr-literal@shared", ImpactBounds::default()).unwrap();
    assert!(
        report
            .impacted
            .iter()
            .any(|node| node.display_id.as_deref() == Some("PRD-010"))
    );
}
