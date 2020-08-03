use crate::hashes::marked::MarkedDigestWriter;
use sha3::{Digest, Sha3_256};
use std::io::{Result as IOResult, Write};

/// A sha3_256 digest.
pub type Sha3_256Digest = [u8; 32];

/// A struct that exposes a Sha3_256 `Write` interface.
///
/// ```
/// # use std::io::{Result};
/// use std::io::Write;
/// use coins_core::hashes::{Sha3_256Writer, MarkedDigestWriter};
///
/// # fn main() -> Result<()> {
/// let mut w = Sha3_256Writer::default();
/// # let data = [0u8; 32];
///
/// // Writing more than once will update the hasher.
/// w.write(&data)?;
///
/// // Call finish to consume the hasher and produce the digest.
/// let digest = w.finish();
/// # Ok(())
/// }
/// ```
pub struct Sha3_256Writer {
    internal: Sha3_256,
}

impl Default for Sha3_256Writer {
    fn default() -> Sha3_256Writer {
        Sha3_256Writer {
            internal: Sha3_256::new(),
        }
    }
}

impl Write for Sha3_256Writer {
    fn write(&mut self, buf: &[u8]) -> IOResult<usize> {
        self.internal.input(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> IOResult<()> {
        Ok(())
    }
}

impl MarkedDigestWriter<Sha3_256Digest> for Sha3_256Writer {
    fn finish(self) -> Sha3_256Digest {
        let result = self.internal.result();

        let mut digest = Sha3_256Digest::default();
        digest[..].copy_from_slice(&result[..]);
        digest
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ser::ByteFormat;

    #[test]
    fn it_hashes_sha3_256() {
        let mut w = Sha3_256Writer::default();
        w.write(&[00, 00]).unwrap();
        let result = w.finish();
        assert_eq!(
            result,
            Sha3_256Digest::deserialize_hex(
                "762ba6a3d9312bf3e6dc71e74f34208e889fc44e6ff400724deecfeda7d5b3ce"
            )
            .unwrap()
        );
    }
}

