#![feature(test)]

extern crate hdrsample;
extern crate rand;
extern crate test;

use hdrsample::*;
use hdrsample::serialization::*;
use self::rand::distributions::range::Range;
use self::rand::distributions::IndependentSample;
use self::test::Bencher;

#[bench]
fn serialize_small(b: &mut Bencher) {
    // 2048 counts
    do_serialize_bench(b, 1, 2047, 3)
}

#[bench]
fn serialize_medium(b: &mut Bencher) {
    // 56320 counts
    do_serialize_bench(b, 1, u64::max_value(), 3)
}

#[bench]
fn serialize_large(b: &mut Bencher) {
    // About 6 * 10^6 counts
    do_serialize_bench(b, 1, u64::max_value(), 5)
}

fn do_serialize_bench(b: &mut Bencher, low: u64, high: u64, digits: u8) {
    let mut s = V2Serializer::new();
    let mut vec = Vec::new();

    let range = Range::new(low, high);

    let mut h = Histogram::<u64>::new_with_bounds(low, high, digits).unwrap();

    let mut rng = rand::weak_rng();
    for _ in 0..100 {
        h.record(range.ind_sample(&mut rng)).unwrap();
    };

    b.iter(|| {
        vec.clear();

        let _ = s.serialize(&h, &mut vec).unwrap();
    });

    b.bytes = vec.len() as u64;
}
