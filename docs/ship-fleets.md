# Ships & Fleets — Types, Lifecycle, Risk, and Fuel Math (MVP)

This document defines **ship types**, a **ship lifecycle state machine**, **construction rules**, and **exact fuel + risk math** for the MVP of *XStar Adventures*.

Ships represent **leverage**. They extend the player’s reach, but only as long as logistics, risk, and attention are managed.

> Stations are commitments. Ships are leverage. Fuel is the truth.

---

## 1. Ship Roster (MVP)

MVP includes the player ship and three autonomous ship types. Each is intentionally specialized and incomplete on its own.

### PlayerShip

* Manual control
* Can mine, scan, and fight
* Strongest early-game problem solver
* Late-game role: crisis breaker, reconnaissance, boss encounters

---

### Scout

* Fast, low fuel capacity
* Excellent sensors
* Minimal combat capability

**Primary role:** reduce fog and increase confidence

---

### Miner

* Moderate speed and fuel capacity
* High cargo capacity
* Weak combat capability

**Primary role:** resource extraction

---

### Security

* Moderate speed and fuel capacity
* Strong combat capability
* Minimal cargo

**Primary role:** escorts, patrols, raid response

---

## 2. Ship Construction Model

For MVP, ships are constructed at **existing stations**.

* Pay ore
* Commit build time
* Ship spawns near the station

Ship construction increases station visibility and pirate interest.

No dedicated shipyard station exists in MVP.

---

## 3. ShipKind Enum (Formal Definition)

```rust
enum ShipKind {
  PlayerShip,
  Scout,
  Miner,
  Security,
}
```

### Code Mapping (Bootstrap)

These ship types will map directly to:

* `ShipKind::PlayerShip`
* `ShipKind::Scout`
* `ShipKind::Miner`
* `ShipKind::Security`

---

## 4. Fleet Role vs Ship Kind

Ship kind defines **capabilities**.
Fleet role defines **intent**.

```rust
enum FleetRole {
  Scout,
  Mining,
  Security,
}
```

### Code Mapping (Bootstrap)

Roles map directly to:

* `FleetRole::Scout`
* `FleetRole::Mining`
* `FleetRole::Security`

In MVP:

* One fleet = one ship
* ShipKind and FleetRole usually align

---

## 5. Ship Lifecycle State Machine

Ships use a small, deterministic set of states.

```rust
enum ShipState {
  Idle,        // awaiting orders
  InTransit,   // traveling between zones
  Executing,   // performing task in a zone
  Returning,   // heading to refuel or home
  Refueling,   // docked and replenishing fuel
  Damaged,     // performance degraded
  Disabled,    // immobile; rescue or abandon
}
```

### Code Mapping (Bootstrap)

State mapping:

* `ShipState::Idle`
* `ShipState::InTransit`
* `ShipState::Executing`
* `ShipState::Returning`
* `ShipState::Refueling`
* `ShipState::Damaged`
* `ShipState::Disabled`

---

## 6. Deterministic State Transitions

* `Idle → InTransit` — order assigned
* `InTransit → Executing` — arrives at target
* `Executing → Returning` — task complete or risk exceeded
* `Returning → Refueling` — reaches station
* `Refueling → Idle` — fuel restored
* `Any → Damaged` — sustained harassment or combat
* `Damaged → Disabled` — ignored or re-hit
* `Disabled → (Recovered | Abandoned)` — player/security response

Ships never disappear instantly; failure is visible and actionable.

---

## 7. Build Time (MVP)

| ShipKind | Build Time |
| -------- | ---------: |
| Scout    |        60s |
| Miner    |       120s |
| Security |       120s |

PlayerShip exists at run start.

---

## 8. Fuel Model (Exact MVP Math)

Fuel units are shared with station fuel.

### 8.1 Fuel Capacity & Burn Rate

| ShipKind   | Capacity | Burn / min |
| ---------- | -------: | ---------: |
| PlayerShip |       60 |        2.0 |
| Scout      |       30 |        1.0 |
| Miner      |       45 |        1.5 |
| Security   |       45 |        1.5 |

---

### 8.2 Fuel Thresholds

* **Low fuel:** 25% capacity → cautious behavior
* **Critical fuel:** 10% capacity → forced return or risk Disabled

Fuel is a deterministic limiter; no RNG involved.

---

## 9. Fleet Risk Model Integration

Ships contribute to fleet decisions through **risk scoring**, not special-case logic.

### 9.1 Fuel Margin

```text
FuelMargin = (CurrentFuel - FuelRequiredForRoundTrip) / Capacity
```

This is the single most important variable in fleet behavior.

---

### 9.2 Capability Profiles (Qualitative)

* **Scout:** high sensor strength, low combat
* **Miner:** high cargo, low combat
* **Security:** high combat, low cargo

Capabilities offset task risk without branching AI logic.

---

### 9.3 Total Risk Formula

```text
TotalRisk = BaseTaskRisk
          + FuelRisk(FuelMargin)
          + ThreatRisk(PiratePressure, Confidence)
          - CapabilityOffset(ShipKind, TaskType)
          + ZoneModifierRisk
```

Risk drives:

* abort decisions
* escort requests
* task prioritization

---

## 10. Escalation Over Time

Ships do not scale via upgrades in MVP.

Difficulty increases because:

* pirate strength escalates with time
* zones become more hostile
* logistics networks stretch thin

Soft progression comes from:

* more ships
* better placement
* smarter delegation

---

## 11. MVP Design Rules (Hard Constraints)

* Ships are underpowered alone
* Ships are powerful in coordination
* Failures are predictable, not random
* PlayerShip remains relevant at all stages

---

## 12. Design Intent Summary

Ships exist to:

* extend reach
* multiply attention
* create logistical tension
* fail in explainable ways

> Ships are leverage — and leverage always has a cost.
