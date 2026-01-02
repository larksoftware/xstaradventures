# World Structure & Fog of War — XStar Adventures

This document defines how the world is structured and how **fog of war / information uncertainty** works in *XStar Adventures*. The goal is not to hide content, but to control **knowledge, risk, and pressure**.

> The sector exists in full at run start. The player does not unlock systems — they reduce uncertainty.

---

## 1. World Structure Overview

The game world is organized into three conceptual layers:

### 1.1 Sector (Macro Layer)

* A single sector per run
* Represented as a **graph** of connected systems (nodes) and routes (edges)
* Fully generated at run start
* Typical size:

  * **MVP:** 5–7 systems
  * **Full game:** 8–15 systems (hard cap per sector)

This layer is the primary planning and delegation view.

---

### 1.2 Systems / Zones (Mid Layer)

Each system (zone) contains:

* Resource fields (asteroids)
* Stations (player-built or pirate)
* Pirate presence and pressure
* Entry/exit points for routes

Systems are bounded, legible spaces where local pressure accumulates.

---

### 1.3 Points of Interest (Micro Layer)

Within systems:

* Mining nodes
* Pirate raids
* Pirate bases
* Distress or escalation events

These are spatially present but driven by simulation, not missions.

---

## 2. Fog of War Philosophy

Fog of war in *XStar Adventures* represents **informational uncertainty**, not unexplored space.

Key rules:

* The sector is fully simulated from the start
* The player simply lacks reliable information
* Knowledge degrades over time if not refreshed

> Fog hides *certainty*, not *existence*.

---

## 3. Starting State

At the beginning of a run:

* The player knows **one system** (arrival point)
* Adjacent systems appear as:

  * dim silhouettes
  * vague distance indicators
  * unknown resources
  * unknown threats

Routes are visible but risky and poorly understood.

---

## 4. Information Layers

Each system progresses through layers of knowledge. Different actions reveal different layers.

### Layer 0 — Existence

* System node visible
* Routes visible
* No data

---

### Layer 1 — Geography

Unlocked by:

* Player travel
* Scout flyby

Reveals:

* System size
* Rough travel time
* Approximate number of resource fields

---

### Layer 2 — Resources

Unlocked by:

* Scout fleets
* Mining attempts
* Station sensor range

Reveals:

* Ore types
* Richness ranges (low / medium / high)
* Depletion risk estimates

---

### Layer 3 — Threats

Unlocked by:

* Pirate encounters
* Sensor stations
* Sustained traffic

Reveals:

* Pirate pressure estimates
* Raid likelihood
* Approximate pirate base locations

---

### Layer 4 — Stability & Trends (Late Game)

Unlocked by:

* Repeated observation
* Pattern recognition

Reveals:

* Escalation trends
* System stability forecasts
* Emerging danger zones

---

## 5. Discovery Methods

Systems can be investigated via:

* **Player travel** — fastest, riskiest
* **Scout fleets** — slower, imperfect
* **Sensor stations** — passive, limited range

Each method produces incomplete or probabilistic information.

---

## 6. Travel Between Systems

### Route-Based Travel (Recommended)

* Systems are connected by routes
* Travel places ships in an **in-transit state**
* Travel time depends on distance, ship speed, and route risk

Unknown or high-pressure routes:

* Increase travel time
* Increase interception risk
* Increase chance of surprise events

---

## 7. Fog Interaction with Fleets

* Fleets operate with **confidence levels**
* Low-confidence information leads to:

  * conservative routing
  * slower execution
  * aborted tasks

Fleet mistakes feel justified due to uncertainty, not randomness.

---

## 8. Pirate Knowledge Asymmetry

Pirates:

* Operate with full sector knowledge
* Ignore player fog
* Exploit predictable routes and weak visibility

This asymmetry creates pressure and surprise without cheating.

---

## 9. Fog Decay (Critical Rule)

Knowledge decays over time if not refreshed:

* Threat estimates become stale
* Resource projections drift
* Stability forecasts lose accuracy

> Fog must never fully disappear.

If the player ever feels omniscient, tension collapses.

---

## 10. Run Progression with Fog

### Early Run (0–20 min)

* 1–2 known systems
* Player scouts personally
* Fleets unreliable

### Mid Run (20–50 min)

* 3–5 partially known systems
* Delegation dominates
* Pirate bases hinted

### Late Run (50–90 min)

* Most systems known
* Pressure saturates
* Player decides what to abandon

Exploration never truly ends — attention does.

---

## 11. MVP Scope

For MVP:

* Full fog system
* 2–3 information layers (Existence, Resources, Threats)
* Scout fleets + player travel only
* No advanced sensor upgrades

---

## 12. Design Intent Summary

Fog of war exists to:

* Create meaningful uncertainty
* Justify fleet AI imperfection
* Drive exploration decisions
* Amplify pressure over time

The player is never asked to explore everything.
They are asked to decide **what is worth knowing**.

---

## 13. KnowledgeLayer Mapping

These game layers map directly to `KnowledgeLayer` in code:

* Layer 0 — Existence → `KnowledgeLayer::Existence`
* Layer 1 — Geography → `KnowledgeLayer::Geography`
* Layer 2 — Resources → `KnowledgeLayer::Resources`
* Layer 3 — Threats → `KnowledgeLayer::Threats`
* Layer 4 — Stability & Trends → `KnowledgeLayer::Stability`

---

## 14. FogConfig Defaults (Prototype)

The current bootstrap uses a tunable `FogConfig` resource with decay and floor values
per knowledge layer. Defaults (subject to tuning):

* Decay: Existence 0.0005, Geography 0.001, Resources 0.0015, Threats 0.002, Stability 0.0025
* Floor: Existence 0.25, Geography 0.2, Resources 0.15, Threats 0.12, Stability 0.1
