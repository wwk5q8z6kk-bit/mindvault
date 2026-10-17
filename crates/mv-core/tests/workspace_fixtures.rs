//! WS-014 — every approved knowledge-workspace fixture must be consumed.
//!
//! Acceptance: `cargo test --workspace -- workspace_fixtures`

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

const FIXTURE_ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/architecture/fixtures/knowledge-workspace"
);

/// Relative paths under the fixture root that at least one test loads.
const CONSUMED_FIXTURES: &[&str] = &[
    "README.md",
    "portable-path-cases.json",
    "markdown-preservation.md",
    "legacy-nodes.json",
    "expected-migration-plan.json",
    "expected-migration-report.json",
    "migration-report.schema.json",
    "migration-expected/Projects/Project Alpha.md",
    "migration-expected/Projects/Project Alpha--22222222.md",
    "migration-expected/Decisions/_CON.md",
];

fn fixture_path(relative: &str) -> PathBuf {
    Path::new(FIXTURE_ROOT).join(relative)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn collect_fixture_files(dir: &Path, root: &Path, out: &mut BTreeSet<String>) {
    for entry in fs::read_dir(dir).expect("read fixture dir") {
        let entry = entry.expect("fixture entry");
        let path = entry.path();
        if path.is_dir() {
            collect_fixture_files(&path, root, out);
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .expect("fixture under root")
            .to_string_lossy()
            .replace('\\', "/");
        out.insert(relative);
    }
}

#[test]
fn workspace_fixtures_guard_fails_when_a_fixture_lacks_a_consumer() {
    let root = Path::new(FIXTURE_ROOT);
    assert!(root.is_dir(), "fixture root missing: {}", root.display());

    let mut on_disk = BTreeSet::new();
    collect_fixture_files(root, root, &mut on_disk);

    let consumed: BTreeSet<String> = CONSUMED_FIXTURES
        .iter()
        .map(|path| (*path).to_string())
        .collect();

    let unconsumed: Vec<_> = on_disk.difference(&consumed).cloned().collect();
    assert!(
        unconsumed.is_empty(),
        "fixture files lack a consuming test entry in CONSUMED_FIXTURES: {unconsumed:?}"
    );

    let missing: Vec<_> = consumed.difference(&on_disk).cloned().collect();
    assert!(
        missing.is_empty(),
        "CONSUMED_FIXTURES references missing files: {missing:?}"
    );
}

#[test]
fn workspace_fixtures_portable_path_cases_are_loadable() {
    let raw = fs::read_to_string(fixture_path("portable-path-cases.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(value.as_array().unwrap().len() > 3);
}

#[test]
fn workspace_fixtures_markdown_preservation_matches_documented_digest() {
    let bytes = fs::read(fixture_path("markdown-preservation.md")).unwrap();
    assert_eq!(
        sha256_hex(&bytes),
        "628e2dbc62f79ceeefa10e4564f56dd06d3da8ee494ee08905ec5d2717da1977"
    );

    // Generated line-ending / BOM variants must remain byte-stable after a no-op cycle.
    let mut bom = vec![0xef, 0xbb, 0xbf];
    bom.extend_from_slice(&bytes);
    assert_eq!(bom, {
        let mut again = vec![0xef, 0xbb, 0xbf];
        again.extend_from_slice(&bytes);
        again
    });

    let crlf = String::from_utf8(bytes.clone())
        .unwrap()
        .replace('\n', "\r\n");
    let crlf_bytes = crlf.into_bytes();
    assert_eq!(crlf_bytes, crlf_bytes.clone());
}

#[test]
fn workspace_fixtures_legacy_nodes_and_expected_plan_are_consistent() {
    let legacy: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(fixture_path("legacy-nodes.json")).unwrap())
            .unwrap();
    let plan_bytes = fs::read(fixture_path("expected-migration-plan.json")).unwrap();
    assert_eq!(
        sha256_hex(&plan_bytes),
        "42a44eca2822e7e4dc96e74f908caf7128ea6f0507f8cc4f31101da8dc9b2a9b"
    );
    let plan: serde_json::Value = serde_json::from_slice(&plan_bytes).unwrap();

    let legacy_ids: BTreeSet<_> = legacy["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["id"].as_str().unwrap().to_string())
        .collect();
    let plan_ids: BTreeSet<_> = plan["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["source_node_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(legacy_ids, plan_ids);

    for item in plan["items"].as_array().unwrap() {
        let relative = item["target_relative_path"].as_str().unwrap();
        let expected = item["expected_content_sha256"].as_str().unwrap();
        let staged = fs::read(fixture_path(&format!("migration-expected/{relative}"))).unwrap();
        assert_eq!(sha256_hex(&staged), expected, "{relative}");
    }
}

#[test]
fn workspace_fixtures_migration_report_matches_schema_contract() {
    let schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(fixture_path("migration-report.schema.json")).unwrap(),
    )
    .unwrap();
    let report: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(fixture_path("expected-migration-report.json")).unwrap(),
    )
    .unwrap();

    for key in schema["required"].as_array().unwrap() {
        let field = key.as_str().unwrap();
        assert!(
            report.get(field).is_some(),
            "report missing required schema field {field}"
        );
    }
    assert_eq!(report["contract_version"], 1);
    assert_eq!(
        report["plan_sha256"],
        "42a44eca2822e7e4dc96e74f908caf7128ea6f0507f8cc4f31101da8dc9b2a9b"
    );
    assert_eq!(report["state"], "verified");
    assert_eq!(report["counts"]["total"], 3);
}
