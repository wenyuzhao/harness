#![allow(unused)]

use std::{path::Path, sync::Mutex, vec};

use clap::Parser;
use tempdir::TempDir;

pub static SYNC: Mutex<()> = Mutex::new(());

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
    commits: usize,
    prev_pwd: Option<std::path::PathBuf>,
}

impl Drop for TestCrate {
    fn drop(&mut self) {
        if let Some(prev_pwd) = self.prev_pwd.as_ref() {
            std::env::set_current_dir(prev_pwd).unwrap();
        }
    }
}

impl TestCrate {
    pub fn new(name: Option<&str>) -> anyhow::Result<Self> {
        let temp_dir = TempDir::new("harness")?;
        let test_crate = temp_dir.path();
        // Create project
        let dir = std::env::current_dir()?;
        std::env::set_current_dir(test_crate)?;
        println!("Creating test crate in {}", test_crate.display());
        let name = name.unwrap_or("harness-test");
        exec("cargo", &["init", "--name", name, "--lib"])?;
        std::fs::write(".gitignore", "/target\nCargo.lock")?;
        exec("cargo", &["build"])?;
        exec("git", &["add", "."])?;
        exec("git", &["commit", "-m", "Initial Commit"])?;
        exec("git", &["branch", "-M", "main"])?;
        std::env::set_current_dir(dir)?;
        Ok(Self {
            temp_dir,
            commits: 0,
            prev_pwd: None,
        })
    }

    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    pub fn enter(mut self) -> anyhow::Result<Self> {
        self.prev_pwd = Some(std::env::current_dir()?);
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
        self.commits += 1;
        let commit = get_latest_commit()?;
        println!("Commit #{}: {}", self.commits, commit);
        Ok(commit)
    }

    pub fn harness_run(&self, args: &[&str]) -> anyhow::Result<String> {
        let mut cmd_args = vec!["harness", "run"];
        cmd_args.extend_from_slice(args);
        harness_cli::entey(&harness_cli::Cli::parse_from(cmd_args))?;
        let config_toml_str = std::fs::read_to_string("target/harness/logs/latest/config.toml")?;
        let config_toml: toml::Table = toml::from_str(&config_toml_str)?;
        Ok(config_toml
            .get("runid")
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned())
    }

    pub fn get_harness_log(&self, bench: &str, build: &str) -> anyhow::Result<String> {
        Ok(std::fs::read_to_string(format!(
            "target/harness/logs/latest/{}.{}.log",
            bench, build
        ))?)
    }
}
