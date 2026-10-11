mod benchmark;
mod scenario;

use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::{Parser, Subcommand};
use factory_core::{FactorySimulation, GameError};
use factory_protocol::{GameCommand, Item, Pos, ResourceKind, Terrain};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

#[derive(Parser)]
#[command(
    name = "factoryctl",
    about = "Headless Tiny Factory world editor and simulator"
)]
struct Cli {
    #[command(subcommand)]
    command: CtlCommand,
}

#[derive(Subcommand)]
enum CtlCommand {
    New {
        #[arg(long)]
        seed: u64,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Step {
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        ticks: u64,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    Summary {
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Inspect {
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        x: i32,
        #[arg(long)]
        y: i32,
        #[arg(long)]
        json: bool,
    },
    Place {
        kind: String,
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        x: i32,
        #[arg(long)]
        y: i32,
        #[arg(long)]
        direction: String,
        #[arg(long)]
        json: bool,
    },
    Harvest {
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        x: i32,
        #[arg(long)]
        y: i32,
        #[arg(long)]
        json: bool,
    },
    Move {
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        direction: String,
        #[arg(long)]
        json: bool,
    },
    Rotate {
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        id: u64,
        #[arg(long)]
        direction: String,
        #[arg(long)]
        json: bool,
    },
    Configure {
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        id: u64,
        #[arg(long)]
        recipe: String,
        #[arg(long)]
        json: bool,
    },
    Transfer {
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        id: u64,
        #[arg(long)]
        route: String,
        #[arg(long)]
        item: String,
        #[arg(long)]
        count: u32,
        #[arg(long)]
        json: bool,
    },
    Remove {
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        id: u64,
        #[arg(long)]
        json: bool,
    },
    Validate {
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Scenario {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Bench {
        /// Saved factory layouts to compare. Repeat --world for each layout.
        #[arg(long, required = true)]
        world: Vec<PathBuf>,
        /// Label this executable build in the report (for example, a Git SHA).
        #[arg(long)]
        build_label: Option<String>,
        #[arg(long, default_value_t = 1_000, value_parser = clap::value_parser!(u64).range(1..))]
        ticks: u64,
        #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u64).range(1..))]
        samples: u64,
        #[arg(long, default_value_t = 1)]
        warmup: u64,
        #[arg(long)]
        json: bool,
    },
    Debug {
        #[command(subcommand)]
        action: DebugAction,
    },
}

#[derive(Subcommand)]
enum DebugAction {
    Grant {
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        item: String,
        #[arg(long)]
        count: u32,
        #[arg(long)]
        json: bool,
    },
    Resource {
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        x: i32,
        #[arg(long)]
        y: i32,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        quantity: u32,
        #[arg(long)]
        json: bool,
    },
    Player {
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        x: i32,
        #[arg(long)]
        y: i32,
        #[arg(long)]
        json: bool,
    },
    Terrain {
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        x: i32,
        #[arg(long)]
        y: i32,
        #[arg(long)]
        terrain: String,
        #[arg(long)]
        json: bool,
    },
    Belt {
        #[arg(long)]
        world: PathBuf,
        #[arg(long)]
        id: u64,
        #[arg(long)]
        item: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

type Response = Result<(String, Value, bool), Box<dyn std::error::Error>>;

fn main() -> ExitCode {
    let json_requested = std::env::args().any(|arg| arg == "--json");
    match run(Cli::parse().command) {
        Ok((human, value, as_json)) => {
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&value).expect("JSON response")
                );
            } else {
                println!("{human}");
            }
            if value.get("ok") == Some(&Value::Bool(false)) {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            if json_requested {
                println!("{}", json!({"ok": false, "error": error.to_string()}));
            } else {
                eprintln!("error: {error}");
            }
            ExitCode::FAILURE
        }
    }
}

fn run(command: CtlCommand) -> Response {
    match command {
        CtlCommand::New { seed, output, json } => {
            let sim = FactorySimulation::new(seed);
            save_atomic(&output, &sim)?;
            Ok((
                format!(
                    "Created seed {seed} world at {} (hash {})",
                    output.display(),
                    sim.state_hash()
                ),
                json!({"ok": true, "command": "new", "output": output, "tick": sim.tick(), "state_hash": sim.state_hash()}),
                json,
            ))
        }
        CtlCommand::Step {
            world,
            ticks,
            output,
            json,
        } => {
            let mut sim = load(&world)?;
            let events = sim.step_many(ticks);
            let destination = output.as_ref().unwrap_or(&world);
            save_atomic(destination, &sim)?;
            Ok((
                format!(
                    "Advanced {ticks} ticks to tick {}. {} events. Hash {}",
                    sim.tick(),
                    events.len(),
                    sim.state_hash()
                ),
                json!({"ok": true, "command": "step", "ticks_advanced": ticks, "tick": sim.tick(), "events": events,
                    "structures": sim.snapshot().structures, "materials": sim.material_totals(), "produced_parts": sim.produced_parts(),
                    "state_hash": sim.state_hash(), "output": destination}),
                json,
            ))
        }
        CtlCommand::Summary { world, json } => {
            let sim = load(&world)?;
            let state = sim.snapshot();
            Ok((
                format!(
                    "Seed {} | tick {} | player ({}, {}) | {} structures | parts {}/10 | hash {}",
                    state.seed,
                    state.tick,
                    state.player.position.x,
                    state.player.position.y,
                    state.structures.len(),
                    state.mechanical_parts_produced,
                    state.state_hash
                ),
                json!({"ok": true, "command": "summary", "seed": state.seed, "tick": state.tick,
                    "player": state.player, "resources": resource_counts(&state), "structures": state.structures,
                    "materials": sim.material_totals(), "produced_parts": state.mechanical_parts_produced,
                    "objective_complete": state.objective_complete, "state_hash": state.state_hash}),
                json,
            ))
        }
        CtlCommand::Inspect { world, x, y, json } => {
            let sim = load(&world)?;
            let state = sim.snapshot();
            let tile = state
                .tiles
                .iter()
                .find(|tile| tile.position == Pos::new(x, y))
                .ok_or(GameError::OutOfBounds)?;
            let resource = tile
                .resource_id
                .and_then(|id| state.resources.iter().find(|node| node.id == id));
            let structure = tile
                .structure_id
                .and_then(|id| state.structures.iter().find(|machine| machine.id == id));
            Ok((
                format!(
                    "Tile ({x}, {y}): {:?}; resource {:?}; structure {:?}",
                    tile.terrain, resource, structure
                ),
                json!({"ok": true, "command": "inspect", "tick": state.tick, "tile": tile,
                    "resource": resource, "structure": structure, "state_hash": state.state_hash}),
                json,
            ))
        }
        CtlCommand::Place {
            kind,
            world,
            x,
            y,
            direction,
            json,
        } => mutate_command(
            "place",
            world,
            json,
            GameCommand::Build {
                kind: parse(&kind)?,
                position: Pos::new(x, y),
                direction: parse(&direction)?,
            },
        ),
        CtlCommand::Harvest { world, x, y, json } => mutate_command(
            "harvest",
            world,
            json,
            GameCommand::Harvest {
                position: Pos::new(x, y),
            },
        ),
        CtlCommand::Move {
            world,
            direction,
            json,
        } => mutate_command(
            "move",
            world,
            json,
            GameCommand::Move {
                direction: parse(&direction)?,
            },
        ),
        CtlCommand::Rotate {
            world,
            id,
            direction,
            json,
        } => mutate_command(
            "rotate",
            world,
            json,
            GameCommand::Rotate {
                structure_id: id,
                direction: parse(&direction)?,
            },
        ),
        CtlCommand::Configure {
            world,
            id,
            recipe,
            json,
        } => mutate_command(
            "configure",
            world,
            json,
            GameCommand::Configure {
                structure_id: id,
                recipe: parse(&recipe)?,
            },
        ),
        CtlCommand::Transfer {
            world,
            id,
            route,
            item,
            count,
            json,
        } => mutate_command(
            "transfer",
            world,
            json,
            GameCommand::Transfer {
                structure_id: id,
                route: parse(&route)?,
                item: parse(&item)?,
                count,
            },
        ),
        CtlCommand::Remove { world, id, json } => mutate_command(
            "remove",
            world,
            json,
            GameCommand::Remove { structure_id: id },
        ),
        CtlCommand::Validate { world, json } => {
            let sim = load(&world)?;
            sim.validate()?;
            Ok((
                format!(
                    "Valid format v1 world at tick {} (hash {})",
                    sim.tick(),
                    sim.state_hash()
                ),
                json!({"ok": true, "command": "validate", "tick": sim.tick(), "state_hash": sim.state_hash(), "materials": sim.material_totals()}),
                json,
            ))
        }
        CtlCommand::Scenario { file, json } => {
            let report = scenario::run_file(&file)?;
            let human = format!(
                "Scenario {}: {} ({} assertions, tick {}, hash {})",
                report.name,
                if report.passed { "PASS" } else { "FAIL" },
                report.assertions.len(),
                report.tick,
                report.state_hash
            );
            let mut value = serde_json::to_value(report)?;
            let passed = value["passed"].clone();
            value
                .as_object_mut()
                .expect("report is an object")
                .insert("ok".into(), passed);
            Ok((human, value, json))
        }
        CtlCommand::Bench {
            world,
            build_label,
            ticks,
            samples,
            warmup,
            json,
        } => {
            let worlds = world
                .into_iter()
                .map(|path| Ok((path.clone(), load(&path)?)))
                .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
            let report = benchmark::run(worlds, build_label, ticks, samples, warmup)?;
            let human = report.human_summary();
            Ok((human, serde_json::to_value(report)?, json))
        }
        CtlCommand::Debug { action } => run_debug(action),
    }
}

fn run_debug(action: DebugAction) -> Response {
    type Operation = Box<dyn FnOnce(&mut FactorySimulation) -> Result<(), GameError>>;
    let (world, json, description, operation): (_, _, _, Operation) = match action {
        DebugAction::Grant {
            world,
            item,
            count,
            json,
        } => {
            let item: Item = parse(&item)?;
            (
                world,
                json,
                format!("granted {count} {item:?}"),
                Box::new(move |sim| {
                    sim.debug().grant(item, count);
                    Ok(())
                }),
            )
        }
        DebugAction::Resource {
            world,
            x,
            y,
            kind,
            quantity,
            json,
        } => {
            let kind: ResourceKind = parse(&kind)?;
            (
                world,
                json,
                format!("set resource at ({x}, {y})"),
                Box::new(move |sim| {
                    sim.debug().set_resource(Pos::new(x, y), kind, quantity)?;
                    Ok(())
                }),
            )
        }
        DebugAction::Player { world, x, y, json } => (
            world,
            json,
            format!("moved player to ({x}, {y})"),
            Box::new(move |sim| sim.debug().set_player_position(Pos::new(x, y))),
        ),
        DebugAction::Terrain {
            world,
            x,
            y,
            terrain,
            json,
        } => {
            let terrain: Terrain = parse(&terrain)?;
            (
                world,
                json,
                format!("set terrain at ({x}, {y})"),
                Box::new(move |sim| sim.debug().set_terrain(Pos::new(x, y), terrain)),
            )
        }
        DebugAction::Belt {
            world,
            id,
            item,
            json,
        } => {
            let item: Option<Item> = item.map(|name| parse(&name)).transpose()?;
            (
                world,
                json,
                format!("set belt {id}"),
                Box::new(move |sim| sim.debug().set_belt_item(id, item)),
            )
        }
    };
    let mut sim = load(&world)?;
    operation(&mut sim)?;
    save_atomic(&world, &sim)?;
    Ok((
        format!("Debug: {description}. Hash {}", sim.state_hash()),
        json!({"ok": true, "command": "debug", "description": description, "tick": sim.tick(), "state_hash": sim.state_hash()}),
        json,
    ))
}

fn mutate_command(name: &str, world: PathBuf, as_json: bool, command: GameCommand) -> Response {
    let mut sim = load(&world)?;
    let events = sim.execute(command)?;
    save_atomic(&world, &sim)?;
    let state = sim.snapshot();
    Ok((
        format!(
            "{name}: {:?}. Tick {}. Hash {}",
            events,
            sim.tick(),
            sim.state_hash()
        ),
        json!({"ok": true, "command": name, "events": events, "tick": sim.tick(), "state_hash": sim.state_hash(),
            "player": state.player, "structures": state.structures}),
        as_json,
    ))
}

fn resource_counts(state: &factory_protocol::GameSnapshot) -> Value {
    let mut counts = std::collections::BTreeMap::<String, u64>::new();
    for resource in &state.resources {
        *counts
            .entry(format!("{:?}", resource.kind).to_lowercase())
            .or_default() += resource.quantity as u64;
    }
    json!(counts)
}

fn parse<T: DeserializeOwned>(value: &str) -> Result<T, Box<dyn std::error::Error>> {
    Ok(serde_json::from_value(Value::String(
        value.replace('-', "_").to_lowercase(),
    ))?)
}

fn load(path: &Path) -> Result<FactorySimulation, Box<dyn std::error::Error>> {
    Ok(FactorySimulation::from_json(&fs::read_to_string(path)?)?)
}

fn save_atomic(path: &Path, sim: &FactorySimulation) -> Result<(), Box<dyn std::error::Error>> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let file_name = path
        .file_name()
        .ok_or("output path has no file name")?
        .to_string_lossy();
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let temporary = parent.join(format!(".{file_name}.{}.{}.tmp", std::process::id(), nonce));
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(sim.to_json()?.as_bytes())?;
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
