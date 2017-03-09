extern crate rand;
extern crate test;

use super::v2_serializer::varint_write;
use super::deserializer::varint_read;
use std::io::Cursor;
use self::rand::Rng;
use self::test::Bencher;

#[bench]
fn varint_write_rand_1_k(b: &mut Bencher) {
    do_varint_write_rand(b, 1000)
}

#[bench]
fn varint_write_rand_1_m(b: &mut Bencher) {
    do_varint_write_rand(b, 1000_000)
}

#[bench]
fn varint_read_rand_1_k(b: &mut Bencher) {
    do_varint_read_rand(b, 1000)
}

#[bench]
fn varint_read_rand_1_m(b: &mut Bencher) {
    do_varint_read_rand(b, 1000_000)
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

fn do_varint_read_rand(b: &mut Bencher, num: usize) {
    let mut rng = rand::weak_rng();

    let mut vec = Vec::new();
    vec.resize(9 * num, 0);
    let mut bytes_written = 0;

    for _ in 0..num {
        bytes_written += varint_write(rng.gen(), &mut vec[bytes_written..]);
    }

    b.iter(|| {
        let mut cursor = Cursor::new(&vec);
        for _ in 0..num {
            let _ = varint_read(&mut cursor);
        }
    });
}
