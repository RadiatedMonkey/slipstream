use std::io::Read;

use byteorder::ReadBytesExt;
use slipstream_shared::{
    cursor::RefCursor,
    error::{CorruptionError, SlipstreamError, SlipstreamResult},
};

macro_rules! impl_byteorder_arrays {
    ($($ty: ty),*) => {
        paste::paste! {
            /// Extends the byteorder [`WriteBytesExt`] trait with reading methods for arrays of primitive types.
            ///
            /// [`WriteBytesExt`]: byteorder::WriteBytesExt
            pub trait WriteArrayExt: byteorder::WriteBytesExt {
                /// Writes a `u8` array of size `N` to the cursor.
                fn write_u8_array<const N: usize>(&mut self, values: [u8; N]) -> std::io::Result<()>;
                /// Writes a `i8` array of size `N` to the cursor.
                fn write_i8_array<const N: usize>(&mut self, values: [i8; N]) -> std::io::Result<()>;

                $(
                    #[doc = concat!("Writes a `", stringify!($ty), "` array of size `N` to the cursor.")]
                    fn [<write_ $ty _array>]<const N: usize, B: byteorder::ByteOrder>(&mut self, values: [$ty; N])
                        -> std::io::Result<()>;
                )*
            }

            impl<W: byteorder::WriteBytesExt> WriteArrayExt for W {
                fn write_u8_array<const N: usize>(&mut self, values: [u8; N]) -> std::io::Result<()> {
                    for b in values {
                        self.write_u8(b)?;
                    }

                    Ok(())
                }

                fn write_i8_array<const N: usize>(&mut self, values: [i8; N]) -> std::io::Result<()> {
                    for b in values {
                        self.write_i8(b)?;
                    }

                    Ok(())
                }

                $(
                    fn [< write_ $ty _array >]<const N: usize, B: byteorder::ByteOrder>(&mut self, values: [$ty; N])
                        -> std::io::Result<()>
                    {
                        for b in values {
                            self.[<write_ $ty>]::<B>(b)?;
                        }

                        Ok(())
                    }
                )*
            }

            pub trait ReadArrayExt: byteorder::ReadBytesExt {
                fn read_u8_array<const N: usize>(&mut self) -> std::io::Result<[u8; N]>;
                fn read_i8_array<const N: usize>(&mut self) -> std::io::Result<[i8; N]>;

                $(
                    fn [< read_ $ty _array >]<const N: usize, B: byteorder::ByteOrder>(&mut self)
                        -> std::io::Result<[$ty; N]>;
                )*
            }

            impl<R: byteorder::ReadBytesExt> ReadArrayExt for R {
                fn read_u8_array<const N: usize>(&mut self) -> std::io::Result<[u8; N]> {
                    let mut array = [0; N];
                    for i in 0..N {
                        array[i] = self.read_u8()?;
                    }

                    Ok(array)
                }

                fn read_i8_array<const N: usize>(&mut self) -> std::io::Result<[i8; N]> {
                    let mut array = [0; N];
                    for i in 0..N {
                        array[i] = self.read_i8()?;
                    }

                    Ok(array)
                }

                $(
                    fn [< read_ $ty _array >]<const N: usize, B: byteorder::ByteOrder>(&mut self)
                        -> std::io::Result<[$ty; N]>
                    {
                        let mut array = [Default::default(); N];
                        for i in 0..N {
                            array[i] = self.[< read_ $ty >]::<B>()?;
                        }

                        Ok(array)
                    }
                )*
            }
        }
    }
}

impl_byteorder_arrays!(u16, i16, u32, i32, u64, i64, u128, i128, f32, f64);

/// Extend the [`ReadBytesExt`] trait with extra functions for reading string .
pub trait ReadStringExt: ReadBytesExt {
    /// Reads a `String` with a `u32` length prefix.
    fn read_u32_string<B: byteorder::ByteOrder>(&mut self) -> SlipstreamResult<String>;

    /// Reads a `String` with a null terminator.
    fn read_null_string<B: byteorder::ByteOrder>(&mut self) -> SlipstreamResult<String>;
}

impl ReadStringExt for RefCursor<[u8]> {
    fn read_u32_string<B: byteorder::ByteOrder>(&mut self) -> SlipstreamResult<String> {
        let str_len = self.read_u32::<B>()?;
        let mut str_buf = vec![0; str_len as usize];
        self.read_exact(&mut str_buf)?;

        Ok(String::from_utf8(str_buf)?)
    }

    fn read_null_string<B: byteorder::ByteOrder>(&mut self) -> SlipstreamResult<String> {
        let rem = self.as_remaining();
        let null_pos = rem.iter().position(|&c| c == 0).ok_or_else(|| {
            SlipstreamError::from(CorruptionError {
                reason: "did not find string null terminator before EOF".to_owned(),
                ..Default::default()
            })
        })?;

        let mut str_buf = vec![0; null_pos];
        self.read_exact(&mut str_buf)?;

        Ok(String::from_utf8(str_buf)?)
    }
}

pub trait Deserialize: Sized {
    fn deserialize(reader: &mut RefCursor<[u8]>) -> SlipstreamResult<Self>;
}

pub trait Serialize {
    fn serialize(&self) -> SlipstreamResult<Vec<u8>> {
        let mut out = Vec::new();
        self.serialize_into(&mut out)?;

        Ok(out)
    }

    fn serialize_into(&self, writer: &mut Vec<u8>) -> SlipstreamResult<()>;
}
