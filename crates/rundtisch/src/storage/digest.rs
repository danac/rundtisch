use std::fmt;

/// Raw 32-byte BLAKE3 digest. Also the source of an object's ETag.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Blake3Digest([u8; 32]);

impl Blake3Digest {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn hash(bytes: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(bytes);
        Self(*hasher.finalize().as_bytes())
    }

    pub fn to_hex(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(64);
        for byte in self.0 {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
        out
    }

    /// Strong ETag, quoted hex of the digest.
    pub fn etag(&self) -> String {
        format!("\"{}\"", self.to_hex())
    }

    pub(crate) fn from_slice(bytes: &[u8]) -> Result<Self, super::StoreError> {
        let array: [u8; 32] = bytes.try_into().map_err(|_| {
            super::StoreError::InvalidInput("blake3 digest must be 32 bytes".into())
        })?;
        Ok(Self(array))
    }
}

impl fmt::Debug for Blake3Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Blake3Digest({})", self.to_hex())
    }
}

impl fmt::Display for Blake3Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[cfg(test)]
pub(crate) fn sha256(bytes: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etag_is_quoted_hex() {
        let digest = Blake3Digest::hash(b"crockis");
        assert_eq!(digest.as_bytes().len(), 32);
        assert_eq!(digest.etag(), format!("\"{}\"", digest.to_hex()));
        assert_eq!(digest, Blake3Digest::hash(b"crockis"));
        assert_ne!(digest, Blake3Digest::hash(b"other"));
    }
}
