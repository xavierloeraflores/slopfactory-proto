# Headless CLI

`factoryctl` creates and manipulates JSON saves without Bevy. Each successful mutating command atomically replaces the save; a failed command leaves it intact. Add `--json` for structured output. Commands include `new`, `step`, `summary`, `inspect`, `place`, `harvest`, `scenario`, and `validate`. Development item injection is separate from gameplay commands.
