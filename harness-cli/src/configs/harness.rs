//! The harness benchmarking configs
//!
//! This should be placed in the `[package.metadata.harness]` section of the `Cargo.toml` file.
//!
//! If the `harness` section is not present, a default config will be created, which contains
//! a default profile, with two builds: `HEAD` pointing to the current commit, and `HEAD~1` pointing to the previous commit.
//!
//! # Example:
//!
//! The following example defines a `default` profile.
//!
//! Note that the `default` profile will be used by the runner by default,
//! if no profile name is specified when running `cargo harness run`.
//!
//! ```toml
//! [package.metadata.harness.profiles.default]
//! iterations = 3 # Optional. Default to 5
//! invocations = 40 # Optional. Default to 10
//! probes = ["harness-perf"] # Optional. Default to an empty list
//! # Additional environment variables to set for all builds and benchmarks
//! # Optional. Default to no additional environment variables
//! env = { PERF_EVENTS = "PERF_COUNT_HW_CPU_CYCLES,PERF_COUNT_HW_INSTRUCTIONS" }
//!
//! # The list of builds to evaluate.
//! # If not specified, two builds `HEAD` and `HEAD~1` will be evaluated by default.
//! [package.metadata.harness.profiles.default.builds]
//! # No extra build configurations.
//! # Default cargo features and the current git commit will be used to produce the build.
//! # No extra environment variables will be set.
//! foo = {}
//! # Another build with extra cargo features, and disabled default features
//! bar = { features = ["unstable"], default-features = false }
//! # Extra environment variables only for this build.
//! baz = { env = { "FOO" = "BAR" } }
//! # Compile this build with a specific git commit.
//! qux = { commit = "a1b2c3d4e5f6" }
//! ````
use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use toml::Table;

/// The information we care in a Cargo.toml
#[derive(Deserialize)]
pub(crate) struct CargoConfig {
    /// The packege section of the Cargo.toml
    package: CargoConfigPackage,
    /// The bench list of the Cargo.toml
    #[serde(default)]
    bench: Vec<CargoBenchConfig>,
    /// Other fields
    #[serde(flatten)]
    _others: HashMap<String, toml::Value>,
}

impl CargoConfig {
    /// Load the Cargo.toml file
    fn load_cargo_toml() -> anyhow::Result<CargoConfig> {
        if !PathBuf::from("./Cargo.toml").is_file() {
            anyhow::bail!("Failed to load ./Cargo.toml");
        }
        let s = std::fs::read_to_string("./Cargo.toml")?;
        Ok(toml::from_str::<CargoConfig>(&s)?)
    }

    pub(crate) fn load_benches() -> anyhow::Result<Vec<String>> {
        Ok(Self::load_cargo_toml()?
            .bench
            .iter()
            .filter_map(|b| {
                if !b.harness {
                    Some(b.name.clone())
                } else {
                    None
                }
            })
            .collect())
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
struct CargoBenchConfig {
    /// bench name
    name: String,
    /// we only care about benches with `harness=false`
    #[serde(default)]
    harness: bool,
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

/// The harness configuration.
///
/// This should be placed in the `[package.metadata.harness]` section of the `Cargo.toml` file.
///
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

/// The benchmarking profile.
///
/// A harness config can contain multiple profiles, each with a unique name.
///
/// The `default` profile will be used by the runner by default.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Profile {
    /// Enabled probes and their configurations. The configuration must be a TOML table (e.g. `example_probe = { param = "42" }`).
    #[serde(default)]
    pub probes: HashMap<String, Table>,
    /// Environment variables to set to all builds and benchmarks
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Builds to evaluate
    #[serde(default)]
    pub builds: HashMap<String, BuildConfig>,
    /// Number of iterations. Default is 5
    #[serde(default = "default_iterations")]
    pub iterations: usize,
    /// Number of invocations. Default is 10
    #[serde(default = "default_invocations")]
    pub invocations: usize,
    /// The baseline build name. This is only used for data reporting.
    pub baseline: Option<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            probes: HashMap::new(),
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
    /// Extra cargo features used for compilation. Default to no extra features.
    #[serde(default)]
    pub features: Vec<String>,
    /// Whether to use default features. Default to `true`
    #[serde(default = "default_true", rename = "default-features")]
    pub default_features: bool,
    /// Environment variables to set. Default to no extra environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// The commit used to produce the build. Default to the current commit.
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
