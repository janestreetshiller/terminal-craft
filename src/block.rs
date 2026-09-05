//! Block types, harvest table, craft recipes, and Multiplexerverse palette.

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
            Self::Terra => (201, 169, 78),
            Self::Slate => (122, 111, 99),
            Self::Sand => (217, 192, 138),
            Self::Wood => (122, 83, 48),
            Self::Leaf => (127, 174, 74),
            Self::Rust => (176, 100, 56),
            Self::Jungle => (77, 124, 58),
            Self::TsSlate => (111, 127, 149),
            Self::Ruby => (255, 77, 94),
            Self::Core => (77, 232, 224),
            Self::Conduit => (255, 180, 84),
            Self::Beacon => (255, 180, 84),
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
