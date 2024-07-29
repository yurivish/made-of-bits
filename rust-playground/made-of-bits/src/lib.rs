#![allow(unused)]

mod bitbuf;
mod bits;
mod bitvec;
mod intbuf;
mod js;
mod zorder;

use to_js::js;

#[js(name_prefix = "foo_")]
fn add(a: u32, b: u32) -> u32 {
    a + b
}

/// Implements a version of `std::panic::catch_unwind` that does not require unwind safety
/// for its closure argument. This allows us to test the panic behavior of our implementations
/// without requiring the trait to require `UnwindSafe`. Our testing always clones the `BitVec`
/// for use inside the closure, so there is no danger of observing corrupted internal state after
/// a panic occurs.
pub fn catch_unwind<F: FnOnce() -> R, R>(f: F) -> std::thread::Result<R> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
}

pub fn panics<F: FnOnce() -> R, R>(f: F) -> bool {
    catch_unwind(f).is_err()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = foo_add(2, 2);
        assert_eq!(result, 4);
    }
}
