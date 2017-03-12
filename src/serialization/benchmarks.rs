extern crate rand;
extern crate test;

use super::v2_serializer::varint_write;
use super::deserializer::varint_read;
use std::io::Cursor;
use self::rand::distributions::range::Range;
use self::rand::distributions::IndependentSample;
use self::test::Bencher;

#[bench]
fn varint_write_rand(b: &mut Bencher) {
    do_varint_write_rand(b, 1000_000, Range::new(0, u64::max_value()))
}

#[bench]
fn varint_write_rand_1_byte(b: &mut Bencher) {
    do_varint_write_rand(b, 1000_000, Range::new(0, 128))
}

#[bench]
fn varint_write_rand_9_bytes(b: &mut Bencher) {
    do_varint_write_rand(b, 1000_000, Range::new(1 << 56, u64::max_value()))
}

#[bench]
fn varint_read_rand(b: &mut Bencher) {
    do_varint_read_rand(b, 1000_000, Range::new(0, u64::max_value()))
}

#[bench]
fn varint_read_rand_1_byte(b: &mut Bencher) {
    do_varint_read_rand(b, 1000_000, Range::new(0, 128))
}

#[bench]
fn varint_read_rand_9_byte(b: &mut Bencher) {
    do_varint_read_rand(b, 1000_000, Range::new(1 << 56, u64::max_value()))
}

fn do_varint_write_rand(b: &mut Bencher, num: usize, range: Range<u64>) {
    let mut rng = rand::weak_rng();

    let mut vec: Vec<u64> = Vec::new();

    for _ in 0..num {
        vec.push(range.ind_sample(&mut rng));
    }

    let mut buf = [0; 9];
    b.iter(|| {
        for i in vec.iter() {
            let _ = varint_write(*i, &mut buf);
        }
    });
}

fn do_varint_read_rand(b: &mut Bencher, num: usize, range: Range<u64>) {
    let mut rng = rand::weak_rng();

    let mut vec = Vec::new();
    vec.resize(9 * num, 0);
    let mut bytes_written = 0;

    for _ in 0..num {
        bytes_written += varint_write(range.ind_sample(&mut rng), &mut vec[bytes_written..]);
    }

    b.iter(|| {
        let mut cursor = Cursor::new(&vec);
        for _ in 0..num {
            let _ = varint_read(&mut cursor);
        }
    });
}
