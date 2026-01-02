# Stations — Types, Lifecycle, Crises, and Build/Fuel Math (MVP)

This document formalizes station types, defines a **Station enum + lifecycle state machine**, maps stations into **crisis generation**, and proposes **exact build-time and fuel math** suitable for the MVP.

Stations are the game’s spine:

> A station is a commitment. It creates value *and* pressure.

---

## 1. Station Types (MVP)

MVP includes three station types. Each type is simple, legible, and directly tied to delegation pressure.

### MiningOutpost

* **Purpose:** economic engine
* **Input:** fuel
* **Output:** ore over time
* **Pressure:** increases pirate interest and crisis likelihood

### FuelDepot

* **Purpose:** logistics anchor / range extender
* **Input:** delivered fuel
* **Output:** refueling support for fleets and player
* **Pressure:** becomes a chokepoint; losing it strands assets

### SensorStation

* **Purpose:** information control
* **Input:** fuel
* **Output:** slows fog decay, improves threat estimates
* **Pressure:** reduces uncertainty short-term but attracts smarter pirate behavior

---

## 2. Station Enum (Formal Definition)

```rust
enum StationKind {
  MiningOutpost,
  FuelDepot,
  SensorStation,
}
```

### Code Mapping (Bootstrap)

These station types will map directly to code enums:

* `StationKind::MiningOutpost`
* `StationKind::FuelDepot`
* `StationKind::SensorStation`

---

## 3. Lifecycle State Machine

Stations move through states that are **legible** and **predictive**. They do not fail instantly.

### State Enum

```rust
enum StationState {
  Deploying,   // building; no output; highly vulnerable
  Operational, // normal output; stable
  Strained,    // performance degraded; problems emerging
  Failing,     // imminent collapse; intervention required
  Failed,      // destroyed/abandoned; permanent outcome
}
```

### Code Mapping (Bootstrap)

Lifecycle states map 1:1 to:

* `StationState::Deploying`
* `StationState::Operational`
* `StationState::Strained`
* `StationState::Failing`
* `StationState::Failed`

---

## 4. Station Health Model (Inputs to State)

Stations do not have “HP” in the traditional sense. They have a few systemic meters:

* **FuelLevel** (0..max)
* **Integrity** (0..100) — degrades under attack/neglect
* **PressureExposure** (0..100) — derived from pirate pressure and station visibility
* **MaintenanceDebt** (0..100) — grows when strained or under-supplied

Stations transition states based on thresholds and timers, not RNG.

---

## 5. State Transitions (Deterministic Rules)

### Deploying → Operational

* build timer completes
* minimum fuel present (if required)

### Operational → Strained

* fuel below threshold OR
* sustained harassment OR
* maintenance debt crosses threshold

### Strained → Operational

* fuel restored AND
* harassment stops for a short recovery window AND
* maintenance debt reduced

### Strained → Failing

* fuel hits critical low OR
* harassment spikes OR
* integrity drops below threshold

### Failing → Strained

* player or fleet intervention occurs
* crisis mitigated and integrity stabilized

### Failing → Failed

* integrity reaches 0 OR
* failing timer expires without mitigation

---

## 6. Crisis Generation Mapping

Stations are primary crisis generators.

### Crisis Types (MVP)

```rust
enum CrisisType {
  FuelShortage,
  PirateHarassment,
}
```

Crisis generation is **event + threshold based**:

* A station creates a crisis when it crosses defined thresholds.
* Crises are tied to station entity IDs.

---

## 7. Crisis Triggers (Per Station)

### 7.1 FuelShortage Crisis

Triggers when:

* `FuelLevel <= low_fuel_threshold` for a sustained window

Escalates when:

* `FuelLevel <= critical_fuel_threshold`
* Output is suppressed and recovery slows

Effects:

* transitions Operational → Strained
* increases pirate target desirability
* may strand fleets if it is a FuelDepot

---

### 7.2 PirateHarassment Crisis

Triggers when:

* pirate pressure in zone is above threshold AND
* station visibility/exposure is above threshold

Escalates when:

* repeated raid events occur within an interval
* station integrity declines

Effects:

* transitions Operational → Strained
* increases maintenance debt
* can push Strained → Failing

---

## 8. Crisis Escalation Stages (Station-Coupled)

For MVP, crises share the same stage model:

```rust
enum CrisisStage {
  Stable,
  Strained,
  Failing,
  Resolved,
}
```

Mapping:

* StationState Operational → CrisisStage Stable
* StationState Strained → CrisisStage Strained
* StationState Failing → CrisisStage Failing
* StationState Operational (post-recovery) → CrisisStage Resolved

Crises are therefore never “detached” from station reality.

---

## 9. Exact Build-Time & Fuel Math (MVP)

All numbers below assume:

* **FixedUpdate tick = 1 second** (easy to reason about)
* 60–90 minute run target
* Early player intervention is common

### 9.1 Build Times

| StationKind   |  Build Time | Notes                         |
| ------------- | ----------: | ----------------------------- |
| MiningOutpost | 240s (4:00) | long enough to be interrupted |
| FuelDepot     | 180s (3:00) | supports early expansion      |
| SensorStation | 120s (2:00) | fast visibility relief        |

Build rule:

* During Deploying, the station is **non-functional** and **highly vulnerable**.

---

### 9.2 Fuel Consumption

Fuel is measured in abstract “units.” The goal is stable arithmetic and readable thresholds.

#### Baseline consumption (per minute)

| StationKind   | Fuel / min |
| ------------- | ---------: |
| MiningOutpost |        1.0 |
| FuelDepot     |        0.5 |
| SensorStation |       0.75 |

FuelDepot note:

* FuelDepot *stores* fuel and consumes a small amount to operate.

---

### 9.3 Fuel Capacity

| StationKind   | Fuel Capacity |
| ------------- | ------------: |
| MiningOutpost |            30 |
| FuelDepot     |           120 |
| SensorStation |            40 |

Interpretation:

* MiningOutpost: ~30 minutes of autonomy
* FuelDepot: supports a local network / chokepoint
* SensorStation: ~50 minutes of autonomy

---

### 9.4 Fuel Thresholds (Crisis Triggers)

| Threshold               |        Value | Meaning                       |
| ----------------------- | -----------: | ----------------------------- |
| low_fuel_threshold      | 25% capacity | Station becomes Strained soon |
| critical_fuel_threshold | 10% capacity | Station enters Failing risk   |

FuelShortage trigger window:

* if below low threshold for **60 seconds** → create FuelShortage crisis

Failing timer:

* if below critical threshold for **120 seconds** → push to Failing

---

### 9.5 Mining Output (Simple, MVP)

MiningOutpost production rate:

* **1 ore unit / 10 seconds** (6 ore per minute)

Optional richness modifier (if zone resources support it):

* Low: 0.75x
* Medium: 1.0x
* High: 1.25x

Keep this qualitative in UI.

---

## 10. Escalation Ties (Stations Generate Pressure)

Stations raise zone visibility and pirate interest:

* Every station increases zone “attention”
* MiningOutposts increase attention the most
* FuelDepots increase attention as chokepoints

This ensures:

* station spam is punished
* overexpansion creates cascading crises

---

## 11. MVP Implementation Notes

* Stations should be spawnable via a simple build menu
* The state machine should be centralized (one system manages transitions)
* Crises should be generated by state threshold crossings
* UI should surface:

  * StationState
  * FuelLevel
  * Active Crisis + stage
  * Time-to-fail when in Failing

---

## 13. Save/Load Fields (RON Example)

Stations that are in crisis can serialize `crisis_type` and `crisis_stage`:

```ron
(
  stations: [
    (
      kind: MiningOutpost,
      state: Strained,
      x: -120.0,
      y: 80.0,
      fuel: 6.0,
      fuel_capacity: 30.0,
      build_remaining: 0.0,
      crisis_type: FuelShortage,
      crisis_stage: Strained,
    ),
  ],
)
```

## 12. Design Intent Summary

Stations exist to:

* produce value
* attract pressure
* create crises when neglected
* force delegation and prioritization

They are not upgrades.

> Stations are bets, and the game is deciding which bets to keep.
