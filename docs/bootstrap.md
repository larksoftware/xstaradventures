# Bootstrap Notes

This repository now contains a minimal Bevy application to enable iteration on
the architecture defined in `docs/bevy-architecture.md`.

## Entry Point

- `src/main.rs` wires the Bevy plugins and window configuration.

## Plugin Layout

- `src/plugins/core.rs` for app state and fixed timestep config.
- `src/plugins/worldgen.rs` for deterministic world seeding.
- `src/plugins/sim.rs` for fixed-timestep simulation systems.
- `src/plugins/orders.rs` for command/order flow.
- `src/plugins/ui.rs` for HUD and UI scaffolding.
- `src/plugins/render2d.rs` for camera and debug visuals.
- `src/plugins/saveload.rs` for future serialization.
- `src/stations.rs` for station enums and config helpers.
- `src/ships.rs` for ship/fleet enums and config helpers.

## Shared World Data

- `src/world.rs` defines `Sector`, `SystemNode`, and `RouteEdge` shared by
  worldgen and simulation.

## Zone Modifiers (MVP)

Each node may have 0–1 modifier:

- `HighRadiation`
- `NebulaInterference`
- `RichOreVeins`
- `ClearSignals`

## Adding New Systems

- Prefer adding systems inside the plugin that owns the feature.
- Keep simulation systems on `FixedUpdate`.
- Keep presentation and input systems on `Update`.

## State Flow

- `Boot` initializes resources then transitions to `Loading`.
- `Loading` shows a short overlay, then moves to `InGame`.
- `InGame` runs simulation, UI, and rendering systems.

## Controls

- Space: toggle pause.
- `[` / `]`: decrease/increase tick rate.
- `-` / `=`: decrease/increase world seed.
- `N`: toggle node rendering.
- `R`: toggle route rendering.
- `S`: save stub (RON serialization log).
- `L`: load stub (apply `saves/sector.ron` if present, else sample RON).

See `docs/controls.md` for the full list.

## Debug Visuals

- World view: ships/stations render as green/gold squares with labels.
- Map view: nodes and routes render with labels (`L# %` and modifier icon).
- Map view includes a compass (N/E/S/W) and map-only panels.
- World view shows ship/station panels only.
- Map grid can be toggled; reveal/debug keys expand visible nodes.
- Map zoom override cycles with `C` (returns to auto-fit after presets).
