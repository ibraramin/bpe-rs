//! Integration tests for the public `byte_bpe::Tokenizer` API.
//!
//! The tokenizer is byte level: all 256 byte values are present in the base
//! alphabet, so `decode(encode(input)) == input` must hold for arbitrary bytes,
//! including bytes that never appear in the training corpus.

use std::sync::OnceLock;

use byte_bpe::Tokenizer;
use proptest::prelude::*;

/// Vocabulary size used by every test. Large enough to learn a few merges from
/// the fixed corpus while keeping training fast.
const VOCAB_SIZE: usize = 512;

/// Fixed, deterministic, in-memory corpus. No files or network are involved.
fn fixed_corpus() -> Vec<u8> {
    let base = b"the quick brown fox jumps over the lazy dog; \
                 pack my box with five dozen liquor jugs; \
                 now is the winter of our discontent made glorious summer; \
                 to be or not to be that is the question; \
                 all happy families are alike each unhappy family is unhappy in its own way; \
                 it was the best of times it was the worst of times;";
    let mut corpus = Vec::with_capacity(base.len() * 128);
    for _ in 0..128 {
        corpus.extend_from_slice(base);
    }
    corpus
}

/// Train exactly once for the whole test binary and share the result across
/// proptest cases. Re-training per generated case would dominate runtime.
fn shared_tokenizer() -> &'static Tokenizer {
    static TOKENIZER: OnceLock<Tokenizer> = OnceLock::new();
    TOKENIZER.get_or_init(|| Tokenizer::train(&fixed_corpus(), VOCAB_SIZE))
}

#[test]
fn roundtrip_sample_bytes() {
    let tokenizer = Tokenizer::train(&fixed_corpus(), VOCAB_SIZE);
    let samples: [&[u8]; 5] = [
        b"the quick brown fox jumps over the lazy dog",
        b"hello world, 12345!",
        &[0x00, 0x01, 0x7f, 0x80, 0xfe, 0xff],
        b"unseen \x00\xff combinations \x80\x81",
        b"",
    ];

    for sample in samples {
        let ids = tokenizer.encode(sample);
        let decoded = tokenizer
            .decode(&ids)
            .expect("ids produced by encode must always decode");
        assert_eq!(decoded, sample);
    }
}

proptest! {
    /// Property: for any byte string up to 256 bytes, encode then decode is the
    /// identity. The tokenizer is trained once via `shared_tokenizer`, not per
    /// generated case.
    #[test]
    fn roundtrip_arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..=256)) {
        let tokenizer = shared_tokenizer();
        let ids = tokenizer.encode(&bytes);
        let decoded = tokenizer
            .decode(&ids)
            .expect("byte-level encode must always produce decodable ids");
        prop_assert_eq!(decoded, bytes);
    }
}

#[test]
fn edge_cases() {
    let tokenizer = Tokenizer::train(&fixed_corpus(), VOCAB_SIZE);

    // Empty input encodes to no tokens and decodes back to no bytes.
    let empty_ids = tokenizer.encode(b"");
    assert!(empty_ids.is_empty());
    assert_eq!(tokenizer.decode(&empty_ids).unwrap(), Vec::<u8>::new());

    // A long run of the maximum byte value.
    let high_run = vec![0xff_u8; 255];
    let ids = tokenizer.encode(&high_run);
    assert_eq!(tokenizer.decode(&ids).unwrap(), high_run);

    // All 256 distinct byte values in ascending order.
    let every_byte: Vec<u8> = (0..=255).collect();
    let ids = tokenizer.encode(&every_byte);
    assert_eq!(tokenizer.decode(&ids).unwrap(), every_byte);

    // Low and high control bytes interleaved with corpus-like ASCII.
    let mixed: Vec<u8> = (0..=255).flat_map(|b| [b, b'a', b]).collect();
    let ids = tokenizer.encode(&mixed);
    assert_eq!(tokenizer.decode(&ids).unwrap(), mixed);
}

#[test]
fn out_of_range_decode_id_is_an_error() {
    let tokenizer = Tokenizer::train(&fixed_corpus(), VOCAB_SIZE);

    // One past the last valid id and an extreme value must both be rejected.
    let just_past = tokenizer.vocab_size() as u32;
    assert!(tokenizer.decode(&[just_past]).is_err());
    assert!(tokenizer.decode(&[u32::MAX]).is_err());
    assert!(tokenizer.decode(&[0, just_past, 1]).is_err());
}

/// Removes the temp file even when an assertion inside the test panics.
struct TempFile(std::path::PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn save_and_load_preserves_vocab_and_encodings() {
    let tokenizer = Tokenizer::train(&fixed_corpus(), VOCAB_SIZE);

    // Unique per process and per call so parallel tests cannot collide.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_nanos();
    let mut path = std::env::temp_dir();
    path.push(format!(
        "byte-bpe-roundtrip-{}-{}.bin",
        std::process::id(),
        nonce
    ));
    let _cleanup = TempFile(path.clone());

    tokenizer.save(&path).expect("save must succeed");
    let loaded = Tokenizer::from_file(&path).expect("load must succeed");

    assert_eq!(loaded.vocab_size(), tokenizer.vocab_size());

    let samples: [&[u8]; 4] = [
        b"the quick brown fox",
        b"a string with characters not in the corpus: \x00\xff\x80",
        &[0xff; 255],
        b"",
    ];
    for sample in samples {
        assert_eq!(loaded.encode(sample), tokenizer.encode(sample));
    }
}
