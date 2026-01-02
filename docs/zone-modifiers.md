# Zone Modifiers & Fleet Risk — XStar Adventures

This document formalizes **zone-level buffs/debuffs**, defines which modifiers appear in the MVP, and maps those modifiers directly into **fleet risk and decision-making**.

The goal is to introduce meaningful variation between zones without increasing micromanagement or UI noise.

> Zone modifiers bend decisions; they do not demand attention.

---

## 1. Design Principles (Hard Rules)

All zone modifiers must follow these rules:

* Modifiers are **systemic**, not tactical
* Modifiers apply to **zones**, not individual ships
* Modifiers affect **risk, confidence, and efficiency**, not raw controls
* Modifiers are **slow-moving** and predictable
* Modifiers are **legible at a glance**

If a modifier requires constant manual reaction, it is out of scope.

---

## 2. ZoneModifier Enum (Formal Definition)

All zone buffs/debuffs are represented by a single enum. Each modifier is exclusive and dominant.

```rust
enum ZoneModifier {
  // Environmental
  HighRadiation,
  NebulaInterference,

  // Economic
  RichOreVeins,
  DepletedResources,

  // Informational
  SensorNoise,
  ClearSignals,
}
```

Notes:

* Modifiers are **qualitative**, not numeric
* Numeric effects are derived centrally (no per-modifier math scattered in code)
* Zones may have **zero or one** modifier in the MVP

### Current MVP Code Mapping

The bootstrap implementation currently includes the MVP subset:

* `HighRadiation`
* `NebulaInterference`
* `RichOreVeins`
* `ClearSignals`

---

## 3. Modifier Categories & Effects

### 3.1 Environmental Modifiers

#### HighRadiation

* ↑ fuel consumption
* ↓ sensor reliability
* ↑ fleet attrition risk

Narrative meaning:

> High output systems attract attention and strain logistics.

---

#### NebulaInterference

* ↓ scouting confidence
* ↑ ambush success chance
* ↓ long-range threat accuracy

Narrative meaning:

> Visibility is poor; surprises are common.

---

### 3.2 Economic Modifiers

#### RichOreVeins

* ↑ mining yield
* ↑ pirate attention
* ↑ escalation speed

Narrative meaning:

> Opportunity draws danger.

---

#### DepletedResources

* ↓ mining yield
* ↓ pirate interest
* ↑ long-term stability

Narrative meaning:

> Quiet, limited, but predictable.

---

### 3.3 Informational Modifiers

#### SensorNoise

* Fog decays faster
* Threat estimates drift
* False positives possible

Narrative meaning:

> Intel cannot be trusted for long.

---

#### ClearSignals

* Fog decays slower
* Higher scouting confidence
* More reliable forecasts

Narrative meaning:

> Information ages gracefully here.

---

## 4. MVP Modifier Selection

For MVP, the modifier set is intentionally limited.

### Included in MVP

* HighRadiation
* NebulaInterference
* RichOreVeins
* ClearSignals

### Explicitly Excluded (Post-MVP)

* Hazard stacking
* Dynamic modifier changes
* Rare anomalies
* Per-ship effects

MVP zones:

* May have **0 or 1 modifier**
* Many zones will have **no modifier**
* Star type provides the baseline; modifiers are secondary

---

## 5. Fleet Risk Model Overview

Fleets do not react to modifiers directly.

Instead:

* Modifiers influence **risk scoring inputs**
* Risk scores influence **fleet decisions**

This keeps AI logic simple and extensible.

---

## 6. Fleet Risk Inputs

Each fleet computes a risk score per task using weighted inputs:

* Base mission risk
* Fuel margin
* Confidence level (fog)
* Pirate pressure
* Zone modifier influence

```text
TotalRisk = BaseRisk
          + FuelRisk
          + ConfidenceRisk
          + PiratePressureRisk
          + ZoneModifierRisk
```

---

## 7. ZoneModifier → Risk Mapping

### HighRadiation

* +FuelRisk
* +AttritionRisk

Effect:

* Fleets abort earlier
* Conservative routing preferred

---

### NebulaInterference

* +ConfidenceRisk
* +AmbushRisk

Effect:

* Scouts report incomplete data
* Security fleets patrol longer

---

### RichOreVeins

* -EconomicRisk
* +PiratePressureRisk

Effect:

* Mining fleets accept higher danger
* Escorts recommended

---

### ClearSignals

* -ConfidenceRisk

Effect:

* Fleets operate more decisively
* Faster task completion

---

## 8. Fleet Behavior Changes (No New AI Branches)

Zone modifiers **do not introduce special-case logic**.

Instead they bias existing decisions:

* Route selection
* Abort thresholds
* Escort requests
* Task prioritization

This ensures:

* Predictable behavior
* Explainable failures
* Low maintenance complexity

---

## 9. UI Representation

Zone modifiers appear:

* In the zone/system panel
* As a single icon
* With a one-line description

Example:

> **Nebula Interference** — scouting confidence decays faster

No numeric tooltips are shown to the player.

---

## 10. Design Intent Summary

Zone modifiers exist to:

* Differentiate systems meaningfully
* Create risk asymmetry
* Justify fleet imperfection
* Increase replayability without content bloat

> If a modifier does not change player decisions, it should not exist.
