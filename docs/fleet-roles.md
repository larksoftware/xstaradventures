# XStar Adventures — Fleet Roles & Autonomy Design

This document formalizes **fleet roles**, their responsibilities, and how they scale through autonomy. Fleets are the primary way the player converts personal effort into systemic leverage.

---

## 1. Fleet Design Philosophy

**Fleets do jobs, not tasks.**

The player does not micromanage ships. Instead, they assign responsibility and accept imperfect execution.

Key principles:

* Fleets exist to replace repetitive player labor
* Fleets are fallible by design
* Fleet mistakes create downstream gameplay
* Fleets scale responsibility faster than control

Delegation is power — and risk.

---

## 2. Fleet Role Overview

The game supports a small number of clearly defined fleet roles. Each role solves a specific systemic problem and introduces specific failure modes.

Core roles:

* Scout Fleet
* Mining Fleet
* Logistics Fleet
* Security Fleet
* Command Fleet

Not all roles are available at the start; some unlock as the player scales.

---

## 3. Scout Fleet

**Purpose:** Reveal information and reduce uncertainty.

### Responsibilities

* Explore nearby systems
* Identify resource nodes
* Detect pirate pressure and movement
* Provide early warning for crises

### Strengths

* High mobility
* Low operating cost
* Improves decision quality for other fleets

### Weaknesses

* Minimal combat capability
* Incomplete or outdated information

### Failure Modes

* Missed threats
* Inaccurate resource classification
* Interception by pirates

Scout fleets do not prevent danger — they make danger visible.

---

## 4. Mining Fleet

**Purpose:** Convert space into material.

### Responsibilities

* Extract ore from resource nodes
* Supply stations with raw materials

### Strengths

* High economic value
* Scales production beyond player capacity

### Weaknesses

* Stationary or predictable routes
* Highly attractive pirate targets

### Failure Modes

* Equipment damage
* Reduced yield
* Losses due to insufficient protection

Mining fleets create value — and visibility.

---

## 5. Logistics Fleet

**Purpose:** Keep the system alive.

### Responsibilities

* Transport fuel
* Move ore between stations
* Deliver repair supplies

### Strengths

* Enables station uptime
* Prevents cascading failures

### Weaknesses

* Indirect value (often overlooked)
* Vulnerable routes

### Failure Modes

* Delivery delays
* Route inefficiencies
* Supply theft

Logistics fleets are the nervous system of the sector.

---

## 6. Security Fleet

**Purpose:** Shape conflict, not eliminate it.

### Responsibilities

* Defend stations
* Escort other fleets
* Patrol trade routes

### Strengths

* Direct combat capability
* Deterrence

### Weaknesses

* Expensive to maintain
* Can escalate political pressure

### Failure Modes

* Attrition
* Poor response prioritization
* Provoking stronger enemies

Security fleets influence where pressure occurs, not whether it exists.

---

## 7. Command Fleet

**Purpose:** Improve coordination and autonomy.

### Responsibilities

* Extend command range
* Improve fleet decision-making
* Reduce automation drift

### Strengths

* Increases efficiency across multiple fleets
* Enables higher autonomy tiers

### Weaknesses

* Rare and costly
* Strategic target for enemies

### Failure Modes

* Command overload
* Coordination collapse
* Sector-wide inefficiency spikes

Command fleets represent the player’s transition from captain to leader.

---

## 8. Fleet Autonomy Levels

Autonomy defines how much interpretation fleets apply to their orders.

### Tier I — Manual

* Executes explicit orders
* No self-preservation
* High player attention required

### Tier II — Assisted

* Executes intent
* Requests help when threatened
* Minimal reprioritization

### Tier III — Autonomous

* Adjusts routes and timing
* Retreats when necessary
* Coordinates locally

### Tier IV — Strategic

* Adapts to sector-wide changes
* Coordinates multiple fleets
* May override player priorities

Higher autonomy increases efficiency and scale — and introduces risk.

---

## 9. Risk Tolerance & Priorities

Each fleet has:

* A risk tolerance profile (Cautious → Desperate)
* Weighted priorities (yield, safety, speed, cost)

These parameters directly influence AI decision-making and failure likelihood.

---

## 10. Fleet Experience & Learning (Future Extension)

While not required for MVP, fleets may later gain:

* Experience modifiers
* Behavioral quirks
* Performance drift over time

These systems should reinforce personality, not optimization.

---

## 11. Player Interaction Model

The player interacts with fleets through:

* Role assignment
* Intent selection
* Risk tolerance adjustment
* Escort assignment

There is no per-ship control.

---

## 12. Design Intent Summary

Fleets exist to:

* Replace manual labor
* Create scale and leverage
* Introduce systemic failure
* Force prioritization

A successful player does not eliminate fleet mistakes — they **build systems resilient enough to survive them**.

