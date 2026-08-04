//! Deterministic checksum primitives shared by state and terrain hashing.

/// FNV-1a 64-bit over a byte slice. Stable across platforms and builds.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Incremental FNV-1a hasher so large fields hash without one giant buffer.
pub struct Fnv1a64 {
    state: u64,
}

impl Fnv1a64 {
    pub fn new() -> Self {
        Self {
            state: 0xcbf2_9ce4_8422_2325,
        }
    }

    pub fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state ^= u64::from(byte);
            self.state = self.state.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub fn update_u32(&mut self, value: u32) {
        self.update(&value.to_le_bytes());
    }

    pub fn update_u64(&mut self, value: u64) {
        self.update(&value.to_le_bytes());
    }

    pub fn update_i32(&mut self, value: i32) {
        self.update(&value.to_le_bytes());
    }

    pub fn update_i64(&mut self, value: i64) {
        self.update(&value.to_le_bytes());
    }

    pub fn update_i128(&mut self, value: i128) {
        self.update(&value.to_le_bytes());
    }

    pub fn finish(&self) -> u64 {
        self.state
    }
}

impl Default for Fnv1a64 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incremental_matches_one_shot() {
        let bytes = [1_u8, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut hasher = Fnv1a64::new();
        hasher.update(&bytes[..4]);
        hasher.update(&bytes[4..]);
        assert_eq!(hasher.finish(), fnv1a64(&bytes));
    }

    #[test]
    fn known_vector() {
        // FNV-1a("") is the offset basis; FNV-1a("a") is a published vector.
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    }
}
