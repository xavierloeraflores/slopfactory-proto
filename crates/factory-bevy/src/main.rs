mod assets;
mod input;
mod presentation;
mod ui;

use std::{
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::{asset::AssetPlugin, prelude::*};
use factory_core::FactorySimulation;
use factory_protocol::{Direction, GameCommand, GameEvent, Pos, StructureId, StructureKind};

const STEP_SECONDS: f64 = 1.0 / 20.0;
const SPEEDS: [f64; 5] = [0.25, 0.5, 1.0, 2.0, 4.0];

#[derive(Resource)]
struct GameState {
    simulation: FactorySimulation,
    save_path: PathBuf,
    accumulator: f64,
    speed_index: usize,
    paused: bool,
    dirty: bool,
    tool: Option<StructureKind>,
    direction: Direction,
    selected_structure: Option<StructureId>,
    menu_open: bool,
    selected_item_index: usize,
    cursor_tile: Option<Pos>,
    cursor_over_ui: bool,
    last_drag_tile: Option<Pos>,
    move_cooldown: f32,
    status: String,
}

impl GameState {
    fn execute(&mut self, command: GameCommand) {
        match self.simulation.execute(command) {
            Ok(events) => {
                self.status = events
                    .first()
                    .map_or_else(|| "Action completed".into(), describe_event);
                self.dirty = true;
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    fn save(&mut self) {
        let result = save_atomic(&self.save_path, &self.simulation);
        match result {
            Ok(()) => self.status = format!("Saved {}", self.save_path.display()),
            Err(error) => self.status = format!("Save failed: {error}"),
        }
    }

    fn load(&mut self) {
        let result = fs::read_to_string(&self.save_path)
            .map_err(|error| error.to_string())
            .and_then(|json| {
                FactorySimulation::from_json(&json).map_err(|error| error.to_string())
            });
        match result {
            Ok(simulation) => {
                self.simulation = simulation;
                self.selected_structure = None;
                self.menu_open = false;
                self.dirty = true;
                self.status = format!("Loaded {}", self.save_path.display());
            }
            Err(error) => self.status = format!("Load failed: {error}"),
        }
    }
}

fn save_atomic(path: &Path, simulation: &FactorySimulation) -> Result<(), Box<dyn Error>> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let file_name = path.file_name().ok_or("save path has no file name")?;
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let temporary = parent.join(format!(
        ".{}.{}.{}.tmp",
        file_name.to_string_lossy(),
        std::process::id(),
        nonce
    ));
    let result = (|| -> Result<(), Box<dyn Error>> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(simulation.to_json()?.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn describe_event(event: &GameEvent) -> String {
    match event {
        GameEvent::PlayerMoved { position } => {
            format!("Moved to ({}, {})", position.x, position.y)
        }
        GameEvent::Harvested {
            item, remaining, ..
        } => format!("Harvested {item:?}; {remaining} left"),
        GameEvent::Built { kind, position, .. } => {
            format!("Placed {kind:?} at ({}, {})", position.x, position.y)
        }
        GameEvent::Removed { structure_id } => format!("Removed structure #{structure_id}"),
        GameEvent::Rotated {
            structure_id,
            direction,
        } => format!("Structure #{structure_id} faces {direction:?}"),
        GameEvent::Configured { recipe, .. } => format!("Recipe set to {recipe:?}"),
        GameEvent::Transferred { item, count, .. } => {
            format!("Transferred {count} {item:?}")
        }
        GameEvent::Produced { item, count, .. } => format!("Produced {count} {item:?}"),
        GameEvent::ObjectiveCompleted => "Objective complete!".into(),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut seed = 42;
    let mut world = None;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--seed" => seed = args.next().ok_or("--seed needs a value")?.parse()?,
            "--world" => world = Some(PathBuf::from(args.next().ok_or("--world needs a path")?)),
            _ => return Err(format!("unknown argument: {argument}").into()),
        }
    }
    let save_path = world.unwrap_or_else(|| PathBuf::from("tiny-factory-save.json"));
    let asset_path = asset_directory()?;
    let simulation = if save_path.exists() {
        FactorySimulation::from_json(&fs::read_to_string(&save_path)?)?
    } else {
        FactorySimulation::new(seed)
    };
    let initial_status = if simulation.produced_parts() >= 10 {
        "Objective complete! The factory keeps running."
    } else {
        "WASD to move. Right click a crafter, then E to interact."
    };
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: asset_path.to_string_lossy().into_owned(),
                    ..default()
                })
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Tiny Factory".into(),
                        resolution: (1180, 760).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb(0.10, 0.15, 0.18)))
        .insert_resource(GameState {
            simulation,
            save_path,
            accumulator: 0.0,
            speed_index: 2,
            paused: false,
            dirty: true,
            tool: None,
            direction: Direction::North,
            selected_structure: None,
            menu_open: false,
            selected_item_index: 0,
            cursor_tile: None,
            cursor_over_ui: false,
            last_drag_tile: None,
            move_cooldown: 0.0,
            status: initial_status.into(),
        })
        .init_resource::<presentation::RenderEntities>()
        .add_systems(Startup, presentation::setup)
        .add_systems(
            Update,
            (
                input::cursor,
                input::keyboard,
                input::mouse,
                input::zoom,
                advance_simulation,
                presentation::sync,
                presentation::animate,
                ui::update,
            )
                .chain(),
        )
        .run();
    Ok(())
}

fn asset_directory() -> Result<PathBuf, Box<dyn Error>> {
    let mut candidates = Vec::new();
    candidates.push(std::env::current_dir()?.join("assets"));
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        candidates.push(parent.join("assets"));
    }
    if let Some(workspace) = PathBuf::from(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2) {
        candidates.push(workspace.join("assets"));
    }
    candidates
        .into_iter()
        .find(|path| path.join("sprites/grass.png").is_file())
        .ok_or_else(|| "sprite assets not found beside the working directory or executable".into())
}

fn advance_simulation(time: Res<Time>, mut game: ResMut<GameState>) {
    if game.paused {
        return;
    }
    game.accumulator += time.delta_secs_f64() * SPEEDS[game.speed_index];
    let mut steps = 0;
    while game.accumulator >= STEP_SECONDS && steps < 100 {
        let events = game.simulation.step();
        game.accumulator -= STEP_SECONDS;
        game.dirty = true;
        for event in events {
            if matches!(event, factory_protocol::GameEvent::ObjectiveCompleted) {
                game.status = "Objective complete! The factory keeps running.".into();
            }
        }
        steps += 1;
    }
}
