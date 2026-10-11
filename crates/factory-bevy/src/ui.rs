use std::collections::BTreeMap;

use bevy::{prelude::*, text::FontSize};
use factory_core::{StructureState, recipe_inputs, recipe_time};
use factory_protocol::{ALL_ITEMS, GameCommand, Item, Recipe, StructureKind, TransferRoute};

use crate::{GameState, SPEEDS};

#[derive(Component)]
pub(crate) enum TextKind {
    Hud,
    Toolbar,
    Inspector,
    Status,
    CrafterState,
    ItemCounts(Item),
}

const RECIPES: [Recipe; 3] = [
    Recipe::IronPlate,
    Recipe::CopperPlate,
    Recipe::MechanicalPart,
];

#[derive(Component)]
pub(crate) struct CrafterPanel;

#[derive(Component, Clone, Copy)]
pub(crate) enum UiAction {
    Recipe(Recipe),
    Transfer(TransferRoute, Item),
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
            width: px(430.0),
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
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(16.0),
                top: px(145.0),
                width: px(430.0),
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                padding: UiRect::all(px(10.0)),
                ..default()
            },
            BackgroundColor(panel),
            CrafterPanel,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("RECIPES  •  click to select"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(foreground),
            ));
            for recipe in RECIPES {
                let ingredients = recipe_inputs(recipe)
                    .iter()
                    .map(|(item, count)| format!("{count} {}", item_name(*item)))
                    .collect::<Vec<_>>()
                    .join(" + ");
                parent
                    .spawn((
                        Button,
                        Node {
                            width: percent(100.0),
                            height: px(30.0),
                            padding: UiRect::axes(px(8.0), px(4.0)),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.20, 0.29, 0.33)),
                        UiAction::Recipe(recipe),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new(format!(
                                "{}  ←  {}  ({} ticks)",
                                recipe_name(recipe),
                                ingredients,
                                recipe_time(recipe)
                            )),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(foreground),
                        ));
                    });
            }
            parent.spawn((
                Text::new("BLOCKED: Select a recipe."),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.78, 0.46)),
                TextKind::CrafterState,
            ));
            parent.spawn((
                Text::new("ITEMS                         Bag / In / Out"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(foreground),
            ));
            for item in ALL_ITEMS {
                parent
                    .spawn((Node {
                        width: percent(100.0),
                        height: px(28.0),
                        align_items: AlignItems::Center,
                        column_gap: px(3.0),
                        ..default()
                    },))
                    .with_children(|row| {
                        row.spawn((
                            Text::new(item_name(item)),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(foreground),
                            Node {
                                width: px(230.0),
                                ..default()
                            },
                            TextKind::ItemCounts(item),
                        ));
                        for (route, caption) in [
                            (TransferRoute::PlayerToInput, "Add"),
                            (TransferRoute::InputToPlayer, "Back"),
                            (TransferRoute::OutputToPlayer, "Take"),
                        ] {
                            row.spawn((
                                Button,
                                Node {
                                    width: px(54.0),
                                    height: px(25.0),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.20, 0.29, 0.33)),
                                UiAction::Transfer(route, item),
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    Text::new(caption),
                                    TextFont {
                                        font_size: FontSize::Px(12.0),
                                        ..default()
                                    },
                                    TextColor(foreground),
                                ));
                            });
                        }
                    });
            }
            parent.spawn((
                Text::new("Click Add / Back / Take for 1 item; Shift-click for 5."),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(foreground),
            ));
        });
}

pub fn actions(
    buttons: Query<(&Interaction, &UiAction), Changed<Interaction>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<GameState>,
) {
    if !game.menu_open {
        return;
    }
    let Some(structure_id) = game.selected_structure else {
        return;
    };
    if !game
        .simulation
        .structure(structure_id)
        .is_some_and(|machine| machine.kind() == StructureKind::Crafter)
    {
        return;
    }
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *action {
            UiAction::Recipe(recipe) => game.execute(GameCommand::Configure {
                structure_id,
                recipe,
            }),
            UiAction::Transfer(route, item) => {
                let count = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)
                {
                    5
                } else {
                    1
                };
                game.execute(GameCommand::Transfer {
                    structure_id,
                    route,
                    item,
                    count,
                });
            }
        }
    }
}

pub fn update(
    game: Res<GameState>,
    mut texts: Query<(&mut Text, &TextKind)>,
    mut panel: Single<&mut Node, With<CrafterPanel>>,
    mut buttons: Query<(&UiAction, &Interaction, &mut BackgroundColor)>,
) {
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
        "1 Miner  2 Belt  3 Crafter  |  R Rotate  Esc Cancel\nWASD Move  |  Left Harvest/Build  Right Inspect  E Crafter controls\nSpace Pause  [ ] Speed  Wheel Zoom  Del Remove  F5 Save  F9 Load\nTool: {:?}  Facing: {:?}  Tile: {cursor}",
        game.tool, game.direction
    );
    let inspector = inspect_text(&game);
    let crafter = game
        .selected_structure
        .and_then(|id| game.simulation.structure(id))
        .filter(|machine| machine.kind() == StructureKind::Crafter && game.menu_open);
    panel.display = if crafter.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    let state = crafter.and_then(|machine| {
        if let StructureState::Crafter {
            recipe,
            progress,
            input,
            output,
        } = &machine.state
        {
            Some((recipe, progress, input, output))
        } else {
            None
        }
    });
    let machine_status = state.map_or_else(String::new, |(recipe, progress, input, output)| {
        crafter_status(*recipe, *progress, input, output)
    });
    for (mut text, kind) in &mut texts {
        let content = match kind {
            TextKind::Hud => hud.clone(),
            TextKind::Toolbar => toolbar.clone(),
            TextKind::Inspector => inspector.clone(),
            TextKind::Status => game.status.clone(),
            TextKind::CrafterState => machine_status.clone(),
            TextKind::ItemCounts(item) => {
                let (input, output) = state.map_or((0, 0), |(_, _, input, output)| {
                    (
                        input.get(item).copied().unwrap_or(0),
                        output.get(item).copied().unwrap_or(0),
                    )
                });
                format!(
                    "{:<16} {:>2} / {:>2} / {:>2}",
                    item_name(*item),
                    game.simulation.inventory_count(*item),
                    input,
                    output
                )
            }
        };
        if text.0 != content {
            text.0 = content;
        }
    }
    for (action, interaction, mut background) in &mut buttons {
        let selected = matches!(action, UiAction::Recipe(recipe)
            if state.is_some_and(|(selected, _, _, _)| *selected == Some(*recipe)));
        let available = match action {
            UiAction::Recipe(_) => true,
            UiAction::Transfer(route, item) => match route {
                TransferRoute::PlayerToInput => game.simulation.inventory_count(*item) > 0,
                TransferRoute::InputToPlayer => {
                    state.is_some_and(|(_, _, input, _)| input.get(item).copied().unwrap_or(0) > 0)
                }
                TransferRoute::OutputToPlayer => state
                    .is_some_and(|(_, _, _, output)| output.get(item).copied().unwrap_or(0) > 0),
            },
        };
        background.0 = if selected {
            Color::srgb(0.32, 0.57, 0.40)
        } else if !available {
            Color::srgb(0.15, 0.20, 0.23)
        } else if *interaction == Interaction::Hovered {
            Color::srgb(0.34, 0.47, 0.50)
        } else {
            Color::srgb(0.20, 0.29, 0.33)
        };
    }
}

fn inspect_text(game: &GameState) -> String {
    let Some(id) = game.selected_structure else {
        return "INSPECT\nRight-click a structure.\nRight-click a crafter for controls.".into();
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
                "Recipe: {}  |  Progress: {progress}/{}\n",
                recipe.map_or("None", recipe_name),
                recipe.map_or(0, recipe_time)
            ));
            if !game.menu_open {
                description.push_str(&format!(
                    "Input: {}  |  Output: {}\n",
                    stacks(input),
                    stacks(output)
                ));
                description.push_str(&format!(
                    "{}\n",
                    crafter_status(*recipe, *progress, input, output)
                ));
            }
            description.push_str("E: toggle clickable controls\n");
        }
    }
    description.push_str("\nR Rotate  |  Del Remove");
    description
}

fn crafter_status(
    recipe: Option<Recipe>,
    progress: u32,
    input: &BTreeMap<Item, u32>,
    output: &BTreeMap<Item, u32>,
) -> String {
    let Some(recipe) = recipe else {
        return "BLOCKED: Select a recipe.".into();
    };
    let output_count: u32 = output.values().sum();
    if output_count >= 4 {
        return format!("BLOCKED: Output full ({output_count}/4). Take items or connect a belt.");
    }
    if progress > 0 {
        return format!("CRAFTING: {progress}/{} ticks", recipe_time(recipe));
    }
    let missing = recipe_inputs(recipe)
        .iter()
        .filter_map(|(item, needed)| {
            let held = input.get(item).copied().unwrap_or(0);
            (held < *needed).then(|| format!("{} {held}/{needed}", item_name(*item)))
        })
        .collect::<Vec<_>>();
    if missing.is_empty() {
        "READY: Ingredients loaded.".into()
    } else {
        format!(
            "BLOCKED: Missing input: {}. Use Add or a belt.",
            missing.join(", ")
        )
    }
}

fn recipe_name(recipe: Recipe) -> &'static str {
    match recipe {
        Recipe::IronPlate => "Iron plate",
        Recipe::CopperPlate => "Copper plate",
        Recipe::MechanicalPart => "Mechanical part",
    }
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
