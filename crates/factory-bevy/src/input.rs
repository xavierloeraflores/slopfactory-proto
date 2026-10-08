use bevy::{input::mouse::MouseWheel, prelude::*, window::PrimaryWindow};
use factory_protocol::{
    ALL_ITEMS, Direction, GameCommand, Pos, Recipe, StructureKind, TransferRoute,
};

use crate::GameState;

pub fn cursor(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut game: ResMut<GameState>,
) {
    let (camera, camera_transform) = *camera;
    let position = window.cursor_position();
    let tile = position
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
        .map(|world| {
            Pos::new(
                ((world.x + 8.0) / 16.0).floor() as i32,
                ((world.y + 8.0) / 16.0).floor() as i32,
            )
        });
    let over_ui = position.is_some_and(|cursor| {
        let width = window.resolution.width();
        let height = window.resolution.height();
        (cursor.x < 340.0 && cursor.y < 235.0)
            || (cursor.x > width - 380.0 && cursor.y < 440.0)
            || cursor.y > height - 145.0
    });
    if game.cursor_tile != tile || game.cursor_over_ui != over_ui {
        game.cursor_tile = tile;
        game.cursor_over_ui = over_ui;
        game.dirty = true;
    }
}

pub fn keyboard(keys: Res<ButtonInput<KeyCode>>, time: Res<Time>, mut game: ResMut<GameState>) {
    if keys.just_pressed(KeyCode::Digit1) {
        game.tool = Some(StructureKind::Miner);
        game.dirty = true;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        game.tool = Some(StructureKind::Conveyor);
        game.dirty = true;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        game.tool = Some(StructureKind::Crafter);
        game.dirty = true;
    }
    if keys.just_pressed(KeyCode::Escape) {
        if game.menu_open {
            game.menu_open = false;
        } else {
            game.tool = None;
            game.selected_structure = None;
        }
        game.dirty = true;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        if game.tool.is_some() {
            game.direction = game.direction.clockwise();
            game.dirty = true;
        } else if let Some(structure_id) = game.selected_structure {
            let direction = game
                .simulation
                .structure(structure_id)
                .map(|machine| machine.direction.clockwise());
            if let Some(direction) = direction {
                game.execute(GameCommand::Rotate {
                    structure_id,
                    direction,
                });
            }
        }
    }
    if keys.just_pressed(KeyCode::Delete)
        && let Some(structure_id) = game.selected_structure
    {
        game.execute(GameCommand::Remove { structure_id });
    }
    if keys.just_pressed(KeyCode::Space) {
        game.paused = !game.paused;
        game.status = if game.paused {
            "Simulation paused"
        } else {
            "Simulation running"
        }
        .into();
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        game.speed_index = game.speed_index.saturating_sub(1);
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        game.speed_index = (game.speed_index + 1).min(super::SPEEDS.len() - 1);
    }
    if keys.just_pressed(KeyCode::F5) {
        game.save();
    }
    if keys.just_pressed(KeyCode::F9) {
        game.load();
    }
    if keys.just_pressed(KeyCode::KeyE)
        && let Some(id) = game.selected_structure
        && game
            .simulation
            .structure(id)
            .is_some_and(|machine| machine.kind() == StructureKind::Crafter)
    {
        game.menu_open = !game.menu_open;
        game.status = if game.menu_open {
            "Crafter controls open"
        } else {
            "Crafter controls closed"
        }
        .into();
    }
    if game.menu_open
        && let Some(id) = game.selected_structure
    {
        let selected_recipe = if keys.just_pressed(KeyCode::F1) {
            Some(Recipe::IronPlate)
        } else if keys.just_pressed(KeyCode::F2) {
            Some(Recipe::CopperPlate)
        } else if keys.just_pressed(KeyCode::F3) {
            Some(Recipe::MechanicalPart)
        } else {
            None
        };
        if let Some(recipe) = selected_recipe {
            game.execute(GameCommand::Configure {
                structure_id: id,
                recipe,
            });
        }
        if keys.just_pressed(KeyCode::Tab) {
            game.selected_item_index = (game.selected_item_index + 1) % ALL_ITEMS.len();
        }
        let route = if keys.just_pressed(KeyCode::KeyI) {
            Some(TransferRoute::PlayerToInput)
        } else if keys.just_pressed(KeyCode::KeyU) {
            Some(TransferRoute::InputToPlayer)
        } else if keys.just_pressed(KeyCode::KeyO) {
            Some(TransferRoute::OutputToPlayer)
        } else {
            None
        };
        if let Some(route) = route {
            let count = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
                5
            } else {
                1
            };
            let item = ALL_ITEMS[game.selected_item_index];
            game.execute(GameCommand::Transfer {
                structure_id: id,
                route,
                item,
                count,
            });
        }
    }

    game.move_cooldown -= time.delta_secs();
    let directions = [
        (KeyCode::KeyW, Direction::North),
        (KeyCode::KeyD, Direction::East),
        (KeyCode::KeyS, Direction::South),
        (KeyCode::KeyA, Direction::West),
    ];
    if let Some((_, direction)) = directions.iter().find(|(key, _)| keys.pressed(*key)) {
        let immediate = directions.iter().any(|(key, _)| keys.just_pressed(*key));
        if immediate || game.move_cooldown <= 0.0 {
            game.execute(GameCommand::Move {
                direction: *direction,
            });
            game.move_cooldown = 0.12;
        }
    } else {
        game.move_cooldown = 0.0;
    }
}

pub fn mouse(buttons: Res<ButtonInput<MouseButton>>, mut game: ResMut<GameState>) {
    if buttons.just_pressed(MouseButton::Right) && !game.cursor_over_ui {
        let selected = game
            .cursor_tile
            .and_then(|tile| game.simulation.structure_at(tile).map(|machine| machine.id));
        game.selected_structure = selected;
        game.menu_open = false;
        game.status = game.selected_structure.map_or_else(
            || "No structure at cursor".into(),
            |id| format!("Inspecting structure #{id}"),
        );
    }
    if !buttons.pressed(MouseButton::Left) {
        game.last_drag_tile = None;
        return;
    }
    if game.cursor_over_ui {
        return;
    }
    let Some(position) = game.cursor_tile else {
        return;
    };
    if let Some(kind) = game.tool {
        let drag = kind == StructureKind::Conveyor;
        if buttons.just_pressed(MouseButton::Left) || drag && game.last_drag_tile != Some(position)
        {
            let direction = game.direction;
            game.execute(GameCommand::Build {
                kind,
                position,
                direction,
            });
            game.last_drag_tile = Some(position);
        }
    } else if buttons.just_pressed(MouseButton::Left) {
        game.execute(GameCommand::Harvest { position });
    }
}

pub fn zoom(mut wheel: MessageReader<MouseWheel>, camera: Single<&mut Projection, With<Camera2d>>) {
    let mut projection = camera.into_inner();
    for event in wheel.read() {
        if let Projection::Orthographic(orthographic) = &mut *projection {
            orthographic.scale = (orthographic.scale * (1.0 - event.y * 0.12)).clamp(0.2, 1.4);
        }
    }
}
