use assert_cmd::prelude::{CommandCargoExt, OutputAssertExt}; // Add methods on commands

#[test]
fn normal_run() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = std::process::Command::cargo_bin("jmrg")?;
    cmd.args(vec!["-M", "t", "-D", "dt"])
        .arg("./tests/data/1.json")
        .arg("./tests/data/2.json.gz")
        .arg("./tests/data/3.json.bz2");

    let pred = predicates::str::is_match(
        "\\{\"add\": \"15_1\"\\, \"dt\": \"1970-01-01T00:00:00\\.015000000\\+00:00\"}\
        \n\\{\"t\":15, \"add\": \"15_3\"\\}\
        \n\\{\"t\":16, \"add\": \"16_2\"\\}\
        \n\\{\"t\":16, \"add\": \"16_1\"\\}\
        \n\\{\"t\":17, \"add\": \"17_2\"\\}\
        \n\\{\"t\":18, \"add\": \"18_1\"\\}\
        \n\\{\"t\":19, \"add\": \"19_3\"\\}",
    )
    .unwrap();
    cmd.assert()
        .success()
        .stdout(pred)
        .stderr(predicates::str::is_empty());
    Ok(())
}
