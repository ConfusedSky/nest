use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::{
    borrow::Cow,
    fs::read_to_string,
    path::{Path, PathBuf},
    process::Command,
}; // Run programs

macro_rules! print {
    ($($xs:expr),*) => {{
        use std::io::Write;
        let _ = std::write!(std::io::stderr().lock(), $($xs),*);
    }};
}

macro_rules! println {
    ($($xs:expr),*) => {{
        use std::io::Write;
        let _ = std::writeln!(std::io::stderr().lock(), $($xs),*);
    }};
}

#[test]
fn integration_test_runner() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("WE SHOULD HAVE ACCESS TO THE MANIFEST DIR");
    let dir = PathBuf::from(manifest_dir + "/scripts/test");

    let mut failures = Vec::new();

    for file in dir.read_dir().expect("We can read the directory").flatten() {
        let file = file
            .file_name()
            .to_str()
            .ok_or("Failed to convert filename to string")?
            .to_string();
        let file = file
            .split('.')
            .next()
            .ok_or("Failed to remove the extension from script name")?;

        print!("test {} ... ", file);

        let script = "test/".to_string() + file;
        let result = std::panic::catch_unwind(|| test_script(script.as_str()));
        match result {
            Ok(res) => match res {
                Ok(()) => {
                    println!("ok")
                }
                Err(e) => {
                    println!("FAILED");
                    println!("{:?}", e);
                    panic!("{}", e);
                }
            },
            Err(e) => {
                println!("FAILED");
                failures.push(e);
            }
        }
    }

    if !failures.is_empty() {
        let first = failures.pop().unwrap();
        std::panic::resume_unwind(first);
    }

    Ok(())
}

fn test_script(script: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(file!());
    let mut script_path = path
        .parent()
        .expect("Couldn't get this files parent path")
        .parent()
        .expect("Couldn't get this files parent path")
        .join("scripts")
        .join(script);
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
