pub(crate) struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    pub(crate) fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
        }
    }

    pub(crate) fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    pub(crate) fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            self.parent[rb] = ra;
        }
    }
}

/// Deterministic UUID-formatted string (8-4-4-4-12 hex) derived from `seed`.
pub fn deterministic_id(seed: &str) -> String {
    let h1 = fnv1a_64(seed, 0);
    let h2 = fnv1a_64(seed, 0x6c14_4f3a_7af5_c5d2); // arbitrary fixed salt
    let b1 = h1.to_be_bytes();
    let b2 = h2.to_be_bytes();
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b1[0],
        b1[1],
        b1[2],
        b1[3],
        b1[4],
        b1[5],
        b1[6],
        b1[7],
        b2[0],
        b2[1],
        b2[2],
        b2[3],
        b2[4],
        b2[5],
        b2[6],
        b2[7]
    )
}

fn fnv1a_64(seed: &str, salt: u64) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET ^ salt;
    for b in seed.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
