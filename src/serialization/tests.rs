extern crate rand;

use super::{V2_COOKIE, V2_HEADER_SIZE, V2Serializer, V2SerializeError, V2DeflateSerializer, V2DeflateSerializeError};
use super::v2_serializer::{counts_array_max_encoded_size, encode_counts, varint_write, zig_zag_encode};
use super::deserializer::{Deserializer, varint_read, varint_read_slice, zig_zag_decode};
use super::byteorder::{BigEndian, ReadBytesExt};
use super::super::{Counter, Histogram};
use super::super::num::traits::{Saturating, ToPrimitive};
use super::super::tests::helpers::histo64;
use std::io::{Cursor, Write};
use std::fmt::{Debug, Display};
use std::iter::once;
use self::rand::{Rand, Rng};
use self::rand::distributions::range::{Range, SampleRange};
use self::rand::distributions::IndependentSample;

#[test]
fn serialize_all_zeros() {
    let h = histo64(1, 2047, 3);
    let mut s = V2Serializer::new();
    let mut vec = Vec::new();

    let bytes_written = s.serialize(&h, &mut vec).unwrap();
    assert_eq!(V2_HEADER_SIZE + 1, bytes_written);

    let mut reader = vec.as_slice();

    assert_eq!(V2_COOKIE, reader.read_u32::<BigEndian>().unwrap());
    // payload length
    assert_eq!(1, reader.read_u32::<BigEndian>().unwrap());
    // normalizing offset
    assert_eq!(0, reader.read_u32::<BigEndian>().unwrap());
    // num digits
    assert_eq!(3, reader.read_u32::<BigEndian>().unwrap());
    // lowest
    assert_eq!(1, reader.read_u64::<BigEndian>().unwrap());
    // highest
    assert_eq!(2047, reader.read_u64::<BigEndian>().unwrap());
    // conversion ratio
    assert_eq!(1.0, reader.read_f64::<BigEndian>().unwrap());
}

#[test]
fn serialize_roundtrip_all_zeros() {
    let orig = histo64(1, 2047, 3);
    let mut s = V2Serializer::new();
    let mut vec = Vec::new();

    let bytes_written = s.serialize(&orig, &mut vec).unwrap();
    assert_eq!(V2_HEADER_SIZE + 1, bytes_written);

    let mut d = Deserializer::new();

    let mut cursor = Cursor::new(vec);
    let deser: Histogram<u64> = d.deserialize(&mut cursor).unwrap();

    assert_eq!(orig.highest_trackable_value, deser.highest_trackable_value);
    assert_eq!(orig.lowest_discernible_value, deser.lowest_discernible_value);
    assert_eq!(orig.significant_value_digits, deser.significant_value_digits);
    assert_eq!(orig.bucket_count, deser.bucket_count);
    assert_eq!(orig.sub_bucket_count, deser.sub_bucket_count);
    assert_eq!(orig.sub_bucket_half_count, deser.sub_bucket_half_count);
    assert_eq!(orig.sub_bucket_half_count_magnitude, deser.sub_bucket_half_count_magnitude);
    assert_eq!(orig.sub_bucket_mask, deser.sub_bucket_mask);
    assert_eq!(orig.leading_zero_count_base, deser.leading_zero_count_base);
    assert_eq!(orig.unit_magnitude, deser.unit_magnitude);
    assert_eq!(orig.unit_magnitude_mask, deser.unit_magnitude_mask);
    assert_eq!(orig.highest_equivalent(orig.max_value), deser.max_value);

    // never saw a value, so min never gets set
    assert_eq!(u64::max_value(), deser.min_non_zero_value);

    assert_eq!(orig.total_count, deser.total_count);
    assert_eq!(orig.counts, deser.counts);
}

#[test]
fn serialize_roundtrip_1_count_for_every_value_1_bucket() {
    let mut h = histo64(1, 2047, 3);
    assert_eq!(1, h.bucket_count);
    assert_eq!(10, h.sub_bucket_half_count_magnitude);


    for v in 0..2048 {
        h.record(v).unwrap();
    }

    let mut s = V2Serializer::new();
    let mut vec = Vec::new();

    let bytes_written = s.serialize(&h, &mut vec).unwrap();
    assert_eq!(V2_HEADER_SIZE + 2048, bytes_written);

    let mut d = Deserializer::new();

    let mut cursor = Cursor::new(vec);
    let h2: Histogram<u64> = d.deserialize(&mut cursor).unwrap();

    assert_deserialized_histogram_matches_orig(h, h2);
}

#[test]
fn serialize_roundtrip_1_count_for_every_value_2_buckets() {
    let mut h = histo64(1, 4095, 3);
    assert_eq!(2, h.bucket_count);
    assert_eq!(2048 + 1024, h.counts.len());

    for v in 0..4095 {
        h.record(v).unwrap();
    }

    let mut s = V2Serializer::new();
    let mut vec = Vec::new();

    let bytes_written = s.serialize(&h, &mut vec).unwrap();
    assert_eq!(V2_HEADER_SIZE + 2048 + 1024, bytes_written);

    let mut d = Deserializer::new();

    let mut cursor = Cursor::new(vec);
    let h2: Histogram<u64> = d.deserialize(&mut cursor).unwrap();

    assert_deserialized_histogram_matches_orig(h, h2);
}

#[test]
fn serialize_roundtrip_random_v2_u64() {
    do_serialize_roundtrip_random(V2Serializer::new(), i64::max_value() as u64);
}

#[test]
fn serialize_roundtrip_random_v2_u32() {
    do_serialize_roundtrip_random(V2Serializer::new(), u32::max_value());
}

#[test]
fn serialize_roundtrip_random_v2_u16() {
    do_serialize_roundtrip_random(V2Serializer::new(), u16::max_value());
}

#[test]
fn serialize_roundtrip_random_v2_u8() {
    do_serialize_roundtrip_random(V2Serializer::new(), u8::max_value());
}

#[test]
fn serialize_roundtrip_random_v2_deflate_u64() {
    do_serialize_roundtrip_random(V2DeflateSerializer::new(), i64::max_value() as u64);
}

#[test]
fn serialize_roundtrip_random_v2_deflate_u32() {
    do_serialize_roundtrip_random(V2DeflateSerializer::new(), u32::max_value());
}

#[test]
fn serialize_roundtrip_random_v2_deflate_u16() {
    do_serialize_roundtrip_random(V2DeflateSerializer::new(), u16::max_value());
}

#[test]
fn serialize_roundtrip_random_v2_deflate_u8() {
    do_serialize_roundtrip_random(V2DeflateSerializer::new(), u8::max_value());
}

#[test]
fn encode_counts_all_zeros() {
    let h = histo64(1, u64::max_value(), 3);
    let counts_len = h.counts.len();
    let mut vec = vec![0; counts_array_max_encoded_size(counts_len).unwrap()];

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
    let mut vec = vec![0; counts_array_max_encoded_size(counts_len).unwrap()];

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
    let mut vec = vec![0; counts_array_max_encoded_size(counts_len).unwrap()];

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
    let mut vec = vec![0; counts_array_max_encoded_size(counts_len).unwrap()];

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
    let mut vec = vec![0; counts_array_max_encoded_size(h.counts.len()).unwrap()];

    // first position
    h.record_n(0, i64::max_value() as u64 + 1).unwrap();
    assert_eq!(V2SerializeError::CountNotSerializable, encode_counts(&h, &mut vec[..]).unwrap_err());
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
fn varint_write_read_slice_roundtrip_rand_1_byte() {
    do_varint_write_read_slice_roundtrip_rand(1);
}

#[test]
fn varint_write_read_slice_roundtrip_rand_2_byte() {
    do_varint_write_read_slice_roundtrip_rand(2);
}

#[test]
fn varint_write_read_slice_roundtrip_rand_3_byte() {
    do_varint_write_read_slice_roundtrip_rand(3);
}

#[test]
fn varint_write_read_slice_roundtrip_rand_4_byte() {
    do_varint_write_read_slice_roundtrip_rand(4);
}

#[test]
fn varint_write_read_slice_roundtrip_rand_5_byte() {
    do_varint_write_read_slice_roundtrip_rand(5);
}

#[test]
fn varint_write_read_slice_roundtrip_rand_6_byte() {
    do_varint_write_read_slice_roundtrip_rand(6);
}

#[test]
fn varint_write_read_slice_roundtrip_rand_7_byte() {
    do_varint_write_read_slice_roundtrip_rand(7);
}

#[test]
fn varint_write_read_slice_roundtrip_rand_8_byte() {
    do_varint_write_read_slice_roundtrip_rand(8);
}

#[test]
fn varint_write_read_slice_roundtrip_rand_9_byte() {
    do_varint_write_read_slice_roundtrip_rand(9);
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
fn zig_zag_decode_0() {
    assert_eq!(0, zig_zag_decode(0));
}

#[test]
fn zig_zag_decode_1() {
    assert_eq!(-1, zig_zag_decode(1));
}

#[test]
fn zig_zag_decode_2() {
    assert_eq!(1, zig_zag_decode(2));
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
fn zig_zag_decode_u64_max_to_i64_min() {
    assert_eq!(i64::min_value(), zig_zag_decode(u64::max_value()))
}

#[test]
fn zig_zag_decode_u64_max_penultimate_to_i64_max() {
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

// Test that varint test helpers are correct

#[test]
fn largest_number_in_7_bit_chunk_correct() {
    // 8 chunks (indices 0-7) of 7 bits gets you to 56 bits. Last byte in varint is handled
    // differently, so we don't test that here.
    for i in 0..8 {
        let largest = largest_number_in_7_bit_chunk(i);
        assert_eq!((i as u32 + 1) * 7, largest.count_ones());

        assert_eq!(64 - ((i as u32) + 1) * 7, largest.leading_zeros());
        // any larger and it will be in the next chunk
        assert_eq!(largest.leading_zeros() - 1, (largest + 1).leading_zeros());
    };
}

fn do_varint_write_read_roundtrip_rand(byte_length: usize) {
    assert!(byte_length <= 9 && byte_length >= 1);

    let smallest_in_range = smallest_number_in_n_byte_varint(byte_length);
    let largest_in_range = largest_number_in_n_byte_varint(byte_length);

    let mut buf = [0; 9];
    // Bunch of random numbers, plus the start and end of the range
    let range = Range::new(smallest_in_range, largest_in_range);
    for i in RandomRangeIter::new(rand::weak_rng(), range).take(100_000)
        .chain(once(smallest_in_range))
        .chain(once(largest_in_range)) {
        for i in 0..(buf.len()) {
            buf[i] = 0;
        };
        let bytes_written = varint_write(i, &mut buf);
        assert_eq!(byte_length, bytes_written);
        assert_eq!(i, varint_read(&mut &buf[..bytes_written]).unwrap());

        // make sure the other bytes are all still 0
        assert_eq!(vec![0; 9 - bytes_written], &buf[bytes_written..]);
    };
}

fn do_varint_write_read_slice_roundtrip_rand(byte_length: usize) {
    assert!(byte_length <= 9 && byte_length >= 1);

    let smallest_in_range = smallest_number_in_n_byte_varint(byte_length);
    let largest_in_range = largest_number_in_n_byte_varint(byte_length);

    let mut buf = [0; 9];

    // Bunch of random numbers, plus the start and end of the range
    let range = Range::new(smallest_in_range, largest_in_range);
    for i in RandomRangeIter::new(rand::weak_rng(), range).take(100_000)
        .chain(once(smallest_in_range))
        .chain(once(largest_in_range)) {
        for i in 0..(buf.len()) {
            buf[i] = 0;
        };
        let bytes_written = varint_write(i, &mut buf);
        assert_eq!(byte_length, bytes_written);
        assert_eq!((i, bytes_written), varint_read_slice(&mut &buf[..bytes_written]));

        // make sure the other bytes are all still 0
        assert_eq!(vec![0; 9 - bytes_written], &buf[bytes_written..]);
    }
}

fn do_serialize_roundtrip_random<S, T>(mut serializer: S, max_count: T)
    where S: TestOnlyHypotheticalSerializerInterface,
          T: Counter + Debug + Display + Rand + Saturating + ToPrimitive + SampleRange {
    let mut d = Deserializer::new();
    let mut vec = Vec::new();
    let mut count_rng = rand::weak_rng();

    let range = Range::<T>::new(T::one(), max_count);
    for _ in 0..100 {
        vec.clear();
        let mut h = Histogram::<T>::new_with_bounds(1, u64::max_value(), 3).unwrap();

        for value in RandomVarintEncodedLengthIter::new(rand::weak_rng()).take(1000) {
            let count = range.ind_sample(&mut count_rng);
            // don't let accumulated per-value count exceed max_count
            let existing_count = h.count_at(value).unwrap();
            let sum = existing_count.saturating_add(count);
            if sum >= max_count {
                // cap it to max count
                h.record_n(value, max_count - existing_count).unwrap();
            } else {
                // sum won't exceed max_count
                h.record_n(value, count).unwrap();
            }
        }

        let bytes_written = serializer.serialize(&h, &mut vec).unwrap();
        assert_eq!(bytes_written, vec.len());

        let mut cursor = Cursor::new(&vec);
        let h2: Histogram<T> = d.deserialize(&mut cursor).unwrap();

        assert_deserialized_histogram_matches_orig(h, h2);
    }
}

fn assert_deserialized_histogram_matches_orig<T: Counter + Debug>(orig: Histogram<T>, deser: Histogram<T>) {
    assert_eq!(orig.highest_trackable_value, deser.highest_trackable_value);
    assert_eq!(orig.lowest_discernible_value, deser.lowest_discernible_value);
    assert_eq!(orig.significant_value_digits, deser.significant_value_digits);
    assert_eq!(orig.bucket_count, deser.bucket_count);
    assert_eq!(orig.sub_bucket_count, deser.sub_bucket_count);
    assert_eq!(orig.sub_bucket_half_count, deser.sub_bucket_half_count);
    assert_eq!(orig.sub_bucket_half_count_magnitude, deser.sub_bucket_half_count_magnitude);
    assert_eq!(orig.sub_bucket_mask, deser.sub_bucket_mask);
    assert_eq!(orig.leading_zero_count_base, deser.leading_zero_count_base);
    assert_eq!(orig.unit_magnitude, deser.unit_magnitude);
    assert_eq!(orig.unit_magnitude_mask, deser.unit_magnitude_mask);

    // in buckets past the first, can only match up to precision in that bucket
    assert_eq!(orig.highest_equivalent(orig.max_value), deser.max_value);
    assert_eq!(orig.lowest_equivalent(orig.min_non_zero_value), deser.min_non_zero_value);

    assert_eq!(orig.counts, deser.counts);

    // total counts will not equal if any individual count has saturated at a point where that did
    // *not* saturate the total count: the deserialized one will have missed the lost increments.
    assert!(orig.total_count >= deser.total_count);
    assert_eq!(deser.total_count,
    deser.counts.iter().fold(0_u64, |acc, &i| acc.saturating_add(i.as_u64())));
}

/// Smallest number in our varint encoding that takes the given number of bytes
fn smallest_number_in_n_byte_varint(byte_length: usize) -> u64 {
    assert!(byte_length <= 9 && byte_length >= 1);

    match byte_length {
        1 => 0,
        // one greater than the largest of the previous length
        _ => largest_number_in_n_byte_varint(byte_length - 1) + 1
    }
}

/// Largest number in our varint encoding that takes the given number of bytes
fn largest_number_in_n_byte_varint(byte_length: usize) -> u64 {
    assert!(byte_length <= 9 && byte_length >= 1);

    match byte_length {
        9 => u64::max_value(),
        _ => largest_number_in_7_bit_chunk(byte_length - 1)
    }
}

/// The largest in the set of numbers that have at least 1 bit set in the n'th chunk of 7 bits.
fn largest_number_in_7_bit_chunk(chunk_index: usize) -> u64 {
    // Our 9-byte varints do different encoding in the last byte, so we don't handle them here
    assert!(chunk_index <= 7);

    // 1 in every bit below the lowest bit in this chunk
    let lower_bits = match chunk_index {
        0 => 0,
        _ => largest_number_in_7_bit_chunk(chunk_index - 1)
    };

    // 1 in every bit in this chunk
    let this_chunk = 0x7F_u64 << (chunk_index * 7);

    lower_bits | this_chunk
}


struct RandomRangeIter<T: SampleRange, R: Rng> {
    range: Range<T>,
    rng: R
}

impl<T: SampleRange, R: Rng> RandomRangeIter<T, R> {
    fn new(rng: R, range: Range<T>) -> RandomRangeIter<T, R> {
        RandomRangeIter {
            rng: rng,
            range: range
        }
    }
}

impl<T: SampleRange, R: Rng> Iterator for RandomRangeIter<T, R> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.range.ind_sample(&mut self.rng))
    }
}

// Evenly distributed random numbers end up biased heavily towards longer encoded byte lengths:
// there are a lot more large numbers than there are small (duh), but for exercising serialization
// code paths, we'd like many at all byte lengths. This is also arguably more representative of
// real data. This should emit values whose varint lengths are uniformly distributed across the
// whole length range (1 to 9).
struct RandomVarintEncodedLengthIter<R: Rng> {
    ranges: [Range<u64>; 9],
    range_for_picking_range: Range<usize>,
    rng: R
}

impl<R: Rng> RandomVarintEncodedLengthIter<R> {
    fn new(rng: R) -> RandomVarintEncodedLengthIter<R> {
        RandomVarintEncodedLengthIter {
            ranges: [
                Range::new(smallest_number_in_n_byte_varint(1), largest_number_in_n_byte_varint(1) + 1),
                Range::new(smallest_number_in_n_byte_varint(2), largest_number_in_n_byte_varint(2) + 1),
                Range::new(smallest_number_in_n_byte_varint(3), largest_number_in_n_byte_varint(3) + 1),
                Range::new(smallest_number_in_n_byte_varint(4), largest_number_in_n_byte_varint(4) + 1),
                Range::new(smallest_number_in_n_byte_varint(5), largest_number_in_n_byte_varint(5) + 1),
                Range::new(smallest_number_in_n_byte_varint(6), largest_number_in_n_byte_varint(6) + 1),
                Range::new(smallest_number_in_n_byte_varint(7), largest_number_in_n_byte_varint(7) + 1),
                Range::new(smallest_number_in_n_byte_varint(8), largest_number_in_n_byte_varint(8) + 1),
                Range::new(smallest_number_in_n_byte_varint(9), largest_number_in_n_byte_varint(9)),
            ],
            range_for_picking_range: Range::new(0, 9),
            rng: rng
        }
    }
}

impl<R: Rng> Iterator for RandomVarintEncodedLengthIter<R> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        // pick the range we'll use
        let value_range = self.ranges[self.range_for_picking_range.ind_sample(&mut self.rng)];

        Some(value_range.ind_sample(&mut self.rng))
    }
}

include!("test_serialize_trait.rs");
