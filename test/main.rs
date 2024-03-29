use assert_cmd::prelude::*; // Add methods on commands
use itertools::Itertools;
use predicates::prelude::*; // Used for writing assertions
use std::{borrow::Cow, fs::read_to_string, path::PathBuf, process::Command}; // Run programs

// TODO: Make failure output better
wren_macros::generate_tests!();

#[test]
fn should_work_without_extension() -> Result<(), Box<dyn std::error::Error>> {
    test_script("test/empty")
}

fn test_script(script: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut script_path = PathBuf::from(script);
    if script_path.extension().is_none() {
        script_path.set_extension("wren");
    }

    if !script_path.is_file() {
        panic!("Script file does not exist at {script_path:?}");
    }

    let text =
        read_to_string(&script_path).unwrap_or_else(|_| panic!("Failed to read {script_path:?}"));
    let expectations = text.split('\n');
    let expectations =
        expectations.filter_map(|item| item.split("// expect").nth(1).map(str::to_string));
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
        .map(|x| x.split(": ").skip(1).join(": "))
        .collect();
    let mut expectations = expectations.join("\n");
    if !expectations.is_empty() {
        expectations += "\n";
    }

    let mut cmd = Command::cargo_bin("nest")?;
    cmd.arg(script);
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("WE SHOULD HAVE ACCESS TO THE MANIFEST DIR");
    cmd.current_dir(manifest_dir);
    cmd.assert()
        .success()
        .stdout(predicate::str::diff(Cow::from(expectations)));

    Ok(())
}
