use clap::Parser;
pub use harness_macros::entry;

#[derive(Parser, Debug)]
pub struct BenchArgs {
    #[arg(long, default_value = "false")]
    pub bench: bool,
    #[arg(short = 'n', long, default_value = "1")]
    /// Number of iterations to run
    pub iterations: usize,
    #[arg(long)]
    #[doc(hidden)]
    /// Overwrite benchmark name
    pub overwrite_benchmark_name: Option<String>,
    #[arg(long)]
    #[doc(hidden)]
    /// Overwrite crate name
    pub overwrite_crate_name: Option<String>,
}

pub fn black_box<T>(dummy: T) -> T {
    unsafe {
        let ret = std::ptr::read_volatile(&dummy);
        std::mem::forget(dummy);
        ret
    }
}

pub struct Bencher {
    name: String,
    prologue_fn: Option<Box<dyn FnMut()>>,
    iter_fn: Option<Box<dyn FnMut()>>,
    epilogue_fn: Option<Box<dyn FnMut()>>,
}

impl Bencher {
    #[doc(hidden)]
    pub fn new(fname: &'static str) -> Self {
        let fname = std::path::PathBuf::from(fname);
        let name = fname.file_stem().unwrap().to_str().unwrap().to_owned();
        Self {
            name,
            prologue_fn: None,
            iter_fn: None,
            epilogue_fn: None,
        }
    }

    pub fn prologue<T, F: 'static + FnMut() -> T>(&mut self, mut f: F) {
        self.prologue_fn = Some(Box::new(move || {
            f();
        }));
    }

    pub fn iter<T, F: 'static + FnMut() -> T>(&mut self, mut f: F) {
        self.iter_fn = Some(Box::new(move || {
            black_box(f());
        }));
    }

    pub fn epilogue<T, F: 'static + FnMut() -> T>(&mut self, mut f: F) {
        self.prologue_fn = Some(Box::new(move || {
            f();
        }));
    }

    #[doc(hidden)]
    pub fn run(&mut self) {
        let args = BenchArgs::parse();
        let name = if let Some(n) = args.overwrite_benchmark_name.as_ref() {
            n.clone()
        } else {
            self.name.clone()
        };
        let crate_name = if let Some(n) = args.overwrite_crate_name.as_ref() {
            n.clone()
        } else {
            "cargo-harness".to_owned()
        };
        for i in 0..args.iterations {
            let (start_label, end_label) = if i != args.iterations - 1 {
                (
                    format!("warmup {} ", i + 1),
                    format!("completed warmup {}", i + 1),
                )
            } else {
                ("".to_owned(), "PASSED".to_owned())
            };
            eprintln!(
                "===== {} {} starting {}=====",
                crate_name, name, start_label
            );
            if let Some(ref mut f) = self.prologue_fn {
                f();
            }
            let time = std::time::Instant::now();
            if let Some(ref mut f) = self.iter_fn {
                f();
            }
            let elapsed = time.elapsed().as_micros() as f64 / 1000.0;
            if let Some(ref mut f) = self.epilogue_fn {
                f();
            }
            eprintln!(
                "===== {} {} {} in {:.1} msec =====",
                crate_name, name, end_label, elapsed
            );
        }
    }
}
