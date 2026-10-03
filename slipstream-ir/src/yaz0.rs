use byteorder::{BigEndian, ReadBytesExt};
use slipstream_shared::{
    cursor::RefCursor,
    error::{CorruptionError, IncorrectFormat, SlipstreamError, SlipstreamResult},
};

use crate::encoding::ReadArrayExt;

/// Magic of a YAZ0 file.
pub const YAZ0_MAGIC: [u8; 4] = [0x59, 0x61, 0x7a, 0x30];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// Size in bytes of the uncompressed file.
    pub uncompressed_size: u32,
    /// Reserved for special use. Always 0 in Mario Kart Wii.
    pub reserved: [u32; 2],
}

impl Header {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let magic = reader.read_u8_array::<4>()?;
        if magic != YAZ0_MAGIC {
            return Err(IncorrectFormat {
                expected_magic: YAZ0_MAGIC.to_vec(),
                found_magic: magic.to_vec(),
                location: Some(reader.position()),
            }
            .into());
        }

        let uncompressed_size = reader.read_u32::<BigEndian>()?;
        let reserved = reader.read_u32_array::<2, BigEndian>()?;

        if reserved != [0, 0] {
            tracing::warn!("`reserved` field in YAZ0 header is not all zeros");
        }

        Ok(Self {
            uncompressed_size,
            reserved,
        })
    }
}

pub fn decompress(compressed: &mut RefCursor<[u8]>) -> SlipstreamResult<Vec<u8>> {
    let yaz0_file = Yaz0File::deserialize(compressed)?;

    Ok(yaz0_file.uncompressed)
}

pub fn compress_yaz0(_uncompressed: &[u8]) -> Vec<u8> {
    todo!()
}

/// Implements YAZ0 compression and decompression.
///
/// Data is compressed using run-length encoding. A YAZ0 file consists of many data groups each having two fields
/// - Group header (1 byte)
/// - 8 chunks (8-24 bytes)
///
/// Each bit in the header corresponds to a chunk (MSB corresponds to chunk 1).
///
/// If the bit of the given chunk is set, the chunk consists of a single byte and can be copied to the output stream directly.
/// Otherwise we need to deserialize the chunk. It can be in two formats
///
/// 1. `NR RR`          `SIZE = N + 2`
/// 2. `0R RR NN`       `SIZE = N + 0x12`
///
/// `RRR` is a value between `0x000` and `0xfff`. Go back `RRR + 1` bytes in the output stream to find the start of
/// the data to copy.
/// `SIZE` is calculated from `N` to find the number of bytes to be copied.
///
/// Some chunks may also reference themselves. For example if `RRR = 1` (go back 1 + 1 = 2) and `SIZE = 10`, the previous 2 bytes
/// are copied 10/2 = 5 times.
///
/// See [`Yaz0Header`] for the binary format of the header.
/// See the [`Custom Mario Kart Wiiki`](`https://mkwiiki.org/wiki/YAZ0_(File_Format)`) for more info.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yaz0File {
    /// See [`Yaz0Header`] for the binary format of the header.
    pub header: Header,
    /// The uncompressed output stream.
    pub uncompressed: Vec<u8>,
}

impl Yaz0File {
    pub fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self> {
        let header = Header::deserialize(reader)?;

        tracing::trace!(
            "Decompressing Yaz0 archive ({} -> {})",
            reader.full_len(),
            header.uncompressed_size
        );

        let mut uncompressed = Vec::with_capacity(header.uncompressed_size as usize);

        // Amount of chunks in a data group
        const CHUNK_COUNT: usize = 8;

        let mut total_chunks = 0;
        while uncompressed.len() < uncompressed.capacity() {
            let mut group_header = reader.read_u8()?;
            for _ in 0..CHUNK_COUNT {
                if uncompressed.len() >= uncompressed.capacity() {
                    break;
                }

                total_chunks += 1;

                // If the bit is set, the chunk is 1 byte.
                // Otherwise it is 2 or 3 bytes
                let bit = (group_header & 0x80) != 0;
                group_header <<= 1;

                if bit {
                    // Copy over 1 byte immediately
                    uncompressed.push(reader.read_u8()?);
                } else {
                    // Perform run-length decoding.

                    // Read first two bytes of chunk
                    let b1 = reader.read_u8()? as usize;
                    let b2 = reader.read_u8()? as usize;

                    let rrr = (b1 & 0x0f) << 8 | b2;

                    let mut n = b1 >> 4;
                    let copy_size = if n == 0 {
                        // 3 byte data, NN is at the end
                        n = reader.read_u8()? as usize;
                        n + 0x12
                    } else {
                        n + 2
                    };

                    let copy_start = uncompressed.len().checked_sub(rrr + 1).ok_or_else(|| {
                        SlipstreamError::from(CorruptionError {
                            reason: "data group references byte before start of file".to_owned(),
                            location: Some(reader.position()),
                        })
                    })?;

                    for i in 0..copy_size {
                        let b = uncompressed[copy_start + i];
                        uncompressed.push(b);
                    }
                }
            }
        }

        if uncompressed.len() != header.uncompressed_size as usize {
            return Err(CorruptionError {
                reason: format!(
                    "uncompressed size in header does not equal actual size ({} vs. {})",
                    header.uncompressed_size,
                    uncompressed.len()
                ),
                ..Default::default()
            }
            .into());
        }

        tracing::trace!("Successfully decompressed {total_chunks} chunks in Yaz0 archive");

        Ok(Self {
            header,
            uncompressed,
        })
    }
}
