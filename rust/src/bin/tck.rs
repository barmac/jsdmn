//! Command line TCK runner.
//!
//! Usage: `tck <path-to-tck/TestCases> [--filter <substring>] [--csv <file>] [--quiet]`

use rsdmn::tck::run_all;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let mut root: Option<PathBuf> = None;
    let mut filter: Option<String> = None;
    let mut csv: Option<PathBuf> = None;
    let mut quiet = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--filter" => filter = args.next(),
            "--csv" => csv = args.next().map(PathBuf::from),
            "--quiet" | "-q" => quiet = true,
            _ => root = Some(PathBuf::from(a)),
        }
    }
    let root = root.unwrap_or_else(|| PathBuf::from("../tck/TestCases"));
    let results = run_all(
        &root,
        &["compliance-level-2", "compliance-level-3"],
        filter.as_deref(),
    );
    let mut passed = 0;
    for r in &results {
        if r.success {
            passed += 1;
        } else if !quiet {
            println!("FAIL {} {} {}: {}", r.suite, r.file, r.id, r.message);
        }
    }
    if let Some(p) = csv {
        let mut f = std::fs::File::create(p).expect("cannot create csv");
        for r in &results {
            let status = if r.success { "SUCCESS" } else { "ERROR" };
            let msg = if r.success {
                String::new()
            } else {
                r.message.replace('"', "'")
            };
            writeln!(
                f,
                "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"",
                r.suite, r.file, r.id, status, msg
            )
            .unwrap();
        }
    }
    println!("{} / {} test cases passed", passed, results.len());
    if passed != results.len() {
        std::process::exit(1);
    }
}
