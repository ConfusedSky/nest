use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs

#[test]
fn test_scheduler() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("rust_wren")?;
    eprintln!("{:?}", std::env::current_dir().unwrap());
    cmd.arg("test_scheduler");
    cmd.assert()
        .success()
        .stdout(predicate::eq(include_str!("test_scheduler.stdout")));

    Ok(())
}
