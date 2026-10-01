use std::{
    io::{self, SeekFrom, Write},
    sync::Arc,
};

/// Estimates the size of this type when serialized into a buffer.
/// This is used to preallocate.
pub trait SizeEstimate {
    fn estimate_size(&self) -> usize;
}

#[derive(Default)]
pub struct MutCursor {
    inner: Vec<u8>,
}

impl MutCursor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.inner
    }

    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl Write for MutCursor {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_all(buf)?;
        Ok(buf.len())
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.inner.extend_from_slice(buf);
        Ok(())
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        let mut written = 0;
        for buf in bufs {
            self.write_all(buf)?;
            written += buf.len();
        }
        Ok(written)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A `RangedCursor` is very similar to the std's [`Cursor`]
/// but instead stores its contents in a reference counter.
///
/// This allows the cursor to very cheaply be cloned.
///
/// [`Cursor`]: std::io::Cursor
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RefCursor<T: ?Sized> {
    inner: Arc<T>,
    /// The current position of the cursor.
    pos: u64,
    lower_bound: u64,
}

impl<T: ?Sized> RefCursor<T> {
    pub fn new(inner: Arc<T>) -> Self {
        Self {
            inner,
            pos: 0,
            lower_bound: 0,
        }
    }

    pub fn into_inner(self) -> Arc<T> {
        self.inner
    }

    /// Returns a cursor that only reads the remaining bytes.
    pub fn tail(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            pos: 0,
            lower_bound: self.position(),
        }
    }

    /// Cuts everything left of the cursor. The underlying buffer will not be modified, but
    /// the cursor will not see any of the contents anymore.
    ///
    /// The position is reset back to 0.
    pub fn set_tail(&mut self) {
        self.lower_bound = self.position();
        self.pos = 0;
    }

    /// The current position of the cursor.
    pub fn position(&self) -> u64 {
        self.pos
    }

    /// Sets the current position of the cursor.
    ///
    /// This position is relative to the start of the cursor slice.
    pub fn set_position(&mut self, pos: u64) {
        self.pos = pos;
    }
}

impl<T: AsRef<[u8]> + ?Sized> RefCursor<T> {
    /// Returns a reference to the bytes remaining in this buffer.
    ///
    /// I.e this buffer will be the bytes in the range `pos...range.end`.
    ///
    /// If the cursor is past the end of the buffer, the remaining buffer will be empty.
    pub fn remaining(&self) -> &[u8] {
        &self.inner.as_ref().as_ref()[(self.pos + self.lower_bound) as usize..]
    }

    /// Returns the length of the entire underlying buffer that this cursor has a view into.
    pub fn full_len(&self) -> usize {
        self.inner.as_ref().as_ref().len()
    }

    /// The length of the remaining buffer.
    pub fn remaining_len(&self) -> usize {
        self.remaining().len()
    }

    /// Dumps all contents to the given file in binary format.
    pub fn dump<P: AsRef<std::path::Path>>(&self, path: P) -> io::Result<()> {
        std::fs::write(path.as_ref(), self.inner.as_ref())
    }
}

impl<T: ?Sized> Clone for RefCursor<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            pos: self.pos,
            lower_bound: self.lower_bound,
        }
    }
}

/// This is a nearly exact copy of the standard library.
impl<T> io::Read for RefCursor<T>
where
    T: AsRef<[u8]> + ?Sized,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let rem = self.remaining();
        let n = std::cmp::min(buf.len(), rem.len());

        buf[..n].copy_from_slice(&rem[..n]);
        self.set_position(self.position() + n as u64);

        Ok(n)
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        let mut nread = 0;
        for buf in bufs {
            let n = self.read(buf)?;
            nread += n;
            if n < buf.len() {
                break;
            }
        }

        Ok(nread)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let rem = self.remaining();
        let n = buf.len();

        if rem.len() < n {
            // Set cursor to EOF
            self.set_position(self.inner.as_ref().as_ref().len() as u64);
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }

        buf.copy_from_slice(&rem[..n]);
        self.set_position(self.position() + buf.len() as u64);

        Ok(())
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        let rem = self.remaining();
        let n = rem.len();

        buf.reserve(n);
        buf.extend_from_slice(rem);

        self.set_position(self.position() + n as u64);

        Ok(n)
    }

    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        let rem = self.remaining();
        let n = rem.len();

        let content = str::from_utf8(rem)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid utf-8"))?;

        buf.reserve(n);
        buf.push_str(content);

        self.set_position(self.position() + n as u64);

        Ok(n)
    }
}

/// This is a nearly exact copy of the standard library.
impl<T> io::Seek for RefCursor<T>
where
    T: AsRef<[u8]> + ?Sized,
{
    fn seek(&mut self, style: SeekFrom) -> io::Result<u64> {
        let (base_pos, offset) = match style {
            SeekFrom::Start(n) => {
                self.set_position(n);
                return Ok(n);
            }
            SeekFrom::End(n) => (self.inner.as_ref().as_ref().len() as u64, n),
            SeekFrom::Current(n) => (self.position(), n),
        };

        match base_pos.checked_add_signed(offset) {
            Some(n) => {
                self.set_position(n);
                Ok(n)
            }
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid seek to a negative or overflowing position",
            )),
        }
    }

    fn stream_position(&mut self) -> io::Result<u64> {
        Ok(self.position())
    }
}
