#![allow(unused)]

mod bitbuf;
mod bits;
mod bitvec;
#[cfg(test)]
mod bitvec_test;
mod bitvecs;
mod intbuf;

use to_js::js;

#[js]
fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
