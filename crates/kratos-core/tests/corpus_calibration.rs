use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use kratos_core::analyze::analyze_project;
use kratos_core::model::{
    EntrypointKind, ImportKind, ImportSpecifierKind, OrphanKind, ReportV2, REPORT_CURRENT,
};
use kratos_core::report_diff::{
    diff_reports_with_identity, ReportFindingIds, FINDING_IDENTITY_VERSION,
};
use serde_json::Value;

const FINDING_KINDS: [&str; 6] = [
    "brokenImports",
    "orphanFiles",
    "deadExports",
    "unusedImports",
    "routeEntrypoints",
    "deletionCandidates",
];
const REQUIRED_CASE_CLASSES: [&str; 4] = [
    "mixed-positive",
    "dynamic-usage-negative",
    "route-substring-negative",
    "empty-noop",
];

#[derive(Clone, Debug)]
struct CorpusManifest {
    identity_version: u32,
    cases: Vec<CorpusCase>,
}

#[derive(Clone, Debug)]
struct CorpusCase {
    id: String,
    root: PathBuf,
    class: String,
    expected: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KindEvaluation {
    true_positives: Vec<String>,
    false_positives: Vec<String>,
    false_negatives: Vec<String>,
    ordered_match: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CaseEvaluation {
    case_id: String,
    observed: BTreeMap<String, Vec<String>>,
    kinds: BTreeMap<String, KindEvaluation>,
}

#[test]
fn corpus_manifest_is_valid() {
    let manifest = load_manifest().expect("corpus manifest should parse and validate");

    assert_eq!(manifest.identity_version, FINDING_IDENTITY_VERSION);
    assert_eq!(manifest.identity_version, 1);
    assert!(manifest.cases.len() >= 4);

    let mut unsafe_manifest = manifest.clone();
    unsafe_manifest.cases[0].id = "../escape".to_string();
    let error = validate_manifest(&unsafe_manifest)
        .expect_err("case IDs that could escape a temporary root must be rejected");
    assert!(error.contains("safe lowercase slug"));
}

#[test]
fn corpus_manifest_covers_all_finding_kinds_and_required_case_classes() {
    let manifest = load_manifest().expect("corpus manifest should parse and validate");
    let classes = manifest
        .cases
        .iter()
        .map(|case| case.class.as_str())
        .collect::<BTreeSet<_>>();

    for required in REQUIRED_CASE_CLASSES {
        assert!(
            classes.contains(required),
            "missing corpus class: {required}"
        );
    }

    let mixed = manifest
        .cases
        .iter()
        .find(|case| case.class == "mixed-positive")
        .expect("mixed-positive case should exist");
    for kind in FINDING_KINDS {
        assert!(
            !mixed.expected[kind].is_empty(),
            "mixed-positive must cover {kind}"
        );
    }
}

#[test]
fn corpus_expected_and_observed_ids_match() {
    let manifest = load_manifest().expect("corpus manifest should parse and validate");
    let evaluations = evaluate_manifest(&manifest).expect("corpus should evaluate");
    let rendered = render_evaluations(manifest.identity_version, &evaluations);
    println!("{rendered}");
    ensure_no_mismatches(&evaluations).expect("canonical corpus should have FP=0 and FN=0");
}

#[test]
fn corpus_evaluator_reports_false_positives_and_false_negatives() {
    let manifest = load_manifest().expect("corpus manifest should parse and validate");
    let mixed = manifest
        .cases
        .iter()
        .find(|case| case.class == "mixed-positive")
        .expect("mixed-positive case should exist");
    let observed = observe_case(&corpus_root().join(&mixed.root)).expect("case should analyze");
    let (kind, real_id) = FINDING_KINDS
        .iter()
        .find_map(|kind| {
            observed[*kind]
                .iter()
                .next()
                .map(|id| ((*kind).to_string(), id.clone()))
        })
        .expect("mixed-positive should contain a finding");

    let fake_id = format!("kratos:v1:{}", "0".repeat(64));
    assert!(!observed.values().any(|ids| ids.contains(&fake_id)));
    assert!(!mixed.expected.values().any(|ids| ids.contains(&fake_id)));
    let mut wrong_expected = mixed.expected.clone();
    let expected_ids = wrong_expected.get_mut(&kind).expect("kind should exist");
    let replacement = expected_ids
        .iter_mut()
        .find(|id| **id == real_id)
        .expect("observed ID should be canonical expected truth");
    *replacement = fake_id.clone();
    expected_ids.sort();

    let result = evaluate_sets(&mixed.id, &wrong_expected, &observed);
    let kind_result = &result.kinds[&kind];
    assert_eq!(kind_result.true_positives.len(), observed[&kind].len() - 1);
    assert_eq!(kind_result.false_positives, vec![real_id]);
    assert_eq!(kind_result.false_negatives, vec![fake_id]);
    assert!(!kind_result.ordered_match);
    let error = ensure_no_mismatches(std::slice::from_ref(&result))
        .expect_err("a well-formed wrong replacement ID must fail calibration");
    assert!(error.contains(&format!("{}/{}", mixed.id, kind)));
    assert!(error.contains("fp=[\"kratos:v1:"));
    assert!(error.contains("fn=[\"kratos:v1:"));
    println!("injected-replacement-error={error}");
}

#[test]
fn corpus_evaluator_preserves_duplicate_multiplicity_and_order() {
    let id_a = format!("kratos:v1:{}", "a".repeat(64));
    let id_b = format!("kratos:v1:{}", "b".repeat(64));
    let mut expected = FINDING_KINDS
        .into_iter()
        .map(|kind| (kind.to_string(), Vec::new()))
        .collect::<BTreeMap<_, _>>();
    expected.insert(
        "brokenImports".to_string(),
        vec![id_a.clone(), id_a.clone(), id_b.clone()],
    );

    let matching = evaluate_sets("duplicate-probe", &expected, &expected);
    assert_eq!(matching.kinds["brokenImports"].true_positives.len(), 3);
    assert!(matching.kinds["brokenImports"].ordered_match);

    let mut reordered = expected.clone();
    reordered.insert("brokenImports".to_string(), vec![id_a.clone(), id_b, id_a]);
    let result = evaluate_sets("duplicate-probe", &expected, &reordered);
    assert_eq!(result.kinds["brokenImports"].true_positives.len(), 3);
    assert!(result.kinds["brokenImports"].false_positives.is_empty());
    assert!(result.kinds["brokenImports"].false_negatives.is_empty());
    assert!(!result.kinds["brokenImports"].ordered_match);
    ensure_no_mismatches(std::slice::from_ref(&result))
        .expect_err("reordered duplicate IDs must fail ordered calibration");
}

#[test]
fn corpus_results_are_root_independent_and_deterministic() {
    let manifest = load_manifest().expect("corpus manifest should parse and validate");
    let first = evaluate_manifest(&manifest).expect("first evaluation should succeed");
    let second = evaluate_manifest(&manifest).expect("second evaluation should succeed");
    assert_eq!(first, second);
    assert_eq!(
        render_evaluations(manifest.identity_version, &first),
        render_evaluations(manifest.identity_version, &second)
    );

    let temp = TestDirectory::new("root-independence");
    let mut copied = Vec::new();
    for case in &manifest.cases {
        let source = corpus_root().join(&case.root);
        let target = temp.path.join(&case.id);
        copy_tree(&source, &target).expect("fixture should copy");
        let observed = observe_case(&target).expect("copied case should analyze");
        copied.push(evaluate_sets(&case.id, &case.expected, &observed));
    }
    copied.sort_by(|left, right| left.case_id.cmp(&right.case_id));

    assert_eq!(first, copied);
    assert_eq!(
        render_evaluations(manifest.identity_version, &first),
        render_evaluations(manifest.identity_version, &copied)
    );
}

#[test]
fn corpus_evaluation_is_no_write_and_schema_v3_compatible() {
    let manifest = load_manifest().expect("corpus manifest should parse and validate");
    let before = snapshot_tree(&corpus_root()).expect("corpus should be readable");

    for case in &manifest.cases {
        let root = corpus_root().join(&case.root);
        let report = analyze_project(&root).expect("case should analyze");
        assert_eq!(report.version, REPORT_CURRENT);
        assert_eq!(report.version, 3);
        let baseline = ReportV2::new(root);
        let diff = diff_reports_with_identity(&baseline, &report);
        assert_eq!(diff.identity_version, FINDING_IDENTITY_VERSION);
        assert_eq!(diff.identity_version, 1);
        assert_eq!(diff.summary.totals.resolved, 0);
        assert_eq!(diff.summary.totals.persisted, 0);
    }

    let after = snapshot_tree(&corpus_root()).expect("corpus should remain readable");
    assert_eq!(before, after, "corpus evaluation modified fixture bytes");
    assert!(
        !contains_directory_named(&corpus_root(), ".kratos").expect("corpus should be walkable"),
        "corpus evaluation created a .kratos directory"
    );
}

#[test]
fn known_negative_cases_preserve_their_specific_contracts() {
    let manifest = load_manifest().expect("corpus manifest should parse and validate");
    let dynamic_case = manifest
        .cases
        .iter()
        .find(|case| case.class == "dynamic-usage-negative")
        .expect("dynamic-usage-negative case should exist");
    let dynamic_root = corpus_root().join(&dynamic_case.root);
    let dynamic_report = analyze_project(&dynamic_root).expect("dynamic case should analyze");
    assert_eq!(dynamic_report.summary.files_scanned, 2);
    assert_eq!(dynamic_report.modules.len(), 2);
    assert!(dynamic_report.findings.dead_exports.is_empty());
    let chunk = dynamic_report
        .modules
        .iter()
        .find(|module| module.relative_path == "src/Chunk.tsx")
        .expect("dynamic target should be discovered");
    assert_eq!(chunk.imported_by_count, 1);
    assert_eq!(chunk.importers.len(), 1);
    assert_eq!(chunk.importers[0].kind, ImportKind::Dynamic);
    assert_eq!(chunk.importers[0].specifiers.len(), 1);
    assert_eq!(
        chunk.importers[0].specifiers[0].kind,
        ImportSpecifierKind::Default
    );
    assert_eq!(
        chunk.importers[0].specifiers[0].imported.as_deref(),
        Some("default")
    );
    let dynamic_observed = observe_case(&dynamic_root).expect("dynamic case should evaluate");
    assert!(dynamic_observed.values().all(Vec::is_empty));

    let empty_case = manifest
        .cases
        .iter()
        .find(|case| case.class == "empty-noop")
        .expect("empty-noop case should exist");
    let empty_root = corpus_root().join(&empty_case.root);
    let empty_report = analyze_project(&empty_root).expect("empty case should analyze");
    assert_eq!(empty_report.summary.files_scanned, 0);
    assert!(empty_report.modules.is_empty());
    assert_eq!(empty_report.summary.broken_imports, 0);
    assert_eq!(empty_report.summary.orphan_files, 0);
    assert_eq!(empty_report.summary.dead_exports, 0);
    assert_eq!(empty_report.summary.unused_imports, 0);
    assert_eq!(empty_report.summary.route_entrypoints, 0);
    assert_eq!(empty_report.summary.deletion_candidates, 0);
    let empty_observed = observe_case(&empty_root).expect("empty case should evaluate");
    assert!(empty_observed.values().all(Vec::is_empty));

    let route_case = manifest
        .cases
        .iter()
        .find(|case| case.class == "route-substring-negative")
        .expect("route-substring-negative case should exist");
    let route_root = corpus_root().join(&route_case.root);
    let route_report = analyze_project(&route_root).expect("route case should analyze");
    assert_eq!(route_report.summary.files_scanned, 2);
    assert_eq!(route_report.modules.len(), 2);
    let router_orphan = route_report
        .findings
        .orphan_files
        .iter()
        .find(|finding| finding.file.ends_with("src/lib/router.ts"))
        .expect("router substring fixture should be classified as an orphan");
    assert_eq!(router_orphan.kind, OrphanKind::Module);
    let observed = observe_case(&route_root).expect("route case should evaluate");
    assert!(observed["routeEntrypoints"].is_empty());
    assert!(observed["brokenImports"].is_empty());
    assert!(observed["unusedImports"].is_empty());
    for expected_orphan_signal in ["orphanFiles", "deadExports", "deletionCandidates"] {
        assert!(
            !observed[expected_orphan_signal].is_empty(),
            "route substring fixture must exercise orphan classification through {expected_orphan_signal}"
        );
    }
}

#[test]
fn mixed_positive_has_independent_semantic_witnesses() {
    let manifest = load_manifest().expect("corpus manifest should parse and validate");
    let mixed = manifest
        .cases
        .iter()
        .find(|case| case.class == "mixed-positive")
        .expect("mixed-positive case should exist");
    let report = analyze_project(&corpus_root().join(&mixed.root)).expect("case should analyze");

    assert_eq!(report.summary.files_scanned, 4);
    assert_eq!(report.modules.len(), 4);
    assert!(report.findings.broken_imports.iter().any(|finding| {
        finding.file.ends_with("src/broken.ts") && finding.source == "./missing"
    }));
    assert!(report.findings.orphan_files.iter().any(|finding| {
        finding.file.ends_with("src/orphan.ts") && finding.kind == OrphanKind::Module
    }));
    for export_name in ["broken", "orphaned"] {
        assert!(
            report
                .findings
                .dead_exports
                .iter()
                .any(|finding| finding.export_name == export_name),
            "missing dead export witness: {export_name}"
        );
    }
    assert!(report.findings.unused_imports.iter().any(|finding| {
        finding.file.ends_with("pages/index.tsx")
            && finding.source == "../src/lib/math"
            && finding.local == "unused"
            && finding.imported == "unused"
    }));
    assert!(report.findings.route_entrypoints.iter().any(|finding| {
        finding.file.ends_with("pages/index.tsx") && finding.kind == EntrypointKind::NextPagesRoute
    }));
    assert!(report
        .findings
        .deletion_candidates
        .iter()
        .any(|finding| finding.file.ends_with("src/orphan.ts")));
}

fn load_manifest() -> Result<CorpusManifest, String> {
    let path = corpus_root().join("manifest.json");
    let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "manifest root must be an object".to_string())?;
    reject_unknown_keys(object.keys(), &["identityVersion", "cases"], "manifest")?;

    let identity_version = required_u32(object.get("identityVersion"), "identityVersion")?;
    let raw_cases = object
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "cases must be an array".to_string())?;
    let mut cases = Vec::new();
    for (index, raw_case) in raw_cases.iter().enumerate() {
        let case_object = raw_case
            .as_object()
            .ok_or_else(|| format!("cases[{index}] must be an object"))?;
        reject_unknown_keys(
            case_object.keys(),
            &["id", "root", "class", "expected"],
            &format!("cases[{index}]"),
        )?;
        let id = required_string(case_object.get("id"), &format!("cases[{index}].id"))?;
        let root_text = required_string(case_object.get("root"), &format!("cases[{index}].root"))?;
        let class = required_string(case_object.get("class"), &format!("cases[{index}].class"))?;
        let expected_object = case_object
            .get("expected")
            .and_then(Value::as_object)
            .ok_or_else(|| format!("cases[{index}].expected must be an object"))?;
        reject_unknown_keys(
            expected_object.keys(),
            &FINDING_KINDS,
            &format!("cases[{index}].expected"),
        )?;

        let mut expected = BTreeMap::new();
        for kind in FINDING_KINDS {
            let ids = expected_object
                .get(kind)
                .and_then(Value::as_array)
                .ok_or_else(|| format!("cases[{index}].expected.{kind} must be an array"))?
                .iter()
                .enumerate()
                .map(|(id_index, value)| {
                    value.as_str().map(str::to_string).ok_or_else(|| {
                        format!("cases[{index}].expected.{kind}[{id_index}] must be a string")
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            expected.insert(kind.to_string(), ids);
        }
        cases.push(CorpusCase {
            id,
            root: PathBuf::from(root_text),
            class,
            expected,
        });
    }

    let manifest = CorpusManifest {
        identity_version,
        cases,
    };
    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_manifest(manifest: &CorpusManifest) -> Result<(), String> {
    if manifest.identity_version != FINDING_IDENTITY_VERSION || manifest.identity_version != 1 {
        return Err(format!(
            "manifest identityVersion {} does not match runtime {}",
            manifest.identity_version, FINDING_IDENTITY_VERSION
        ));
    }
    if manifest.cases.len() < 4 {
        return Err("manifest must contain at least four cases".to_string());
    }

    let ids = manifest
        .cases
        .iter()
        .map(|case| case.id.clone())
        .collect::<Vec<_>>();
    let mut sorted_ids = ids.clone();
    sorted_ids.sort();
    if ids != sorted_ids {
        return Err("cases must be sorted by id".to_string());
    }
    if ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
        return Err("case ids must be unique".to_string());
    }

    let canonical_corpus = fs::canonicalize(corpus_root())
        .map_err(|error| format!("failed to canonicalize corpus root: {error}"))?;
    let mut roots = BTreeSet::new();
    for case in &manifest.cases {
        if !valid_case_id(&case.id) {
            return Err(format!(
                "case id must be a safe lowercase slug: {}",
                case.id
            ));
        }
        if case.root.is_absolute()
            || case
                .root
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(format!("case {} root must be corpus-relative", case.id));
        }
        if !roots.insert(case.root.clone()) {
            return Err(format!("duplicate case root: {}", case.root.display()));
        }
        let root = corpus_root().join(&case.root);
        if !root.is_dir() {
            return Err(format!(
                "case {} root does not exist: {}",
                case.id,
                root.display()
            ));
        }
        let canonical_root = fs::canonicalize(&root)
            .map_err(|error| format!("failed to canonicalize {}: {error}", root.display()))?;
        if !canonical_root.starts_with(&canonical_corpus) {
            return Err(format!("case {} root escapes the corpus", case.id));
        }

        for kind in FINDING_KINDS {
            let values = case
                .expected
                .get(kind)
                .ok_or_else(|| format!("case {} missing kind {kind}", case.id))?;
            let mut sorted = values.clone();
            sorted.sort();
            if values != &sorted {
                return Err(format!("case {} {kind} ids must be sorted", case.id));
            }
            for id in values {
                if !valid_finding_id(id, manifest.identity_version) {
                    return Err(format!("case {} has invalid {kind} id: {id}", case.id));
                }
            }
        }
    }
    Ok(())
}

fn evaluate_manifest(manifest: &CorpusManifest) -> Result<Vec<CaseEvaluation>, String> {
    let mut evaluations = manifest
        .cases
        .iter()
        .map(|case| {
            let observed = observe_case(&corpus_root().join(&case.root))?;
            Ok(evaluate_sets(&case.id, &case.expected, &observed))
        })
        .collect::<Result<Vec<_>, String>>()?;
    evaluations.sort_by(|left, right| left.case_id.cmp(&right.case_id));
    Ok(evaluations)
}

fn observe_case(root: &Path) -> Result<BTreeMap<String, Vec<String>>, String> {
    let report = analyze_project(root).map_err(|error| error.to_string())?;
    if report.version != REPORT_CURRENT {
        return Err(format!("unexpected report schema: {}", report.version));
    }
    let baseline = ReportV2::new(root.to_path_buf());
    if baseline.version != REPORT_CURRENT {
        return Err(format!(
            "unexpected baseline report schema: {}",
            baseline.version
        ));
    }
    let diff = diff_reports_with_identity(&baseline, &report);
    if diff.identity_version != FINDING_IDENTITY_VERSION {
        return Err(format!(
            "unexpected finding identity version: {}",
            diff.identity_version
        ));
    }
    if diff.summary.totals.resolved != 0 || diff.summary.totals.persisted != 0 {
        return Err("empty baseline produced resolved or persisted findings".to_string());
    }
    if has_resolved_or_persisted_ids(&diff.finding_ids) {
        return Err("empty baseline produced resolved or persisted finding IDs".to_string());
    }
    Ok(observed_ids(&diff.finding_ids))
}

fn has_resolved_or_persisted_ids(ids: &ReportFindingIds) -> bool {
    [
        &ids.broken_imports,
        &ids.orphan_files,
        &ids.dead_exports,
        &ids.unused_imports,
        &ids.route_entrypoints,
        &ids.deletion_candidates,
    ]
    .iter()
    .any(|ids| !ids.resolved.is_empty() || !ids.persisted.is_empty())
}

fn observed_ids(ids: &ReportFindingIds) -> BTreeMap<String, Vec<String>> {
    BTreeMap::from([
        (
            "brokenImports".to_string(),
            ids.broken_imports.introduced.clone(),
        ),
        (
            "orphanFiles".to_string(),
            ids.orphan_files.introduced.clone(),
        ),
        (
            "deadExports".to_string(),
            ids.dead_exports.introduced.clone(),
        ),
        (
            "unusedImports".to_string(),
            ids.unused_imports.introduced.clone(),
        ),
        (
            "routeEntrypoints".to_string(),
            ids.route_entrypoints.introduced.clone(),
        ),
        (
            "deletionCandidates".to_string(),
            ids.deletion_candidates.introduced.clone(),
        ),
    ])
}

fn evaluate_sets(
    case_id: &str,
    expected: &BTreeMap<String, Vec<String>>,
    observed: &BTreeMap<String, Vec<String>>,
) -> CaseEvaluation {
    let mut kinds = BTreeMap::new();
    for kind in FINDING_KINDS {
        kinds.insert(
            kind.to_string(),
            evaluate_multisets(&expected[kind], &observed[kind]),
        );
    }
    CaseEvaluation {
        case_id: case_id.to_string(),
        observed: observed.clone(),
        kinds,
    }
}

fn evaluate_multisets(expected: &[String], observed: &[String]) -> KindEvaluation {
    let expected_counts = multiplicities(expected);
    let observed_counts = multiplicities(observed);
    let all_ids = expected_counts
        .keys()
        .chain(observed_counts.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut true_positives = Vec::new();
    let mut false_positives = Vec::new();
    let mut false_negatives = Vec::new();
    for id in all_ids {
        let expected_count = expected_counts.get(&id).copied().unwrap_or(0);
        let observed_count = observed_counts.get(&id).copied().unwrap_or(0);
        true_positives
            .extend(std::iter::repeat(id.clone()).take(expected_count.min(observed_count)));
        false_positives.extend(
            std::iter::repeat(id.clone()).take(observed_count.saturating_sub(expected_count)),
        );
        false_negatives
            .extend(std::iter::repeat(id).take(expected_count.saturating_sub(observed_count)));
    }
    KindEvaluation {
        true_positives,
        false_positives,
        false_negatives,
        ordered_match: expected == observed,
    }
}

fn multiplicities(ids: &[String]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for id in ids {
        *counts.entry(id.clone()).or_default() += 1;
    }
    counts
}

fn ensure_no_mismatches(evaluations: &[CaseEvaluation]) -> Result<(), String> {
    let mismatches = evaluations
        .iter()
        .flat_map(|case| {
            case.kinds
                .iter()
                .filter(|(_, result)| {
                    !result.false_positives.is_empty()
                        || !result.false_negatives.is_empty()
                        || !result.ordered_match
                })
                .map(move |(kind, result)| {
                    format!(
                        "{}/{}: fp={:?}, fn={:?}, orderedMatch={}",
                        case.case_id,
                        kind,
                        result.false_positives,
                        result.false_negatives,
                        result.ordered_match
                    )
                })
        })
        .collect::<Vec<_>>();
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "canonical corpus mismatch:\n{}",
            mismatches.join("\n")
        ))
    }
}

fn render_evaluations(identity_version: u32, evaluations: &[CaseEvaluation]) -> String {
    let mut lines = vec![format!("identityVersion={identity_version}")];
    let mut total_tp = 0;
    let mut total_fp = 0;
    let mut total_fn = 0;
    for case in evaluations {
        lines.push(format!("case={}", case.case_id));
        for kind in FINDING_KINDS {
            let result = &case.kinds[kind];
            total_tp += result.true_positives.len();
            total_fp += result.false_positives.len();
            total_fn += result.false_negatives.len();
            lines.push(format!(
                "  kind={kind} tp={} fp={} fn={} orderedMatch={}",
                result.true_positives.len(),
                result.false_positives.len(),
                result.false_negatives.len(),
                result.ordered_match
            ));
            for id in &case.observed[kind] {
                lines.push(format!("    observed={id}"));
            }
            for (label, ids) in [
                ("tp", &result.true_positives),
                ("fp", &result.false_positives),
                ("fn", &result.false_negatives),
            ] {
                for id in ids {
                    lines.push(format!("    {label}={id}"));
                }
            }
        }
    }
    lines.push(format!("totals tp={total_tp} fp={total_fp} fn={total_fn}"));
    lines.join("\n")
}

fn valid_finding_id(id: &str, version: u32) -> bool {
    let prefix = format!("kratos:v{version}:");
    id.strip_prefix(&prefix).is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn valid_case_id(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('-')
        && !id.ends_with('-')
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn reject_unknown_keys<'a>(
    keys: impl Iterator<Item = &'a String>,
    allowed: &[&str],
    context: &str,
) -> Result<(), String> {
    for key in keys {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("{context} contains unknown key: {key}"));
        }
    }
    Ok(())
}

fn required_string(value: Option<&Value>, field: &str) -> Result<String, String> {
    value
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{field} must be a non-empty string"))
}

fn required_u32(value: Option<&Value>, field: &str) -> Result<u32, String> {
    value
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| format!("{field} must be a u32"))
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/corpus")
}

fn snapshot_tree(root: &Path) -> std::io::Result<BTreeMap<PathBuf, Vec<u8>>> {
    let mut snapshot = BTreeMap::new();
    snapshot_tree_into(root, root, &mut snapshot)?;
    Ok(snapshot)
}

fn snapshot_tree_into(
    root: &Path,
    current: &Path,
    snapshot: &mut BTreeMap<PathBuf, Vec<u8>>,
) -> std::io::Result<()> {
    let mut entries = fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            snapshot_tree_into(root, &path, snapshot)?;
        } else {
            snapshot.insert(
                path.strip_prefix(root)
                    .expect("path should be under root")
                    .to_path_buf(),
                fs::read(path)?,
            );
        }
    }
    Ok(())
}

fn contains_directory_named(root: &Path, name: &str) -> std::io::Result<bool> {
    let mut entries = fs::read_dir(root)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            if entry.file_name() == name {
                return Ok(true);
            }
            if contains_directory_named(&path, name)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn copy_tree(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::create_dir_all(target)?;
    let mut entries = fs::read_dir(source)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_tree(&source_path, &target_path)?;
        } else {
            fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "kratos-corpus-{label}-{}-{nonce}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("temporary directory should be created");
        Self { path }
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
