use std::{fs::OpenOptions, path::Path, process::Command};

use cargo_metadata::{MetadataCommand, Package};
use clap::Parser;

#[derive(Debug)]
struct BuildVariant {
    name: String,
    features: Vec<String>,
    default_features: bool,
    commit: Option<String>,
}

#[derive(Debug)]
struct Harness {
    run_id: String,
    crate_name: String,
    benches: Vec<String>,
    variants: Vec<BuildVariant>,
    iterations: usize,
    invocations: usize,
}

impl Harness {
    fn new(
        run_id: String,
        crate_name: String,
        variants: Vec<BuildVariant>,
        iterations: usize,
        invocations: usize,
    ) -> Self {
        Self {
            run_id,
            crate_name,
            benches: Vec::new(),
            variants,
            iterations,
            invocations,
        }
    }

    fn collect_benches(&mut self) -> anyhow::Result<()> {
        let meta = MetadataCommand::new()
            .manifest_path("./Cargo.toml")
            .exec()
            .unwrap();
        let Some(pkg) = meta.root_package() else {
            anyhow::bail!("No root package found");
        };
        for target in &pkg.targets {
            if target.is_bench() {
                self.benches.push(target.name.clone());
            }
        }
        Ok(())
    }

    fn run_one(
        &self,
        variant: &BuildVariant,
        bench: &str,
        target_dir: &Path,
    ) -> anyhow::Result<()> {
        let dir = target_dir.join("harness").join("logs").join(&self.run_id);
        std::fs::create_dir_all(&dir)?;
        let log_file = dir.join(format!("{}.{}.log", bench, variant.name));
        let outputs = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(log_file)?;
        let errors = outputs.try_clone()?;
        let out = Command::new("cargo")
            .stdout(outputs)
            .stderr(errors)
            .args(["bench", "--bench", bench])
            .arg("--features")
            .arg(variant.features.join(" "))
            .args(if !variant.default_features {
                &["--no-default-features"] as &[&str]
            } else {
                &[] as &[&str]
            })
            .args(["--", "-n"])
            .arg(format!("{}", self.iterations))
            .arg("--overwrite-crate-name")
            .arg(&self.crate_name)
            .arg("--overwrite-benchmark-name")
            .arg(bench)
            .status()?;
        if out.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Failed to run bench `{}` with variant {:?}",
                bench,
                variant
            ))
        }
    }

    fn run(&mut self, target_dir: &Path) -> anyhow::Result<()> {
        self.collect_benches()?;
        for bench in &self.benches {
            print!("{} ", bench);
            for i in 0..self.invocations {
                print!("{}", i);
                for (index, variant) in self.variants.iter().enumerate() {
                    assert!(index < 26);
                    const KEYS: &str = "abcdefghijklmnopqrstuvwxyz";
                    let key = KEYS.chars().nth(index).unwrap();
                    let result = self.run_one(variant, bench, target_dir);
                    match result {
                        Ok(_) => {
                            print!("{}", key)
                        }
                        Err(_) => {
                            print!(".")
                        }
                    }
                }
            }
            println!();
        }
        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct HarnessCmdArgs {
    #[arg(short = 'n', long, default_value = "1")]
    /// Number of iterations
    pub iterations: usize,
    #[arg(short = 'i', long, default_value = "1")]
    /// Number of invocations
    pub invocations: usize,
    #[arg(long, default_value = "default")]
    /// Benchmarking profile
    pub profile: String,
}

fn get_build_variants(pkg: &Package, profile: &str) -> anyhow::Result<Vec<BuildVariant>> {
    let variants = pkg
        .metadata
        .get("harness")
        .and_then(|v| v.get("profiles"))
        .and_then(|v| v.get(profile))
        .and_then(|v| v.get("build-variants"))
        .and_then(|v| v.as_object());
    if let Some(variants) = variants {
        let mut results = vec![];
        for (k, v) in variants {
            let features = v
                .get("features")
                .and_then(|v| v.as_array())
                .map(|v| v.iter().map(|v| v.as_str().unwrap().to_owned()).collect())
                .unwrap_or_default();
            let default_features = v
                .get("default-features")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let commit = v
                .get("commit")
                .and_then(|v| v.as_str())
                .map(|v| v.to_owned());
            results.push(BuildVariant {
                name: k.to_owned(),
                features,
                default_features,
                commit,
            });
        }
        Ok(results)
    } else {
        anyhow::bail!(
            "Key `package.metadata.harness.profiles.{}.build-variants` not found in Cargo.toml",
            profile
        );
    }
}

fn main() -> anyhow::Result<()> {
    let args: std::env::Args = std::env::args();
    let mut args = args.collect::<Vec<_>>();
    if args.len() > 1 && args[1] == "harness" {
        args = args[1..].to_vec();
    }
    let args = HarnessCmdArgs::parse_from(args);
    let meta = MetadataCommand::new()
        .manifest_path("./Cargo.toml")
        .exec()
        .unwrap();
    let target_dir = meta.target_directory.as_std_path();
    let Some(pkg) = meta.root_package() else {
        anyhow::bail!("No root package found");
    };
    let variants = get_build_variants(pkg, &args.profile)?;
    let time = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let run_id = format!("{}-{}", args.profile, time);
    let mut harness = Harness::new(
        run_id,
        pkg.name.clone(),
        variants,
        args.iterations,
        args.invocations,
    );
    harness.run(target_dir)?;
    Ok(())
}
