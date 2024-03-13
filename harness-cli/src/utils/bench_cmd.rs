use std::{path::Path, process::Command};

use crate::configs::{harness::Profile, run_info::RunInfo};

fn generate_cargo_build_args_and_envs(profile: &Profile, build: &str, cmd: &mut Command) {
    let build = &profile.builds[build];
    // features
    if !build.features.is_empty() {
        cmd.arg("--features");
        cmd.arg(build.features.join(","));
    }
    if !build.default_features {
        cmd.arg("--no-default-features");
    }
    // envs
    let mut envs = profile.env.clone();
    for (k, v) in &build.env {
        envs.insert(k.clone(), v.clone());
    }
    cmd.envs(envs);
}

pub fn get_bench_build_command(profile: &Profile, build: &str) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.arg("bench");
    generate_cargo_build_args_and_envs(profile, build, &mut cmd);
    cmd.arg("--no-run");
    cmd
}

pub fn get_bench_run_command(
    run: &RunInfo,
    bench: &str,
    build_name: &str,
    invocation: usize,
    log_dir: Option<&Path>,
) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.arg("bench");
    generate_cargo_build_args_and_envs(&run.profile, build_name, &mut cmd);
    // pass bench name
    cmd.args(["--bench", bench]);
    // run args
    cmd.args(["--", "-n"])
        .arg(format!("{}", run.profile.iterations))
        .arg("--overwrite-crate-name")
        .arg(&run.crate_info.name)
        .arg("--overwrite-benchmark-name")
        .arg(bench)
        .arg("--current-invocation")
        .arg(format!("{invocation}"))
        .arg("--current-build")
        .arg(build_name);
    if let Some(log_dir) = log_dir {
        cmd.arg("--output-csv").arg(log_dir.join("results.csv"));
    }
    if !run.profile.probes.is_empty() {
        let probes_json_str = serde_json::to_string(&run.profile.probes).unwrap();
        cmd.args(["--probes".to_owned(), probes_json_str]);
    }
    cmd
}
