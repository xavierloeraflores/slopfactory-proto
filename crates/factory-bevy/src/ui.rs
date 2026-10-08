use std::collections::BTreeMap;

use bevy::{prelude::*, text::FontSize};
use factory_core::{StructureState, recipe_time};
use factory_protocol::{ALL_ITEMS, Item};

use crate::{GameState, SPEEDS};

#[derive(Component)]
pub(crate) enum TextKind {
    Hud,
    Toolbar,
    Inspector,
    Status,
}

pub fn setup(commands: &mut Commands) {
    let panel = Color::srgba(0.10, 0.16, 0.20, 0.92);
    let foreground = Color::srgb(0.93, 0.93, 0.82);
    commands.spawn((
        Text::new("Tiny Factory"),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(foreground),
        Node {
            position_type: PositionType::Absolute,
            left: px(16.0),
            top: px(16.0),
            width: px(322.0),
            padding: UiRect::all(px(12.0)),
            ..default()
        },
        BackgroundColor(panel),
        TextKind::Hud,
    ));
    commands.spawn((
        Text::new("Controls"),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(foreground),
        Node {
            position_type: PositionType::Absolute,
            left: px(16.0),
            bottom: px(16.0),
            width: px(700.0),
            padding: UiRect::all(px(12.0)),
            ..default()
        },
        BackgroundColor(panel),
        TextKind::Toolbar,
    ));
    commands.spawn((
        Text::new("Right-click a structure to inspect"),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(foreground),
        Node {
            position_type: PositionType::Absolute,
            right: px(16.0),
            top: px(16.0),
            width: px(354.0),
            padding: UiRect::all(px(12.0)),
            ..default()
        },
        BackgroundColor(panel),
        TextKind::Inspector,
    ));
    commands.spawn((
        Text::new("Ready"),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.78, 0.46)),
        Node {
            position_type: PositionType::Absolute,
            right: px(16.0),
            bottom: px(16.0),
            width: px(430.0),
            padding: UiRect::all(px(12.0)),
            ..default()
        },
        BackgroundColor(panel),
        TextKind::Status,
    ));
}

pub fn update(game: Res<GameState>, mut texts: Query<(&mut Text, &TextKind)>) {
    let parts = game.simulation.produced_parts();
    let progress = format!(
        "[{}{}]",
        "#".repeat(parts.min(10) as usize),
        ".".repeat(10usize.saturating_sub(parts as usize))
    );
    let completion = if parts >= 10 {
        "FACTORY GOAL COMPLETE\n"
    } else {
        ""
    };
    let inventory = ALL_ITEMS
        .iter()
        .map(|item| {
            format!(
                "{:>16}: {}",
                item_name(*item),
                game.simulation.inventory_count(*item)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let hud = format!(
        "TINY FACTORY  |  seed {}\nObjective: {parts}/10 parts {progress}\n{completion}Tick {}  |  {}  |  {}x\nPlayer ({}, {})\n\nINVENTORY\n{}",
        game.simulation.seed(),
        game.simulation.tick(),
        if game.paused { "PAUSED" } else { "RUNNING" },
        SPEEDS[game.speed_index],
        game.simulation.player_position().x,
        game.simulation.player_position().y,
        inventory
    );
    let cursor = game
        .cursor_tile
        .map_or_else(|| "offscreen".into(), |p| format!("{}, {}", p.x, p.y));
    let toolbar = format!(
        "1 Miner  2 Belt  3 Crafter  |  R Rotate  Esc Cancel\nWASD Move  |  Left Harvest/Build  Right Inspect  E Interact\nSpace Pause  [ ] Speed  Wheel Zoom  Del Remove  F5 Save  F9 Load\nTool: {:?}  Facing: {:?}  Tile: {cursor}",
        game.tool, game.direction
    );
    let inspector = inspect_text(&game);
    for (mut text, kind) in &mut texts {
        let content = match kind {
            TextKind::Hud => &hud,
            TextKind::Toolbar => &toolbar,
            TextKind::Inspector => &inspector,
            TextKind::Status => &game.status,
        };
        if text.0 != *content {
            text.0.clone_from(content);
        }
    }
}

fn inspect_text(game: &GameState) -> String {
    let Some(id) = game.selected_structure else {
        return "INSPECT\nRight-click a structure.\nSelect a crafter, then press E to open controls.".into();
    };
    let Some(machine) = game.simulation.structure(id) else {
        return "INSPECT\nStructure removed".into();
    };
    let mut description = format!(
        "INSPECT  #{} {:?}\nPosition ({}, {})  Facing {:?}\n",
        id,
        machine.kind(),
        machine.position.x,
        machine.position.y,
        machine.direction
    );
    match &machine.state {
        StructureState::Miner {
            progress,
            output,
            resource_id,
        } => {
            description.push_str(&format!("Extraction: {progress}/60 ticks\nOutput buffer: {output}/4\nDeposit remaining: {}\n",
                game.simulation.resource_quantity(*resource_id).unwrap_or(0)));
        }
        StructureState::Conveyor { item } => description.push_str(&format!(
            "Belt item: {:?}\nOne hop every five ticks\n",
            item
        )),
        StructureState::Crafter {
            recipe,
            progress,
            input,
            output,
        } => {
            description.push_str(&format!(
                "Recipe: {:?}\nProgress: {progress}/{} ticks\nInput: {}\nOutput: {}\n",
                recipe,
                recipe.map_or(0, recipe_time),
                stacks(input),
                stacks(output)
            ));
            description.push_str("\nE: open/close crafter controls\n");
            if game.menu_open {
                description.push_str(&format!("F1 Iron plate  F2 Copper plate\nF3 Mechanical part\nTab Item: {}\nI Insert  U Take input  O Take output\nShift + transfer key: five items\n", item_name(ALL_ITEMS[game.selected_item_index])));
            }
        }
    }
    description.push_str("\nR Rotate  |  Del Remove");
    description
}

fn stacks(items: &BTreeMap<Item, u32>) -> String {
    let parts: Vec<_> = items
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(item, count)| format!("{}×{}", count, item_name(*item)))
        .collect();
    if parts.is_empty() {
        "empty".into()
    } else {
        parts.join(", ")
    }
}

fn item_name(item: Item) -> &'static str {
    match item {
        Item::IronOre => "Iron ore",
        Item::CopperOre => "Copper ore",
        Item::Stone => "Stone",
        Item::Wood => "Wood",
        Item::IronPlate => "Iron plate",
        Item::CopperPlate => "Copper plate",
        Item::MechanicalPart => "Mechanical part",
    }
}
