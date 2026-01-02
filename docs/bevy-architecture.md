# Bevy Architecture Plan — XStar Adventures (One Page)

## 1) Goals

* **Ship MVP fast**: 2D top-down sandbox with stations, fleets, pirates, crises, delegation UI.
* **System-first**: deterministic, testable simulation loop (ECS-friendly).
* **Cross-platform**: macOS/Windows/Linux.

---

## 2) High-Level Architecture

### Two Worlds (Conceptually)

* **Simulation World (truth)**: data + rules (stations/fleets/pirates/crises).
* **Presentation World (view)**: rendering + UI + input.

In practice, both live in Bevy ECS, but we keep boundaries:

* Simulation components are **pure state**.
* View components are **derived/visual**.

---

## 3) App Structure (Plugins)

### Core Plugins (MVP)

1. **CorePlugin**

   * time config, fixed update rate, game states
2. **WorldGenPlugin**

   * seeded sector generation (systems, routes, nodes)
3. **SimPlugin**

   * fixed-timestep simulation: stations, fleets, pirates, crises
4. **OrdersPlugin**

   * player intents/commands → validated orders → sim inputs
5. **UIPlugin** (egui recommended)

   * delegation panels, problems feed
6. **Render2DPlugin**

   * camera, sprites, overlays (pressure zones, status rings)
7. **SaveLoadPlugin** (optional for slice)

   * serialize sim state (RON/JSON), deterministic seeds

---

## 4) Scheduling Model

### Use **FixedUpdate** for simulation

* Deterministic tick (e.g., 10–20 Hz)
* All economy/AI/crisis progression happens here

### Use **Update** for presentation

* Input collection
* UI rendering
* Camera movement
* Interpolation (optional)

**Rule:** UI can request actions anytime, but actions are applied only at tick boundaries.

---

## 5) ECS Data Model (Core Components)

### Sector & Navigation

* `SystemNode { id, pos, tags }`
* `RouteEdge { a, b, distance, risk }`
* `ResourceNode { ore_kind, richness, depletion }`
* `PressureField { pirate_pressure, faction_pressure }`

### Stations

* `Station { kind }`
* `StationState { Deploying|Operational|Strained|Failed }`
* `Fuel { current, max }`
* `Output { rate, buffer }`
* `Vulnerability { value }`

### Fleets

* `Fleet { role }`
* `FleetIntent { Scout(..)|Mine(..)|Defend(..)|Patrol(..) }`
* `RiskTolerance { Cautious|Balanced|Aggressive|Desperate }`
* `Priorities { yield_w, safety_w, cost_w, speed_w }`
* `Awareness { last_scout_time, threat_estimate, uncertainty }`
* `Fuel { current, max }`

### Pirates

* `PirateGroup { doctrine, epoch }`
* `PirateBase { tier, boss_alive }`
* `RaidPlan { target, eta, strength }`

### Crises

* `CrisisStage { Stable|Strained|Failing|Resolved }`
* `CrisisType { FuelShortage|PirateHarassment|... }`
* `Crisis { crisis_type, stage, timer, affected_entities }`

---

## 6) Key Systems (MVP)

### Simulation Systems (FixedUpdate)

* `station_lifecycle_system`
* `fuel_consumption_system`
* `production_system` (ore output)
* `fleet_decision_system` (intent → decision)
* `fleet_execution_system` (move/act)
* `pirate_escalation_system` (epoch + pressure)
* `pirate_target_selection_system`
* `crisis_detection_system` (create/update crises)
* `crisis_escalation_system` (stage transitions)
* `consequence_system` (failure → world change)

### Presentation Systems (Update)

* `ui_delegation_panels_system`
* `ui_problems_feed_system`
* `map_overlay_render_system` (pressure zones/status)
* `selection_system` (click/hover)

---

## 7) Command/Order Flow (UI → Sim)

### Pattern

1. UI emits a **Command** event
2. Orders layer validates and converts to **Order**
3. Orders are queued and applied in next **FixedUpdate**

### Events/Resources

* `CommandEvent` (raw UI intent)
* `OrderQueue` (validated)
* `OrderAppliedEvent` (feedback)

This ensures determinism and debuggability.

---

## 8) UI Approach (Recommended)

### Use `bevy_egui`

* Fast iteration on panels
* Perfect for fleet/station dashboards
* Easy to create Problems Feed with 3-action suggestions

Keep UI data minimal:

* Read from sim components
* Write only commands (never mutate sim state directly)

---

## 9) Testing Strategy (Rust-Native)

* Put pure logic in `sim/logic` modules
* Unit test:

  * station state transitions
  * crisis escalation rules
  * fleet decision scoring
* Use deterministic seeds for reproducible sim runs

---

## 10) MVP Implementation Order

1. Bevy app + camera + basic map nodes
2. Sector graph + seeded worldgen
3. Stations: build, fuel, output, lifecycle states
4. Pirates: roaming patrols + pressure overlay
5. Fleets: Scout/Mining/Security (intent + risk)
6. Delegation UI (egui): fleet panel + problems feed
7. Crisis escalation (fuel shortage + harassment)
8. Pirate base + boss encounter (single mechanic)

---

## 11) Definition of Done (Vertical Slice)

* A full run (60–90 min) is playable
* Delegation is clear and fast
* Pirates escalate with time
* Crises cascade when ignored
* Player intervention can save/lose stations
* The world changes permanently (abandon/capture/ruin)

