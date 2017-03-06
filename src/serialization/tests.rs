extern crate rand;

use super::*;
use super::super::tests::helpers::histo64;
use self::rand::Rng;
use self::rand::distributions::range::Range;
use self::rand::distributions::IndependentSample;
use std::io::Cursor;

#[test]
fn encode_counts_all_zeros() {
    let h = histo64(1, u64::max_value(), 3);
    let counts_len = h.counts.len();
    let mut vec = vec![0; V2Serializer::counts_array_max_encoded_size(counts_len)];

    // because max is 0, it doesn't bother traversing the rest of the counts array

    let encoded_len = encode_counts(&h, &mut vec[..]).unwrap();
    assert_eq!(1, encoded_len);
    assert_eq!(0, vec[0]);

    let mut cursor = Cursor::new(vec);
    assert_eq!(0, zig_zag_decode(varint_read(&mut cursor).unwrap()));
}

#[test]
fn encode_counts_last_count_incremented() {
    let mut h = histo64(1, 2047, 3);
    let counts_len = h.counts.len();
    let mut vec = vec![0; V2Serializer::counts_array_max_encoded_size(counts_len)];

    assert_eq!(1, h.bucket_count);
    assert_eq!(2048, counts_len);

    // last in first (and only) bucket
    h.record(2047).unwrap();
    let encoded_len = encode_counts(&h, &mut vec[..]).unwrap();
    assert_eq!(3, encoded_len);

    let mut cursor = Cursor::new(vec);
    // 2047 zeroes. 2047 is 11 bits, so 2 7-byte chunks.
    assert_eq!(-2047, zig_zag_decode(varint_read(&mut cursor).unwrap()));
    assert_eq!(2, cursor.position());

    // then a 1
    assert_eq!(1, zig_zag_decode(varint_read(&mut cursor).unwrap()));
    assert_eq!(3, cursor.position());
}

#[test]
fn encode_counts_first_count_incremented() {
    let mut h = histo64(1, 2047, 3);
    let counts_len = h.counts.len();
    let mut vec = vec![0; V2Serializer::counts_array_max_encoded_size(counts_len)];

    assert_eq!(1, h.bucket_count);
    assert_eq!(2048, counts_len);

    // first position
    h.record(0).unwrap();
    let encoded_len = encode_counts(&h, &mut vec[..]).unwrap();

    assert_eq!(1, encoded_len);

    let mut cursor = Cursor::new(vec);
    // zero position has a 1
    assert_eq!(1, zig_zag_decode(varint_read(&mut cursor).unwrap()));
    assert_eq!(1, cursor.position());

    // max is 1, so rest isn't set
}

#[test]
fn encode_counts_first_and_last_count_incremented() {
    let mut h = histo64(1, 2047, 3);
    let counts_len = h.counts.len();
    let mut vec = vec![0; V2Serializer::counts_array_max_encoded_size(counts_len)];

    assert_eq!(1, h.bucket_count);
    assert_eq!(2048, counts_len);

    // first position
    h.record(0).unwrap();
    // last position in first (and only) bucket
    h.record(2047).unwrap();
    let encoded_len = encode_counts(&h, &mut vec[..]).unwrap();

    assert_eq!(4, encoded_len);

    let mut cursor = Cursor::new(vec);
    // zero position has a 1
    assert_eq!(1, zig_zag_decode(varint_read(&mut cursor).unwrap()));
    assert_eq!(1, cursor.position());

    // 2046 zeroes, then a 1.
    assert_eq!(-2046, zig_zag_decode(varint_read(&mut cursor).unwrap()));
    assert_eq!(3, cursor.position());

    assert_eq!(1, zig_zag_decode(varint_read(&mut cursor).unwrap()));
    assert_eq!(4, cursor.position());
}

#[test]
fn encode_counts_count_too_big() {
    let mut h = histo64(1, 2047, 3);
    let mut vec = vec![0; V2Serializer::counts_array_max_encoded_size(h.counts.len())];

    // first position
    h.record_n(0, i64::max_value() as u64 + 1).unwrap();
    assert_eq!(SerializeError::CountNotSerializable, encode_counts(&h, &mut vec[..]).unwrap_err());
}


#[test]
fn varint_write_3_bit_value() {
    let mut buf = [0; 9];
    let length = varint_write(6, &mut buf[..]);
    assert_eq!(1, length);
    assert_eq!(0x6, buf[0]);
}

#[test]
fn varint_write_7_bit_value() {
    let mut buf = [0; 9];
    let length = varint_write(127, &mut buf[..]);
    assert_eq!(1, length);
    assert_eq!(0x7F, buf[0]);
}

#[test]
fn varint_write_9_bit_value() {
    let mut buf = [0; 9];
    let length = varint_write(256, &mut buf[..]);
    assert_eq!(2, length);
    // marker high bit w/ 0's, then 9th bit (2nd bit of 2nd 7-bit group)
    assert_eq!(vec![0x80, 0x02].as_slice(), &buf[0..length]);
}

#[test]
fn varint_write_u64_max() {
    let mut buf = [0; 9];
    let length = varint_write(u64::max_value(), &mut buf[..]);
    assert_eq!(9, length);
    assert_eq!(vec![0xFF; 9].as_slice(), &buf[..]);
}

#[test]
fn varint_read_u64_max() {
    let input = &mut Cursor::new(vec![0xFF; 9]);
    assert_eq!(u64::max_value(), varint_read(input).unwrap());
}

#[test]
fn varint_read_u64_zero() {
    let input = &mut Cursor::new(vec![0x00; 9]);
    assert_eq!(0, varint_read(input).unwrap());
}

#[test]
fn varint_write_read_roundtrip_rand_1_byte() {
    do_varint_write_read_roundtrip_rand(1);
}

#[test]
fn varint_write_read_roundtrip_rand_2_byte() {
    do_varint_write_read_roundtrip_rand(2);
}

#[test]
fn varint_write_read_roundtrip_rand_3_byte() {
    do_varint_write_read_roundtrip_rand(3);
}

#[test]
fn varint_write_read_roundtrip_rand_4_byte() {
    do_varint_write_read_roundtrip_rand(4);
}

#[test]
fn varint_write_read_roundtrip_rand_5_byte() {
    do_varint_write_read_roundtrip_rand(5);
}

#[test]
fn varint_write_read_roundtrip_rand_6_byte() {
    do_varint_write_read_roundtrip_rand(6);
}

#[test]
fn varint_write_read_roundtrip_rand_7_byte() {
    do_varint_write_read_roundtrip_rand(7);
}

#[test]
fn varint_write_read_roundtrip_rand_8_byte() {
    do_varint_write_read_roundtrip_rand(8);
}

#[test]
fn varint_write_read_roundtrip_rand_9_byte() {
    do_varint_write_read_roundtrip_rand(9);
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

fn do_varint_write_read_roundtrip_rand(length: usize) {
    let range = Range::new(1 << ((length - 1) * 7), 1 << (length * 7));
    let mut rng = rand::weak_rng();
    let mut buf = [0; 9];
    for _ in 1..1_000_000 {
        for i in 0..(buf.len()) {
            buf[i] = 0;
        };
        let r: u64 = range.ind_sample(&mut rng);
        let bytes_written = varint_write(r, &mut buf);
        assert_eq!(length, bytes_written);
        assert_eq!(r, varint_read(&mut &buf[..bytes_written]).unwrap());

        // make sure the other bytes are all still 0
        assert_eq!(vec![0; 9 - bytes_written], &buf[bytes_written..]);
    };
}
