# Controls

This document lists the current bootstrap controls and what they affect.

## Player Controls

- `W` / `S`: apply forward/reverse thrust (Newtonian physics - ship maintains velocity).
- `A` / `D`: rotate ship left/right.
- `Space`: apply braking thrust (decelerates toward zero; disabled while W/S held).
- `N`: engage autopilot to selected target (press Tab first to select; movement keys disengage).
- `J`: interact (mine ore, refuel station, transfer ore, build outpost, activate jump gate).
- `H`: center camera on player ship.
- Left Mouse Button: fire weapon at pirates.
- `Tab`: cycle through nearby tactical targets (shows arrow when far, circle when near).
- `,` / `.`: decrease/increase scout fleet risk tolerance.

**Note**: Movement uses realistic space physics. Thrust accelerates your ship in the direction it's facing. Ship will continue moving at current velocity until you apply counter-thrust to slow down or change direction.

**Autopilot**: Press Tab to select a target, then N to engage autopilot. The ship will rotate toward the target, accelerate, then brake to stop within docking range. Autopilot disengages automatically on arrival, or if any movement key is pressed.

**Jump Gates**: Gates connect zones and appear along routes between nodes. To jump:
1. Fly near a jump gate (within 25 units).
2. Press J to activate. Costs 5 fuel.
3. After a brief transition, you arrive in the destination zone.

## Docking & Interactions

All interactions use proximity - fly close to something and press `J` to interact. The action depends on what's nearby:

| Target | Range | Action |
|--------|-------|--------|
| Asteroid (Ore Node) | 24 | **Hold J** to mine. CommonOre fills cargo; FuelOre refuels ship directly. |
| Shipyard / Refinery | 22 | **Press J** to dock and open the station menu. |
| Other Station | 22 | **Press J** to transfer: fuel from ship → station, ore from cargo → station storage. |
| System Node | 26 | **Press J** to build Mining Outpost (costs 18 ore). Must have no station nearby. |
| Jump Gate | 25 | **Press J** to jump to destination zone (costs 5 fuel). |

### Station Menu (Shipyard / Refinery)

When docked at a Shipyard or Refinery, a menu appears with available actions:

**Shipyard Menu**:
- Build Scout (15 ore, 120s) - Spawns a scout ship when complete
- Cancel job (50% ore refund)
- Undock

**Refinery Menu**:
- Convert 5 ore → 10 fuel (60s)
- Convert 10 ore → 20 fuel (90s)
- Collect converted fuel (transfers to your cargo)
- Cancel job (50% ore refund)
- Undock

**Job Rules**:
- Jobs pause when station is Strained or Failing (low fuel)
- Jobs are lost (no refund) if station becomes Failed
- Press Escape or click Undock to leave the station menu

**Tips**:
- Use autopilot (Tab to select, N to engage) to fly within docking range automatically.
- The target reticle changes from an arrow (far) to a circle (near/docked) when in range.
- Mining is continuous while holding J; other interactions trigger once per press.
- Station resupply transfers 10 fuel and 8 ore per interaction.

## Simulation Controls

- `Shift+P`: toggle pause.
- `[` / `]`: decrease/increase simulation tick rate.

## Map Controls

- `M`: toggle map view (macro map vs world view; ships/stations render in world view).
- Mouse Wheel: zoom in/out (smooth, works at all times in map view).
- Right-click + Drag: pan the map view.

## Debug Window (F3 to toggle)

Press `F3` to open/close the debug window. All debug commands require **Shift** to be held and only work when the debug window is open.

### World Generation (debug window only)

- `Shift+-` / `Shift+=`: decrease/increase the world seed (regenerates nodes/routes).
- `Shift+V`: reveal adjacent nodes.
- `Shift+U`: reveal all nodes.
- `Shift+Z`: clear reveals.
- `Shift+B`: spawn FuelDepot.
- `Shift+1`: spawn Refinery.
- `Shift+2`: spawn Shipyard.
- `Shift+3`: spawn Outpost (NPC trader station).
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

Note: Intel refresh has a short cooldown to prevent spamming.
Note: Grid and reveal keys only affect map view.

## Save/Load

- `F5`: serialize the sector to RON and log the payload size.
- `F9`: load sector data from `saves/sector.ron` if present, otherwise load the sample RON.
