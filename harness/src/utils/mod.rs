use std::{env, fs::File, path::PathBuf, process::Output};

use once_cell::sync::Lazy;

/// Downloads a file from the given URL and saves it to the cache dir.
/// This file will be cached and reused for future runs, until a `cargo clean` is performed.
#[cfg(feature = "net")]
pub fn download_file(key: impl AsRef<str>, url: impl AsRef<str>) -> anyhow::Result<PathBuf> {
    let cache_dir = PathBuf::from(env::var("HARNESS_BENCH_CACHE_DIR").unwrap());
    let path = cache_dir.join(key.as_ref());
    if path.exists() {
        return Ok(path);
    }
    let mut response = reqwest::blocking::get(url.as_ref())?;
    let mut file = File::create(&path)?;
    response.copy_to(&mut file)?;
    Ok(path)
}

/// Get a cached file from the cache dir.
pub fn get_cached_file(key: impl AsRef<str>) -> Option<PathBuf> {
    let cache_dir = PathBuf::from(env::var("HARNESS_BENCH_CACHE_DIR").unwrap());
    let file = cache_dir.join(key.as_ref());
    if file.exists() {
        Some(file)
    } else {
        None
    }
}

/// Execute a command.
pub fn exec(cmd: impl AsRef<str>, args: &[&str]) -> anyhow::Result<()> {
    let status = std::process::Command::new(cmd.as_ref())
        .args(args)
        .status()?;
    if !status.success() {
        anyhow::bail!("Failed to run command: {}", status);
    }
    Ok(())
}

/// Execute a command and capture all outputs.
pub fn exec_captured(cmd: impl AsRef<str>, args: &[&str]) -> anyhow::Result<Output> {
    let output = std::process::Command::new(cmd.as_ref())
        .args(args)
        .output()?;
    Ok(output)
}

/// The cache directory for all cached benchmarks.
/// This directory will NOT be erased until a `cargo clean` is performed.
pub static HARNESS_BENCH_CACHE_DIR: Lazy<PathBuf> = Lazy::new(|| {
    PathBuf::from(env::var("HARNESS_BENCH_CACHE_DIR").expect("HARNESS_BENCH_CACHE_DIR not set"))
});

/// The scratch directory for the current benchmark iteration.
/// This directory will be erased before each iteration.
pub static HARNESS_BENCH_SCRATCH_DIR: Lazy<PathBuf> = Lazy::new(|| {
    PathBuf::from(env::var("HARNESS_BENCH_SCRATCH_DIR").expect("HARNESS_BENCH_CACHE_DIR not set"))
});

/// The run ID for the current benchmark run.
pub static HARNESS_BENCH_RUNID: Lazy<PathBuf> = Lazy::new(|| {
    PathBuf::from(env::var("HARNESS_BENCH_RUNID").expect("HARNESS_BENCH_CACHE_DIR not set"))
});
