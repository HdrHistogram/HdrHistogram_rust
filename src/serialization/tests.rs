extern crate rand;

use super::*;
use self::rand::Rng;

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

    for _ in 0..1_000_000{
        let r = rng.gen::<i64>();
        let encoded = zig_zag_encode(r);
        let decoded = zig_zag_decode(encoded);

        assert_eq!(r, decoded);
    }
}
