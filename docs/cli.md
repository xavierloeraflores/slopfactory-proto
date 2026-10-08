# Headless CLI

`factoryctl` creates and manipulates JSON saves without Bevy. Each successful mutating command writes a temporary file in the destination directory, syncs it, and atomically renames it. A failed action leaves the original save intact. `step` accepts `--output` to write to another file; otherwise it replaces `--world`.

Add `--json` to any command for structured output. Failures use a nonzero exit code. Summaries and step output include ticks, state hash, machine inventories, belt contents, recipe progress, production count, and raw-material-equivalent totals.

```sh
factoryctl new --seed 42 --output world.json
factoryctl summary --world world.json --json
factoryctl inspect --world world.json --x 35 --y 31 --json
factoryctl move --world world.json --direction east
factoryctl harvest --world world.json --x 35 --y 31
factoryctl place miner --world world.json --x 35 --y 31 --direction north
factoryctl place conveyor --world world.json --x 35 --y 32 --direction north
factoryctl place crafter --world world.json --x 35 --y 33 --direction east
factoryctl configure --world world.json --id 3 --recipe iron_plate
factoryctl transfer --world world.json --id 3 --route player_to_input --item iron_ore --count 2
factoryctl rotate --world world.json --id 3 --direction east
factoryctl remove --world world.json --id 3
factoryctl step --world world.json --ticks 100
factoryctl validate --world world.json
factoryctl scenario --file scenarios/starting-world.json --json
factoryctl scenario --file scenarios/mining-belts.json --json
factoryctl scenario --file scenarios/crafting-test.json --json
```

`configure` and `transfer` require a crafter ID. IDs are assigned in build order; read them from `place --json`, `inspect`, or `summary`. Transfers support `player_to_input`, `input_to_player`, and `output_to_player`.

Controlled scenario setup uses `factoryctl debug grant`, `debug resource`, `debug player`, `debug terrain`, and `debug belt`. These are separate from the portable gameplay command enum. For example:

```sh
factoryctl debug grant --world world.json --item wood --count 5
factoryctl debug resource --world world.json --x 34 --y 32 --kind iron --quantity 20
factoryctl debug player --world world.json --x 34 --y 31
```

Scenario files contain a seed and ordered steps. Steps may execute a portable command, advance ticks, set up controlled test state, capture material totals, or assert state. The assertion report includes each expected and actual value. Failed assertions exit nonzero while preserving the full JSON report. `materials_unchanged` checks all deposits and item locations, converting plates and mechanical parts back into their ore and wood equivalents and counting ingredients reserved by in-progress recipes. Capture a baseline after construction, since building deliberately consumes materials.
