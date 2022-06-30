use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs

#[test]
fn test_scheduler() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("rust_wren")?;
    cmd.arg("test_scheduler");
    cmd.assert()
        .success()
        .stdout(predicate::eq(include_str!("test_scheduler.stdout")));

    Ok(())
}

#[test]
fn test_scheduled_tasks_should_run() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("rust_wren")?;
    cmd.arg("scheduled_tasks_should_run");
    cmd.assert().success().stdout(predicate::eq("test\n"));

    Ok(())
}
