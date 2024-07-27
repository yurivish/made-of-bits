#![allow(unused)]

mod bitbuf;
mod bits;
mod bitvec;
#[cfg(test)]
mod bitvec_test;
mod bitvecs;
mod intbuf;

use to_js::js;

#[js(prefix = "foo_")]
fn add(a: u32, b: u32) -> u32 {
    a + b
}

/// Implements a version of `std::panic::catch_unwind` that does not require unwind safety
/// for its closure argument. This allows us to test the panic behavior of `BitVec` implementations
/// without requiring the trait to require `UnwindSafe`. Our testing always clones the `BitVec`
/// for use inside the closure, so there is no danger of observing corrupted internal state after
/// a panic occurs.
pub fn catch_unwind<F: FnOnce() -> R, R>(f: F) -> std::thread::Result<R> {
    // Register a do-nothing panic hook to prevent intended panics from printing stack traces.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    std::panic::set_hook(prev_hook);
    result
}

mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = foo_add(2, 2);
        assert_eq!(result, 4);
    }
}
