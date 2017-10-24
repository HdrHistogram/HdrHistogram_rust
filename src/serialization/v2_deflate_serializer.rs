use super::super::Histogram;
use core::counter::Counter;
use super::V2_COMPRESSED_COOKIE;
use super::v2_serializer::{V2Serializer, V2SerializeError};
use super::byteorder::{BigEndian, WriteBytesExt};
use super::flate2::Compression;
use std;
use std::io::{ErrorKind, Write};
use super::flate2::write::ZlibEncoder;

/// Errors that occur during serialization.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum V2DeflateSerializeError {
    /// The underlying serialization failed
    InternalSerializationError(V2SerializeError),
    /// An i/o operation failed.
    IoError(ErrorKind)
}

impl std::convert::From<std::io::Error> for V2DeflateSerializeError {
    fn from(e: std::io::Error) -> Self {
        V2DeflateSerializeError::IoError(e.kind())
    }
}

/// Serializer for the V2 + DEFLATE binary format.
///
/// It's called "deflate" to stay consistent with the naming used in the Java implementation, but
/// it actually uses zlib's wrapper format around plain DEFLATE.
pub struct V2DeflateSerializer {
    uncompressed_buf: Vec<u8>,
    compressed_buf: Vec<u8>,
    v2_serializer: V2Serializer
}

impl V2DeflateSerializer {
    /// Create a new serializer.
    pub fn new() -> V2DeflateSerializer {
        V2DeflateSerializer {
            uncompressed_buf: Vec::new(),
            compressed_buf: Vec::new(),
            v2_serializer: V2Serializer::new()
        }
    }

    /// Serialize the histogram into the provided writer.
    /// Returns the number of bytes written, or an error.
    ///
    /// Note that `Vec<u8>` is a reasonable `Write` implementation for simple usage.
    pub fn serialize<T: Counter, W: Write>(&mut self, h: &Histogram<T>, writer: &mut W)
                                           -> Result<usize, V2DeflateSerializeError> {
        // TODO benchmark serializing in chunks rather than all at once: each uncompressed v2 chunk
        // could be compressed and written to the compressed buf, possibly using an approach like
        // that of https://github.com/jonhoo/hdrsample/issues/32#issuecomment-287583055.
        // This would reduce the overall buffer size needed for plain v2 serialization, and be
        // more cache friendly.

        self.uncompressed_buf.clear();
        self.compressed_buf.clear();
        // TODO serialize directly into uncompressed_buf without the buffering inside v2_serializer
        let uncompressed_len = self.v2_serializer.serialize(h, &mut self.uncompressed_buf)
            .map_err(|e| V2DeflateSerializeError::InternalSerializationError(e))?;

        debug_assert_eq!(self.uncompressed_buf.len(), uncompressed_len);
        // On randomized test histograms we get about 10% compression, but of course random data
        // doesn't compress well. Real-world data may compress better, so let's assume a more
        // optimistic 50% compression as a baseline to reserve. If we're overly optimistic that's
        // still only one more allocation the first time it's needed.
        self.compressed_buf.reserve(self.uncompressed_buf.len() / 2);

        self.compressed_buf.write_u32::<BigEndian>(V2_COMPRESSED_COOKIE)?;
        // placeholder for length
        self.compressed_buf.write_u32::<BigEndian>(0)?;

        // TODO pluggable compressors? configurable compression levels?
        // TODO benchmark https://github.com/sile/libflate
        // TODO if uncompressed_len is near the limit of 16-bit usize, and compression grows the
        // data instead of shrinking it (which we cannot really predict), writing to compressed_buf
        // could panic as Vec overflows its internal `usize`.

        {
            // TODO reuse deflate buf, or switch to lower-level flate2::Compress
            let mut compressor = ZlibEncoder::new(&mut self.compressed_buf, Compression::Default);
            compressor.write_all(&self.uncompressed_buf[0..uncompressed_len])?;
            let _ = compressor.finish()?;
        }

        // fill in length placeholder. Won't underflow since length is always at least 8, and won't
        // overflow u32 as the largest array is about 6 million entries, so about 54MiB encoded (if
        // counter is u64).
        let total_compressed_len = self.compressed_buf.len();
        (&mut self.compressed_buf[4..8]).write_u32::<BigEndian>((total_compressed_len as u32) - 8)?;

        writer.write_all(&self.compressed_buf)?;

        Ok(total_compressed_len)
    }
}
