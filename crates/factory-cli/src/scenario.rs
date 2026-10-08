use std::{error::Error, fs, path::Path};

use factory_core::FactorySimulation;
use factory_protocol::{
    GameCommand, GameEvent, Item, MaterialTotals, Pos, ResourceKind, StructureKind,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Deserialize)]
struct ScenarioFile {
    name: String,
    seed: u64,
    steps: Vec<ScenarioStep>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ScenarioStep {
    Command {
        command: GameCommand,
    },
    Step {
        ticks: u64,
    },
    DebugGrant {
        item: Item,
        count: u32,
    },
    DebugResource {
        position: Pos,
        kind: ResourceKind,
        quantity: u32,
    },
    DebugPlayer {
        position: Pos,
    },
    CaptureMaterials,
    Assert {
        assertion: Assertion,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Assertion {
    Inventory {
        item: Item,
        equals: u32,
    },
    PlayerPosition {
        position: Pos,
    },
    Reachable {
        position: Pos,
    },
    Resource {
        position: Pos,
        kind: ResourceKind,
        quantity_at_least: Option<u32>,
        quantity_equals: Option<u32>,
    },
    Structure {
        position: Pos,
        kind: StructureKind,
        belt_item: Option<Item>,
        output_item: Option<Item>,
        output_at_least: Option<u32>,
        input_item: Option<Item>,
        input_at_least: Option<u32>,
        progress: Option<u32>,
    },
    Tick {
        equals: u64,
    },
    ProducedParts {
        at_least: u32,
    },
    ObjectiveComplete,
    MaterialsUnchanged,
}

#[derive(Debug, Serialize)]
pub struct AssertionResult {
    pub name: String,
    pub passed: bool,
    pub expected: Value,
    pub actual: Value,
}

#[derive(Debug, Serialize)]
pub struct ScenarioReport {
    pub name: String,
    pub passed: bool,
    pub tick: u64,
    pub state_hash: String,
    pub materials: MaterialTotals,
    pub assertions: Vec<AssertionResult>,
    pub events: Vec<GameEvent>,
}

pub fn run_file(path: &Path) -> Result<ScenarioReport, Box<dyn Error>> {
    let definition: ScenarioFile = serde_json::from_str(&fs::read_to_string(path)?)?;
    let mut sim = FactorySimulation::new(definition.seed);
    let mut assertions = Vec::new();
    let mut events = Vec::new();
    let mut captured_materials = None;
    for step in definition.steps {
        match step {
            ScenarioStep::Command { command } => events.extend(sim.execute(command)?),
            ScenarioStep::Step { ticks } => events.extend(sim.step_many(ticks)),
            ScenarioStep::DebugGrant { item, count } => sim.debug().grant(item, count),
            ScenarioStep::DebugResource {
                position,
                kind,
                quantity,
            } => {
                sim.debug().set_resource(position, kind, quantity)?;
            }
            ScenarioStep::DebugPlayer { position } => sim.debug().set_player_position(position)?,
            ScenarioStep::CaptureMaterials => captured_materials = Some(sim.material_totals()),
            ScenarioStep::Assert { assertion } => {
                assertions.push(evaluate(&sim, captured_materials, assertion))
            }
        }
    }
    sim.validate()?;
    Ok(ScenarioReport {
        name: definition.name,
        passed: assertions.iter().all(|result| result.passed) && !assertions.is_empty(),
        tick: sim.tick(),
        state_hash: sim.state_hash(),
        materials: sim.material_totals(),
        assertions,
        events,
    })
}

fn result(name: &str, expected: Value, actual: Value) -> AssertionResult {
    AssertionResult {
        name: name.into(),
        passed: expected == actual,
        expected,
        actual,
    }
}

fn evaluate(
    sim: &FactorySimulation,
    captured: Option<MaterialTotals>,
    assertion: Assertion,
) -> AssertionResult {
    match assertion {
        Assertion::Inventory { item, equals } => {
            result("inventory", json!(equals), json!(sim.inventory_count(item)))
        }
        Assertion::PlayerPosition { position } => result(
            "player_position",
            json!(position),
            json!(sim.player_position()),
        ),
        Assertion::Reachable { position } => result(
            "reachable",
            json!(true),
            json!(sim.reachable_tiles().contains(&position)),
        ),
        Assertion::Resource {
            position,
            kind,
            quantity_at_least,
            quantity_equals,
        } => {
            let node = sim.resource_at(position);
            let actual =
                json!({"kind": node.map(|n| n.kind), "quantity": node.map(|n| n.quantity)});
            let passed = node.is_some_and(|n| {
                n.kind == kind
                    && quantity_at_least.is_none_or(|min| n.quantity >= min)
                    && quantity_equals.is_none_or(|exact| n.quantity == exact)
            });
            AssertionResult {
                name: "resource".into(),
                passed,
                expected: json!({"kind": kind, "quantity_at_least": quantity_at_least, "quantity_equals": quantity_equals}),
                actual,
            }
        }
        Assertion::Structure {
            position,
            kind,
            belt_item,
            output_item,
            output_at_least,
            input_item,
            input_at_least,
            progress,
        } => {
            let state = sim.snapshot();
            let machine = state.structures.iter().find(|s| s.position == position);
            let actual_output = output_item
                .and_then(|item| {
                    machine.and_then(|m| {
                        m.output
                            .iter()
                            .find(|stack| stack.item == item)
                            .map(|stack| stack.count)
                    })
                })
                .unwrap_or(0);
            let actual_input = input_item
                .and_then(|item| {
                    machine.and_then(|m| {
                        m.input
                            .iter()
                            .find(|stack| stack.item == item)
                            .map(|stack| stack.count)
                    })
                })
                .unwrap_or(0);
            let passed = machine.is_some_and(|m| {
                m.kind == kind
                    && belt_item.is_none_or(|item| m.belt_item == Some(item))
                    && output_at_least.is_none_or(|min| actual_output >= min)
                    && input_at_least.is_none_or(|min| actual_input >= min)
                    && progress.is_none_or(|value| m.progress == value)
            });
            AssertionResult {
                name: "structure".into(),
                passed,
                expected: json!({"position": position, "kind": kind, "belt_item": belt_item,
                    "output_item": output_item, "output_at_least": output_at_least,
                    "input_item": input_item, "input_at_least": input_at_least, "progress": progress}),
                actual: json!({"structure": machine, "matched_output": actual_output, "matched_input": actual_input}),
            }
        }
        Assertion::Tick { equals } => result("tick", json!(equals), json!(sim.tick())),
        Assertion::ProducedParts { at_least } => AssertionResult {
            name: "produced_parts".into(),
            passed: sim.produced_parts() >= at_least,
            expected: json!({"at_least": at_least}),
            actual: json!(sim.produced_parts()),
        },
        Assertion::ObjectiveComplete => result(
            "objective_complete",
            json!(true),
            json!(sim.snapshot().objective_complete),
        ),
        Assertion::MaterialsUnchanged => AssertionResult {
            name: "materials_unchanged".into(),
            passed: captured == Some(sim.material_totals()),
            expected: json!(captured),
            actual: json!(sim.material_totals()),
        },
    }
}
