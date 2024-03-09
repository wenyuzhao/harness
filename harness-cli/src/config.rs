use std::{collections::HashMap, path::PathBuf};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::utils;

/// The information we care in a Cargo.toml
#[derive(Deserialize)]
pub(crate) struct CargoConfig {
    /// The packege section of the Cargo.toml
    package: CargoConfigPackage,
    /// The bench list of the Cargo.toml
    #[serde(default)]
    pub(crate) bench: Vec<CargoBenchConfig>,
    /// Other fields
    #[serde(flatten)]
    _others: HashMap<String, toml::Value>,
}

impl CargoConfig {
    /// Load the Cargo.toml file
    pub fn load_cargo_toml() -> anyhow::Result<CargoConfig> {
        if !PathBuf::from("./Cargo.toml").is_file() {
            anyhow::bail!("Failed to load ./Cargo.toml");
        }
        let s = std::fs::read_to_string("./Cargo.toml")?;
        Ok(toml::from_str::<CargoConfig>(&s)?)
    }
}

/// The package section of the Cargo.toml
#[derive(Deserialize)]
struct CargoConfigPackage {
    /// The custom metadata section of the Cargo.toml
    metadata: Option<CargoConfigPackageMetadata>,
    /// Other fields
    #[serde(flatten)]
    _others: HashMap<String, toml::Value>,
}

/// The bench item of the bench list in the Cargo.toml
#[derive(Deserialize)]
pub(crate) struct CargoBenchConfig {
    /// bench name
    pub(crate) name: String,
    /// we only care about benches with `harness=false`
    #[serde(default)]
    pub(crate) harness: bool,
    /// Other fields
    #[serde(flatten)]
    _others: HashMap<String, toml::Value>,
}

/// The custom metadata section of the Cargo.toml
#[derive(Deserialize)]
struct CargoConfigPackageMetadata {
    /// The harness config
    harness: Option<HarnessConfig>,
    #[serde(flatten)]
    _others: HashMap<String, toml::Value>,
}

/// The harness configuration
#[derive(Serialize, Deserialize, Debug)]
pub struct HarnessConfig {
    /// Evaluation profiles
    pub profiles: HashMap<String, Profile>,
}

impl HarnessConfig {
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

/// The benchmarking profile
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Profile {
    /// Enabled probes
    #[serde(default)]
    pub probes: Vec<String>,
    /// Environment variables to set
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Builds to evaluate
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

/// The build configuration used for evaluation
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BuildConfig {
    /// Enabled cargo features
    #[serde(default)]
    pub features: Vec<String>,
    /// Whether to use default features
    #[serde(default = "default_true", rename = "default-features")]
    pub default_features: bool,
    /// Environment variables to set
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// The commit used to produce the build
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

/// The metdata of the crate
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrateInfo {
    /// Crate name
    pub name: String,
    /// Path to the target directory
    pub target_dir: PathBuf,
    /// Benchmark names
    pub benches: Vec<String>,
}

/// The evaluation run metadata. This will be colelcted before eaech evaluation and dumped to the log directory.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunInfo {
    /// Benchmark run id
    pub runid: String,
    /// Benchmark start time
    #[serde(rename = "start-time-utc")]
    pub start_timestamp_utc: i64,
    /// Benchmark finish time
    #[serde(rename = "finish-time-utc")]
    pub finish_timestamp_utc: Option<i64>,
    /// The commit that the profile is loaded from. This is also used as the default build commit
    pub commit: String,
    /// The crate info
    #[serde(rename = "crate")]
    pub crate_info: CrateInfo,
    /// The enabled evaluation profile
    pub profile: Profile,
    /// Current system info
    pub system: SystemInfo,
}

impl RunInfo {
    pub fn new(
        crate_info: CrateInfo,
        profile: Profile,
        runid: String,
        start_time: DateTime<Local>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            crate_info,
            system: utils::sys::get_current_system_info(),
            profile,
            runid,
            commit: utils::git::get_git_hash()?,
            start_timestamp_utc: start_time.to_utc().timestamp(),
            finish_timestamp_utc: None,
        })
    }

    pub fn load(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemInfo {
    pub host: String,
    pub os: String,
    pub arch: String,
    #[serde(rename = "kernel-version")]
    pub kernel: String,
    #[serde(rename = "cpu-model")]
    pub cpu_model: String,
    #[serde(rename = "cpu-frequency")]
    pub cpu_frequency: Vec<usize>,
    pub memory_size: usize,
    pub swap_size: usize,
    #[cfg(target_os = "linux")]
    pub users: Vec<String>,
    pub processes: usize,
    pub env: HashMap<String, String>,
    pub pid: usize,
    pub rustc: String,
    #[cfg(target_os = "linux")]
    #[serde(rename = "scaling-governor")]
    pub scaling_governor: Vec<String>,
}
