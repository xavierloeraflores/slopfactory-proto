//! Engine-independent commands and presentation contracts for Tiny Factory.

use serde::{Deserialize, Serialize};

pub const FORMAT_VERSION: u32 = 1;
pub const WORLD_SIZE: i32 = 64;
pub const CHUNK_SIZE: i32 = 16;
pub const TICKS_PER_SECOND: u32 = 20;
pub const OBJECTIVE_PARTS: u32 = 10;

pub type StructureId = u64;
pub type ResourceId = u64;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const fn offset(self, dx: i32, dy: i32) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }

    pub fn manhattan(self, other: Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub const fn delta(self) -> (i32, i32) {
        match self {
            Self::North => (0, 1),
            Self::East => (1, 0),
            Self::South => (0, -1),
            Self::West => (-1, 0),
        }
    }

    pub const fn clockwise(self) -> Self {
        match self {
            Self::North => Self::East,
            Self::East => Self::South,
            Self::South => Self::West,
            Self::West => Self::North,
        }
    }

    pub const fn apply(self, pos: Pos) -> Pos {
        let (dx, dy) = self.delta();
        pos.offset(dx, dy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Terrain {
    Grass,
    Dirt,
    Sand,
    Water,
    Rock,
}

impl Terrain {
    pub const fn passable(self) -> bool {
        !matches!(self, Self::Water | Self::Rock)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Item {
    IronOre,
    CopperOre,
    Stone,
    Wood,
    IronPlate,
    CopperPlate,
    MechanicalPart,
}

pub const ALL_ITEMS: [Item; 7] = [
    Item::IronOre,
    Item::CopperOre,
    Item::Stone,
    Item::Wood,
    Item::IronPlate,
    Item::CopperPlate,
    Item::MechanicalPart,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Iron,
    Copper,
    Stone,
    Tree,
}

impl ResourceKind {
    pub const fn item(self) -> Item {
        match self {
            Self::Iron => Item::IronOre,
            Self::Copper => Item::CopperOre,
            Self::Stone => Item::Stone,
            Self::Tree => Item::Wood,
        }
    }

    pub const fn mineable(self) -> bool {
        !matches!(self, Self::Tree)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructureKind {
    Miner,
    Conveyor,
    Crafter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recipe {
    IronPlate,
    CopperPlate,
    MechanicalPart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferRoute {
    PlayerToInput,
    InputToPlayer,
    OutputToPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameCommand {
    Move {
        direction: Direction,
    },
    Harvest {
        position: Pos,
    },
    Build {
        kind: StructureKind,
        position: Pos,
        direction: Direction,
    },
    Rotate {
        structure_id: StructureId,
        direction: Direction,
    },
    Configure {
        structure_id: StructureId,
        recipe: Recipe,
    },
    Transfer {
        structure_id: StructureId,
        route: TransferRoute,
        item: Item,
        count: u32,
    },
    Remove {
        structure_id: StructureId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameEvent {
    PlayerMoved {
        position: Pos,
    },
    Harvested {
        resource_id: ResourceId,
        item: Item,
        remaining: u32,
    },
    Built {
        structure_id: StructureId,
        kind: StructureKind,
        position: Pos,
    },
    Removed {
        structure_id: StructureId,
    },
    Rotated {
        structure_id: StructureId,
        direction: Direction,
    },
    Configured {
        structure_id: StructureId,
        recipe: Recipe,
    },
    Transferred {
        structure_id: StructureId,
        item: Item,
        count: u32,
    },
    Produced {
        structure_id: StructureId,
        item: Item,
        count: u32,
    },
    ObjectiveCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item: Item,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileSnapshot {
    pub position: Pos,
    pub terrain: Terrain,
    pub resource_id: Option<ResourceId>,
    pub structure_id: Option<StructureId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    pub id: ResourceId,
    pub position: Pos,
    pub kind: ResourceKind,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureSnapshot {
    pub id: StructureId,
    pub position: Pos,
    pub kind: StructureKind,
    pub direction: Direction,
    pub recipe: Option<Recipe>,
    pub progress: u32,
    pub input: Vec<ItemStack>,
    pub output: Vec<ItemStack>,
    pub belt_item: Option<Item>,
    pub resource_id: Option<ResourceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub position: Pos,
    pub inventory: Vec<ItemStack>,
}

/// Raw-material equivalents currently present in the world and inventories.
/// Construction is a deliberate sink, so compare these totals after placement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialTotals {
    pub iron: u64,
    pub copper: u64,
    pub stone: u64,
    pub wood: u64,
}

impl MaterialTotals {
    pub fn add_resource(&mut self, kind: ResourceKind, count: u64) {
        match kind {
            ResourceKind::Iron => self.iron += count,
            ResourceKind::Copper => self.copper += count,
            ResourceKind::Stone => self.stone += count,
            ResourceKind::Tree => self.wood += count,
        }
    }

    pub fn add_item(&mut self, item: Item, count: u64) {
        match item {
            Item::IronOre => self.iron += count,
            Item::CopperOre => self.copper += count,
            Item::Stone => self.stone += count,
            Item::Wood => self.wood += count,
            Item::IronPlate => self.iron += count * 2,
            Item::CopperPlate => self.copper += count * 2,
            Item::MechanicalPart => {
                self.iron += count * 4;
                self.wood += count;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub format_version: u32,
    pub seed: u64,
    pub tick: u64,
    pub width: i32,
    pub height: i32,
    pub player: PlayerSnapshot,
    pub tiles: Vec<TileSnapshot>,
    pub resources: Vec<ResourceSnapshot>,
    pub structures: Vec<StructureSnapshot>,
    pub mechanical_parts_produced: u32,
    pub objective_complete: bool,
    pub state_hash: String,
}

/// Only data a graphical frontend needs. IDs are stable game IDs, never engine entities.
pub type RenderSnapshot = GameSnapshot;
