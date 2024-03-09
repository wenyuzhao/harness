use harness::utils::{download_file, exec, get_cached_file, HARNESS_BENCH_SCRATCH_DIR};
use harness::{bench, Bencher};

const NAME: &'static str = "enwiki-20240101-pages-articles-multistream16.xml-p20460153p20570392";

fn startup() {
    let file = get_cached_file(NAME)
        .or_else(|| {
            let url = format!("https://dumps.wikimedia.org/enwiki/20240101/{NAME}.bz2");
            println!("Downloading file: {url}");
            let bz2 = download_file(format!("{NAME}.bz2"), url).unwrap();
            println!("Decompressing file: {}", bz2.display());
            exec("bzip2", &["-dk", bz2.as_os_str().to_str().unwrap()]).unwrap();
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
