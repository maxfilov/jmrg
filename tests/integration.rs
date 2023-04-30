use assert_cmd::prelude::{CommandCargoExt, OutputAssertExt}; // Add methods on commands

#[test]
#[ignore]
fn normal_run() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = std::process::Command::cargo_bin("jmrg")?;
    cmd.args(vec!["-k", "t"])
        .arg("./tests/data/1.json")
        .arg("./tests/data/2.json.gz")
        .arg("./tests/data/3.json.bz2");

    let predicate = predicates::path::eq_file("./tests/data/output/normal_run.json")
        .utf8()
        .unwrap();
    cmd.assert().success().stdout(predicate);
    Ok(())
}
