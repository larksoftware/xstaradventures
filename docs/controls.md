# Controls

This document lists the current bootstrap controls and what they affect.

## Player Controls

- `W` / `S`: apply forward/reverse thrust (Newtonian physics - ship maintains velocity).
- `A` / `D`: rotate ship left/right.
- `F`: interact (mine ore, refuel station, transfer ore, build outpost).
- `H`: toggle home beacon arrow (points to nearest revealed node).
- Left Mouse Button: fire weapon at pirates.
- `Tab`: cycle through nearby tactical targets (shows arrow when far, circle when near).
- `,` / `.`: decrease/increase scout fleet risk tolerance.

**Note**: Movement uses realistic space physics. Thrust accelerates your ship in the direction it's facing. Ship will continue moving at current velocity until you apply counter-thrust to slow down or change direction.

## Simulation Controls

- Space: toggle pause.
- `[` / `]`: decrease/increase simulation tick rate.
- `M`: toggle map view (macro map vs world view; ships/stations render in world view).

## Debug Window (F3 to toggle)

Press `F3` to open/close the debug window. All debug commands only work when the debug window is open.

### World Generation (debug window only)

- `-` / `=`: decrease/increase the world seed (regenerates nodes/routes).
- `V`: reveal adjacent nodes.
- `U`: reveal all nodes.
- `Z`: clear reveals.
- `B`: spawn FuelDepot.
- `J`: spawn Scout.

### Map & Debug Rendering (debug window only)

- `C`: cycle map zoom level (map view only).
- `N`: toggle node rendering.
- `R`: toggle route rendering.
- `F`: toggle fog/intel rings.
- `G`: toggle map grid.
- `P`: toggle world backdrop.
- `T`: toggle route labels (distance + risk).
- `Y`: toggle node labels.
- `I`: refresh intel confidence to full.
- `O`: advance intel layer by one step.
- `K`: randomize zone modifiers.

Note: Intel refresh has a short cooldown to prevent spamming.
Note: Grid and reveal keys only affect map view.

## Save/Load

- `F5`: serialize the sector to RON and log the payload size.
- `F9`: load sector data from `saves/sector.ron` if present, otherwise load the sample RON.
