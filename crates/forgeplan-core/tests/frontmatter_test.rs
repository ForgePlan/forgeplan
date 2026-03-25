use forgeplan_core::artifact::frontmatter;

#[test]
fn parse_valid_frontmatter() {
    let content = "---\nid: PRD-001\ntitle: Test\nstatus: Draft\n---\n\n# Body here\n";
    let (fm, body) = frontmatter::parse_frontmatter(content).unwrap();
    assert_eq!(
        fm.get("id").unwrap(),
        &serde_yml::Value::String("PRD-001".into())
    );
    assert!(body.contains("# Body here"));
}

#[test]
fn parse_no_frontmatter_fails() {
    let content = "# Just a heading\n";
    assert!(frontmatter::parse_frontmatter(content).is_err());
}

#[test]
fn render_roundtrip() {
    let content = "---\nid: RFC-002\ntitle: My RFC\n---\n\n# Content\n\nSome text.\n";
    let (fm, body) = frontmatter::parse_frontmatter(content).unwrap();
    let rendered = frontmatter::render_frontmatter(&fm, &body).unwrap();
    // Re-parse and verify
    let (fm2, body2) = frontmatter::parse_frontmatter(&rendered).unwrap();
    assert_eq!(fm, fm2);
    assert_eq!(body.trim(), body2.trim());
}
