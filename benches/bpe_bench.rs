//! Criterion throughput benchmark: our byte-level BPE encode versus HuggingFace
//! `tokenizers` BPE encode on the same in-memory corpus.
//!
//! No external corpus file is required and no numbers are hard coded. Every
//! measurement is produced by Criterion at run time.

use std::collections::HashSet;
use std::hint::black_box;

use byte_bpe::Tokenizer;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tokenizers::models::bpe::{BpeTrainer, BPE};
use tokenizers::tokenizer::Trainer;

/// Target vocabulary size for both tokenizers.
const VOCAB_SIZE: usize = 512;

/// Deterministic corpus generated in code from public-domain-style ASCII text.
/// The same bytes are used for training and for both encode benchmarks.
fn build_corpus() -> Vec<u8> {
    let paragraph = b"the quick brown fox jumps over the lazy dog; \
                      pack my box with five dozen liquor jugs; \
                      now is the winter of our discontent made glorious summer; \
                      to be or not to be that is the question; \
                      all happy families are alike each unhappy family is unhappy in its own way; \
                      it was the best of times it was the worst of times; \
                      we hold these truths to be self evident that all men are created equal; \
                      four score and seven years ago our fathers brought forth on this continent;";
    let mut corpus = Vec::with_capacity(paragraph.len() * 512);
    for _ in 0..512 {
        corpus.extend_from_slice(paragraph);
    }
    corpus
}

/// Our tokenizer: train once, then benchmark `encode` over the whole corpus.
fn bench_byte_bpe(c: &mut Criterion, corpus: &[u8]) {
    let tokenizer = Tokenizer::train(corpus, VOCAB_SIZE);

    let mut group = c.benchmark_group("encode_throughput");
    group.throughput(Throughput::Bytes(corpus.len() as u64));
    group.bench_function(BenchmarkId::new("byte_bpe", VOCAB_SIZE), |b| {
        b.iter(|| {
            let ids = tokenizer.encode(black_box(corpus));
            black_box(ids)
        });
    });
    group.finish();
}

/// HuggingFace comparison group. Exact settings, mirrored in the README:
///   * model: `tokenizers::models::bpe::BPE::default()`
///   * trainer: `BpeTrainer::builder().vocab_size(512)
///       .initial_alphabet((0u8..=255).map(char::from).collect::<HashSet<char>>())
///       .show_progress(false).build()`
///   * special tokens: none
///   * normalizer / pre-tokenizer / decoder: none (plain BPE over Unicode scalars)
///   * corpus: the identical ASCII bytes, fed as whitespace separated words
///   * encode: `Tokenizer::encode(text, false)`, no special tokens added
///
/// Version note: `tokenizers` 0.21 removed the older
/// `Tokenizer::train_from_iterator` name. The dynamically typed
/// `tokenizers::Tokenizer` also uses an internal `ModelWrapper`, so the BPE
/// model is trained directly through the `Trainer` trait (`feed` then `train`)
/// and only then wrapped in a `Tokenizer`. This is the documented 0.21 path.
fn bench_huggingface_bpe(c: &mut Criterion, corpus: &[u8]) {
    let text = std::str::from_utf8(corpus).expect("the generated corpus is ASCII");

    let mut trainer = BpeTrainer::builder()
        .vocab_size(VOCAB_SIZE)
        .initial_alphabet((0u8..=255).map(char::from).collect::<HashSet<char>>())
        .show_progress(false)
        .build();
    trainer
        .feed(std::iter::once(text), |s| {
            Ok(s.split_whitespace().map(str::to_owned).collect())
        })
        .expect("HuggingFace trainer feed must succeed");

    let mut model = BPE::default();
    let special_tokens = trainer
        .train(&mut model)
        .expect("HuggingFace BPE training must succeed");
    let mut hf = tokenizers::Tokenizer::new(model);
    hf.add_special_tokens(&special_tokens);

    let mut group = c.benchmark_group("encode_throughput");
    group.throughput(Throughput::Bytes(corpus.len() as u64));
    group.bench_function(BenchmarkId::new("huggingface_bpe", VOCAB_SIZE), |b| {
        b.iter(|| {
            let encoding = hf.encode(black_box(text), false).expect("HF encode");
            black_box(encoding.get_ids().to_vec())
        });
    });
    group.finish();
}

fn benches(c: &mut Criterion) {
    let corpus = build_corpus();
    bench_byte_bpe(c, &corpus);
    bench_huggingface_bpe(c, &corpus);
}

criterion_group!(benches_entry, benches);
criterion_main!(benches_entry);
