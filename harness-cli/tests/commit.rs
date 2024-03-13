use crate::helper::TestCrate;

mod helper;

const BENCH_DUMP_GIT_COMMIT: &str = r#"
#[harness::bench]
fn bench(bencher: &harness::Bencher) {
    bencher.time(|| {});
    println!("GIT[{}]", git_info2::get().head.last_commit_hash.unwrap().as_str());
}
"#;

const CARGO_TOML_COMMON: &str = r#"
[package]
name = "harness-test"
version = "0.1.0"
edition = "2021"

[dev-dependencies]
harness = "0.0.4"
git_info2 = "0.1.2"

[[bench]]
name = "foo"
harness = false
"#;

#[test]
fn test_default_build_targets() -> anyhow::Result<()> {
    let _guard = helper::SYNC.lock().unwrap();
    let mut test_crate = TestCrate::new(None)?.enter()?;
    test_crate.file("benches/foo.rs", BENCH_DUMP_GIT_COMMIT)?;
    test_crate.file("Cargo.toml", CARGO_TOML_COMMON)?;
    let commit1 = test_crate.commit()?;
    // Make some changes to Cargo.lock
    test_crate.add_dep("structopt")?;
    let commit2 = test_crate.commit()?;
    // Run benchmark
    test_crate.harness_run(&["-i", "1", "-n", "1"])?;
    // Check output
    let output1 = test_crate.get_harness_log("foo", "HEAD~1")?;
    assert!(output1.contains(&format!("GIT[{commit1}]")));
    let output2 = test_crate.get_harness_log("foo", "HEAD")?;
    assert!(output2.contains(&format!("GIT[{commit2}]")));
    assert_eq!(test_crate.get_current_branch(), Some("main".to_owned()));
    Ok(())
}

#[test]
fn test_empty_build_targets() -> anyhow::Result<()> {
    let _guard = helper::SYNC.lock().unwrap();
    let mut test_crate = TestCrate::new(None)?.enter()?;
    test_crate.file("benches/foo.rs", BENCH_DUMP_GIT_COMMIT)?;
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
    let commit = test_crate.commit()?;
    // Run benchmark
    test_crate.harness_run(&["-i", "1", "-n", "1"])?;
    // Check output
    let output1 = test_crate.get_harness_log("foo", "build_a")?;
    assert!(output1.contains(&format!("GIT[{commit}]")));
    let output2 = test_crate.get_harness_log("foo", "build_b")?;
    assert!(output2.contains(&format!("GIT[{commit}]")));
    assert_eq!(test_crate.get_current_branch(), Some("main".to_owned()));
    Ok(())
}

#[test]
fn test_build_targets_with_different_commits() -> anyhow::Result<()> {
    let _guard = helper::SYNC.lock().unwrap();
    let mut test_crate = TestCrate::new(None)?.enter()?;
    test_crate.file("benches/foo.rs", BENCH_DUMP_GIT_COMMIT)?;
    test_crate.file("Cargo.toml", CARGO_TOML_COMMON)?;
    let commit1 = test_crate.commit()?;
    test_crate.file(
        "Cargo.toml",
        format!(
            r#"
            {CARGO_TOML_COMMON}

            [package.metadata.harness.profiles.default.builds]
            build_a = {{}}
            build_b = {{ commit = "{commit1}" }}
        "#
        ),
    )?;
    let commit2 = test_crate.commit()?;
    // Run benchmark
    test_crate.harness_run(&["-i", "1", "-n", "1"])?;
    // Check output
    let output1 = test_crate.get_harness_log("foo", "build_a")?;
    assert!(output1.contains(&format!("GIT[{commit2}]")));
    let output2 = test_crate.get_harness_log("foo", "build_b")?;
    assert!(output2.contains(&format!("GIT[{commit1}]")));
    assert_eq!(test_crate.get_current_branch(), Some("main".to_owned()));
    Ok(())
}
