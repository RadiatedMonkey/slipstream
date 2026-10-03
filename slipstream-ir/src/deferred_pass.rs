use std::{collections::HashMap, io::Write};

use byteorder::{BigEndian, WriteBytesExt};
use slipstream_shared::{SlipstreamResult, cursor::MutCursor};

/// Placeholder that is used for 32-bit offsets. A recognizable placeholder
/// makes it easier to find offsets.
pub const DEFER_PLACEHOLDER: u32 = 0xDEADBEEF;
/// Placeholder that is used for 24-bit offsets. A recognizable placeholder
/// makes it easier to find offsets.
pub const DEFER_PLACEHOLDER24: u32 = 0xDEBEEF;

#[derive(Debug)]
pub struct DeferredString {
    /// Offset to the string within its string pool.
    pub pool_offset: u32,
    /// Exact location of the offset that needs to be replaced.
    pub location: u32,
}

#[derive(Debug)]
pub struct DeferredData {}

#[derive(Debug)]
pub enum DeferredOffset {
    String(DeferredString),
    /// The strings in an `ARC` file have 24-bit offsets.
    ArcString(DeferredString),
    SectionData(DeferredData),
}

/// The deferred pass keeps track of which values need to be modified in
/// a second pass over the file.
///
/// This is required because the string and data offsets aren't known during the first pass.
pub struct DeferredPass {
    deferred: Vec<DeferredOffset>,
    strings: StringPool,
}

impl DeferredPass {
    pub fn new() -> Self {
        Self {
            deferred: Vec::new(),
            strings: StringPool::new(),
        }
    }

    #[inline]
    pub fn defer_arc_string<S: AsRef<str>>(
        &mut self,
        writer: &mut MutCursor,
        string: S,
    ) -> SlipstreamResult<()> {
        self.deferred
            .push(DeferredOffset::ArcString(DeferredString {
                location: writer.len() as u32,
                pool_offset: self.strings.insert(string.as_ref())?,
            }));

        writer.write_u24::<BigEndian>(DEFER_PLACEHOLDER24)?;

        Ok(())
    }

    #[inline]
    pub fn defer_string<'a, S: Into<&'a str>>(
        &mut self,
        writer: &mut MutCursor,
        string: S,
    ) -> SlipstreamResult<()> {
        self.deferred.push(DeferredOffset::String(DeferredString {
            location: writer.len() as u32,
            pool_offset: self.strings.insert(string.into())?,
        }));

        writer.write_u32::<BigEndian>(DEFER_PLACEHOLDER)?;

        Ok(())
    }
}

/// Keeps track of the current entries in the string pool.
///
/// This is only used for serialisation.
pub struct StringPool {
    /// Maps a string to an index into the string pool.
    strings: HashMap<String, u32>,
    buffer: MutCursor,
}

impl StringPool {
    pub fn new() -> Self {
        Self {
            strings: HashMap::new(),
            buffer: MutCursor::new(),
        }
    }

    /// Consumes the pool, returning the buffer that was created.
    #[inline]
    pub fn finish(self) -> MutCursor {
        self.buffer
    }

    /// Inserts a string into the pool.
    pub fn insert(&mut self, string: &str) -> SlipstreamResult<u32> {
        // Not using the entry API here to avoid unnecessary cloning of the string.
        Ok(match self.strings.get(string) {
            Some(i) => *i,
            None => {
                // Insert the string
                let index = self.buffer.len() as u32;

                // Add length prefix
                self.buffer.write_u32::<BigEndian>(string.len() as u32)?;
                self.buffer.write_all(string.as_bytes())?;
                self.buffer.write_u8(0)?; // Null terminator

                index
            }
        })
    }
}
