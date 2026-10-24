//! SHA-256 of each 128 KiB window of a PDF object.
//!
//! A viewer range is one of these windows. The digest is stored at copy time
//! so a slice can be checked without reading the rest of the file.

use sha2::{Digest, Sha256};

/// Same width as the PDF viewer's range window.
pub const PDF_CHUNK_BYTES: u64 = 131_072;

pub fn chunk_digests(bytes: &[u8], chunk: u64) -> Vec<[u8; 32]> {
    if chunk == 0 || bytes.is_empty() {
        return Vec::new();
    }
    let width = chunk as usize;
    let mut out = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        let end = (offset + width).min(bytes.len());
        out.push(sha256_32(&bytes[offset..end]));
        offset = end;
    }
    out
}

pub fn pack_digests(digests: &[[u8; 32]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(digests.len() * 32);
    for digest in digests {
        out.extend_from_slice(digest);
    }
    out
}

pub fn digest_at(packed: &[u8], index: u64) -> Option<&[u8]> {
    let start = (index as usize).checked_mul(32)?;
    packed.get(start..start + 32)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkCover {
    pub index: u64,
    pub start: u64,
    pub end: u64,
}

/// Inclusive windows that cover `start..=end`.
pub fn chunks_covering(start: u64, end: u64, total: u64, chunk: u64) -> Option<Vec<ChunkCover>> {
    if chunk == 0 || total == 0 || end < start || end >= total {
        return None;
    }
    let first = start / chunk;
    let last = end / chunk;
    let mut out = Vec::new();
    for index in first..=last {
        let chunk_start = index.checked_mul(chunk)?;
        if chunk_start >= total {
            return None;
        }
        let chunk_end = chunk_start.saturating_add(chunk - 1).min(total - 1);
        out.push(ChunkCover {
            index,
            start: chunk_start,
            end: chunk_end,
        });
    }
    Some(out)
}

pub fn chunk_matches(bytes: &[u8], digest: &[u8]) -> bool {
    digest.len() == 32 && sha256_32(bytes) == digest
}

fn sha256_32(bytes: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_short_body_is_one_digest() {
        let body = b"hello-pdf";
        let packed = pack_digests(&chunk_digests(body, 8));
        assert_eq!(packed.len(), 64);
        assert!(chunk_matches(&body[..8], digest_at(&packed, 0).unwrap()));
        assert!(chunk_matches(&body[8..], digest_at(&packed, 1).unwrap()));
        assert!(!chunk_matches(b"hello-pX", digest_at(&packed, 0).unwrap()));
    }

    #[test]
    fn a_middle_window_names_its_chunk() {
        let covers = chunks_covering(8, 18, 20, 8).unwrap();
        assert_eq!(
            covers,
            vec![
                ChunkCover {
                    index: 1,
                    start: 8,
                    end: 15
                },
                ChunkCover {
                    index: 2,
                    start: 16,
                    end: 19
                }
            ]
        );
        assert!(chunks_covering(0, 20, 20, 8).is_none());
    }
}
