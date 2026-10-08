# Architecture

The dependency direction is `factory-bevy` and `factory-cli` → `factory-core` → `factory-protocol`. The protocol has no engine dependency. The core owns the only authoritative game state. A command changes it; an ordered 20 Hz tick advances it; snapshots expose it. The Bevy adapter holds only rendering entities, input state, and timing accumulation.

Coordinates and quantities are integers. State collections use stable ordering for deterministic serialization and hashing. The tick pipeline resolves extraction, crafting, and item transfers in a fixed order. Transfer intents are evaluated against a snapshot of occupied destinations so an item moves at most one belt per transfer tick.
