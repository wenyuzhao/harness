use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub profiles: HashMap<String, Profile>,
}

fn one() -> usize {
    1
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Profile {
    #[serde(default)]
    pub probes: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub builds: HashMap<String, BuildConfig>,
    /// Number of iterations
    #[serde(default = "one")]
    pub iterations: usize,
    /// Number of invocations
    #[serde(default = "one")]
    pub invocations: usize,
    /// The baseline build name. This is only used for data reporting.
    pub baseline: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BuildConfig {
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default = "default_true", rename = "default-features")]
    pub default_features: bool,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub commit: Option<String>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            features: Vec::new(),
            default_features: true,
            env: HashMap::new(),
            commit: None,
        }
    }

}

pub fn load_from_cargo_toml() -> anyhow::Result<Config> {
    if !PathBuf::from("./Cargo.toml").is_file() {
        anyhow::bail!("Failed to load ./Cargo.toml");
    }
    let s = std::fs::read_to_string("./Cargo.toml")?;
    Ok(toml::from_str::<CargoConfig>(&s)?.package.metadata.harness)
}
