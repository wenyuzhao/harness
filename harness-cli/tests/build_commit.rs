use clap::Parser;
use tempdir::TempDir;

mod helper;

#[test]
fn test_build_commit() -> anyhow::Result<()> {
    let temp_dir = TempDir::new("harness")?;
    let test_crate = temp_dir.path();
    // Create project
    std::env::set_current_dir(test_crate)?;
    helper::exec("cargo", &["init", "--name", "harness-test", "--lib"])?;
    std::fs::create_dir_all("benches")?;
    std::fs::write(
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
    std::fs::write(
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
    std::fs::write(".gitignore", "/target\nCargo.lock")?;
    // Build project
    helper::exec("cargo", &["build"])?;
    // Commit
    helper::exec("git", &["add", "."])?;
    helper::exec("git", &["commit", "-m", "test"])?;
    let commit1 = helper::get_latest_commit()?;
    println!("Commit #1: {}", commit1);
    // Make some changes to Cargo.lock
    helper::exec("cargo", &["add", "structopt"])?;
    helper::exec("cargo", &["build"])?;
    helper::exec("git", &["add", "."])?;
    helper::exec("git", &["commit", "-m", "test"])?;
    let commit2 = helper::get_latest_commit()?;
    println!("Commit #2: {}", commit2);
    // Run benchmark
    harness_cli::entey(&harness_cli::Cli::parse_from(vec![
        "harness", "run", "-i", "1", "-n", "1",
    ]))?;
    // Check output
    let output1 = std::fs::read_to_string("target/harness/logs/latest/foo.HEAD~1.log")?;
    assert!(!output1.contains("structopt"));
    assert!(output1.contains(&format!("commit: {commit1}\n")),);
    let output2 = std::fs::read_to_string("target/harness/logs/latest/foo.HEAD.log")?;
    assert!(output2.contains("structopt"));
    assert!(output2.contains(&format!("commit: {commit2}\n")),);
    Ok(())
}
