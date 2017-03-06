extern crate byteorder;

use super::{Counter, Histogram};
use std::io::{self, Read, Write};
use self::byteorder::{BigEndian, WriteBytesExt};

#[path = "tests.rs"]
#[cfg(test)]
mod tests;

const V2_COOKIE_BASE: u32 = 0x1c849303;

const V2_COOKIE: u32 = V2_COOKIE_BASE | 0x10;

const V2_HEADER_SIZE: usize = 40;

/// Serializer for the V2 format.
pub struct V2Serializer {
    buf: Vec<u8>
}

impl V2Serializer {

    /// Serialize the histogram.
    /// Returns the number of bytes written, or an io error.
    pub fn serialize<T: Counter, W: Write>(&mut self, h: &Histogram<T>, writer: &mut W) -> io::Result<usize> {
        self.buf.clear();
        let max_size = Self::max_encoded_size(h);
        self.buf.reserve(max_size);

        self.buf.write_u32::<BigEndian>(V2_COOKIE)?;
        // placeholder for length
        self.buf.write_u32::<BigEndian>(0)?;
        // normalizing index offset
        self.buf.write_u32::<BigEndian>(0)?;
        self.buf.write_u32::<BigEndian>(h.significant_value_digits as u32)?;
        self.buf.write_u64::<BigEndian>(h.lowest_discernible_value)?;
        self.buf.write_u64::<BigEndian>(h.highest_trackable_value)?;
        // int to double conversion
        self.buf.write_f64::<BigEndian>(1.0)?;

        debug_assert_eq!(V2_HEADER_SIZE, self.buf.len());

        unsafe {
            self.buf.set_len(max_size);
        }

        let counts_len = encode_counts(h, &mut self.buf[V2_HEADER_SIZE..]);
        let total_len = V2_HEADER_SIZE + counts_len;

        writer.write_all(&mut self.buf[0..(total_len)]).map(|_| total_len)
    }

    fn max_encoded_size<T: Counter>(h: &Histogram<T>) -> usize {
        Self::counts_array_max_encoded_size(h.counts.len()) + V2_HEADER_SIZE
    }

    fn counts_array_max_encoded_size(length: usize) -> usize{
        // LEB128-64b9B uses at most 9 bytes
        // Won't overflow (except sometimes on 16 bit systems) because largest possible counts
        // len is 47 buckets, each with 2^17 half count, for a total of 6e6. This product will
        // therefore be about 5e7 (50 million) at most.
        length * 9
    }
}

/// Encode counts array into slice.
fn encode_counts<T: Counter>(h: &Histogram<T>, buf: &mut [u8]) -> usize {
    let index_limit = h.index_for(h.max()) + 1;
    let mut index = 0;
    let mut bytes_written = 0;

    while index < index_limit {
        let count = h.count_at_index(index);
        index += 1;

        // Non-negative values are positive counts or 1 zero, negative values are repeated zeros.

        let mut zero_count = 0;
        if count == T::zero() {
            zero_count = 1;

            while (index < index_limit) && (h.count_at_index(index) == T::zero()) {
                zero_count += 1;
                index += 1;
            }
        }

        let count_or_zeros: i64 = if zero_count > 1 {
            // zero count can be at most the entire counts array, which is at most 6e6, so will fit.
            -zero_count
        } else {
            // TODO handle count exceeding i64::max_value()
            count.to_i64().unwrap()
        };

        let zz = zig_zag_encode(count_or_zeros);

        bytes_written += varint_write(zz, &mut buf[bytes_written..]);
    }

    bytes_written
}

/// Write a number as a LEB128-64b9B little endian base 128 varint to buf. This is not
/// quite the same as Protobuf's LEB128 as it encodes 64 bit values in a max of 9 bytes, not 10.
/// The first 8 7-bit chunks are encoded normally (up through the first 7 bytes of input). The last
/// byte is added to the buf as-is. This limits the input to 8 bytes, but that's all we need.
fn varint_write(input: u64, buf: &mut [u8]) -> usize {
    // The loop is unrolled because the special case is awkward to express in a loop, and it
    // probably makes the branch predictor happier to do it this way.

    if (input >> 7) == 0 {
        buf[0] = input as u8;
        return 1;
    } else {
        // set high bit because more bytes are coming, then next 7 bits of value.
        buf[0] = 0x80 | ((input & 0x7F) as u8);
        if (input >> 7 * 2) == 0 {
            // nothing above bottom 2 chunks, this is the last byte, so no high bit
            buf[1] = nth_7b_chunk(input, 1);
            return 2;
        } else {
            buf[1] = nth_7b_chunk_with_high_bit(input, 1);
            if (input >> 7 * 3) == 0 {
                buf[2] = nth_7b_chunk(input, 2);
                return 3;
            } else {
                buf[2] = nth_7b_chunk_with_high_bit(input, 2);
                if (input >> 7 * 4) == 0 {
                    buf[3] = nth_7b_chunk(input, 3);
                    return 4;
                } else {
                    buf[3] = nth_7b_chunk_with_high_bit(input, 3);
                    if (input >> 7 * 5) == 0 {
                        buf[4] = nth_7b_chunk(input, 4);
                        return 5;
                    } else {
                        buf[4] = nth_7b_chunk_with_high_bit(input, 4);
                        if (input >> 7 * 6) == 0 {
                            buf[5] = nth_7b_chunk(input, 5);
                            return 6;
                        } else {
                            buf[5] = nth_7b_chunk_with_high_bit(input, 5);
                            if (input >> 7 * 7) == 0 {
                                buf[6] = nth_7b_chunk(input, 6);
                                return 7;
                            } else {
                                buf[6] = nth_7b_chunk_with_high_bit(input, 6);
                                if (input >> 7 * 8) == 0 {
                                    buf[7] = nth_7b_chunk(input, 7);
                                    return 8;
                                } else {
                                    buf[7] = nth_7b_chunk_with_high_bit(input, 7);
                                    // special case: write last whole byte as is
                                    buf[8] = (input >> 56) as u8;
                                    return 9;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Read a LEB128-64b9B from the buffer
#[allow(dead_code)]
fn varint_read<R: Read>(reader: &mut R) -> io::Result<u64> {
    let mut b = read_u8(reader)?;

    // take low 7 bits
    let mut value: u64 = low_7_bits(b);

    if is_high_bit_set(b) {
        // high bit set, keep reading
        b = read_u8(reader)?;
        value |= low_7_bits(b) << 7;
        if is_high_bit_set(b) {
            b = read_u8(reader)?;
            value |= low_7_bits(b) << 7 * 2;
            if is_high_bit_set(b) {
                b = read_u8(reader)?;
                value |= low_7_bits(b) << 7 * 3;
                if is_high_bit_set(b) {
                    b = read_u8(reader)?;
                    value |= low_7_bits(b) << 7 * 4;
                    if is_high_bit_set(b) {
                        b = read_u8(reader)?;
                        value |= low_7_bits(b) << 7 * 5;
                        if is_high_bit_set(b) {
                            b = read_u8(reader)?;
                            value |= low_7_bits(b) << 7 * 6;
                            if is_high_bit_set(b) {
                                b = read_u8(reader)?;
                                value |= low_7_bits(b) << 7 * 7;
                                if is_high_bit_set(b) {
                                    b = read_u8(reader)?;
                                    // special case: use last byte as is
                                    value |= (b as u64) << 7 * 8;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(value)
}

/// input: a u64
/// n: >0, how many 7-bit shifts to do
/// Returns the n'th chunk (starting from least significant) of 7 bits as a byte with the the high
/// bit unchanged.
fn nth_7b_chunk(input: u64, n: u8) -> u8 {
    (input >> 7 * n) as u8
}

/// input: a u64
/// n: >0, how many 7-bit shifts to do
/// Returns the n'th chunk (starting from least significant) of 7 bits as a byte.
/// The high bit in the byte will be set (not one of the 7 bits that map to input bits).
fn nth_7b_chunk_with_high_bit(input: u64, n: u8) -> u8 {
    nth_7b_chunk(input, n) | 0x80
}

/// truncate byte to low 7 bits, cast to u64
fn low_7_bits(b: u8) -> u64 {
    (b & 0x7F) as u64
}

fn is_high_bit_set(b: u8) -> bool {
    // TODO benchmark leading zeros rather than masking
    (b & 0x80) != 0
}

fn read_u8<R: Read>(reader: &mut R) -> io::Result<u8> {
    let mut buf = [0; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

/// Map signed numbers to unsigned: 0 to 0, -1 to 1, 1 to 2, -2 to 3, etc
#[allow(dead_code)] // TODO
fn zig_zag_encode(num: i64) -> u64 {
    // If num < 0, num >> 63 is all 1 and vice versa.
    ((num << 1) ^ (num >> 63)) as u64
}

#[allow(dead_code)] // TODO
fn zig_zag_decode(encoded: u64) -> i64 {
    ((encoded >> 1) as i64) ^ -((encoded & 1) as i64)
}
