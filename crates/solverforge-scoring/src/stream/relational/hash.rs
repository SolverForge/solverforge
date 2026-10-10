/* Deterministic fast hashing for engine-internal index keys.

Index keys are minted by the engine itself (`RowHandle` slots, source
indexes, descriptor ids) or read from planning entities; they are never
adversarial input, so the DoS-resistant seeded default buys nothing on the
event path and costs a full SipHash round per probe. The default also
randomises bucket order per process, which an engine advertising a
reproducible environment mode cannot want.

Integer writes take a multiply-xor round; opaque keys such as `String`
codes fall back to byte-wise FNV-1a.
*/

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

pub(crate) type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<FastHasher>>;
pub(crate) type FastSet<T> = HashSet<T, BuildHasherDefault<FastHasher>>;

#[derive(Default)]
pub(crate) struct FastHasher(u64);

impl FastHasher {
    #[inline]
    fn mix(&mut self, value: u64) {
        self.0 = (self.0 ^ value).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        self.0 ^= self.0 >> 29;
    }
}

impl Hasher for FastHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    #[inline]
    fn write_u8(&mut self, value: u8) {
        self.mix(u64::from(value));
    }

    #[inline]
    fn write_u16(&mut self, value: u16) {
        self.mix(u64::from(value));
    }

    #[inline]
    fn write_u32(&mut self, value: u32) {
        self.mix(u64::from(value));
    }

    #[inline]
    fn write_u64(&mut self, value: u64) {
        self.mix(value);
    }

    #[inline]
    fn write_usize(&mut self, value: usize) {
        self.mix(value as u64);
    }
}
