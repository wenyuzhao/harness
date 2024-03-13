use crate::helper::TestCrate;

mod helper;

const BENCH_DUMP_LOCKFILE: &str = r#"
#[harness::bench]
fn bench(bencher: &harness::Bencher) {
    bencher.time(|| {});
    let lock = std::fs::read_to_string("Cargo.lock").unwrap();
    println!("{}", lock);
}
"#;

const CARGO_TOML_COMMON: &str = r#"
[package]
name = "harness-test"
version = "0.1.0"
edition = "2021"

[dev-dependencies]
harness = "0.0.4"

[[bench]]
name = "foo"
harness = false
"#;

#[test]
fn test_different_lockfiles() -> anyhow::Result<()> {
    let _guard = helper::SYNC.lock().unwrap();
    let mut test_crate = TestCrate::new(None)?.enter()?;
    test_crate.file("benches/foo.rs", BENCH_DUMP_LOCKFILE)?;
    test_crate.file("Cargo.toml", CARGO_TOML_COMMON)?;
    test_crate.commit()?;
    // Make some changes to Cargo.lock
    test_crate.add_dep("structopt")?;
    test_crate.commit()?;
    // Run benchmark
    test_crate.harness_run(&["-i", "1", "-n", "1"])?;
    // Check output
    let output1 = test_crate.get_harness_log("foo", "HEAD~1")?;
    assert!(!output1.contains("structopt"));
    let output2 = test_crate.get_harness_log("foo", "HEAD")?;
    assert!(output2.contains("structopt"));
    Ok(())
}

#[test]
fn test_reproduced_run() -> anyhow::Result<()> {
    let _guard = helper::SYNC.lock().unwrap();
    let mut test_crate = TestCrate::new(None)?.enter()?;
    test_crate.file("benches/foo.rs", BENCH_DUMP_LOCKFILE)?;
    test_crate.file(
        "Cargo.toml",
        format!(
            r#"
            {CARGO_TOML_COMMON}
            [package.metadata.harness.profiles.default.builds]
            build_a = {{}}
            build_b = {{}}
            "#
        ),
    )?;
    test_crate.commit()?;
    // Run benchmark
    let runid = test_crate.harness_run(&["-i", "1", "-n", "1"])?;
    let output = test_crate.get_harness_log("foo", "build_a")?;
    assert!(!output.contains("structopt"));
    // Make some changes to Cargo.lock
    test_crate.add_dep("structopt")?;
    test_crate.commit()?;
    // Run benchmark
    test_crate.harness_run(&["-i", "1", "-n", "1", "--config", &runid])?;
    // Check output
    let output = test_crate.get_harness_log("foo", "build_a")?;
    assert!(!output.contains("structopt"));
    // Run benchmark
    test_crate.harness_run(&["-i", "1", "-n", "1"])?;
    // Check output
    let output = test_crate.get_harness_log("foo", "build_a")?;
    assert!(output.contains("structopt"));
    Ok(())
}
