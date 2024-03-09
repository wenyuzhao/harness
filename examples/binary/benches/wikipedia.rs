use harness::utils::{download_file, exec, get_cached_file, HARNESS_BENCH_SCRATCH_DIR};
use harness::{bench, Bencher};

const NAME: &'static str = "enwiki-20240101-pages-articles-multistream24.xml-p56564554p57025655";

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
        binary::compress(in_file, out_file);
    });
}
