//! Seeded pseudo-random number generator (xorshift64*).
//!
//! The platform supplies the seed; the core never reads its own entropy (D11).

#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub const fn new(seed: u64) -> Self {
        // xorshift has a fixed point at zero, so remap a zero seed.
        let state = if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        };
        Rng { state }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A value in `lo..=hi`. Returns `lo` if `hi < lo`.
    pub fn range_inclusive(&mut self, lo: u64, hi: u64) -> u64 {
        if hi <= lo {
            return lo;
        }
        match (hi - lo).checked_add(1) {
            Some(span) => lo + self.next_u64() % span,
            None => self.next_u64(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn zero_seed_is_not_stuck() {
        let mut r = Rng::new(0);
        assert_ne!(r.next_u64(), r.next_u64());
    }

    #[test]
    fn range_inclusive_stays_in_bounds() {
        let mut r = Rng::new(7);
        for _ in 0..1_000 {
            let v = r.range_inclusive(2_000, 6_000);
            assert!((2_000..=6_000).contains(&v));
        }
        assert_eq!(r.range_inclusive(5, 5), 5);
        assert_eq!(r.range_inclusive(9, 3), 9);
    }
}
