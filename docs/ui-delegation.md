# XStar Adventures — Delegation UI Design

This document formalizes the **delegation UI** for fleets and stations. The UI must empower the player to make strategic decisions quickly, understand consequences at a glance, and avoid micromanagement.

---

## 1. UI Design Goals

### Primary Goal

> Let the player make meaningful command decisions in seconds.

### Secondary Goals

* Communicate system state without noise
* Surface problems as actionable situations
* Scale to late-game without becoming a spreadsheet

### Non-Goals

* Per-ship control
* Complex nested menus
* Alert spam

---

## 2. Core UI Principles (Non-Negotiable)

1. **Intent, not instructions**

   * Players assign responsibility ("protect this", "mine that"), not tactics.

2. **Problems, not alerts**

   * The UI surfaces issues as questions to resolve, not pop-up interruptions.

3. **Strategic verbs**

   * Controls should be expressed as verbs: Stabilize, Reinforce, Evacuate.

4. **Compression over expansion**

   * As scale increases, UI aggregates; it does not add complexity.

---

## 3. Primary Screen: Galaxy / Sector View

The Galaxy View is the player’s command surface.

### Visual Layers

* **Systems / nodes:** procedural layout
* **Stations:** icons with lifecycle status rings
* **Fleets:** icons with routes and current intent
* **Pressure zones:** pirate/faction overlays
* **Routes:** supply lanes and escort links

### Map Status Language (No Text Required)

* 🟢 Stable
* 🟡 Strained
* 🔴 Failing
* ⚫ Lost / Abandoned

Stations and fleets must be readable at a glance.

---

## 4. Persistent Command Bar (Always Visible)

A compact dock (bottom or side) that acts as the player’s quick nav.

### Tabs

* **Fleets** (count + issues)
* **Stations** (count + issues)
* **Logistics** (optional later)
* **Events / Problems**

Each tab displays a minimal badge:

* Total assets
* Number of critical issues

No blinking, no modal popups.

---

## 5. Fleet Panel (Core Interaction)

Selecting a fleet opens a single, compact panel.

### 5.1 Fleet Header

* Fleet name
* Role icon (Scout / Mining / Logistics / Security / Command)
* Autonomy tier indicator
* Current status (Stable/Strained/Failing)

### 5.2 Intent Section

> “This fleet exists to:”

* Pick **one primary intent**

  * Protect Station X
  * Patrol Route Y
  * Mine Node Z
  * Scout Sector Q

Changing intent updates AI assumptions and priorities.

### 5.3 Priority Controls (Max 3)

Use at most **three sliders** (role-specific) that map directly to AI weights.

Examples:

* **Mining:** Yield ↔ Safety, Speed ↔ Efficiency, Cost ↔ Stability
* **Security:** Coverage ↔ Concentration, Deterrence ↔ Response, Risk ↔ Loss Tolerance
* **Scout:** Coverage ↔ Depth, Speed ↔ Caution, Stealth ↔ Data Quality

### 5.4 Risk Tolerance (Single Slider)

* Cautious → Balanced → Aggressive → Desperate

Risk tolerance shifts decision outcomes across all action classes.

### 5.5 Action Buttons (Strategic)

Small set of verbs (contextual):

* Stabilize
* Reroute
* Hold
* Retreat
* Request Escort

These are command-level toggles, not per-ship instructions.

---

## 6. Escort Assignment (Drag-and-Link)

Escorts are assigned visually.

* Drag a **Security Fleet** onto another fleet or station
* Creates a **soft escort link**, not a fleet merge

UI shows:

* Escort strength
* Coverage radius
* Opportunity cost (what this fleet stops doing)

This encourages tradeoffs and prevents escort spam.

---

## 7. Station Panel (Strategic Control)

Selecting a station opens a station panel.

### 7.1 Station Overview

* Type (Outpost, Fuel Depot, etc.)
* Output summary
* Dependency summary (fuel, ore, defense)
* Lifecycle state

### 7.2 Operational Mode (One Setting)

* Maximize Output
* Balanced
* Low Profile
* Emergency Shutdown

Modes adjust station behavior and fleet priorities indirectly.

### 7.3 Strategic Verbs

Contextual actions:

* Reinforce
* Downscale
* Evacuate
* Isolate

No module-level micromanagement.

---

## 8. Problems Feed (No Alert Spam)

A single list of the player’s actionable issues.

### 8.1 Problems Are Phrased as Situations

Examples:

* “Fuel delivery delayed to Fuel Depot Gamma”
* “Mining yield falling in Sector J4”
* “Security coverage insufficient near Route Delta”

### 8.2 Clicking a Problem

* Highlights affected assets
* Presents **3 suggested actions max**

Example suggestions:

* Assign escort
* Lower risk tolerance
* Temporarily downscale station

This keeps the player in flow.

---

## 9. Player Intervention Mode

The UI must make intervention feel powerful and costly.

Intervention affordances:

* “Fly to location”
* “Override autonomy temporarily”
* “Prioritize this at the expense of others”

When choosing intervention, show the explicit tradeoff:

* “Addressing this will delay response elsewhere.”

---

## 10. Scaling Rules (Late-Game)

As assets grow:

* Fleets group by role and sector
* Stations group by system
* Problems aggregate ("3 logistics issues in Outer Ring")

Drill-down remains available, but is optional.

---

## 11. Speed Targets (Usability Requirements)

* Any delegation change in **≤ 3 clicks**
* Any problem understood in **≤ 5 seconds**
* No deep modal chains

If it’s slower, the UI must be simplified.

---

## 12. Design Intent Summary

The delegation UI exists to:

* Preserve the fantasy of command
* Prevent micromanagement
* Make problems actionable and readable
* Scale cleanly into late-game complexity

The player should feel like a commander, not an operator.

