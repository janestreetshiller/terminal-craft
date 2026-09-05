//! Seeded voxel world. Six lang territories around a kernel core.

use crate::block::Block;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

pub const SX: i32 = 96;
pub const SY: i32 = 40;
pub const SZ: i32 = 96;

#[derive(Clone, Copy, Debug)]
pub struct Lang {
    pub label: &'static str,
    pub surface: Block,
    pub amp: f32,
    pub tree: f32,
    pub gem: f32,
}

pub const LANGS: [Lang; 6] = [
    Lang { label: "KERNEL", surface: Block::Terra, amp: 9.0, tree: 0.010, gem: 0.002 },
    Lang { label: "PYTHON PRAIRIE", surface: Block::Terra, amp: 4.0, tree: 0.018, gem: 0.002 },
    Lang { label: "RUBY RAILS", surface: Block::Terra, amp: 8.0, tree: 0.020, gem: 0.050 },
    Lang { label: "RUST BELT", surface: Block::Rust, amp: 11.0, tree: 0.005, gem: 0.010 },
    Lang { label: "JAVA JUNGLE", surface: Block::Jungle, amp: 13.0, tree: 0.075, gem: 0.004 },
    Lang { label: "TYPESCRIPT TOPANGA", surface: Block::TsSlate, amp: 9.0, tree: 0.012, gem: 0.004 },
];

/// Project pillars around origin (world-space, origin at map center).
const CLAIMS: [(f32, f32, usize); 5] = [
    (22.0, -8.0, 1),
    (-18.0, 20.0, 2),
    (26.0, 24.0, 3),
    (-24.0, -18.0, 4),
    (6.0, 30.0, 5),
];

pub struct World {
    pub seed: u32,
    data: Vec<u8>,
}

impl World {
    pub fn generate(seed: u32) -> Self {
        let mut w = Self {
            seed,
            data: vec![0; (SX * SY * SZ) as usize],
        };
        for z in 0..SZ {
            for x in 0..SX {
                let wx = (x - SX / 2) as f32;
                let wz = (z - SZ / 2) as f32;
                let info = terrain(seed, wx, wz);
                let h = info.h.clamp(1, SY - 4);
                for y in 0..=h {
                    let b = if y == h && info.gem > 0.0 && hash01(seed, x * 7 + 1, z * 7 - 3) < info.gem {
                        Block::Ruby
                    } else if y >= h - 2 {
                        info.surface
                    } else {
                        Block::Slate
                    };
                    w.set(x, y, z, b);
                }
            }
        }
        // trees
        for z in 2..SZ - 2 {
            for x in 2..SX - 2 {
                if x % 14 != 3 || z % 14 != 5 {
                    continue;
                }
                let wx = (x - SX / 2) as f32;
                let wz = (z - SZ / 2) as f32;
                let info = terrain(seed, wx, wz);
                if hash01(seed, x + 19, z + 7) > info.tree * 8.0 {
                    continue;
                }
                let h = info.h.clamp(1, SY - 8);
                let th = 3 + ((hash(seed, x, z) >> 8) % 3) as i32;
                for y in 1..=th {
                    w.set(x, h + y, z, Block::Wood);
                }
                let top = h + th;
                for dz in -2..=2 {
                    for dx in -2..=2 {
                        if dx * dx + dz * dz > 5 {
                            continue;
                        }
                        for dy in 0..=2 {
                            if !w.get(x + dx, top + dy, z + dz).is_solid() {
                                w.set(x + dx, top + dy, z + dz, Block::Leaf);
                            }
                        }
                    }
                }
            }
        }
        // claim pillars + a worktree stub
        for (cx, cz, lang) in CLAIMS {
            let x = (cx as i32 + SX / 2).clamp(2, SX - 3);
            let z = (cz as i32 + SZ / 2).clamp(2, SZ - 3);
            let h = w.surface_y(x, z).max(4);
            for y in h..=(h + 6).min(SY - 2) {
                w.set(x, y, z, Block::Beacon);
            }
            w.set(x, (h + 7).min(SY - 1), z, LANGS[lang].surface);
        }
        w
    }

    #[inline]
    fn idx(x: i32, y: i32, z: i32) -> Option<usize> {
        if x < 0 || y < 0 || z < 0 || x >= SX || y >= SY || z >= SZ {
            return None;
        }
        Some((y * SZ * SX + z * SX + x) as usize)
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> Block {
        match Self::idx(x, y, z) {
            Some(i) => Block::from_u8(self.data[i]),
            None => Block::Air,
        }
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, b: Block) {
        if let Some(i) = Self::idx(x, y, z) {
            self.data[i] = b as u8;
        }
    }

    pub fn surface_y(&self, x: i32, z: i32) -> i32 {
        for y in (0..SY).rev() {
            if self.get(x, y, z).is_solid() {
                return y;
            }
        }
        0
    }

    pub fn spawn(&self) -> (f32, f32, f32) {
        let x = SX / 2;
        let z = SZ / 2;
        let y = self.surface_y(x, z) + 2;
        (x as f32 + 0.5, y as f32 + 0.1, z as f32 + 0.5)
    }

    pub fn lang_at(&self, x: i32, z: i32) -> &'static str {
        let wx = (x - SX / 2) as f32;
        let wz = (z - SZ / 2) as f32;
        terrain(self.seed, wx, wz).label
    }

    pub fn save(&self, path: &Path, inv: &[u16; 5], creative: bool, slot: u8) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut f = fs::File::create(path)?;
        f.write_all(b"TCRF")?;
        f.write_all(&1u16.to_le_bytes())?;
        f.write_all(&self.seed.to_le_bytes())?;
        f.write_all(&(SX as u16).to_le_bytes())?;
        f.write_all(&(SY as u16).to_le_bytes())?;
        f.write_all(&(SZ as u16).to_le_bytes())?;
        f.write_all(&[if creative { 1 } else { 0 }, slot])?;
        for n in inv {
            f.write_all(&n.to_le_bytes())?;
        }
        f.write_all(&self.data)?;
        Ok(())
    }

    pub fn load(path: &Path) -> io::Result<(Self, [u16; 5], bool, u8)> {
        let mut f = fs::File::open(path)?;
        let mut mag = [0u8; 4];
        f.read_exact(&mut mag)?;
        if &mag != b"TCRF" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "not a TUICraft world"));
        }
        let mut u16b = [0u8; 2];
        f.read_exact(&mut u16b)?;
        let mut seedb = [0u8; 4];
        f.read_exact(&mut seedb)?;
        let seed = u32::from_le_bytes(seedb);
        f.read_exact(&mut u16b)?;
        let sx = u16::from_le_bytes(u16b) as i32;
        f.read_exact(&mut u16b)?;
        let sy = u16::from_le_bytes(u16b) as i32;
        f.read_exact(&mut u16b)?;
        let sz = u16::from_le_bytes(u16b) as i32;
        if sx != SX || sy != SY || sz != SZ {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "world size mismatch"));
        }
        let mut flags = [0u8; 2];
        f.read_exact(&mut flags)?;
        let mut inv = [0u16; 5];
        for n in &mut inv {
            f.read_exact(&mut u16b)?;
            *n = u16::from_le_bytes(u16b);
        }
        let mut data = vec![0u8; (SX * SY * SZ) as usize];
        f.read_exact(&mut data)?;
        Ok((
            Self { seed, data },
            inv,
            flags[0] != 0,
            flags[1],
        ))
    }
}

struct Col {
    h: i32,
    surface: Block,
    gem: f32,
    tree: f32,
    label: &'static str,
}

fn terrain(seed: u32, x: f32, z: f32) -> Col {
    let mut n = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0 / 23.0;
    let mut norm = 0.0;
    for _ in 0..3 {
        n += vnoise(seed ^ 0xA11CE, x * freq, z * freq) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.1;
    }
    n = (n / norm).clamp(0.0, 1.0).powf(1.2);
    let b = vnoise(seed ^ 0xB10B, x / 90.0, z / 90.0);

    let mut range = 4.0 + 13.0 * sstep(((b - 0.25) / 0.55).clamp(0.0, 1.0));
    let mut surface = if b < 0.32 { Block::Sand } else { Block::Terra };
    let mut lang = 0usize;
    let mut gem = 0.002f32;
    let mut tree = LANGS[0].tree;

    let mut best = None;
    let mut bd = f32::INFINITY;
    for (cx, cz, li) in CLAIMS {
        let d = (x - cx) * (x - cx) + (z - cz) * (z - cz);
        if d < bd {
            bd = d;
            best = Some(li);
        }
    }
    if let Some(li) = best {
        bd = bd.sqrt();
        if bd < 28.0 {
            let lang_def = LANGS[li];
            let f = sstep(((28.0 - bd) / 10.0).clamp(0.0, 1.0));
            range += (lang_def.amp - range) * f;
            if f > 0.5 {
                lang = li;
                surface = lang_def.surface;
                gem = lang_def.gem;
                tree = lang_def.tree;
            }
        }
    }

    let d0 = (x * x + z * z).sqrt();
    if d0 < 16.0 {
        let cf = sstep(((16.0 - d0) / 8.0).clamp(0.0, 1.0));
        range += (3.5 - range) * cf;
        if cf > 0.5 {
            lang = 0;
            surface = Block::Terra;
            gem = 0.002;
            tree = LANGS[0].tree;
        }
    }

    let h = 1 + (n * range).round() as i32;
    if lang == 0 && b > 0.74 && h >= 9 {
        surface = Block::Slate;
    }
    Col {
        h,
        surface,
        gem,
        tree,
        label: LANGS[lang].label,
    }
}

fn sstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn hash(seed: u32, x: i32, z: i32) -> u32 {
    let mut h = seed
        .wrapping_mul(374761393)
        ^ (x as u32).wrapping_mul(668265263)
        ^ (z as u32).wrapping_mul(2147483647);
    h ^= h >> 13;
    h = h.wrapping_mul(1274126177);
    h ^= h >> 16;
    h
}

fn hash01(seed: u32, x: i32, z: i32) -> f32 {
    (hash(seed, x, z) as f32) / (u32::MAX as f32)
}

fn vnoise(seed: u32, x: f32, z: f32) -> f32 {
    let x0 = x.floor() as i32;
    let z0 = z.floor() as i32;
    let fx = sstep(x - x0 as f32);
    let fz = sstep(z - z0 as f32);
    let a = hash01(seed, x0, z0);
    let b = hash01(seed, x0 + 1, z0);
    let c = hash01(seed, x0, z0 + 1);
    let d = hash01(seed, x0 + 1, z0 + 1);
    let u = a + (b - a) * fx;
    let v = c + (d - c) * fx;
    u + (v - u) * fz
}
