//! The hasher of the VM's maps and sets (decision AW7): a folded
//! multiplication over the words of a key, keyed once per process, in
//! place of the standard library's SipHash, whose `write` alone cost a
//! hundred instructions and more for the one to twelve bytes of a typical
//! key (a name, a token's text, an Integer). The order of a map or a set
//! is the order of insertion (`IndexMap`), so the hash is not observable
//! by a program; `Value.hash()` (`vm::hash_of`) keeps the standard
//! hasher with its fixed keys, being a number a program reads.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::OnceLock;

use indexmap::{IndexMap, IndexSet};

use crate::value::Value;

/// A map from values to values in insertion order, hashed by [`ValueHasher`].
pub type ValueMap = IndexMap<Value, Value, ValueHashing>;

/// A set of values in insertion order, hashed by [`ValueHasher`].
pub type ValueSet = IndexSet<Value, ValueHashing>;

/// The [`BuildHasher`] of the VM's maps and sets: the process's key, read
/// once per map or set, from which every hasher starts.
#[derive(Clone, Copy, Debug)]
pub struct ValueHashing {
    key: u64,
}

impl Default for ValueHashing {
    fn default() -> ValueHashing {
        ValueHashing { key: key() }
    }
}

impl BuildHasher for ValueHashing {
    type Hasher = ValueHasher;

    #[inline]
    fn build_hasher(&self) -> ValueHasher {
        ValueHasher { state: self.key }
    }
}

/// The key of this process, random: keys chosen to collide in one run do
/// not collide in the next, the defence the standard hasher's keys give a
/// server against a flood of colliding keys, though a multiplication is
/// easier to reverse than SipHash once the key is known.
fn key() -> u64 {
    static KEY: OnceLock<u64> = OnceLock::new();
    *KEY.get_or_init(|| RandomState::new().hash_one(0x0052_454e_5949u64))
}

/// The hasher: one state word, every word written folded in by a
/// multiplication whose 128-bit product is folded to 64 bits, so that
/// every bit of the state depends on every bit written so far. A span of
/// bytes is read as two words (the first and the last eight, overlapping;
/// the first and last four, or the first, middle and last byte, of a
/// shorter span), sixteen at a time before that.
#[derive(Clone, Debug)]
pub struct ValueHasher {
    state: u64,
}

/// The multiplier of a word written whole: odd, so that the low half of
/// the product is a bijection of the state.
const WORD: u64 = 0x9e37_79b9_7f4a_7c15;

/// The seed of a span's second word, which multiplies its first.
const SPAN: u64 = 0xf135_7aea_2e62_a9c5;

#[inline(always)]
fn fold(a: u64, b: u64) -> u64 {
    let product = u128::from(a) * u128::from(b);
    (product as u64) ^ ((product >> 64) as u64)
}

#[inline(always)]
fn word_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}

#[inline(always)]
fn half_at(bytes: &[u8], at: usize) -> u64 {
    u64::from(u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()))
}

impl ValueHasher {
    #[inline(always)]
    fn word(&mut self, word: u64) {
        self.state = fold(self.state ^ word, WORD);
    }
}

impl Hasher for ValueHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.state
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let len = bytes.len();
        let (first, second) = if len > 16 {
            let mut rest = bytes;
            while rest.len() > 16 {
                self.state = fold(self.state ^ word_at(rest, 0), (word_at(rest, 8) ^ SPAN) | 1);
                rest = &rest[16..];
            }
            (word_at(bytes, len - 16), word_at(bytes, len - 8))
        } else if len >= 8 {
            (word_at(bytes, 0), word_at(bytes, len - 8))
        } else if len >= 4 {
            (half_at(bytes, 0), half_at(bytes, len - 4))
        } else if len > 0 {
            let last = u64::from(bytes[len - 1]);
            (u64::from(bytes[0]), (last << 8) | u64::from(bytes[len / 2]))
        } else {
            (0, 0)
        };
        // the length tells apart spans whose overlapping reads agree
        // ("aaaa" and "aaaaa"); the multiplier stays odd
        let multiplier = (second ^ SPAN ^ ((len as u64) << 1)) | 1;
        self.state = fold(self.state ^ first, multiplier);
    }

    #[inline]
    fn write_u8(&mut self, n: u8) {
        self.word(u64::from(n));
    }

    #[inline]
    fn write_u16(&mut self, n: u16) {
        self.word(u64::from(n));
    }

    #[inline]
    fn write_u32(&mut self, n: u32) {
        self.word(u64::from(n));
    }

    #[inline]
    fn write_u64(&mut self, n: u64) {
        self.word(n);
    }

    #[inline]
    fn write_u128(&mut self, n: u128) {
        self.word(n as u64);
        self.word((n >> 64) as u64);
    }

    #[inline]
    fn write_usize(&mut self, n: usize) {
        self.word(n as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash_of(text: &str) -> u64 {
        let mut hasher = ValueHashing::default().build_hasher();
        hasher.write(text.as_bytes());
        hasher.finish()
    }

    #[test]
    fn spans_whose_overlapping_reads_agree_hash_apart() {
        let spans: Vec<String> = (0..40).map(|length| "a".repeat(length)).collect();
        for (index, span) in spans.iter().enumerate() {
            for other in &spans[index + 1..] {
                assert_ne!(hash_of(span), hash_of(other), "{span:?} and {other:?}");
            }
        }
        assert_ne!(hash_of("abab"), hash_of("ababab"));
        assert_ne!(hash_of("list_a"), hash_of("list_b"));
        assert_ne!(hash_of(&"x".repeat(33)), hash_of(&("x".repeat(32) + "y")));
    }

    #[test]
    fn the_key_holds_for_the_process() {
        let one = ValueHashing::default().hash_one(Value::text("name"));
        let two = ValueHashing::default().hash_one(Value::text(String::from("name")));
        assert_eq!(one, two);
        assert_ne!(one, ValueHashing::default().hash_one(Value::integer(5)));
    }
}
