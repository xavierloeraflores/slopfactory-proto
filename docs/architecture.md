# Architecture

The dependency direction is `factory-bevy` and `factory-cli` → `factory-core` → `factory-protocol`. The protocol has no engine dependency. The core owns the only authoritative game state. A command changes it; an ordered 20 Hz tick advances it; snapshots expose it. The Bevy adapter holds only rendering entities, input state, and timing accumulation.

Coordinates and quantities are integers. State collections use ordered `BTreeMap`s and row-major vectors for deterministic JSON serialization and SHA-256 state hashing. The tick pipeline resolves extraction, crafting, belt transfers, then machine exports. Belt transfer intents are evaluated against the pre-transfer occupancy state, so an item moves at most one belt per five-tick transfer phase. Structure IDs choose a stable winner for contention. Miners and crafters retain output when belts are blocked.

The world is stored in 16 chunks of 16×16 tiles. Resource and structure IDs are stable integers. Saves use format version 1 and contain no Bevy handles. `FactorySimulation::validate` checks chunk topology, bidirectional tile references, machine capacities, and monotonic ID allocation.

The Bevy frontend keeps a map from stable resource and structure IDs to render entities, and updates those entities from core snapshots. Terrain sprites are created once. Camera tracking, animation, preview tint, and UI use Bevy state but never alter canonical game rules. The frontend uses the same atomic JSON save format as the CLI. Asset lookup checks the working directory, the executable directory, and the development workspace for the original sprites.
