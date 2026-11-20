//! Runs the DMN TCK (git submodule at `../tck`) and requires every
//! compliance level 2 and 3 test case to pass.

use rsdmn::tck::run_all;
use std::path::Path;

#[test]
fn dmn_tck_compliance_levels_2_and_3() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tck/TestCases");
    if !root.join("compliance-level-3").exists() {
        assert!(
            std::env::var("RSDMN_REQUIRE_TCK").is_err(),
            "TCK not found at {}; run `git submodule update --init`",
            root.display()
        );
        eprintln!(
            "skipping: TCK not found at {} (run `git submodule update --init`)",
            root.display()
        );
        return;
    }
    let results = run_all(&root, &["compliance-level-2", "compliance-level-3"], None);
    let failures: Vec<_> = results.iter().filter(|r| !r.success).collect();
    for f in &failures {
        eprintln!("FAIL {} {} {}: {}", f.suite, f.file, f.id, f.message);
    }
    assert!(
        results.len() > 3000,
        "only {} test cases found",
        results.len()
    );
    assert!(
        failures.is_empty(),
        "{} of {} TCK test cases failed",
        failures.len(),
        results.len()
    );
}
