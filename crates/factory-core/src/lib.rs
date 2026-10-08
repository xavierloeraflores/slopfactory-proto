//! Canonical, deterministic Tiny Factory simulation. No graphics engine is used here.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use factory_protocol::{
    CHUNK_SIZE, Direction, FORMAT_VERSION, GameCommand, GameEvent, GameSnapshot, Item, ItemStack,
    MaterialTotals, OBJECTIVE_PARTS, PlayerSnapshot, Pos, Recipe, RenderSnapshot, ResourceId,
    ResourceKind, ResourceSnapshot, StructureId, StructureKind, StructureSnapshot, Terrain,
    TileSnapshot, TransferRoute, WORLD_SIZE,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

const INTERACTION_RANGE: i32 = 3;
const MINER_BUFFER_CAPACITY: u32 = 4;
const CRAFTER_OUTPUT_CAPACITY: u32 = 4;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GameError {
    #[error("position is outside the world")]
    OutOfBounds,
    #[error("target is out of interaction range")]
    OutOfRange,
    #[error("terrain blocks the action")]
    BlockedTerrain,
    #[error("tile already contains a structure")]
    Occupied,
    #[error("no resource is present")]
    NoResource,
    #[error("resource is depleted")]
    Depleted,
    #[error("resource cannot be mined by a machine")]
    NotMineable,
    #[error("insufficient materials")]
    InsufficientMaterials,
    #[error("structure does not exist")]
    NoStructure,
    #[error("structure is not a crafter")]
    NotCrafter,
    #[error("invalid transfer")]
    InvalidTransfer,
    #[error("storage capacity would be exceeded")]
    CapacityExceeded,
    #[error("unsupported save format version {0}")]
    UnsupportedVersion(u32),
    #[error("invalid save: {0}")]
    InvalidSave(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub terrain: Terrain,
    pub resource_id: Option<ResourceId>,
    pub structure_id: Option<StructureId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chunk {
    pub origin: Pos,
    pub tiles: Vec<Tile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceNode {
    pub id: ResourceId,
    pub position: Pos,
    pub kind: ResourceKind,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructureState {
    Miner {
        progress: u32,
        output: u32,
        resource_id: ResourceId,
    },
    Conveyor {
        item: Option<Item>,
    },
    Crafter {
        recipe: Option<Recipe>,
        progress: u32,
        input: BTreeMap<Item, u32>,
        output: BTreeMap<Item, u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Structure {
    pub id: StructureId,
    pub position: Pos,
    pub direction: Direction,
    pub state: StructureState,
}

impl Structure {
    pub fn kind(&self) -> StructureKind {
        match self.state {
            StructureState::Miner { .. } => StructureKind::Miner,
            StructureState::Conveyor { .. } => StructureKind::Conveyor,
            StructureState::Crafter { .. } => StructureKind::Crafter,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub position: Pos,
    pub inventory: BTreeMap<Item, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactorySimulation {
    format_version: u32,
    seed: u64,
    tick: u64,
    chunks: Vec<Chunk>,
    resources: BTreeMap<ResourceId, ResourceNode>,
    structures: BTreeMap<StructureId, Structure>,
    player: Player,
    mechanical_parts_produced: u32,
    next_resource_id: ResourceId,
    next_structure_id: StructureId,
}

impl FactorySimulation {
    pub const fn tick(&self) -> u64 {
        self.tick
    }
    pub const fn seed(&self) -> u64 {
        self.seed
    }
    pub fn player_position(&self) -> Pos {
        self.player.position
    }
    pub fn inventory_count(&self, item: Item) -> u32 {
        self.player.inventory.get(&item).copied().unwrap_or(0)
    }
    pub fn resource_quantity(&self, id: ResourceId) -> Option<u32> {
        self.resources.get(&id).map(|resource| resource.quantity)
    }
    pub fn structure(&self, id: StructureId) -> Option<&Structure> {
        self.structures.get(&id)
    }
    pub fn produced_parts(&self) -> u32 {
        self.mechanical_parts_produced
    }

    pub fn material_totals(&self) -> MaterialTotals {
        let mut totals = MaterialTotals::default();
        for node in self.resources.values() {
            totals.add_resource(node.kind, node.quantity as u64);
        }
        for (item, count) in &self.player.inventory {
            totals.add_item(*item, *count as u64);
        }
        for structure in self.structures.values() {
            match &structure.state {
                StructureState::Miner {
                    output,
                    resource_id,
                    ..
                } => {
                    if let Some(node) = self.resources.get(resource_id) {
                        totals.add_item(node.kind.item(), *output as u64);
                    }
                }
                StructureState::Conveyor { item: Some(item) } => totals.add_item(*item, 1),
                StructureState::Conveyor { item: None } => {}
                StructureState::Crafter {
                    recipe,
                    progress,
                    input,
                    output,
                } => {
                    for (item, count) in input.iter().chain(output) {
                        totals.add_item(*item, *count as u64);
                    }
                    if *progress > 0
                        && let Some(recipe) = recipe
                    {
                        for (item, count) in recipe_inputs(*recipe) {
                            totals.add_item(*item, *count as u64);
                        }
                    }
                }
            }
        }
        totals
    }

    pub fn new(seed: u64) -> Self {
        let mut chunks = Vec::with_capacity(16);
        for cy in 0..4 {
            for cx in 0..4 {
                let mut tiles = Vec::with_capacity((CHUNK_SIZE * CHUNK_SIZE) as usize);
                for ly in 0..CHUNK_SIZE {
                    for lx in 0..CHUNK_SIZE {
                        let pos = Pos::new(cx * CHUNK_SIZE + lx, cy * CHUNK_SIZE + ly);
                        tiles.push(Tile {
                            terrain: terrain_at(seed, pos),
                            resource_id: None,
                            structure_id: None,
                        });
                    }
                }
                chunks.push(Chunk {
                    origin: Pos::new(cx * CHUNK_SIZE, cy * CHUNK_SIZE),
                    tiles,
                });
            }
        }
        let mut simulation = Self {
            format_version: FORMAT_VERSION,
            seed,
            tick: 0,
            chunks,
            resources: BTreeMap::new(),
            structures: BTreeMap::new(),
            player: Player {
                position: Pos::new(32, 32),
                inventory: BTreeMap::from([
                    (Item::Stone, 20),
                    (Item::IronOre, 12),
                    (Item::Wood, 5),
                ]),
            },
            mechanical_parts_produced: 0,
            next_resource_id: 1,
            next_structure_id: 1,
        };
        for (pos, kind, quantity) in [
            (Pos::new(35, 31), ResourceKind::Iron, 120),
            (Pos::new(36, 31), ResourceKind::Iron, 120),
            (Pos::new(29, 34), ResourceKind::Copper, 120),
            (Pos::new(32, 36), ResourceKind::Stone, 150),
            (Pos::new(34, 36), ResourceKind::Stone, 150),
            (Pos::new(30, 30), ResourceKind::Tree, 45),
            (Pos::new(28, 32), ResourceKind::Tree, 45),
        ] {
            simulation.add_resource(pos, kind, quantity);
        }
        // Additional deposits diversify the map without making the start dependent on chance.
        let reachable = simulation.reachable_tiles();
        for y in 1..WORLD_SIZE - 1 {
            for x in 1..WORLD_SIZE - 1 {
                let pos = Pos::new(x, y);
                if pos.manhattan(simulation.player.position) < 12
                    || !reachable.contains(&pos)
                    || !simulation
                        .tile(pos)
                        .is_some_and(|tile| tile.terrain.passable() && tile.resource_id.is_none())
                {
                    continue;
                }
                let roll = coord_hash(seed ^ 0x56b2_2d31, x, y) % 512;
                let kind = match roll {
                    0 => Some(ResourceKind::Iron),
                    1 => Some(ResourceKind::Copper),
                    2 => Some(ResourceKind::Stone),
                    3..=5 => Some(ResourceKind::Tree),
                    _ => None,
                };
                if let Some(kind) = kind {
                    let quantity = if kind == ResourceKind::Tree { 25 } else { 80 };
                    simulation.add_resource(pos, kind, quantity);
                }
            }
        }
        simulation
    }

    pub fn tile(&self, pos: Pos) -> Option<&Tile> {
        let (chunk_index, tile_index) = tile_indices(pos)?;
        self.chunks.get(chunk_index)?.tiles.get(tile_index)
    }

    fn tile_mut(&mut self, pos: Pos) -> Option<&mut Tile> {
        let (chunk_index, tile_index) = tile_indices(pos)?;
        self.chunks.get_mut(chunk_index)?.tiles.get_mut(tile_index)
    }

    fn add_resource(&mut self, pos: Pos, kind: ResourceKind, quantity: u32) -> ResourceId {
        let id = self.next_resource_id;
        self.next_resource_id += 1;
        if let Some(tile) = self.tile_mut(pos) {
            tile.terrain = Terrain::Grass;
            tile.resource_id = Some(id);
        }
        self.resources.insert(
            id,
            ResourceNode {
                id,
                position: pos,
                kind,
                quantity,
            },
        );
        id
    }

    pub fn resource_at(&self, pos: Pos) -> Option<&ResourceNode> {
        self.tile(pos)?
            .resource_id
            .and_then(|id| self.resources.get(&id))
    }

    pub fn reachable_tiles(&self) -> BTreeSet<Pos> {
        let mut visited = BTreeSet::from([self.player.position]);
        let mut queue = VecDeque::from([self.player.position]);
        while let Some(pos) = queue.pop_front() {
            for direction in [
                Direction::North,
                Direction::East,
                Direction::South,
                Direction::West,
            ] {
                let neighbor = direction.apply(pos);
                if !visited.contains(&neighbor)
                    && self
                        .tile(neighbor)
                        .is_some_and(|tile| tile.terrain.passable())
                {
                    visited.insert(neighbor);
                    queue.push_back(neighbor);
                }
            }
        }
        visited
    }

    pub fn structure_at(&self, pos: Pos) -> Option<&Structure> {
        self.tile(pos)?
            .structure_id
            .and_then(|id| self.structures.get(&id))
    }

    pub fn execute(&mut self, command: GameCommand) -> Result<Vec<GameEvent>, GameError> {
        match command {
            GameCommand::Move { direction } => self.move_player(direction),
            GameCommand::Harvest { position } => self.harvest(position),
            GameCommand::Build {
                kind,
                position,
                direction,
            } => self.build(kind, position, direction),
            GameCommand::Rotate {
                structure_id,
                direction,
            } => self.rotate(structure_id, direction),
            GameCommand::Configure {
                structure_id,
                recipe,
            } => self.configure(structure_id, recipe),
            GameCommand::Transfer {
                structure_id,
                route,
                item,
                count,
            } => self.transfer_manual(structure_id, route, item, count),
            GameCommand::Remove { structure_id } => self.remove(structure_id),
        }
    }

    fn in_range(&self, pos: Pos) -> Result<(), GameError> {
        if self.tile(pos).is_none() {
            return Err(GameError::OutOfBounds);
        }
        if self.player.position.manhattan(pos) > INTERACTION_RANGE {
            return Err(GameError::OutOfRange);
        }
        Ok(())
    }

    fn move_player(&mut self, direction: Direction) -> Result<Vec<GameEvent>, GameError> {
        let destination = direction.apply(self.player.position);
        let tile = self.tile(destination).ok_or(GameError::OutOfBounds)?;
        if !tile.terrain.passable() || tile.structure_id.is_some() {
            return Err(GameError::BlockedTerrain);
        }
        self.player.position = destination;
        Ok(vec![GameEvent::PlayerMoved {
            position: destination,
        }])
    }

    fn harvest(&mut self, position: Pos) -> Result<Vec<GameEvent>, GameError> {
        self.in_range(position)?;
        let id = self
            .tile(position)
            .and_then(|tile| tile.resource_id)
            .ok_or(GameError::NoResource)?;
        if self
            .tile(position)
            .is_some_and(|tile| tile.structure_id.is_some())
        {
            return Err(GameError::Occupied);
        }
        let resource = self.resources.get_mut(&id).ok_or(GameError::NoResource)?;
        if resource.quantity == 0 {
            return Err(GameError::Depleted);
        }
        resource.quantity -= 1;
        let item = resource.kind.item();
        *self.player.inventory.entry(item).or_default() += 1;
        Ok(vec![GameEvent::Harvested {
            resource_id: id,
            item,
            remaining: resource.quantity,
        }])
    }

    fn build(
        &mut self,
        kind: StructureKind,
        position: Pos,
        direction: Direction,
    ) -> Result<Vec<GameEvent>, GameError> {
        self.in_range(position)?;
        let tile = self.tile(position).ok_or(GameError::OutOfBounds)?;
        if !tile.terrain.passable() {
            return Err(GameError::BlockedTerrain);
        }
        if tile.structure_id.is_some() {
            return Err(GameError::Occupied);
        }
        if position == self.player.position {
            return Err(GameError::Occupied);
        }
        let resource_id = match kind {
            StructureKind::Miner => {
                let id = tile.resource_id.ok_or(GameError::NoResource)?;
                let node = self.resources.get(&id).ok_or(GameError::NoResource)?;
                if !node.kind.mineable() {
                    return Err(GameError::NotMineable);
                }
                if node.quantity == 0 {
                    return Err(GameError::Depleted);
                }
                Some(id)
            }
            _ => {
                if tile.resource_id.is_some() {
                    return Err(GameError::Occupied);
                }
                None
            }
        };
        for (item, count) in construction_cost(kind) {
            if self.player.inventory.get(item).copied().unwrap_or(0) < *count {
                return Err(GameError::InsufficientMaterials);
            }
        }
        let id = self.next_structure_id;
        self.next_structure_id += 1;
        let state = match kind {
            StructureKind::Miner => StructureState::Miner {
                progress: 0,
                output: 0,
                resource_id: resource_id.expect("miner resource validated"),
            },
            StructureKind::Conveyor => StructureState::Conveyor { item: None },
            StructureKind::Crafter => StructureState::Crafter {
                recipe: None,
                progress: 0,
                input: BTreeMap::new(),
                output: BTreeMap::new(),
            },
        };
        self.structures.insert(
            id,
            Structure {
                id,
                position,
                direction,
                state,
            },
        );
        self.tile_mut(position)
            .expect("position validated")
            .structure_id = Some(id);
        for (item, count) in construction_cost(kind) {
            *self
                .player
                .inventory
                .get_mut(item)
                .expect("materials checked") -= count;
        }
        Ok(vec![GameEvent::Built {
            structure_id: id,
            kind,
            position,
        }])
    }

    fn rotate(
        &mut self,
        structure_id: StructureId,
        direction: Direction,
    ) -> Result<Vec<GameEvent>, GameError> {
        let position = self
            .structures
            .get(&structure_id)
            .ok_or(GameError::NoStructure)?
            .position;
        self.in_range(position)?;
        self.structures
            .get_mut(&structure_id)
            .expect("structure exists")
            .direction = direction;
        Ok(vec![GameEvent::Rotated {
            structure_id,
            direction,
        }])
    }

    fn configure(
        &mut self,
        structure_id: StructureId,
        recipe: Recipe,
    ) -> Result<Vec<GameEvent>, GameError> {
        let position = self
            .structures
            .get(&structure_id)
            .ok_or(GameError::NoStructure)?
            .position;
        self.in_range(position)?;
        let structure = self
            .structures
            .get_mut(&structure_id)
            .expect("structure exists");
        let StructureState::Crafter {
            recipe: selected,
            progress,
            input,
            ..
        } = &mut structure.state
        else {
            return Err(GameError::NotCrafter);
        };
        if *progress > 0 {
            let reserved: u32 = recipe_inputs(selected.expect("active recipe"))
                .iter()
                .map(|(_, count)| count)
                .sum();
            if inventory_total(input) + reserved > 24 {
                return Err(GameError::CapacityExceeded);
            }
            for (item, count) in recipe_inputs(selected.expect("active recipe")) {
                *input.entry(*item).or_default() += count;
            }
        }
        *selected = Some(recipe);
        *progress = 0;
        Ok(vec![GameEvent::Configured {
            structure_id,
            recipe,
        }])
    }

    fn transfer_manual(
        &mut self,
        structure_id: StructureId,
        route: TransferRoute,
        item: Item,
        count: u32,
    ) -> Result<Vec<GameEvent>, GameError> {
        if count == 0 {
            return Err(GameError::InvalidTransfer);
        }
        let position = self
            .structures
            .get(&structure_id)
            .ok_or(GameError::NoStructure)?
            .position;
        self.in_range(position)?;
        let structure = self
            .structures
            .get_mut(&structure_id)
            .expect("structure exists");
        let StructureState::Crafter { input, output, .. } = &mut structure.state else {
            return Err(GameError::NotCrafter);
        };
        match route {
            TransferRoute::PlayerToInput => {
                if self.player.inventory.get(&item).copied().unwrap_or(0) < count {
                    return Err(GameError::InsufficientMaterials);
                }
                if inventory_total(input) + count > 24 {
                    return Err(GameError::CapacityExceeded);
                }
                *self.player.inventory.get_mut(&item).expect("checked") -= count;
                *input.entry(item).or_default() += count;
            }
            TransferRoute::InputToPlayer | TransferRoute::OutputToPlayer => {
                let source = if route == TransferRoute::InputToPlayer {
                    input
                } else {
                    output
                };
                if source.get(&item).copied().unwrap_or(0) < count {
                    return Err(GameError::InsufficientMaterials);
                }
                *source.get_mut(&item).expect("checked") -= count;
                *self.player.inventory.entry(item).or_default() += count;
            }
        }
        Ok(vec![GameEvent::Transferred {
            structure_id,
            item,
            count,
        }])
    }

    fn remove(&mut self, structure_id: StructureId) -> Result<Vec<GameEvent>, GameError> {
        let structure = self
            .structures
            .get(&structure_id)
            .ok_or(GameError::NoStructure)?;
        self.in_range(structure.position)?;
        let structure = self
            .structures
            .remove(&structure_id)
            .expect("structure exists");
        self.tile_mut(structure.position)
            .expect("position valid")
            .structure_id = None;
        match structure.state {
            StructureState::Miner {
                output,
                resource_id,
                ..
            } => {
                let item = self
                    .resources
                    .get(&resource_id)
                    .expect("resource exists")
                    .kind
                    .item();
                *self.player.inventory.entry(item).or_default() += output;
            }
            StructureState::Conveyor { item: Some(item) } => {
                *self.player.inventory.entry(item).or_default() += 1
            }
            StructureState::Conveyor { item: None } => {}
            StructureState::Crafter {
                recipe,
                progress,
                input,
                output,
            } => {
                for (item, count) in input.into_iter().chain(output) {
                    *self.player.inventory.entry(item).or_default() += count;
                }
                if progress > 0
                    && let Some(recipe) = recipe
                {
                    for (item, count) in recipe_inputs(recipe) {
                        *self.player.inventory.entry(*item).or_default() += count;
                    }
                }
            }
        }
        Ok(vec![GameEvent::Removed { structure_id }])
    }

    /// Advance one 50 ms simulation tick. This clock is independent of Bevy.
    /// Pipeline: extraction, crafting, belt transfers, machine exports.
    pub fn step(&mut self) -> Vec<GameEvent> {
        self.tick += 1;
        let previous_parts = self.mechanical_parts_produced;
        self.mine_tick();
        let mut events = self.craft_tick();
        if self.tick.is_multiple_of(5) {
            self.transfer_belts();
        }
        self.export_machines();
        if previous_parts < OBJECTIVE_PARTS && self.mechanical_parts_produced >= OBJECTIVE_PARTS {
            events.push(GameEvent::ObjectiveCompleted);
        }
        events
    }

    pub fn step_many(&mut self, ticks: u64) -> Vec<GameEvent> {
        let mut events = Vec::new();
        for _ in 0..ticks {
            events.extend(self.step());
        }
        events
    }

    fn mine_tick(&mut self) {
        for structure in self.structures.values_mut() {
            let StructureState::Miner {
                progress,
                output,
                resource_id,
            } = &mut structure.state
            else {
                continue;
            };
            if *output >= MINER_BUFFER_CAPACITY {
                continue;
            }
            let Some(node) = self.resources.get_mut(resource_id) else {
                continue;
            };
            if node.quantity == 0 {
                continue;
            }
            *progress += 1;
            if *progress == 60 {
                node.quantity -= 1;
                *output += 1;
                *progress = 0;
            }
        }
    }

    fn craft_tick(&mut self) -> Vec<GameEvent> {
        let mut events = Vec::new();
        for structure in self.structures.values_mut() {
            let StructureState::Crafter {
                recipe: selected,
                progress,
                input,
                output,
            } = &mut structure.state
            else {
                continue;
            };
            let Some(recipe) = *selected else {
                continue;
            };
            if *progress == 0 {
                if inventory_total(output) >= CRAFTER_OUTPUT_CAPACITY {
                    continue;
                }
                if recipe_inputs(recipe)
                    .iter()
                    .any(|(item, count)| input.get(item).copied().unwrap_or(0) < *count)
                {
                    continue;
                }
                for (item, count) in recipe_inputs(recipe) {
                    *input.get_mut(item).expect("ingredients checked") -= count;
                }
            }
            if *progress < recipe_time(recipe) {
                *progress += 1;
            }
            if *progress == recipe_time(recipe) && inventory_total(output) < CRAFTER_OUTPUT_CAPACITY
            {
                let item = recipe_output(recipe);
                *output.entry(item).or_default() += 1;
                *progress = 0;
                if item == Item::MechanicalPart {
                    self.mechanical_parts_produced += 1;
                }
                events.push(GameEvent::Produced {
                    structure_id: structure.id,
                    item,
                    count: 1,
                });
            }
        }
        events
    }

    fn transfer_belts(&mut self) {
        // Read all sources and destinations before moving anything. No item can cross
        // more than one belt during this transfer phase.
        let mut intents = Vec::new();
        let mut claimed = BTreeSet::new();
        for source in self.structures.values() {
            let StructureState::Conveyor { item: Some(item) } = source.state else {
                continue;
            };
            let destination_position = source.direction.apply(source.position);
            let Some(destination_id) = self
                .tile(destination_position)
                .and_then(|tile| tile.structure_id)
            else {
                continue;
            };
            if claimed.contains(&destination_id) {
                continue;
            }
            let Some(destination) = self.structures.get(&destination_id) else {
                continue;
            };
            let accepts = match &destination.state {
                StructureState::Conveyor { item: None } => true,
                StructureState::Crafter {
                    recipe: Some(recipe),
                    input,
                    ..
                } => {
                    recipe_inputs(*recipe)
                        .iter()
                        .any(|(ingredient, _)| *ingredient == item)
                        && inventory_total(input) < 24
                }
                _ => false,
            };
            if accepts {
                claimed.insert(destination_id);
                intents.push((source.id, destination_id, item));
            }
        }
        for (source_id, destination_id, item) in intents {
            if let Some(Structure {
                state: StructureState::Conveyor { item: slot },
                ..
            }) = self.structures.get_mut(&source_id)
            {
                *slot = None;
            }
            match &mut self
                .structures
                .get_mut(&destination_id)
                .expect("destination exists")
                .state
            {
                StructureState::Conveyor { item: slot } => *slot = Some(item),
                StructureState::Crafter { input, .. } => *input.entry(item).or_default() += 1,
                _ => unreachable!("destination was accepted"),
            }
        }
    }

    fn export_machines(&mut self) {
        let mut intents = Vec::new();
        let mut claimed = BTreeSet::new();
        for source in self.structures.values() {
            let item = match &source.state {
                StructureState::Miner {
                    output,
                    resource_id,
                    ..
                } if *output > 0 => self.resources.get(resource_id).map(|node| node.kind.item()),
                StructureState::Crafter { output, .. } => output
                    .iter()
                    .find(|(_, count)| **count > 0)
                    .map(|(item, _)| *item),
                _ => None,
            };
            let Some(item) = item else {
                continue;
            };
            let Some(destination_id) = self
                .tile(source.direction.apply(source.position))
                .and_then(|tile| tile.structure_id)
            else {
                continue;
            };
            if claimed.contains(&destination_id) {
                continue;
            }
            if self
                .structures
                .get(&destination_id)
                .is_some_and(|destination| {
                    matches!(destination.state, StructureState::Conveyor { item: None })
                })
            {
                claimed.insert(destination_id);
                intents.push((source.id, destination_id, item));
            }
        }
        for (source_id, destination_id, item) in intents {
            match &mut self
                .structures
                .get_mut(&source_id)
                .expect("source exists")
                .state
            {
                StructureState::Miner { output, .. } => *output -= 1,
                StructureState::Crafter { output, .. } => {
                    *output.get_mut(&item).expect("item selected") -= 1
                }
                _ => unreachable!("source was accepted"),
            }
            if let StructureState::Conveyor { item: slot } = &mut self
                .structures
                .get_mut(&destination_id)
                .expect("destination exists")
                .state
            {
                *slot = Some(item);
            }
        }
    }

    pub fn snapshot(&self) -> GameSnapshot {
        let mut tiles = Vec::with_capacity((WORLD_SIZE * WORLD_SIZE) as usize);
        for y in 0..WORLD_SIZE {
            for x in 0..WORLD_SIZE {
                let position = Pos::new(x, y);
                let tile = self
                    .tile(position)
                    .expect("generated world contains every tile");
                tiles.push(TileSnapshot {
                    position,
                    terrain: tile.terrain,
                    resource_id: tile.resource_id,
                    structure_id: tile.structure_id,
                });
            }
        }
        GameSnapshot {
            format_version: self.format_version,
            seed: self.seed,
            tick: self.tick,
            width: WORLD_SIZE,
            height: WORLD_SIZE,
            player: PlayerSnapshot {
                position: self.player.position,
                inventory: stacks(&self.player.inventory),
            },
            tiles,
            resources: self
                .resources
                .values()
                .map(|node| ResourceSnapshot {
                    id: node.id,
                    position: node.position,
                    kind: node.kind,
                    quantity: node.quantity,
                })
                .collect(),
            structures: self
                .structures
                .values()
                .map(|structure| structure.snapshot(&self.resources))
                .collect(),
            mechanical_parts_produced: self.mechanical_parts_produced,
            objective_complete: self.mechanical_parts_produced >= OBJECTIVE_PARTS,
            state_hash: self.state_hash(),
        }
    }

    pub fn render_snapshot(&self) -> RenderSnapshot {
        self.snapshot()
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, GameError> {
        let state: Self = serde_json::from_str(json)
            .map_err(|error| GameError::InvalidSave(error.to_string()))?;
        state.validate()?;
        Ok(state)
    }

    pub fn state_hash(&self) -> String {
        let bytes = serde_json::to_vec(self).expect("simulation state is serializable");
        format!("{:x}", Sha256::digest(bytes))
    }

    pub fn validate(&self) -> Result<(), GameError> {
        if self.format_version != FORMAT_VERSION {
            return Err(GameError::UnsupportedVersion(self.format_version));
        }
        if self.chunks.len() != 16 || self.chunks.iter().any(|chunk| chunk.tiles.len() != 256) {
            return Err(GameError::InvalidSave("chunk topology is invalid".into()));
        }
        for (index, chunk) in self.chunks.iter().enumerate() {
            let expected = Pos::new(
                (index as i32 % 4) * CHUNK_SIZE,
                (index as i32 / 4) * CHUNK_SIZE,
            );
            if chunk.origin != expected {
                return Err(GameError::InvalidSave(
                    "chunk origin/order is invalid".into(),
                ));
            }
        }
        if !self
            .tile(self.player.position)
            .is_some_and(|tile| tile.terrain.passable() && tile.structure_id.is_none())
        {
            return Err(GameError::InvalidSave(
                "player position is blocked or outside the world".into(),
            ));
        }
        for y in 0..WORLD_SIZE {
            for x in 0..WORLD_SIZE {
                let pos = Pos::new(x, y);
                let tile = self.tile(pos).expect("chunk topology checked");
                if let Some(id) = tile.resource_id
                    && self
                        .resources
                        .get(&id)
                        .is_none_or(|node| node.position != pos)
                {
                    return Err(GameError::InvalidSave(format!(
                        "tile ({x}, {y}) has invalid resource reference"
                    )));
                }
                if let Some(id) = tile.structure_id
                    && (self
                        .structures
                        .get(&id)
                        .is_none_or(|structure| structure.position != pos)
                        || !tile.terrain.passable())
                {
                    return Err(GameError::InvalidSave(format!(
                        "tile ({x}, {y}) has invalid structure reference"
                    )));
                }
            }
        }
        for (key, node) in &self.resources {
            if *key != node.id {
                return Err(GameError::InvalidSave(format!(
                    "resource key {} disagrees with ID {}",
                    key, node.id
                )));
            }
            if self.tile(node.position).and_then(|tile| tile.resource_id) != Some(node.id) {
                return Err(GameError::InvalidSave(format!(
                    "resource {} has broken tile reference",
                    node.id
                )));
            }
        }
        for (key, structure) in &self.structures {
            if *key != structure.id {
                return Err(GameError::InvalidSave(format!(
                    "structure key {} disagrees with ID {}",
                    key, structure.id
                )));
            }
            if self
                .tile(structure.position)
                .and_then(|tile| tile.structure_id)
                != Some(structure.id)
            {
                return Err(GameError::InvalidSave(format!(
                    "structure {} has broken tile reference",
                    structure.id
                )));
            }
            match &structure.state {
                StructureState::Miner {
                    progress,
                    output,
                    resource_id,
                } => {
                    let node = self.resources.get(resource_id).ok_or_else(|| {
                        GameError::InvalidSave(format!("miner {} lacks resource", key))
                    })?;
                    if node.position != structure.position
                        || !node.kind.mineable()
                        || *progress >= 60
                        || *output > MINER_BUFFER_CAPACITY
                    {
                        return Err(GameError::InvalidSave(format!(
                            "miner {} has invalid state",
                            key
                        )));
                    }
                }
                StructureState::Conveyor { .. } => {}
                StructureState::Crafter {
                    recipe,
                    progress,
                    input,
                    output,
                } => {
                    if inventory_total(input) > 24
                        || inventory_total(output) > CRAFTER_OUTPUT_CAPACITY
                        || recipe.is_none() && *progress != 0
                        || recipe.is_some_and(|recipe| *progress > recipe_time(recipe))
                    {
                        return Err(GameError::InvalidSave(format!(
                            "crafter {} has invalid state",
                            key
                        )));
                    }
                }
            }
        }
        if self
            .resources
            .keys()
            .next_back()
            .is_some_and(|max| self.next_resource_id <= *max)
            || self
                .structures
                .keys()
                .next_back()
                .is_some_and(|max| self.next_structure_id <= *max)
        {
            return Err(GameError::InvalidSave(
                "next entity ID is not monotonic".into(),
            ));
        }
        Ok(())
    }

    /// Controlled scenario setup, intentionally separate from gameplay commands.
    #[cfg(any(test, feature = "debug-api"))]
    pub fn debug(&mut self) -> DebugWorld<'_> {
        DebugWorld { simulation: self }
    }
}

#[cfg(any(test, feature = "debug-api"))]
pub struct DebugWorld<'a> {
    simulation: &'a mut FactorySimulation,
}

#[cfg(any(test, feature = "debug-api"))]
impl DebugWorld<'_> {
    pub fn set_terrain(&mut self, pos: Pos, terrain: Terrain) -> Result<(), GameError> {
        let tile = self.simulation.tile(pos).ok_or(GameError::OutOfBounds)?;
        if !terrain.passable()
            && (tile.structure_id.is_some()
                || pos == self.simulation.player.position
                || tile.resource_id.is_some())
        {
            return Err(GameError::Occupied);
        }
        self.simulation
            .tile_mut(pos)
            .ok_or(GameError::OutOfBounds)?
            .terrain = terrain;
        Ok(())
    }

    pub fn grant(&mut self, item: Item, count: u32) {
        *self.simulation.player.inventory.entry(item).or_default() += count;
    }

    pub fn set_resource(
        &mut self,
        pos: Pos,
        kind: ResourceKind,
        quantity: u32,
    ) -> Result<ResourceId, GameError> {
        let tile = self.simulation.tile(pos).ok_or(GameError::OutOfBounds)?;
        if tile.structure_id.is_some() {
            return Err(GameError::Occupied);
        }
        if let Some(old_id) = tile.resource_id {
            self.simulation.resources.remove(&old_id);
        }
        Ok(self.simulation.add_resource(pos, kind, quantity))
    }

    pub fn set_player_position(&mut self, pos: Pos) -> Result<(), GameError> {
        let tile = self.simulation.tile(pos).ok_or(GameError::OutOfBounds)?;
        if !tile.terrain.passable() || tile.structure_id.is_some() {
            return Err(GameError::BlockedTerrain);
        }
        self.simulation.player.position = pos;
        Ok(())
    }

    pub fn set_belt_item(&mut self, id: StructureId, item: Option<Item>) -> Result<(), GameError> {
        let structure = self
            .simulation
            .structures
            .get_mut(&id)
            .ok_or(GameError::NoStructure)?;
        let StructureState::Conveyor { item: slot } = &mut structure.state else {
            return Err(GameError::InvalidTransfer);
        };
        *slot = item;
        Ok(())
    }
}

impl Structure {
    fn snapshot(&self, resources: &BTreeMap<ResourceId, ResourceNode>) -> StructureSnapshot {
        let (recipe, progress, input, output, belt_item, resource_id) = match &self.state {
            StructureState::Miner {
                progress,
                output,
                resource_id,
            } => {
                let item = resources
                    .get(resource_id)
                    .map(|node| node.kind.item())
                    .unwrap_or(Item::Stone);
                (
                    None,
                    *progress,
                    Vec::new(),
                    vec![ItemStack {
                        item,
                        count: *output,
                    }],
                    None,
                    Some(*resource_id),
                )
            }
            StructureState::Conveyor { item } => (None, 0, Vec::new(), Vec::new(), *item, None),
            StructureState::Crafter {
                recipe,
                progress,
                input,
                output,
            } => (
                *recipe,
                *progress,
                stacks(input),
                stacks(output),
                None,
                None,
            ),
        };
        StructureSnapshot {
            id: self.id,
            position: self.position,
            kind: self.kind(),
            direction: self.direction,
            recipe,
            progress,
            input,
            output,
            belt_item,
            resource_id,
        }
    }
}

fn stacks(inventory: &BTreeMap<Item, u32>) -> Vec<ItemStack> {
    inventory
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(item, count)| ItemStack {
            item: *item,
            count: *count,
        })
        .collect()
}

pub fn construction_cost(kind: StructureKind) -> &'static [(Item, u32)] {
    match kind {
        StructureKind::Miner => &[(Item::Stone, 4), (Item::IronOre, 2)],
        StructureKind::Conveyor => &[(Item::Stone, 1)],
        StructureKind::Crafter => &[(Item::Stone, 6), (Item::IronOre, 4)],
    }
}

pub fn recipe_inputs(recipe: Recipe) -> &'static [(Item, u32)] {
    match recipe {
        Recipe::IronPlate => &[(Item::IronOre, 2)],
        Recipe::CopperPlate => &[(Item::CopperOre, 2)],
        Recipe::MechanicalPart => &[(Item::IronPlate, 2), (Item::Wood, 1)],
    }
}

pub const fn recipe_output(recipe: Recipe) -> Item {
    match recipe {
        Recipe::IronPlate => Item::IronPlate,
        Recipe::CopperPlate => Item::CopperPlate,
        Recipe::MechanicalPart => Item::MechanicalPart,
    }
}

pub const fn recipe_time(recipe: Recipe) -> u32 {
    match recipe {
        Recipe::IronPlate | Recipe::CopperPlate => 40,
        Recipe::MechanicalPart => 60,
    }
}

fn inventory_total(inventory: &BTreeMap<Item, u32>) -> u32 {
    inventory.values().sum()
}

fn tile_indices(pos: Pos) -> Option<(usize, usize)> {
    if pos.x < 0 || pos.y < 0 || pos.x >= WORLD_SIZE || pos.y >= WORLD_SIZE {
        return None;
    }
    let chunk = ((pos.y / CHUNK_SIZE) * 4 + pos.x / CHUNK_SIZE) as usize;
    let tile = ((pos.y % CHUNK_SIZE) * CHUNK_SIZE + pos.x % CHUNK_SIZE) as usize;
    Some((chunk, tile))
}

fn terrain_at(seed: u64, pos: Pos) -> Terrain {
    if pos.manhattan(Pos::new(32, 32)) < 12 {
        return Terrain::Grass;
    }
    match coord_hash(seed, pos.x, pos.y) % 100 {
        0..=61 => Terrain::Grass,
        62..=76 => Terrain::Dirt,
        77..=87 => Terrain::Sand,
        88..=93 => Terrain::Water,
        _ => Terrain::Rock,
    }
}

fn coord_hash(seed: u64, x: i32, y: i32) -> u64 {
    let mut value = seed
        ^ (x as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (y as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_generation_is_reproducible_and_has_nearby_resources() {
        let first = FactorySimulation::new(42);
        let second = FactorySimulation::new(42);
        assert_eq!(first.state_hash(), second.state_hash());
        assert_ne!(first.state_hash(), FactorySimulation::new(43).state_hash());
        assert_eq!(first.chunks.len(), 16);
        assert!(first.chunks.iter().all(|chunk| chunk.tiles.len() == 256));
        let reachable = first.reachable_tiles();
        assert!(
            first
                .resources
                .values()
                .all(|node| reachable.contains(&node.position))
        );
        for kind in [
            ResourceKind::Iron,
            ResourceKind::Copper,
            ResourceKind::Stone,
            ResourceKind::Tree,
        ] {
            assert!(first.resources.values().any(|node| {
                node.kind == kind
                    && node.quantity > 0
                    && node.position.manhattan(first.player.position) <= 6
                    && first
                        .tile(node.position)
                        .is_some_and(|tile| tile.terrain.passable())
            }));
        }
        assert_eq!(first.inventory_count(Item::Stone), 20);
        assert_eq!(first.inventory_count(Item::IronOre), 12);
        assert_eq!(first.inventory_count(Item::Wood), 5);
    }

    #[test]
    fn harvesting_respects_range_and_depletion() {
        let mut simulation = FactorySimulation::new(42);
        let position = Pos::new(34, 32);
        let id = simulation
            .debug()
            .set_resource(position, ResourceKind::Iron, 2)
            .unwrap();
        let before = simulation.inventory_count(Item::IronOre);
        for remaining in [1, 0] {
            let events = simulation
                .execute(GameCommand::Harvest { position })
                .unwrap();
            assert_eq!(
                events,
                vec![GameEvent::Harvested {
                    resource_id: id,
                    item: Item::IronOre,
                    remaining
                }]
            );
        }
        assert_eq!(simulation.inventory_count(Item::IronOre), before + 2);
        assert_eq!(simulation.resource_quantity(id), Some(0));
        assert_eq!(
            simulation.execute(GameCommand::Harvest { position }),
            Err(GameError::Depleted)
        );
        assert_eq!(
            simulation.execute(GameCommand::Harvest {
                position: Pos::new(3, 3)
            }),
            Err(GameError::OutOfRange)
        );
    }

    #[test]
    fn movement_checks_terrain_and_world_edges() {
        let mut simulation = FactorySimulation::new(42);
        assert_eq!(
            simulation
                .execute(GameCommand::Move {
                    direction: Direction::East
                })
                .unwrap(),
            vec![GameEvent::PlayerMoved {
                position: Pos::new(33, 32)
            }]
        );
        simulation
            .debug()
            .set_terrain(Pos::new(0, 0), Terrain::Grass)
            .unwrap();
        simulation
            .debug()
            .set_player_position(Pos::new(0, 0))
            .unwrap();
        assert_eq!(
            simulation.execute(GameCommand::Move {
                direction: Direction::West
            }),
            Err(GameError::OutOfBounds)
        );
    }

    #[test]
    fn json_round_trip_preserves_hash() {
        let mut simulation = FactorySimulation::new(7);
        simulation
            .debug()
            .set_player_position(Pos::new(31, 31))
            .unwrap();
        simulation
            .execute(GameCommand::Harvest {
                position: Pos::new(30, 30),
            })
            .unwrap();
        let json = simulation.to_json().unwrap();
        let loaded = FactorySimulation::from_json(&json).unwrap();
        assert_eq!(simulation, loaded);
        assert_eq!(simulation.state_hash(), loaded.state_hash());
    }

    #[test]
    fn format_one_serialization_contract_is_stable() {
        let simulation = FactorySimulation::new(42);
        assert_eq!(simulation.format_version, 1);
        assert_eq!(
            simulation.state_hash(),
            "471fc7420dc39251cd9a846ec503610b942f602d2411fa9419581ec3f0e1e979"
        );
        let value: serde_json::Value =
            serde_json::from_str(&simulation.to_json().unwrap()).unwrap();
        assert_eq!(value["player"]["inventory"]["iron_ore"], 12);
        assert_eq!(
            value["chunks"][0]["origin"],
            serde_json::json!({"x": 0, "y": 0})
        );
    }

    #[test]
    fn miner_stops_at_depleted_deposit() {
        let mut simulation = FactorySimulation::new(42);
        let pos = Pos::new(34, 32);
        let node = simulation
            .debug()
            .set_resource(pos, ResourceKind::Iron, 1)
            .unwrap();
        let miner = simulation
            .execute(GameCommand::Build {
                kind: StructureKind::Miner,
                position: pos,
                direction: Direction::East,
            })
            .unwrap();
        assert!(matches!(miner[0], GameEvent::Built { .. }));
        simulation.step_many(600);
        assert_eq!(simulation.resource_quantity(node), Some(0));
        assert_eq!(
            simulation.structures.values().next().unwrap().state,
            StructureState::Miner {
                progress: 0,
                output: 1,
                resource_id: node
            }
        );
        simulation.validate().unwrap();
    }

    #[test]
    fn placement_rejects_blocked_terrain_and_missing_cost_without_mutation() {
        let mut simulation = FactorySimulation::new(42);
        let blocked = Pos::new(33, 32);
        simulation
            .debug()
            .set_terrain(blocked, Terrain::Water)
            .unwrap();
        let before = simulation.state_hash();
        assert_eq!(
            simulation.execute(GameCommand::Build {
                kind: StructureKind::Conveyor,
                position: blocked,
                direction: Direction::East
            }),
            Err(GameError::BlockedTerrain)
        );
        assert_eq!(simulation.state_hash(), before);
        simulation.player.inventory.insert(Item::Stone, 0);
        let before = simulation.state_hash();
        assert_eq!(
            simulation.execute(GameCommand::Build {
                kind: StructureKind::Conveyor,
                position: Pos::new(32, 33),
                direction: Direction::North
            }),
            Err(GameError::InsufficientMaterials)
        );
        assert_eq!(simulation.state_hash(), before);
    }
}
