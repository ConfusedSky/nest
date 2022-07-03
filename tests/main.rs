use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::{borrow::Cow, fs::read_to_string, path::Path, process::Command}; // Run programs

// TODO: Make failure output better
wren_macros::generate_tests!();

fn test_script(script: &str) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("WE SHOULD HAVE ACCESS TO THE MANIFEST DIR");
    let manifest_path = Path::new(&manifest_dir);
    let mut script_path = manifest_path.join("scripts").join(script);
    script_path.set_extension("wren");

    if !script_path.is_file() {
        panic!("Script file does not exist at {:?}", script_path);
    }

    let text =
        read_to_string(&script_path).unwrap_or_else(|_| panic!("Failed to read {:?}", script_path));
    let expectations = text.split('\n');
    let expectation = "// expect";
    let expectations =
        expectations.filter_map(|item| item.split(expectation).nth(1).map(str::to_string));
    let mut raw_expectations: Vec<String> = Vec::new();
    let mut ordered_expectations: Vec<String> = Vec::new();
    for e in expectations {
        if e.starts_with(':') {
            raw_expectations.push(e);
        } else {
            ordered_expectations.push(e);
        }
    }
    ordered_expectations.sort();

    ordered_expectations.extend(raw_expectations);
    let expectations: Vec<String> = ordered_expectations
        .into_iter()
        .filter_map(|x| x.split(": ").nth(1).map(str::to_string))
        .map(|s| s.trim().to_string())
        .collect();
    let expectations = expectations.join("\n") + "\n";

    let mut cmd = Command::cargo_bin("rust_wren")?;
    cmd.arg(script);
    cmd.assert()
        .success()
        .stdout(predicate::str::diff(Cow::from(expectations)));

    Ok(())
}
