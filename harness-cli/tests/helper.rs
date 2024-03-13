use std::vec;

use clap::Parser;
use tempdir::TempDir;

pub fn exec(cmd: impl AsRef<str>, args: &[&str]) -> anyhow::Result<()> {
    let output = std::process::Command::new(cmd.as_ref())
        .args(args)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("Failed to run command: {} {}", cmd.as_ref(), args.join(" "));
    }
    Ok(())
}

pub fn get_latest_commit() -> anyhow::Result<String> {
    if let Some(x) = git_info2::get().head.last_commit_hash.clone() {
        Ok(x)
    } else {
        anyhow::bail!("Failed to get latest commit");
    }
}

pub struct TestCrate {
    temp_dir: TempDir,
}

impl TestCrate {
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = TempDir::new("harness")?;
        let test_crate = temp_dir.path();
        // Create project
        let dir = std::env::current_dir()?;
        std::env::set_current_dir(test_crate)?;
        exec("cargo", &["init", "--name", "harness-test", "--lib"])?;
        std::fs::write(".gitignore", "/target\nCargo.lock")?;
        exec("cargo", &["build"])?;
        exec("git", &["add", "."])?;
        exec("git", &["commit", "-m", "Initial Commit"])?;
        exec("git", &["branch", "-M", "main"])?;
        std::env::set_current_dir(dir)?;
        Ok(Self { temp_dir })
    }

    pub fn enter(self) -> anyhow::Result<Self> {
        std::env::set_current_dir(self.temp_dir.path())?;
        Ok(self)
    }

    pub fn file(&mut self, path: impl AsRef<str>, content: impl AsRef<str>) -> anyhow::Result<()> {
        let full_path = self.temp_dir.path().join(path.as_ref());
        let dir = full_path.parent().unwrap();
        std::fs::create_dir_all(dir)?;
        std::fs::write(self.temp_dir.path().join(path.as_ref()), content.as_ref())?;
        Ok(())
    }

    pub fn add_dep(&mut self, dep: &str) -> anyhow::Result<()> {
        exec("cargo", &["add", dep])?;
        Ok(())
    }

    pub fn get_current_branch(&self) -> Option<String> {
        git_info2::get().current_branch
    }

    pub fn commit(&mut self) -> anyhow::Result<String> {
        exec("git", &["add", "."])?;
        exec("git", &["commit", "-m", "test"])?;
        Ok(get_latest_commit()?)
    }

    pub fn harness_run(&self, args: &[&str]) -> anyhow::Result<()> {
        let mut cmd_args = vec!["harness", "run"];
        cmd_args.extend_from_slice(args);
        harness_cli::entey(&harness_cli::Cli::parse_from(cmd_args))?;
        Ok(())
    }

    pub fn get_harness_log(&self, bench: &str, build: &str) -> anyhow::Result<String> {
        Ok(std::fs::read_to_string(format!(
            "target/harness/logs/latest/{}.{}.log",
            bench, build
        ))?)
    }
}
