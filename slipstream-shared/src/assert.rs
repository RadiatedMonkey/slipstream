/// Asserts that the type implementing this trait is `Send`.
pub trait AssertSendSync: Send + Sync {}

/// Reads a `u8` from the given cursor and asserts that it matches the given pattern.
/// If not, it returns an [`SlipstreamError::AssertFailed`] error with the given message.
///
/// The return value of the macro is the byte read from the cursor.
#[macro_export]
macro_rules! assert_u8 {
    ($cursor:expr, $pattern:pat, $msg:expr) => {{
        use byteorder::ReadBytesExt;

        let byte = $cursor.read_u8()?;
        if !matches!(byte, $pattern) {
            return Err($crate::error::SlipstreamError::from(
                $crate::error::AssertFailed {
                    reason: format!(
                        "{} (value {byte:#04x} does not match pattern {}",
                        $msg,
                        stringify!($pattern)
                    ),
                    location: Some($cursor.position()),
                },
            ));
        }

        byte
    }};
}

#[macro_export]
macro_rules! verify {
    ($expr:expr, $msg:expr) => {
        if !$expr {
            return Err($crate::error::SlipstreamError::from(
                $crate::error::AssertFailed {
                    reason: format!($msg),
                    location: None,
                },
            ));
        }
    };

    ($expr:expr, $msg:expr, $($fmt:tt)+) => {
        if !$expr {
            return Err($crate::error::SlipstreamError::from(
                $crate::error::AssertFailed {
                    reason: format!($msg, $($fmt)+),
                    location: None,
                },
            ));
        }
    };
}
