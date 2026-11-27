#![cfg(feature = "simd")]

//! Correctness gates for the provisional SIMD candidate.
//!
//! The single-threaded `encode` is the unchanged reference. Every SIMD result
//! must be byte-identical to it and must decode back to the original bytes.

use std::sync::OnceLock;

use byte_bpe::Tokenizer;
use proptest::prelude::*;

const VOCAB_SIZE: usize = 512;
const CORPUS: &[u8] = include_bytes!("../results/corpus.txt");

fn shared_tokenizer() -> &'static Tokenizer {
    static TOKENIZER: OnceLock<Tokenizer> = OnceLock::new();
    TOKENIZER.get_or_init(|| Tokenizer::train(CORPUS, VOCAB_SIZE))
}

fn edge_inputs() -> Vec<Vec<u8>> {
    vec![
        Vec::new(),
        b"a".to_vec(),
        "naïve café — 日本語 🚀 — Ελληνικά".as_bytes().to_vec(),
        vec![0x00, 0x7f, 0x80, 0xc3, 0xa9, 0xff],
        (0..=255u8).collect(),
        vec![0xff; 1024],
        CORPUS.to_vec(),
    ]
}

#[test]
fn simd_ids_are_byte_identical_to_single_threaded() {
    let tokenizer = shared_tokenizer();
    for input in edge_inputs() {
        assert_eq!(
            tokenizer.encode_simd(&input),
            tokenizer.encode(&input),
            "SIMD encode diverged for input {:?}",
            &input[..input.len().min(64)]
        );
    }
}

#[test]
fn simd_encode_roundtrips_every_edge_input() {
    let tokenizer = shared_tokenizer();
    for input in edge_inputs() {
        let ids = tokenizer.encode_simd(&input);
        assert_eq!(
            tokenizer.decode(&ids).expect("ids from encode must decode"),
            input
        );
    }
}

#[test]
fn simd_ids_survive_save_and_load() {
    let tokenizer = shared_tokenizer();
    let mut path = std::env::temp_dir();
    path.push(format!("byte-bpe-simd-{}.bin", std::process::id()));
    tokenizer.save(&path).expect("save must succeed");
    let loaded = Tokenizer::from_file(&path).expect("load must succeed");
    let _ = std::fs::remove_file(&path);
    for input in edge_inputs() {
        assert_eq!(loaded.encode_simd(&input), tokenizer.encode(&input));
    }
}

proptest! {
    #[test]
    fn simd_matches_reference_for_arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..=512)) {
        let tokenizer = shared_tokenizer();
        prop_assert_eq!(tokenizer.encode_simd(&bytes), tokenizer.encode(&bytes));
        prop_assert_eq!(tokenizer.decode(&tokenizer.encode_simd(&bytes)).unwrap(), bytes);
    }
}
