use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub(crate) struct CargoConfig {
    package: CargoConfigPackage,
    #[serde(default)]
    pub(crate) bench: Vec<CargoBenchConfig>,
    #[serde(flatten)]
    _others: HashMap<String, toml::Value>,
}

#[derive(Deserialize)]
struct CargoConfigPackage {
    metadata: Option<CargoConfigPackageMetadata>,
    #[serde(flatten)]
    _others: HashMap<String, toml::Value>,
}

#[derive(Deserialize)]
pub(crate) struct CargoBenchConfig {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) harness: bool,
    #[serde(flatten)]
    _others: HashMap<String, toml::Value>,
}

#[derive(Deserialize)]
struct CargoConfigPackageMetadata {
    harness: Option<HarnessConfig>,
    #[serde(flatten)]
    _others: HashMap<String, toml::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HarnessConfig {
    pub profiles: HashMap<String, Profile>,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            profiles: [("default".to_owned(), Default::default())]
                .into_iter()
                .collect(),
        }
    }
}

fn default_iterations() -> usize {
    5
}

fn default_invocations() -> usize {
    10
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Profile {
    #[serde(default)]
    pub probes: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub builds: HashMap<String, BuildConfig>,
    /// Number of iterations
    #[serde(default = "default_iterations")]
    pub iterations: usize,
    /// Number of invocations
    #[serde(default = "default_invocations")]
    pub invocations: usize,
    /// The baseline build name. This is only used for data reporting.
    pub baseline: Option<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            probes: Vec::new(),
            env: HashMap::new(),
            builds: HashMap::new(),
            iterations: default_iterations(),
            invocations: default_invocations(),
            baseline: None,
        }
    }
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

/// Load the harness configuration from the `Cargo.toml` file
/// If the `harness` section is not present, a default config with a default profile is returned.
pub fn load_from_cargo_toml() -> anyhow::Result<HarnessConfig> {
    if !PathBuf::from("./Cargo.toml").is_file() {
        anyhow::bail!("Failed to load ./Cargo.toml");
    }
    let s = std::fs::read_to_string("./Cargo.toml")?;
    let mut harness = toml::from_str::<CargoConfig>(&s)?
        .package
        .metadata
        .and_then(|m| m.harness)
        .unwrap_or_default();
    if harness.profiles.is_empty() {
        harness
            .profiles
            .insert("default".to_owned(), Default::default());
    }
    Ok(harness)
}

pub fn load_cargo_toml() -> anyhow::Result<CargoConfig> {
    if !PathBuf::from("./Cargo.toml").is_file() {
        anyhow::bail!("Failed to load ./Cargo.toml");
    }
    let s = std::fs::read_to_string("./Cargo.toml")?;
    Ok(toml::from_str::<CargoConfig>(&s)?)
}
