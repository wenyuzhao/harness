use harness::utils::{download_file, get_cached_file, HARNESS_BENCH_SCRATCH_DIR};
use harness::{bench, Bencher};

const NAME: &'static str = "file-examples-com-wav-fe7590659365eb907974df6.wav";

fn startup() {
    let file = get_cached_file(NAME)
        .or_else(|| {
            let url = format!(
                "https://file-examples.com/storage/fe7590659365eb907974df6/2017/11/file_example_WAV_10MG.wav"
            );
            println!("Downloading file: {url}");
            download_file(NAME, url).unwrap();
            get_cached_file(NAME)
        })
        .unwrap();
    println!("Downloaded file: {}", file.display());
}

#[bench(oneshot, startup=startup)]
fn bar(bencher: &Bencher) {
    let in_file = get_cached_file(NAME).unwrap();
    let out_file = HARNESS_BENCH_SCRATCH_DIR.join("out.zip");
    assert!(!out_file.exists());
    bencher.time(|| {
        binary::compress(&in_file, &out_file);
    });
    // Get compressed size
    let original_size = std::fs::metadata(&in_file).unwrap().len();
    let compressed_size = std::fs::metadata(&out_file).unwrap().len();
    bencher.add_stat("original-size", original_size);
    bencher.add_stat("compressed-size", compressed_size);
    bencher.add_stat(
        "compression-ratio",
        compressed_size as f64 / original_size as f64,
    );
}
