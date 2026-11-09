# MarkovJunior in Rust

This is an in-progress Rust port of the MarkovJunior interpreter.

The current engine supports two-dimensional grids, literal and wildcard rules,
the default square symmetries, and `one`, `all`, `prl`, `sequence`, `markov`,
and `path` nodes. It loads one model XML file, runs a seeded simulation, and
prints the final grid as text.

The `symmetry` attribute accepts `()`, `(x)`, `(y)`, `(x)(y)`, `(xy+)`, and
`(xy)`. Child rules can override their parent node's symmetry group.

It does not yet support 3D grids, inference, fields, or map nodes,
convolution, convchain, WFC, image or voxel rule inputs, graphical output, or
the upstream multi-model runner.

## Run

```sh
cargo run -- MarkovJunior/models/Growth.xml --width 32 --height 32 --steps 1000 --seed 7
```

Arguments after the model path are optional. Defaults are width `64`, height
`64`, steps `10000`, and seed `0`.

## Code layout

- `grid.rs` owns grid state and symbols.
- `rule.rs` parses patterns, expands symmetries, finds matches, and applies rules.
- `node.rs` runs rule nodes and branch nodes.
- `path.rs` finds and paints grid paths.
- `model.rs` loads XML and exposes the simulation API.
- `main.rs` provides the text-output CLI.