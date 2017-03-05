use std::io::{self, Read, Write};

#[path = "tests.rs"]
#[cfg(test)]
mod tests;

/// Write a number as a LEB128-64b9B little endian base 128 varint to buf. This is not
/// quite the same as Protobuf's LEB128 as it encodes 64 bit values in a max of 9 bytes, not 10.
/// The first 8 7-bit chunks are encoded normally (up through the first 7 bytes of input). The last
/// byte is added to the buf as-is. This limits the input to 8 bytes, but that's all we need.
#[allow(dead_code)]
fn varint_write<W: Write> (input: u64, writer: &mut W) -> io::Result<usize> {
    // The loop is unrolled because the special case is awkward to express in a loop, and it
    // probably makes the branch predictor happier to do it this way.

    let mut arr: [u8; 9] = [0; 9];
    let mut bytes_used = 0;

    if (input >> 7) == 0 {
        arr[0] = input as u8;
        bytes_used += 1;
    } else {
        // set high bit because more bytes are coming, then next 7 bits of value.
        arr[0] = 0x80 | ((input & 0x7F) as u8);
        bytes_used += 1;
        if (input >> 7 * 2) == 0 {
            // nothing above bottom 2 chunks, this is the last byte, so no high bit
            arr[1] = nth_7b_chunk(input, 1);
            bytes_used += 1;
        } else {
            arr[1] = nth_7b_chunk_with_high_bit(input, 1);
            bytes_used += 1;
            if (input >> 7 * 3) == 0 {
                arr[2] = nth_7b_chunk(input, 2);
                bytes_used += 1;
            } else {
                arr[2] = nth_7b_chunk_with_high_bit(input, 2);
                bytes_used += 1;
                if (input >> 7 * 4) == 0 {
                    arr[3] = nth_7b_chunk(input, 3);
                    bytes_used += 1;
                } else {
                    arr[3] = nth_7b_chunk_with_high_bit(input, 3);
                    bytes_used += 1;
                    if (input >> 7 * 5) == 0 {
                        arr[4] = nth_7b_chunk(input, 4);
                        bytes_used += 1;
                    } else {
                        arr[4] = nth_7b_chunk_with_high_bit(input, 4);
                        bytes_used += 1;
                        if (input >> 7 * 6) == 0 {
                            arr[5] = nth_7b_chunk(input, 5);
                            bytes_used += 1;
                        } else {
                            arr[5] = nth_7b_chunk_with_high_bit(input, 5);
                            bytes_used += 1;
                            if (input >> 7 * 7) == 0 {
                                arr[6] = nth_7b_chunk(input, 6);
                                bytes_used += 1;
                            } else {
                                arr[6] = nth_7b_chunk_with_high_bit(input, 6);
                                bytes_used += 1;
                                if (input >> 7 * 8) == 0 {
                                    arr[7] = nth_7b_chunk(input, 7);
                                    bytes_used += 1;
                                } else {
                                    arr[7] = nth_7b_chunk_with_high_bit(input, 7);
                                    bytes_used += 1;
                                    // special case: write last whole byte as is
                                    arr[8] = (input >> 56) as u8;
                                    bytes_used += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    return writer.write_all(&arr[0..bytes_used]).map(|_| bytes_used)
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
/// bit unset.
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
