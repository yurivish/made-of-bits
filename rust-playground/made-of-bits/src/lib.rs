#![allow(unused)]

mod bitbuf;
mod bits;
mod bitvec;
#[cfg(test)]
mod bitvec_test;
mod intbuf;
mod sortedarraybitvec;
mod sortedarraybitvec_js;

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
