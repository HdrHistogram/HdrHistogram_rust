extern crate rand;

use super::*;
use self::rand::Rng;
use std::io::Cursor;

#[test]
fn varint_write_3_bit_value() {
    let buf = &mut Cursor::new(Vec::<u8>::new());
    assert_eq!(1, super::varint_write(6, buf).unwrap());

    let vec = buf.get_ref();
    assert_eq!(1, vec.len());
    assert_eq!(0x6, vec[0]);
}

#[test]
fn varint_write_7_bit_value() {
    let buf = &mut Cursor::new(Vec::<u8>::new());
    assert_eq!(1, super::varint_write(127, buf).unwrap());

    let vec = buf.get_ref();
    assert_eq!(1, vec.len());
    assert_eq!(0x7F, vec[0]);
}

#[test]
fn varint_write_9_bit_value() {
    let buf = &mut Cursor::new(Vec::<u8>::new());
    assert_eq!(2, super::varint_write(256, buf).unwrap());

    // marker high bit w/ 0's, then 9th bit (2nd bit of 2nd 7-bit group)
    assert_eq!(&vec![0x80, 0x02], buf.get_ref());
}

#[test]
fn varint_write_u64_max() {
    let buf = &mut Cursor::new(Vec::<u8>::new());
    assert_eq!(9, super::varint_write(u64::max_value(), buf).unwrap());

    assert_eq!(&vec![0xFF; 9], buf.get_ref());
}

#[test]
fn varint_read_u64_max() {
    let input = &mut Cursor::new(vec![0xFF; 9]);
    assert_eq!(u64::max_value(), super::varint_read(input).unwrap());
}

#[test]
fn varint_read_u64_zero() {
    let input = &mut Cursor::new(vec![0x00; 9]);
    assert_eq!(0, super::varint_read(input).unwrap());
}

#[test]
fn varint_write_read_roundtrip_rand() {
    let mut rng = rand::weak_rng();
    let mut vec = Vec::<u8>::new();
    vec.reserve(9);
    for _ in 1..1_000_000 {
        vec.clear();
        let int: u64 = rng.gen();
        let bytes_written = super::varint_write(int, &mut vec).unwrap();
        assert_eq!(vec.len(), bytes_written);
        assert_eq!(int, super::varint_read(&mut vec.as_slice()).unwrap());
    }
}

#[test]
fn zig_zag_encode_0() {
    assert_eq!(0, zig_zag_encode(0));
}

#[test]
fn zig_zag_encode_neg_1() {
    assert_eq!(1, zig_zag_encode(-1));
}

#[test]
fn zig_zag_encode_1() {
    assert_eq!(2, zig_zag_encode(1));
}

#[test]
fn zig_zag_encode_i64_max() {
    assert_eq!(u64::max_value() - 1, zig_zag_encode(i64::max_value()));
}

#[test]
fn zig_zag_encode_i64_min() {
    assert_eq!(u64::max_value(), zig_zag_encode(i64::min_value()));
}

#[test]
fn zig_zag_decode_i64_min() {
    assert_eq!(i64::min_value(), zig_zag_decode(u64::max_value()))
}

#[test]
fn zig_zag_decode_i64_max() {
    assert_eq!(i64::max_value(), zig_zag_decode(u64::max_value() - 1))
}

#[test]
fn zig_zag_roundtrip_random() {
    let mut rng = rand::weak_rng();

    for _ in 0..1_000_000 {
        let r: i64 = rng.gen();
        let encoded = zig_zag_encode(r);
        let decoded = zig_zag_decode(encoded);

        assert_eq!(r, decoded);
    }
}
