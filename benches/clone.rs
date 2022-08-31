#![feature(test)]

extern crate test;

use hdrhistogram::*;
use test::Bencher;

#[bench]
fn clone(b: &mut Bencher) {
    let mut h = Histogram::<u32>::new_with_bounds(1, 100_000, 3).unwrap();
    for i in 0..100_000 {
        h.record(i).unwrap();
    }

    b.iter(|| h.clone())
}
