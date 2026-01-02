# Controls

This document lists the current bootstrap controls and what they affect.

## Simulation Controls

- Space: toggle pause.
- `[` / `]`: decrease/increase simulation tick rate.

## World Generation

- `-` / `=`: decrease/increase the world seed (regenerates nodes/routes).
- `V`: reveal adjacent nodes (debug).
- `A`: reveal all nodes (debug).
- `Z`: clear reveals (debug).
- `B`: spawn FuelDepot (debug).
- `J`: spawn Scout (debug).
- `H`: center camera on revealed node (world view).

## Map & Debug Rendering

- `M`: toggle map view (macro map vs world view; ships/stations render in world view).
- `N`: toggle node rendering.
- `R`: toggle route rendering.
- `F`: toggle fog/intel rings.
- `G`: toggle map grid.
- `P`: toggle world backdrop.
- `T`: toggle route labels.
- `I`: refresh intel confidence to full (debug).
- `O`: advance intel layer by one step (debug).
- `K`: randomize zone modifiers (debug).

Note: intel refresh has a short cooldown to prevent spamming.
Note: grid and reveal keys only affect map view.

## Save/Load Stubs

- `S`: serialize the sector to RON and log the payload size.
- `L`: load sector data from `saves/sector.ron` if present, otherwise load the sample RON.
