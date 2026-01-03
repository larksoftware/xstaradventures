# Controls

This document lists the current bootstrap controls and what they affect.

## Player Controls

- `W` / `S`: apply forward/reverse thrust (Newtonian physics - ship maintains velocity).
- `A` / `D`: rotate ship left/right.
- `Space`: apply braking thrust (decelerates toward zero; disabled while W/S held).
- `N`: engage autopilot to selected target (press Tab first to select; movement keys disengage).
- `F`: interact (mine ore, refuel station, transfer ore, build outpost).
- `H`: toggle home beacon arrow (points to nearest revealed node).
- Left Mouse Button: fire weapon at pirates.
- `Tab`: cycle through nearby tactical targets (shows arrow when far, circle when near).
- `,` / `.`: decrease/increase scout fleet risk tolerance.

**Note**: Movement uses realistic space physics. Thrust accelerates your ship in the direction it's facing. Ship will continue moving at current velocity until you apply counter-thrust to slow down or change direction.

**Autopilot**: Press Tab to select a target, then N to engage autopilot. The ship will rotate toward the target, accelerate, then brake to stop within docking range. Autopilot disengages automatically on arrival, or if any movement key is pressed.

## Simulation Controls

- `Escape`: toggle pause.
- `[` / `]`: decrease/increase simulation tick rate.

## Map Controls

- `M`: toggle map view (macro map vs world view; ships/stations render in world view).

## Debug Window (F3 to toggle)

Press `F3` to open/close the debug window. All debug commands require **Shift** to be held and only work when the debug window is open.

### World Generation (debug window only)

- `Shift+-` / `Shift+=`: decrease/increase the world seed (regenerates nodes/routes).
- `Shift+V`: reveal adjacent nodes.
- `Shift+U`: reveal all nodes.
- `Shift+Z`: clear reveals.
- `Shift+B`: spawn FuelDepot.
- `Shift+S`: spawn Scout.
- `Shift+P`: spawn Pirate.

### Map & Debug Rendering (debug window only)

- `Shift+N`: toggle node rendering.
- `Shift+R`: toggle route rendering.
- `Shift+F`: toggle fog/intel rings.
- `Shift+G`: toggle map grid.
- `Shift+T`: toggle route labels (distance + risk).
- `Shift+Y`: toggle node labels.
- `Shift+I`: refresh intel confidence to full.
- `Shift+O`: advance intel layer by one step.
- `Shift+K`: randomize zone modifiers.
- `Shift+C`: cycle map zoom level.

Note: Intel refresh has a short cooldown to prevent spamming.
Note: Grid and reveal keys only affect map view.

## Save/Load

- `F5`: serialize the sector to RON and log the payload size.
- `F9`: load sector data from `saves/sector.ron` if present, otherwise load the sample RON.
