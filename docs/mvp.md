# XStar Adventures — MVP Definition & Requirements

This document defines the **Minimum Viable Product (MVP)** for *XStar Adventures*. The MVP is the smallest complete version of the game that proves the core fantasy: **delegation under pressure in a living sandbox**.

The MVP is not a demo or tutorial — it is a **truth test**.

---

## 1. MVP Design Goals

### Primary Goal

> Prove that delegation + escalation + player intervention is fun.

### Secondary Goals

* Validate that stations feel like commitments
* Validate that fleet AI mistakes feel fair
* Validate that pirates create pressure without missions

### Non-Goals

* Content breadth
* Visual polish
* Narrative completeness

---

## 2. MVP Success Criteria (Hard Gates)

The MVP is successful if:

* Players regularly check the map and problems feed
* Players delegate instead of doing everything manually
* Players feel pressure before boredom
* Players personally intervene during crises
* Players want to see the system scale

If these are not true, scope must shrink or systems must change.

---

## 3. MVP World Scope

* **1 sector only**
* **5–7 star systems**
* Procedurally generated once per run
* No fast travel

Target playtime per run: **60–90 minutes**

---

## 4. MVP Core Gameplay Loop

Scout → Mine → Build Station → Pressure Emerges → Delegate Fleets → Crisis Escalates → Player Intervenes → World Changes

This loop must function end-to-end.

---

## 5. MVP Systems (Must Be Fully Implemented)

### 5.1 Player Ship

Required:

* Top-down 2D movement
* Fuel consumption
* Manual mining
* Scanning
* Basic combat

Explicitly excluded:

* Multiple ship classes
* Skill trees
* Loadout complexity

---

### 5.2 Resources

* **Common Ore** — basic construction
* **Advanced Ore** — improved fleets/stations

No exotic or relic materials.

---

### 5.3 Stations

Station Types:

* **Basic Outpost** — produces ore, requires fuel
* **Fuel Depot** — extends range, supports fleets

Station lifecycle states:

* Deployment
* Operational
* Strained
* Failed

Stations must be able to fail permanently.

---

### 5.4 Fleets

Fleet Roles (MVP):

* Scout Fleet
* Mining Fleet
* Security Fleet

Fleet capabilities:

* Assigned intent
* Risk tolerance
* Autonomous execution
* Failure & reporting

Logistics fleets are deferred; the player fills this role early.

---

### 5.5 Fleet AI

Fleet AI must include:

* Intent-based decision-making
* Imperfect awareness
* Risk tolerance
* Priority weighting
* Reasoned failure (no RNG-only failures)

---

### 5.6 Delegation UI

Required UI:

* Galaxy/sector map
* Fleet panel (intent, risk)
* Station panel (status, mode)
* Problems feed

Usability requirements:

* ≤ 3 clicks to change delegation
* ≤ 5 seconds to understand a problem

---

### 5.7 Pirates

Pirate systems required:

* Roaming patrols (Epoch 1–2 behavior)
* Pressure zones
* One pirate base
* One boss encounter

Pirates escalate with time, not player wealth.

---

### 5.8 Crisis System (MVP Cut)

Implemented crisis types:

* Fuel shortages
* Pirate harassment

Crisis stages:

* Stable
* Strained
* Failing
* Resolved

Crises must cascade if ignored.

---

## 6. MVP Systems Explicitly Cut

* Factions & politics
* Logistics fleets
* Automation drift
* Crew experience
* Advanced pirate adaptation
* Multiple sectors
* Endgame states
* Visual/audio polish

Anything not supporting the core loop is excluded.

---

## 7. MVP Build Order (High-Level)

1. Player ship movement, fuel, mining
2. Single station + lifecycle states
3. Pirate patrols + pressure
4. Basic combat
5. Fleets + delegation UI
6. Crisis escalation
7. Pirate base + boss

UI must not be left to the end.

---

## 8. MVP Kill Criteria (Very Important)

The project should pause or pivot if:

* Delegation feels like busywork
* Fleet failures feel random or unfair
* Players ignore stations
* Pirates feel grindy instead of threatening

Failure here is cheaper than scaling the wrong game.

---

## 9. MVP Design Intent Summary

The MVP exists to prove:

* Delegation creates engagement
* Pressure creates stories
* Failure creates gameplay

If this MVP is compelling, *XStar Adventures* deserves to scale.

