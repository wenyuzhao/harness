pub trait Benchmark: Default {
    fn prologue(&mut self) {}
    fn iter(&mut self) {}
    fn epilogue(&mut self) {}
}
