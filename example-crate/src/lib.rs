/// recursive fibonacci
#[cfg(feature = "rec")]
pub fn fib(n: usize) -> usize {
    if n == 0 || n == 1 {
        return n;
    }
    fib(n - 1) + fib(n - 2)
}

/// iterative fibonacci
#[cfg(not(feature = "rec"))]
pub fn fib(n: usize) -> usize {
    let mut a = 0;
    let mut b = 1;
    for _ in 0..n {
        let tmp = a;
        a = b;
        b = tmp + b;
    }
    a
}
