use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use factory_core::recipe_time;
use factory_protocol::{Direction, Pos, ResourceId, StructureId, StructureKind};

use crate::{
    GameState,
    assets::{SpriteHandles, item_key, resource_key, structure_key, terrain_key},
    ui,
};

const TILE: f32 = 16.0;

#[derive(Resource, Default)]
pub struct RenderEntities {
    resources: BTreeMap<ResourceId, Entity>,
    structures: BTreeMap<StructureId, Entity>,
    items: BTreeMap<StructureId, Entity>,
    progress: BTreeMap<StructureId, Entity>,
    player: Option<Entity>,
    preview: Option<Entity>,
}

#[derive(Component)]
pub(crate) struct PlayerVisual;

#[derive(Component)]
struct PreviewVisual;

#[derive(Component)]
pub(crate) struct BeltItemVisual {
    position: Pos,
    direction: Direction,
}

pub fn world_position(position: Pos, depth: f32) -> Vec3 {
    Vec3::new(position.x as f32 * TILE, position.y as f32 * TILE, depth)
}

fn facing_rotation(direction: Direction) -> Quat {
    let radians = match direction {
        Direction::North => 0.0,
        Direction::East => -std::f32::consts::FRAC_PI_2,
        Direction::South => std::f32::consts::PI,
        Direction::West => std::f32::consts::FRAC_PI_2,
    };
    Quat::from_rotation_z(radians)
}

pub fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    game: Res<GameState>,
    mut entities: ResMut<RenderEntities>,
) {
    let handles = SpriteHandles::load(&server);
    let state = game.simulation.render_snapshot();
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.5,
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_translation(world_position(state.player.position, 1000.0)),
    ));
    for tile in &state.tiles {
        commands.spawn((
            Sprite::from_image(handles.image(terrain_key(tile.terrain))),
            Transform::from_translation(world_position(tile.position, 0.0)),
        ));
    }
    for resource in &state.resources {
        if resource.quantity > 0 {
            let entity = commands
                .spawn((
                    Sprite::from_image(handles.image(resource_key(resource.kind))),
                    Transform::from_translation(world_position(resource.position, 1.0)),
                ))
                .id();
            entities.resources.insert(resource.id, entity);
        }
    }
    entities.player = Some(
        commands
            .spawn((
                Sprite::from_image(handles.image("player")),
                Transform::from_translation(world_position(state.player.position, 4.0)),
                PlayerVisual,
            ))
            .id(),
    );
    entities.preview = Some(
        commands
            .spawn((
                Sprite::from_image(handles.image("conveyor")),
                Transform::from_xyz(0.0, 0.0, 5.0),
                Visibility::Hidden,
                PreviewVisual,
            ))
            .id(),
    );
    ui::setup(&mut commands);
    commands.insert_resource(handles);
}

pub fn sync(
    mut commands: Commands,
    mut game: ResMut<GameState>,
    mut entities: ResMut<RenderEntities>,
    handles: Res<SpriteHandles>,
    mut sprites: Query<&mut Sprite>,
    mut transforms: Query<&mut Transform>,
    mut visibilities: Query<&mut Visibility>,
) {
    if !game.dirty {
        return;
    }
    let state = game.simulation.render_snapshot();
    let live_resources: BTreeSet<_> = state
        .resources
        .iter()
        .filter(|node| node.quantity > 0)
        .map(|node| node.id)
        .collect();
    entities.resources.retain(|id, entity| {
        if live_resources.contains(id) {
            true
        } else {
            commands.entity(*entity).despawn();
            false
        }
    });
    for resource in state.resources.iter().filter(|node| node.quantity > 0) {
        if let Some(entity) = entities.resources.get(&resource.id).copied() {
            if let Ok(mut sprite) = sprites.get_mut(entity) {
                sprite.image = handles.image(resource_key(resource.kind));
            }
            if let Ok(mut transform) = transforms.get_mut(entity) {
                transform.translation = world_position(resource.position, 1.0);
            }
        } else {
            let entity = commands
                .spawn((
                    Sprite::from_image(handles.image(resource_key(resource.kind))),
                    Transform::from_translation(world_position(resource.position, 1.0)),
                ))
                .id();
            entities.resources.insert(resource.id, entity);
        }
    }

    let live_structures: BTreeSet<_> = state.structures.iter().map(|machine| machine.id).collect();
    entities.structures.retain(|id, entity| {
        if live_structures.contains(id) {
            true
        } else {
            commands.entity(*entity).despawn();
            false
        }
    });
    entities.items.retain(|id, entity| {
        if live_structures.contains(id) {
            true
        } else {
            commands.entity(*entity).despawn();
            false
        }
    });
    entities.progress.retain(|id, entity| {
        if live_structures.contains(id) {
            true
        } else {
            commands.entity(*entity).despawn();
            false
        }
    });
    for machine in &state.structures {
        if let Some(entity) = entities.structures.get(&machine.id).copied() {
            if let Ok(mut sprite) = sprites.get_mut(entity) {
                sprite.image = handles.image(structure_key(machine.kind));
            }
            if let Ok(mut transform) = transforms.get_mut(entity) {
                transform.translation = world_position(machine.position, 2.0);
                transform.rotation = facing_rotation(machine.direction);
            }
        } else {
            let entity = commands
                .spawn((
                    Sprite::from_image(handles.image(structure_key(machine.kind))),
                    Transform::from_translation(world_position(machine.position, 2.0))
                        .with_rotation(facing_rotation(machine.direction)),
                ))
                .id();
            entities.structures.insert(machine.id, entity);
        }
        if let Some(item) = machine.belt_item {
            if let Some(entity) = entities.items.get(&machine.id).copied() {
                if let Ok(mut sprite) = sprites.get_mut(entity) {
                    sprite.image = handles.image(item_key(item));
                }
                if let Ok(mut transform) = transforms.get_mut(entity) {
                    transform.translation = world_position(machine.position, 3.0);
                }
                commands.entity(entity).insert(BeltItemVisual {
                    position: machine.position,
                    direction: machine.direction,
                });
            } else {
                let entity = commands
                    .spawn((
                        Sprite::from_image(handles.image(item_key(item))),
                        Transform::from_translation(world_position(machine.position, 3.0))
                            .with_scale(Vec3::splat(0.65)),
                        BeltItemVisual {
                            position: machine.position,
                            direction: machine.direction,
                        },
                    ))
                    .id();
                entities.items.insert(machine.id, entity);
            }
        } else if let Some(entity) = entities.items.remove(&machine.id) {
            commands.entity(entity).despawn();
        }
        if machine.kind == StructureKind::Crafter {
            let width = machine.recipe.map_or(0.0, |recipe| {
                12.0 * machine.progress as f32 / recipe_time(recipe) as f32
            });
            if width > 0.0 {
                if let Some(entity) = entities.progress.get(&machine.id).copied() {
                    if let Ok(mut sprite) = sprites.get_mut(entity) {
                        sprite.custom_size = Some(Vec2::new(width, 2.0));
                    }
                    if let Ok(mut transform) = transforms.get_mut(entity) {
                        transform.translation =
                            world_position(machine.position, 3.5) + Vec3::new(0.0, -7.0, 0.0);
                    }
                } else {
                    let mut sprite =
                        Sprite::from_color(Color::srgb(0.95, 0.73, 0.35), Vec2::new(width, 2.0));
                    sprite.custom_size = Some(Vec2::new(width, 2.0));
                    let entity = commands
                        .spawn((
                            sprite,
                            Transform::from_translation(
                                world_position(machine.position, 3.5) + Vec3::new(0.0, -7.0, 0.0),
                            ),
                        ))
                        .id();
                    entities.progress.insert(machine.id, entity);
                }
            } else if let Some(entity) = entities.progress.remove(&machine.id) {
                commands.entity(entity).despawn();
            }
        } else if let Some(entity) = entities.progress.remove(&machine.id) {
            commands.entity(entity).despawn();
        }
    }

    if let Some(preview) = entities.preview {
        if let (Some(tool), Some(position)) = (game.tool, game.cursor_tile) {
            if !game.cursor_over_ui {
                if let Ok(mut sprite) = sprites.get_mut(preview) {
                    sprite.image = handles.image(structure_key(tool));
                    sprite.color = if game.simulation.can_build(tool, position).is_ok() {
                        Color::srgba(0.55, 1.0, 0.62, 0.65)
                    } else {
                        Color::srgba(1.0, 0.45, 0.44, 0.65)
                    };
                }
                if let Ok(mut transform) = transforms.get_mut(preview) {
                    transform.translation = world_position(position, 5.0);
                    transform.rotation = facing_rotation(game.direction);
                }
                if let Ok(mut visibility) = visibilities.get_mut(preview) {
                    *visibility = Visibility::Visible;
                }
            } else if let Ok(mut visibility) = visibilities.get_mut(preview) {
                *visibility = Visibility::Hidden;
            }
        } else if let Ok(mut visibility) = visibilities.get_mut(preview) {
            *visibility = Visibility::Hidden;
        }
    }
    if game
        .selected_structure
        .is_some_and(|id| !live_structures.contains(&id))
    {
        game.selected_structure = None;
        game.menu_open = false;
    }
    game.dirty = false;
}

#[allow(clippy::type_complexity)] // Bevy needs disjoint mutable Transform queries for these visuals.
pub fn animate(
    time: Res<Time>,
    game: Res<GameState>,
    mut player: Query<
        &mut Transform,
        (
            With<PlayerVisual>,
            Without<Camera2d>,
            Without<BeltItemVisual>,
        ),
    >,
    mut camera: Query<
        &mut Transform,
        (
            With<Camera2d>,
            Without<PlayerVisual>,
            Without<BeltItemVisual>,
        ),
    >,
    mut items: Query<(&BeltItemVisual, &mut Transform), (Without<PlayerVisual>, Without<Camera2d>)>,
) {
    let target = world_position(game.simulation.player_position(), 4.0);
    if let Ok(mut transform) = player.single_mut() {
        let factor = 1.0 - (-14.0 * time.delta_secs()).exp();
        transform.translation = transform.translation.lerp(target, factor);
        if let Ok(mut camera_transform) = camera.single_mut() {
            let camera_target = Vec3::new(transform.translation.x, transform.translation.y, 1000.0);
            let camera_factor = 1.0 - (-7.0 * time.delta_secs()).exp();
            camera_transform.translation = camera_transform
                .translation
                .lerp(camera_target, camera_factor);
        }
    }
    let phase = ((game.simulation.tick() % 5) as f32
        + (game.accumulator / super::STEP_SECONDS) as f32)
        .clamp(0.0, 5.0)
        / 5.0;
    for (marker, mut transform) in &mut items {
        let (dx, dy) = marker.direction.delta();
        transform.translation = world_position(marker.position, 3.0)
            + Vec3::new(dx as f32, dy as f32, 0.0) * ((phase - 0.5) * 6.0);
    }
}
