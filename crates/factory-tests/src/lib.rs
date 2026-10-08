//! Headless cross-crate integration scenarios.

#[cfg(test)]
mod scenarios {
    use factory_core::{FactorySimulation, GameError};
    use factory_protocol::{
        Direction, GameCommand, GameEvent, Item, Pos, Recipe, StructureId, StructureKind,
        TransferRoute,
    };

    fn build(
        sim: &mut FactorySimulation,
        kind: StructureKind,
        at: Pos,
        facing: Direction,
        player: Pos,
    ) -> StructureId {
        sim.debug().set_player_position(player).unwrap();
        let events = sim
            .execute(GameCommand::Build {
                kind,
                position: at,
                direction: facing,
            })
            .unwrap();
        let GameEvent::Built { structure_id, .. } = events[0] else {
            panic!("expected build event")
        };
        structure_id
    }

    #[test]
    fn construction_is_atomic_and_preserves_deposits() {
        let mut sim = FactorySimulation::new(42);
        let before = sim.state_hash();
        assert_eq!(
            sim.execute(GameCommand::Build {
                kind: StructureKind::Miner,
                position: Pos::new(33, 32),
                direction: Direction::East
            }),
            Err(GameError::NoResource)
        );
        assert_eq!(sim.state_hash(), before);
        let id = build(
            &mut sim,
            StructureKind::Miner,
            Pos::new(35, 31),
            Direction::North,
            Pos::new(34, 32),
        );
        assert_eq!(sim.inventory_count(Item::Stone), 16);
        assert_eq!(sim.inventory_count(Item::IronOre), 10);
        sim.execute(GameCommand::Remove { structure_id: id })
            .unwrap();
        assert!(sim.resource_at(Pos::new(35, 31)).is_some());
        assert_eq!(sim.inventory_count(Item::Stone), 16);
    }

    #[test]
    fn miner_and_belt_backpressure_preserve_every_ore() {
        let mut sim = FactorySimulation::new(42);
        let node = sim.resource_at(Pos::new(35, 31)).unwrap().id;
        let miner = build(
            &mut sim,
            StructureKind::Miner,
            Pos::new(35, 31),
            Direction::North,
            Pos::new(34, 32),
        );
        let near = build(
            &mut sim,
            StructureKind::Conveyor,
            Pos::new(35, 32),
            Direction::North,
            Pos::new(34, 32),
        );
        let far = build(
            &mut sim,
            StructureKind::Conveyor,
            Pos::new(35, 33),
            Direction::North,
            Pos::new(34, 32),
        );
        sim.step_many(59);
        assert_eq!(sim.resource_quantity(node), Some(120));
        sim.step();
        assert_eq!(sim.resource_quantity(node), Some(119));
        assert_eq!(
            sim.snapshot()
                .structures
                .iter()
                .find(|s| s.id == near)
                .unwrap()
                .belt_item,
            Some(Item::IronOre)
        );
        sim.step_many(5);
        assert_eq!(
            sim.snapshot()
                .structures
                .iter()
                .find(|s| s.id == far)
                .unwrap()
                .belt_item,
            Some(Item::IronOre)
        );
        sim.step_many(500);
        let state = sim.snapshot();
        assert_eq!(
            state
                .structures
                .iter()
                .find(|s| s.id == near)
                .unwrap()
                .belt_item,
            Some(Item::IronOre)
        );
        assert_eq!(
            state
                .structures
                .iter()
                .find(|s| s.id == far)
                .unwrap()
                .belt_item,
            Some(Item::IronOre)
        );
        assert_eq!(
            state
                .structures
                .iter()
                .find(|s| s.id == miner)
                .unwrap()
                .output[0]
                .count,
            4
        );
        let remaining = sim.resource_quantity(node);
        sim.step_many(120);
        assert_eq!(sim.resource_quantity(node), remaining);
        assert_eq!(120 - remaining.unwrap(), 6); // two belts plus four buffered
    }

    #[test]
    fn belt_item_moves_at_most_one_tile_per_transfer_tick() {
        let mut sim = FactorySimulation::new(42);
        let a = build(
            &mut sim,
            StructureKind::Conveyor,
            Pos::new(33, 32),
            Direction::East,
            Pos::new(32, 32),
        );
        let b = build(
            &mut sim,
            StructureKind::Conveyor,
            Pos::new(34, 32),
            Direction::East,
            Pos::new(32, 32),
        );
        let c = build(
            &mut sim,
            StructureKind::Conveyor,
            Pos::new(35, 32),
            Direction::East,
            Pos::new(34, 31),
        );
        sim.debug().set_belt_item(a, Some(Item::CopperOre)).unwrap();
        sim.step_many(5);
        assert_eq!(
            sim.snapshot()
                .structures
                .iter()
                .find(|s| s.id == b)
                .unwrap()
                .belt_item,
            Some(Item::CopperOre)
        );
        assert_eq!(
            sim.snapshot()
                .structures
                .iter()
                .find(|s| s.id == c)
                .unwrap()
                .belt_item,
            None
        );
        sim.step_many(5);
        assert_eq!(
            sim.snapshot()
                .structures
                .iter()
                .find(|s| s.id == c)
                .unwrap()
                .belt_item,
            Some(Item::CopperOre)
        );
    }

    #[test]
    fn crafting_buffering_and_objective_survive_replay() {
        let mut sim = FactorySimulation::new(42);
        let id = build(
            &mut sim,
            StructureKind::Crafter,
            Pos::new(33, 32),
            Direction::East,
            Pos::new(32, 32),
        );
        sim.execute(GameCommand::Configure {
            structure_id: id,
            recipe: Recipe::IronPlate,
        })
        .unwrap();
        sim.debug().grant(Item::IronOre, 12);
        sim.execute(GameCommand::Transfer {
            structure_id: id,
            route: TransferRoute::PlayerToInput,
            item: Item::IronOre,
            count: 16,
        })
        .unwrap();
        sim.step_many(39);
        let state = sim
            .snapshot()
            .structures
            .into_iter()
            .find(|s| s.id == id)
            .unwrap();
        assert_eq!(state.progress, 39);
        assert!(state.output.is_empty());
        assert_eq!(state.input[0].count, 14);
        sim.step();
        sim.step_many(160);
        let state = sim
            .snapshot()
            .structures
            .into_iter()
            .find(|s| s.id == id)
            .unwrap();
        assert_eq!(state.output[0].count, 4);
        assert_eq!(state.input[0].count, 8);
        assert_eq!(state.progress, 0);
        let mut replay = FactorySimulation::from_json(&sim.to_json().unwrap()).unwrap();
        sim.execute(GameCommand::Transfer {
            structure_id: id,
            route: TransferRoute::OutputToPlayer,
            item: Item::IronPlate,
            count: 1,
        })
        .unwrap();
        replay
            .execute(GameCommand::Transfer {
                structure_id: id,
                route: TransferRoute::OutputToPlayer,
                item: Item::IronPlate,
                count: 1,
            })
            .unwrap();
        sim.step_many(40);
        replay.step_many(40);
        assert_eq!(sim.state_hash(), replay.state_hash());
        assert_eq!(
            sim.snapshot()
                .structures
                .iter()
                .find(|s| s.id == id)
                .unwrap()
                .output[0]
                .count,
            4
        );
    }

    #[test]
    fn ten_mechanical_parts_complete_objective_once() {
        let mut sim = FactorySimulation::new(42);
        let id = build(
            &mut sim,
            StructureKind::Crafter,
            Pos::new(33, 32),
            Direction::East,
            Pos::new(32, 32),
        );
        sim.execute(GameCommand::Configure {
            structure_id: id,
            recipe: Recipe::MechanicalPart,
        })
        .unwrap();
        sim.debug().grant(Item::IronPlate, 20);
        sim.debug().grant(Item::Wood, 5);
        let materials_before = sim.material_totals();
        let mut completions = 0;
        for batch in [4, 4, 2] {
            for (item, count) in [(Item::IronPlate, 2 * batch), (Item::Wood, batch)] {
                sim.execute(GameCommand::Transfer {
                    structure_id: id,
                    route: TransferRoute::PlayerToInput,
                    item,
                    count,
                })
                .unwrap();
            }
            completions += sim
                .step_many(batch as u64 * 60)
                .iter()
                .filter(|e| **e == GameEvent::ObjectiveCompleted)
                .count();
            sim.execute(GameCommand::Transfer {
                structure_id: id,
                route: TransferRoute::OutputToPlayer,
                item: Item::MechanicalPart,
                count: batch,
            })
            .unwrap();
        }
        assert_eq!(sim.produced_parts(), 10);
        assert_eq!(sim.material_totals(), materials_before);
        assert_eq!(completions, 1);
        assert!(sim.snapshot().objective_complete);
        sim.step();
        assert_eq!(sim.tick(), 601);
    }

    #[test]
    fn mined_ore_reaches_crafter_and_finished_plates_export_without_loss() {
        let mut sim = FactorySimulation::new(42);
        let node = sim.resource_at(Pos::new(35, 31)).unwrap().id;
        let miner = build(
            &mut sim,
            StructureKind::Miner,
            Pos::new(35, 31),
            Direction::North,
            Pos::new(34, 32),
        );
        let a = build(
            &mut sim,
            StructureKind::Conveyor,
            Pos::new(35, 32),
            Direction::North,
            Pos::new(34, 32),
        );
        let b = build(
            &mut sim,
            StructureKind::Conveyor,
            Pos::new(35, 33),
            Direction::North,
            Pos::new(34, 32),
        );
        let crafter = build(
            &mut sim,
            StructureKind::Crafter,
            Pos::new(35, 34),
            Direction::East,
            Pos::new(34, 33),
        );
        let outlet = build(
            &mut sim,
            StructureKind::Conveyor,
            Pos::new(36, 34),
            Direction::East,
            Pos::new(36, 33),
        );
        sim.debug().set_player_position(Pos::new(34, 33)).unwrap();
        sim.execute(GameCommand::Configure {
            structure_id: crafter,
            recipe: Recipe::IronPlate,
        })
        .unwrap();
        let materials_before = sim.material_totals();
        sim.step_many(1000);
        assert_eq!(sim.material_totals(), materials_before);
        let state = sim.snapshot();
        let get = |id| state.structures.iter().find(|s| s.id == id).unwrap();
        let ore_in_miner = get(miner)
            .output
            .iter()
            .filter(|s| s.item == Item::IronOre)
            .map(|s| s.count)
            .sum::<u32>();
        let ore_on_belts = [a, b]
            .into_iter()
            .filter(|id| get(*id).belt_item == Some(Item::IronOre))
            .count() as u32;
        let ore_in_crafter = get(crafter)
            .input
            .iter()
            .find(|s| s.item == Item::IronOre)
            .map_or(0, |s| s.count);
        let plates_in_crafter = get(crafter)
            .output
            .iter()
            .find(|s| s.item == Item::IronPlate)
            .map_or(0, |s| s.count);
        let plates_on_belt = u32::from(get(outlet).belt_item == Some(Item::IronPlate));
        let reserved_ore = u32::from(get(crafter).progress > 0) * 2;
        let extracted = 120 - sim.resource_quantity(node).unwrap();
        assert!(plates_in_crafter + plates_on_belt > 0);
        assert_eq!(
            extracted,
            ore_in_miner
                + ore_on_belts
                + ore_in_crafter
                + 2 * (plates_in_crafter + plates_on_belt)
                + reserved_ore
        );
    }

    #[test]
    fn contested_exports_use_stable_id_order() {
        use factory_protocol::ResourceKind;
        let mut sim = FactorySimulation::new(42);
        sim.debug()
            .set_resource(Pos::new(34, 32), ResourceKind::Iron, 10)
            .unwrap();
        sim.debug()
            .set_resource(Pos::new(36, 32), ResourceKind::Copper, 10)
            .unwrap();
        sim.debug().grant(Item::Stone, 10);
        let left = build(
            &mut sim,
            StructureKind::Miner,
            Pos::new(34, 32),
            Direction::East,
            Pos::new(33, 32),
        );
        let right = build(
            &mut sim,
            StructureKind::Miner,
            Pos::new(36, 32),
            Direction::West,
            Pos::new(37, 32),
        );
        let belt = build(
            &mut sim,
            StructureKind::Conveyor,
            Pos::new(35, 32),
            Direction::North,
            Pos::new(35, 31),
        );
        sim.step_many(60);
        let state = sim.snapshot();
        assert!(left < right);
        assert_eq!(
            state
                .structures
                .iter()
                .find(|s| s.id == belt)
                .unwrap()
                .belt_item,
            Some(Item::IronOre)
        );
        assert_eq!(
            state
                .structures
                .iter()
                .find(|s| s.id == right)
                .unwrap()
                .output[0]
                .count,
            1
        );
    }

    #[test]
    fn ordinary_play_can_build_a_line_and_produce_ten_parts() {
        let mut sim = FactorySimulation::new(42);
        let walk = |world: &mut FactorySimulation, direction| {
            world.execute(GameCommand::Move { direction }).unwrap();
        };
        walk(&mut sim, Direction::East);
        sim.execute(GameCommand::Build {
            kind: StructureKind::Miner,
            position: Pos::new(35, 31),
            direction: Direction::North,
        })
        .unwrap();
        for position in [Pos::new(35, 32), Pos::new(35, 33)] {
            sim.execute(GameCommand::Build {
                kind: StructureKind::Conveyor,
                position,
                direction: Direction::North,
            })
            .unwrap();
        }
        walk(&mut sim, Direction::North);
        let plates = match sim
            .execute(GameCommand::Build {
                kind: StructureKind::Crafter,
                position: Pos::new(35, 34),
                direction: Direction::East,
            })
            .unwrap()[0]
        {
            GameEvent::Built { structure_id, .. } => structure_id,
            _ => unreachable!(),
        };
        walk(&mut sim, Direction::East);
        sim.execute(GameCommand::Build {
            kind: StructureKind::Conveyor,
            position: Pos::new(36, 34),
            direction: Direction::East,
        })
        .unwrap();
        walk(&mut sim, Direction::North);
        let parts = match sim
            .execute(GameCommand::Build {
                kind: StructureKind::Crafter,
                position: Pos::new(37, 34),
                direction: Direction::East,
            })
            .unwrap()[0]
        {
            GameEvent::Built { structure_id, .. } => structure_id,
            _ => unreachable!(),
        };
        for (structure_id, recipe) in [(plates, Recipe::IronPlate), (parts, Recipe::MechanicalPart)]
        {
            sim.execute(GameCommand::Configure {
                structure_id,
                recipe,
            })
            .unwrap();
        }
        assert_eq!(sim.inventory_count(Item::Stone), 1);
        assert_eq!(sim.inventory_count(Item::IronOre), 2);
        let materials_after_build = sim.material_totals();

        for _ in 0..4 {
            walk(&mut sim, Direction::West);
        }
        walk(&mut sim, Direction::South);
        for _ in 0..5 {
            sim.execute(GameCommand::Harvest {
                position: Pos::new(30, 30),
            })
            .unwrap();
        }
        assert_eq!(sim.inventory_count(Item::Wood), 10);
        for _ in 0..4 {
            walk(&mut sim, Direction::East);
        }
        walk(&mut sim, Direction::North);
        sim.execute(GameCommand::Transfer {
            structure_id: parts,
            route: TransferRoute::PlayerToInput,
            item: Item::Wood,
            count: 10,
        })
        .unwrap();
        assert_eq!(sim.material_totals(), materials_after_build);

        for _ in 0..100 {
            if sim.produced_parts() >= 10 {
                break;
            }
            sim.step_many(100);
            let output = sim
                .snapshot()
                .structures
                .iter()
                .find(|machine| machine.id == parts)
                .and_then(|machine| {
                    machine
                        .output
                        .iter()
                        .find(|stack| stack.item == Item::MechanicalPart)
                })
                .map_or(0, |stack| stack.count);
            if output > 0 {
                sim.execute(GameCommand::Transfer {
                    structure_id: parts,
                    route: TransferRoute::OutputToPlayer,
                    item: Item::MechanicalPart,
                    count: output,
                })
                .unwrap();
            }
        }
        assert_eq!(sim.produced_parts(), 10);
        assert!(sim.snapshot().objective_complete);
        assert_eq!(sim.material_totals(), materials_after_build);
        sim.validate().unwrap();
    }

    #[test]
    fn stone_mines_automatically_and_copper_plates_use_their_recipe() {
        let mut stone = FactorySimulation::new(42);
        stone
            .execute(GameCommand::Move {
                direction: Direction::North,
            })
            .unwrap();
        let node = stone.resource_at(Pos::new(32, 36)).unwrap().id;
        let miner = stone
            .execute(GameCommand::Build {
                kind: StructureKind::Miner,
                position: Pos::new(32, 36),
                direction: Direction::East,
            })
            .unwrap();
        let GameEvent::Built { structure_id, .. } = miner[0] else {
            unreachable!()
        };
        stone.step_many(60);
        assert_eq!(stone.resource_quantity(node), Some(149));
        assert_eq!(
            stone
                .snapshot()
                .structures
                .iter()
                .find(|s| s.id == structure_id)
                .unwrap()
                .output[0]
                .item,
            Item::Stone
        );

        let mut copper = FactorySimulation::new(42);
        copper
            .execute(GameCommand::Move {
                direction: Direction::West,
            })
            .unwrap();
        copper
            .execute(GameCommand::Move {
                direction: Direction::North,
            })
            .unwrap();
        for _ in 0..2 {
            copper
                .execute(GameCommand::Harvest {
                    position: Pos::new(29, 34),
                })
                .unwrap();
        }
        let crafter = copper
            .execute(GameCommand::Build {
                kind: StructureKind::Crafter,
                position: Pos::new(32, 33),
                direction: Direction::East,
            })
            .unwrap();
        let GameEvent::Built { structure_id, .. } = crafter[0] else {
            unreachable!()
        };
        copper
            .execute(GameCommand::Configure {
                structure_id,
                recipe: Recipe::CopperPlate,
            })
            .unwrap();
        copper
            .execute(GameCommand::Transfer {
                structure_id,
                route: TransferRoute::PlayerToInput,
                item: Item::CopperOre,
                count: 2,
            })
            .unwrap();
        copper.step_many(40);
        assert_eq!(
            copper
                .snapshot()
                .structures
                .iter()
                .find(|s| s.id == structure_id)
                .unwrap()
                .output[0]
                .item,
            Item::CopperPlate
        );
    }
}
