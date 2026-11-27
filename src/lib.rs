//! # byte-bpe
//!
//! A small, dependency-free implementation of byte-level Byte Pair Encoding (BPE).
//!
//! `byte-bpe` trains a vocabulary from raw bytes, encodes byte slices into token
//! ids, decodes ids back into bytes, and round-trips vocabularies and merge rules
//! through a simple plain-text format. It is meant to be a readable, self-contained
//! baseline for comparison with production tokenizers such as HuggingFace
//! `tokenizers`.
//!
//! ## Byte alphabet
//!
//! Token ids `0..=255` always denote the single byte with the same value, so all
//! 256 byte values are representable and encoding bytes can never fail. Training
//! may append further tokens; each new token is the concatenation of two existing
//! tokens and receives the next free id.
//!
//! ## Example
//!
//! ```
//! use byte_bpe::Tokenizer;
//!
//! let corpus = b"the quick brown fox jumps over the lazy dog";
//! let tokenizer = Tokenizer::train(corpus, 300);
//!
//! let ids = tokenizer.encode(b"the fox");
//! let decoded = tokenizer.decode(&ids).unwrap();
//! assert_eq!(decoded.as_slice(), &b"the fox"[..]);
//! assert!(tokenizer.vocab_size() >= 256);
//! ```
//!
//! ## Optional features
//!
//! No features are enabled by default; [`Tokenizer::encode`] is the original
//! single-threaded reference path in every build. The provisional `simd` feature
//! adds `Tokenizer::encode_simd` as a falsifiable experiment; see `results/` for
//! its pre-registered criteria and measured outcome. A parallel-scan candidate
//! was measured and reverted because it showed no reproducible win.
//!
//! ## Serialization format
//!
//! [`Tokenizer::save`] writes and [`Tokenizer::from_file`] reads this format:
//!
//! 1. One line holding the vocabulary size `N` as a decimal integer.
//! 2. Exactly `N` lines, each containing one token's bytes as lowercase
//!    hexadecimal with no separators (a blank line denotes empty bytes).
//! 3. Zero or more lines, one per merge rule, each of the form `LEFT RIGHT`
//!    with decimal token ids, in priority order (earliest rule first).

use std::collections::HashMap;
use std::io;
use std::path::Path;

/// A trained byte-level BPE tokenizer.
#[derive(Debug, Clone)]
pub struct Tokenizer {
    /// Token id to token bytes. Ids `0..=255` are the individual bytes.
    vocab: Vec<Vec<u8>>,
    /// Merge rules in creation (priority) order. Rule at index `r` creates id `256 + r`.
    merges: Vec<(u32, u32)>,
}

impl Tokenizer {
    /// Train a byte-level BPE from `corpus` until the vocabulary reaches `vocab_size`.
    ///
    /// The minimum vocabulary size is 256 (the byte alphabet); a smaller request is
    /// treated as 256. Training is deterministic: the same corpus and vocabulary
    /// size always produce the same vocabulary and merge rules. When several pairs
    /// share the highest frequency, the pair with the lowest `(left, right)` ids wins.
    pub fn train(corpus: &[u8], vocab_size: usize) -> Self {
        let target = vocab_size.max(256);

        let mut vocab: Vec<Vec<u8>> = (0..=255u8).map(|byte| vec![byte]).collect();
        let mut merges: Vec<(u32, u32)> = Vec::new();
        let mut ids: Vec<u32> = corpus.iter().map(|&byte| u32::from(byte)).collect();

        while vocab.len() < target {
            let mut counts: HashMap<(u32, u32), usize> = HashMap::new();
            for pair in ids.windows(2) {
                *counts.entry((pair[0], pair[1])).or_insert(0) += 1;
            }

            // Scan explicitly instead of relying on HashMap iteration order so the
            // frequency tie-break is fully deterministic.
            let mut best: Option<(u32, u32)> = None;
            let mut best_count = 0usize;
            for (&pair, &count) in &counts {
                let better = match best {
                    None => true,
                    Some(current) => count > best_count || (count == best_count && pair < current),
                };
                if better {
                    best = Some(pair);
                    best_count = count;
                }
            }

            let (left, right) = match best {
                Some(pair) => pair,
                None => break, // no adjacent pairs remain
            };

            let new_id = vocab.len() as u32;
            let mut token =
                Vec::with_capacity(vocab[left as usize].len() + vocab[right as usize].len());
            token.extend_from_slice(&vocab[left as usize]);
            token.extend_from_slice(&vocab[right as usize]);
            vocab.push(token);
            merges.push((left, right));

            ids = apply_merge(&ids, left, right, new_id);
        }

        Tokenizer { vocab, merges }
    }

    /// Encode bytes into token ids using the learned merge rules in priority order.
    ///
    /// On each pass the applicable merge with the lowest rank (earliest in the
    /// training order) is applied to all of its non-overlapping occurrences at once.
    /// A merge can only introduce a token created after all earlier rules, so no
    /// lower-rank rule can become applicable later and the result is deterministic.
    pub fn encode(&self, input: &[u8]) -> Vec<u32> {
        let mut ids: Vec<u32> = input.iter().map(|&byte| u32::from(byte)).collect();
        let ranks: HashMap<(u32, u32), (usize, u32)> = self
            .merges
            .iter()
            .enumerate()
            .map(|(rank, &(left, right))| ((left, right), (rank, 256 + rank as u32)))
            .collect();

        loop {
            let mut best_rank = usize::MAX;
            let mut best: Option<(u32, u32, u32)> = None; // (left, right, new_id)
            for pair in ids.windows(2) {
                if let Some(&(rank, new_id)) = ranks.get(&(pair[0], pair[1])) {
                    if rank < best_rank {
                        best_rank = rank;
                        best = Some((pair[0], pair[1], new_id));
                    }
                }
            }

            match best {
                Some((left, right, new_id)) => ids = apply_merge(&ids, left, right, new_id),
                None => break,
            }
        }

        ids
    }

    /// SIMD candidate: an SSE2-accelerated minimum-rank scan over a flat
    /// `(left, right)` lookup table. Compiled only with the provisional `simd`
    /// feature and retained only if the pre-registered benchmark win is measured
    /// (see `results/preregistration.md` and `results/preregistration-reconciliation.md`).
    #[cfg(feature = "simd")]
    pub fn encode_simd(&self, input: &[u8]) -> Vec<u32> {
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = input;
            self.encode(input)
        }
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::*;

            let mut ids: Vec<u32> = input.iter().map(|&byte| u32::from(byte)).collect();
            if ids.len() < 2 {
                return ids;
            }

            let v = self.vocab_size();
            let table_len = match v.checked_mul(v) {
                Some(len) if len <= 1 << 24 => len,
                // Refuse an unreasonable allocation: fall back to the reference path.
                _ => return self.encode(input),
            };
            let mut table = vec![u32::MAX; table_len];
            for (rank, &(left, right)) in self.merges.iter().enumerate() {
                table[left as usize * v + right as usize] = rank as u32;
            }

            loop {
                let n = ids.len();
                let mut best = u32::MAX;
                let mut i = 0usize;

                // Four windows per iteration. SSE2 is baseline on x86_64, so no
                // target-feature attribute or CPU ID check is required.
                while i + 4 < n {
                    unsafe {
                        let left = _mm_loadu_si128(ids.as_ptr().add(i) as *const __m128i);
                        let right = _mm_loadu_si128(ids.as_ptr().add(i + 1) as *const __m128i);
                        let idx = _mm_add_epi32(
                            _mm_mullo_epi32(left, _mm_set1_epi32(v as i32)),
                            right,
                        );
                        let mut slots = [0u32; 4];
                        _mm_storeu_si128(slots.as_mut_ptr() as *mut __m128i, idx);
                        // SSE2 has no integer gather; table reads remain scalar.
                        let ranks = _mm_setr_epi32(
                            table[slots[0] as usize] as i32,
                            table[slots[1] as usize] as i32,
                            table[slots[2] as usize] as i32,
                            table[slots[3] as usize] as i32,
                        );
                        let paired =
                            _mm_min_epu32(ranks, _mm_shuffle_epi32(ranks, 0b10_11_00_01));
                        let min4 =
                            _mm_min_epu32(paired, _mm_shuffle_epi32(paired, 0b00_00_10_10));
                        let candidate = _mm_cvtsi128_si32(min4) as u32;
                        if candidate < best {
                            best = candidate;
                        }
                    }
                    i += 4;
                }
                while i + 1 < n {
                    let candidate = table[ids[i] as usize * v + ids[i + 1] as usize];
                    if candidate < best {
                        best = candidate;
                    }
                    i += 1;
                }

                if best == u32::MAX {
                    break;
                }
                let (left, right) = self.merges[best as usize];
                ids = apply_merge(&ids, left, right, 256 + best);
            }

            ids
        }
    }

    /// Decode token ids back into bytes.
    ///
    /// Returns `Err` when any id is greater than or equal to [`Tokenizer::vocab_size`].
    pub fn decode(&self, ids: &[u32]) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        for &id in ids {
            match self.vocab.get(id as usize) {
                Some(token) => out.extend_from_slice(token),
                None => {
                    return Err(format!(
                        "token id {} is out of range (vocabulary size {})",
                        id,
                        self.vocab.len()
                    ))
                }
            }
        }
        Ok(out)
    }

    /// Save the vocabulary and merge rules to `path` as plain text.
    ///
    /// See the [crate-level documentation](crate) for the exact format.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let mut text = format!("{}\n", self.vocab.len());
        for token in &self.vocab {
            text.push_str(&hex_encode(token));
            text.push('\n');
        }
        for &(left, right) in &self.merges {
            text.push_str(&format!("{} {}\n", left, right));
        }
        std::fs::write(path, text)
    }

    /// Load a tokenizer previously written by [`Tokenizer::save`].
    ///
    /// Yields [`io::ErrorKind::InvalidData`] with a descriptive message if the file
    /// does not follow the documented format.
    pub fn from_file(path: &Path) -> io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        parse(&text).map_err(|message| io::Error::new(io::ErrorKind::InvalidData, message))
    }

    /// Return the number of tokens in the vocabulary.
    pub fn vocab_size(&self) -> usize {
        self.vocab.len()
    }
}

/// Replace every non-overlapping occurrence of adjacent ids `(left, right)` with
/// `new_id`, scanning from left to right.
fn apply_merge(ids: &[u32], left: u32, right: u32, new_id: u32) -> Vec<u32> {
    let mut out = Vec::with_capacity(ids.len());
    let mut index = 0;
    while index < ids.len() {
        if index + 1 < ids.len() && ids[index] == left && ids[index + 1] == right {
            out.push(new_id);
            index += 2;
        } else {
            out.push(ids[index]);
            index += 1;
        }
    }
    out
}

/// Encode bytes as lowercase hexadecimal without separators.
fn hex_encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Decode hexadecimal without separators into bytes.
fn hex_decode(text: &str) -> Result<Vec<u8>, String> {
    if !text.len().is_multiple_of(2) {
        return Err(format!("hex token has odd length: {:?}", text));
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    // `as_chunks` is stable and avoids clippy's newer `chunks_exact` lint.
    for pair in text.as_bytes().as_chunks::<2>().0 {
        let high =
            hex_value(pair[0]).ok_or_else(|| format!("invalid hex digit in token: {:?}", text))?;
        let low =
            hex_value(pair[1]).ok_or_else(|| format!("invalid hex digit in token: {:?}", text))?;
        out.push((high << 4) | low);
    }
    Ok(out)
}

/// Value of one hexadecimal digit, accepting both cases.
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Parse a decimal token id from a merge line field.
fn parse_id(field: Option<&str>, label: &str) -> Result<u32, String> {
    let raw = field.ok_or_else(|| format!("missing {}", label))?;
    raw.parse()
        .map_err(|_| format!("invalid {}: {:?}", label, raw))
}

/// Parse the documented plain-text representation into a tokenizer.
fn parse(text: &str) -> Result<Tokenizer, String> {
    let mut lines = text.lines();

    let size_line = lines.next().ok_or("empty file: missing vocabulary size")?;
    let size: usize = size_line
        .trim()
        .parse()
        .map_err(|_| format!("invalid vocabulary size: {:?}", size_line))?;
    if size < 256 {
        return Err(format!(
            "vocabulary size must be at least 256, found {}",
            size
        ));
    }

    let mut vocab = Vec::with_capacity(size);
    for index in 0..size {
        let line = lines
            .next()
            .ok_or_else(|| format!("unexpected end of file: missing token {}", index))?;
        vocab.push(hex_decode(line)?);
    }

    let mut merges = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        let left = parse_id(fields.next(), "left merge id")?;
        let right = parse_id(fields.next(), "right merge id")?;
        if fields.next().is_some() {
            return Err(format!("malformed merge line: {:?}", line));
        }
        if left as usize >= size || right as usize >= size {
            return Err(format!(
                "merge id out of range in line {:?} (vocabulary size {})",
                line, size
            ));
        }
        merges.push((left, right));
    }

    // Each merge must be consistent with the vocabulary token it creates. This
    // catches corrupted hand-edited files before they can produce bad encodings.
    for (rank, &(left, right)) in merges.iter().enumerate() {
        let new_id = 256 + rank;
        if new_id >= size {
            return Err(format!("too many merge rules for vocabulary size {}", size));
        }
        let mut expected =
            Vec::with_capacity(vocab[left as usize].len() + vocab[right as usize].len());
        expected.extend_from_slice(&vocab[left as usize]);
        expected.extend_from_slice(&vocab[right as usize]);
        if vocab[new_id] != expected {
            return Err(format!(
                "merge {} ({}, {}) does not match vocabulary token {}",
                rank, left, right, new_id
            ));
        }
    }

    Ok(Tokenizer { vocab, merges })
}
