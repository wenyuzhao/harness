#[cfg(not(feature = "unstable"))]
pub fn sort<T: Ord>(slice: &mut [T]) {
    slice.sort();
}

#[cfg(feature = "unstable")]
pub fn sort<T: Ord>(slice: &mut [T]) {
    slice.sort_unstable();
}

pub fn is_sorted<T: Ord>(slice: &[T]) -> bool {
    slice.windows(2).all(|w| w[0] <= w[1])
}
