extern crate rand;
extern crate test;

use super::*;
use self::rand::Rng;
use self::test::Bencher;

#[bench]
fn varint_write_rand_1000(b: &mut Bencher) {
    do_varint_write_rand(b, 1000)
}

#[bench]
fn varint_write_rand_1000_000(b: &mut Bencher) {
    do_varint_write_rand(b, 1000_000)
}

fn do_varint_write_rand(b: &mut Bencher, num: usize) {
    let mut rng = rand::weak_rng();

    let mut vec: Vec<u64> = Vec::new();

    for _ in 0..num {
        vec.push(rng.gen());
    }

    let mut buf = [0; 9];
    b.iter(|| {
        for i in vec.iter() {
            let _ = varint_write(*i, &mut buf);
        }
    });
}
