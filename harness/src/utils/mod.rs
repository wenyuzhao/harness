use std::{path::PathBuf, process::Output};

/// Downloads a file from the given URL and saves it to the cache dir.
/// This file will be cached and reused for future runs, until a `cargo clean` is performed.
pub fn download_file(key: impl AsRef<str>, url: impl AsRef<str>) -> anyhow::Result<PathBuf> {
    let cache_dir = PathBuf::from(env::var("HARNESS_BENCH_CACHE_DIR").unwrap());
    let file = cache_dir.join(key);
    if file.exists() {
        return Ok(file);
    }
    let mut response = reqwest::blocking::get(url)?;
    let mut file = File::create(file)?;
    response.copy_to(&mut file)?;
    Ok(())
}

/// Get a cached file from the cache dir.
pub fn get_cached_file(key: impl AsRef<str>) -> Option<PathBuf> {
    let cache_dir = PathBuf::from(env::var("HARNESS_BENCH_CACHE_DIR").unwrap());
    let file = cache_dir.join(key);
    if file.exists() {
        Some(file)
    } else {
        None
    }
}

/// Execute a command.
pub fn exec(cmd: impl AsRef<str>, args: impl Iterator<Item = &str>) -> anyhow::Result<()> {
    let status = std::process::Command::new(cmd).args(args).status()?;
    if !status.success() {
        anyhow::bail!("Failed to run command: {}", status);
    }
    Ok(())
}

/// Execute a command and capture all outputs.
pub fn exec_captured(
    cmd: impl AsRef<str>,
    args: impl Iterator<Item = &str>,
) -> anyhow::Result<Output> {
    let output = std::process::Command::new(cmd).args(args).output()?;
    Ok(output)
}
