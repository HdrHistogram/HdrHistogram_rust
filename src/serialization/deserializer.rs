use super::V2_COOKIE;
use super::super::{Counter, Histogram};
use super::super::num::ToPrimitive;
use std::io::{self, Cursor, ErrorKind, Read};
use std;
use super::byteorder::{BigEndian, ReadBytesExt};

/// Errors that can happen during deserialization.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DeserializeError {
    /// An i/o operation failed.
    IoError(ErrorKind),
    /// The cookie (first 4 bytes) did not match that for any supported format.
    InvalidCookie,
    /// The histogram uses features that this implementation doesn't support (yet), so it cannot
    /// be deserialized correctly.
    UnsupportedFeature,
    /// A count exceeded what can be represented in the chosen counter type.
    UnsuitableCounterType,
    /// The histogram instance could not be created because the serialized parameters were invalid
    /// (e.g. lowest value, highest value, etc.)
    InvalidParameters,
    /// The current system's pointer width cannot represent the encoded histogram.
    UsizeTypeTooSmall,
    /// The encoded array is longer than it should be for the histogram's value range.
    EncodedArrayTooLong
}

impl std::convert::From<std::io::Error> for DeserializeError {
    fn from(e: std::io::Error) -> Self {
        DeserializeError::IoError(e.kind())
    }
}

/// Deserializer for all supported formats.
/// Deserializers are intended to be re-used for many histograms.
pub struct Deserializer {
    payload_buf: Vec<u8>
}

impl Deserializer {
    /// Create a new deserializer.
    pub fn new() -> Deserializer {
        Deserializer {
            payload_buf: Vec::new()
        }
    }

    /// Deserialize an encoded histogram.
    pub fn deserialize<T: Counter, R: Read>(&mut self, reader: &mut R)
                                            -> Result<Histogram<T>, DeserializeError> {
        // TODO benchmark minimizing read calls by reading into a fixed-size header buffer

        let cookie = reader.read_u32::<BigEndian>()?;

        if cookie != V2_COOKIE {
            return Err(DeserializeError::InvalidCookie);
        }

        let payload_len = reader.read_u32::<BigEndian>()?.to_usize()
            .ok_or(DeserializeError::UsizeTypeTooSmall)?;
        let normalizing_offset = reader.read_u32::<BigEndian>()?;
        if normalizing_offset != 0 {
            return Err(DeserializeError::UnsupportedFeature);
        }
        let num_digits = reader.read_u32::<BigEndian>()?.to_u8()
            .ok_or(DeserializeError::InvalidParameters)?;
        let low = reader.read_u64::<BigEndian>()?;
        let high = reader.read_u64::<BigEndian>()?;
        let int_double_ratio = reader.read_f64::<BigEndian>()?;
        if int_double_ratio != 1.0 {
            return Err(DeserializeError::UnsupportedFeature);
        }

        let mut h = Histogram::new_with_bounds(low, high, num_digits)
            .map_err(|_| DeserializeError::InvalidParameters)?;

        if payload_len > self.payload_buf.len() {
            self.payload_buf.resize(payload_len, 0);
        }

        let mut payload_slice = &mut self.payload_buf[0..payload_len];
        reader.read_exact(&mut payload_slice)?;

        let mut cursor = Cursor::new(&payload_slice);
        let mut dest_index: usize = 0;
        while cursor.position() < payload_slice.len() as u64 {
            let num = zig_zag_decode(varint_read(&mut cursor)?);

            if num < 0 {
                let zero_count = (-num).to_usize()
                    .ok_or(DeserializeError::UsizeTypeTooSmall)?;
                // skip the zeros
                dest_index = dest_index.checked_add(zero_count)
                    .ok_or(DeserializeError::UsizeTypeTooSmall)?;
                continue;
            } else {
                let count: T = T::from_i64(num)
                    .ok_or(DeserializeError::UnsuitableCounterType)?;

                h.set_count_at_index(dest_index, count)
                    .map_err(|_| DeserializeError::EncodedArrayTooLong)?;

                dest_index = dest_index.checked_add(1)
                    .ok_or(DeserializeError::UsizeTypeTooSmall)?;
            }
        }

        // TODO restat is expensive; should accumulate the necessary state while deserializing
        // dest_index is one past the last written index, and is therefore the length to scan
        h.restat(dest_index);

        Ok(h)
    }
}

/// Read a LEB128-64b9B from the buffer
pub fn varint_read<R: Read>(reader: &mut R) -> io::Result<u64> {
    let mut b = reader.read_u8()?;

    // take low 7 bits
    let mut value: u64 = low_7_bits(b);

    if is_high_bit_set(b) {
        // high bit set, keep reading
        b = reader.read_u8()?;
        value |= low_7_bits(b) << 7;
        if is_high_bit_set(b) {
            b = reader.read_u8()?;
            value |= low_7_bits(b) << 7 * 2;
            if is_high_bit_set(b) {
                b = reader.read_u8()?;
                value |= low_7_bits(b) << 7 * 3;
                if is_high_bit_set(b) {
                    b = reader.read_u8()?;
                    value |= low_7_bits(b) << 7 * 4;
                    if is_high_bit_set(b) {
                        b = reader.read_u8()?;
                        value |= low_7_bits(b) << 7 * 5;
                        if is_high_bit_set(b) {
                            b = reader.read_u8()?;
                            value |= low_7_bits(b) << 7 * 6;
                            if is_high_bit_set(b) {
                                b = reader.read_u8()?;
                                value |= low_7_bits(b) << 7 * 7;
                                if is_high_bit_set(b) {
                                    b = reader.read_u8()?;
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

/// truncate byte to low 7 bits, cast to u64
#[inline]
fn low_7_bits(b: u8) -> u64 {
    (b & 0x7F) as u64
}

#[inline]
fn is_high_bit_set(b: u8) -> bool {
    // TODO benchmark leading zeros rather than masking
    (b & 0x80) != 0
}

#[inline]
pub fn zig_zag_decode(encoded: u64) -> i64 {
    ((encoded >> 1) as i64) ^ -((encoded & 1) as i64)
}
