//! A memory-optimised sparse variant of [`crate::Histogram`].
//!
//! [`PackedHistogram`]'s backing store grows with the number of *populated*
//! buckets rather than with the histogram's `counts_len`, so many
//! sparsely-populated histograms cost a fraction of the dense footprint while
//! keeping the same bucket geometry and value/index mapping.
//!
//! It reuses the dense geometry through a `Histogram<u8>` oracle whose `counts`
//! vector is emptied (the geometry lookups never index `counts`), exactly as the
//! C `hdr_packed_histogram` does with `counts == NULL`. Records are a
//! binary-search + insert; queries are a blocked prefix-sum over only the
//! populated buckets, mirroring the dense `value_at_quantile`.

use crate::Histogram;
use crate::errors::{CreationError, RecordError};

const ORIGINAL_MAX: u64 = 0;
const ORIGINAL_MIN: u64 = u64::MAX;

/// A sparse, memory-optimised histogram. Interoperates with the dense
/// [`crate::Histogram`] geometry and (via serialization, added separately) the
/// V2 wire format.
#[derive(Debug)]
pub struct PackedHistogram {
    geom: Histogram<u8>, // geometry oracle; its `counts` is emptied and never indexed
    counts_len: usize,
    idx: Vec<u32>, // populated flat counts indices, ascending
    cnt: Vec<u8>,  // one count per populated bucket, little-endian, width `width`
    width: u8,     // count byte width: 1, 2, 4, 8
    total_count: u64,
    max_value: u64,
    min_non_zero_value: u64,
}

#[inline]
fn width_max(w: u8) -> u64 {
    match w {
        1 => 0xFF,
        2 => 0xFFFF,
        4 => 0xFFFF_FFFF,
        _ => u64::MAX,
    }
}

impl PackedHistogram {
    /// Create a sparse histogram with the same bounds as `Histogram::new_with_bounds`.
    pub fn new_with_bounds(low: u64, high: u64, sigfig: u8) -> Result<Self, CreationError> {
        let mut geom = Histogram::<u8>::new_with_bounds(low, high, sigfig)?;
        let counts_len = geom.distinct_values();
        geom.clear_counts_for_oracle();
        Ok(PackedHistogram {
            geom,
            counts_len,
            idx: Vec::new(),
            cnt: Vec::new(),
            width: 1,
            total_count: 0,
            max_value: ORIGINAL_MAX,
            min_non_zero_value: ORIGINAL_MIN,
        })
    }

    // ---- count-width backing ----

    #[inline]
    fn slot_get(&self, i: usize) -> u64 {
        let off = i * self.width as usize;
        match self.width {
            1 => u64::from(self.cnt[off]),
            2 => u64::from(u16::from_le_bytes([self.cnt[off], self.cnt[off + 1]])),
            4 => u64::from(u32::from_le_bytes(
                self.cnt[off..off + 4].try_into().unwrap(),
            )),
            _ => u64::from_le_bytes(self.cnt[off..off + 8].try_into().unwrap()),
        }
    }

    #[inline]
    fn slot_set(&mut self, i: usize, val: u64) {
        let off = i * self.width as usize;
        match self.width {
            1 => self.cnt[off] = val as u8,
            2 => self.cnt[off..off + 2].copy_from_slice(&(val as u16).to_le_bytes()),
            4 => self.cnt[off..off + 4].copy_from_slice(&(val as u32).to_le_bytes()),
            _ => self.cnt[off..off + 8].copy_from_slice(&val.to_le_bytes()),
        }
    }

    fn widen_to_fit(&mut self, need: u64) {
        let mut nw = self.width;
        while need > width_max(nw) && nw < 8 {
            nw *= 2;
        }
        if nw == self.width {
            return;
        }
        let size = self.idx.len();
        let mut nb = vec![0u8; size * nw as usize];
        for i in 0..size {
            let v = self.slot_get(i);
            let off = i * nw as usize;
            match nw {
                2 => nb[off..off + 2].copy_from_slice(&(v as u16).to_le_bytes()),
                4 => nb[off..off + 4].copy_from_slice(&(v as u32).to_le_bytes()),
                _ => nb[off..off + 8].copy_from_slice(&v.to_le_bytes()),
            }
        }
        self.cnt = nb;
        self.width = nw;
    }

    /// First position in `idx` whose value is >= key.
    #[inline]
    fn lower_bound(&self, key: u32) -> usize {
        let a = &self.idx;
        let (mut base, mut n) = (0usize, a.len());
        while n > 0 {
            let half = n >> 1;
            let mid = base + half;
            if a[mid] < key {
                base = mid + 1;
                n -= half + 1;
            } else {
                n = half;
            }
        }
        base
    }

    fn sparse_add(&mut self, ci: u32, delta: u64) {
        let pos = self.lower_bound(ci);
        if pos < self.idx.len() && self.idx[pos] == ci {
            let nv = self.slot_get(pos).saturating_add(delta);
            self.widen_to_fit(nv);
            self.slot_set(pos, nv);
            return;
        }
        self.widen_to_fit(delta);
        let w = self.width as usize;
        self.idx.insert(pos, ci);
        // open a width-sized zeroed gap at pos*w (copy_within has memmove semantics)
        let gap = pos * w;
        let old_len = self.cnt.len();
        self.cnt.resize(old_len + w, 0);
        self.cnt.copy_within(gap..old_len, gap + w);
        for b in self.cnt[gap..gap + w].iter_mut() {
            *b = 0;
        }
        self.slot_set(pos, delta);
    }

    // ---- record ----

    /// Record a single occurrence of `value`.
    pub fn record(&mut self, value: u64) -> Result<(), RecordError> {
        self.record_n(value, 1)
    }

    /// Record `count` occurrences of `value`. Returns `Err(())` if the value is
    /// out of the histogram's representable range (mirrors the dense
    /// `RecordError` for an unresizable histogram).
    pub fn record_n(&mut self, value: u64, count: u64) -> Result<(), RecordError> {
        let index = match self.geom.index_for(value) {
            Some(i) if i < self.counts_len => i,
            _ => return Err(RecordError::ValueOutOfRangeResizeDisabled),
        };
        if count != 0 {
            self.sparse_add(index as u32, count);
            self.total_count = self.total_count.saturating_add(count);
            if value > self.max_value {
                self.max_value = value;
            }
            if value != 0 && value < self.min_non_zero_value {
                self.min_non_zero_value = value;
            }
        }
        Ok(())
    }

    // ---- basic queries (bit-parity with dense) ----

    /// Total recorded count.
    pub fn len(&self) -> u64 {
        self.total_count
    }

    /// Whether nothing has been recorded.
    pub fn is_empty(&self) -> bool {
        self.total_count == 0
    }

    /// Lowest recorded value (0 if bucket 0 is populated or empty), matching `Histogram::min`.
    pub fn min(&self) -> u64 {
        if self.total_count == 0 {
            return 0;
        }
        // bucket 0 populated -> min is 0
        if self.idx.first() == Some(&0) && self.slot_get(0) != 0 {
            return 0;
        }
        if self.min_non_zero_value == ORIGINAL_MIN {
            // Defensive: unreachable via the public API (total_count > 0 with bucket 0
            // unpopulated implies a non-zero value was recorded, so min_non_zero_value
            // is set). Mirrors dense min_nz().
            0
        } else {
            self.geom.lowest_equivalent(self.min_non_zero_value)
        }
    }

    /// Highest recorded value, matching `Histogram::max` (overflow-safe).
    pub fn max(&self) -> u64 {
        if self.max_value == ORIGINAL_MAX {
            ORIGINAL_MAX
        } else {
            self.geom.highest_equivalent(self.max_value)
        }
    }

    /// Count recorded at `value`'s bucket (0 if out of range / unpopulated).
    pub fn count_at(&self, value: u64) -> u64 {
        match self.geom.index_for(value) {
            Some(ci) if ci < self.counts_len => {
                let pos = self.lower_bound(ci as u32);
                if pos < self.idx.len() && self.idx[pos] == ci as u32 {
                    self.slot_get(pos)
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    /// Number of populated buckets.
    pub fn populated(&self) -> usize {
        self.idx.len()
    }

    /// Current per-count byte width (1/2/4/8).
    pub fn count_width(&self) -> u8 {
        self.width
    }

    /// Value at the given percentile (`0.0..=100.0`).
    pub fn value_at_percentile(&self, percentile: f64) -> u64 {
        self.value_at_quantile(percentile / 100.0)
    }

    /// Value at the given quantile, bit-parity with `Histogram::value_at_quantile`.
    pub fn value_at_quantile(&self, quantile: f64) -> u64 {
        let quantile = if quantile > 1.0 { 1.0 } else { quantile };
        let fractional_count = quantile * self.total_count as f64;
        let mut count_at_quantile = fractional_count.ceil() as u64;
        if count_at_quantile == 0 {
            count_at_quantile = 1;
        }

        match self.scan_position(count_at_quantile) {
            Some(pos) => {
                let v = self.geom.value_for(self.idx[pos] as usize);
                if quantile == 0.0 {
                    self.geom.lowest_equivalent(v)
                } else {
                    self.geom.highest_equivalent(v)
                }
            }
            None => 0,
        }
    }

    /// Width-specialized blocked prefix-sum: returns the position in `idx` whose
    /// cumulative count first reaches `target` (mirrors the dense chunked scan).
    /// Hoisting the width match out of the per-element read lets the block sum
    /// vectorize. Widths <= 4 cannot overflow u64 across the array (at most
    /// counts_len buckets, each < 2^32), so no per-element saturation is needed;
    /// width 8 keeps the saturating scalar walk.
    fn scan_position(&self, target: u64) -> Option<usize> {
        const CHUNK: usize = 8;
        let n = self.idx.len();
        let c = &self.cnt;
        macro_rules! scan {
            ($get:expr) => {{
                let mut running: u64 = 0;
                let mut i = 0usize;
                while i + CHUNK <= n {
                    let mut s: u64 = 0;
                    for k in 0..CHUNK {
                        s += $get(i + k);
                    }
                    if running + s >= target {
                        for k in 0..CHUNK {
                            running += $get(i + k);
                            if running >= target {
                                return Some(i + k);
                            }
                        }
                    }
                    running += s;
                    i += CHUNK;
                }
                while i < n {
                    running += $get(i);
                    if running >= target {
                        return Some(i);
                    }
                    i += 1;
                }
                None
            }};
        }
        match self.width {
            1 => scan!(|j: usize| u64::from(c[j])),
            2 => scan!(|j: usize| u64::from(u16::from_le_bytes([c[2 * j], c[2 * j + 1]]))),
            4 => scan!(|j: usize| u64::from(u32::from_le_bytes([
                c[4 * j],
                c[4 * j + 1],
                c[4 * j + 2],
                c[4 * j + 3]
            ]))),
            _ => {
                let mut running: u64 = 0;
                let mut i = 0usize;
                while i < n {
                    running = running.saturating_add(u64::from_le_bytes(
                        c[8 * i..8 * i + 8].try_into().unwrap(),
                    ));
                    if running >= target {
                        return Some(i);
                    }
                    i += 1;
                }
                None
            }
        }
    }

    /// Bytes held by the sparse backing (idx + cnt capacities); the geometry
    /// oracle is a fixed small overhead excluded here.
    pub fn memory_size(&self) -> usize {
        self.idx.capacity() * std::mem::size_of::<u32>() + self.cnt.capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::PackedHistogram;
    use crate::Histogram;

    pub(super) struct Xs(pub u64);
    impl Xs {
        pub(super) fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }
    }

    #[test]
    fn parity_random() {
        let pcts = [0.0, 1.0, 25.0, 50.0, 75.0, 90.0, 99.0, 99.9, 100.0];
        for trial in 0..300u64 {
            let mut rng = Xs(0x9E37_79B9_7F4A_7C15u64.wrapping_mul(trial + 1) | 1);
            let mut d: Histogram<u64> = Histogram::new_with_bounds(1, 3_600_000_000, 3).unwrap();
            let mut p = PackedHistogram::new_with_bounds(1, 3_600_000_000, 3).unwrap();
            let n = 1 + (rng.next() % 4000);
            for _ in 0..n {
                let v = rng.next() % 3_600_000_000 + 1;
                let c = 1 + (rng.next() % 9);
                d.record_n(v, c).unwrap();
                p.record_n(v, c).unwrap();
            }
            assert_eq!(d.len(), p.len(), "trial {trial}: total");
            assert_eq!(d.min(), p.min(), "trial {trial}: min");
            assert_eq!(d.max(), p.max(), "trial {trial}: max");
            // count parity, index by index
            for i in 0..d.distinct_values() {
                let v = d.value_for_test(i);
                assert_eq!(
                    d.count_at(v),
                    p.count_at(v),
                    "trial {trial}: count at flat {i}"
                );
            }
            for &pc in &pcts {
                assert_eq!(
                    d.value_at_percentile(pc),
                    p.value_at_percentile(pc),
                    "trial {trial}: p{pc}"
                );
            }
        }
    }

    #[test]
    fn width_growth() {
        let mut p = PackedHistogram::new_with_bounds(1, 3_600_000_000, 3).unwrap();
        p.record_n(6147, 200).unwrap();
        assert_eq!(p.count_width(), 1);
        p.record_n(6147, 70_000).unwrap();
        assert_eq!(p.count_width(), 4);
        p.record_n(6147, 5_000_000_000).unwrap();
        assert_eq!(p.count_width(), 8);
        assert_eq!(p.count_at(6147), 200 + 70_000 + 5_000_000_000);
    }
}

#[cfg(test)]
mod mem_proof {
    use super::PackedHistogram;
    #[test]
    fn memory_win() {
        let dense_per_histo = 23552usize * 8; // counts_len * size_of::<u64> for (1,3.6e9,3)
        for d in [10usize, 100] {
            let mut p = PackedHistogram::new_with_bounds(1, 3_600_000_000, 3).unwrap();
            for k in 0..d {
                p.record(1 + (3_600_000_000u64 / (d as u64 + 1)) * (k as u64 + 1))
                    .unwrap();
            }
            let packed = p.memory_size();
            println!(
                "D={:<4} dense={} B  packed={} B  win={:.0}x",
                d,
                dense_per_histo,
                packed,
                dense_per_histo as f64 / packed as f64
            );
        }
    }
}

// ---- V2 serialization (byte-identical to the dense V2Serializer / V2DeflateSerializer) ----
#[cfg(feature = "serialization")]
mod packed_serialization {
    use super::PackedHistogram;
    use crate::serialization::{
        V2_COMPRESSED_COOKIE, V2_COOKIE, V2_HEADER_SIZE, varint_read_slice, varint_write,
        zig_zag_decode, zig_zag_encode,
    };
    use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
    use flate2::Compression;
    use flate2::read::ZlibDecoder;
    use flate2::write::ZlibEncoder;
    use std::io::{self, Read, Write};

    /// Error serializing a `PackedHistogram`.
    #[derive(Debug)]
    pub enum PackedSerializeError {
        /// A count above `i64::MAX` cannot be zig-zag encoded.
        CountNotSerializable,
        /// An i/o error occurred.
        Io(io::Error),
    }
    impl From<io::Error> for PackedSerializeError {
        fn from(e: io::Error) -> Self {
            PackedSerializeError::Io(e)
        }
    }

    /// Error deserializing a `PackedHistogram`.
    #[derive(Debug)]
    pub enum PackedDeserializeError {
        /// The stream did not start with a supported V2 cookie.
        InvalidCookie,
        /// A non-zero normalizing offset or non-1.0 conversion ratio (packed is never rotated).
        UnsupportedFeature,
        /// Header parameters could not build a histogram, or the payload indexed out of range.
        InvalidParameters,
        /// An i/o error occurred.
        Io(io::Error),
    }
    impl From<io::Error> for PackedDeserializeError {
        fn from(e: io::Error) -> Self {
            PackedDeserializeError::Io(e)
        }
    }

    impl PackedHistogram {
        /// Serialize into the standard uncompressed V2 format, byte-identical to
        /// `V2Serializer` on an equivalent dense histogram.
        pub fn serialize_v2<W: Write>(
            &self,
            writer: &mut W,
        ) -> Result<usize, PackedSerializeError> {
            let buf = self.to_v2_bytes()?;
            writer.write_all(&buf)?;
            Ok(buf.len())
        }

        /// Serialize into the standard V2 + DEFLATE format, byte-identical to
        /// `V2DeflateSerializer` on an equivalent dense histogram.
        pub fn serialize_v2_deflate<W: Write>(
            &self,
            writer: &mut W,
        ) -> Result<usize, PackedSerializeError> {
            let uncompressed = self.to_v2_bytes()?;
            let mut out: Vec<u8> = Vec::new();
            out.write_u32::<BigEndian>(V2_COMPRESSED_COOKIE)?;
            out.write_u32::<BigEndian>(0)?; // length placeholder
            {
                let mut enc = ZlibEncoder::new(&mut out, Compression::default());
                enc.write_all(&uncompressed)?;
                let _ = enc.finish()?;
            }
            let compressed_len = (out.len() - 8) as u32;
            (&mut out[4..8]).write_u32::<BigEndian>(compressed_len)?;
            writer.write_all(&out)?;
            Ok(out.len())
        }

        fn to_v2_bytes(&self) -> Result<Vec<u8>, PackedSerializeError> {
            let mut buf: Vec<u8> = Vec::new();
            buf.write_u32::<BigEndian>(V2_COOKIE)?;
            buf.write_u32::<BigEndian>(0)?; // payload length placeholder
            buf.write_u32::<BigEndian>(0)?; // normalizing index offset
            buf.write_u32::<BigEndian>(u32::from(self.geom.sigfig()))?;
            buf.write_u64::<BigEndian>(self.geom.low())?;
            buf.write_u64::<BigEndian>(self.geom.high())?;
            buf.write_f64::<BigEndian>(1.0)?;
            debug_assert_eq!(buf.len(), V2_HEADER_SIZE);

            let payload_len = self.encode_counts_into(&mut buf)?;
            (&mut buf[4..8]).write_u32::<BigEndian>(payload_len as u32)?;
            Ok(buf)
        }

        /// Stream the RLE + zig-zag varint payload directly from the sparse
        /// backing, identical to the dense `encode_counts`.
        fn encode_counts_into(&self, buf: &mut Vec<u8>) -> Result<usize, PackedSerializeError> {
            let index_limit = match self.geom.index_for(self.max()) {
                Some(i) => i,
                // Defensive: max() is always representable, so index_for never returns None.
                None => return Ok(0),
            };
            let start = buf.len();
            let mut j = 0usize; // cursor over idx
            let mut index = 0usize;
            let mut tmp = [0u8; 9];
            while index <= index_limit {
                let count = self.count_at_flat(index, &mut j);
                index += 1;
                let mut zero_count: i64 = 0;
                if count == 0 {
                    zero_count = 1;
                    while index <= index_limit && self.count_at_flat(index, &mut j) == 0 {
                        zero_count += 1;
                        index += 1;
                    }
                }
                let count_or_zeros: i64 = if zero_count > 1 {
                    -zero_count
                } else if count > i64::MAX as u64 {
                    return Err(PackedSerializeError::CountNotSerializable);
                } else {
                    count as i64
                };
                let w = varint_write(zig_zag_encode(count_or_zeros), &mut tmp);
                buf.extend_from_slice(&tmp[..w]);
            }
            Ok(buf.len() - start)
        }

        /// Count at flat index `src`, advancing the monotonic cursor `j` over `idx`.
        #[inline]
        fn count_at_flat(&self, src: usize, j: &mut usize) -> u64 {
            while *j < self.idx.len() && (self.idx[*j] as usize) < src {
                *j += 1;
            }
            if *j < self.idx.len() && self.idx[*j] as usize == src {
                self.slot_get(*j)
            } else {
                0
            }
        }

        /// Deserialize a standard V2 (compressed or not) stream into a new packed histogram.
        pub fn deserialize<R: Read>(reader: &mut R) -> Result<Self, PackedDeserializeError> {
            let cookie = reader.read_u32::<BigEndian>()?;
            match cookie {
                V2_COOKIE => Self::deser_v2(reader),
                V2_COMPRESSED_COOKIE => {
                    let payload_len = reader.read_u32::<BigEndian>()? as u64;
                    let mut z = ZlibDecoder::new(reader.take(payload_len));
                    if z.read_u32::<BigEndian>()? != V2_COOKIE {
                        return Err(PackedDeserializeError::InvalidCookie);
                    }
                    Self::deser_v2(&mut z)
                }
                _ => Err(PackedDeserializeError::InvalidCookie),
            }
        }

        fn deser_v2<R: Read>(reader: &mut R) -> Result<Self, PackedDeserializeError> {
            let payload_len = reader.read_u32::<BigEndian>()? as usize;
            if reader.read_u32::<BigEndian>()? != 0 {
                return Err(PackedDeserializeError::UnsupportedFeature);
            }
            let sig = reader.read_u32::<BigEndian>()?;
            let low = reader.read_u64::<BigEndian>()?;
            let high = reader.read_u64::<BigEndian>()?;
            if reader.read_f64::<BigEndian>()? != 1.0 {
                return Err(PackedDeserializeError::UnsupportedFeature);
            }
            let sig = if sig <= 5 {
                sig as u8
            } else {
                return Err(PackedDeserializeError::InvalidParameters);
            };
            let mut p = PackedHistogram::new_with_bounds(low, high, sig)
                .map_err(|_| PackedDeserializeError::InvalidParameters)?;

            let mut payload = vec![0u8; payload_len];
            reader.read_exact(&mut payload)?;

            let mut i = 0usize;
            let mut dst: usize = 0;
            let mut total: u64 = 0;
            while i < payload_len {
                let (zz, br) = if payload_len - i >= 9 {
                    varint_read_slice(&payload[i..i + 9])
                } else {
                    let mut t = [0u8; 9];
                    t[..payload_len - i].copy_from_slice(&payload[i..]);
                    varint_read_slice(&t)
                };
                i += br;
                let val = zig_zag_decode(zz);
                if val < 0 {
                    dst = dst
                        .checked_add((-val) as usize)
                        .ok_or(PackedDeserializeError::InvalidParameters)?;
                } else {
                    if dst >= p.counts_len {
                        return Err(PackedDeserializeError::InvalidParameters);
                    }
                    if val != 0 {
                        p.sparse_add(dst as u32, val as u64);
                        total = total.saturating_add(val as u64);
                    }
                    dst += 1;
                }
                if dst > p.counts_len {
                    return Err(PackedDeserializeError::InvalidParameters);
                }
            }
            p.total_count = total;
            // Reconstruct max/min from the populated buckets so min()/max() match dense.
            if let Some(&last) = p.idx.last() {
                p.max_value = p.geom.value_for(last as usize);
                // first non-zero populated bucket
                let first_nz = if p.idx.first() == Some(&0) {
                    p.idx.get(1)
                } else {
                    p.idx.first()
                };
                if let Some(&fnz) = first_nz {
                    p.min_non_zero_value = p.geom.value_for(fnz as usize);
                }
            }
            Ok(p)
        }
    }
}

#[cfg(all(test, feature = "serialization"))]
mod serialization_tests {
    use super::PackedHistogram;
    use super::tests::Xs;
    use crate::Histogram;
    use crate::serialization::{Deserializer, Serializer, V2DeflateSerializer, V2Serializer};
    use std::io::Cursor;

    #[test]
    fn v2_byte_identical_and_roundtrip() {
        for trial in 0..80u64 {
            let mut rng = Xs(0x2545_F491_4F6C_DD1Du64.wrapping_mul(trial + 1) | 1);
            let mut d: Histogram<u64> = Histogram::new_with_bounds(1, 3_600_000_000, 3).unwrap();
            let mut p = PackedHistogram::new_with_bounds(1, 3_600_000_000, 3).unwrap();
            let n = rng.next() % 3000; // includes empty
            for _ in 0..n {
                let v = rng.next() % 3_600_000_000 + 1;
                let c = 1 + (rng.next() % 9);
                d.record_n(v, c).unwrap();
                p.record_n(v, c).unwrap();
            }

            // 1. uncompressed V2 byte-identical
            let mut dv2 = Vec::new();
            let _ = V2Serializer::new().serialize(&d, &mut dv2).unwrap();
            let mut pv2 = Vec::new();
            let _ = p.serialize_v2(&mut pv2).unwrap();
            assert_eq!(dv2, pv2, "trial {trial}: V2 bytes differ");

            // 2. deflate V2 byte-identical
            let mut dz = Vec::new();
            let _ = V2DeflateSerializer::new().serialize(&d, &mut dz).unwrap();
            let mut pz = Vec::new();
            let _ = p.serialize_v2_deflate(&mut pz).unwrap();
            assert_eq!(dz, pz, "trial {trial}: deflate bytes differ");

            // 3. packed encode -> dense decode == original dense
            let dfp: Histogram<u64> = Deserializer::new()
                .deserialize(&mut Cursor::new(&pv2))
                .unwrap();
            assert_eq!(dfp.len(), d.len());
            for i in 0..d.distinct_values() {
                let v = d.value_for_test(i);
                assert_eq!(
                    dfp.count_at(v),
                    d.count_at(v),
                    "trial {trial}: decoded count flat {i}"
                );
            }

            // 4. dense encode -> packed decode == dense (queries)
            let prt = PackedHistogram::deserialize(&mut Cursor::new(&dv2)).unwrap();
            assert_eq!(prt.len(), d.len());
            assert_eq!(prt.min(), d.min());
            assert_eq!(prt.max(), d.max());
            for &pc in &[0.0, 50.0, 90.0, 99.0, 99.9, 100.0] {
                assert_eq!(
                    prt.value_at_percentile(pc),
                    d.value_at_percentile(pc),
                    "trial {trial}: rt p{pc}"
                );
            }

            // 5. deflate round-trip through packed
            let prt2 = PackedHistogram::deserialize(&mut Cursor::new(&pz)).unwrap();
            assert_eq!(prt2.len(), d.len());
        }
    }
}

#[cfg(all(test, feature = "serialization"))]
mod fuzz_tests {
    use super::PackedHistogram;
    use crate::Histogram;
    use crate::serialization::{Serializer, V2Serializer};
    use rand::rngs::SmallRng;
    use rand::{RngExt, SeedableRng};
    use std::io::Cursor;

    // Randomized differential fuzz: many random workloads must keep packed
    // bit-for-bit equal to dense on every query and on the V2 encoding.
    #[test]
    fn fuzz_differential() {
        let mut rng = SmallRng::seed_from_u64(0xC0FF_EE00_1234_5678);
        for _ in 0..4000 {
            let mut d: Histogram<u64> = Histogram::new_with_bounds(1, 3_600_000_000, 3).unwrap();
            let mut p = PackedHistogram::new_with_bounds(1, 3_600_000_000, 3).unwrap();
            let n = rng.random_range(0..500);
            for _ in 0..n {
                let v = rng.random_range(1..3_600_000_000u64);
                let c = rng.random_range(1..100_000u64);
                d.record_n(v, c).unwrap();
                p.record_n(v, c).unwrap();
            }
            assert_eq!(d.len(), p.len());
            assert_eq!(d.min(), p.min());
            assert_eq!(d.max(), p.max());
            for _ in 0..8 {
                let q = rng.random_range(0.0..100.0);
                assert_eq!(d.value_at_percentile(q), p.value_at_percentile(q), "q={q}");
            }
            let mut dv = Vec::new();
            let _ = V2Serializer::new().serialize(&d, &mut dv).unwrap();
            let mut pv = Vec::new();
            let _ = p.serialize_v2(&mut pv).unwrap();
            assert_eq!(dv, pv, "encode diverged");
        }
    }

    // Hostile-decode fuzz: PackedHistogram::deserialize must never panic on
    // arbitrary bytes, and any successful decode must survive re-encode/re-decode.
    #[test]
    fn fuzz_hostile_decode() {
        let mut rng = SmallRng::seed_from_u64(0xDEAD_BEEF_CAFE_F00D);
        // seed with a valid stream so the corpus includes near-valid inputs
        let mut seed = PackedHistogram::new_with_bounds(1, 3_600_000_000, 3).unwrap();
        seed.record_n(12345, 6).unwrap();
        seed.record_n(2_000_000, 500_000).unwrap();
        let mut valid = Vec::new();
        let _ = seed.serialize_v2(&mut valid).unwrap();

        for _ in 0..200_000 {
            let mut data: Vec<u8> = if rng.random_bool(0.3) {
                valid.clone()
            } else {
                let len = rng.random_range(0..48);
                (0..len).map(|_| rng.random_range(0..=u8::MAX)).collect()
            };
            // random single-byte corruption of the valid-ish inputs
            if !data.is_empty() && rng.random_bool(0.5) {
                let i = rng.random_range(0..data.len());
                data[i] = rng.random_range(0..=u8::MAX);
            }
            if let Ok(hp) = PackedHistogram::deserialize(&mut Cursor::new(&data)) {
                // exercise queries (must not panic)
                let _ = (hp.len(), hp.min(), hp.max());
                let _ = hp.value_at_percentile(99.9);
                // re-encode + re-decode must round-trip total
                let mut re = Vec::new();
                let _ = hp.serialize_v2(&mut re).unwrap();
                let hp2 = PackedHistogram::deserialize(&mut Cursor::new(&re)).unwrap();
                assert_eq!(hp.len(), hp2.len(), "total drift on re-decode");
            }
        }
    }
}

#[cfg(all(test, feature = "serialization"))]
mod coverage_tests {
    use super::PackedHistogram;
    use crate::serialization::{V2_COOKIE, V2_HEADER_SIZE, varint_write, zig_zag_encode};
    use byteorder::{BigEndian, WriteBytesExt};
    use std::io::Cursor;

    #[test]
    fn record_and_count_out_of_range() {
        let mut p = PackedHistogram::new_with_bounds(1, 1000, 3).unwrap();
        assert!(p.record(u64::MAX).is_err()); // beyond the representable range
        assert_eq!(p.count_at(u64::MAX), 0); // out of range -> 0
        assert_eq!(p.count_at(500), 0); // in range, never recorded
    }

    #[test]
    fn empty_and_populated_accessors() {
        let mut p = PackedHistogram::new_with_bounds(1, 1000, 3).unwrap();
        assert!(p.is_empty());
        assert_eq!(p.populated(), 0);
        p.record(500).unwrap();
        assert!(!p.is_empty());
        assert_eq!(p.populated(), 1);
    }

    #[test]
    fn count_above_i64_not_serializable() {
        let mut p = PackedHistogram::new_with_bounds(1, 3_600_000_000, 3).unwrap();
        p.record_n(6147, u64::MAX).unwrap(); // width widens to 8; count > i64::MAX
        let mut out = Vec::new();
        assert!(p.serialize_v2(&mut out).is_err());
    }

    struct FailWriter;
    impl std::io::Write for FailWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("fail"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::other("fail"))
        }
    }

    #[test]
    fn serialize_and_deserialize_io_errors() {
        let mut p = PackedHistogram::new_with_bounds(1, 1000, 3).unwrap();
        p.record(500).unwrap();
        assert!(p.serialize_v2(&mut FailWriter).is_err());
        assert!(p.serialize_v2_deflate(&mut FailWriter).is_err());
        // truncated reader -> read_u32 EOF -> Io error path
        assert!(PackedHistogram::deserialize(&mut Cursor::new(&[])).is_err());
    }

    #[test]
    fn deserialize_bad_cookie() {
        let bytes = [0u8, 0, 0, 0, 1, 2, 3, 4];
        assert!(PackedHistogram::deserialize(&mut Cursor::new(&bytes)).is_err());
    }

    #[test]
    fn deserialize_corrupt_index_overflow() {
        // Valid header, but the payload skips past counts_len then writes a count,
        // hitting the dst-overflow guard.
        let counts_len = {
            let p = PackedHistogram::new_with_bounds(1, 3_600_000_000, 3).unwrap();
            p.counts_len
        };
        let mut payload = Vec::new();
        let mut tmp = [0u8; 9];
        // negative zero-run == counts_len (skip to counts_len), then a positive count
        let w = varint_write(zig_zag_encode(-(counts_len as i64)), &mut tmp);
        payload.extend_from_slice(&tmp[..w]);
        let w = varint_write(zig_zag_encode(1), &mut tmp);
        payload.extend_from_slice(&tmp[..w]);

        let mut stream = Vec::new();
        stream.write_u32::<BigEndian>(V2_COOKIE).unwrap();
        stream.write_u32::<BigEndian>(payload.len() as u32).unwrap();
        stream.write_u32::<BigEndian>(0).unwrap();
        stream.write_u32::<BigEndian>(3).unwrap();
        stream.write_u64::<BigEndian>(1).unwrap();
        stream.write_u64::<BigEndian>(3_600_000_000).unwrap();
        stream.write_f64::<BigEndian>(1.0).unwrap();
        assert_eq!(stream.len(), V2_HEADER_SIZE);
        stream.extend_from_slice(&payload);

        assert!(PackedHistogram::deserialize(&mut Cursor::new(&stream)).is_err());
    }
}
