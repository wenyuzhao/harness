use crate::helper::TestCrate;

mod helper;

#[test]
fn test_build_commit() -> anyhow::Result<()> {
    let mut test_crate = TestCrate::new()?.enter()?;
    test_crate.file(
        "benches/foo.rs",
        r#"
            use harness::{bench, Bencher};

            #[bench]
            fn bench(bencher: &Bencher) {
                bencher.time(|| {});
                let lock = std::fs::read_to_string("Cargo.lock").unwrap();
                println!("{}", lock);
            }
        "#,
    )?;
    test_crate.file(
        "Cargo.toml",
        r#"
            [package]
            name = "harness-test"
            version = "0.1.0"
            edition = "2021"

            [dev-dependencies]
            harness = "0.0.4"

            [[bench]]
            name = "foo"
            harness = false
        "#,
    )?;
    let commit1 = test_crate.commit()?;
    println!("Commit #1: {}", commit1);
    // Make some changes to Cargo.lock
    test_crate.add_dep("structopt")?;
    let commit2 = test_crate.commit()?;
    println!("Commit #2: {}", commit2);
    // Run benchmark
    test_crate.harness_run(&["-i", "1", "-n", "1"])?;
    // Check output
    let output1 = test_crate.get_harness_log("foo", "HEAD~1")?;
    assert!(!output1.contains("structopt"));
    assert!(output1.contains(&format!("commit: {commit1}\n")));
    let output2 = test_crate.get_harness_log("foo", "HEAD")?;
    assert!(output2.contains("structopt"));
    assert!(output2.contains(&format!("commit: {commit2}\n")));
    Ok(())
}
