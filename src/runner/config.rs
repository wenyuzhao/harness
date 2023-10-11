use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;

#[derive(Deserialize)]
struct CargoConfig {
    package: CargoConfigPackage,
    #[serde(flatten)]
    _others: HashMap<String, toml::Value>,
}

#[derive(Deserialize)]
struct CargoConfigPackage {
    metadata: CargoConfigPackageMetadata,
    #[serde(flatten)]
    _others: HashMap<String, toml::Value>,
}

#[derive(Deserialize)]
struct CargoConfigPackageMetadata {
    harness: Config,
    #[serde(flatten)]
    _others: HashMap<String, toml::Value>,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub profiles: HashMap<String, Profile>,
}

fn one() -> usize {
    1
}

#[derive(Deserialize, Debug, Clone)]
pub struct Profile {
    #[serde(default)]
    pub probes: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(rename = "build-variants")]
    pub build_variants: HashMap<String, BuildVariant>,
    /// Number of iterations
    #[serde(default = "one")]
    pub iterations: usize,
    /// Number of invocations
    #[serde(default = "one")]
    pub invocations: usize,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize, Debug, Clone)]
pub struct BuildVariant {
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default = "default_true", rename = "default-features")]
    pub default_features: bool,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub commit: Option<String>,
}

pub fn load_from_cargo_toml() -> anyhow::Result<Config> {
    if !PathBuf::from("./Cargo.toml").is_file() {
        anyhow::bail!("Failed to load ./Cargo.toml");
    }
    let s = std::fs::read_to_string("./Cargo.toml")?;
    Ok(toml::from_str::<CargoConfig>(&s)?.package.metadata.harness)
}
