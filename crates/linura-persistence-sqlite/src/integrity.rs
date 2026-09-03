use std::fmt::{Debug, Formatter};

use linura_transaction::{ContentDigest, TransactionStoreError, digest_bytes};
use sha2::{Digest, Sha256};

pub(crate) const INTEGRITY_KEY_BYTES: usize = 32;
pub(crate) const INTEGRITY_TAG_BYTES: usize = 32;
const SHA256_BLOCK_BYTES: usize = 64;

/// Independent secret used only to authenticate SQLite records.
///
/// Production composition roots must provision the same protected 256-bit
/// value on restart. SQLite stores only a domain-separated fingerprint and
/// per-record HMACs; neither reveals this key.
pub struct SqliteIntegrityKey {
    bytes: Vec<u8>,
}

impl Debug for SqliteIntegrityKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SqliteIntegrityKey")
            .field("fingerprint", &self.fingerprint())
            .finish_non_exhaustive()
    }
}

impl Drop for SqliteIntegrityKey {
    fn drop(&mut self) {
        self.bytes.fill(0);
    }
}

impl SqliteIntegrityKey {
    pub fn new(mut bytes: Vec<u8>) -> Result<Self, TransactionStoreError> {
        if bytes.len() != INTEGRITY_KEY_BYTES || bytes.iter().all(|byte| *byte == 0) {
            bytes.fill(0);
            return Err(TransactionStoreError::AuthorityRejected);
        }
        Ok(Self { bytes })
    }

    #[must_use]
    pub fn fingerprint(&self) -> ContentDigest {
        digest_bytes(
            "linura.sqlite.record-integrity-key-fingerprint.v1",
            &self.bytes,
        )
    }

    pub(crate) fn tag(
        &self,
        record_domain: &str,
        canonical: &[u8],
    ) -> Result<[u8; INTEGRITY_TAG_BYTES], TransactionStoreError> {
        Ok(hmac_sha256(
            &self.bytes,
            [
                b"linura.sqlite.record-integrity.v1".as_slice(),
                record_domain.as_bytes(),
                canonical,
            ],
        ))
    }

    #[must_use]
    pub(crate) fn verify(&self, record_domain: &str, canonical: &[u8], tag: &[u8]) -> bool {
        if tag.len() != INTEGRITY_TAG_BYTES {
            return false;
        }
        let Ok(expected) = self.tag(record_domain, canonical) else {
            return false;
        };
        constant_time_eq(&expected, tag)
    }
}

fn hmac_sha256<'a>(
    key: &[u8],
    fields: impl IntoIterator<Item = &'a [u8]>,
) -> [u8; INTEGRITY_TAG_BYTES] {
    let mut key_block = [0_u8; SHA256_BLOCK_BYTES];
    if key.len() > SHA256_BLOCK_BYTES {
        let digest = Sha256::digest(key);
        key_block[..INTEGRITY_TAG_BYTES].copy_from_slice(&digest);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut inner_pad = [0x36_u8; SHA256_BLOCK_BYTES];
    let mut outer_pad = [0x5c_u8; SHA256_BLOCK_BYTES];
    for index in 0..SHA256_BLOCK_BYTES {
        inner_pad[index] ^= key_block[index];
        outer_pad[index] ^= key_block[index];
    }

    let mut inner = Sha256::new();
    inner.update(inner_pad);
    for field in fields {
        inner.update(u64::try_from(field.len()).unwrap_or(u64::MAX).to_be_bytes());
        inner.update(field);
    }
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_digest);
    let digest = outer.finalize();

    let mut tag = [0_u8; INTEGRITY_TAG_BYTES];
    tag.copy_from_slice(&digest);
    key_block.fill(0);
    inner_pad.fill(0);
    outer_pad.fill(0);
    tag
}

fn constant_time_eq(expected: &[u8; INTEGRITY_TAG_BYTES], actual: &[u8]) -> bool {
    let mut difference = 0_u8;
    for (left, right) in expected.iter().zip(actual.iter()) {
        difference |= left ^ right;
    }
    difference == 0
}

pub(crate) fn canonical_field(buffer: &mut Vec<u8>, bytes: &[u8]) {
    buffer.extend_from_slice(&u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_be_bytes());
    buffer.extend_from_slice(bytes);
}

pub(crate) fn canonical_optional(buffer: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            canonical_field(buffer, &[1]);
            canonical_field(buffer, value.as_bytes());
        }
        None => canonical_field(buffer, &[0]),
    }
}
