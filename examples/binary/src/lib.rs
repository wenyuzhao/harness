use std::{env, path::Path};

use harness::utils::exec;

fn compress_with_7z(file: &str, out: &str) {
    exec("7z", &["a", "-tzip", out, file]).unwrap();
}

fn compress_with_zip(file: &str, out: &str) {
    exec("zip", &[out, file]).unwrap();
}

pub fn compress(file: impl AsRef<Path>, out: impl AsRef<Path>) {
    let bin = env::var("BIN").expect("BIN env var not set");
    let file = file.as_ref().to_str().unwrap();
    let out = out.as_ref().to_str().unwrap();
    match bin.as_str() {
        "7z" => compress_with_7z(file, out),
        "zip" => compress_with_zip(file, out),
        _ => panic!("Unknown $BIN value: {}", bin),
    }
}
