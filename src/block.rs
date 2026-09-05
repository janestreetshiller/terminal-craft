//! Block types, harvest table, craft recipes, and original procedural material palette.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Block {
    Air = 0,
    Terra = 1,
    Slate = 2,
    Sand = 3,
    Wood = 4,
    Leaf = 5,
    Rust = 6,
    Jungle = 7,
    TsSlate = 8,
    Ruby = 9,
    Core = 10,
    Conduit = 11,
    Beacon = 12,
}

impl Block {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Terra,
            2 => Self::Slate,
            3 => Self::Sand,
            4 => Self::Wood,
            5 => Self::Leaf,
            6 => Self::Rust,
            7 => Self::Jungle,
            8 => Self::TsSlate,
            9 => Self::Ruby,
            10 => Self::Core,
            11 => Self::Conduit,
            12 => Self::Beacon,
            _ => Self::Air,
        }
    }

    pub fn is_solid(self) -> bool {
        self != Self::Air
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Air => "AIR",
            Self::Terra => "TERRA",
            Self::Slate => "SLATE",
            Self::Sand => "SAND",
            Self::Wood => "WOOD",
            Self::Leaf => "LEAF",
            Self::Rust => "RUST",
            Self::Jungle => "JUNGLE",
            Self::TsSlate => "TSLATE",
            Self::Ruby => "RUBY",
            Self::Core => "CORE",
            Self::Conduit => "CONDUIT",
            Self::Beacon => "BEACON",
        }
    }

    /// Face-tinted RGB. `face`: 0=+Y 1=-Y 2=+X 3=-X 4=+Z 5=-Z
    pub fn rgb(self, face: u8) -> (u8, u8, u8) {
        let (r, g, b) = match self {
            Self::Air => (70, 99, 155),
            Self::Terra if face == 0 => (110, 151, 76),
            Self::Terra => (132, 98, 65),
            Self::Slate => (128, 134, 136),
            Self::Sand => (217, 192, 138),
            Self::Wood => (122, 83, 48),
            Self::Leaf => (97, 145, 76),
            Self::Rust => (176, 100, 56),
            Self::Jungle => (77, 124, 58),
            Self::TsSlate => (111, 127, 149),
            Self::Ruby => (125, 119, 120),
            Self::Core => (105, 184, 178),
            Self::Conduit => (203, 171, 105),
            Self::Beacon => (225, 207, 150),
        };
        let shade = match face {
            0 => 1.00,
            1 => 0.48,
            2 | 3 => 0.78,
            _ => 0.88,
        };
        (
            ((r as f32) * shade) as u8,
            ((g as f32) * shade) as u8,
            ((b as f32) * shade) as u8,
        )
    }

    /// Original 16x16 procedural pixel materials, shared by world and previews.
    pub fn texel(self, face: u8, u: f32, v: f32) -> (u8, u8, u8) {
        let x = (u.clamp(0.0, 0.9999) * 16.0) as i32;
        let y = (v.clamp(0.0, 0.9999) * 16.0) as i32;
        let mut hash = (x as u32).wrapping_mul(0x45d9f3b)
            ^ (y as u32).wrapping_mul(0x119de1f3)
            ^ (self as u32).wrapping_mul(0x27d4eb2d);
        hash = (hash ^ (hash >> 16)).wrapping_mul(0x45d9f3b);
        let noise = ((hash >> 20) & 15) as i16 - 7;
        let mut color = self.rgb(face);
        let mut delta = noise;
        match self {
            Self::Terra if face > 1 && y >= 13 + (x % 3) => color = (81, 119, 54),
            Self::Wood if face < 2 => {
                let ring = (x - 7).abs().max((y - 7).abs());
                color = (160, 121, 76);
                delta = if ring % 3 == 0 { -22 } else { noise };
            }
            Self::Wood => delta += if (x + (y / 5)) % 4 == 0 { -18 } else { 3 },
            Self::Slate | Self::TsSlate | Self::Rust => {
                delta += if (y + x / 5) % 7 == 0 { -8 } else { 2 }
            }
            Self::Leaf | Self::Jungle => delta *= 2,
            Self::Ruby if hash & 7 < 2 => color = (170, 75, 75),
            Self::Core | Self::Conduit | Self::Beacon if x == 2 || x == 13 || y == 2 || y == 13 => {
                color = (74, 89, 88);
                delta = 0;
            }
            _ => {}
        }
        let shade = |c: u8| (c as i16 + delta).clamp(0, 255) as u8;
        (shade(color.0), shade(color.1), shade(color.2))
    }

    pub fn harvest(self) -> Option<Res> {
        match self {
            Self::Terra | Self::Jungle => Some(Res::Terra),
            Self::Slate | Self::Rust | Self::TsSlate => Some(Res::Slate),
            Self::Sand => Some(Res::Sand),
            Self::Wood | Self::Leaf => Some(Res::Wood),
            Self::Ruby => Some(Res::Gem),
            Self::Core | Self::Conduit | Self::Beacon | Self::Air => None,
        }
    }
}

#[cfg(test)]
mod material_tests {
    use super::*;
    #[test]
    fn materials_have_readable_surfaces_not_flat_color_tiles() {
        let grass = Block::Terra.texel(0, 0.4, 0.4);
        let dirt = Block::Terra.texel(2, 0.4, 0.3);
        assert!(
            grass.1 > grass.0 && grass.1 > grass.2,
            "grass top must read green"
        );
        assert!(
            dirt.0 > dirt.1 && dirt.1 > dirt.2,
            "grass sides must read earth"
        );
        assert_ne!(
            Block::Wood.texel(0, 0.2, 0.2),
            Block::Wood.texel(2, 0.2, 0.2)
        );
        let mut colors = std::collections::HashSet::new();
        for x in 0..16 {
            for y in 0..16 {
                colors.insert(Block::Slate.texel(0, x as f32 / 16.0, y as f32 / 16.0));
            }
        }
        assert!(colors.len() > 8, "stone needs restrained texture variation");
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Res {
    Terra,
    Slate,
    Wood,
    Sand,
    Gem,
}

impl Res {
    pub fn name(self) -> &'static str {
        match self {
            Self::Terra => "TERRA",
            Self::Slate => "SLATE",
            Self::Wood => "WOOD",
            Self::Sand => "SAND",
            Self::Gem => "GEM",
        }
    }
}

pub const HOTBAR: [Block; 6] = [
    Block::Terra,
    Block::Slate,
    Block::Wood,
    Block::Core,
    Block::Conduit,
    Block::Beacon,
];

#[derive(Clone, Debug, Default)]
pub struct Inventory {
    pub terra: u16,
    pub slate: u16,
    pub wood: u16,
    pub sand: u16,
    pub gem: u16,
}

impl Inventory {
    pub fn add(&mut self, r: Res, n: u16) {
        match r {
            Res::Terra => self.terra = self.terra.saturating_add(n),
            Res::Slate => self.slate = self.slate.saturating_add(n),
            Res::Wood => self.wood = self.wood.saturating_add(n),
            Res::Sand => self.sand = self.sand.saturating_add(n),
            Res::Gem => self.gem = self.gem.saturating_add(n),
        }
    }

    pub fn pack(&self) -> [u16; 5] {
        [self.terra, self.slate, self.wood, self.sand, self.gem]
    }

    pub fn unpack(v: [u16; 5]) -> Self {
        Self {
            terra: v[0],
            slate: v[1],
            wood: v[2],
            sand: v[3],
            gem: v[4],
        }
    }

    pub fn try_pay(&mut self, block: Block, creative: bool) -> bool {
        if creative {
            return true;
        }
        match block {
            Block::Terra if self.terra >= 1 => {
                self.terra -= 1;
                true
            }
            Block::Slate if self.slate >= 1 => {
                self.slate -= 1;
                true
            }
            Block::Sand if self.sand >= 1 => {
                self.sand -= 1;
                true
            }
            Block::Wood if self.wood >= 1 => {
                self.wood -= 1;
                true
            }
            Block::Core if self.slate >= 8 && self.gem >= 1 => {
                self.slate -= 8;
                self.gem -= 1;
                true
            }
            Block::Conduit if self.slate >= 3 && self.sand >= 1 => {
                self.slate -= 3;
                self.sand -= 1;
                true
            }
            Block::Beacon if self.sand >= 4 && self.gem >= 1 => {
                self.sand -= 4;
                self.gem -= 1;
                true
            }
            _ => false,
        }
    }

    pub fn held_count(&self, block: Block) -> u16 {
        match block {
            Block::Terra => self.terra,
            Block::Slate => self.slate,
            Block::Wood => self.wood,
            Block::Sand => self.sand,
            Block::Core => self.slate.saturating_div(8).min(self.gem),
            Block::Conduit => self.slate.saturating_div(3).min(self.sand),
            Block::Beacon => self.sand.saturating_div(4).min(self.gem),
            _ => 0,
        }
    }
}
