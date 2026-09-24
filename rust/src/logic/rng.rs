/// Small seedable PRNG (mulberry32), bit-for-bit the same as the web build's,
/// so seeded layouts come out identical.
#[derive(Clone, Debug)]
pub struct Rng {
    s: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self { s: seed }
    }

    /// Seeded from the clock, for behavior that should differ run to run.
    pub fn from_time() -> Self {
        let ms = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(1);
        Self::new((ms ^ (ms >> 32)) as u32)
    }

    pub fn next(&mut self) -> f64 {
        self.s = self.s.wrapping_add(0x6d2b_79f5);
        let mut t = self.s;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        ((t ^ (t >> 14)) as f64) / 4_294_967_296.0
    }

    pub fn nextf(&mut self) -> f32 {
        self.next() as f32
    }

    /// Uniform in [a, b).
    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }

    pub fn index(&mut self, len: usize) -> usize {
        ((self.next() * len as f64).floor() as usize).min(len.saturating_sub(1))
    }

    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.index(items.len())]
    }

    /// Fisher-Yates, in the same order as the web build's shuffles.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        let mut i = items.len();
        while i > 1 {
            i -= 1;
            let j = (self.next() * (i + 1) as f64).floor() as usize;
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_mulberry32_reference() {
        // First outputs of mulberry32(1), as produced by the TypeScript Rng.
        let mut r = Rng::new(1);
        let a = r.next();
        let b = r.next();
        assert!((a - 0.627_073_940_588_161_3).abs() < 1e-12, "{a}");
        assert!((b - 0.002_735_721_180_215_478).abs() < 1e-12, "{b}");
    }
}
