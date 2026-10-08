use std::collections::BTreeMap;

use bevy::prelude::*;
use factory_protocol::{Item, ResourceKind, StructureKind, Terrain};

#[derive(Resource)]
pub struct SpriteHandles(BTreeMap<&'static str, Handle<Image>>);

impl SpriteHandles {
    pub fn load(server: &AssetServer) -> Self {
        let names = [
            "grass",
            "dirt",
            "sand",
            "water",
            "rock",
            "iron_deposit",
            "copper_deposit",
            "stone_deposit",
            "tree",
            "miner",
            "conveyor",
            "crafter",
            "player",
            "iron_ore",
            "copper_ore",
            "stone",
            "wood",
            "iron_plate",
            "copper_plate",
            "mechanical_part",
        ];
        Self(
            names
                .into_iter()
                .map(|name| (name, server.load(format!("sprites/{name}.png"))))
                .collect(),
        )
    }

    pub fn image(&self, key: &'static str) -> Handle<Image> {
        self.0
            .get(key)
            .expect("all portable sprite keys have an asset")
            .clone()
    }
}

pub const fn terrain_key(terrain: Terrain) -> &'static str {
    match terrain {
        Terrain::Grass => "grass",
        Terrain::Dirt => "dirt",
        Terrain::Sand => "sand",
        Terrain::Water => "water",
        Terrain::Rock => "rock",
    }
}

pub const fn resource_key(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Iron => "iron_deposit",
        ResourceKind::Copper => "copper_deposit",
        ResourceKind::Stone => "stone_deposit",
        ResourceKind::Tree => "tree",
    }
}

pub const fn structure_key(kind: StructureKind) -> &'static str {
    match kind {
        StructureKind::Miner => "miner",
        StructureKind::Conveyor => "conveyor",
        StructureKind::Crafter => "crafter",
    }
}

pub const fn item_key(item: Item) -> &'static str {
    match item {
        Item::IronOre => "iron_ore",
        Item::CopperOre => "copper_ore",
        Item::Stone => "stone",
        Item::Wood => "wood",
        Item::IronPlate => "iron_plate",
        Item::CopperPlate => "copper_plate",
        Item::MechanicalPart => "mechanical_part",
    }
}
