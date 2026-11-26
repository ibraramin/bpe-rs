//! Short terminal demo used to record the README demo GIF.
//!
//! Run with: cargo run --release --example demo

use std::hint::black_box;
use std::time::Instant;

use byte_bpe::Tokenizer;

fn main() {
    let paragraph = b"the quick brown fox jumps over the lazy dog; \
                      pack my box with five dozen liquor jugs; \
                      now is the winter of our discontent made glorious summer;";
    let mut corpus = Vec::new();
    for _ in 0..512 {
        corpus.extend_from_slice(paragraph);
    }

    println!("byte-bpe demo");
    println!("corpus: {} bytes, vocab target 512", corpus.len());

    let tokenizer = Tokenizer::train(&corpus, 512);
    println!(
        "trained vocab: {} tokens ({} merges)",
        tokenizer.vocab_size(),
        tokenizer.vocab_size() - 256
    );

    let sample = b"the quick brown fox";
    let ids = tokenizer.encode(sample);
    let decoded = tokenizer
        .decode(&ids)
        .expect("encode output always decodes");
    println!("input:   {}", String::from_utf8_lossy(sample));
    println!("ids:     {:?}", ids);
    println!("decoded: {}", String::from_utf8_lossy(&decoded));
    assert_eq!(decoded, sample);

    let started = Instant::now();
    for _ in 0..3 {
        black_box(tokenizer.encode(black_box(&corpus)));
    }
    let elapsed = started.elapsed();
    let mib_per_s = (3.0 * corpus.len() as f64) / elapsed.as_secs_f64() / (1024.0 * 1024.0);
    println!(
        "encode:  {:.3} MiB/s ({} passes, this machine)",
        mib_per_s, 3
    );
    println!("roundtrip OK");
}
