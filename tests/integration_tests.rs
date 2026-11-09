use std::fs;
use std::path::Path;
use qlean::lint_source;
use qlean::output::sarif::format_sarif;

#[test]
fn test_ancilla_leak_good() {
    let dir = Path::new("tests/fixtures/ancilla_leak/good");
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let source = fs::read_to_string(&path).unwrap();
        let diags = lint_source(path.to_str().unwrap(), &source);
        let leaks: Vec<_> = diags.iter().filter(|d| d.rule == "ancilla-leak").collect();
        assert!(
            leaks.is_empty(),
            "Expected 0 leaks for {:?}, but got {:?}",
            path,
            leaks
        );
    }
}

#[test]
fn test_ancilla_leak_bad() {
    let dir = Path::new("tests/fixtures/ancilla_leak/bad");
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let source = fs::read_to_string(&path).unwrap();
        let diags = lint_source(path.to_str().unwrap(), &source);
        let leaks: Vec<_> = diags.iter().filter(|d| d.rule == "ancilla-leak").collect();
        assert!(
            !leaks.is_empty(),
            "Expected at least 1 leak for {:?}, but got none",
            path
        );
    }
}

#[test]
fn test_missing_uncompute_good() {
    let dir = Path::new("tests/fixtures/missing_uncompute/good");
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let source = fs::read_to_string(&path).unwrap();
        let diags = lint_source(path.to_str().unwrap(), &source);
        let uncomputes: Vec<_> = diags.iter().filter(|d| d.rule == "missing-uncompute").collect();
        assert!(
            uncomputes.is_empty(),
            "Expected 0 missing uncompute for {:?}, but got {:?}",
            path,
            uncomputes
        );
    }
}

#[test]
fn test_missing_uncompute_bad() {
    let dir = Path::new("tests/fixtures/missing_uncompute/bad");
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let source = fs::read_to_string(&path).unwrap();
        let diags = lint_source(path.to_str().unwrap(), &source);
        let uncomputes: Vec<_> = diags.iter().filter(|d| d.rule == "missing-uncompute").collect();
        assert!(
            !uncomputes.is_empty(),
            "Expected at least 1 missing uncompute for {:?}, but got none",
            path
        );
    }
}

#[test]
fn test_mid_circuit_entanglement_good() {
    let dir = Path::new("tests/fixtures/mid_circuit_entanglement/good");
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let source = fs::read_to_string(&path).unwrap();
        let diags = lint_source(path.to_str().unwrap(), &source);
        let entanglements: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "mid-circuit-entanglement")
            .collect();
        assert!(
            entanglements.is_empty(),
            "Expected 0 entanglement warnings for {:?}, but got {:?}",
            path,
            entanglements
        );
    }
}

#[test]
fn test_mid_circuit_entanglement_bad() {
    let dir = Path::new("tests/fixtures/mid_circuit_entanglement/bad");
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let source = fs::read_to_string(&path).unwrap();
        let diags = lint_source(path.to_str().unwrap(), &source);
        let entanglements: Vec<_> = diags
            .iter()
            .filter(|d| d.rule == "mid-circuit-entanglement")
            .collect();
        assert!(
            !entanglements.is_empty(),
            "Expected at least 1 entanglement warning for {:?}, but got none",
            path
        );
    }
}

#[test]
fn test_corpus_no_false_positives() {
    let dir = Path::new("tests/corpus");
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let source = fs::read_to_string(&path).unwrap();
        let diags = lint_source(path.to_str().unwrap(), &source);
        assert!(
            diags.is_empty(),
            "Expected 0 diagnostics for {:?}, but got {:?}",
            path,
            diags
        );
    }
}

#[test]
fn test_sarif_output_generation() {
    let source = "namespace Test { operation Bad() : Unit { use q = Qubit(); X(q); } }";
    let diags = lint_source("test.qs", source);
    let sarif = format_sarif(&diags);
    assert!(sarif.contains("sarif-schema-2.1.0.json"));
    assert!(sarif.contains("ancilla-leak"));
}

#[test]
fn test_include_ast_lints() {
    let source = "namespace Test { operation Op() : Unit { let x = 1 / 0; } }";
    let default_diags = qlean::lint_source("test.qs", source);
    assert!(default_diags.is_empty());

    let ast_diags = qlean::lint_source_with_options("test.qs", source, true);
    assert!(!ast_diags.is_empty());
    assert!(ast_diags.iter().any(|d| d.rule == "divisionByZero"));
}
