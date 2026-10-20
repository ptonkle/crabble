//! The word list: whether a run of letters is a real word (GLOSSARY.md,
//! Real word). The list is ENABLE, unchanged (public domain; LICENSES.md;
//! the person, CR-Q11: "don't include beyond what is in ENABLE").
//!
//! `data/words/enable1.bin` is the list front coded in blocks by
//! `tools/words/pack.py` (its layout is written there). It is built into
//! the module and searched where it lies: a binary search over the blocks'
//! first words, then a walk through one block. Nothing is parsed at load
//! and nothing is kept.

const PACKED: &[u8] = include_bytes!("../../../data/words/enable1.bin");

/// The longest word the packed form can hold (ENABLE's longest is 28).
const LONGEST: usize = 31;

/// A shared-letters byte is below this; a letter is at or above it.
const FIRST_LETTER: u8 = b'a';

fn u32_at(at: usize) -> usize {
    let b = &PACKED[at..at + 4];
    u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize
}

/// How many words the list holds.
pub fn count() -> usize {
    u32_at(4)
}

fn blocks() -> usize {
    u32_at(12)
}

fn body() -> &'static [u8] {
    &PACKED[16 + 4 * blocks()..]
}

/// Where block `k` starts in the body.
fn block_start(k: usize) -> usize {
    u32_at(16 + 4 * k)
}

/// Walks a stretch of the body word by word, each word in lowercase.
struct Walk {
    at: usize,
    end: usize,
    word: [u8; LONGEST],
    len: usize,
}

impl Walk {
    fn block(k: usize) -> Walk {
        let end = if k + 1 < blocks() {
            block_start(k + 1)
        } else {
            body().len()
        };
        Walk {
            at: block_start(k),
            end,
            word: [0; LONGEST],
            len: 0,
        }
    }

    fn next(&mut self) -> Option<&[u8]> {
        if self.at >= self.end {
            return None;
        }
        let body = body();
        let mut len = body[self.at] as usize;
        self.at += 1;
        while self.at < self.end && body[self.at] >= FIRST_LETTER {
            self.word[len] = body[self.at];
            len += 1;
            self.at += 1;
        }
        self.len = len;
        Some(&self.word[..len])
    }
}

/// Whether `word` is a real word: two or more capital letters A to Z that
/// ENABLE lists. Anything else (a lowercase letter, a space, one letter) is
/// not.
pub fn is_real(word: &str) -> bool {
    let bytes = word.as_bytes();
    if bytes.len() < 2 || bytes.len() > LONGEST || !bytes.iter().all(u8::is_ascii_uppercase) {
        return false;
    }
    let mut key = [0u8; LONGEST];
    for (k, b) in bytes.iter().enumerate() {
        key[k] = b.to_ascii_lowercase();
    }
    let key = &key[..bytes.len()];
    // the last block whose first word is at or before the key
    let (mut lo, mut hi) = (0, blocks());
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        let mut w = Walk::block(mid);
        match w.next() {
            Some(first) if first <= key => lo = mid,
            _ => hi = mid,
        }
    }
    let mut w = Walk::block(lo);
    while let Some(next) = w.next() {
        if next == key {
            return true;
        }
        if next > key {
            return false;
        }
    }
    false
}

/// Every word in the list, in order, in capitals (for tests and tools).
pub fn all() -> Vec<String> {
    let mut out = Vec::with_capacity(count());
    for k in 0..blocks() {
        let mut w = Walk::block(k);
        while let Some(word) = w.next() {
            out.push(word.iter().map(|b| b.to_ascii_uppercase() as char).collect());
        }
    }
    out
}

#[cfg(test)]
mod tests;
